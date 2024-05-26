use crate::machine_assign::MachineAssign;
use ckb_traits::{CellDataProvider, ExtensionProvider, HeaderProvider};
use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::{
    bytes::Bytes,
    decoder::{build_decoder, Decoder},
    instructions::{execute, extract_opcode, insts},
    machine::{CoreMachine, Machine, SupportMachine},
    registers::A7,
    Error, Memory, Register,
};
use gdbstub::{
    arch::{Arch, SingleStepGdbBehavior},
    common::Signal,
    conn::{Connection, ConnectionExt},
    stub::{
        run_blocking::{BlockingEventLoop, Event, WaitForStopReasonError},
        SingleThreadStopReason,
    },
    target::{
        ext::{
            base::{
                single_register_access::{SingleRegisterAccess, SingleRegisterAccessOps},
                singlethread::{
                    SingleThreadBase, SingleThreadRangeStepping, SingleThreadRangeSteppingOps, SingleThreadResume,
                    SingleThreadResumeOps, SingleThreadSingleStep, SingleThreadSingleStepOps,
                },
                BaseOps,
            },
            breakpoints::{
                Breakpoints, BreakpointsOps, HwWatchpoint, HwWatchpointOps, SwBreakpoint, SwBreakpointOps, WatchKind,
            },
            catch_syscalls::{CatchSyscallPosition, CatchSyscalls, CatchSyscallsOps, SyscallNumbers},
        },
        Target, TargetError, TargetResult,
    },
};
use gdbstub_arch::riscv::reg::id::RiscvRegId;
use gdbstub_arch::riscv::Riscv64;
use std::collections::HashSet;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub enum ExecMode {
    Step,
    Continue,
    RangeStep(u64, u64),
}

pub enum FilteredSyscalls {
    None,
    All,
    Filter(HashSet<u64>),
}

impl FilteredSyscalls {
    pub fn filtered(&self, syscall_number: &u64) -> bool {
        match self {
            FilteredSyscalls::None => false,
            FilteredSyscalls::All => true,
            FilteredSyscalls::Filter(filter) => filter.contains(syscall_number),
        }
    }
}

pub struct GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    pub machine: MachineAssign<DL>,
    exec_mode: ExecMode,
    decoder: Decoder,
    breakpoints: Vec<u64>,
    catch_syscalls: FilteredSyscalls,
    watchpoints: Vec<(u64, WatchKind)>,
    memory_writes: Vec<u64>,
    memory_reads: Vec<u64>,
}

// Note a lot of code in this file is copied over from
// https://github.com/daniel5151/gdbstub/blob/36f166e1aabe47ea2f0508207372e4a302fbac87/examples/armv4t/emu.rs
#[derive(Debug, Clone, PartialEq, Eq)]
enum VmEvent {
    IncomingData,
    DoneStep,
    Exited(u8),
    Break,
    WatchWrite(u64),
    WatchRead(u64),
    CatchSyscall(u64),
    Error(Error),
}

impl<DL> GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    pub fn new(machine: MachineAssign<DL>) -> Self {
        let decoder = build_decoder::<u64>(machine.isa(), machine.version());
        Self {
            machine,
            decoder,
            exec_mode: ExecMode::Continue,
            breakpoints: vec![],
            catch_syscalls: FilteredSyscalls::None,
            watchpoints: vec![],
            memory_writes: vec![],
            memory_reads: vec![],
        }
    }

    fn clear_memory_ops(&mut self) {
        self.memory_writes.clear();
        self.memory_reads.clear();
    }
}

