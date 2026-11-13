# Registered primitive-function package example

Implemented through the source-owned compiler and executable CLI on its supported
guarded Linux hosts. Run `svr check examples/multi-file-packages/app` or
`svr run examples/multi-file-packages/app`; `build --emit js` emits the same program.
Locks and complete context/editor/package formatter integration remain planned.

The independent `app` package calls its registered `application` module. That file
explicitly imports `utilities::arithmetic` from a separate local library, whose
registered module exports `square` and keeps `product` private. A local primitive
alias crosses the function interface as canonical Int. Both the interpreter and
Node-executed JavaScript print `49` in `registered_package_functions` regressions.

```rust
use sovra::compiler::{interpreter, project::packages};
let package = packages::compile_registered_primitive_functions(
    "examples/multi-file-packages/app"
)?;
assert_eq!(interpreter::run(package.ir())?, ["49"]);
```

Source captures, checked-unit owner IDs and diagnostics remain file-owned. Every
private/unused body is checked; a module declaration never creates record/model
constructors or allows undeclared/transitive dependency imports.

On supported Linux/procfs hosts, explicitly generate a source lock with
`svr lock PATH_TO_APP`, then use `svr run --locked PATH_TO_APP` or
`svr check --executable-package --locked PATH_TO_APP`. `lock --check` verifies
without writing. Package formatting includes registered sources and leaves locks
untouched; regenerate explicitly after reviewed changes. See the
[lock contract](../../docs/reference/package-source-locks.md).
