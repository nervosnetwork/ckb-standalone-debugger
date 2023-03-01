# CKB Standalone Debugger

CKB standalone debugger is a collection of debugging tools.

- [ckb-debugger](./ckb-debugger)
- [ckb-debugger-api](./ckb-debugger-api)
- [ckb-mock-tx-types](./ckb-mock-tx-types)
- [ckb-vm-debug-utils](./ckb-vm-debug-utils)
- [ckb-vm-pprof](./ckb-vm-pprof)

We provide a command line tool that allows you to develop contracts offline. To use it, simply type:

```sh
$ cargo build --release
$ export PATH=$PATH:$(pwd)/target/release
```

And then refer to the sample programs we provided [examples](./ckb-debugger/examples/)

# Licences

MIT
