mod frames;
mod timer;

#[macro_use]
extern crate lazy_static;

use crate::{
    frames::{Frame, Report, Symbol},
    timer::Timer,
};
use addr2line::gimli::{self, Error as GimliError, RegisterRule, RiscV, UnwindSection};
use ckb_vm::{machine::asm::AsmMachine, Bytes, CoreMachine, Memory};
use log::trace;
use nix::sys::signal;
use protobuf::Message;
use std::borrow::Cow;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_int;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

pub fn is_profiler_started() -> bool {
    PROFILER.lock().expect("Mutex lock failure").is_some()
}

pub fn start_profiler(
    fname: &str,
    machine: &Pin<Box<AsmMachine>>,
    program: &Bytes,
    frequency_per_sec: i32,
) -> Result<(), String> {
    if is_profiler_started() {
        return Err("Profiler already started!".to_string());
    }

    let context = build_context(program)?;

    // install signal handler
    let handler = signal::SigHandler::Handler(perf_signal_handler);
    let sigaction = signal::SigAction::new(handler, signal::SaFlags::SA_RESTART, signal::SigSet::empty());
    unsafe { signal::sigaction(signal::SIGPROF, &sigaction) }.map_err(|e| format!("sigaction install error: {}", e))?;

    let profiler = Profiler {
        fname: fname.to_string(),
        machine: machine.deref() as *const AsmMachine as usize,
        context,
        timer: Timer::new(frequency_per_sec),
        report: Report::default(),
    };

    *(PROFILER.lock().expect("Mutex lock failure")) = Some(profiler);

    Ok(())
}

pub fn stop_profiler() -> Result<(), String> {
    let mut profiler = PROFILER.lock().expect("Mutex lock failure");
    if profiler.is_none() {
        return Err("Profiler not started!".to_string());
    }
    // save profiled data
    let inner_profiler = profiler.deref().as_ref().unwrap();
    let fname = &inner_profiler.fname;
    let timing = inner_profiler.timer.timing();
    let profile_data = inner_profiler.report.pprof(timing).expect("pprof serialization");
    let data = profile_data.write_to_bytes().expect("protobuf serialization");
    fs::write(fname, data).expect("write");

    // uninstall signal handler
    let handler = signal::SigHandler::SigIgn;
    unsafe { signal::signal(signal::SIGPROF, handler) }.map_err(|e| format!("sigaction uninstall error: {}", e))?;

    *profiler = None;

    Ok(())
}

lazy_static! {
    static ref PROFILER: Mutex<Option<Profiler>> = Mutex::new(None);
}

type Addr2LineEndianReader = gimli::EndianReader<gimli::RunTimeEndian, Arc<[u8]>>;
type Addr2LineContext = addr2line::Context<Addr2LineEndianReader>;
type Addr2LineFrameIter<'a> = addr2line::FrameIter<'a, Addr2LineEndianReader>;

struct DebugContext {
    addr_context: Addr2LineContext,
    debug_frame: gimli::DebugFrame<Addr2LineEndianReader>,
}

struct Profiler {
    fname: String,
    machine: usize,
    context: DebugContext,
    // Drop behavior is enough for timer
    #[allow(dead_code)]
    timer: Timer,
    report: Report,
}

struct StackUnwinder<'a> {
    context: &'a DebugContext,
    machine: &'a mut AsmMachine,
    // Only keeping 3 registers here: 0 is pc, 1 is ra, 2 is s0(fp).
    registers: [Option<u64>; 32],
    state: Option<(gimli::UnwindTableRow<Addr2LineEndianReader>, u64)>,
    unwind_context: gimli::UnwindContext<Addr2LineEndianReader>,
    at_start: bool,
}

struct UnwindInfo {
    row: gimli::UnwindTableRow<Addr2LineEndianReader>,
    // personality: Option<Pointer>,
    // lsda: Option<Pointer>,
    // initial_address: u64,
}

impl<'a> StackUnwinder<'a> {
    fn new(context: &'a DebugContext, machine: &'a mut AsmMachine) -> Self {
        let mut registers = [None; 32];
        for (i, v) in machine.machine.registers().iter().enumerate() {
            registers[i] = Some(*v);
        }
        Self {
            context,
            machine,
            registers,
            state: None,
            unwind_context: gimli::UnwindContext::new(),
            at_start: true,
        }
    }

    fn unwind_info(&mut self, address: u64) -> Result<Option<UnwindInfo>, String> {
        let bases = Default::default();
        let fde = self.context.debug_frame.fde_for_address(&bases, address, gimli::DebugFrame::cie_from_offset);
        let fde = match fde {
            Ok(fde) => fde,
            Err(e) => {
                if e == GimliError::NoUnwindInfoForAddress {
                    trace!("Unwind stopping at {:x}", address);
                    return Ok(None);
                } else {
                    return Err(format!("Error from fde_for_address: \"{}\", address: {:x}", e, address));
                }
            }
        };
        let mut table = fde
            .rows(&self.context.debug_frame, &bases, &mut self.unwind_context)
            .map_err(|e| format!("Error creating UnwindTable: \"{}\", address: {:x}", e, address))?;

        let mut result_row = None;
        while let Some(row) = table.next_row().map_err(|e| format!("Error fetching next row: {}", e))? {
            if row.contains(address) {
                result_row = Some(row.clone());
                break;
            }
        }

        if result_row.is_none() {
            trace!("Unwind row iteration stopping at {:x}", address);
        }

        Ok(result_row.map(|row| UnwindInfo { row }))
    }

