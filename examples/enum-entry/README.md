# Source-owned enum entry

This example is executed by `tests/entry_source_v2.rs` through the opt-in Rust
[source-unit API](../../docs/reference/entry-source-v2.md). Its output is `7`.
The enum/type declarations and entry function are checked from this one `.svr`
file without caller-supplied metadata.

```rust
use sovra::compiler::v2::{
    check_entry_source, EntrySourceLimits, EntryLoweringLimits, RuntimeVmPolicy,
};
let unit = check_entry_source("main.svr", source, EntrySourceLimits::DEFAULT)?;
let outcome = unit.execute(&[], EntryLoweringLimits::DEFAULT, RuntimeVmPolicy::DEFAULT)?;
assert_eq!(outcome.output, vec!["7"]);
```

The snippet assumes source contains main.svr and the caller handles errors.
This is a language fixture, not an installed CLI/package application. Default
`svr run` does not yet select the extended enum pipeline. Additional functions,
modules/imports and JS/native/tooling integration remain open.

Check with `svr check --experimental-v2 main.svr` or execute with
`svr run --experimental-v2 main.svr`. Execution uses finite limits and no stdin/file
hosts. Default CLI execution and backend integration remain separate.

Inspect complete linked lowering with `svr build --experimental-v2 --emit ir main.svr`.
This prints [bounded compiler IR](../../docs/reference/experimental-v2-ir.md) and
executes no source operations; it does not produce a native/JavaScript image.
