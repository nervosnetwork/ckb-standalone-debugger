# CKB Standalone Debugger

CKB standalone debugger is a collection of debugging tools.

- [ckb-debugger](./ckb-debugger)
- [ckb-vm-debug-utils](./ckb-vm-debug-utils)
- [ckb-vm-pprof](./ckb-vm-pprof)
- [ckb-vm-pprof-converter](./ckb-vm-pprof-converter)
- [ckb-vm-pprof-protos](./ckb-vm-pprof-protos)
- [ckb-vm-signal-profiler](./ckb-vm-signal-profiler)

We provide a command line tool that allows you to develop CKB scripts offline. To install

1. Download binarys from [Releases page](https://github.com/nervosnetwork/ckb-standalone-debugger/releases)
2. Or install from source code using cargo:

```sh
$ cargo install --locked --git https://github.com/nervosnetwork/ckb-standalone-debugger ckb-debugger
```

And then refer to the sample programs we provided [examples](./ckb-debugger/examples/)

# Notes

## macOS

On macOS, the `protoc` binary must be available to compile `ckb-vm-pprof-converter`. This can be installed via [homebrew](https://brew.sh/):

```bash
$ brew install protobuf
```

# Licences

MIT
