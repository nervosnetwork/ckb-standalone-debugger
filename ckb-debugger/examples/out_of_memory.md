# Example out_of_memory

If your code triggers OOM, then ckb-debugger will print out the function call stack:

```sh
$ make out_of_memory
$ ckb-debugger --bin out_of_memory

??:??:??
/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/ckb-c-stdlib/libc/entry.h:9:_start
/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:25:main
/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:21:c
/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:17:b
/home/ubuntu/src/ckb-standalone-debugger/ckb-debugger/examples/out_of_memory.c:7:a

pc  : 0x           12D28
zero: 0x               0 ra  : 0x           12D3E sp  : 0x          3FFFA0 gp  : 0x           146E8
tp  : 0x               0 t0  : 0x               0 t1  : 0x               0 t2  : 0x               0
s0  : 0x          3FFFB0 s1  : 0x               0 a0  : 0x               0 a1  : 0x          400000
a2  : 0x               0 a3  : 0x               0 a4  : 0x               0 a5  : 0x               0
a6  : 0x               0 a7  : 0x               0 s2  : 0x               0 s3  : 0x               0
s4  : 0x               0 s5  : 0x               0 s6  : 0x               0 s7  : 0x               0
s8  : 0x               0 s9  : 0x               0 s10 : 0x               0 s11 : 0x               0
t3  : 0x               0 t4  : 0x               0 t5  : 0x               0 t6  : 0x               0

Error: MemOutOfBound
```
