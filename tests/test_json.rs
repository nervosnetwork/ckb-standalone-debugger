use ckb_standalone_debugger::run_json;
use std::fs::read_to_string;

#[test]
pub fn test_sample_json() {
    let mock_tx = read_to_string("tests/programs/sample.json").unwrap();
    let result = run_json(
        &mock_tx,
        "type",
        "0xc5cdeb1ad5963030517a09152e9ad3ff6dcafc7a9fb915ef4698a5f4c6f1795c",
        "1000",
    );
    assert_eq!(result, "{\"cycle\":680,\"error\":null}");
}
