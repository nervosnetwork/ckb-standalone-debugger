#!/bin/bash
# simple tests
../target/debug/ckb-debugger --tx-file=../../tests/programs/sample.json --script-group-type=type --script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c
../target/debug/ckb-debugger --tx-file=../../tests/programs/sample_data1.json --script-group-type=type --script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba

# test with pprof
../target/debug/ckb-debugger  --replace-binary=../../deps/ckb-vm-pprof/res/fib --pprof \
--tx-file=../../tests/programs/sample_data1.json --script-group-type=type \
--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba \
| inferno-flamegraph > ../../deps/ckb-vm-pprof/res/fib.svg
