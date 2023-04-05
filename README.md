# CKB Standalone Debugger

CKB standalone debugger is a collection of debugging tools.

- [ckb-debugger](./ckb-debugger)
- [ckb-debugger-api](./ckb-debugger-api)
- [ckb-mock-tx-types](./ckb-mock-tx-types)
- [ckb-vm-debug-utils](./ckb-vm-debug-utils)
- [ckb-vm-pprof](./ckb-vm-pprof)

We provide a command line tool that allows you to develop CKB scripts offline. To install

```sh
cargo install --git https://github.com/nervosnetwork/ckb-standalone-debugger ckb-debugger
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
