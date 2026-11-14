# Type-only module example (experimental v2)

```sh
svr check --experimental-v2 main.svr
svr run --experimental-v2 main.svr
svr test --experimental-v2 main.svr
svr fmt --experimental-v2 --check main.svr
```

The `models` namespace owns a record, enum and alias. Root functions construct and
match those qualified types; the program prints `7`. Only type declarations are
supported inside modules. Module functions, imports and dependency linking remain
unsupported. This is a same-file language example, not a published library or
package visibility claim. See [type module scope](../../docs/reference/type-modules-v2.md).
