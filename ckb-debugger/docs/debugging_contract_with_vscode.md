# Debugging Contracts with VSCode

The `ckb-debugger` provides a gdb-server mode for contract debugging. In VSCode, this requires the [Native Debug](https://marketplace.visualstudio.com/items?itemName=webfreak.debug) extension to connect to `ckb-debugger`'s gdb-server for debugging.
* **Note:** While CodeLLDB supports remote debugging, it does not work well in this case due to its use of `LLDB` version 17.

## Compile

The compiled contract must include the appropriate symbol files for debugging. For C, add `-g`; for Rust, set `strip = false` and `debug = true` in the `profile` section. After compiling, since binaries with symbol files tend to be large, you need to process them with `llvm-objcopy`:

```shell
cp build/ckb-debug/<Contract-Name> build/ckb-debug/<Contract-Name>.debug
llvm-objcopy --strip-debug --strip-all build/ckb-debug/<Contract-Name>
```

To configure `tasks.json` in VSCode (using the `c1` contract as an example):

```json
{
    "label": "Build Debug",
    "type": "shell",
    "command": "make build && cp build/ckb-debug/c1 build/ckb-debug/c1.debug && llvm-objcopy --strip-debug --strip-all build/ckb-debug/c1"
}
```

### Running `ckb-debugger`

(Skip this section if the contract you're debugging doesn't require transaction information.)

Typically, contracts require transaction data, which can be extracted from unit tests using the [dump_tx](https://github.com/nervosnetwork/ckb-testtool/blob/41c76ed2aef128fdf6e7e73b61d1bfa45e02b005/src/context.rs#L573) function. Add the following code before `context.verify_tx(&tx, MAX_CYCLES)` in your unit tests:

```rust
let tx_data = context.dump_tx(&tx).expect("dump tx info");
std::fs::write(
    "tx.json",
    serde_json::to_string_pretty(&tx_data).expect("json"),
).expect("write tx");
```

Then start `ckb-debugger` with:

```shell
ckb-debugger \
    --bin=build/ckb-debug/c1 \
    --mode=gdb_gdbstub \
    --gdb-listen=0.0.0.0:8000 \
    --tx-file=tests/tx.json \
    -s=lock \
    -i=0
```

* This example uses port 8000, but feel free to change it if needed.
* Adjust `-s` and `-i` according to your contract’s requirements.

You can configure `tasks.json` in VSCode to automatically start `ckb-debugger`:

```json
{
    "label": "Debug c1",
    "isBackground": true,
    "type": "process",
    "command": "ckb-debugger",
    "args": [
        "--bin=build/ckb-debug/c1",
        "--mode=gdb_gdbstub",
        "--gdb-listen=0.0.0.0:8000",
        "--tx-file=tests/tx.json",
        "-s=lock",
        "-i=0"
    ],
    "options": {
        "cwd": "${workspaceRoot}"
    }
}
```
* The `isBackground` setting ensures the task runs in the background and stays active during debugging.

Since `ckb-debugger` doesn’t automatically exit after debugging, you'll need a task to stop it:

```json
{
    "label": "stop-ckb-debugger",
    "type": "shell",
    "command": "killall ckb-debugger || true"
}
```

### GDB Debugging

```shell
gdb build/ckb-debug/c1.debug
```

Then connect to `ckb-debugger`'s GDB server:

```shell
target remote 127.0.0.1:8000
```

Once connected, you can debug using standard GDB commands.

### LLDB Debugging

* Ensure you are using LLDB version 18 or later.

```shell
lldb build/ckb-debugger/c1.debug
```

Then connect to `ckb-debugger`'s GDB server (you can omit the local address and just use the port):

```shell
gdb-remote 8000
```

After connecting, use standard LLDB commands for debugging.

### VSCode Debugging

* GDB must be installed.
* The Native Debug extension is required for debugging in VSCode.

First, configure `tasks.json` as described above. Then set up your `launch.json` for debugging:

```json
{
    "name": "GDB",
    "type": "gdb",
    "request": "attach",
    "executable": "build/ckb-debug/c1.debug",
    "debugger_args": [],
    "cwd": "${workspaceRoot}",
    "remote": true,
    "target": "127.0.0.1:8000",
    "preLaunchTask": "Debug c1",
    "postDebugTask": "stop-ckb-debugger"
}
```

After launching the debugger, you can set breakpoints and inspect variables as usual.
