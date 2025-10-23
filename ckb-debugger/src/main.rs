use ckb_chain_spec::consensus::{ConsensusBuilder, TYPE_ID_CODE_HASH};
use ckb_debugger::{
    ElfDumper, FileOperation, FileStream, FileWriter, HumanReadableCycles, MachineAnalyzer, MachineCoverage,
    MachineFlamegraph, MachineOverlap, MachineStepLog, Random, Stdio, Timestamp, arch, get_script_hash_by_index,
    instruction_decode, mock_tx_analyze, mock_tx_embed, print_vm_tree_recursive,
};
use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction, ReprMockTransaction, Resource};
use ckb_script::types::{DebugPrinter, Machine, SgData, VmContext, VmId};
use ckb_script::{
    ROOT_VM_ID, ScriptError, ScriptGroupType, ScriptVersion, TransactionScriptsVerifier, TxVerifyEnv,
    generate_ckb_syscalls,
};
use ckb_types::core::cell::{CellMeta, resolve_transaction};
use ckb_types::core::{Capacity, DepType, HeaderView, ScriptHashType, TransactionBuilder, hardfork};
use ckb_types::packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, Script, ScriptOpt};
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::decoder::build_decoder;
use ckb_vm::error::Error;
use ckb_vm::instructions::execute;
use ckb_vm::{Bytes, CoreMachine, Register, SupportMachine};
use ckb_vm::{DefaultMachineRunner, Syscalls};
use ckb_vm_debug_utils::{GdbStubHandler, GdbStubHandlerEventLoop};
use ckb_vm_fuzzing_utils::SynchronousSyscalls;
use ckb_vm_syscall_tracer::{
    BinaryLocatorCollector, Collector, CollectorKey, SyscallBasedCollector, VmCreateCollector,
};
use clap::{App, Arg, crate_version};
use gdbstub::{
    conn::ConnectionExt,
    stub::{DisconnectReason, GdbStub},
};
use gdbstub_arch::riscv::Riscv64;
use probe::probe;
use protobuf_ckb_syscalls::ProtobufVmRunnerImpls;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_gdb_listen = "127.0.0.1:9999";
    let default_max_cycles = format!("{}", 3_500_000_000u64);
    let default_mode = "fast";
    let default_script_version = "2";
    let default_script = "input.0.lock";
    let default_vm_id = ROOT_VM_ID.to_string();

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
            Arg::with_name("enable-flamegraph")
                .long("enable-flamegraph")
                .required(false)
                .takes_value(false)
                .help("Set to true to enable flamegraph"),
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
            Arg::with_name("flamegraph-output")
                .long("flamegraph-output")
                .help("Performance profiling, specify output file for further use")
                .takes_value(true),
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
                .default_value(default_script)
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
        .arg(Arg::with_name("steplog-output").long("steplog-output").help("Save step log to file").takes_value(true))
        .arg(
            Arg::with_name("tx-file")
                .long("tx-file")
                .short("f")
                .help("Filename containing JSON formatted transaction dump")
                .takes_value(true),
        )
        .arg(Arg::with_name("vm-id").long("vm-id").default_value(&default_vm_id).help("VM ID").takes_value(true))
        .get_matches();

    let matches_args = matches.values_of("args").unwrap_or_default();
    let matches_bin = matches.value_of("bin");
    let matches_cell_index = matches.value_of("cell-index");
    let matches_cell_type = matches.value_of("cell-type");
    let matches_coverage_output = matches.value_of("coverage-output");
    let matches_dump_file = matches.value_of("dump-file");
    let matches_enable_coverage = matches.is_present("enable-coverage");
    let matches_enable_flamegraph = matches.is_present("enable-flamegraph");
    let matches_enable_overlapping_detection = matches.is_present("enable-overlapping-detection");
    let matches_enable_steplog = matches.is_present("enable-steplog");
    let matches_flamegraph_output = matches.value_of("flamegraph-output");
    let matches_gdb_listen = matches.value_of("gdb-listen").unwrap();
    let matches_max_cycles = matches.value_of("max-cycles").unwrap();
    let matches_mode = matches.value_of("mode").unwrap();
    let matches_prompt = matches.is_present("prompt");
    let matches_read_file_name = matches.value_of("read-file");
    let matches_script = matches.value_of("script");
    let matches_script_group_type = matches.value_of("script-group-type");
    let matches_script_hash = matches.value_of("script-hash");
    let matches_script_version = matches.value_of("script-version").unwrap();
    let matches_steplog_output = matches.value_of("steplog-output");
    let matches_tx_file = matches.value_of("tx-file");
    let matches_vm_id = u64::from_str_radix(matches.value_of("vm-id").unwrap(), 10).unwrap();
    let matches_mode = if matches_enable_coverage
        | matches_enable_flamegraph
        | matches_enable_overlapping_detection
        | matches_enable_steplog
    {
        "full"
    } else {
        matches_mode
    };
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
                                    .hash_type(ScriptHashType::Type)
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
                            .hash_type(ScriptHashType::Type)
                            .build(),
                    )
                    .build_exact_capacity(Capacity::zero())?,
                out_point: OutPoint::new(Byte32::from_slice(&vec![0x00; 32])?, 1),
                ..Default::default()
            };

            let mut mock_info = MockInfo::default();
            mock_info.cell_deps.push(MockCellDep {
                cell_dep: CellDep::new_builder().out_point(cell_meta_lock.out_point).dep_type(DepType::Code).build(),
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
            _ => unreachable!(),
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
    let verifier_program_elf = || -> Bytes {
        for e in &mut verifier_resolve_transaction.resolved_cell_deps {
            if e.out_point == verifier_script_out_point {
                return e.mem_cell_data.clone().unwrap();
            }
        }
        unreachable!()
    }();
    let verifier_hardforks = hardfork::HardForks {
        ckb2021: hardfork::CKB2021::new_mirana().as_builder().rfc_0032(20).build().unwrap(),
        ckb2023: hardfork::CKB2023::new_mirana().as_builder().rfc_0049(30).build().unwrap(),
    };
    let verifier_consensus = Arc::new(ConsensusBuilder::default().hardfork_switch(verifier_hardforks).build());
    let verifier_epoch = match verifier_script_version {
        ScriptVersion::V0 => ckb_types::core::EpochNumberWithFraction::new(15, 0, 1),
        ScriptVersion::V1 => ckb_types::core::EpochNumberWithFraction::new(25, 0, 1),
        ScriptVersion::V2 => ckb_types::core::EpochNumberWithFraction::new(35, 0, 1),
    };
    let verifier_header_view = HeaderView::new_advanced_builder().epoch(verifier_epoch.pack()).build();
    let verifier_tx_env = Arc::new(TxVerifyEnv::new_commit(&verifier_header_view));
    let verifier = TransactionScriptsVerifier::new(
        Arc::new(verifier_resolve_transaction.clone()),
        verifier_resource.clone(),
        verifier_consensus.clone(),
        verifier_tx_env.clone(),
    );
    let verifier_script_group = verifier.find_script_group(verifier_script_group_type, &verifier_script_hash).unwrap();
    let verifier_scheduler = verifier.create_scheduler(verifier_script_group).unwrap();
    let verifier_sg_data = verifier_scheduler.sg_data();

    if matches_mode == "fast" {
        let verifier_syscalls = |vm_id: &VmId,
                                 sg_data: &SgData<Resource>,
                                 vm_context: &VmContext<Resource>,
                                 vm_v: &Vec<Option<&str>>|
         -> Vec<Box<(dyn Syscalls<<Machine as DefaultMachineRunner>::Inner>)>> {
            let debug_printer: DebugPrinter = Arc::new(|_: &Byte32, message: &str| {
                let message = message.trim_end_matches('\n');
                if message != "" {
                    arch::println(&format!("{}", &format!("Script log: {}", message)));
                }
            });
            let mut sys_patch = generate_ckb_syscalls(vm_id, sg_data, vm_context, &debug_printer);
            if let Some(data) = vm_v[0] {
                sys_patch.push(Box::new(ElfDumper::new(data.to_string(), 4097, 64)));
            }
            sys_patch.push(Box::new(FileOperation::new()));
            if let Some(name) = vm_v[1] {
                sys_patch.push(Box::new(FileStream::new(name)));
            }
            sys_patch.push(Box::new(FileWriter::new()));
            sys_patch.push(Box::new(Random::new()));
            sys_patch.push(Box::new(Stdio::new(false)));
            sys_patch.push(Box::new(Timestamp::new()));
            return sys_patch;
        };
        let verifier: TransactionScriptsVerifier<Resource, Vec<Option<&str>>, Machine> =
            TransactionScriptsVerifier::new_with_generator(
                Arc::new(verifier_resolve_transaction.clone()),
                verifier_resource,
                verifier_consensus.clone(),
                verifier_tx_env.clone(),
                verifier_syscalls,
                vec![matches_dump_file, matches_read_file_name],
            );

        let mut scheduler = verifier.create_scheduler(verifier_script_group)?;
        scheduler.set_root_vm_args(matches_args.map(|s| Bytes::copy_from_slice(s.as_bytes())).collect());
        let result = scheduler.run(ckb_script::RunMode::LimitCycles(verifier_max_cycles));

        if result.is_err() {
            arch::println(&format!("Run result: {}", result.unwrap_err()));
            std::process::exit(254);
        }
        let result = result.unwrap();
        arch::println(&format!("Run result: {}", result.exit_code));
        arch::println(&format!("All cycles: {}", HumanReadableCycles(result.consumed_cycles)));
        if result.exit_code == 0 {
            std::process::exit(0);
        } else {
            std::process::exit(254);
        }
    }

    arch::println("Pre gather: collect vm creation");
    let collector: BinaryLocatorCollector<VmCreateCollector> = BinaryLocatorCollector::default();
    let collector_result = {
        let verifier: TransactionScriptsVerifier<Resource, _, Machine> = TransactionScriptsVerifier::new_with_generator(
            Arc::new(verifier_resolve_transaction.clone()),
            verifier_resource.clone(),
            verifier_consensus.clone(),
            verifier_tx_env.clone(),
            BinaryLocatorCollector::<VmCreateCollector>::syscall_generator,
            collector.clone(),
        );
        let collector_result = collector.collect(&verifier, verifier_script_group);
        match collector_result {
            Ok(data) => {
                arch::println(&format!("Run result: {}", data.exit_code));
                arch::println(&format!("All cycles: {}", HumanReadableCycles(data.cycles)));
                data
            }
            Err(err) => {
                arch::println(&format!("Run result: {}", err));
                std::process::exit(254);
            }
        }
    };
    let mut hint = HashMap::new();
    let mut tree = HashMap::new();
    for (k, v) in collector_result.traces {
        hint.insert(
            k.clone(),
            format!(
                "{}[{}][{}..{}]",
                match v.0.source {
                    0x1 => "input",
                    0x2 => "output",
                    0x3 => "cell_dep",
                    0x4 => "header_dep",
                    0x0100000000000001 => "group_input",
                    0x0100000000000002 => "group_output",
                    _ => unreachable!(),
                },
                v.0.index,
                v.0.offset,
                v.0.offset + v.0.length
            ),
        );
        if !v.1.vm_creations.is_empty() {
            tree.insert(k.clone(), v.1.vm_creations.clone());
        }
    }
    if !tree.is_empty() {
        print_vm_tree_recursive(&tree, &hint, CollectorKey { vm_id: 0, generation_id: 0 }, "", false);
    }

    arch::println(&format!("Pre gather: collect syscalls"));
    let collector: BinaryLocatorCollector<SyscallBasedCollector> = BinaryLocatorCollector::default();
    let collector_result = {
        let verifier: TransactionScriptsVerifier<Resource, _, Machine> = TransactionScriptsVerifier::new_with_generator(
            Arc::new(verifier_resolve_transaction),
            verifier_resource,
            verifier_consensus,
            verifier_tx_env,
            BinaryLocatorCollector::<SyscallBasedCollector>::syscall_generator,
            collector.clone(),
        );
        let collector_result = collector.collect(&verifier, verifier_script_group);
        match collector_result {
            Ok(data) => {
                arch::println(&format!("Run result: {}", data.exit_code));
                arch::println(&format!("All cycles: {}", HumanReadableCycles(data.cycles)));
                data
            }
            Err(err) => {
                arch::println(&format!("Run result: {}", err));
                std::process::exit(254);
            }
        }
    };

    arch::println(&format!("Actual run: start"));
    let machine_collector_key = CollectorKey { vm_id: matches_vm_id, generation_id: 0 };
    let machine_program_elf = match collector_result.traces.get(&machine_collector_key) {
        Some(data) => {
            let binary_locator = &data.0;
            let binary_locator_end = binary_locator.offset + binary_locator.length;
            let binary = if binary_locator.from_cell {
                match binary_locator.source {
                    0x1 => verifier_sg_data.rtx.resolved_inputs[binary_locator.index as usize]
                        .clone()
                        .mem_cell_data
                        .unwrap(),
                    0x2 => {
                        verifier_sg_data.tx_info.outputs[binary_locator.index as usize].clone().mem_cell_data.unwrap()
                    }
                    0x3 => verifier_sg_data.rtx.resolved_cell_deps[binary_locator.index as usize]
                        .clone()
                        .mem_cell_data
                        .unwrap(),
                    0x4 => {
                        unreachable!()
                    }
                    0x0100000000000001 => {
                        let i = verifier_sg_data.group_inputs().get(binary_locator.index as usize).unwrap();
                        verifier_sg_data.rtx.resolved_inputs[*i].clone().mem_cell_data.unwrap()
                    }
                    0x0100000000000002 => {
                        let i = verifier_sg_data.group_outputs().get(binary_locator.index as usize).unwrap();
                        verifier_sg_data.tx_info.outputs[*i].clone().mem_cell_data.unwrap()
                    }
                    _ => unreachable!(),
                }
            } else {
                verifier_sg_data.rtx.transaction.witnesses().get_unchecked(binary_locator.index as usize).as_bytes()
            }
            .slice(binary_locator.offset as usize..binary_locator_end as usize);
            binary
        }
        None => verifier_program_elf.clone(),
    };
    let machine_trace_data = match collector_result.traces.get(&machine_collector_key) {
        Some(data) => Bytes::copy_from_slice(&Vec::<u8>::from(data.1.clone())),
        None => Bytes::new(),
    };
    let machine_init = || {
        let mut machine_trace_impls = ProtobufVmRunnerImpls::new_with_bytes(machine_trace_data.clone()).unwrap();
        machine_trace_impls.set_debug_printer(Box::new(|message: &str| {
            let message = message.trim_end_matches('\n');
            if message != "" {
                arch::println(&format!("{}", &format!("Script log: {}", message)));
            }
        }));
        let machine_args: Vec<Bytes> = machine_trace_impls.args().iter().map(|e| Bytes::copy_from_slice(e)).collect();
        let machine_syscall = SynchronousSyscalls::new(machine_trace_impls);
        let machine_version = match verifier_script_version {
            ScriptVersion::V0 => ckb_vm::machine::VERSION0,
            ScriptVersion::V1 => ckb_vm::machine::VERSION1,
            ScriptVersion::V2 => ckb_vm::machine::VERSION2,
        };
        let machine_core = ckb_vm::DefaultCoreMachine::<u64, ckb_vm::WXorXMemory<ckb_vm::FlatMemory<u64>>>::new(
            ckb_vm::ISA_IMC | ckb_vm::ISA_B | ckb_vm::ISA_MOP,
            machine_version,
            verifier_max_cycles,
        );
        let machine_builder =
            ckb_vm::DefaultMachineBuilder::new(machine_core).instruction_cycle_func(Box::new(estimate_cycles));
        let mut machine = machine_builder.syscall(Box::new(machine_syscall)).build();

        machine.load_program(&machine_program_elf, machine_args.into_iter().map(Ok)).unwrap();
        machine
    };
    #[cfg(feature = "asm")]
    let machine_init_asm = || {
        let mut machine_trace_impls = ProtobufVmRunnerImpls::new_with_bytes(machine_trace_data.clone()).unwrap();
        machine_trace_impls.set_debug_printer(Box::new(|message: &str| {
            let message = message.trim_end_matches('\n');
            if message != "" {
                arch::println(&format!("{}", &format!("Script log: {}", message)));
            }
        }));
        let machine_args: Vec<Bytes> = machine_trace_impls.args().iter().map(|e| Bytes::copy_from_slice(e)).collect();
        let machine_syscall = SynchronousSyscalls::new(machine_trace_impls);
        let machine_version = match verifier_script_version {
            ScriptVersion::V0 => ckb_vm::machine::VERSION0,
            ScriptVersion::V1 => ckb_vm::machine::VERSION1,
            ScriptVersion::V2 => ckb_vm::machine::VERSION2,
        };
        let machine_core = <Box<ckb_vm::machine::asm::AsmCoreMachine> as SupportMachine>::new(
            ckb_vm::ISA_IMC | ckb_vm::ISA_B | ckb_vm::ISA_MOP,
            machine_version,
            verifier_max_cycles,
        );
        let machine_builder =
            ckb_vm::DefaultMachineBuilder::new(machine_core).instruction_cycle_func(Box::new(estimate_cycles));
        let machine = machine_builder.syscall(Box::new(machine_syscall)).build();
        let mut machine = ckb_vm::machine::asm::AsmMachine::new(machine);
        machine.load_program(&machine_program_elf, machine_args.into_iter().map(Ok)).unwrap();
        machine
    };

    if matches_mode == "full" {
        let machine = machine_init();
        let machine_coverage = MachineCoverage::new(&verifier_program_elf)?;
        let machine_flamegraph = MachineFlamegraph::new(&verifier_program_elf)?;
        let machine_overlap = MachineOverlap::new(&verifier_program_elf)?;
        let machine_steplog = MachineStepLog::new(matches_steplog_output.unwrap_or("ckb-debugger.steplog"));
        let mut machine =
            MachineAnalyzer::new(machine, machine_coverage, machine_flamegraph, machine_overlap, machine_steplog);
        if matches_enable_coverage {
            machine.enable_coverage = 1;
        }
        if matches_enable_flamegraph {
            machine.enable_flamegraph = 1;
        }
        if matches_enable_overlapping_detection {
            machine.enable_overlap = 1;
        }
        if matches_enable_steplog {
            machine.enable_steplog = 1;
        }
        let result = machine.run();
        match result {
            Ok(data) => {
                if let Some(fp) = matches_flamegraph_output {
                    let mut output = std::fs::File::create(&fp)?;
                    machine.flamegraph.display_flamegraph(&mut output);
                }
                if let Some(fp) = matches_coverage_output {
                    let mut output = std::fs::File::create(&fp)?;
                    machine.coverage.display_lcov(&mut output)?;
                }
                arch::println(&format!("Run result: {}", data));
                arch::println(&format!("All cycles: {}", HumanReadableCycles(machine.machine.cycles())));
                if data != 0 {
                    std::process::exit(254);
                }
                return Ok(());
            }
            Err(err) => {
                arch::println(&format!("Run result: {}", err));
                machine.flamegraph.display_stacktrace("", &mut std::io::stdout());
                arch::println("");
                arch::println(&format!("{}", machine));
                return Err(Box::new(err));
            }
        }
    }

    if matches_mode == "gdb" {
        let listener = TcpListener::bind(matches_gdb_listen)?;
        arch::println(&format!("Listening for gdb remote connection on {}", matches_gdb_listen));
        for res in listener.incoming() {
            if let Ok(stream) = res {
                arch::println(&format!("Accepted connection from: {}, booting VM", stream.peer_addr()?));
                let mut machine = machine_init();
                machine.set_running(true);
                let mut h = GdbStubHandler::new(machine);
                let connection: Box<(dyn ConnectionExt<Error = std::io::Error> + 'static)> = Box::new(stream);
                let gdb = GdbStub::new(connection);

                let result = match gdb.run_blocking::<GdbStubHandlerEventLoop<_, Riscv64>>(&mut h) {
                    Ok(disconnect_reason) => match disconnect_reason {
                        DisconnectReason::Disconnect => {
                            arch::println(&format!("GDB client has disconnected. Running to completion..."));
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
                        arch::println(&format!("Run result: {}", exit_code));
                        arch::println(&format!("All cycles: {}", HumanReadableCycles(cycles)));
                    }
                    Err(e) => {
                        arch::println(&format!("Error: {}", e));
                    }
                }
            }
        }
        return Ok(());
    }

    if matches_mode == "probe" {
        if matches_prompt {
            arch::println("Enter to start executing:");
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line).expect("read");
        }
        #[cfg(not(feature = "asm"))]
        let mut machine = machine_init();
        #[cfg(feature = "asm")]
        let mut machine = machine_init_asm().machine;
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
        arch::println(&format!("Run result: {:?}", result));
        arch::println(&format!("All cycles: {}", HumanReadableCycles(machine.cycles())));
    }

    Ok(())
}
