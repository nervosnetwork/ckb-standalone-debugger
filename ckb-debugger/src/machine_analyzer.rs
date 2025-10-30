use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::decoder::{Decoder, build_decoder};
use ckb_vm::instructions::instruction_length;
use ckb_vm::machine::VERSION0;
use ckb_vm::registers::{A0, SP};
use ckb_vm::{
    Bytes, CoreMachine, DefaultCoreMachine, DefaultMachine, Error, FlatMemory, ISA_MOP, Machine, Register,
    SupportMachine, WXorXMemory,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::rc::Rc;

type Addr2LineEndianReader = addr2line::gimli::EndianReader<addr2line::gimli::RunTimeEndian, Rc<[u8]>>;
type Addr2LineContext = addr2line::Context<Addr2LineEndianReader>;
type Addr2LineFrameIter<'a> = addr2line::FrameIter<'a, Addr2LineEndianReader>;

fn sprint_fun(frame_iter: &mut Addr2LineFrameIter) -> String {
    let mut s = String::from("??");
    loop {
        if let Some(data) = frame_iter.next().unwrap() {
            if let Some(function) = data.function {
                s = String::from(addr2line::demangle_auto(Cow::from(function.raw_name().unwrap()), function.language));
                continue;
            }
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
        Self { addr: 0, link: 0, pc: 0, parent: None, childs: vec![], cycles: 0, regs: [[0; 32]; 2] }
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
        Tags { addr, file: String::from("??"), line: 0xffffffff, func: String::from("??") }
    }

    pub fn func(&self) -> String {
        if self.func != "??" { self.func.clone() } else { format!("func_0x{:x}", self.addr) }
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

pub struct MachineFlamegraph {
    addrctx: Addr2LineContext,
    trie_root: Rc<RefCell<TrieNode>>,
    trie_node: Rc<RefCell<TrieNode>>,
    cache_tag: HashMap<u64, Tags>,
    cache_fun: HashMap<u64, String>,
}

impl MachineFlamegraph {
    pub fn new(program: &Bytes) -> Result<Self, Box<dyn std::error::Error>> {
        let object = addr2line::object::File::parse(program.as_ref())?;
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
        })
    }

    pub fn reset(&mut self, program: &Bytes) -> Result<(), Box<dyn std::error::Error>> {
        let object = addr2line::object::File::parse(program.as_ref())?;
        let ctx = addr2line::Context::new(&object)?;
        let trie_root = Rc::new(RefCell::new(TrieNode::root()));
        let elf = goblin::elf::Elf::parse(&program)?;
        trie_root.borrow_mut().addr = elf.entry;
        self.addrctx = ctx;
        self.trie_root = trie_root.clone();
        self.trie_node = trie_root;
        self.cache_tag = HashMap::new();
        self.cache_fun = goblin_fun(&elf);
        Ok(())
    }

    pub fn get_tag(&mut self, addr: u64) -> Tags {
        if let Some(data) = self.cache_tag.get(&addr) {
            return data.clone();
        }
        let mut tag = Tags::new(addr);
        let loc = self.addrctx.find_location(addr).unwrap();
        if let Some(loc) = loc {
            tag.file = loc.file.unwrap().to_string();
            if let Some(line) = loc.line {
                tag.line = line;
            }
        }
        let mut frame_iter = self.addrctx.find_frames(addr).skip_all_loads().unwrap();
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

    pub fn step(
        &mut self,
        decoder: &mut Decoder,
        machine: &mut DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>,
    ) -> Result<(), Error> {
        let pc = machine.pc().to_u64();
        let inst = decoder.decode(machine.memory_mut(), pc)?;
        let opcode = ckb_vm::instructions::extract_opcode(inst);
        let cycles = estimate_cycles(inst);
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

        let jump = |s: &mut Self, addr: u64| {
            let mut f = s.trie_node.clone();
            loop {
                if f.borrow().link == addr {
                    for i in 0..32 {
                        s.trie_node.borrow_mut().regs[1][i] = machine.registers()[i].to_u64();
                    }
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
            jump(self, addr);
            return Ok(());
        };
        if opcode == ckb_vm::instructions::insts::OP_JALR_VERSION0 {
            let inst_length = instruction_length(inst) as u64;
            let inst = ckb_vm::instructions::Itype(inst);
            let base = machine.registers()[inst.rs1()].to_u64();
            let addr = base.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
            let link = pc + inst_length;
            if self.cache_fun.contains_key(&addr) {
                call(self, addr, link);
                return Ok(());
            }
            jump(self, addr);
            return Ok(());
        };
        if opcode == ckb_vm::instructions::insts::OP_JALR_VERSION1 {
            let inst_length = instruction_length(inst) as u64;
            let inst = ckb_vm::instructions::Itype(inst);
            let base = machine.registers()[inst.rs1()].to_u64();
            let addr = base.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
            let link = pc + inst_length;
            if self.cache_fun.contains_key(&addr) {
                call(self, addr, link);
                return Ok(());
            }
            jump(self, addr);
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
            jump(self, addr);
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
            jump(self, addr);
            return Ok(());
        }
        return Ok(());
    }
}

pub struct MachineOverlap {
    sbrk_addr: u64,
    sbrk_heap: u64,
}

impl MachineOverlap {
    pub fn new(program: &Bytes) -> Result<Self, Box<dyn std::error::Error>> {
        let elf = goblin::elf::Elf::parse(&program)?;
        Ok(Self { sbrk_addr: goblin_get_sym(&elf, "_sbrk"), sbrk_heap: goblin_get_sym(&elf, "_end") })
    }

    pub fn step(
        &mut self,
        decoder: &mut Decoder,
        machine: &mut DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>,
        flamegraph: &MachineFlamegraph,
    ) -> Result<(), Error> {
        let pc = machine.pc().to_u64();
        let sp = machine.registers()[SP].to_u64();
        if sp < self.sbrk_heap {
            return Err(Error::External(format!("Heap and stack overlapping sp={} heap={}", sp, self.sbrk_heap)));
        }
        let inst = decoder.decode(machine.memory_mut(), pc)?;
        let opcode = ckb_vm::instructions::extract_opcode(inst);
        let addr = match opcode {
            ckb_vm::instructions::insts::OP_JAL => {
                let inst = ckb_vm::instructions::Utype(inst);
                let addr = pc.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
                addr
            }
            ckb_vm::instructions::insts::OP_JALR_VERSION0 => {
                let inst = ckb_vm::instructions::Itype(inst);
                let base = machine.registers()[inst.rs1()].to_u64();
                let addr = base.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
                addr
            }
            ckb_vm::instructions::insts::OP_JALR_VERSION1 => {
                let inst = ckb_vm::instructions::Itype(inst);
                let base = machine.registers()[inst.rs1()].to_u64();
                let addr = base.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
                addr
            }
            ckb_vm::instructions::insts::OP_FAR_JUMP_ABS => {
                let inst = ckb_vm::instructions::Utype(inst);
                let addr = (inst.immediate_s() as u64) & 0xfffffffffffffffe;
                addr
            }
            ckb_vm::instructions::insts::OP_FAR_JUMP_REL => {
                let inst = ckb_vm::instructions::Utype(inst);
                let addr = pc.wrapping_add(inst.immediate_s() as u64) & 0xfffffffffffffffe;
                addr
            }
            _ => return Ok(()),
        };

        let mut f = flamegraph.trie_node.clone();
        loop {
            if f.borrow().link == addr {
                if flamegraph.trie_node.borrow().addr == self.sbrk_addr {
                    // https://github.com/nervosnetwork/riscv-newlib/blob/newlib-4.1.0-fork/libgloss/riscv/sys_sbrk.c#L49
                    // Note incr could be negative.
                    self.sbrk_heap = flamegraph.trie_node.borrow().regs[0][A0].wrapping_add(machine.registers()[A0]);
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

        return Ok(());
    }
}

pub struct MachineStepLog {
    file: Option<std::fs::File>,
    name: String,
}

impl MachineStepLog {
    pub fn new(filename: &str) -> Self {
        Self { file: None, name: filename.to_string() }
    }

    pub fn step(
        &mut self,
        machine: &mut DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>,
    ) -> Result<(), Error> {
        match self.file {
            Some(ref mut data) => {
                data.write_all(format!("{}", machine).as_bytes())?;
            }
            None => {
                let mut data = std::fs::File::create(&self.name).unwrap();
                data.write_all(format!("{}", machine).as_bytes())?;
                self.file = Some(data)
            }
        }
        Ok(())
    }
}

pub struct MachineCoverage {
    addrctx: Addr2LineContext,
    pc_dict: HashMap<u64, u8>,
    results: HashMap<String, Vec<u8>>,
}

impl MachineCoverage {
    pub fn new(program: &Bytes) -> Result<Self, Box<dyn std::error::Error>> {
        let object = addr2line::object::File::parse(program.as_ref())?;
        let ctx = addr2line::Context::new(&object)?;
        Ok(Self { addrctx: ctx, pc_dict: HashMap::new(), results: HashMap::new() })
    }

    pub fn step(
        &mut self,
        machine: &mut DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>,
    ) -> Result<(), Error> {
        let pc = machine.pc().to_u64();
        if self.pc_dict.get(&pc).map_or(0, |v| *v) != 0 {
            return Ok(());
        }
        self.pc_dict.insert(pc, 1);

        let location = self.addrctx.find_location(pc).unwrap();
        if location.is_none() {
            return Ok(());
        }
        let location = location.unwrap();
        let file = location.file.unwrap().to_string();
        let line = location.line;
        if line.is_none() {
            return Ok(());
        }
        let line = line.unwrap() as usize;
        assert!(line > 0);
        if !self.results.contains_key(&file) {
            self.results.insert(file.clone(), vec![]);
        }
        let list = self.results.get_mut(&file).unwrap();
        if line > list.len() {
            list.resize(line, 0);
        }
        list[line - 1] = 1;
        return Ok(());
    }
}

impl MachineCoverage {
    pub fn display_lcov(&mut self, writer: &mut impl std::io::Write) -> Result<(), Box<dyn std::error::Error>> {
        // The lcov file format is a simple, text-based format used to store code coverage data generated by tools like
        // gcov. It's typically used to represent line-level and function-level coverage, and branch coverage in some
        // cases.
        //
        // Here's a breakdown of the format:
        //
        // TN: Test Name. Identifies the test case being reported.
        // SF: Source File. Identifies the path of the source file.
        // FN: Function Name. Represents a function.
        // FNDA: Function Data. Indicates which functions are called.
        // FNF: Function Found. Total number of functions found in the file.
        // FNH: Function Hit. Total number of functions executed.
        // DA: Data/Execution Count. Represents line numbers and their execution counts.
        // LH: Lines Hit. The total number of lines with a non-zero execution count.
        // LF: Lines Found. The total number of instrumented lines.
        // BRDA: Branch Data. Specifies the branch numbers and their execution counts.
        // BRF: Branch Found. The total number of branches.
        // BRH: Branch Hit. The total number of branches executed.
        // end_of_record: Marks the end of a file section.
        for (name, list) in &self.results {
            if !std::fs::exists(name).unwrap() {
                continue;
            }
            writeln!(writer, "SF:{}", name)?;
            for (i, hit) in list.iter().enumerate() {
                writeln!(writer, "DA:{},{}", i + 1, *hit)?;
            }
            let lh = list.iter().filter(|&e| *e != 0).count();
            let lf = std::io::BufReader::new(std::fs::File::open(name)?).lines().count();
            for i in list.len()..lf {
                writeln!(writer, "DA:{},{}", i + 1, 0)?;
            }
            writeln!(writer, "LH:{}", lf)?;
            writeln!(writer, "LF:{}", lh)?;
            writeln!(writer, "end_of_record")?;
        }
        writer.flush()?;
        Ok(())
    }
}

pub struct MachineAnalyzer {
    pub enable_coverage: u8,
    pub enable_flamegraph: u8,
    pub enable_overlap: u8,
    pub enable_steplog: u8,
    pub machine: DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>,
    pub coverage: MachineCoverage,
    pub flamegraph: MachineFlamegraph,
    pub overlap: MachineOverlap,
    pub steplog: MachineStepLog,
}

impl CoreMachine for MachineAnalyzer {
    type REG = u64;
    type MEM = WXorXMemory<FlatMemory<u64>>;

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

impl Machine for MachineAnalyzer {
    fn ecall(&mut self) -> Result<(), Error> {
        self.machine.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.machine.ebreak()
    }
}

impl std::fmt::Display for MachineAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.machine.fmt(f)
    }
}

impl MachineAnalyzer {
    pub fn new(
        machine: DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>,
        coverage: MachineCoverage,
        flamegraph: MachineFlamegraph,
        overlap: MachineOverlap,
        steplog: MachineStepLog,
    ) -> Self {
        Self {
            enable_coverage: 0,
            enable_flamegraph: 0,
            enable_overlap: 0,
            enable_steplog: 0,
            machine,
            coverage,
            flamegraph,
            overlap,
            steplog,
        }
    }

    pub fn run(&mut self) -> Result<i8, Error> {
        if self.isa() & ISA_MOP != 0 && self.version() == VERSION0 {
            return Err(Error::InvalidVersion);
        }
        let mut decoder = build_decoder::<u64>(self.isa(), self.version());
        self.machine.set_running(true);
        while self.machine.running() {
            if self.machine.reset_signal() {
                decoder.reset_instructions_cache();
                self.flamegraph = MachineFlamegraph::new(&self.machine.code()).unwrap();
            }
            if self.enable_coverage > 0 {
                self.coverage.step(&mut self.machine)?;
            }
            if self.enable_flamegraph > 0 {
                self.flamegraph.step(&mut decoder, &mut self.machine)?;
            }
            if self.enable_flamegraph > 0 && self.enable_overlap > 0 {
                self.overlap.step(&mut decoder, &mut self.machine, &self.flamegraph)?;
            }
            if self.enable_steplog > 0 {
                self.steplog.step(&mut self.machine)?;
            }
            self.machine.step(&mut decoder)?;
        }
        Ok(self.machine.exit_code())
    }
}
