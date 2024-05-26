set -ex

ckb-debugger --mode decode-instruction 0x00054363

ckb-debugger --mode fast --bin always_failure 2>&1 | grep "ValidationFailure"
ckb-debugger --mode full --bin always_failure 2>&1 | grep "Run result: 1"

ckb-debugger --mode fast --tx-file exec.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file exec.json --cell-index 0 --cell-type input --script-group-type lock

ckb-debugger --mode fast --bin fib
ckb-debugger --mode full --bin fib --pprof /tmp/fib.pprof
ckb-debugger --mode full --bin fib --enable-overlapping-detection
ckb-debugger --mode full --bin fib --enable-steplog
ckb-debugger --mode fast --bin fib --max-cycles 100 2>&1 | grep "ExceededMaximumCycles"
ckb-debugger --mode full --bin fib --max-cycles 100 2>&1 | grep "CyclesExceeded"

ckb-debugger --mode full --bin file_operations | grep "Run result: 0"

ckb-debugger --mode fast --tx-file mock_tx.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file mock_tx.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file mock_tx.json --cell-index 0 --cell-type input --script-group-type lock --bin always_failure 2>&1 | grep "Run result: 1"

ckb-debugger --mode fast --bin out_of_memory 2>&1 | grep "MemOutOfBound"
ckb-debugger --mode full --bin out_of_memory 2>&1 | grep "MemOutOfBound"

ckb-debugger --mode fast --bin print_log
ckb-debugger --mode full --bin print_log

ckb-debugger --bin read_file --read-file read_file.c | grep "Run result: 0"

ckb-debugger --mode fast --tx-file spawn.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file spawn.json --cell-index 0 --cell-type input --script-group-type lock
ckb-debugger --mode full --tx-file spawn.json --cell-index 0 --cell-type input --script-group-type lock --pid 0 --pprof /tmp/spawn.pprof
ckb-debugger --mode full --tx-file spawn.json --cell-index 0 --cell-type input --script-group-type lock --pid 1 --pprof /tmp/spawn.pprof

ckb-debugger --mode full --bin timenow
