use std::process::Command;
use std::sync::LazyLock;

static CKB_DEBUGGER: LazyLock<&str> = LazyLock::new(|| {
    let _ = Command::new("cargo").args(["build", "--release"]).output().unwrap();
    "../target/release/ckb-debugger"
});

#[test]
pub fn test_always_failure_v0() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--bin", "examples/always_failure", "--script-version", "0"])
        .output()
        .unwrap()
        .stdout;
    assert!(String::from_utf8(result).unwrap().contains("write on executable page"));
}

#[test]
pub fn test_always_failure_v1() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--bin", "examples/always_failure", "--script-version", "1"])
        .output()
        .unwrap()
        .stdout;
    assert!(String::from_utf8(result).unwrap().contains("Run result: 1"));
}

#[test]
pub fn test_always_failure_v2() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--bin", "examples/always_failure", "--script-version", "2"])
        .output()
        .unwrap()
        .stdout;
    assert!(String::from_utf8(result).unwrap().contains("Run result: 1"));
}

#[test]
pub fn test_dynamic() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--mode", "full", "--tx-file", "examples/dynamic.json"])
        .output()
        .unwrap()
        .stdout;
    let result: Vec<&str> = std::str::from_utf8(&result).unwrap().lines().collect();
    assert_eq!(result[result.len() - 2], "Run result: 0");
    assert_eq!(result[result.len() - 1], "All cycles: 2785573(2.7M)");
}

#[test]
pub fn test_exec() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--tx-file", "examples/exec.json", "--script", "input.0.lock"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![
        b"Script log: exec_caller".to_vec(),
        b"Script log: exec_callee".to_vec(),
        b"Run result: 0".to_vec(),
        b"All cycles: 83564(81.6K)".to_vec(),
    ]
    .join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_fib_flamegraph() {
    let outdir = tempfile::tempdir().unwrap();
    let output_path = outdir.path().join("fib.pprof");
    let output = output_path.to_str().unwrap();
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--mode", "full", "--bin", "examples/fib", "--flamegraph-output", output])
        .output()
        .unwrap()
        .stdout;
    let result: Vec<&str> = std::str::from_utf8(&result).unwrap().lines().collect();
    assert_eq!(result[result.len() - 2], "Run result: 0");
    assert_eq!(result[result.len() - 1], "All cycles: 1361(1.3K)");
}

#[test]
pub fn test_mock_tx() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--tx-file", "examples/mock_tx.json", "--script", "input.0.lock"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![b"Run result: 0".to_vec(), b"All cycles: 1641938(1.6M)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_mock_tx_replace_bin() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--tx-file", "examples/mock_tx.json", "--script", "input.0.lock", "--bin", "examples/always_failure"])
        .output()
        .unwrap()
        .stdout;
    assert!(String::from_utf8(result).unwrap().contains("Run result: 1"));
}

#[test]
pub fn test_instruction_decode() {
    let result =
        Command::new(*CKB_DEBUGGER).args(["--mode", "instruction-decode", "0x00054363"]).output().unwrap().stdout;
    let mut expect = vec![
        b"       Assembly = blt a0,zero,6".to_vec(),
        b"         Binary = 00000000000001010100001101100011".to_vec(),
        b"    Hexadecimal = 00054363".to_vec(),
        b"Instruction set = I".to_vec(),
    ]
    .join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_out_of_memory() {
    let result = Command::new(*CKB_DEBUGGER).args(["--bin", "examples/out_of_memory"]).output().unwrap().stdout;
    assert!(String::from_utf8(result).unwrap().contains("out of bound"));
}

#[test]
pub fn test_print_log() {
    let result = Command::new(*CKB_DEBUGGER).args(["--bin", "examples/print_log"]).output().unwrap().stdout;
    let mut expect = vec![
        b"Script log: n = 5".to_vec(),
        b"Script log: n = 4".to_vec(),
        b"Script log: n = 3".to_vec(),
        b"Script log: n = 2".to_vec(),
        b"Script log: n = 1".to_vec(),
        b"Script log: n = 0".to_vec(),
        b"Script log: n = 1".to_vec(),
        b"Script log: n = 2".to_vec(),
        b"Script log: n = 1".to_vec(),
        b"Script log: n = 0".to_vec(),
        b"Script log: n = 3".to_vec(),
        b"Script log: n = 2".to_vec(),
        b"Script log: n = 1".to_vec(),
        b"Script log: n = 0".to_vec(),
        b"Script log: n = 1".to_vec(),
        b"Run result: 0".to_vec(),
        b"All cycles: 41925(40.9K)".to_vec(),
    ]
    .join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_spawn() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--tx-file", "examples/spawn.json", "--script", "input.0.lock"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![b"Run result: 0".to_vec(), b"All cycles: 119776(117.0K)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_spawn_cycle_mismatch_tx() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--tx-file", "examples/spawn_cycle_mismatch_tx.json", "--script", "input.0.lock"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![b"Run result: 0".to_vec(), b"All cycles: 1652400(1.6M)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);

    let result = Command::new(*CKB_DEBUGGER)
        .args(["--tx-file", "examples/spawn_cycle_mismatch_tx.json", "--script", "output.0.type"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![b"Run result: 0".to_vec(), b"All cycles: 47533919(45.3M)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result[result.len() - expect.len()..], expect);
}
