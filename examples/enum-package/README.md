# Single-entry enum package

This package selects `src/main.svr`, which returns an enum through linked helpers,
prints `14` from main and captures `ok` from its root test function.

From the repository root:

```sh
svr check --experimental-v2 examples/enum-package
svr run --experimental-v2 examples/enum-package
svr test --experimental-v2 --format json examples/enum-package
svr build --experimental-v2 --emit ir examples/enum-package
```

The [experimental package mode](../../docs/reference/experimental-v2-packages.md)
uses guarded captured bytes on supported Linux/procfs hosts. It supports one root
entry only: dependencies, registered modules and imports remain
unavailable. Runtime hosts stay denied by default, and selecting a package does
not change the caller's working directory. This is not full package/backend or
cross-platform qualification.

Pin/check the captured source inputs with:

```sh
svr lock --experimental-v2 examples/enum-package
svr lock --experimental-v2 --check examples/enum-package
svr run --experimental-v2 --locked examples/enum-package
```

[Lock verification](../../docs/reference/experimental-v2-locks.md) checks integrity
before compilation; it does not authenticate the publisher or replace source
checking. Multi-source packages and other-host/backend qualification remain open.

The example includes a generated version-one `sovra.lock`; changes to its manifest
or entry require an explicit lock update before using `--locked`.

Format the entry explicitly with
`svr fmt --experimental-v2 examples/enum-package/src/main.svr`. Default mode only
prints source. If you use `--write`, review the change and explicitly regenerate
the source lock before using locked commands. V2 package-wide formatting remains open.