impl<DL> GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    pub fn run_till_exited(mut self) -> Result<(i8, u64), Error> {
        while self.machine.running() {
            self.step_inner()?;
        }
        Ok((self.machine.exit_code(), self.machine.scheduler.consumed_cycles()))
    }

    fn next_opcode(&mut self) -> Option<u16> {
        let pc = self.machine.pc().to_u64();
        let memory = self.machine.memory_mut();
        let inst = self.decoder.decode(memory, pc).ok()?;
        Some(extract_opcode(inst))
    }

    fn step_inner(&mut self) -> Result<(), Error> {
        let instruction = {
            let pc = self.machine.pc().to_u64();
            let memory = self.machine.memory_mut();
            self.decoder.decode(memory, pc)?
        };
        let cycles = estimate_cycles(instruction);
        self.machine.add_cycles(cycles)?;
        self.clear_memory_ops();
        execute(instruction, self)
    }

    fn step(&mut self) -> Option<VmEvent> {
        if self.machine.reset_signal() {
            self.decoder.reset_instructions_cache()
        }
        if !self.machine.running() {
            return Some(VmEvent::Exited(self.machine.exit_code() as u8));
        }
        match self.step_inner() {
            Ok(_) => {
                if let Some(opcode) = self.next_opcode() {
                    if opcode == insts::OP_ECALL {
                        let number = self.machine.registers()[A7].clone();
                        if self.catch_syscalls.filtered(&number) {
                            return Some(VmEvent::CatchSyscall(number));
                        }
                    }
                }
                if self.breakpoints.contains(self.machine.pc()) {
                    return Some(VmEvent::Break);
                }
                if !self.memory_writes.is_empty() {
                    return Some(VmEvent::WatchWrite(self.memory_writes.pop().unwrap()));
                }
                if !self.memory_reads.is_empty() {
                    return Some(VmEvent::WatchRead(self.memory_reads.pop().unwrap()));
                }
                None
            }
            Err(e) => Some(VmEvent::Error(e)),
        }
    }

    fn execute(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> VmEvent {
        if poll_incoming_data() {
            return VmEvent::IncomingData;
        }
        match self.exec_mode.clone() {
            ExecMode::Step => self.step().unwrap_or(VmEvent::DoneStep),
            ExecMode::Continue => {
                let mut executed_cycles = 0;
                loop {
                    if let Some(event) = self.step() {
                        break event;
                    }

                    executed_cycles += 1;
                    if executed_cycles % 1024 == 0 && poll_incoming_data() {
                        break VmEvent::IncomingData;
                    }
                }
            }
            ExecMode::RangeStep(start, end) => {
                let mut executed_cycles = 0;
                loop {
                    if let Some(event) = self.step() {
                        break event;
                    }

                    if !(start.to_u64()..end.to_u64()).contains(&self.machine.pc().to_u64()) {
                        break VmEvent::DoneStep;
                    }

                    executed_cycles += 1;
                    if executed_cycles % 1024 == 0 && poll_incoming_data() {
                        break VmEvent::IncomingData;
                    }
                }
            }
        }
    }
}

impl<DL> Target for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    type Arch = Riscv64;
    type Error = Error;

    fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }

    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }

    fn support_catch_syscalls(&mut self) -> Option<CatchSyscallsOps<'_, Self>> {
        Some(self)
    }

    fn guard_rail_single_step_gdb_behavior(&self) -> SingleStepGdbBehavior {
        SingleStepGdbBehavior::Optional
    }
}

