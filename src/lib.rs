use ckb_script::{ScriptGroupType, TransactionScriptsVerifier};
use ckb_sdk_types::transaction::{MockResourceLoader, MockTransaction, Resource};
use ckb_types::{
    bytes::Bytes,
    core::{cell::resolve_transaction, Cycle, HeaderView},
    packed::{Byte32, CellOutput, OutPoint},
    H256,
};
use std::collections::HashSet;

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
