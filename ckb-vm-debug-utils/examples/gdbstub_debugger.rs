#[macro_use]
extern crate log;

use bytes::Bytes;
use ckb_vm::{
    DefaultCoreMachine, DefaultMachineBuilder, SparseMemory, SupportMachine, WXorXMemory, ISA_B, ISA_IMC, ISA_MOP,
};
#[cfg(feature = "stdio")]
use ckb_vm_debug_utils::Stdio;
use ckb_vm_debug_utils::{GdbStubHandler, GdbStubHandlerEventLoop};
use gdbstub::{conn::ConnectionExt, stub::GdbStub};
use gdbstub_arch::riscv::Riscv64;
use std::env;
use std::fs::File;
use std::io::Read;
use std::net::TcpListener;

fn main() {
    drop(env_logger::init());
    let args: Vec<String> = env::args().skip(1).collect();

    let listener = TcpListener::bind(&args[0]).expect("listen");
    debug!("Listening on {}", args[0]);

    let mut file = File::open(&args[1]).expect("open program");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let program: Bytes = buffer.into();
    let program_args: Vec<Bytes> = args.into_iter().skip(1).map(|a| a.into()).collect();

    for res in listener.incoming() {
        debug!("Got connection");
        if let Ok(stream) = res {
            // TODO: vm version and isa should be configurable in the future.
            let machine_core = DefaultCoreMachine::<u64, WXorXMemory<SparseMemory<u64>>>::new(
                ISA_IMC | ISA_B | ISA_MOP,
                1,
                u64::max_value(),
            );
            let machine_builder = DefaultMachineBuilder::new(machine_core);
            #[cfg(feature = "stdio")]
            let mut machine = machine_builder.syscall(Box::new(Stdio::new(true))).build();
            #[cfg(not(feature = "stdio"))]
            let mut machine = machine_builder.build();
            machine.load_program(&program, &program_args).expect("load program");
            machine.set_running(true);
            let mut h: GdbStubHandler<_, Riscv64> = GdbStubHandler::new(machine);
            let connection: Box<(dyn ConnectionExt<Error = std::io::Error> + 'static)> = Box::new(stream);
            let gdb = GdbStub::new(connection);

            let result = gdb.run_blocking::<GdbStubHandlerEventLoop<_, _>>(&mut h);
            println!("Disconnect reason: {:?}", result);
        }
        debug!("Connection closed");
    }
}
