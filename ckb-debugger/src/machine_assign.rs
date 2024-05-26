use ckb_script::{DataPieceId, RunMode, Scheduler, ROOT_VM_ID};
use ckb_traits::{CellDataProvider, ExtensionProvider, HeaderProvider};
use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::decoder::Decoder;
use ckb_vm::instructions::execute;
use ckb_vm::machine::Pause;
use ckb_vm::registers::A7;
use ckb_vm::{
    Bytes, CoreMachine, DefaultCoreMachine, Error, FlatMemory, Machine, SupportMachine, Syscalls, WXorXMemory,
};

pub struct MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    pub id: u64,
    pub scheduler: Scheduler<DL>,
    pub expand_cycles: u64,
    pub expand_syscalls: Vec<Box<(dyn Syscalls<DefaultCoreMachine<u64, WXorXMemory<FlatMemory<u64>>>>)>>,
}

impl<DL> CoreMachine for MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    type REG = u64;
    type MEM = WXorXMemory<FlatMemory<u64>>;

    fn pc(&self) -> &Self::REG {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.pc()
    }

    fn update_pc(&mut self, pc: Self::REG) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.update_pc(pc)
    }

    fn commit_pc(&mut self) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.commit_pc()
    }

    fn memory(&self) -> &Self::MEM {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.memory()
    }

    fn memory_mut(&mut self) -> &mut Self::MEM {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.memory_mut()
    }

    fn registers(&self) -> &[Self::REG] {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.registers()
    }

    fn set_register(&mut self, idx: usize, value: Self::REG) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.set_register(idx, value)
    }

    fn version(&self) -> u32 {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.version()
    }

    fn isa(&self) -> u8 {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.isa()
    }
}
impl<DL> SupportMachine for MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn cycles(&self) -> u64 {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.cycles()
    }

    fn set_cycles(&mut self, cycles: u64) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.set_cycles(cycles)
    }

    fn max_cycles(&self) -> u64 {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.max_cycles()
    }

    fn set_max_cycles(&mut self, cycles: u64) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.set_max_cycles(cycles);
    }

    fn running(&self) -> bool {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.running()
    }

    fn set_running(&mut self, running: bool) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.set_running(running)
    }

    fn reset(&mut self, max_cycles: u64) {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.reset(max_cycles)
    }

    fn reset_signal(&mut self) -> bool {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.reset_signal()
    }

    fn code(&self) -> &Bytes {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.code()
    }
}

impl<DL> Machine for MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn ecall(&mut self) -> Result<(), Error> {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        for i in 0..self.expand_syscalls.len() {
            if self.expand_syscalls[i].ecall(dm.inner_mut())? {
                return Ok(());
            }
        }
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        let result = dm.ecall();
        let cycles = dm.cycles();
        if result == Err(Error::Yield) {
            dm.set_cycles(0);
            self.scheduler.iterate_process_results(self.id, Err(Error::Yield), cycles)?;
            self.scheduler.consumed_cycles_add(self.scheduler.current_iteration_cycles)?;
            self.wait()?;
            return Ok(());
        }
        if dm.registers()[A7] == 93 {
            dm.set_cycles(0);
            self.scheduler.consumed_cycles_add(cycles)?;
            return Ok(());
        }
        result
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        dm.ebreak()
    }
}

impl<DL> std::fmt::Display for MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dm = &self.scheduler.instantiated.get(&self.id).unwrap().1.machine;
        dm.fmt(f)
    }
}

impl<DL> MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    pub fn new(id: u64, scheduler: Scheduler<DL>) -> Result<Self, Error> {
        let mut r = Self { id: id, scheduler: scheduler, expand_cycles: u64::MAX, expand_syscalls: vec![] };
        if r.scheduler.states.is_empty() {
            assert_eq!(r.scheduler.boot_vm(&DataPieceId::Program, 0, u64::MAX, &[])?, ROOT_VM_ID);
        }
        Ok(r)
    }

    pub fn exit_code(&self) -> i8 {
        let root_vm = &self.scheduler.instantiated[&ROOT_VM_ID];
        root_vm.1.machine.exit_code()
    }

    pub fn step(&mut self, decoder: &mut Decoder) -> Result<(), Error> {
        let instruction = {
            let pc = *self.pc();
            let memory = self.memory_mut();
            decoder.decode(memory, pc)?
        };
        let cycles = estimate_cycles(instruction);
        self.add_cycles(cycles)?;
        execute(instruction, self)
    }

    pub fn wait(&mut self) -> Result<(), Error> {
        loop {
            self.scheduler.current_iteration_cycles = 0;
            let im = self.scheduler.iterate_prepare_machine(Pause::new(), self.expand_cycles)?;
            let id = im.0;
            let vm = im.1;
            if self.id == id {
                break;
            }
            let result = vm.run();
            let cycles = vm.machine.cycles();
            vm.machine.set_cycles(0);
            self.scheduler.iterate_process_results(id, result, cycles)?;
            self.scheduler.consumed_cycles_add(self.scheduler.current_iteration_cycles)?;
            self.expand_cycles = self.expand_cycles.checked_sub(self.scheduler.current_iteration_cycles).unwrap();
        }
        Ok(())
    }

    pub fn done(&mut self) -> Result<(), Error> {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        let dmexit = dm.exit_code();
        self.scheduler.iterate_process_results(self.id, Ok(dmexit), 0)?;
        self.scheduler.consumed_cycles_add(self.scheduler.current_iteration_cycles)?;
        self.scheduler.run(RunMode::LimitCycles(self.expand_cycles))?;
        return Ok(());
    }
}
