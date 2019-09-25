use ckb_script::{ScriptGroupType, TransactionScriptsVerifier};
use ckb_sdk_types::transaction::{
    MockResourceLoader, MockTransaction, ReprMockTransaction, Resource,
};
use ckb_types::{
    bytes::Bytes,
    core::{cell::resolve_transaction, Cycle, HeaderView},
    packed::{Byte32, CellOutput, OutPoint},
    H256,
};
use faster_hex::hex_decode_fallback;
use serde_derive::{Deserialize, Serialize};
use serde_json::{from_str as from_json_str, to_string as to_json_string};
use serde_plain::from_str as from_plain_str;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

struct DummyResourceLoader {}

impl MockResourceLoader for DummyResourceLoader {
    fn get_header(&mut self, _hash: H256) -> Result<Option<HeaderView>, String> {
        return Err(
            "In standalone debugger, MockTransaction should provide all needed information!"
                .to_string(),
        );
    }

    fn get_live_cell(
        &mut self,
        _out_point: OutPoint,
    ) -> Result<Option<(CellOutput, Bytes)>, String> {
        return Err(
            "In standalone debugger, MockTransaction should provide all needed information!"
                .to_string(),
        );
    }
}

pub fn run(
    mock_tx: &MockTransaction,
    script_group_type: &ScriptGroupType,
    script_hash: &Byte32,
    max_cycle: Cycle,
    debug_printer: Option<Box<dyn Fn(&Byte32, &str)>>,
) -> Result<Cycle, String> {
    let resource = Resource::from_both(mock_tx, DummyResourceLoader {})?;
    let tx = mock_tx.core_transaction();
    let rtx = {
        let mut seen_inputs = HashSet::new();
        resolve_transaction(tx, &mut seen_inputs, &resource, &resource)
            .map_err(|err| format!("Resolve transaction error: {:?}", err))?
    };
    let mut verifier = TransactionScriptsVerifier::new(&rtx, &resource);
    if let Some(debug_printer) = debug_printer {
        verifier.set_debug_printer(debug_printer);
    }
    verifier
        .verify_single(script_group_type, script_hash, max_cycle)
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
) -> Result<Cycle, String> {
    let repr_mock_tx: ReprMockTransaction = from_json_str(mock_tx).map_err(|e| e.to_string())?;
    let mock_tx: MockTransaction = repr_mock_tx.into();
    let script_group_type: ScriptGroupType =
        from_plain_str(script_group_type).map_err(|e| e.to_string())?;
    if hex_script_hash.len() != 66 || (!hex_script_hash.starts_with("0x")) {
        return Err("Invalid script hash format!".to_string());
    }
    let mut b = [0u8; 32];
    hex_decode_fallback(&hex_script_hash.as_bytes()[2..], &mut b[..]);
    let script_hash = Byte32::new(b);
    let max_cycle: Cycle = max_cycle
        .parse()
        .map_err(|_| "Invalid max cycle!".to_string())?;
    // TODO: debug printer support
    run(&mock_tx, &script_group_type, &script_hash, max_cycle, None)
}

#[wasm_bindgen]
pub fn run_json(
    mock_tx: &str,
    script_group_type: &str,
    hex_script_hash: &str,
    max_cycle: &str,
) -> String {
    let json_result: JsonResult =
        internal_run_json(mock_tx, script_group_type, hex_script_hash, max_cycle).into();
    to_json_string(&json_result).expect("JSON serialization should not fail!")
}
