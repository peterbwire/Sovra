# Syntax-independent captured package graphs

`packages::capture_package_sources(root, PackageCaptureLimits)` captures every
selected entry and registered module across a resolved local dependency graph.
Each file retains its own inventory index, canonical package owner and exact UTF-8
bytes. Manifest snapshots are the same bytes used by graph resolution. Files are
never concatenated, rewritten or parsed by this API. Generic source can therefore
be captured without routing it through the legacy executable parser.

The sealed `CapturedPackageSources` exposes immutable inventory, manifests and
source records, actual aggregate byte count and canonical root. Aliased dependency
paths share one canonical package identity; every graph edge remains retained.
Existing graph/path/manifest admission and guarded opened-handle source reads
apply. Duplicate opened file identities refuse. Supported reads currently require
Linux x86/x86_64/aarch64 with procfs inspection; no weaker-host fallback is added.
This is not an atomic filesystem transaction.

`PackageCaptureLimits` can reduce source count, aggregate source bytes and per-file
bytes. It cannot raise existing inventory, 64-MiB package or frontend-file ceilings.
Observed sizes are preflighted before source reads, and actual bytes are admitted
again through the guarded reader. Zero does not select an unlimited path. Capture
failure returns no partial object. Existing manifest/graph quotas remain separate.

`generate_captured_source_lock` and `verify_captured_source_lock` use the existing
strict version-one source-lock schema, including every captured source and manifest
and all dependency aliases. Their output matches the legacy parsed-source lock for
the same bytes. They neither reread sources nor write a lock. An existing lock can
be read and checked using `verify_existing_captured_source_lock`, through the same
guarded lock reader. Missing, malformed, stale or linked locks refuse with E4165.
Hashes establish byte integrity, not authentication or semantic validity.

This API supplies inputs rather than executable-linker authority. Capture and lock verification do not validate syntax/module headers,
resolve source imports, enforce language visibility, type-check or authorize
execution. Malformed source can be captured and fingerprinted. Generic Check/Run now compile registered graphs explicitly; multi-file native Build still refuses rather than falling back to an entry. No CLI flag or native image contract changes.

Next integration work must collect declarations with per-file owners, bind imports
under package/module visibility, preserve nominal and function identities across
the checked pipeline, map diagnostics to original files, and qualify locked
independent consumers and native images. Those requirements remain open. Editor
polish is deferred and Task Manager is unchanged.

Follow-on APIs now provide [owned declarations/import visibility](generic-package-declarations-v2.md) and [checked package types/signatures](generic-package-types-v2.md). Executable generic package linking remains open.

[Source-aware package body checking](generic-package-bodies-v2.md) now connects scoped calls and nominal operations to checked bodies, with explicit-library VM execution verified. CLI/native package integration remains open.

See [selected-root package compilation and commands](generic-package-compilation-v2.md) for current Check/Run/Lock support and current package suite/native support and remaining qualification gates.

[Generic package suites](generic-package-testing-v2.md) now run root-entry tests with dependency/module helpers, shared snapshot checking and all-root preflight. Native package builds now use [versioned source/topology images](generic-package-native-v2.md).

[Versioned generic package native images](generic-package-native-v2.md) now support source-backed native graph builds, with in-memory startup checking and separate embedded/capture authority.

Declaration tables now expose a `GenericPackageSources` origin view: filesystem callers can obtain `captured()` only for actual guarded captures; native image inputs return None and do not mint filesystem lock/read authority.
