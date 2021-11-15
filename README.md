# ckb-standalone-debugger
A standalone debugger enabling off-chain contract development. Both a separate library, and a standalone binary is available for use.

# Usage

For Rust library usage, refer to the included tests, they are quite self-explanatory.

See the command line help part for usage on the binary debugger:

```
ckb-debugger 0.20.1

USAGE:
    ckb-debugger [FLAGS] [OPTIONS] --mode <mode> [args]...

FLAGS:
    -h, --help       Prints help information
        --step       Set to true to enable step mode, where we print PC address for each instruction
    -V, --version    Prints version information

OPTIONS:
        --bin <bin>                                File used to replace the binary denoted in the script
        --cell-index <cell-index>                  Index of cell to run
        --cell-type <cell-type>                    Type of cell to run [possible values: input, output]
        --dump-file <dump-file>                    Dump file name
        --gdb-listen <gdb-listen>                  Address to listen for GDB remote debugging server
        --max-cycles <max-cycles>                  Max cycles [default: 70000000]
        --mode <mode>
            Execution mode of debugger [default: full]  [possible values: full, fast, gdb]

        --pprof <pprof>                            Performance profiling, specify output file for further use
        --script-group-type <script-group-type>    Script group type [possible values: lock, type]
        --script-hash <script-hash>                Script hash
        --script-version <script-version>          Script version [default: 1]
        --skip-end <skip-end>                      End address to skip printing debug info
        --skip-start <skip-start>                  Start address to skip printing debug info
        --tx-file <tx-file>                        Filename containing JSON formatted transaction dump

ARGS:
    <args>...
```

[ckb-transaction-dumper](https://github.com/xxuejie/ckb-transaction-dumper) can be used to dump the full mocked transaction used in the debugger from CKB.
