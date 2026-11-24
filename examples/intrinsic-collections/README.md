# Intrinsic collection consumer

This executable/test package uses ordered intrinsic Map/Set operations, typed
entry/key/value snapshots, immutable remove/reinsert, Option matching and display.
It consumes an existing external generic `Box<T>` provider from
`../generic-packages/helpers` to verify imported ownership of standard entry values.
It intentionally has no Task Manager, UI, host-I/O or Node dependency.

From the repository root, using a current `svr` executable:

```sh
svr lock --experimental-generics examples/intrinsic-collections/app
svr check --experimental-generics --experimental-collections --locked examples/intrinsic-collections/app
svr run --experimental-generics --experimental-collections --locked examples/intrinsic-collections/app
svr test --experimental-generics --experimental-collections --locked examples/intrinsic-collections/app
svr build --experimental-generics --experimental-collections --locked --emit exe --output ./collection-consumer examples/intrinsic-collections/app
./collection-consumer
```

Use an `.exe` output name on Windows; actual collection executable launch evidence
is currently Linux. The integration suite copies this exact source into an
independent locked graph and runs generated executables after deleting that graph,
with an empty environment and nonexistent PATH. A separate single-entry package
consumer verifies that the collection native profile does not require dependencies.
No generated lock or executable is checked in. See the
[CLI/native contract](../../docs/reference/generic-collection-cli-v2.md).
