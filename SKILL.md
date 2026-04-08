---
name: ckb-standalone-debugger
description: Use when working on the ckb-standalone-debugger monorepo, especially for tasks involving offline CKB contract execution, mock transaction debugging, GDB sessions, flamegraph / coverage analysis, custom debugger syscalls, or WASM support.
---

# CKB Standalone Debugger Skill

Use this skill when working on the ckb-standalone-debugger monorepo, especially for tasks involving offline CKB contract execution, mock transaction debugging, GDB sessions, flamegraph / coverage analysis, custom debugger syscalls, or WASM support.

## Scope

This repository is a Rust workspace with these crates:

- `ckb-debugger`: the main CLI + library for offline script verification and debugging
- `ckb-vm-debug-utils`: GDB remote debugging helpers and stdio support
- `ckb-vm-pprof`: low-level profiling output for flamegraph-style analysis
- `ckb-vm-pprof-converter`: converts profiling output to pprof-compatible protobuf forms
- `ckb-vm-pprof-protos`: protobuf definitions and generated bindings
- `ckb-vm-signal-profiler`: Linux-only SIGPROF-based profiler for ckb-vm

Normally, we only focus on `ckb-debugger` CLI tool.

## What The Main Tool Does

`ckb-debugger` is a standalone debugger for Nervos CKB scripts. It can:

- execute a local binary directly as a simple mocked script
- execute a script group from a mocked transaction JSON file
- replace the script binary at runtime with `--bin`
- emulate on-chain verification using `TransactionScriptsVerifier`
- expose debugger-only syscalls for file I/O, timestamp, random, and file writing
- run in `fast`, `full`, `gdb`, `probe`, or `instruction-decode` modes
- export a minimal WASM API via `run_json`

## Read These Files First

When starting work, prioritize these files:

1. `ckb-debugger/src/main.rs`
2. `ckb-debugger/src/api.rs`
3. `ckb-debugger/src/lib.rs`
4. `ckb-debugger/tests/test_command.rs`
5. `ckb-debugger/tests/test_api.rs`
6. `ckb-debugger/examples/README.md`
7. `ckb-debugger/docs/*.md`

Then read supporting modules based on the task:

- execution / analysis: `machine_analyzer.rs`, `instruction_decode.rs`, `misc.rs`
- mock tx parsing / validation: `mock_tx_analyzer.rs`, `mock_tx_embed.rs`
- debugger syscalls: `syscall_*.rs`
- platform differences: `arch_unix.rs`, `arch_windows.rs`, `arch_wasm.rs`, `arch_wasm_wasi.rs`

## Source Map

### Public API

- `ckb-debugger/src/api.rs`
  - `run(...)`: minimal Rust API for running one script group from `MockTransaction`
  - `run_json(...)`: wasm-bindgen export for browser / Node.js usage
- `ckb-debugger/src/lib.rs`
  - re-exports the public API and most reusable helper types

### CLI Flow

`ckb-debugger/src/main.rs` does all major orchestration:

1. parse CLI args
2. load a mock transaction from `--tx-file`, stdin, or build a minimal one from `--bin`
3. optionally expand the mock-tx DSL via `mock_tx_embed`
4. validate the transaction JSON via `mock_tx_analyze`
5. resolve the selected script group and script hash
6. construct consensus / hardfork / epoch context based on script version
7. choose execution mode and run

### Analysis / Execution Helpers

- `machine_analyzer.rs`: wraps a VM to collect coverage, flamegraph, overlap detection, and step log
- `instruction_decode.rs`: decodes one RISC-V instruction to text
- `misc.rs`: cycle formatting, script lookup, VM tree printing
- `mock_tx_analyzer.rs`: validates mock tx structure and catches malformed inputs
- `mock_tx_embed.rs`: expands DSL placeholders inside mock transaction JSON

### Custom Debugger Syscalls

These syscalls are debugger conveniences, not on-chain CKB syscalls:

- `57`: native linux syscall: close
- `62`: native linux syscall: lseek
- `63`: native linux syscall: read
- `64`: native linux syscall: write
- `66`: native linux syscall: writev
- `80`: native linux syscall: fstat
- `4097`: dumps reconstructed ELF data when enabled internally (`syscall_elf_dumper.rs`)
- `9000`: file stream (`syscall_file_stream.rs`)
- `9001`: timestamp (`syscall_timestamp.rs`)
- `9002`: random (`syscall_random.rs`)
- `9003` to `9012`: file operations (`syscall_file_operation.rs`)
- `9013`: file_write (`syscall_file_write.rs`)

## Execution Modes

The current CLI modes in code are:

