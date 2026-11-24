# CLI execution of registered modules

Status: **Partial G12; executable command integration implemented on the guarded
Linux reader's supported hosts.** Registered source declarations, callable/record
interfaces and all selected bodies use the source-owned checked pipeline.

```text
svr check examples/multi-file-packages/app
svr check --executable-package --format json examples/multi-file-records/app
svr run examples/multi-file-records/app
svr build --emit js examples/multi-file-records/app
svr test --format json PATH_TO_REGISTERED_PACKAGE
```

The callable example prints `49`; the record example prints `25` then `1.5`.
The emitted JavaScript has the same outputs under Node. Default IR builds also
use complete linked checked IR. `build --emit exe` consumes that IR through the
existing native builder; this slice does not qualify new native hosts/artifacts.

## Selection and compatibility

Presence of `[modules]`, including an empty section, selects executable package
compilation. CLI classification also detects registrations in any declared local
dependency, including unused/transitive dependencies. It uses bounded manifest/
graph admission before choosing the compiler. Entry-only dependency packages keep
the older compiler path; projects with neither registrations nor dependencies
retain default partial application checking. Explicit `--executable-package` still
forces executable checking. Standalone `.svr` command behavior is unchanged.

`check`, `run`, `build` and package `test` select the registered record/callable
pipeline when required. All registered sources, private helpers and unused bodies
are checked. Unsupported/invalid inputs fail; there is no fallback to the partial
application scanner, entry-only compilation or unchecked lowering. Compilation
failure prevents execution/emission and cannot publish a partial native artifact.

`test` retains root-only discovery and test-host behavior: only eligible root entry
functions named `test_...` execute. Main, dependency test functions and functions
inside registered modules do not run automatically; their bodies are still
checked. Test diagnostics retain the root entry or actual failing module source.

Source-owned diagnostics use the existing JSON check schema/codes/zero-based
locations. Registered checking does not add a new JSON schema. CLI source/metadata/
interface/emission exhaustion retains ordinary failures and no unchecked fallback.
`--service-calls` remains an application-scanner mode and is refused for executable
registered/dependency packages rather than claiming complete DSL checking.

## Modes deliberately still unavailable

Compiler package context and source overlays/navigation retain
their strict entry-only resolver and reject `[modules]`, including dependency
registrations. A correctly framed overlay request is rejected on graph admission;
it is not silently reduced to entry sources. Editor tasks requiring that context
therefore remain unavailable until their complete source-inventory integration.
The public older `packages::resolve`/`compile` APIs are also unchanged; Rust callers
use the explicit source-owned compiler APIs for registered inputs.

[Explicit source locks](package-source-locks.md) now retain parsed manifest bytes
and verify the same captured inputs consumed by locked compilation. Unlocked
classification may perform extra bounded manifest reads; it is not lock verification
or an atomic filesystem snapshot. Module paths and graph/source/checked budgets
remain active. Package formatting now includes registered sources through the
compiler inventory; see [formatter scope](formatter.md). It never rewrites locks.
