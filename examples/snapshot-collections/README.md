# Ordered collection consumer

This runnable example depends on the local [reference package](../../stdlib/snapshot-collections/README.md).
From the repository root:

```sh
svr lock --experimental-generics examples/snapshot-collections
svr check --experimental-generics --locked examples/snapshot-collections
svr run --experimental-generics --locked examples/snapshot-collections
svr test --experimental-generics --locked examples/snapshot-collections
svr build --experimental-generics --locked --emit exe --output /tmp/collections-app examples/snapshot-collections
/tmp/collections-app
```

The consumer checks insertion order, replacement and duplicate set insertion.
Failures cause execution to fail. Native execution is host-free and needs neither
Node.js nor project files after building. This source-backed executable uses the
Rust runtime; direct machine-code lowering and other-host qualification remain open.
