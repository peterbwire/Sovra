# Enum language tests

This independent source has no `main`. It tests an enum payload returned through
a linked helper and a nullary alternative, using a Unit alias for one test.
An unexpected outcome selects a nonzero status to fail the test.

```sh
svr test --experimental-v2 main.svr
svr test --experimental-v2 --format json main.svr
```

Both tests pass and capture `14` and `end`. The [experimental test
runner](../../docs/reference/experimental-v2-test.md) uses finite limits and no
stdin/filesystem services. Default CLI parsing and package/native/IDE integration
remain separate work. The fixture runs in Rust library and CLI regression tests.

Inspect complete linked lowering with `svr build --experimental-v2 --emit ir main.svr`.
This prints [bounded compiler IR](../../docs/reference/experimental-v2-ir.md) and
executes no source operations; it does not produce a native/JavaScript image.

Format this source without execution using `svr fmt --experimental-v2 main.svr`.
Use `--check` to inspect formatting needs or `--write` for guarded replacement; see
[v2 formatting](../../docs/reference/experimental-v2-format.md) for its safety gates.
