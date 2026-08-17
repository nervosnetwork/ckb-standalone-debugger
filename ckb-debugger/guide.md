# CKB Debugger User Guide

`ckb-debugger` is an offline execution and debugging tool for CKB scripts: it can directly run local RISC-V ELF files, or load CKB mock transactions, reproducing script execution without sending a transaction.

## Build Commands

```sh
$ cd ckb-debugger
$ cargo build --release
```

The commands below assume the current directory is `ckb-debugger/examples`, and that the debugger has already been built at the repository root as `target/debug/ckb-debugger`. If the binary is already in `PATH`, you can replace `../../target/debug/ckb-debugger` with `ckb-debugger`.

## Transaction JSON

`--tx-file` accepts the mock transaction JSON used by `ckb-debugger`, not a normal transaction JSON that only contains `tx`. The top level must include both `mock_info` and `tx`. See `examples/mock_tx.json` for reference.

```json
{
    "mock_info": {
        "inputs": [],
        "cell_deps": [],
        "header_deps": []
    },
    "tx": {
        "version": "0x0",
        "cell_deps": [],
        "header_deps": [],
        "inputs": [],
        "outputs": [],
        "outputs_data": [],
        "witnesses": []
    }
}
```

The `mock_info` field provides local mock objects referenced by the transaction, allowing execution without contacting a CKB node:

- `inputs`: each item contains `input`, `output`, `data`, and optional `header`.
    - `input` contains `since` and `previous_output`.
    - `output` contains capacity, a lock script, and an optional type script.
    - `data` is the data for that input cell.
- `cell_deps`: each item contains `cell_dep`, `output`, `data`, and optional `header`.
    - The `out_point` and `dep_type` in `cell_dep` must match the references in `tx.cell_deps`.
    - `dep_type` is either `code` or `dep_group`.
    - For `code`, `data` is usually the script ELF; for `dep_group`, it is the encoded list of out points.
- `header_deps`: each item is a complete block header object. `tx.header_deps` stores only the hashes of those headers.

Inputs and cell deps in `mock_info` may include `header`, but when no header is present, the field should either be omitted or set to `null`. Every input, cell dep, and header dep used by the transaction must be provided in `mock_info`, and vice versa; the validator rejects missing or unused objects.

The `tx` field is the actual transaction content passed to the CKB VM:

- `version`: a hexadecimal string, for example `"0x0"`.
- `cell_deps`: only the cell deps actually used by the transaction are listed; each item contains `out_point` and `dep_type`.
- `header_deps`: an array of block header hashes referenced by the transaction.
- `inputs`: each item contains `since` and `previous_output`.
- `outputs`: each item contains `capacity`, `lock`, and optional `type`.
- `outputs_data`: each output has a corresponding cell data entry; the array length must match the number of `outputs`.
- `witnesses`: a byte array for witnesses; keep it as an empty array if no witness exists.

The format of a script object is as follows:

```json
{
    "code_hash": "0x<64 hex characters>",
    "hash_type": "data2",
    "args": "0x<even-length hex string>"
}
```

`hash_type` can be `data`, `data1`, `data2`, or `type`. `code_hash` must be a 32-byte hash, and `args`, cell data, witness, and other binary fields must be valid hex strings beginning with `0x`. Numeric values such as `capacity`, `since`, `index`, and `version` should also be written as strings beginning with `0x`, not as JSON numbers.

## Transaction JSON DSL

The Transaction JSON allows special placeholder DSLs to reference local files or generate type ID scripts. The DSL is only enabled when the file is read via `--tx-file <path>`. Placeholders may appear inside JSON strings; `def_type` can also directly replace a JSON script object. The syntax is as follows:

```text
{{ data path/to/file }}       -> file contents, encoded as hex
{{ hash path/to/file }}       -> Blake2b-256 hash of the file contents
{{ def_type name }}           -> define and generate a type ID script
{{ ref_type name }}           -> reference the previously defined type ID script hash
```

