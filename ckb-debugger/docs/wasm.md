# WebAssembly

ckb-debugger supports compiling to wasm and wasm-wasi, so it can run in the browser or native nodejs.

However, its functionality is limited on browsers, in the browser, only the most basic transaction verification can be performed.

## Build wasm32-unknown-unknown

```sh
$ cargo install wasm-pack
$ wasm-pack build --target nodejs
```

```js
import * as wasm from 'ckb-debugger';

const tx_file = fs.readFileSync('tx.json', 'utf8');
const result = wasm.run_json(tx_file, 'lock', '0x494e57d09aa7d17ad9559046fdab6a455811f6f86c5f6594b76de934d47e2553', '1000000000')
console.log(result)
```

## Build wasm32-wasip1

```sh
$ cargo install wasm-pack
$ wasm-pack build --target nodejs
```

```js
import * as fs from 'node:fs'
import * as wasi from 'node:wasi'

const wasihost = new wasi.WASI({
  version: 'preview1',
  args: ['ckb-debugger', '--bin', '/path/to/binary'],
  preopens: {
    '/path/to/binary': '/path/to/binary',
    '/path/to/ckb-debugger.wasm': '/path/to/ckb-debugger.wasm',
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
