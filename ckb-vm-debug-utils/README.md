# CKB VM Debug Utils

Utilities aiding CKB VM debug, including the following components:

* gdb remote debugging support
* standard IO components, so you can debug with printf as you wish

# How to use it

While this library contains components to plugin to your CKB VM runtime, we also prepare a bare metal binary showcasing how to use the components. Notice for now this binary only runs simple RISC-V programs, it doesn't support syscalls used in CKB. Later we might combine this with [ckb-standalone-debugger](https://github.com/nervosnetwork/ckb-standalone-debugger) to create a unified debugging experience for CKB.

```bash
$ cat program.c
int power(int, int);

int main() {
  int i, result;

  for (i = 0; i < 10; i++) {
    result += power(2, i);
  }
  return result;
}

int power(int base, int n) {
  int i, p;
  p = 1;
  for (i = 1; i <= n; i++) p = p * base;
  return p;
}
$ git clone https://github.com/nervosnetwork/ckb-vm-debug-utils
$ cd ckb-vm-debug-utils
$ cargo build
$ riscv64-unknown-elf-gcc -g ../program.c -o program
$ ./target/debug/baremetal 0.0.0.0:2000 program
```

Now CKB VM's debug server has been started, in a different terminal, we can launch gdb:

```bash
$ cd ckb-vm-debug-utils
$ gdb program
(gdb) target remote localhost:2000
Remote debugging using localhost:2000
0x00000000000100c8 in _start ()
(gdb) b main
Breakpoint 1 at 0x101ba: file program.c, line 6.
(gdb) c
Continuing.

Breakpoint 1, main () at program.c:6
6         for (i = 0; i < 10; i++) {
(gdb) s
7           result += power(2, i);
(gdb) print i
$1 = 0
(gdb)
```

As we can see, gdb works here.
