# ckb-standalone-debugger
A standalone debugger enabling off-chain contract development

# Usage

For Rust usage, refer to the included tests, they are quite self-explanatory.

The interesting part here, is that by compiling this project to a WASM target, you can run CKB's script debugger either in node.js or in a modern browser. We have included a short js package showcasing how you can do that.

First, make sure you have [wasm-pack](https://github.com/rustwasm/wasm-pack) installed. Also make sure you are using a clang installation with WASM target enabled in the build(if you are using the official build of LLVM 8.0.0+, this should already work). Now you can try the following steps:

```bash
$ git clone https://github.com/nervosnetwork/ckb-standalone-debugger
$ cd ckb-standalone-debugger
$ cd js
$ npm install
$ npx webpack-dev-server
```

Now use your browser to open <http://localhost:8081>, open developer tools, you will be able to find the script debugger's output.

