use ckb_chain_spec::consensus::{ConsensusBuilder, TYPE_ID_CODE_HASH};
use ckb_debugger::{
    ElfDumper, FileOperation, FileStream, FileWriter, GdbStubHandler, GdbStubHandlerEventLoop, HumanReadableCycles,
    MachineAnalyzer, MachineAssign, MachineCoverage, MachineOverlap, MachineProfile, MachineStepLog, Random, Stdio,
    Timestamp, get_script_hash_by_index, instruction_decode, mock_tx_analyze, mock_tx_embed,
};
use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction, ReprMockTransaction, Resource};
use ckb_script::{ROOT_VM_ID, ScriptError, ScriptGroupType, ScriptVersion, TransactionScriptsVerifier, TxVerifyEnv};
use ckb_types::core::cell::{CellMeta, resolve_transaction};
use ckb_types::core::{Capacity, DepType, HeaderView, ScriptHashType, TransactionBuilder, hardfork};
use ckb_types::packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, Script, ScriptOpt};
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::decoder::build_decoder;
use ckb_vm::error::Error;
use ckb_vm::instructions::execute;
use ckb_vm::{Bytes, CoreMachine, Register, SupportMachine};
use clap::{App, Arg, crate_version};
use gdbstub::{
    conn::ConnectionExt,
    stub::{DisconnectReason, GdbStub},
};
use probe::probe;
use std::collections::HashSet;
use std::io::{BufRead, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let default_gdb_listen = "127.0.0.1:9999";
    let default_max_cycles = format!("{}", 3_500_000_000u64);
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
        .arg(
            Arg::with_name("coverage-output")
                .long("coverage-output")
                .help("Save coverage info in lcov format to file")
                .takes_value(true),
        )
        .arg(Arg::with_name("dump-file").long("dump-file").help("Dump file name").takes_value(true))
        .arg(
            Arg::with_name("enable-coverage")
                .long("enable-coverage")
                .required(false)
                .takes_value(false)
                .help("Set to true to enable coverage info"),
        )
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
                .possible_values(&["decode-instruction", "fast", "full", "gdb", "instruction-decode", "probe"])
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
            Arg::with_name("script")
                .long("script")
                .default_value("input.0.lock")
                .help("A convenience method for setting cell-type, cell-index and script-group-type at the same time")
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
    let matches_coverage_output = matches.value_of("coverage-output");
    let matches_dump_file = matches.value_of("dump-file");
    let matches_enable_coverage = matches.is_present("enable-coverage");
    let matches_enable_overlapping_detection = matches.is_present("enable-overlapping-detection");
    let matches_enable_steplog = matches.is_present("enable-steplog");
    let matches_gdb_listen = matches.value_of("gdb-listen").unwrap();
    let matches_max_cycles = matches.value_of("max-cycles").unwrap();
    let matches_mode = matches.value_of("mode").unwrap();
    let matches_pid = u64::from_str_radix(matches.value_of("pid").unwrap(), 10).unwrap();
    let matches_pprof = matches.value_of("pprof");
    let matches_prompt = matches.is_present("prompt");
    let matches_read_file_name = matches.value_of("read-file");
    let matches_script = matches.value_of("script");
    let matches_script_group_type = matches.value_of("script-group-type");
    let matches_script_hash = matches.value_of("script-hash");
    let matches_script_version = matches.value_of("script-version").unwrap();
    let matches_tx_file = matches.value_of("tx-file");

    if matches!(matches_mode, "decode-instruction" | "instruction-decode") {
        let args: Vec<String> = matches_args.clone().into_iter().map(|s| s.into()).collect();
        let inst_str = &args[0];
        let inst_bin = if inst_str.starts_with("0x") {
            u32::from_str_radix(&inst_str[2..], 16)?
        } else {
            u32::from_str_radix(&inst_str, 10)?
        };
        instruction_decode(inst_bin);
        return Ok(());
    }

    let verifier_max_cycles: u64 = matches_max_cycles.parse()?;
    let verifier_mock_tx: MockTransaction = match matches_tx_file {
        Some("-") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            mock_tx_analyze(&buf)?;
            let repr_mock_tx: ReprMockTransaction = serde_json::from_str(&buf)?;
            repr_mock_tx.into()
        }
        Some(doc) => {
            let buf = std::fs::read_to_string(doc)?;
            let buf = mock_tx_embed(PathBuf::from(doc.to_string()), &buf);
            mock_tx_analyze(&buf)?;
            let repr_mock_tx: ReprMockTransaction = serde_json::from_str(&buf)?;
            repr_mock_tx.into()
        }
        None => {
            let cell_meta_lock_data = Bytes::copy_from_slice(&std::fs::read(matches_bin.unwrap())?);
            let cell_meta_lock = CellMeta {
                cell_output: CellOutput::new_builder()
                    .type_(
                        ScriptOpt::new_builder()
                            .set(Some(
                                Script::new_builder()
                                    .code_hash(Byte32::from_slice(TYPE_ID_CODE_HASH.as_bytes())?)
                                    .hash_type(ScriptHashType::Type.into())
                                    .args(Bytes::copy_from_slice(&vec![0u8; 32]).pack())
                                    .build(),
                            ))
                            .build(),
                    )
                    .build_exact_capacity(Capacity::bytes(cell_meta_lock_data.len())?)?,
                out_point: OutPoint::new(Byte32::from_slice(&vec![0x00; 32])?, 0),
                data_bytes: cell_meta_lock_data.len() as u64,
                mem_cell_data: Some(cell_meta_lock_data.clone()),
                mem_cell_data_hash: Some(Byte32::from_slice(&ckb_hash::blake2b_256(&cell_meta_lock_data))?),
                ..Default::default()
            };
            let cell_meta_i = CellMeta {
                cell_output: CellOutput::new_builder()
                    .lock(
                        Script::new_builder()
                            .code_hash(cell_meta_lock.cell_output.type_().to_opt().unwrap().calc_script_hash())
                            .hash_type(ScriptHashType::Type.into())
                            .build(),
                    )
                    .build_exact_capacity(Capacity::zero())?,
                out_point: OutPoint::new(Byte32::from_slice(&vec![0x00; 32])?, 1),
                ..Default::default()
            };

            let mut mock_info = MockInfo::default();
            mock_info.cell_deps.push(MockCellDep {
                cell_dep: CellDep::new_builder()
                    .out_point(cell_meta_lock.out_point)
                    .dep_type(DepType::Code.into())
                    .build(),
                output: cell_meta_lock.cell_output,
                data: cell_meta_lock_data.clone(),
                header: None,
            });
            mock_info.inputs.push(MockInput {
                input: CellInput::new(cell_meta_i.out_point, 0),
                output: cell_meta_i.cell_output,
                data: Bytes::new(),
                header: None,
            });

            let tx = TransactionBuilder::default();
            let tx = tx.cell_dep(mock_info.cell_deps[0].cell_dep.clone());
            let tx = tx.input(mock_info.inputs[0].input.clone());
            let tx = tx.build();

            MockTransaction { mock_info, tx: tx.data() }
        }
    };
    let verifier_cell_type = match matches_cell_type {
        Some(data) => data,
        None => matches_script.unwrap().split(".").collect::<Vec<&str>>()[0],
    };
    let verifier_cell_index: usize = match matches_cell_index {
        Some(data) => data.parse()?,
        None => matches_script.unwrap().split(".").collect::<Vec<&str>>()[1].parse()?,
    };
    let verifier_script_group_type: ScriptGroupType = match matches_script_group_type {
        Some(data) => serde_plain::from_str(data)?,
        None => serde_plain::from_str(matches_script.unwrap().split(".").collect::<Vec<&str>>()[2])?,
    };
    let verifier_script_hash = || -> Result<Byte32, Box<dyn std::error::Error>> {
        if matches_tx_file.is_none() {
            return Ok(verifier_mock_tx.mock_info.inputs[0].output.calc_lock_hash());
        }
        if let Some(hex_script_hash) = matches_script_hash {
            return Ok(Byte32::from_slice(hex::decode(&hex_script_hash.as_bytes()[2..])?.as_slice())?);
        }
        Ok(get_script_hash_by_index(
            &verifier_mock_tx,
            &verifier_script_group_type,
            verifier_cell_type,
            verifier_cell_index,
        ))
    }()?;
    let verifier_script_version = match matches_script_version {
        "0" => ScriptVersion::V0,
        "1" => ScriptVersion::V1,
        "2" => ScriptVersion::V2,
        _ => panic!("Wrong script version"),
    };
    let verifier_script = || -> Result<Script, ScriptError> {
        let verifier_script_hash = verifier_script_hash.clone();
        for e in &verifier_mock_tx.mock_info.inputs {
            if e.output.calc_lock_hash().as_slice() == verifier_script_hash.as_slice() {
                return Ok(e.output.lock());
            }
            if let Some(kype) = e.output.type_().to_opt() {
                if kype.calc_script_hash().as_slice() == verifier_script_hash.as_slice() {
                    return Ok(kype);
                }
            }
        }
        for e in verifier_mock_tx.core_transaction().outputs().into_iter() {
            if let Some(kype) = e.type_().to_opt() {
                if kype.calc_script_hash().as_slice() == verifier_script_hash.as_slice() {
                    return Ok(kype);
                }
            }
        }
        Err(ScriptError::ScriptNotFound(verifier_script_hash))
    }()?;
    assert_eq!(verifier_script.calc_script_hash(), verifier_script_hash);
    let verifier_script_out_point = || -> Result<OutPoint, ScriptError> {
        match ScriptHashType::try_from(verifier_script.hash_type()).unwrap() {
            ScriptHashType::Data | ScriptHashType::Data1 | ScriptHashType::Data2 => {
                for e in &verifier_mock_tx.mock_info.cell_deps {
                    if ckb_hash::blake2b_256(&e.data) == verifier_script.code_hash().as_slice() {
                        return Ok(e.cell_dep.out_point());
                    }
                }
                unreachable!()
            }
            ScriptHashType::Type => {
                for e in &verifier_mock_tx.mock_info.cell_deps {
                    if let Some(kype) = e.output.type_().to_opt() {
                        if kype.calc_script_hash() == verifier_script.code_hash() {
                            return Ok(e.cell_dep.out_point());
                        }
                    }
                }
                unreachable!()
            }
        }
    }()?;
    let verifier_resource = Resource::from_mock_tx(&verifier_mock_tx)?;
    let mut verifier_resolve_transaction = resolve_transaction(
        verifier_mock_tx.core_transaction(),
        &mut HashSet::new(),
        &verifier_resource,
        &verifier_resource,
    )?;
    if matches_tx_file.is_some() && matches_bin.is_some() {
        for e in &mut verifier_resolve_transaction.resolved_cell_deps {
            if e.out_point == verifier_script_out_point {
                let data = Bytes::copy_from_slice(&std::fs::read(matches_bin.unwrap())?);
                e.mem_cell_data = Some(data);
            }
        }
    }
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
            verifier_resource,
            consensus.clone(),
            tx_env.clone(),
        )
    };
    verifier.set_debug_printer(Box::new(move |_hash: &Byte32, message: &str| {
        let message = message.trim_end_matches('\n');
        if message != "" {
            ckb_debugger::arch::println(&format!("Script log: {}", message));
        }
    }));
    let verifier_script_group = verifier.find_script_group(verifier_script_group_type, &verifier_script_hash).unwrap();
    let machine_assign_init = || -> Result<_, Box<dyn std::error::Error>> {
        let args: Vec<String> = matches_args.clone().into_iter().map(|s| s.into()).collect();
        let args: Vec<Bytes> = args.into_iter().map(|s| s.into()).collect();
        let scheduler = verifier.create_scheduler(&verifier_script_group).unwrap();
        let mut machine_assign = MachineAssign::new(matches_pid, &args, scheduler)?;
        machine_assign.expand_cycles = verifier_max_cycles;
        if let Some(data) = matches_dump_file {
            machine_assign.expand_syscalls.push(Box::new(ElfDumper::new(data.to_string(), 4097, 64)));
        }
        machine_assign.expand_syscalls.push(Box::new(FileOperation::new()));
        if let Some(name) = matches_read_file_name {
            machine_assign.expand_syscalls.push(Box::new(FileStream::new(name)));
        }
        machine_assign.expand_syscalls.push(Box::new(FileWriter::new()));
        machine_assign.expand_syscalls.push(Box::new(Random::new()));
        machine_assign.expand_syscalls.push(Box::new(Stdio::new(false)));
        machine_assign.expand_syscalls.push(Box::new(Timestamp::new()));
        machine_assign.wait()?;
        Ok(machine_assign)
    };

    if matches_mode == "fast" {
        let cycles = verifier.verify_single(verifier_script_group_type, &verifier_script_hash, verifier_max_cycles)?;
        println!("All cycles: {}", HumanReadableCycles(cycles));
        return Ok(());
    }

    if matches_mode == "full" {
        let machine_assign = machine_assign_init()?;
        let machine_profile = MachineProfile::new(&machine_assign.code().clone())?;
        let machine_overlap = MachineOverlap::new(&machine_assign.code().clone())?;
        let machine_steplog = MachineStepLog::new();
        let machine_coverage = MachineCoverage::new(&machine_assign.code().clone())?;
        let mut machine =
            MachineAnalyzer::new(machine_assign, machine_profile, machine_overlap, machine_steplog, machine_coverage);
        if matches_enable_overlapping_detection {
            machine.enable_overlap = 1;
        }
        if matches_enable_steplog {
            machine.enable_steplog = 1;
        }
        if matches_enable_coverage {
            machine.enable_coverage = 1;
        }
        let result = machine.run();
        if matches_pid != ROOT_VM_ID {
            machine.machine.done()?;
        }
        let cycles = machine.machine.scheduler.consumed_cycles();
        match result {
            Ok(data) => {
                println!("Run result: {:?}", data);
                println!("All cycles: {}", HumanReadableCycles(cycles));
                if let Some(fp) = matches_pprof {
                    let mut output = std::fs::File::create(&fp)?;
                    machine.profile.display_flamegraph(&mut output);
                }
                if let Some(fp) = matches_coverage_output {
                    let mut output = std::fs::File::create(&fp)?;
                    machine.coverage.display_lcov(&mut output)?;
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
                    Err(e) => Err(Error::External(format!("Gdbstub encountered a fatal error: {}", e))),
                };
                match result {
                    Ok((exit_code, cycles)) => {
                        println!("Exit code: {:?}", exit_code);
                        println!("All cycles: {}", HumanReadableCycles(cycles));
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

                    #[cfg(not(feature = "asm"))]
                    let memory = (&mut machine.memory_mut().inner_mut()).as_ptr();
                    #[cfg(feature = "asm")]
                    let memory = machine.memory().as_ref().memory.as_ptr();

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
        println!("All cycles: {}", HumanReadableCycles(machine.scheduler.consumed_cycles()));
    }

    Ok(())
}
