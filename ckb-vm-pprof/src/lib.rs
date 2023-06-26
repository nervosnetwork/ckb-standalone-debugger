use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ckb_vm::decoder::{build_decoder, Decoder};
use ckb_vm::instructions::instruction_length;
use ckb_vm::machine::{DefaultMachine, DefaultMachineBuilder, VERSION0};
use ckb_vm::memory::Memory;
use ckb_vm::registers::{A0, SP};
use ckb_vm::{
    cost_model, Bytes, CoreMachine, DefaultCoreMachine, Error, Machine, Register, SparseMemory, SupportMachine,
    Syscalls, WXorXMemory, ISA_MOP,
};
pub use cost_model::estimate_cycles;

type Addr2LineEndianReader = addr2line::gimli::EndianReader<addr2line::gimli::RunTimeEndian, Rc<[u8]>>;
type Addr2LineContext = addr2line::Context<Addr2LineEndianReader>;
type Addr2LineFrameIter<'a> = addr2line::FrameIter<'a, Addr2LineEndianReader>;

fn sprint_fun(frame_iter: &mut Addr2LineFrameIter) -> String {
    let mut s = String::from("??");
    loop {
        if let Some(data) = frame_iter.next().unwrap() {
            let function = data.function.unwrap();
            s = String::from(addr2line::demangle_auto(
                Cow::from(function.raw_name().unwrap()),
                function.language,
            ));
            continue;
        }
        break;
    }
    s
}

fn goblin_fun(elf: &goblin::elf::Elf) -> HashMap<u64, String> {
    let mut map = HashMap::new();
    for sym in &elf.syms {
        if !sym.is_function() {
            continue;
        }
        if let Some(Ok(r)) = elf.strtab.get(sym.st_name) {
            map.insert(sym.st_value, r.to_string());
        }
    }
    map
}

fn goblin_get_sym(elf: &goblin::elf::Elf, sym: &str) -> u64 {
    for e in &elf.syms {
        if let Some(Ok(r)) = elf.strtab.get(e.st_name) {
            if r == sym {
                return e.st_value;
            }
        }
    }
    return 0;
}

struct TrieNode {
    addr: u64,
    link: u64,
    pc: u64,
    parent: Option<Rc<RefCell<TrieNode>>>,
    childs: Vec<Rc<RefCell<TrieNode>>>,
    cycles: u64,
    regs: [[u64; 32]; 2],
}

