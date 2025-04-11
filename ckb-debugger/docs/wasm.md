# WebAssembly

ckb-debugger supports compiling to wasm and wasm-wasi, so it can run in the browser or native nodejs.

However, its functionality is limited on browsers, in the browser, only the most basic transaction verification can be performed.

## Build wasm32-unknown-unknown

```sh
$ cargo install wasm-pack
$ wasm-pack build --target nodejs
```

wasm-pack will generate a directory for its build output called pkg. You can create a new nodejs project and then install the pkg with the following command:

```sh
$ npm install path/to/pkg
```

You can provide a `tx.json` file data and the script hash to execute a script group.

```js
import * as wasm from 'ckb-debugger';

const tx_file = fs.readFileSync('tx.json', 'utf8');
const result = wasm.run_json(tx_file, 'lock', '0x494e57d09aa7d17ad9559046fdab6a455811f6f86c5f6594b76de934d47e2553', '1000000000')
console.log(result)
```

## Build wasm32-wasip1

```sh
$ rustup target add wasm32-wasip1
$ sudo apt install gcc-multilib
$ cargo build --target wasm32-wasip1
```

The above command will compile a ckb-debugger wasm release with WASI. You can configure wasi according to your needs, the relevant documentation explains <https://nodejs.org/api/wasi.html>. For the following examples, we assume that ckb-debugger will read and execute the program /path/to/binary, so we need to give the real local path of this path in preopens.

```js
import * as fs from 'node:fs'
import * as wasi from 'node:wasi'

const wasihost = new wasi.WASI({
  version: 'preview1',
  args: ['ckb-debugger', '--bin', '/path/to/binary'],
  preopens: {
    '/path/to/binary': '/path/to/binary',
  },
});

async function main() {
  const wasm = await WebAssembly.compile(
    fs.readFileSync('/path/to/ckb-debugger.wasm'),
  );
  const instance = await WebAssembly.instantiate(wasm, wasihost.getImportObject());
  wasihost.start(instance);
}

main()
```
