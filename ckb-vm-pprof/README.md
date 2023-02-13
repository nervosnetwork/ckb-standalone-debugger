# CKB VM PProf

This project profiling data in the format expected by the flamegraph visualization tool. It is a sub-project of [ckb-vm](https://github.com/nervosnetwork/ckb-vm).

# Usage

Suppose the program to be tested is:

```c
int fib(int n) {
    if (n == 0 || n == 1) {
        return n;
    } else {
        return fib(n-1) + fib(n-2);
    }
}

int main() {
    if (fib(10) != 55) {
        return 1;
    }
    return 0;
}
```

We should take the `-g` option on compiling for saving the debugging information:

```sh
$ riscv64-unknown-elf-gcc -g -o res/fib res/fib.c
```

To convert the textual representation of a flamegraph to a visual one, first install inferno:

```sh
$ cargo install inferno
```

Then, pass the file created by FlameLayer into inferno-flamegraph:

```sh
$ cargo run -- --bin res/fib | inferno-flamegraph > res/fib.svg
```

Open the svg:

![img](res/fib.svg)

# Know more about ckb-vm-pprof

- [ckb-vm-pprof-converter](https://github.com/xxuejie/ckb-vm-pprof-converter): This project converts raw data emitted by ckb-vm-pprof to profile.proto format supported by pprof for detailed analysis.

# Licences

MIT