See `examples/spawn.json` for an example of its usage.

After reading the JSON file, the source expands all DSL placeholders in the following order:

1. Expand all `data`.
2. Expand all `hash`.
3. Scan and register all `def_type` names.
4. Replace `def_type` with the JSON script object.
5. Replace `ref_type` with the registered script hash.

Therefore, the paths for `data`/`hash` are relative to the Transaction JSON file, not to the shell's current working directory. `ref_type` does not need to appear after the `def_type` in the text; it is enough for a valid definition to exist in the same file. The DSL does not perform file expansion for stdin passed via `--tx-file -`; that input goes straight to JSON validation.

## Typical Scenarios and Recommended Commands

The commands below are listed by typical usage scenario for quick onboarding. These commands have been validated in the repository's `examples`, but they are not guaranteed to be reproducible in all environments. If you encounter problems, please refer to the source code and the scripts in `examples`, or read the source and tests directly.

### Quickly Running a Simple Local ELF

During development, you may sometimes want to run a simple algorithm without depending on any on-chain data. In that case, no transaction JSON is needed. The minimal command is:

```sh
$ ckb-debugger --bin fib
# Run result: 0
# All cycles: 3364(3.3K)
```

Use `--max-cycles` to cap the execution limit:

```sh
$ ckb-debugger --bin fib --max-cycles 1000000
# Run result: cycles error: max cycles exceeded
```

When the process exits with status code 0, it prints `Run result: 0`. When the script returns a non-zero value, the debugger prints that value and exits with a non-zero status. For example:

```sh
$ ckb-debugger --bin always_failure
# Run result: 1
# All cycles: 2493(2.4K)
```

When a VM error occurs, the error is printed directly. For example, the `out_of_memory` example currently prints `Run result: memory error: out of bound`.

```sh
$ ckb-debugger --bin out_of_memory
# Run result: memory error: out of bound
```

### Debugging an Already-On-Chain Transaction

An already on-chain transaction can be downloaded with `ckb-cli` in the format used by the debugger:

```sh
$ ckb-cli --url https://mainnet.ckbapp.dev/rpc \
    mock-tx dump \
    --tx-hash 0x5f0a4162622daa0e50b2cf8f49bc6ece22d1458d96fc12a094d6f074d6adbb55 \
    --output-file mock_tx.json
```

Run the lock script of an input cell:

```sh
$ ckb-debugger --tx-file mock_tx.json --script input.0.lock
```

The format of `--script` is `<cell-type>.<cell-index>.<script-group-type>`, for example `output.0.type`. It can also be written as:

```sh
$ ckb-debugger --tx-file mock_tx.json --cell-type input --cell-index 0 --script-group-type lock
```

If you want to test a newly compiled script in the context of a real transaction, replace the original script with `--bin`:

```sh
$ ckb-debugger --tx-file mock_tx.json --script input.0.lock --bin always_failure
```

### Debugging an Unconfirmed Transaction

An unconfirmed transaction can be prepared by first creating the raw transaction corresponding to the JSON-RPC `send_transaction` payload, then using `ckb-cli` to convert it to a mock transaction:

```sh
$ ckb-cli --url https://mainnet.ckbapp.dev/rpc mock-tx dump --tx-file mock_raw_tx.json --output-file mock_tx.json
```

The network RPC, `ckb-cli`, and the transaction's cell deps must all be available; in an offline environment, use the repository's `mock_tx.json`, `exec.json`, or prepare your own JSON.

### Source-Level Debugging with GDB

`ckb-debugger` supports GDB integration. Start the debugger with a RISC-V ELF built with `-g`:

```sh
$ ckb-debugger --mode gdb --gdb-listen 127.0.0.1:9999 --bin fib
```

Then open another terminal and connect:

```sh
riscv64-unknown-elf-gdb fib
(gdb) target remote 127.0.0.1:9999
(gdb) b fib
(gdb) c
```