- `fast`: default mode, fastest execution path
- `full`: full analysis path; required for coverage / flamegraph / overlap / steplog
- `gdb`: starts a GDB remote server
- `probe`: emits USDT probes while stepping instructions
- `decode-instruction` and `instruction-decode`: decode one instruction and exit

Important: some markdown docs are older than the current code. Prefer `main.rs` over docs when they disagree.

Known doc drift to remember:

- current code uses `--mode gdb`, not `gdb_gdbstub`
- current code default mode is `fast`, even if some docs still show `full`

## Script Selection Model

The debugger selects one script group using either explicit flags or the shorthand `--script`.

Examples:

- `--script input.0.lock`
- `--script output.0.type`

Equivalent low-level fields:

- cell type: `input` or `output`
- cell index: zero-based index
- script group type: `lock` or `type`

If `--script-hash` is not provided, the debugger derives it from the mock transaction data.

## Script Version And Hardfork Behavior

The CLI maps script version to different epochs / hardfork contexts:

- V0 -> CKB2019 behavior
- V1 -> CKB2021 behavior
- V2 -> CKB2023 behavior

The current code builds verifier context using hardfork switches and chooses epochs based on script version. This matters for:

- available VM version
- instruction behavior
- spawn availability
- compatibility with old binaries

Do not assume a binary that works under V2 behaves the same under V0.

## Mock Transaction Workflow

### Basic Inputs

The debugger can run with:

- `--bin <path>` only: builds a tiny synthetic transaction around a local binary
- `--tx-file <json>`: loads a full mocked transaction
- `--tx-file -`: reads JSON from stdin

### Mock Transaction DSL

`mock_tx_embed.rs` supports a lightweight DSL inside tx JSON files:

- `{{ data path/to/file }}`: inline file contents as hex data
- `{{ hash path/to/file }}`: inline blake2b hash of file contents
- `{{ def_type name }}`: define a named type id
- `{{ ref_type name }}`: reference a previously defined type id

This is used heavily by `examples/spawn.json` and helps keep example JSON readable.

### Validation

Before execution, the debugger validates the expanded JSON structure. If a task involves failing or malformed mock transaction files, inspect `mock_tx_analyzer.rs` first.

## How Each Example Maps To A Workflow

Use the examples as the fastest path to understanding intended behavior:

- `examples/mock_tx.md` and `examples/mock_tx.json`
  - running a real or mocked transaction offline
  - replacing a script binary with `--bin`
- `examples/fib.md`, `fib.c`, `fib.flamegraph`, `fib.svg`
  - flamegraph workflow, GDB basics, debug symbol requirement
- `examples/print_log.md`, `print_log.c`
  - log / printf style debugging
- `examples/out_of_memory.md`, `out_of_memory.c`
  - OOM handling, call stack printing, register dump
- `examples/spawn.md`, `spawn.json`, `spawn_cycle_mismatch_tx.json`
  - CKB2023 spawn behavior and mock-tx DSL
- `examples/exec.md`, `exec.json`, `exec_caller*`, `exec_callee*`
  - exec workflow and multi-binary GDB debugging
- `examples/file_write.md`, `file_write.c`
  - debugger file_write syscall 9013
- `examples/file_operations.c`
  - fread / fwrite style syscall bridge examples
- `examples/read_file.c`
  - consuming local file / stdin data via `--read-file`
- `examples/timenow.c`
  - timestamp syscall usage
- `examples/dynamic.json`
  - dynamic script loading scenario
- `examples/always_failure.c`
  - version-specific behavior regression check

If you need expected output for a workflow, inspect `ckb-debugger/tests/test_command.rs` first. It is the best machine-checkable behavior summary in the repo.

## Documentation Guide

- `ckb-debugger/docs/debugging_contract_with_vscode.md`
  - VSCode + GDB workflow, symbol handling, `llvm-objcopy` stripping pattern
- `ckb-debugger/docs/coverage.md`
  - how to emit lcov and render HTML via `genhtml`
- `ckb-debugger/docs/probes.md`
  - USDT probe usage with `bpftrace` / BCC, function call tracing ideas
- `ckb-debugger/docs/wasm.md`
  - browser / Node.js / WASI usage boundaries
- `ckb-debugger/docs/elfutils.py`, `trace.py`, `riscv.h`
  - helper material for low-level tracing, not core CLI behavior

## Companion Crates

### `ckb-vm-debug-utils`

Use this when the task is about remote debugging or embedding GDB support into a different VM host. It provides:

- GDB stub handling
- event loop integration
- stdio helpers for debug-friendly environments

### `ckb-vm-pprof`

Use this when the task is about profiling output and flamegraph generation. It is lower-level than `ckb-debugger` but conceptually aligned.

### `ckb-vm-pprof-converter` and `ckb-vm-pprof-protos`

