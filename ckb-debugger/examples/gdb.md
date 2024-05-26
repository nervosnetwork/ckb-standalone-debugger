# Example gdb debugging

This article introduces debugging programs by using gdb.

For example, we want to know the parameters passed in to a function at runtime:

```sh
$ ckb-debugger --mode gdb --gdb-listen 127.0.0.1:9999 --bin examples/fib
$ riscv64-unknown-elf-gdb examples/fib

$ (gdb) target remote 127.0.0.1:9999
$ (gdb) b fib
$ (gdb) c
    Breakpoint 1, fib (n=5) at fib.c:2
```

At the Breakpoint 1, we learn that fib (n=5) at fib.c:2.
