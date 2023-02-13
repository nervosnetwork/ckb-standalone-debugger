# Example print_log

Tracing program execution by printing logs is a common operation. We will demonstrate how to use it.

First download the standard library:

```sh
$ git clone https://github.com/nervosnetwork/ckb-c-stdlib
```

Build it:

```sh
$ riscv64-unknown-elf-gcc -fno-builtin-printf -nostdinc -nostdlib -nostartfiles -I ./ckb-c-stdlib/libc -I ./ckb-c-stdlib -g -Wl,-static -o print_log print_log.c
```

Debug it:

```sh
$ RUST_LOG=debug ckb-debugger --bin print_log

DEBUG:<unknown>: SCRIPT>n = 5
DEBUG:<unknown>: SCRIPT>n = 4
DEBUG:<unknown>: SCRIPT>n = 3
DEBUG:<unknown>: SCRIPT>n = 2
DEBUG:<unknown>: SCRIPT>n = 1
DEBUG:<unknown>: SCRIPT>n = 0
DEBUG:<unknown>: SCRIPT>n = 1
DEBUG:<unknown>: SCRIPT>n = 2
DEBUG:<unknown>: SCRIPT>n = 1
DEBUG:<unknown>: SCRIPT>n = 0
DEBUG:<unknown>: SCRIPT>n = 3
DEBUG:<unknown>: SCRIPT>n = 2
DEBUG:<unknown>: SCRIPT>n = 1
DEBUG:<unknown>: SCRIPT>n = 0
DEBUG:<unknown>: SCRIPT>n = 1
```


# References

- <https://github.com/nervosnetwork/ckb-vm/discussions/193>