impl<DL> SingleThreadBase for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn read_registers(&mut self, regs: &mut <Self::Arch as Arch>::Registers) -> TargetResult<(), Self> {
        for (i, val) in self.machine.registers().iter().enumerate() {
            regs.x[i] = val.clone().to_u64();
        }
        regs.pc = self.machine.pc().clone().to_u64();
        Ok(())
    }

    fn write_registers(&mut self, regs: &<Self::Arch as Arch>::Registers) -> TargetResult<(), Self> {
        regs.x.iter().enumerate().for_each(|(i, val)| {
            self.machine.set_register(i, val.clone());
        });
        self.machine.update_pc(regs.pc.clone());
        self.machine.commit_pc();
        Ok(())
    }

    fn read_addrs(&mut self, start_addr: <Self::Arch as Arch>::Usize, data: &mut [u8]) -> TargetResult<(), Self> {
        for i in 0..data.len() {
            data[i] =
                self.machine.memory_mut().load8(&(start_addr.to_u64() + i as u64)).map_err(TargetError::Fatal)?.to_u8();
        }
        Ok(())
    }

    fn write_addrs(&mut self, start_addr: <Self::Arch as Arch>::Usize, data: &[u8]) -> TargetResult<(), Self> {
        self.machine.memory_mut().store_bytes(start_addr.to_u64(), data).map_err(TargetError::Fatal)
    }

    fn support_single_register_access(&mut self) -> Option<SingleRegisterAccessOps<'_, (), Self>> {
        Some(self)
    }

    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl<DL> SingleRegisterAccess<()> for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn read_register(
        &mut self,
        _tid: (),
        reg_id: <Self::Arch as Arch>::RegId,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let value = match reg_id {
            RiscvRegId::Pc => self.machine.pc(),
            RiscvRegId::Gpr(idx) => &self.machine.registers()[idx as usize],
            _ => return Err(TargetError::Fatal(Error::External(format!("Invalid register id: {:?}", reg_id)))),
        };
        buf.copy_from_slice(&value.to_u64().to_le_bytes()[0..(u64::BITS as usize / 8)]);
        Ok(buf.len())
    }

    fn write_register(&mut self, _tid: (), reg_id: <Self::Arch as Arch>::RegId, val: &[u8]) -> TargetResult<(), Self> {
        let mut u64_buf = [0u8; 8];
        u64_buf[0..val.len()].copy_from_slice(val);
        let v = u64::from_le_bytes(u64_buf);
        match reg_id {
            RiscvRegId::Pc => {
                self.machine.update_pc(v);
                self.machine.commit_pc();
            }
            RiscvRegId::Gpr(idx) => {
                self.machine.set_register(idx as usize, v);
            }
            _ => return Err(TargetError::Fatal(Error::External(format!("Invalid register id: {:?}", reg_id)))),
        };

        Ok(())
    }
}

// This is only for setting execution modes, the actual execution shall live within
// BlockingEventLoop trait impl.
impl<DL> SingleThreadResume for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        if signal.is_some() {
            return Err(Error::External("no support for continuing with signal".to_string()));
        }
        self.exec_mode = ExecMode::Continue;
        Ok(())
    }

    fn support_single_step(&mut self) -> Option<SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }

    fn support_range_step(&mut self) -> Option<SingleThreadRangeSteppingOps<'_, Self>> {
        Some(self)
    }
}

impl<DL> SingleThreadRangeStepping for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn resume_range_step(&mut self, start: u64, end: u64) -> Result<(), Self::Error> {
        self.exec_mode = ExecMode::RangeStep(start, end);
        Ok(())
    }
}

impl<DL> SingleThreadSingleStep for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        if signal.is_some() {
            return Err(Error::External("no support for stepping with signal".to_string()));
        }
        self.exec_mode = ExecMode::Step;
        Ok(())
    }
}

impl<DL> Breakpoints for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }

    fn support_hw_watchpoint(&mut self) -> Option<HwWatchpointOps<'_, Self>> {
        Some(self)
    }
}

impl<DL> SwBreakpoint for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn add_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        _kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        self.breakpoints.push(addr);
        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        _kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        match self.breakpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => self.breakpoints.remove(pos),
        };

        Ok(true)
    }
}

impl<DL> HwWatchpoint for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn add_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        _len: <Self::Arch as Arch>::Usize,
        kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        self.watchpoints.push((addr, kind));
        Ok(true)
    }

    fn remove_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        _len: <Self::Arch as Arch>::Usize,
        kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        match self.watchpoints.iter().position(|(a, k)| *a == addr && *k == kind) {
            None => return Ok(false),
            Some(pos) => self.breakpoints.remove(pos),
        };

        Ok(true)
    }
}