The GDB mode listens on a TCP address via the `gdbstub` in the source; the debugger starts the VM only after a connection is received. If the port is occupied, use another address and let GDB connect to the same address. When debugging a transaction using `exec.json` in `examples`, start the debugger first and then connect GDB:

```sh
$ ckb-debugger --tx-file exec.json --script input.0.lock --mode gdb --gdb-listen 127.0.0.1:9999
$ riscv64-unknown-elf-gdb --command=exec_gdb_cmd.txt
```

`exec_gdb_cmd.txt` first loads the caller, stops at `__internal_syscall`, continues to `ckb_exec`, then loads the callee and stops at `_start`. When debugging `spawn` or `exec`, switch to the corresponding ELF according to the VM level; `--vm-id` selects the VM to analyze, and the default is 0.

### Performance Analysis with Flamegraph

Coverage, flamegraph, overlap detection, and step logs all require `full` mode. When these analysis switches are used, `ckb-debugger` automatically changes `--mode` to `full`. It is recommended that the ELF include DWARF debug information.

Use flamegraph to analyze function calls and execution time:

```sh
$ ckb-debugger --mode full --bin fib --enable-flamegraph --flamegraph-output fib.flamegraph
```

To convert the text output to SVG, install Inferno:

```sh
$ cargo install inferno
$ cat fib.flamegraph | inferno-flamegraph > fib.svg
```

### Code Coverage Analysis with Coverage

`ckb-debugger` records coverage in Linux LCOV format. First install the `lcov` tool:

```sh
$ sudo apt install lcov
```

Use the following commands to create a coverage record and convert it to an HTML page:

```sh
$ ckb-debugger --bin ed25519 --enable-coverage --coverage-output=coverage.lcov
$ genhtml coverage.lcov -o coverage_html
```

For performance reasons, the debugger records coverage as whether a given line was executed (line data is either 1 or 0), and does not count how many times the line was executed. Be aware of this when using it. Also, when using `genhtml` to generate the HTML page, the source code path must match the DWARF path in the ELF; otherwise the source code will not be displayed correctly in the HTML page. In our `examples`, since the `ed25519` source code is not included, the generated HTML page cannot display the source, but the coverage data remains valid.

### Stack and Heap Overlap Detection

Detect stack and heap overlap:

```sh
$ ckb-debugger --bin fib --enable-overlapping-detection
```

### Step Log

Record the PC and register state for each instruction for later analysis. This feature significantly slows execution and should only be used for debugging:

```sh
$ ckb-debugger --bin fib --enable-steplog --steplog-output fib.steplog
```

If `--steplog-output` is not specified, it defaults to writing to `ckb-debugger.steplog` in the current directory. Whether the analysis output contains content depends on the ELF's debug information and the actual execution path; a successful command exit does not guarantee that the coverage file contains records.

### Logs, Input, and File Syscalls

After compiling a script using the CKB C stdlib `printf` convention, set `RUST_LOG=debug` to view script logs:

```sh
$ RUST_LOG=debug ckb-debugger --bin print_log
```

`--read-file` reads a local file or stdin into the debugger and makes it available to the script's syscall 9000:

```sh
$ ckb-debugger --bin read_file --read-file read_file_input.txt
$ cat read_file_input.txt | ckb-debugger --bin read_file --read-file -
```

The repository's `file_operations` covers syscalls such as `fopen`, `fread`, `fseek`, and `fclose`; `file_write` uses syscall 9013 to write a file:

```sh
$ ckb-debugger --bin file_operations
$ ckb-debugger --bin file_write
```

File paths are interpreted according to the local filesystem when the debugger is running.

### Instruction Decoding

If you only want to view a single RISC-V instruction and do not need a transaction file:

```sh
$ ckb-debugger --mode instruction-decode 0x00000013
#        Assembly = addi zero,0(zero)
#          Binary = 00000000000000000000000000010011
#     Hexadecimal = 00000013
# Instruction set = I
```

## License

MIT
