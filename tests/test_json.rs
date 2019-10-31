use ckb_standalone_debugger::run_json;
use std::fs::read_to_string;

#[test]
pub fn test_sample_json() {
    let mock_tx = read_to_string("tests/programs/sample.json").unwrap();
    let result = run_json(
        &mock_tx,
        "type",
        "0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c",
        "4000",
    );
    assert_eq!(result, "{\"cycle\":3217,\"error\":null}");
}