    fn generate_state(
        &mut self,
        address: u64,
    ) -> Result<Option<(gimli::UnwindTableRow<Addr2LineEndianReader>, u64)>, String> {
        let UnwindInfo { row } = match self.unwind_info(address) {
            Ok(Some(info)) => info,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };
        let cfa = match *row.cfa() {
            gimli::CfaRule::RegisterAndOffset { register, offset } => {
                self.registers[register.0 as usize].expect("missing register value for cfa").wrapping_add(offset as u64)
            }
            _ => return Err(format!("Unknown cfa calculation rule: {:?}", row.cfa())),
        };
        Ok(Some((row, cfa)))
    }
}

impl<'a> Iterator for StackUnwinder<'a> {
    type Item = Symbol;

    fn next(&mut self) -> Option<Self::Item> {
        // handle PC
        if self.at_start {
            self.at_start = false;
            let pc = *self.machine.machine.pc();
            // TODO: what if ra just gets overwritten in the top frame?
            // self.state = self.generate_state(pc).expect("unwinding state");
            return Some(extract_symbol(pc, &self.context));
        }

        if let Some((row, cfa)) = self.state.take() {
            let mut newregs = self.registers.clone();
            newregs[RiscV::RA.0 as usize] = None;
            for &(reg, ref rule) in row.registers() {
                assert!(reg != RiscV::SP);
                newregs[reg.0 as usize] = match *rule {
                    RegisterRule::Undefined => unreachable!(),
                    RegisterRule::SameValue => self.registers[reg.0 as usize],
                    RegisterRule::Register(r) => self.registers[r.0 as usize],
                    RegisterRule::Offset(n) => Some(
                        self.machine.machine.memory_mut().load64(&cfa.wrapping_add(n as u64)).expect("load memory"),
                    ),
                    RegisterRule::ValOffset(n) => Some(cfa.wrapping_add(n as u64)),
                    RegisterRule::Expression(_) => unimplemented!(),
                    RegisterRule::ValExpression(_) => unimplemented!(),
                    RegisterRule::Architectural => unreachable!(),
                };
            }
            newregs[RiscV::SP.0 as usize] = Some(cfa);

            self.registers = newregs;
        }

        if let Some(caller) = self.registers[RiscV::RA.0 as usize] {
            // Unwinding
            self.state = self.generate_state(caller).expect("unwinding state");
            if self.state.is_none() {
                return None;
            }
            Some(extract_symbol(caller, &self.context))
        } else {
            None
        }
    }
}

// Inspired from ckb-vm-pprof
fn extract_symbol(pc: u64, context: &DebugContext) -> Symbol {
    let addr_context = &context.addr_context;
    let mut file = None;
    let mut line = None;

    let loc = addr_context.find_location(pc).unwrap();
    if let Some(loc) = loc {
        file = Some(loc.file.as_ref().unwrap().to_string());
        if let Some(loc_line) = loc.line {
            line = Some(loc_line);
        }
    }
    let mut frame_iter = addr_context.find_frames(pc).unwrap();
    let sprint_fun = |frame_iter: &mut Addr2LineFrameIter| {
        let mut s = String::from("<Unknown>");
        loop {
            if let Some(data) = frame_iter.next().unwrap() {
                if let Some(function) = data.function {
                    s = String::from(addr2line::demangle_auto(
                        Cow::from(function.raw_name().unwrap()),
                        function.language,
                    ));
                    continue;
                }
            }
            break;
        }
        s
    };
    let func = sprint_fun(&mut frame_iter);

    Symbol {
        name: Some(func),
        line,
        file,
    }
}

extern "C" fn perf_signal_handler(_signal: c_int) {
    let mut profiler = PROFILER.lock().expect("Mutex lock failure");
    if let Some(profiler) = profiler.deref_mut() {
        let machine = unsafe { &mut *(profiler.machine as *mut AsmMachine) as &mut AsmMachine };

        trace!("### Start profiling from {:x}", machine.machine.pc());
        let mut stacks = vec![];
        for stack in StackUnwinder::new(&profiler.context, machine) {
            trace!("Stack item: {:?}", stack);
            stacks.push(stack);
        }
        profiler.report.record(&Frame { stacks });
        trace!("### Done profiling for {:x}", machine.machine.pc());
    }
}

fn build_context(program: &Bytes) -> Result<DebugContext, String> {
    use addr2line::object::{Object, ObjectSection};

    // Adapted from https://github.com/gimli-rs/addr2line/blob/fc2de9f47ae513f5a54448167b476ff50f07dca6/src/lib.rs#L87-L148
    // for working with gimli::EndianArcSlice type
    let file = addr2line::object::File::parse(program.as_ref()).map_err(|e| format!("object parsing error: {}", e))?;

    let dwarf = gimli::Dwarf::load(|id| {
        let data = file
            .section_by_name(id.name())
            .and_then(|section| section.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[]));
        Ok(gimli::EndianArcSlice::new(
            Arc::from(&*data),
            gimli::RunTimeEndian::Little,
        ))
    })
    .map_err(|e: gimli::Error| format!("dwarf load error: {}", e))?;

    let addr_context = Addr2LineContext::from_dwarf(dwarf).map_err(|e| format!("context creation error: {}", e))?;

    let debug_frame_section = file
        .section_by_name(gimli::SectionId::DebugFrame.name())
        .and_then(|s| s.uncompressed_data().ok())
        .ok_or_else(|| "Provided binary is missing .debug_frame section!".to_string())?;
    let debug_frame_reader = Addr2LineEndianReader::new(Arc::from(&*debug_frame_section), gimli::RunTimeEndian::Little);

    Ok(DebugContext {
        addr_context,
        debug_frame: debug_frame_reader.into(),
    })
}
