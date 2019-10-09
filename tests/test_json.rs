use ckb_standalone_debugger::run_json;
use std::fs::read_to_string;

#[test]
pub fn test_sample_json() {
    let mock_tx = read_to_string("tests/programs/sample.json").unwrap();
    let result = run_json(
        &mock_tx,
        "type",
        "0x12bec80f9654173c0362fade816040de30b2a15f53c71e3f60570ca39ef8ebb0",
        "4000",
    );
    assert_eq!(result, "{\"cycle\":3527,\"error\":null}");
}
