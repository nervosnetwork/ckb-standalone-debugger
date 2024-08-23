use ckb_mock_tx_types::{MockTransaction, ReprMockTransaction};

#[test]
pub fn test_run_json() {
    let mock_tx_repr_str = std::fs::read_to_string("examples/mock_tx.json").unwrap();
    let mock_tx_repr: ReprMockTransaction = serde_json::from_str(&mock_tx_repr_str).unwrap();
    let mock_tx: MockTransaction = mock_tx_repr.into();
    let script_hash = ckb_debugger::get_script_hash_by_index(&mock_tx, &ckb_script::ScriptGroupType::Lock, "input", 0);
    let script_hash_hex = hex::encode(script_hash.raw_data());
    let result = ckb_debugger::run_json(&mock_tx_repr_str, "lock", &script_hash_hex, "70000000");
    assert_eq!(result, "{\"cycle\":1641938,\"error\":null}");
}
