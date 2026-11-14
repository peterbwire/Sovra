# Matches inside match arms

This [opt-in checked-program fixture](../../docs/reference/recursive-match-v2.md)
prints `7`, `0`, `0`. The helper unwraps a nested nominal enum using exhaustive
matches; every return path is checked. `tests/nested_match_v2.rs` executes this file.

Use check_program_source and CheckedProgramSource::execute with explicit limits,
as in the [linked function example](../enum-functions/README.md). Default svr run,
the earlier single-entry API and standalone match parser remain strict. [Matches inside if/while](../../docs/reference/control-match-v2.md) are now supported
by the program API. Broader CLI/backend/package/tooling support and G13 completion
remain open. This is a language fixture, not a packaged application.

Check with `svr check --experimental-v2 main.svr` or execute with
`svr run --experimental-v2 main.svr`. Execution uses finite limits and no stdin/file
hosts. Default CLI execution and backend integration remain separate.

Inspect complete linked lowering with `svr build --experimental-v2 --emit ir main.svr`.
This prints [bounded compiler IR](../../docs/reference/experimental-v2-ir.md) and
executes no source operations; it does not produce a native/JavaScript image.

Format this source without execution using `svr fmt --experimental-v2 main.svr`.
Use `--check` to inspect formatting needs or `--write` for guarded replacement; see
[v2 formatting](../../docs/reference/experimental-v2-format.md) for its safety gates.
