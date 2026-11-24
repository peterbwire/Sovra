# Source-aware generic package body checking

`check_generic_package_bodies(types, body_limits, propagation_limits)` consumes
checked package types/signatures and checks every original function body. The
result retains both the package/source mappings and a sealed body table. It does
not select a root, emit an executable or enable CLI dependency imports.

Unqualified calls resolve only in the current source. Same-package module calls
and explicitly imported direct dependency/module calls use the caller's original
visibility context. Callee targets retain checked signature identities, so same-
named functions in separate sources cannot be confused. Active value bindings
shadow ordinary callee names and qualified call roots. Generic calls still require
explicit type arguments; ordinary calls must satisfy their checked signatures.
No unknown or inaccessible package call falls back to a global name scan.

Local annotations and explicit call type arguments use the caller's scoped type
registry. Record and enum constructors bind canonical nominal IDs, validate their
original qualified head tokens and reuse existing constructor substitution and
field/payload checking. Match arms resolve enum heads in the caller's scope and
must match the actual scrutinee's nominal ID; a same-named local enum cannot stand
in for an imported enum. Private types/functions and missing imports refuse.
Standard Option/Result propagation and the existing host-free builtins retain
their bounded profiles. No I/O host is introduced.

Each function keeps its original source fragment and original-file owner.
Qualified call facts retain the original callee path range, excluding grouping and
type arguments. Body/type/call errors map to the correct file and safe original
ranges; aggregate admission can have no source location. All original bodies,
including unused dependency functions, are checked before a proof is returned.

Existing body, call, annotation, comparison, field/payload and propagation budgets
apply cumulatively across the whole package. Qualified-name materialization and
original-token verification consume bounded admission. No per-file reset,
unchecked partial table or unbounded fallback is available. Flat body-checking
APIs still reject scoped signatures: callers must use this explicit package path.

Tests feed the resulting proof through existing explicit specialization, concrete
rechecking, literal decoding, layouts, propagation IR and the Rust VM. They execute
imported generic and ordinary calls, private helpers in their owning module,
transitive dependency helpers, remote records, imported enum matches and standard
propagation. Negative cases cover private/unimported/transitive visibility, active
shadowing, sibling import leakage, same-name nominal mismatches, invalid unused
bodies and exact cumulative call admission.

Selected-root compilation and CLI Check/Run now consume this library pipeline. Generic
CLI dependencies/modules remain refused. Owned selected-root compilation and cross-file diagnostics now back CLI Check/Run/Lock. Next work is consolidated native/backend/platform qualification.
Existing single-entry native feature words remain unchanged. E1/E2/G13 remain
open; editor polish is deferred and Task Manager is unchanged.

See [selected-root package compilation and commands](generic-package-compilation-v2.md) for current Check/Run/Lock support and current package suite/native support and remaining qualification gates.

[Generic package suites](generic-package-testing-v2.md) now run root-entry tests with dependency/module helpers, shared snapshot checking and all-root preflight. Native package builds now use [versioned source/topology images](generic-package-native-v2.md).

[Versioned generic package native images](generic-package-native-v2.md) now support source-backed native graph builds, with in-memory startup checking and separate embedded/capture authority.
