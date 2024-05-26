set -ex

ckb-debugger --mode decode-instruction 0x00054363

ckb-debugger --mode fast --bin examples/print_log
ckb-debugger --mode full --bin examples/print_log

ckb-debugger --mode fast --bin examples/fib
ckb-debugger --mode full --bin examples/fib --pprof /tmp/fib.pprof
ckb-debugger --mode full --bin examples/fib --enable-overlapping-detection
ckb-debugger --mode full --bin examples/fib --enable-steplog
ckb-debugger --mode fast --bin examples/fib --max-cycles 100 2>&1 | grep "ExceededMaximumCycles"
ckb-debugger --mode full --bin examples/fib --max-cycles 100 2>&1 | grep "CyclesExceeded"

ckb-debugger --mode fast --bin examples/out_of_memory 2>&1 | grep "MemOutOfBound"
ckb-debugger --mode full --bin examples/out_of_memory 2>&1 | grep "MemOutOfBound"

ckb-debugger --mode fast --tx-file examples/mock_tx.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file examples/mock_tx.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file examples/mock_tx.json --cell-index 0 --cell-type input --script-group-type lock --bin examples/always_failure 2>&1 | grep "Run result: 1"

ckb-debugger --mode fast --tx-file examples/spawn.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file examples/spawn.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file examples/spawn.json --cell-index 0 --cell-type input --script-group-type lock --pid 0 --pprof /tmp/spawn.pprof
ckb-debugger --mode full --tx-file examples/spawn.json --cell-index 0 --cell-type input --script-group-type lock --pid 1 --pprof /tmp/spawn.pprof

ckb-debugger --mode fast --tx-file examples/exec.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file examples/exec.json --cell-index 0 --cell-type input --script-group-type lock

ckb-debugger --mode full --bin examples/timenow
