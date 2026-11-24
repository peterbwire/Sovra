# Selected-root generic package compilation and commands

On qualified Linux/procfs hosts, generic Check/Run now support registered modules
and local dependencies through the source-aware compiler pipeline:

```sh
svr lock --experimental-generics ./project
svr check --experimental-generics --locked --format json ./project
svr run --experimental-generics --locked ./project
svr test --experimental-generics --locked --format json ./project
svr lock --experimental-generics --check ./project
```

Use registered `mod name { ... }` files and explicit file-local
`use dependency::module;` imports. Root entries contain ordinary/generic root
declarations after optional imports; inline entry modules remain unsupported.
All selected source files, types and bodies are checked, including unused
dependency functions. There is no entry-only fallback for registered graphs.

The owned `compile_generic_package(captured, entry, limits)` API consumes immutable
captured graph snapshots and selects only an unqualified function in the root
package's entry source. Qualified, missing, generic or parameterized roots refuse.
All existing bounded specialization, concrete recheck, literal, layout,
propagation-operation and IR stages run before a `CompiledGenericPackage` is
returned. Stage allowances are cumulative across the graph. The default instance
ceiling uses the approved 4096-instance package profile. No stage exhaustion
returns a partial program or selects an unlimited profile.

The compiled object retains source/type/body/IR proofs and can be cloned without
rereading disk inputs. Library compilation/execution can return supported values;
command Run separately requires a parameterless Unit main. Dependency entries'
main functions are never selected implicitly. `execute_generic_package` accepts
an explicit VM policy; CLI Run uses the existing finite host-free command policy.
No stdin, filesystem, process or networking hosts are introduced.

`location(instance, local_span)` maps a concrete function's local byte range into
its original captured file. It checks source/instance ownership, range and UTF-8
boundaries and recomputes scalar line/column coordinates rather than trusting
caller-provided coordinates. Runtime refusals become E3125 diagnostics in the
actual originating file, including dependency modules. Compile errors use E3123
or existing syntax/package codes. Aggregate failures can have null locations.
Source ownership and execution remain stable when disk inputs change or disappear.

Generic Lock now fingerprints every selected source and exact manifest using the
existing strict version-one schema and guarded atomic publisher. Locked Check/Run
capture the graph, verify the existing lock, then compile those exact snapshots.
Missing, stale, malformed or unsafe locks refuse before compiler/VM execution;
checks never rewrite them. Capture/publication is not a whole-filesystem atomic
transaction, authentication or semantic approval. Existing root-only locks remain
schema-compatible.

Package Test now uses [shared snapshot suites](generic-package-testing-v2.md),
with root-entry discovery, all-root preflight and finite aggregate admission.
Multi-file native Build now uses [versioned source/topology images](generic-package-native-v2.md)
with in-memory startup rechecking. Single-entry native images retain their previous
contracts. Package symbol evidence/editor overlays and unsupported-host filesystem
capture remain separate gates.

Tests cover owned selected roots, immutable execution after input removal,
dependency-file Unicode/CRLF runtime spans, invalid public location requests,
finite stage/runtime refusal, independently consumed generic CLI graphs, stale and
missing lock priority, unused dependency errors and native entry refusal without
output side effects. Native graph consumers now execute after project deletion.
Consolidated platform/consumer/backend qualification remains open. E1/E2/G13 remain open. Editor polish
is deferred and Task Manager is unchanged.

A runnable [generic local-package example](../../examples/generic-packages/README.md) consumes exported generic values and propagation helpers.

[Versioned generic package native images](generic-package-native-v2.md) now support source-backed native graph builds, with in-memory startup checking and separate embedded/capture authority.
