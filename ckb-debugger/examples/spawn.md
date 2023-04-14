# Example spawn

syscall spawn is a new interface added since ckb2023.

```sh
$ ckb-debugger --tx-file examples/spawn.json --cell-index 0 --cell-type input --script-group-type lock
```

In order to simplify the writing of tx.json, we designed a simple DSL to assist you in writing tx.json.

```text
{{ data path/to/file }} => replact it by hexdata of path/to/file
{{ hash path/to/file }} => replace it by blake2b of path/to/file
{{ def_type any_type_id_name }} => create a new type_id named any_type_id_name
{{ ref_type any_type_id_name }} => refer to the type_id above
```

Open `spawn.json` to see how we used the DSL.