impl<DL> CatchSyscalls for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn enable_catch_syscalls(
        &mut self,
        filter: Option<SyscallNumbers<'_, <Self::Arch as Arch>::Usize>>,
    ) -> TargetResult<(), Self> {
        self.catch_syscalls = match filter {
            Some(numbers) => FilteredSyscalls::Filter(numbers.collect()),
            None => FilteredSyscalls::All,
        };
        Ok(())
    }

    fn disable_catch_syscalls(&mut self) -> TargetResult<(), Self> {
        self.catch_syscalls = FilteredSyscalls::None;
        Ok(())
    }
}

#[derive(Default)]
pub struct GdbStubHandlerEventLoop<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    dl: PhantomData<DL>,
}

impl<DL> BlockingEventLoop for GdbStubHandlerEventLoop<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    type Target = GdbStubHandler<DL>;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;
    type StopReason = SingleThreadStopReason<u64>;

    fn on_interrupt(
        _target: &mut Self::Target,
    ) -> Result<Option<SingleThreadStopReason<u64>>, <GdbStubHandler<DL> as Target>::Error> {
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT)))
    }

    #[allow(clippy::type_complexity)]
    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<
        Event<SingleThreadStopReason<u64>>,
        WaitForStopReasonError<<Self::Target as Target>::Error, <Self::Connection as Connection>::Error>,
    > {
        let poll_incoming_data = || conn.peek().map(|b| b.is_some()).unwrap_or(true);

        Ok(match target.execute(poll_incoming_data) {
            VmEvent::IncomingData => {
                let byte = conn.read().map_err(WaitForStopReasonError::Connection)?;
                Event::IncomingData(byte)
            }
            VmEvent::DoneStep => Event::TargetStopped(SingleThreadStopReason::DoneStep),
            VmEvent::Exited(code) => Event::TargetStopped(SingleThreadStopReason::Exited(code)),
            VmEvent::Break => Event::TargetStopped(SingleThreadStopReason::SwBreak(())),
            VmEvent::WatchRead(addr) => Event::TargetStopped(SingleThreadStopReason::Watch {
                tid: (),
                kind: WatchKind::Read,
                addr: addr.to_u64(),
            }),
            VmEvent::WatchWrite(addr) => Event::TargetStopped(SingleThreadStopReason::Watch {
                tid: (),
                kind: WatchKind::Write,
                addr: addr.to_u64(),
            }),
            VmEvent::CatchSyscall(number) => Event::TargetStopped(SingleThreadStopReason::CatchSyscall {
                tid: None,
                number: number.to_u64(),
                position: CatchSyscallPosition::Entry,
            }),
            VmEvent::Error(e) => return Err(WaitForStopReasonError::Target(e)),
        })
    }
}

