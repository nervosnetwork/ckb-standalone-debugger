use ckb_chain_spec::consensus::ConsensusBuilder;
use ckb_mock_tx_types::{MockTransaction, ReprMockTransaction, Resource};
use ckb_script::{ScriptGroupType, TransactionScriptsVerifier, TxVerifyEnv};
use ckb_types::{
    core::cell::resolve_transaction,
    core::hardfork::{CKB2021, CKB2023, HardForks},
    core::{Cycle, EpochNumberWithFraction, HeaderView},
    packed::Byte32,
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

pub fn run(
    mock_tx: &MockTransaction,
    script_group_type: &ScriptGroupType,
    script_hash: &Byte32,
    max_cycle: Cycle,
) -> Result<Cycle, Box<dyn std::error::Error>> {
    let resource = Resource::from_mock_tx(mock_tx)?;
    let resolve_transaction =
        resolve_transaction(mock_tx.core_transaction(), &mut HashSet::new(), &resource, &resource)?;
    let hardforks = HardForks { ckb2021: CKB2021::new_dev_default(), ckb2023: CKB2023::new_dev_default() };
    let consensus = Arc::new(ConsensusBuilder::default().hardfork_switch(hardforks).build());
    let epoch = EpochNumberWithFraction::new(0, 0, 1);
    let header = HeaderView::new_advanced_builder().epoch(epoch.pack()).build();
    let tx_env = Arc::new(TxVerifyEnv::new_commit(&header));
    let verifier = TransactionScriptsVerifier::new_with_debug_printer(
        Arc::new(resolve_transaction),
        resource.clone(),
        consensus.clone(),
        tx_env.clone(),
        Arc::new(Box::new(move |_hash: &Byte32, message: &str| {
            let message = message.trim_end_matches('\n');
            if message != "" {
                crate::arch::println(&format!("Script log: {}", message));
            }
        })),
    );
    Ok(verifier.verify_single(*script_group_type, script_hash, max_cycle)?)
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
struct JsonResult {
    cycle: Option<Cycle>,
    error: Option<String>,
}

impl From<Result<Cycle, String>> for JsonResult {
    fn from(result: Result<Cycle, String>) -> JsonResult {
        match result {
            Ok(cycle) => JsonResult { cycle: Some(cycle), error: None },
            Err(error) => JsonResult { cycle: None, error: Some(error) },
        }
    }
}

#[wasm_bindgen]
pub fn run_json(mock_tx: &str, script_group_type: &str, script_hash: &str, max_cycle: &str) -> String {
    let result = || -> Result<Cycle, String> {
        let repr_mock_tx: ReprMockTransaction = serde_json::from_str(mock_tx).map_err(|e| e.to_string())?;
        let mock_tx: MockTransaction = repr_mock_tx.into();
        let script_group_type: ScriptGroupType = serde_plain::from_str(script_group_type).map_err(|e| e.to_string())?;
        let script_hash = if script_hash.starts_with("0x") { &script_hash[2..] } else { &script_hash[0..] };
        let script_hash_byte = hex::decode(&script_hash.as_bytes()).map_err(|e| e.to_string())?;
        let script_hash = Byte32::from_slice(script_hash_byte.as_slice()).map_err(|e| e.to_string())?;
        let max_cycle: Cycle = max_cycle.parse().map_err(|_| "Invalid max cycle!".to_string())?;
        run(&mock_tx, &script_group_type, &script_hash, max_cycle).map_err(|e| e.to_string())
    }();
    let result_json: JsonResult = result.into();
    serde_json::to_string(&result_json).unwrap()
}
