use ckb_script::{CoreMachine as CkbScriptCoreMachineType, DataLocation, ROOT_VM_ID, RunMode, Scheduler, VmArgs};
use ckb_traits::{CellDataProvider, ExtensionProvider, HeaderProvider};
use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::decoder::Decoder;
use ckb_vm::instructions::execute;
use ckb_vm::registers::A7;
use ckb_vm::{Bytes, CoreMachine, Error, Machine, SupportMachine, Syscalls};

pub struct MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    pub id: u64,
    pub scheduler: Scheduler<DL>,
    pub expand_cycles: u64,
    pub expand_syscalls: Vec<Box<(dyn Syscalls<CkbScriptCoreMachineType>)>>,
}

impl<DL> CoreMachine for MachineAssign<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    type REG = u64;
    type MEM = <CkbScriptCoreMachineType as CoreMachine>::MEM;

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
        let sid = dm.registers()[A7];
        if result == Err(Error::Yield) {
            let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
            dm.set_cycles(0);
            self.scheduler.iteration_cycles =
                self.scheduler.iteration_cycles.checked_add(cycles).ok_or(Error::CyclesExceeded)?;
            self.scheduler.iterate_process_results(self.id, Err(Error::Yield))?;
            self.consume_cycles(self.scheduler.iteration_cycles)?;

            let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
            let next_pc = dm.inner_mut().pc() + 4;

            self.wait()?;

            let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
            dm.inner_mut().update_pc(next_pc);
            return Ok(());
        }
        if sid == 93 {
            let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
            dm.set_cycles(0);
            self.scheduler.iteration_cycles =
                self.scheduler.iteration_cycles.checked_add(cycles).ok_or(Error::CyclesExceeded)?;
            self.consume_cycles(self.scheduler.iteration_cycles)?;
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
    pub fn new(id: u64, args: &[Bytes], scheduler: Scheduler<DL>) -> Result<Self, Error> {
        let mut r = Self { id, scheduler, expand_cycles: u64::MAX, expand_syscalls: vec![] };
        if r.scheduler.states.is_empty() {
            let location = DataLocation {
                data_piece_id: r.scheduler.sg_data.sg_info.program_data_piece_id.clone(),
                offset: 0,
                length: u64::MAX,
            };
            assert_eq!(r.scheduler.boot_vm(&location, VmArgs::Vector(args.to_vec()))?, ROOT_VM_ID);
        }
        Ok(r)
    }

    pub fn consume_cycles(&mut self, cycles: u64) -> Result<(), Error> {
        self.scheduler.consume_cycles(cycles)?;
        self.scheduler.iteration_cycles = 0;
        self.expand_cycles = self.expand_cycles.checked_sub(cycles).ok_or(Error::CyclesExceeded)?;
        Ok(())
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
            let im = self.scheduler.iterate_prepare_machine()?;
            let id = im.0;
            let vm = im.1;
            vm.set_max_cycles(self.expand_cycles);
            if self.id == id {
                vm.machine.set_running(true);
                break;
            }
            let result = vm.run();
            let cycles = vm.machine.cycles();
            vm.machine.set_cycles(0);
            self.scheduler.iteration_cycles =
                self.scheduler.iteration_cycles.checked_add(cycles).ok_or(Error::CyclesExceeded)?;
            self.scheduler.iterate_process_results(id, result)?;
            self.consume_cycles(self.scheduler.iteration_cycles)?;
        }
        Ok(())
    }

    pub fn done(&mut self) -> Result<(), Error> {
        let dm = &mut self.scheduler.instantiated.get_mut(&self.id).unwrap().1.machine;
        let dmexit = dm.exit_code();
        self.scheduler.iterate_process_results(self.id, Ok(dmexit))?;
        self.scheduler.run(RunMode::LimitCycles(self.expand_cycles))?;
        return Ok(());
    }
}
