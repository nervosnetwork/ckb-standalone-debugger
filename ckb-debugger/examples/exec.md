# Example exec

Execution of Exec scripts is no different from normal scripts:

```sh
$ ckb-debugger --tx-file examples/exec.json --cell-index 0 --cell-type input --script-group-type lock
```

If you want to use gdb to debug it, the way will be slightly more complicated:

```sh
$ ckb-debugger --tx-file examples/exec.json --cell-index 0 --cell-type input --script-group-type lock --mode gdb --gdb-listen 127.0.0.1:9999
$ riscv64-unknown-elf-gdb --command=examples/exec_gdb_cmd.txt
```

Let's see what `exec_gdb_cmd.txt` says:

```text
file examples/exec_caller          # Import symbols from exec_caller
target remote 127.0.0.1:9999       # Link to ckb-debugger
b __internal_syscall               # Set a breakpoint at __internal_syscall
c                                  # Fetch the 1st breakpoint: ckb_debug
c                                  # Fetch the 2nd breakpoint: ckb_exec
file examples/exec_callee          # Import symbols from exec_callee
b _start                           # Set a breakpoint at exec_callee's _start
c                                  # Continue
l                                  # List raw codes
```
