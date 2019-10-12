# ckb-standalone-debugger
A standalone debugger enabling off-chain contract development

# Usage

For Rust usage, refer to the included tests, they are quite self-explanatory.

The interesting part here, is that by compiling this project to a WASM target, you can run CKB's script debugger either in node.js or in a modern browser. We have included a short js package showcasing how you can do that.

To make this easier, we have published [an npm package](https://www.npmjs.com/package/ckb-standalone-debugger) which you can use directly. The `js` folder in the repo contains a minimal example calling the debugger. Follow those steps to set it up:

```bash
$ git clone https://github.com/nervosnetwork/ckb-standalone-debugger
$ cd ckb-standalone-debugger
$ cd js
$ npm install
$ npx webpack-dev-server
```

Note that you only need a valid `node.js` installation to play with this example, you don't need a Rust installation.

Now use your browser to open <http://localhost:8080>(or whatever address `webpack-dev-server` prompts in the terminal log), open developer tools, you will be able to find the script debugger's output.

# Build notes for wasm32-unknown-unknown target

If you are building for `x86_64` target, everything should works ok. However, due to [certain](https://github.com/alexcrichton/cc-rs/issues/446) [bugs](https://github.com/alexcrichton/cc-rs/issues/447), it's now only possible to build for the `wasm32-unknown-unknown` target under Linux using LLVM 8. So if you do want to build the Rust package on your own, make sure you use Linux with LLVM 8 installation, have latest [wasm-pack](https://github.com/rustwasm/wasm-pack) installed, then you can build via `wasm-pack build`.
