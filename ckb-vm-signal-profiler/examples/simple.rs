// A simple ckb-vm profiler with debugger syscall implemented.

use ckb_vm::{
    Bytes, DefaultMachineRunner, Error as VMError, ISA_A, ISA_B, ISA_IMC, ISA_MOP, Memory, Register, SupportMachine,
    Syscalls,
    machine::{
        DefaultMachineBuilder, VERSION2,
        asm::{AsmCoreMachine, AsmMachine},
    },
    registers::{A0, A7},
};
use std::env;
use std::process::exit;

pub struct Debugger {}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {}
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for Debugger {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), VMError> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, VMError> {
        let number = machine.registers()[A7].to_u64();
        if number != 2177 {
            return Ok(false);
        }

        let mut addr = machine.registers()[A0].to_u64();
        let mut buffer = Vec::new();

        loop {
            let byte = machine.memory_mut().load8(&Mac::REG::from_u64(addr))?.to_u8();
            if byte == 0 {
                break;
            }
            buffer.push(byte);
            addr += 1;
        }

        let s = String::from_utf8(buffer).expect("utf8 error");
        println!("{}", s);

        Ok(true)
    }
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().skip(1).collect();
    let code = std::fs::read(args[0].clone()).unwrap().into();
    let args: Vec<Bytes> = args.into_iter().map(|a| a.into()).collect();

    let asm_core =
        <Box<AsmCoreMachine> as SupportMachine>::new(ISA_IMC | ISA_A | ISA_B | ISA_MOP, VERSION2, u64::max_value());
    let core = DefaultMachineBuilder::new(asm_core).syscall(Box::new(Debugger::new())).build();
    let mut machine = Box::pin(AsmMachine::new(core));

    ckb_vm_signal_profiler::start_profiler("simple.profile", &machine, &code, 99).expect("profiler start failure");

    machine.load_program(&code, args.into_iter().map(Ok)).unwrap();
    let result = machine.run();
    if result != Ok(0) {
        println!("Error result: {:?}", result);
        exit(i32::from(result.unwrap_or(-1)));
    }
    ckb_vm_signal_profiler::stop_profiler().expect("profiler start failure");
}
