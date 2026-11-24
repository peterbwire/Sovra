# Generic local-package consumer

This host-free example consumes an exported generic record and functions from a
registered dependency module, including Option propagation. Run from the repository
root with a freshly built compiler:

```sh
cargo build --bin svr
./target/debug/svr lock --experimental-generics examples/generic-packages/app
./target/debug/svr check --experimental-generics --locked examples/generic-packages/app
./target/debug/svr run --experimental-generics --locked examples/generic-packages/app
./target/debug/svr test --experimental-generics --locked examples/generic-packages/app
```

The consumer exits successfully without output when its checked value is 21.
A different value triggers a runtime failure. Lock includes both manifests and
all registered source files; changing any of them requires explicit regeneration.
The helper package entry is captured/checked but never invoked by the consumer.

Registered graphs currently support generic Check/Run/Lock on qualified Linux
hosts. Package Test runs the two root-entry tests with imported helpers, without
invoking main or dependency/module test functions. Native Build embeds the registered graph and runs without original source files. See the
[package compilation reference](../../docs/reference/generic-package-compilation-v2.md).

Build a source-backed native executable (Rust/Cargo and a linker required):

```sh
./target/debug/svr build --experimental-generics --locked --emit exe --output ./generic-app examples/generic-packages/app
./generic-app
```

The native image rechecks embedded source at startup and grants no I/O hosts or
arguments. See [generic package native images](../../docs/reference/generic-package-native-v2.md)
for limits and qualification scope.
