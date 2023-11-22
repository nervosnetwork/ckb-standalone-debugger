# CKB debugger

A standalone debugger enabling off-chain contract development.

# Usage

For Rust library usage, refer to the included tests, they are quite self-explanatory.

See the command line help part for usage on the binary debugger:

```text
ckb-debugger 0.112.1

USAGE:
    ckb-debugger [FLAGS] [OPTIONS] --mode <mode> --tx-file <tx-file> [args]...

FLAGS:
    -h, --help       Prints help information
        --prompt     Set to true to prompt for stdin input before executing
        --step       Set to true to enable step mode, where we print PC address for each instruction
    -V, --version    Prints version information

OPTIONS:
        --bin <bin>                                File used to replace the binary denoted in the script
    -i, --cell-index <cell-index>                  Index of cell to run
    -t, --cell-type <cell-type>                    Type of cell to run [possible values: input, output]
        --decode <decode>                          Decode RISC-V instruction
        --dump-file <dump-file>                    Dump file name
        --gdb-listen <gdb-listen>                  Address to listen for GDB remote debugging server
        --gdb-specify-depth <gdb-specify-depth>    Specifies the depth of the exec/spawn stack [default: 0]
        --max-cycles <max-cycles>                  Max cycles [default: 70000000]
        --mode <mode>
            Execution mode of debugger [default: full]  [possible values: full, fast, gdb, probe, gdb_gdbstub]

        --pprof <pprof>                            Performance profiling, specify output file for further use
        --read-file <read-file>
            Read content from local file or stdin. Then feed the content to syscall in scripts

    -s, --script-group-type <script-group-type>    Script group type [possible values: lock, type]
        --script-hash <script-hash>                Script hash
        --script-version <script-version>          Script version [default: 2]
        --skip-end <skip-end>                      End address to skip printing debug info
        --skip-start <skip-start>                  Start address to skip printing debug info
    -f, --tx-file <tx-file>                        Filename containing JSON formatted transaction dump

ARGS:
    <args>...
```

[ckb-transaction-dumper](https://github.com/xxuejie/ckb-transaction-dumper) can be used to dump the full mocked transaction used in the debugger from CKB.

# FAQ

## How to Print Debug Message

1. [compile the contract via `-fno-builtin-printf` and replace with `CKB_C_STDLIB_PRINTF`](https://github.com/nervosnetwork/ckb-vm/discussions/193)
2. set `RUST_LOG=debug` to [enable](https://docs.rs/env_logger/latest/env_logger/) output [debug message](https://github.com/nervosnetwork/ckb-standalone-debugger/blob/eaeb6128837cc3103dbaa5eb61a1f49304935e5a/bins/src/main.rs#L266-L268)
