# Matches in conditionals and loops

This [checked-program Rust fixture](../../docs/reference/control-match-v2.md)
prints `9`. Each loop iteration creates a fresh enum value, branches on the counter,
and exhaustively matches inside the selected branch. Arm and loop locals do not
escape. The fixture is executed in `tests/control_match_v2.rs`.

Use check_program_source and checked-program execute with explicit compiler/runtime
limits, as in [enum-functions](../enum-functions/README.md). Default CLI, earlier
standalone/single-entry parsers and JS/native/package/tooling integration remain
separate. This is a language fixture, not an installed/package application.

Check with `svr check --experimental-v2 main.svr` or execute with
`svr run --experimental-v2 main.svr`. Execution uses finite limits and no stdin/file
hosts. Default CLI execution and backend integration remain separate.

Inspect complete linked lowering with `svr build --experimental-v2 --emit ir main.svr`.
This prints [bounded compiler IR](../../docs/reference/experimental-v2-ir.md) and
executes no source operations; it does not produce a native/JavaScript image.

Format this source without execution using `svr fmt --experimental-v2 main.svr`.
Use `--check` to inspect formatting needs or `--write` for guarded replacement; see
[v2 formatting](../../docs/reference/experimental-v2-format.md) for its safety gates.
