# Transparent enum aliases (experimental v2)

```sh
svr check --experimental-v2 main.svr
svr run --experimental-v2 main.svr
svr test --experimental-v2 main.svr
svr fmt --experimental-v2 --check main.svr
```

`Result` and `CopiedResult` name the same nominal `Outcome`; neither declares a
new enum. Alias constructors and match arms retain the canonical enum/variant
IDs. The example prints `7` and tests equality with canonical construction.
Existing payload widening applies; unrelated enums never become interchangeable.
Aliases to arrays, records and primitives are not enum constructor namespaces.
This is a root-source language example, not generic std::Result or package linking.
