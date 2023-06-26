# Example exec

Execution of Exec scripts is no different from normal scripts:

```sh
$ ckb-debugger --tx-file examples/exec.json --cell-index 0 --cell-type input --script-group-type lock
```

But it you want run it in gdb mode, things has been difference:

```sh
$ ckb-debugger --tx-file examples/exec.json --cell-index 0 --cell-type input --script-group-type lock --mode gdb --gdb-listen 127.0.0.1:9999
$ riscv64-unknown-elf-gdb --command=examples/exec_gdb_cmd.txt
```
