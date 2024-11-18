# Example mock-tx

We can download a transaction from the network and execute it locally. To download a transaction, use [ckb-cli](https://github.com/nervosnetwork/ckb-cli):

```sh
$ ckb-cli --url https://mainnet.ckbapp.dev/rpc mock-tx dump --tx-hash 0x5f0a4162622daa0e50b2cf8f49bc6ece22d1458d96fc12a094d6f074d6adbb55 --output-file mock_tx.json
```

You can execute the lock script or type script in the transaction:

```sh
$ ckb-debugger --tx-file mock_tx.json --cell-index 0 --cell-type input --script-group-type lock

Run result: 0
Total cycles consumed: 1697297(1.6M)
Transfer cycles: 12680(12.4K), running cycles: 1684617(1.6M)
```

The option `--bin` gives ckb-debugger a chance to "replace" the script, We try to replace the running code in the transaction above with a new version of the lock.

```c
// always_failure.c
int main() {
    return 1;
}
```

```sh
$ ckb-debugger --tx-file mock_tx.json --cell-index 0 --cell-type input --script-group-type lock --bin always_failure

Run result: 1
Total cycles consumed: 1706(1.7K)
Transfer cycles: 764, running cycles: 942
```

In addition to debugging transactions that have been committed to the chain, you can also debug transactions that are not committed to the chain (including unsent transactions and transactions that failed to be committed to the chain).

To do this, we first need to prepare a transaction in json format, the file content is the same as the payload of json rpc [send_transaction](https://github.com/nervosnetwork/ckb/tree/develop/rpc#method-send_transaction).

```sh
$ ckb-cli --url https://mainnet.ckbapp.dev/rpc mock-tx dump --tx-file mock_raw_tx.json --output-file mock_tx.json
```

In the above command, we use `--tx-file mock_raw_tx.json` to specify a transaction, which is equivalent to `--tx-hash 0x5f0a4162622daa0e50b2cf8f49bc6ece22d1458d96fc12a094d6f074d6adbb55`.
