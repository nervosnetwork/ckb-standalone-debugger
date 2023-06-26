use ckb_chain_spec::consensus::ConsensusBuilder;
use ckb_jsonrpc_types::{CellDep, CellInput, DepType};
use ckb_mock_tx_types::{MockInput, MockResourceLoader, MockTransaction, ReprMockTransaction, Resource};
use ckb_script::{ScriptGroupType, TransactionScriptsVerifier, TxVerifyEnv};
use ckb_types::{
    bytes::Bytes,
    core::{cell::resolve_transaction, Cycle, HeaderView},
    packed::{self, Byte32, CellOutput, OutPoint, OutPointVec},
    prelude::*,
    H256,
};
use faster_hex::hex_decode_fallback;
use serde::{Deserialize, Serialize};
use serde_json::{from_str as from_json_str, to_string as to_json_string};
use serde_plain::from_str as from_plain_str;
use std::collections::HashSet;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
pub mod embed;

pub struct DummyResourceLoader {}

impl MockResourceLoader for DummyResourceLoader {
    fn get_header(&mut self, hash: H256) -> Result<Option<HeaderView>, String> {
        return Err(format!("Header {:x} is missing!", hash));
    }

    fn get_live_cell(&mut self, out_point: OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String> {
        return Err(format!("Cell: {:?} is missing!", out_point));
    }
}

pub fn run(
    mock_tx: &MockTransaction,
    script_group_type: &ScriptGroupType,
    script_hash: &Byte32,
    max_cycle: Cycle,
    debug_printer: Option<Box<dyn Fn(&Byte32, &str) + Sync + Send + 'static>>,
) -> Result<Cycle, String> {
    let resource = Resource::from_both(mock_tx, DummyResourceLoader {})?;
    let tx = mock_tx.core_transaction();
    let rtx = {
        let mut seen_inputs = HashSet::new();
        resolve_transaction(tx, &mut seen_inputs, &resource, &resource)
            .map_err(|err| format!("Resolve transaction error: {:?}", err))?
    };
    let consensus = Arc::new(ConsensusBuilder::default().build());
    let tx_env = Arc::new(TxVerifyEnv::new_commit(&HeaderView::new_advanced_builder().build()));
    let mut verifier =
        TransactionScriptsVerifier::new(Arc::new(rtx), resource.clone(), consensus.clone(), tx_env.clone());
    if let Some(debug_printer) = debug_printer {
        verifier.set_debug_printer(debug_printer);
    }
    verifier
        .verify_single(*script_group_type, script_hash, max_cycle)
        .map_err(|err| format!("Verify script error: {:?}", err))
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
struct JsonResult {
    cycle: Option<Cycle>,
    error: Option<String>,
}

impl From<Result<Cycle, String>> for JsonResult {
    fn from(result: Result<Cycle, String>) -> JsonResult {
        match result {
            Ok(cycle) => JsonResult {
                cycle: Some(cycle),
                error: None,
            },
            Err(error) => JsonResult {
                cycle: None,
                error: Some(error),
            },
        }
    }
}

fn internal_run_json(
    mock_tx: &str,
    script_group_type: &str,
    hex_script_hash: &str,
    max_cycle: &str,
    debug_printer: Option<Box<dyn Fn(&Byte32, &str) + Sync + Send + 'static>>,
) -> Result<Cycle, String> {
    let repr_mock_tx: ReprMockTransaction = from_json_str(mock_tx).map_err(|e| e.to_string())?;
    let mock_tx: MockTransaction = repr_mock_tx.into();
    let script_group_type: ScriptGroupType = from_plain_str(script_group_type).map_err(|e| e.to_string())?;
    if hex_script_hash.len() != 66 || (!hex_script_hash.starts_with("0x")) {
        return Err("Invalid script hash format!".to_string());
    }
    let mut b = [0u8; 32];
    hex_decode_fallback(&hex_script_hash.as_bytes()[2..], &mut b[..]);
    let script_hash = Byte32::new(b);
    let max_cycle: Cycle = max_cycle.parse().map_err(|_| "Invalid max cycle!".to_string())?;
    run(&mock_tx, &script_group_type, &script_hash, max_cycle, debug_printer)
}

#[wasm_bindgen]
pub fn run_json(mock_tx: &str, script_group_type: &str, hex_script_hash: &str, max_cycle: &str) -> String {
    let json_result: JsonResult =
        internal_run_json(mock_tx, script_group_type, hex_script_hash, max_cycle, None).into();
    to_json_string(&json_result).expect("JSON serialization should not fail!")
}

fn parse_dep_group_data(slice: &[u8]) -> Result<OutPointVec, String> {
    if slice.is_empty() {
        Err("data is empty".to_owned())
    } else {
        match OutPointVec::from_slice(slice) {
            Ok(v) => {
                if v.is_empty() {
                    Err("dep group is empty".to_owned())
                } else {
                    Ok(v)
                }
            }
            Err(err) => Err(err.to_string()),
        }
    }
}

pub fn check(tx: &ReprMockTransaction) -> Result<(), String> {
    let mut mock_cell_deps: Vec<CellDep> =
        tx.mock_info.cell_deps.iter().map(|c| c.cell_dep.clone()).collect::<Vec<_>>();
    let mut cell_deps = tx.tx.cell_deps.iter().map(|c| c.clone()).collect::<Vec<_>>();

    for dep in &tx.mock_info.cell_deps {
        if dep.cell_dep.dep_type == DepType::DepGroup {
            let sub_outpoints = parse_dep_group_data(dep.data.as_bytes())?;
            let outpoints: Vec<packed::OutPoint> = sub_outpoints.into_iter().collect::<Vec<_>>();
            let resolved_cell_deps: Vec<CellDep> = outpoints
                .into_iter()
                .map(|o| CellDep {
                    out_point: o.into(),
                    dep_type: DepType::Code,
                })
                .collect::<Vec<_>>();
            cell_deps.extend(resolved_cell_deps);
        }
    }
    let compare = |a: &CellDep, b: &CellDep| {
        let left = serde_json::to_string(a).unwrap();
        let right = serde_json::to_string(b).unwrap();
        left.cmp(&right)
    };
    mock_cell_deps.sort_by(compare);
    cell_deps.sort_by(compare);

    if mock_cell_deps.len() != cell_deps.len() {
        return Err(format!("mock_cell_deps.len() != cell_deps.len("));
    } else {
        for (a, b) in mock_cell_deps.into_iter().zip(cell_deps.into_iter()) {
            if a != b {
                return Err(format!("CellDeps {:?} != {:?}", a, b));
            }
        }
    }

    if tx.mock_info.inputs.len() != tx.tx.inputs.len() {
        return Err(format!("tx.mock_info.inputs.len() != tx.tx.inputs.len() "));
    } else {
        for i in 0..tx.mock_info.inputs.len() {
            let mock_input: MockInput = tx.mock_info.inputs[i].clone().into();
            let input: CellInput = tx.tx.inputs[i].clone();
            let input: packed::CellInput = input.into();
            if mock_input.input != input {
                return Err(format!("inputs at index {} is mismatched", i));
            }
        }
    }
    if tx.mock_info.header_deps.len() != tx.tx.header_deps.len() {
        return Err(format!("tx.mock_info.header_deps.len() != tx.tx.header_deps.len() "));
    }
    Ok(())
}
