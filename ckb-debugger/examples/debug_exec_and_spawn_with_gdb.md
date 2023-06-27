# Debug exec and spawn with gdb

`ckb-debugger` has a special support for debugging programs which create additional processes using the `exec` or `spawn` function. If you have set `--gdb-specify-depth=N` on `ckb-debugger`, `ckb-debugger` will wait at the Nst `exec` or `spawn` syscall.

Consider the following pseudocode, let's use capital letters to denote a script:

```text
A:
  spawn B (where B is a script which will spawn C)
  spawn D
  exec E
```

- If we specify `--gdb-specify-depth=0`, `gdb` will attached the script A.
- If we specify `--gdb-specify-depth=1`, `gdb` will attached the script B.
- If we specify `--gdb-specify-depth=2`, `gdb` will attached the script C.
- If we specify `--gdb-specify-depth=3`, `gdb` will attached the script D.
- If we specify `--gdb-specify-depth=4`, `gdb` will attached the script E.

Let's use a real example shows how `gdb-specify-depth` works, suppose we want to debug the `spawn_callee_strcat` subscript, then you can do:

```sh
$ ckb-debugger --mode gdb --gdb-specify-depth 1 --gdb-listen 127.0.0.1:9999 --tx-file examples/spawn.json --cell-index 0 --cell-type input --script-group-type lock

$ riscv64-unknown-elf-gdb examples/spawn_callee_strcat
> target remote 127.0.0.1:9999
> l
```

You will see that you are in the `main` function of the `spawn_callee_strcat.c`.
