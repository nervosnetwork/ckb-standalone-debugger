use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flag_parser = clap::App::new("ckb-vm-pprof")
        .version("0.2.1")
        .about("A pprof tool for CKB VM")
        .arg(
            clap::Arg::with_name("bin")
                .long("bin")
                .value_name("filename")
                .help("Specify the name of the executable")
                .required(true),
        )
        .arg(
            clap::Arg::with_name("arg")
                .long("arg")
                .value_name("arguments")
                .help("Pass arguments to binary")
                .multiple(true),
        )
        .get_matches();
    let fl_bin = flag_parser.value_of("bin").unwrap();
    let fl_arg: Vec<_> = flag_parser.values_of("arg").unwrap_or_default().collect();

    let code_data = std::fs::read(fl_bin)?;
    let code = ckb_vm::Bytes::from(code_data);
    let isa = ckb_vm::ISA_IMC | ckb_vm::ISA_A | ckb_vm::ISA_B | ckb_vm::ISA_MOP;
    let default_core_machine = ckb_vm::DefaultCoreMachine::<
        u64,
        ckb_vm::memory::wxorx::WXorXMemory<ckb_vm::memory::sparse::SparseMemory<u64>>,
    >::new(isa, ckb_vm::machine::VERSION2, 1 << 32);
    let default_machine = ckb_vm::DefaultMachineBuilder::new(default_core_machine)
        .instruction_cycle_func(Box::new(ckb_vm_pprof::estimate_cycles))
        .build();
    let profile = ckb_vm_pprof::Profile::new(&code)?;
    let mut machine = ckb_vm_pprof::PProfMachine::new(default_machine, profile);
    let mut args = vec![];
    args.append(&mut fl_arg.iter().map(|x| ckb_vm::Bytes::from(x.to_string())).collect());
    machine.load_program(&code, &args)?;
    match machine.run() {
        Ok(data) => {
            if data != 0 {
                println!("Error:");
                println!("  Code({:?})", data);
            }
            machine.profile.display_flamegraph(&mut std::io::stdout());
        }
        Err(err) => {
            std::io::stdout().write_all(b"Trace:\n")?;
            machine.profile.display_stacktrace("  ", &mut std::io::stdout());
            println!("Error:");
            println!("  Err({:?})", err);
        }
    }
    Ok(())
}
