# Checked module function example (experimental v2)

```sh
svr check --experimental-v2 main.svr
svr run --experimental-v2 main.svr
svr test --experimental-v2 main.svr
svr fmt --experimental-v2 --check main.svr
```

An exported function calls a private qualified helper and returns its module's
aliased enum. Root code calls the exported function; direct root calls to the
private helper fail. The program prints `7`. This is same-file module linking,
not dependency imports or package publication. See [supported scope and limits](../../docs/reference/module-functions-v2.md).