impl TrieNode {
    fn root() -> Self {
        Self {
            addr: 0,
            link: 0,
            pc: 0,
            parent: None,
            childs: vec![],
            cycles: 0,
            regs: [[0; 32]; 2],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tags {
    addr: u64,
    file: String,
    line: u32,
    func: String,
}

impl Tags {
    fn new(addr: u64) -> Self {
        Tags {
            addr,
            file: String::from("??"),
            line: 0xffffffff,
            func: String::from("??"),
        }
    }

    pub fn func(&self) -> String {
        if self.func != "??" {
            self.func.clone()
        } else {
            format!("func_0x{:x}", self.addr)
        }
    }

    pub fn simple(&self) -> String {
        format!("{}:{}", self.file, self.func())
    }

    pub fn detail(&self) -> String {
        if self.line == 0xffffffff {
            format!("{}:??:{}", self.file, self.func)
        } else {
            format!("{}:{}:{}", self.file, self.line, self.func)
        }
    }
}

pub struct Profile {
    addrctx: Addr2LineContext,
    trie_root: Rc<RefCell<TrieNode>>,
    trie_node: Rc<RefCell<TrieNode>>,
    cache_tag: HashMap<u64, Tags>,
    cache_fun: HashMap<u64, String>,
    sbrk_addr: u64,
    sbrk_heap: u64,
}

impl Profile {
    pub fn new(program: &Bytes) -> Result<Self, Box<dyn std::error::Error>> {
        let object = object::File::parse(&program)?;
        let ctx = addr2line::Context::new(&object)?;
        let trie_root = Rc::new(RefCell::new(TrieNode::root()));
        let elf = goblin::elf::Elf::parse(&program)?;
        trie_root.borrow_mut().addr = elf.entry;
        Ok(Self {
            addrctx: ctx,
            trie_root: trie_root.clone(),
            trie_node: trie_root,
            cache_tag: HashMap::new(),
            cache_fun: goblin_fun(&elf),
            sbrk_addr: goblin_get_sym(&elf, "_sbrk"),
            sbrk_heap: goblin_get_sym(&elf, "_end"),
        })
    }

    pub fn get_tag(&mut self, addr: u64) -> Tags {
        if let Some(data) = self.cache_tag.get(&addr) {
            return data.clone();
        }
        let mut tag = Tags::new(addr);
        let loc = self.addrctx.find_location(addr).unwrap();
        if let Some(loc) = loc {
            tag.file = loc.file.as_ref().unwrap().to_string();
            if let Some(line) = loc.line {
                tag.line = line;
            }
        }
        let mut frame_iter = self.addrctx.find_frames(addr).unwrap();
        tag.func = sprint_fun(&mut frame_iter);
        self.cache_tag.insert(addr, tag.clone());
        tag
    }

    fn display_flamegraph_rec(&mut self, prefix: &str, node: Rc<RefCell<TrieNode>>, writer: &mut impl std::io::Write) {
        let prefix_name = format!("{}{}", prefix, self.get_tag(node.borrow().addr).simple());
        writer.write_all(format!("{} {}\n", prefix_name, node.borrow().cycles).as_bytes()).unwrap();
        for e in &node.borrow().childs {
            self.display_flamegraph_rec(format!("{}; ", prefix_name).as_str(), e.clone(), writer);
        }
        writer.flush().unwrap();
    }

    pub fn display_flamegraph(&mut self, writer: &mut impl std::io::Write) {
        self.display_flamegraph_rec("", self.trie_root.clone(), writer);
    }

    pub fn display_stacktrace(&mut self, prefix: &str, writer: &mut impl std::io::Write) {
        let mut frame = self.trie_node.clone();
        let mut stack = vec![self.get_tag(frame.borrow().pc).detail()];
        loop {
            stack.push(self.get_tag(frame.borrow().link).detail());
            let parent = frame.borrow().parent.clone();
            if let Some(p) = parent {
                frame = p.clone();
            } else {
                break;
            }
        }
        stack.reverse();
        for i in &stack {
            writer.write_all(format!("{}{}\n", prefix, i).as_bytes()).unwrap();
        }
        writer.flush().unwrap();
    }

    fn step<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>(
        &mut self,
        machine: &mut DefaultMachine<Inner>,
        decoder: &mut Decoder,
    ) -> Result<(), Error> {
        let pc = machine.pc().to_u64();
        let sp = machine.registers()[SP].to_u64();
        if sp < self.sbrk_heap {
            return Err(Error::External(format!(
                "Heap and stack overlapping sp={} heap={}",
                sp, self.sbrk_heap
            )));
        }
        let inst = decoder.decode(machine.memory_mut(), pc)?;
        let opcode = ckb_vm::instructions::extract_opcode(inst);
        let cycles = machine.instruction_cycle_func()(inst);
        self.trie_node.borrow_mut().cycles += cycles;
        self.trie_node.borrow_mut().pc = pc;

        let call = |s: &mut Self, addr: u64, link: u64| {
            let mut regs = [[0; 32]; 2];
            for i in 0..32 {
                regs[0][i] = machine.registers()[i].to_u64();
            }
            let chd = Rc::new(RefCell::new(TrieNode {
                addr: addr,
                link: link,
                pc: pc,
                parent: Some(s.trie_node.clone()),
                childs: vec![],
                cycles: 0,
                regs: regs,
            }));
            s.trie_node.borrow_mut().childs.push(chd.clone());
            s.trie_node = chd;
        };

        let sbrk_or_skip = |s: &mut Self| {
            if s.trie_node.borrow().addr == s.sbrk_addr {
                // https://github.com/nervosnetwork/riscv-newlib/blob/newlib-4.1.0-fork/libgloss/riscv/sys_sbrk.c#L49
                // Note incr could be negative.
                s.sbrk_heap = s.trie_node.borrow().regs[0][A0].wrapping_add(s.trie_node.borrow().regs[1][A0]);
            }
        };

        let quit_or_skip = |s: &mut Self, addr: u64| {
            let mut f = s.trie_node.clone();
            loop {
                if f.borrow().link == addr {
                    for i in 0..32 {
                        s.trie_node.borrow_mut().regs[1][i] = machine.registers()[i].to_u64();
                    }
                    sbrk_or_skip(s);
                    if let Some(p) = f.borrow().parent.clone() {
                        s.trie_node = p.clone();
                    } else {
                        unimplemented!();
                    }
                    break;
                }
                let p = f.borrow().parent.clone();
                if let Some(p) = p {
                    f = p.clone();
                } else {
                    break;
                }
            }
        };

        if opcode == ckb_vm::instructions::insts::OP_JAL {
            let inst_length = instruction_length(inst) as u64;
            let inst = ckb_vm::instructions::Utype(inst);
            let addr = pc.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
            let link = pc + inst_length;
            if self.cache_fun.contains_key(&addr) {
                call(self, addr, link);
                return Ok(());
            }
            quit_or_skip(self, addr);
            return Ok(());
        };
        if opcode == ckb_vm::instructions::insts::OP_JALR_VERSION0
            || opcode == ckb_vm::instructions::insts::OP_JALR_VERSION1
        {
            let inst_length = instruction_length(inst) as u64;
            let inst = ckb_vm::instructions::Itype(inst);
            let base = machine.registers()[inst.rs1()].to_u64();
            let addr = base.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
            let link = pc + inst_length;
            if self.cache_fun.contains_key(&addr) {
                call(self, addr, link);
                return Ok(());
            }
            quit_or_skip(self, addr);
            return Ok(());
        };
        if opcode == ckb_vm::instructions::insts::OP_FAR_JUMP_ABS {
            let inst_length = instruction_length(inst) as u64;
            let inst = ckb_vm::instructions::Utype(inst);
            let addr = (inst.immediate_s() as u64) & 0xfffffffffffffffe;
            let link = pc + inst_length;
            if self.cache_fun.contains_key(&addr) {
                call(self, addr, link);
                return Ok(());
            }
            quit_or_skip(self, addr);
            return Ok(());
        }
        if opcode == ckb_vm::instructions::insts::OP_FAR_JUMP_REL {
            let inst_length = instruction_length(inst) as u64;
            let inst = ckb_vm::instructions::Utype(inst);
            let addr = pc.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
            let link = pc + inst_length;
            if self.cache_fun.contains_key(&addr) {
                call(self, addr, link);
                return Ok(());
            }
            quit_or_skip(self, addr);
            return Ok(());
        }
        return Ok(());
    }
}

pub struct PProfMachine<Inner> {
    pub machine: DefaultMachine<Inner>,
    pub profile: Profile,
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> CoreMachine for PProfMachine<Inner> {
    type REG = <Inner as CoreMachine>::REG;
    type MEM = <Inner as CoreMachine>::MEM;

    fn pc(&self) -> &Self::REG {
        &self.machine.pc()
    }

    fn update_pc(&mut self, pc: Self::REG) {
        self.machine.update_pc(pc)
    }

    fn commit_pc(&mut self) {
        self.machine.commit_pc()
    }

    fn memory(&self) -> &Self::MEM {
        self.machine.memory()
    }

    fn memory_mut(&mut self) -> &mut Self::MEM {
        self.machine.memory_mut()
    }

    fn registers(&self) -> &[Self::REG] {
        self.machine.registers()
    }

    fn set_register(&mut self, idx: usize, value: Self::REG) {
        self.machine.set_register(idx, value)
    }

    fn isa(&self) -> u8 {
        self.machine.isa()
    }

    fn version(&self) -> u32 {
        self.machine.version()
    }
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> Machine for PProfMachine<Inner> {
    fn ecall(&mut self) -> Result<(), Error> {
        self.machine.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.machine.ebreak()
    }
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> PProfMachine<Inner> {
    pub fn new(machine: DefaultMachine<Inner>, profile: Profile) -> Self {
        Self { machine, profile }
    }

    pub fn load_program(&mut self, program: &Bytes, args: &[Bytes]) -> Result<u64, Error> {
        self.machine.load_program(program, args)
    }

    pub fn run(&mut self) -> Result<i8, Error> {
        if self.isa() & ISA_MOP != 0 && self.version() == VERSION0 {
            return Err(Error::InvalidVersion);
        }
        let mut decoder = build_decoder::<Inner::REG>(self.isa(), self.version());
        self.machine.set_running(true);
        while self.machine.running() {
            if self.machine.reset_signal() {
                decoder.reset_instructions_cache();
                self.profile = Profile::new(&self.machine.code()).unwrap();
            }
            self.profile.step(&mut self.machine, &mut decoder)?;
            self.machine.step(&mut decoder)?;
        }
        Ok(self.machine.exit_code())
    }
}

pub fn quick_start(
    syscalls: Vec<Box<(dyn Syscalls<DefaultCoreMachine<u64, WXorXMemory<SparseMemory<u64>>>>)>>,
    fl_bin: &str,
    fl_arg: Vec<&str>,
    output_filename: &str,
) -> Result<(i8, u64), Error> {
    let code_data = std::fs::read(fl_bin)?;
    let code = Bytes::from(code_data);

    let isa = ckb_vm::ISA_IMC | ckb_vm::ISA_A | ckb_vm::ISA_B | ckb_vm::ISA_MOP;
    let default_core_machine = ckb_vm::DefaultCoreMachine::<
        u64,
        ckb_vm::memory::wxorx::WXorXMemory<ckb_vm::memory::sparse::SparseMemory<u64>>,
    >::new(isa, ckb_vm::machine::VERSION2, 1 << 32);
    let mut builder =
        DefaultMachineBuilder::new(default_core_machine).instruction_cycle_func(Box::new(cost_model::estimate_cycles));
    builder = syscalls.into_iter().fold(builder, |builder, syscall| builder.syscall(syscall));
    let default_machine = builder.build();
    let profile = Profile::new(&code).unwrap();
    let mut machine = PProfMachine::new(default_machine, profile);
    let mut args = vec![fl_bin.to_string().into()];
    args.append(&mut fl_arg.iter().map(|x| Bytes::from(x.to_string())).collect());
    machine.load_program(&code, &args).unwrap();
    let result = machine.run();

    if let Err(err) = result {
        machine.profile.display_stacktrace("", &mut std::io::stdout());
        return Err(err);
    }

    if output_filename == "-" {
        machine.profile.display_flamegraph(&mut std::io::stdout());
    } else {
        let mut output = std::fs::File::create(&output_filename).expect("can't create file");
        machine.profile.display_flamegraph(&mut output);
    }

    Ok((0, machine.machine.cycles()))
}
