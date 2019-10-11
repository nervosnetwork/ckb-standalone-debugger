import sample from "../tests/programs/sample.json";
import { run_json_with_printer } from "ckb-standalone-debugger";

const result = run_json_with_printer(
  JSON.stringify(sample),
  "type",
  "0x12bec80f9654173c0362fade816040de30b2a15f53c71e3f60570ca39ef8ebb0",
  "4000",
  (hash, message) => {
    console.log(`Script ${hash} generates debug message: ${message}`);
  });
const result_object = JSON.parse(result);
console.log("Result: ", result_object);
