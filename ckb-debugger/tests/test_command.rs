use std::process::Command;
use std::sync::LazyLock;

static CKB_DEBUGGER: LazyLock<&str> = LazyLock::new(|| {
    let _ = Command::new("cargo").args(["build", "--release"]).output();
    "../target/release/ckb-debugger"
});

#[test]
pub fn test_always_failure_v0() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--bin", "examples/always_failure", "--script-version", "0"])
        .output()
        .unwrap()
        .stderr;
    let mut expect = vec![b"Error: MemWriteOnExecutablePage".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_always_failure_v1() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--bin", "examples/always_failure", "--script-version", "1"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![b"Run result: 1".to_vec(), b"All cycles: 2494(2.4K)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_always_failure_v2() {
    let result = Command::new(*CKB_DEBUGGER)
        .args(["--bin", "examples/always_failure", "--script-version", "2"])
        .output()
        .unwrap()
        .stdout;
    let mut expect = vec![b"Run result: 1".to_vec(), b"All cycles: 2493(2.4K)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
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
        b"All cycles: 8566(8.4K)".to_vec(),
    ]
    .join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_fib_pprof() {
    let output_path = tempfile::tempdir().unwrap().path().join("fib.pprof");
    let output = output_path.to_str().unwrap();
    let result =
        Command::new(*CKB_DEBUGGER).args(["--bin", "examples/fib", "--pprof", output]).output().unwrap().stdout;
    let mut expect = vec![b"Run result: 0".to_vec(), b"All cycles: 3364(3.3K)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
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
    let mut expect = vec![b"Run result: 1".to_vec(), b"All cycles: 2493(2.4K)".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result, expect);
}

#[test]
pub fn test_out_of_memory() {
    let result = Command::new(*CKB_DEBUGGER).args(["--bin", "examples/out_of_memory"]).output().unwrap();
    let mut expect = vec![
        b"??:??:??".to_vec(),
        b"/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/ckb-c-stdlib/libc/entry.h:9:_start".to_vec(),
        b"/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:25:main".to_vec(),
        b"/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:21:c".to_vec(),
        b"/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:17:b".to_vec(),
        b"/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:7:a".to_vec(),
        b"".to_vec(),
        b"pc  : 0x           12D28".to_vec(),
        b"zero: 0x               0 ra  : 0x           12D3E sp  : 0x          3FFFA0 gp  : 0x           146E8".to_vec(),
        b"tp  : 0x               0 t0  : 0x               0 t1  : 0x               0 t2  : 0x               0".to_vec(),
        b"s0  : 0x          3FFFB0 s1  : 0x               0 a0  : 0x               0 a1  : 0x          400000".to_vec(),
        b"a2  : 0x               0 a3  : 0x               0 a4  : 0x               0 a5  : 0x               0".to_vec(),
        b"a6  : 0x               0 a7  : 0x               0 s2  : 0x               0 s3  : 0x               0".to_vec(),
        b"s4  : 0x               0 s5  : 0x               0 s6  : 0x               0 s7  : 0x               0".to_vec(),
        b"s8  : 0x               0 s9  : 0x               0 s10 : 0x               0 s11 : 0x               0".to_vec(),
        b"t3  : 0x               0 t4  : 0x               0 t5  : 0x               0 t6  : 0x               0".to_vec(),
        b"".to_vec(),
    ]
    .join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result.stdout, expect);

    let mut expect = vec![b"Error: MemOutOfBound".to_vec()].join(&b'\n');
    expect.push(b'\n');
    assert_eq!(result.stderr, expect);
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
