/* Inspired from https://github.com/rustwasm/wasm_game_of_life/blob/fc35b7cf803ca4c706bb6498ada5754d95daf8e1/www/bootstrap.js */
import("./index.js")
  .catch(e => console.error("Error importing `index.js`:", e));