impl<DL> Memory for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    type REG = u64;

    fn new() -> Self {
        todo!()
    }

    fn new_with_memory(_: usize) -> Self {
        todo!()
    }

    fn memory_size(&self) -> usize {
        self.machine.memory().memory_size()
    }

    fn load_bytes(&mut self, addr: u64, size: u64) -> Result<Bytes, ckb_vm::Error> {
        self.machine.memory_mut().load_bytes(addr, size)
    }

    fn lr(&self) -> &<Self as Memory>::REG {
        self.machine.memory().lr()
    }

    fn set_lr(&mut self, addr: &<Self as Memory>::REG) {
        self.machine.memory_mut().set_lr(addr)
    }

    fn init_pages(
        &mut self,
        addr: u64,
        size: u64,
        flags: u8,
        source: Option<Bytes>,
        offset_from_addr: u64,
    ) -> Result<(), Error> {
        self.machine.memory_mut().init_pages(addr, size, flags, source, offset_from_addr)
    }

    fn fetch_flag(&mut self, page: u64) -> Result<u8, Error> {
        self.machine.memory_mut().fetch_flag(page)
    }

    fn set_flag(&mut self, page: u64, flag: u8) -> Result<(), Error> {
        self.machine.memory_mut().set_flag(page, flag)
    }

    fn clear_flag(&mut self, page: u64, flag: u8) -> Result<(), Error> {
        self.machine.memory_mut().clear_flag(page, flag)
    }

    fn store_byte(&mut self, addr: u64, size: u64, value: u8) -> Result<(), Error> {
        self.machine.memory_mut().store_byte(addr, size, value)
    }

    fn store_bytes(&mut self, addr: u64, value: &[u8]) -> Result<(), Error> {
        self.machine.memory_mut().store_bytes(addr, value)
    }

    fn execute_load16(&mut self, addr: u64) -> Result<u16, Error> {
        self.machine.memory_mut().execute_load16(addr)
    }

    fn execute_load32(&mut self, addr: u64) -> Result<u32, Error> {
        self.machine.memory_mut().execute_load32(addr)
    }

    fn load8(&mut self, addr: &Self::REG) -> Result<Self::REG, Error> {
        let result = self.machine.memory_mut().load8(addr)?;
        self.memory_reads.push(addr.clone());
        Ok(result)
    }

    fn load16(&mut self, addr: &Self::REG) -> Result<Self::REG, Error> {
        let result = self.machine.memory_mut().load16(addr)?;
        self.memory_reads.push(addr.clone());
        Ok(result)
    }

    fn load32(&mut self, addr: &Self::REG) -> Result<Self::REG, Error> {
        let result = self.machine.memory_mut().load32(addr)?;
        self.memory_reads.push(addr.clone());
        Ok(result)
    }

    fn load64(&mut self, addr: &Self::REG) -> Result<Self::REG, Error> {
        let result = self.machine.memory_mut().load64(addr)?;
        self.memory_reads.push(addr.clone());
        Ok(result)
    }

    fn store8(&mut self, addr: &Self::REG, value: &Self::REG) -> Result<(), Error> {
        self.machine.memory_mut().store8(addr, value)?;
        self.memory_writes.push(addr.clone());
        Ok(())
    }

    fn store16(&mut self, addr: &Self::REG, value: &Self::REG) -> Result<(), Error> {
        self.machine.memory_mut().store16(addr, value)?;
        self.memory_writes.push(addr.clone());
        Ok(())
    }

    fn store32(&mut self, addr: &Self::REG, value: &Self::REG) -> Result<(), Error> {
        self.machine.memory_mut().store32(addr, value)?;
        self.memory_writes.push(addr.clone());
        Ok(())
    }

    fn store64(&mut self, addr: &Self::REG, value: &Self::REG) -> Result<(), Error> {
        self.machine.memory_mut().store64(addr, value)?;
        self.memory_writes.push(addr.clone());
        Ok(())
    }
}

impl<DL> CoreMachine for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    type REG = u64;
    type MEM = Self;

    fn pc(&self) -> &Self::REG {
        self.machine.pc()
    }

    fn update_pc(&mut self, pc: Self::REG) {
        self.machine.update_pc(pc)
    }

    fn commit_pc(&mut self) {
        self.machine.commit_pc()
    }

    fn memory(&self) -> &Self::MEM {
        self
    }

    fn memory_mut(&mut self) -> &mut Self::MEM {
        self
    }

    fn registers(&self) -> &[Self::REG] {
        self.machine.registers()
    }

    fn set_register(&mut self, idx: usize, value: Self::REG) {
        self.machine.set_register(idx, value)
    }

    fn version(&self) -> u32 {
        self.machine.version()
    }

    fn isa(&self) -> u8 {
        self.machine.isa()
    }
}

impl<DL> Machine for GdbStubHandler<DL>
where
    DL: CellDataProvider + HeaderProvider + ExtensionProvider + Send + Sync + Clone + 'static,
{
    fn ecall(&mut self) -> Result<(), Error> {
        self.machine.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.machine.ebreak()
    }
}
