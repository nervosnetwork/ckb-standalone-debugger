#!/bin/bash
# simple tests
../target/debug/ckb-debugger --tx-file=../../tests/programs/sample.json --script-group-type=type --script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c
../target/debug/ckb-debugger --tx-file=../../tests/programs/sample_data1.json --script-group-type=type --script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba

# test with pprof
riscv64-unknown-elf-gcc -O0 -g -o fib fib.c
../target/debug/ckb-debugger  --replace-binary=fib --pprof=output.txt \
--tx-file=../../tests/programs/sample_data1.json --script-group-type=type \
--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
inferno-flamegraph output.txt > fib.svg