Use these when the task is about converting profiler output to protobuf / pprof tooling.

### `ckb-vm-signal-profiler`

Linux-only profiler using `SIGPROF`. Use for non-intrusive sampling when code injection into VM is undesirable.

## Working Rules For AI Agents

When changing code in this repo:

1. Treat `main.rs` as the source of truth for CLI behavior.
2. Treat tests as the source of truth for expected stdout and exit semantics.
3. Assume markdown docs may lag behind code. Update them only after verifying against source.
4. Keep script-version behavior explicit. Do not silently change default VM or hardfork assumptions.
5. If a change touches output text, check `tests/test_command.rs` for snapshot-style assertions.
6. If a change touches mock transaction JSON handling, inspect both `mock_tx_embed.rs` and `mock_tx_analyzer.rs`.
7. If a change touches remote debugging, inspect both `main.rs` GDB mode and `ckb-vm-debug-utils`.
8. If a change touches profiling or analysis flags, inspect `machine_analyzer.rs` and the related docs.
9. If a change touches browser / WASI behavior, inspect the `arch_wasm*.rs` files before editing shared logic.

## Common Command Patterns

### Run a local binary quickly

```sh
ckb-debugger --bin examples/print_log
```

### Run a mocked transaction

```sh
ckb-debugger --tx-file examples/mock_tx.json --script input.0.lock
```

### Replace on-chain script binary with a local one

```sh
ckb-debugger --tx-file examples/mock_tx.json --script input.0.lock --bin examples/always_failure
```

### Generate flamegraph data

```sh
ckb-debugger --mode full --bin examples/fib --enable-flamegraph --flamegraph-output fib.flamegraph
inferno-flamegraph < fib.flamegraph > fib.svg
```

### Generate coverage

```sh
ckb-debugger --mode full --bin path/to/binary --enable-coverage --coverage-output coverage.lcov
genhtml coverage.lcov -o coverage_html
```

### Start GDB mode

```sh
ckb-debugger --mode gdb --gdb-listen 127.0.0.1:9999 --tx-file examples/exec.json --script input.0.lock
```

### Decode a single instruction

```sh
ckb-debugger --mode instruction-decode 0x00054363
```

## Common Failure Modes

- `ScriptNotFound`
  - wrong `--script`, wrong `--script-hash`, or mismatched mock tx contents
- memory out-of-bound / OOM
  - inspect `examples/out_of_memory.md` behavior, stack trace, and register dump path
- unexpected exit code with no analysis output
  - confirm whether you are in `fast` or `full` mode
- no flamegraph symbol names
  - binary likely lacks debug info; compile with `-g`
- coverage HTML missing line counts
  - coverage is presence-only, not execution-frequency aware
- confusion around GDB docs
  - prefer `--mode gdb` from source instead of older markdown examples

## WASM Notes

The WASM support is intentionally narrower than native support.

- browser / `wasm32-unknown-unknown`
  - use `run_json`
  - only basic transaction verification is realistic
  - no normal filesystem
- `wasm32-wasip1`
  - can run in Node.js with WASI preopens
  - filesystem access depends on WASI configuration

If a user asks for browser debugging parity with native GDB / probe workflows, the correct answer is usually that the browser target only supports a reduced subset.

## Probe Notes

Probe mode emits USDT probes for instruction-level tracing. It is relevant for eBPF / BCC / bpftrace workflows, not ordinary debugging. Before editing probe behavior, inspect:

- the `probe!` call sites in `ckb-debugger/src/main.rs`
- `docs/probes.md`
- helper scripts in `docs/trace.py` and `docs/elfutils.py`

## Best Places To Learn Expected Behavior

If you only have time to read a few files before making a change, read these in order:

1. `ckb-debugger/src/main.rs`
2. `ckb-debugger/tests/test_command.rs`
3. `ckb-debugger/src/api.rs`
4. `ckb-debugger/examples/README.md`
5. the specific `examples/*.md` for the feature you are touching

## Good AI Responses In This Repo

A strong response for this repo should:

- distinguish `fast` vs `full` vs `gdb` vs `probe`
- state whether behavior comes from code, tests, or markdown docs
- call out outdated docs when necessary
- mention script version and mock-tx assumptions explicitly
- use existing examples as the baseline before proposing new workflows
- avoid claiming debugger-only syscalls are on-chain syscalls

## Bad Assumptions To Avoid

- do not assume docs are fully current
- do not assume default mode is `full`
- do not assume `gdb_gdbstub` is still a valid mode
- do not assume all platforms support the same file or profiling behavior
- do not assume flamegraph / coverage works without `--mode full`
- do not assume browser WASM can do full native debugging
