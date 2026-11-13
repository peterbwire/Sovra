# Typed array library and consumer

Implemented first G13 fixture. The `collections` package exposes reusable `total`,
`samples`, `empty` and `changed` operations from its explicitly registered `numbers`
module. An independent application consumes them through a direct local dependency.
This is a language/library fixture, not an application host or package registry.

```sh
svr check examples/typed-array-packages/app
svr run examples/typed-array-packages/app
svr build --emit js examples/typed-array-packages/app
```

Expected output in interpreter, Node JavaScript and tested Linux native output:

```text
9
1.5
0
1
4.5
```

`[Int]`, `[Float]` and nested annotations cross function and record interfaces.
Existing recursive numeric conversion widens Int elements at typed boundaries;
changing a copied array does not change its caller's value. `std::array_len`
counts elements, including zero for an empty array. General generics, resizing,
Map/Set, Option/Result and JSON/binary runtime APIs remain planned.

On supported Linux/procfs hosts, `svr lock PATH_TO_APP` followed by
`svr run --locked PATH_TO_APP` enforces the captured graph's source lock. The
registered source loader's [host limits](../../docs/reference/package-source-units.md)
still apply. Windows/macOS/MSRV and release qualification remain separate.
