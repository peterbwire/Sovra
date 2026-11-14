# Linked enum functions

This opt-in Rust [checked-program execution](../../docs/reference/program-execution-v2.md)
fixture prints `14`. It checks all three functions and derives enum/record types
from one `.svr` file, then executes forward calls using canonical callee identities.
It runs in `tests/program_execution_v2.rs`.

```rust
use sovra::compiler::v2::{
    check_program_source, ProgramSourceLimits, ProgramLoweringLimits, RuntimeVmPolicy,
};
let program = check_program_source("main.svr", source, ProgramSourceLimits::DEFAULT)?;
let entry = program.function_id("main").expect("known example entry");
let outcome = program.execute(entry, &[], ProgramLoweringLimits::DEFAULT, RuntimeVmPolicy::DEFAULT)?;
assert_eq!(outcome.output, vec!["14"]);
```

The caller supplies source and handles errors. Default `svr run` does not yet
select the extended enum pipeline. Modules/imports, package/tooling
and JS/native integration remain open. This is a language fixture, not a packaged
or installed CLI application.

Check with `svr check --experimental-v2 main.svr` or execute with
`svr run --experimental-v2 main.svr`. Execution uses finite limits and no stdin/file
hosts. Default CLI execution and backend integration remain separate.

Inspect complete linked lowering with `svr build --experimental-v2 --emit ir main.svr`.
This prints [bounded compiler IR](../../docs/reference/experimental-v2-ir.md) and
executes no source operations; it does not produce a native/JavaScript image.

Format this source without execution using `svr fmt --experimental-v2 main.svr`.
Use `--check` to inspect formatting needs or `--write` for guarded replacement; see
[v2 formatting](../../docs/reference/experimental-v2-format.md) for its safety gates.
