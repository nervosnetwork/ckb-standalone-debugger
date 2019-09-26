use ckb_standalone_debugger::run_json;
use std::fs::read_to_string;

#[test]
pub fn test_sample_json() {
    let mock_tx = read_to_string("tests/programs/sample.json").unwrap();
    let result = run_json(
        &mock_tx,
        "type",
        "0xa9d7502d89f7d3beeb5b184831257efd842da388ecedb2996296adaeab86839c",
        "1000",
    );
    assert_eq!(result, "{\"cycle\":513,\"error\":null}");
}
