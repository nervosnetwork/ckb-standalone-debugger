use ckb_chain_spec::consensus::ConsensusBuilder;
#[cfg(target_family = "unix")]
use ckb_debugger::Stdio;
use ckb_debugger::{
    get_script_hash_by_index, pre_check, DummyResourceLoader, ElfDumper, FileOperation, FileStream,
    HumanReadableCycles, MachineAnalyzer, MachineAssign, MachineOverlap, MachineProfile, MachineStepLog, Random,
    TimeNow,
};
use ckb_debugger::{Embed, GdbStubHandler, GdbStubHandlerEventLoop};
use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction, ReprMockTransaction, Resource};
use ckb_script::{ScriptGroupType, ScriptVersion, TransactionScriptsVerifier, TxVerifyEnv, ROOT_VM_ID};
use ckb_types::core::cell::{resolve_transaction, CellMetaBuilder};
use ckb_types::core::{hardfork, Capacity, DepType, HeaderView, ScriptHashType, TransactionBuilder};
use ckb_types::packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, Script};
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::decoder::build_decoder;
use ckb_vm::error::Error;
use ckb_vm::instructions::execute;
use ckb_vm::machine::VERSION2;
use ckb_vm::{Bytes, CoreMachine, Register, SupportMachine};
use clap::{crate_version, App, Arg};
use gdbstub::{
    conn::ConnectionExt,
    stub::{DisconnectReason, GdbStub, GdbStubError},
};
use probe::probe;
use std::collections::HashSet;
use std::io::{BufRead, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    drop(env_logger::init());

    let default_gdb_listen = "127.0.0.1:9999";
    let default_max_cycles = format!("{}", 70_000_000u64);
    let default_mode = "full";
    let default_pid = ROOT_VM_ID.to_string();
    let default_script_version = "2";

    let matches = App::new("ckb-debugger")
        .version(crate_version!())
        .arg(Arg::with_name("args").multiple(true))
        .arg(
            Arg::with_name("bin")
                .long("bin")
                .help("File used to replace the binary denoted in the script")
                .takes_value(true),
        )
        .arg(Arg::with_name("cell-index").long("cell-index").short("i").help("Index of cell to run").takes_value(true))
        .arg(
            Arg::with_name("cell-type")
                .long("cell-type")
                .short("t")
                .possible_values(&["input", "output"])
                .help("Type of cell to run")
                .takes_value(true),
        )
        .arg(Arg::with_name("dump-file").long("dump-file").help("Dump file name").takes_value(true))
        .arg(
            Arg::with_name("enable-overlapping-detection")
                .long("enable-overlapping-detection")
                .required(false)
                .takes_value(false)
                .help("Set to true to enable overlapping detection between stack and heap"),
        )
        .arg(
            Arg::with_name("enable-steplog")
                .long("enable-steplog")
                .help("Set to true to enable step mode, where we print PC address for each instruction"),
        )
        .arg(
            Arg::with_name("gdb-listen")
                .long("gdb-listen")
                .default_value(default_gdb_listen)
                .help("Address to listen for GDB remote debugging server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-cycles")
                .long("max-cycles")
                .default_value(&default_max_cycles)
                .help("Max cycles")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mode")
                .long("mode")
                .help("Execution mode of debugger")
                .possible_values(&["decode-instruction", "fast", "full", "gdb", "probe"])
                .default_value(&default_mode)
                .required(true)
                .takes_value(true),
        )
        .arg(Arg::with_name("pid").long("pid").default_value(&default_pid).help("Process ID").takes_value(true))
        .arg(
            Arg::with_name("pprof")
                .long("pprof")
                .help("Performance profiling, specify output file for further use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("prompt")
                .long("prompt")
                .required(false)
                .takes_value(false)
                .help("Set to true to prompt for stdin input before executing"),
        )
        .arg(
            Arg::with_name("read-file")
                .long("read-file")
                .help("Read content from local file or stdin. Then feed the content to syscall in scripts")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("script-group-type")
                .long("script-group-type")
                .short("s")
                .possible_values(&["lock", "type"])
                .help("Script group type")
                .takes_value(true),
        )
        .arg(Arg::with_name("script-hash").long("script-hash").help("Script hash").takes_value(true))
        .arg(
            Arg::with_name("script-version")
                .long("script-version")
                .default_value(&default_script_version)
                .help("Script version")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("tx-file")
                .long("tx-file")
                .short("f")
                .help("Filename containing JSON formatted transaction dump")
                .takes_value(true),
        )
        .get_matches();

    let matches_args = matches.values_of("args").unwrap_or_default();
    let matches_bin = matches.value_of("bin");
    let matches_cell_index = matches.value_of("cell-index");
    let matches_cell_type = matches.value_of("cell-type");
    let matches_dump_file = matches.value_of("dump-file");
    let matches_enable_overlapping_detection = matches.is_present("enable-overlapping-detection");
    let matches_enable_steplog = matches.is_present("enable-steplog");
    let matches_gdb_listen = matches.value_of("gdb-listen").unwrap();
    let matches_max_cycles = matches.value_of("max-cycles").unwrap();
    let matches_mode = matches.value_of("mode").unwrap();
    let matches_pid = u64::from_str_radix(matches.value_of("pid").unwrap(), 10).unwrap();
    let matches_pprof = matches.value_of("pprof");
    let matches_prompt = matches.is_present("prompt");
    let matches_read_file_name = matches.value_of("read-file");
    let matches_script_group_type = matches.value_of("script-group-type");
    let matches_script_hash = matches.value_of("script-hash");
    let matches_script_version = matches.value_of("script-version").unwrap();
    let matches_tx_file = matches.value_of("tx-file");

    if matches_mode == "decode-instruction" {
        let args: Vec<String> = matches_args.clone().into_iter().map(|s| s.into()).collect();
        let inst_str = &args[0];
        let inst_bin = if inst_str.starts_with("0x") {
            u32::from_str_radix(&inst_str[2..], 16)?
        } else {
            u32::from_str_radix(&inst_str, 10)?
        };
        let mut inst_tag = String::from("?");
        let mut inst_isa = String::from("?");
        if let Some(i) = ckb_vm::instructions::i::factory::<u64>(inst_bin, VERSION2) {
            assert_eq!(inst_tag.as_str(), "?");
            let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
            inst_tag = tagged_instruction.to_string();
            inst_isa = "I".to_string();
        }
        if let Some(i) = ckb_vm::instructions::m::factory::<u64>(inst_bin, VERSION2) {
            assert_eq!(inst_tag.as_str(), "?");
            let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
            inst_tag = tagged_instruction.to_string();
            inst_isa = "M".to_string();
        }
        if let Some(i) = ckb_vm::instructions::a::factory::<u64>(inst_bin, VERSION2) {
            assert_eq!(inst_tag.as_str(), "?");
            let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
            inst_tag = tagged_instruction.to_string();
            inst_isa = "A".to_string();
        }
        if let Some(i) = ckb_vm::instructions::rvc::factory::<u64>(inst_bin, VERSION2) {
            assert_eq!(inst_tag.as_str(), "?");
            let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
            inst_tag = tagged_instruction.to_string();
            inst_isa = "C".to_string();
        }
        if let Some(i) = ckb_vm::instructions::b::factory::<u64>(inst_bin, VERSION2) {
            assert_eq!(inst_tag.as_str(), "?");
            let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
            inst_tag = tagged_instruction.to_string();
            inst_isa = "B".to_string();
        }
        println!("       Assembly = {}", inst_tag);
        if inst_isa == "C" {
            println!("         Binary = {:016b}", inst_bin);
            println!("    Hexadecimal = {:04x}", inst_bin);
        } else {
            println!("         Binary = {:032b}", inst_bin);
            println!("    Hexadecimal = {:08x}", inst_bin);
        }
        println!("Instruction set = {}", inst_isa);
        return Ok(());
    }

    let verifier_max_cycles: u64 = matches_max_cycles.parse()?;
    let verifier_mock_tx: MockTransaction = match matches_tx_file {
        Some("-") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            let repr_mock_tx: ReprMockTransaction = serde_json::from_str(&buf)?;
            if let Err(msg) = pre_check(&repr_mock_tx) {
                println!("Potential format error found: {}", msg);
            }
            repr_mock_tx.into()
        }
        Some(doc) => {
            let buf = std::fs::read_to_string(doc)?;
            let mut mock_tx_embed = Embed::new(PathBuf::from(doc.to_string()), buf.clone());
            let buf = mock_tx_embed.replace_all();
            let repr_mock_tx: ReprMockTransaction = serde_json::from_str(&buf)?;
            if let Err(msg) = pre_check(&repr_mock_tx) {
                println!("Potential format error found: {}", msg);
            }
            repr_mock_tx.into()
        }
        None => {
            let bin_path = matches_bin.unwrap();
            let bin_data = std::fs::read(bin_path)?;
            let bin_cell_meta = {
                let cell_data = Bytes::copy_from_slice(&bin_data);
                let cell_output =
                    CellOutput::new_builder().capacity(Capacity::bytes(cell_data.len()).unwrap().pack()).build();
                CellMetaBuilder::from_cell_output(cell_output, cell_data).build()
            };
            let bin_cell_hash = bin_cell_meta.mem_cell_data_hash.as_ref().unwrap().to_owned();

            let mut mock_info = MockInfo::default();
            mock_info.cell_deps.push(MockCellDep {
                cell_dep: CellDep::new_builder()
                    .out_point(OutPoint::new(Byte32::from_slice(vec![0x00; 32].as_slice()).unwrap(), 0))
                    .dep_type(DepType::Code.into())
                    .build(),
                output: CellOutput::new_builder().build(),
                data: Bytes::from(bin_data.clone()),
                header: None,
            });
            mock_info.inputs.push(MockInput {
                input: CellInput::new(OutPoint::new(Byte32::from_slice(vec![0x00; 32].as_slice()).unwrap(), 1), 0),
                output: CellOutput::new_builder()
                    .lock(
                        Script::new_builder().code_hash(bin_cell_hash).hash_type(ScriptHashType::Data2.into()).build(),
                    )
                    .build_exact_capacity(Capacity::bytes(bin_data.len()).unwrap())
                    .unwrap(),
                data: Bytes::new(),
                header: None,
            });

            let tx = TransactionBuilder::default();
            let tx = tx.cell_dep(mock_info.cell_deps[0].cell_dep.clone());
            let tx = tx.input(mock_info.inputs[0].input.clone());
            let tx = tx.output(
                CellOutput::new_builder()
                    .capacity(Capacity::zero().pack())
                    .lock(mock_info.inputs[0].output.lock())
                    .build(),
            );
            let tx = tx.build();

            MockTransaction { mock_info: mock_info, tx: tx.data() }
        }
    };
    let verifier_script_group_type = {
        let script_group_type = if matches_tx_file.is_none() { "lock" } else { matches_script_group_type.unwrap() };
        serde_plain::from_str(script_group_type)?
    };
    let verifier_script_hash = if matches_tx_file.is_none() {
        verifier_mock_tx.mock_info.inputs[0].output.calc_lock_hash()
    } else if let Some(hex_script_hash) = matches_script_hash {
        if hex_script_hash.len() != 66 || (!hex_script_hash.starts_with("0x")) {
            panic!("Invalid script hash format!");
        }
        let b = hex::decode(&hex_script_hash.as_bytes()[2..])?;
        Byte32::from_slice(b.as_slice())?
    } else {
        let mut cell_type = matches_cell_type;
        let mut cell_index = matches_cell_index;
        match verifier_script_group_type {
            ScriptGroupType::Lock => {
                if cell_type.is_none() {
                    cell_type = Some("input");
                }
                if cell_index.is_none() {
                    cell_index = Some("0");
                    println!("The cell_index is not specified. Assume --cell-index = 0")
                }
            }
            ScriptGroupType::Type => {
                if cell_type.is_none() || cell_index.is_none() {
                    panic!("You must provide either script hash, or cell type + cell index");
                }
            }
        }
        let cell_type = cell_type.unwrap();
        let cell_index: usize = cell_index.unwrap().parse()?;
        get_script_hash_by_index(&verifier_mock_tx, &verifier_script_group_type, cell_type, cell_index)
    };
    let verifier_script_version = match matches_script_version {
        "0" => ScriptVersion::V0,
        "1" => ScriptVersion::V1,
        "2" => ScriptVersion::V2,
        _ => panic!("Wrong script version"),
    };
    let verifier_resource = Resource::from_both(&verifier_mock_tx, DummyResourceLoader {})?;
    let verifier_resolve_transaction = resolve_transaction(
        verifier_mock_tx.core_transaction(),
        &mut HashSet::new(),
        &verifier_resource,
        &verifier_resource,
    )?;
    let mut verifier = {
        let hardforks = hardfork::HardForks {
            ckb2021: hardfork::CKB2021::new_mirana().as_builder().rfc_0032(20).build().unwrap(),
            ckb2023: hardfork::CKB2023::new_mirana().as_builder().rfc_0049(30).build().unwrap(),
        };
        let consensus = Arc::new(ConsensusBuilder::default().hardfork_switch(hardforks).build());
        let epoch = match verifier_script_version {
            ScriptVersion::V0 => ckb_types::core::EpochNumberWithFraction::new(15, 0, 1),
            ScriptVersion::V1 => ckb_types::core::EpochNumberWithFraction::new(25, 0, 1),
            ScriptVersion::V2 => ckb_types::core::EpochNumberWithFraction::new(35, 0, 1),
        };
        let header_view = HeaderView::new_advanced_builder().epoch(epoch.pack()).build();
        let tx_env = Arc::new(TxVerifyEnv::new_commit(&header_view));
        TransactionScriptsVerifier::new(
            Arc::new(verifier_resolve_transaction.clone()),
            verifier_resource.clone(),
            consensus.clone(),
            tx_env.clone(),
        )
    };
    verifier.set_debug_printer(Box::new(move |_hash: &Byte32, message: &str| {
        print!("Script log: {}", message);
        if !message.ends_with('\n') {
            println!("");
        }
    }));
    let verifier_script_group = verifier.find_script_group(verifier_script_group_type, &verifier_script_hash).unwrap();
    let verifier_program = match matches_bin {
        Some(path) => {
            let data = std::fs::read(path)?;
            data.into()
        }
        None => verifier.extract_script(&verifier_script_group.script)?,
    };

    let machine_assign_init = || -> Result<_, Box<dyn std::error::Error>> {
        let mut scheduler = verifier.create_scheduler(&verifier_script_group).unwrap();
        scheduler.tx_data.program = verifier_program.clone();
        let mut machine_assign = MachineAssign::new(matches_pid, scheduler)?;
        machine_assign.expand_cycles = verifier_max_cycles;
        if let Some(data) = matches_dump_file {
            machine_assign.expand_syscalls.push(Box::new(ElfDumper::new(data.to_string(), 4097, 64)));
        }
        machine_assign.expand_syscalls.push(Box::new(FileOperation::new()));
        if let Some(name) = matches_read_file_name {
            machine_assign.expand_syscalls.push(Box::new(FileStream::new(name)));
        }
        machine_assign.expand_syscalls.push(Box::new(Random::new()));
        #[cfg(target_family = "unix")]
        machine_assign.expand_syscalls.push(Box::new(Stdio::new(false)));
        machine_assign.expand_syscalls.push(Box::new(TimeNow::new()));
        machine_assign.wait()?;
        Ok(machine_assign)
    };

    if matches_mode == "fast" {
        let cycles = verifier.verify_single(verifier_script_group_type, &verifier_script_hash, verifier_max_cycles)?;
        println!("Total cycles consumed: {}", HumanReadableCycles(cycles));
        return Ok(());
    }

    if matches_mode == "full" {
        let machine_assign = machine_assign_init()?;
        let machine_profile = MachineProfile::new(&machine_assign.code().clone())?;
        let machine_overlap = MachineOverlap::new(&machine_assign.code().clone())?;
        let machine_steplog = MachineStepLog::new();
        let mut machine = MachineAnalyzer::new(machine_assign, machine_profile, machine_overlap, machine_steplog);
        if matches_enable_overlapping_detection {
            machine.enable_overlap = 1;
        }
        if matches_enable_steplog {
            machine.enable_steplog = 1;
        }
        let result = machine.run();
        if matches_pid != ROOT_VM_ID {
            machine.machine.done()?;
        }
        let cycles = machine.machine.scheduler.consumed_cycles();
        match result {
            Ok(data) => {
                println!("Run result: {:?}", data);
                println!("Total cycles consumed: {}", HumanReadableCycles(cycles));
                if let Some(fp) = matches_pprof {
                    let mut output = std::fs::File::create(&fp)?;
                    machine.profile.display_flamegraph(&mut output);
                }
                if data != 0 {
                    std::process::exit(254);
                }
                return Ok(());
            }
            Err(err) => {
                machine.profile.display_stacktrace("", &mut std::io::stdout());
                println!("");
                println!("{}", machine);
                return Err(Box::new(err));
            }
        }
    }

    if matches_mode == "gdb" {
        let listener = TcpListener::bind(matches_gdb_listen)?;
        println!("Listening for gdb remote connection on {}", matches_gdb_listen);
        for res in listener.incoming() {
            if let Ok(stream) = res {
                println!("Accepted connection from: {}, booting VM", stream.peer_addr()?);
                let mut machine_assign = machine_assign_init()?;
                machine_assign.set_running(true);
                let mut h = GdbStubHandler::new(machine_assign);
                let connection: Box<(dyn ConnectionExt<Error = std::io::Error> + 'static)> = Box::new(stream);
                let gdb = GdbStub::new(connection);

                let result = match gdb.run_blocking::<GdbStubHandlerEventLoop<_>>(&mut h) {
                    Ok(disconnect_reason) => match disconnect_reason {
                        DisconnectReason::Disconnect => {
                            println!("GDB client has disconnected. Running to completion...");
                            h.run_till_exited()
                        }
                        DisconnectReason::TargetExited(_) => h.run_till_exited(),
                        DisconnectReason::TargetTerminated(sig) => {
                            Err(Error::External(format!("Target terminated with signal {}!", sig)))
                        }
                        DisconnectReason::Kill => Err(Error::External("GDB sent a kill command!".to_string())),
                    },
                    Err(GdbStubError::TargetError(e)) => {
                        Err(Error::External(format!("Target encountered a fatal error: {}", e)))
                    }
                    Err(e) => Err(Error::External(format!("Gdbstub encountered a fatal error: {}", e))),
                };
                match result {
                    Ok((exit_code, cycles)) => {
                        println!("Exit code: {:?}", exit_code);
                        println!("Total cycles consumed: {}", HumanReadableCycles(cycles));
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        }
        return Ok(());
    }

    if matches_mode == "probe" {
        if matches_prompt {
            println!("Enter to start executing:");
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line).expect("read");
        }

        let mut machine = machine_assign_init()?;
        machine.set_running(true);
        let mut decoder = build_decoder::<u64>(verifier_script_version.vm_isa(), verifier_script_version.vm_version());
        let mut step_result = Ok(());
        while machine.running() && step_result.is_ok() {
            let pc = machine.pc().to_u64();
            step_result = decoder
                .decode(machine.memory_mut(), pc)
                .and_then(|inst| {
                    let cycles = estimate_cycles(inst);
                    machine.add_cycles(cycles).map(|_| inst)
                })
                .and_then(|inst| {
                    let regs = machine.registers().as_ptr();
                    let memory = (&mut machine.memory_mut().inner_mut()).as_ptr();
                    let cycles = machine.cycles();
                    probe!(ckb_vm, execute_inst, pc, cycles, inst, regs, memory);
                    let r = execute(inst, &mut machine);
                    let cycles = machine.cycles();
                    probe!(ckb_vm, execute_inst_end, pc, cycles, inst, regs, memory, if r.is_ok() { 0 } else { 1 });
                    r
                });
        }
        let result = step_result.map(|_| machine.exit_code());
        println!("Run result: {:?}", result);
        println!("Total cycles consumed: {}", HumanReadableCycles(machine.scheduler.consumed_cycles()));
    }

    Ok(())
}
