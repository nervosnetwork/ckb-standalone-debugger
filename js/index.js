import sample from "../tests/programs/sample.json";
const rust = import('./pkg/ckb_standalone_debugger');

rust
  .then(({ run_json }) => {
    const result = run_json(
      JSON.stringify(sample),
      "type",
      "0xa9d7502d89f7d3beeb5b184831257efd842da388ecedb2996296adaeab86839c",
      "1000");
    const result_object = JSON.parse(result);
    console.log("Result: ", result_object);
  })
  .catch(console.error);
