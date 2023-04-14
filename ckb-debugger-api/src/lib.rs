use ckb_jsonrpc_types::{CellDep, CellInput};
use ckb_mock_tx_types::{MockCellDep, MockInput, MockResourceLoader, MockTransaction, ReprMockTransaction, Resource};
use ckb_script::{ScriptGroupType, TransactionScriptsVerifier};
use ckb_types::{
    bytes::Bytes,
    core::{cell::resolve_transaction, Cycle, HeaderView},
    packed::{self, Byte32, CellOutput, OutPoint},
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

pub fn run<F: Fn(&Byte32, &str) + Sync + Send + 'static>(
    mock_tx: &MockTransaction,
    script_group_type: &ScriptGroupType,
    script_hash: &Byte32,
    max_cycle: Cycle,
    debug_printer: F,
) -> Result<Cycle, String> {
    let resource = Resource::from_both(mock_tx, DummyResourceLoader {})?;
    let tx = mock_tx.core_transaction();
    let rtx = {
        let mut seen_inputs = HashSet::new();
        resolve_transaction(tx, &mut seen_inputs, &resource, &resource)
            .map_err(|err| format!("Resolve transaction error: {:?}", err))?
    };
    let mut verifier = TransactionScriptsVerifier::new(Arc::new(rtx), resource);
    verifier.set_debug_printer(debug_printer);
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

fn internal_run_json<F: Fn(&Byte32, &str) + Sync + Send + 'static>(
    mock_tx: &str,
    script_group_type: &str,
    hex_script_hash: &str,
    max_cycle: &str,
    debug_printer: F,
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
        internal_run_json(mock_tx, script_group_type, hex_script_hash, max_cycle, |_, _| {}).into();
    to_json_string(&json_result).expect("JSON serialization should not fail!")
}

// use ckb_types::prelude::*;
// use js_sys::Function as JsFunction;
// use faster_hex::hex_encode_fallback;
// #[wasm_bindgen]
// pub fn run_json_with_printer(
//     mock_tx: &str,
//     script_group_type: &str,
//     hex_script_hash: &str,
//     max_cycle: &str,
//     // TODO: not sure if this works, test this or fix ckb-script in next
//     // release. We have to pass by value now since debug_priner in ckb-script
//     // requires static lifetime, and that wasm_bindgen doesn't support
//     // functions with lifetime parameters now.
//     debug_printer: JsFunction,
// ) -> String {
//     let rust_printer = move |hash: &Byte32, message: &str| {
//         let mut hex_bytes = [0u8; 64];
//         hex_encode_fallback(&hash.as_bytes(), &mut hex_bytes);
//         let hex_string = String::from_utf8(hex_bytes.to_vec()).expect("utf8 failiure");
//         let hex_string = format!("0x{}", hex_string).to_string();
//         debug_printer
//             .call2(&JsValue::NULL, &JsValue::from(&hex_string), &JsValue::from(message))
//             .expect("debug printer call should work");
//     };
//     let json_result: JsonResult = internal_run_json(
//         mock_tx,
//         script_group_type,
//         hex_script_hash,
//         max_cycle,
//         rust_printer,
//     )
//     .into();
//     to_json_string(&json_result).expect("JSON serialization should not fail!")
// }

pub fn check(tx: &ReprMockTransaction) -> Result<(), String> {
    if tx.mock_info.cell_deps.len() != tx.tx.cell_deps.len() {
        return Err(format!("Error: tx.mock_info.cell_deps.len() != tx.tx.cell_deps.len()"));
    } else {
        for i in 0..tx.mock_info.cell_deps.len() {
            let mock_cell_dep: MockCellDep = tx.mock_info.cell_deps[i].clone().into();
            let cell_dep: CellDep = tx.tx.cell_deps[i].clone();
            let cell_dep: packed::CellDep = cell_dep.into();
            if mock_cell_dep.cell_dep != cell_dep {
                return Err(format!("Error: cell_deps at index {} is mismatched", i));
            }
        }
    }
    if tx.mock_info.inputs.len() != tx.tx.inputs.len() {
        return Err(format!("Error: tx.mock_info.inputs.len() != tx.tx.inputs.len() "));
    } else {
        for i in 0..tx.mock_info.inputs.len() {
            let mock_input: MockInput = tx.mock_info.inputs[i].clone().into();
            let input: CellInput = tx.tx.inputs[i].clone();
            let input: packed::CellInput = input.into();
            if mock_input.input != input {
                return Err(format!("Error: inputs at index {} is mismatched", i));
            }
        }
    }
    if tx.mock_info.header_deps.len() != tx.tx.header_deps.len() {
        return Err(format!(
            "Error: tx.mock_info.header_deps.len() != tx.tx.header_deps.len() "
        ));
    }
    Ok(())
}
