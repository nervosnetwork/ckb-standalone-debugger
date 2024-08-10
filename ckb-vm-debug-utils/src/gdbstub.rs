use ckb_vm::{
    decoder::{build_decoder, Decoder},
    instructions::{execute, extract_opcode, insts},
    machine::{CoreMachine, DefaultMachine, Machine, SupportMachine},
    registers::A7,
    Bytes, Error, Memory, Register,
};
use gdbstub::{
    arch::Arch,
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
use gdbstub_arch::riscv::reg::{id::RiscvRegId, RiscvCoreRegs};
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash as StdHash;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub enum ExecMode<R: Register> {
    Step,
    Continue,
    RangeStep(R, R),
}

pub enum FilteredSyscalls<R> {
    None,
    All,
    Filter(HashSet<R>),
}

impl<R: StdHash + Eq> FilteredSyscalls<R> {
    pub fn filtered(&self, syscall_number: &R) -> bool {
        match self {
            FilteredSyscalls::None => false,
            FilteredSyscalls::All => true,
            FilteredSyscalls::Filter(filter) => filter.contains(syscall_number),
        }
    }
}

pub struct GdbStubHandler<M: SupportMachine, A> {
    exec_mode: ExecMode<M::REG>,
    machine: DefaultMachine<M>,
    decoder: Decoder,
    breakpoints: Vec<M::REG>,
    catch_syscalls: FilteredSyscalls<M::REG>,
    watchpoints: Vec<(M::REG, WatchKind)>,
    memory_writes: Vec<M::REG>,
    memory_reads: Vec<M::REG>,
    _arch: PhantomData<A>,
}

// Note a lot of code in this file is copied over from
// https://github.com/daniel5151/gdbstub/blob/36f166e1aabe47ea2f0508207372e4a302fbac87/examples/armv4t/emu.rs
#[derive(Debug, Clone, PartialEq, Eq)]
enum VmEvent<R: Register> {
    IncomingData,
    DoneStep,
    Exited(u8),
    Break,
    WatchWrite(R),
    WatchRead(R),
    CatchSyscall(R),
    Error(Error),
}

impl<R: Register, M: SupportMachine + CoreMachine<REG = R>, A: Arch<Usize = R>> GdbStubHandler<M, A> {
    pub fn new(machine: DefaultMachine<M>) -> Self {
        let decoder = build_decoder::<M::REG>(machine.isa(), machine.version());
        Self {
            machine,
            decoder,
            exec_mode: ExecMode::Continue,
            breakpoints: vec![],
            catch_syscalls: FilteredSyscalls::None,
            watchpoints: vec![],
            memory_writes: vec![],
            memory_reads: vec![],
            _arch: PhantomData,
        }
    }

    fn clear_memory_ops(&mut self) {
        self.memory_writes.clear();
        self.memory_reads.clear();
    }
}

impl<R: Register + Debug + Eq + StdHash, M: SupportMachine + CoreMachine<REG = R>, A: Arch<Usize = R>>
    GdbStubHandler<M, A>
{
    pub fn run_till_exited(mut self) -> Result<(i8, u64), Error> {
        while self.machine.running() {
            self.step_inner()?;
        }
        Ok((self.machine.exit_code(), self.machine.cycles()))
    }

    fn next_opcode(&mut self) -> Option<u16> {
        let pc = self.machine.inner_mut().pc().to_u64();
        let memory = self.machine.inner_mut().memory_mut();
        let inst = self.decoder.decode(memory, pc).ok()?;
        Some(extract_opcode(inst))
    }

    fn step_inner(&mut self) -> Result<(), Error> {
        let instruction = {
            let pc = self.machine.inner_mut().pc().to_u64();
            let memory = self.machine.inner_mut().memory_mut();
            self.decoder.decode(memory, pc)?
        };
        let cycles = self.machine.instruction_cycle_func()(instruction);
        self.machine.add_cycles(cycles)?;
        self.clear_memory_ops();
        execute(instruction, self)
    }

    fn step(&mut self) -> Option<VmEvent<M::REG>> {
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
                        let number = self.machine.inner_mut().registers()[A7].clone();
                        if self.catch_syscalls.filtered(&number) {
                            return Some(VmEvent::CatchSyscall(number));
                        }
                    }
                }
                if self.breakpoints.contains(self.machine.inner_mut().pc()) {
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

    fn execute(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> VmEvent<M::REG> {
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

                    if !(start.to_u64()..end.to_u64()).contains(&self.machine.inner_mut().pc().to_u64()) {
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

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > Target for GdbStubHandler<M, A>
{
    type Arch = A;
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
}

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > SingleThreadBase for GdbStubHandler<M, A>
{
    fn read_registers(&mut self, regs: &mut <Self::Arch as Arch>::Registers) -> TargetResult<(), Self> {
        for (i, val) in self.machine.registers().iter().enumerate() {
            regs.x[i] = val.clone();
        }
        regs.pc = self.machine.pc().clone();
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

    fn read_addrs(&mut self, start_addr: <Self::Arch as Arch>::Usize, data: &mut [u8]) -> TargetResult<usize, Self> {
        for i in 0..data.len() {
            data[i] = self
                .machine
                .memory_mut()
                .load8(&M::REG::from_u64(start_addr.to_u64() + i as u64))
                .map_err(TargetError::Fatal)?
                .to_u8();
        }
        Ok(data.len())
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

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > SingleRegisterAccess<()> for GdbStubHandler<M, A>
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
        buf.copy_from_slice(&value.to_u64().to_le_bytes()[0..(R::BITS as usize / 8)]);
        Ok(buf.len())
    }

    fn write_register(&mut self, _tid: (), reg_id: <Self::Arch as Arch>::RegId, val: &[u8]) -> TargetResult<(), Self> {
        let mut u64_buf = [0u8; 8];
        u64_buf[0..val.len()].copy_from_slice(val);
        let v = R::from_u64(u64::from_le_bytes(u64_buf));
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
impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > SingleThreadResume for GdbStubHandler<M, A>
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

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > SingleThreadRangeStepping for GdbStubHandler<M, A>
{
    fn resume_range_step(
        &mut self,
        start: <Self::Arch as Arch>::Usize,
        end: <Self::Arch as Arch>::Usize,
    ) -> Result<(), Self::Error> {
        self.exec_mode = ExecMode::RangeStep(start, end);
        Ok(())
    }
}

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > SingleThreadSingleStep for GdbStubHandler<M, A>
{
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        if signal.is_some() {
            return Err(Error::External("no support for stepping with signal".to_string()));
        }
        self.exec_mode = ExecMode::Step;
        Ok(())
    }
}

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > Breakpoints for GdbStubHandler<M, A>
{
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }

    fn support_hw_watchpoint(&mut self) -> Option<HwWatchpointOps<'_, Self>> {
        Some(self)
    }
}

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > SwBreakpoint for GdbStubHandler<M, A>
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

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > HwWatchpoint for GdbStubHandler<M, A>
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

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > CatchSyscalls for GdbStubHandler<M, A>
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
pub struct GdbStubHandlerEventLoop<M, A> {
    _machine: PhantomData<M>,
    _arch: PhantomData<A>,
}

impl<
        R: Register + Debug + Eq + StdHash,
        M: SupportMachine + CoreMachine<REG = R>,
        A: Arch<Usize = R, Registers = RiscvCoreRegs<R>, RegId = RiscvRegId<R>>,
    > BlockingEventLoop for GdbStubHandlerEventLoop<M, A>
{
    type Target = GdbStubHandler<M, A>;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;
    type StopReason = SingleThreadStopReason<A::Usize>;

    fn on_interrupt(
        _target: &mut Self::Target,
    ) -> Result<Option<SingleThreadStopReason<A::Usize>>, <GdbStubHandler<M, A> as Target>::Error> {
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT)))
    }

    #[allow(clippy::type_complexity)]
    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<
        Event<SingleThreadStopReason<A::Usize>>,
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
            VmEvent::WatchRead(addr) => {
                Event::TargetStopped(SingleThreadStopReason::Watch { tid: (), kind: WatchKind::Read, addr })
            }
            VmEvent::WatchWrite(addr) => {
                Event::TargetStopped(SingleThreadStopReason::Watch { tid: (), kind: WatchKind::Write, addr })
            }
            VmEvent::CatchSyscall(number) => Event::TargetStopped(SingleThreadStopReason::CatchSyscall {
                tid: None,
                number,
                position: CatchSyscallPosition::Entry,
            }),
            VmEvent::Error(e) => return Err(WaitForStopReasonError::Target(e)),
        })
    }
}

impl<R: Register, M: SupportMachine + CoreMachine<REG = R>, A> Memory for GdbStubHandler<M, A> {
    type REG = R;

    fn new() -> Self {
        todo!()
    }

    fn new_with_memory(_: usize) -> Self {
        todo!()
    }

    fn memory_size(&self) -> usize {
        self.machine.memory().memory_size()
    }

    fn load_bytes(&mut self, addr: u64, size: u64) -> Result<bytes::Bytes, ckb_vm::Error> {
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

impl<R: Register, M: SupportMachine + CoreMachine<REG = R>, A> CoreMachine for GdbStubHandler<M, A> {
    type REG = R;
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

impl<R: Register, M: SupportMachine + CoreMachine<REG = R>, A> Machine for GdbStubHandler<M, A> {
    fn ecall(&mut self) -> Result<(), Error> {
        self.machine.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.machine.ebreak()
    }
}
