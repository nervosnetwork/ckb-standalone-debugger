# Example out_of_memory

If your code triggers OOM, then ckb-debugger will print out the function call stack:

```sh
$ riscv64-unknown-elf-gcc -g -o out_of_memory out_of_memory.c

$ ckb-debugger --bin out_of_memory

Trace:
  ??:??:??
  ??:??:??
  /code/ckb-debugger/examples/out_of_memory.c:23:main
  /code/ckb-debugger/examples/out_of_memory.c:19:c
  /code/ckb-debugger/examples/out_of_memory.c:15:b
  /code/ckb-debugger/examples/out_of_memory.c:5:a
Error:
  MemOutOfBound
```
