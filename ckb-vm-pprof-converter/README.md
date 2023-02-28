# ckb-vm-pprof-converter

This project converts raw data emitted by [ckb-vm-pprof](./ckb-vm-pprof) to profile.proto format supported by [pprof](https://github.com/google/pprof) for detailed analysis.

# Usage

Follow the steps from [ckb-vm-pprof](./ckb-vm-pprof/blob/master/README.md), but instead of generating flamegraphs at the last step, use:

```
cargo run -- --bin res/fib | ckb-vm-pprof-converter
```

This will generate a `output.pprof` file in local folder, which you can then load to pprof:

```
pprof res/fib output.pprof
```
