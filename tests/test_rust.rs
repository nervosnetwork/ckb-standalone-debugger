use ckb_script::ScriptGroupType;
use ckb_sdk_types::transaction::{MockCellDep, MockInfo, MockInput, MockTransaction};
use ckb_standalone_debugger::run;
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, DepType, ScriptHashType, TransactionBuilder},
    packed::{self, Byte32, BytesVec, CellDep, CellInput, CellOutput, OutPoint, Script, Witness},
    prelude::*,
};
use std::fs::File;
use std::io::Read;

fn read_file(name: &str) -> Bytes {
    let mut file = File::open(name).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    buffer.into()
}

fn create_mock_cell_dep(data: Bytes, lock: Option<Script>) -> (Byte32, MockCellDep) {
    let hash = CellOutput::calc_data_hash(&data);
    let hash2 = CellOutput::calc_data_hash(hash.as_slice());
    let out_point = OutPoint::new_builder().tx_hash(hash2).build();
    let cell_dep = CellDep::new_builder()
        .out_point(out_point)
        .dep_type(DepType::Code.pack())
        .build();
    let cell_output = CellOutput::new_builder()
        .capacity(Capacity::bytes(data.len() + 200).unwrap().pack())
        .lock(lock.unwrap_or_else(Script::default))
        .build();
    (
        hash,
        MockCellDep {
            cell_dep,
            output: cell_output,
            data,
        },
    )
}

#[test]
pub fn test_bench() {
    let data = read_file("tests/programs/bench.c");
    let code = read_file("tests/programs/bench");
    let script_args: Vec<packed::Bytes> =
        vec!["100".pack(), "100".pack(), "100".pack(), "100".pack()];

    let (_, data_dep) = create_mock_cell_dep(data, None);
    let (code_hash, code_dep) = create_mock_cell_dep(code, None);

    let script_args = BytesVec::new_builder()
        .extend(script_args.into_iter())
        .build();
    let script = Script::new_builder()
        .code_hash(code_hash.clone())
        .hash_type(ScriptHashType::Data.pack())
        .args(script_args)
        .build();
    let (_, input_dep) = create_mock_cell_dep(Bytes::from("abc"), Some(script));
    let script_hash = input_dep.output.calc_lock_hash();
    let cell_input = CellInput::new_builder()
        .previous_output(input_dep.cell_dep.out_point())
        .build();
    let cell_output = CellOutput::new_builder().build();
    let witness = Witness::new_builder().build();
    let transaction = TransactionBuilder::default()
        .input(cell_input.clone())
        .output(cell_output)
        .witness(witness)
        .output_data(packed::Bytes::default())
        .cell_dep(data_dep.cell_dep.clone())
        .cell_dep(code_dep.cell_dep.clone())
        .build();
    let mock_input = MockInput {
        input: cell_input,
        output: input_dep.output,
        data: input_dep.data,
    };
    let mock_info = MockInfo {
        inputs: vec![mock_input],
        cell_deps: vec![data_dep, code_dep],
        header_deps: vec![],
    };
    let mock_transaction = MockTransaction {
        mock_info,
        tx: transaction.data(),
    };

    let result = run(
        &mock_transaction,
        &ScriptGroupType::Lock,
        &script_hash,
        20_000_000,
        None,
    );
    assert!(result.is_ok());
}
