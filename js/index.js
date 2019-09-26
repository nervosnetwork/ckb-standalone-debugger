import sample from "../tests/programs/sample.json";
const rust = import('./pkg/ckb_standalone_debugger');

rust
  .then(({ run_json_with_printer }) => {
    const result = run_json_with_printer(
      JSON.stringify(sample),
      "type",
      "0xc5cdeb1ad5963030517a09152e9ad3ff6dcafc7a9fb915ef4698a5f4c6f1795c",
      "1000",
      (hash, message) => {
        console.log(`Script ${hash} generates debug message: ${message}`);
      });
    const result_object = JSON.parse(result);
    console.log("Result: ", result_object);
  })
  .catch(console.error);
