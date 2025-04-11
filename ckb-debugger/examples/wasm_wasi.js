// ============================================================================
// $ cd ckb-debugger
// $ rustup target add wasm32-wasip1
// $ sudo apt install gcc-multilib
// $ cargo build --target wasm32-wasip1
//
// $ cd ckb-debugger/examples
// $ node wasm_wasi.js
// ============================================================================

const fs = require('node:fs')
const wasi = require('node:wasi')

const wasihost = new wasi.WASI({
    version: 'preview1',
    args: ['ckb-debugger', '--bin', 'file_write'],
    preopens: {
        '/': './',
    },
});

async function main() {
    const wasm = await WebAssembly.compile(
        fs.readFileSync('../../target/wasm32-wasip1/debug/ckb-debugger.wasm'),
    );
    const instance = await WebAssembly.instantiate(wasm, wasihost.getImportObject());

    wasihost.start(instance);
}

main()
