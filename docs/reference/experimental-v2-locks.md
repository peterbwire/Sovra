# Locked experimental single-entry packages

```sh
svr lock --experimental-v2 path/to/app
svr lock --experimental-v2 --check path/to/app
svr check --experimental-v2 --locked --format json path/to/app
svr run --experimental-v2 --locked path/to/app
svr test --experimental-v2 --locked --format json path/to/app
svr build --experimental-v2 --locked --emit ir path/to/app
```

This selects the [supported single-entry package scope](experimental-v2-packages.md),
with no dependency aliases, registered modules or imports. Guarded Linux/procfs
capture/publication requirements remain unchanged; unsupported hosts refuse.
Default commands retain their existing behavior. Standalone `.svr` inputs cannot
use `--locked`, and duplicate options or unsupported lock options return usage
status 2. Options after `run`'s `--` remain program arguments and grant nothing.

## Generation and verification

Generation hashes the exact captured manifest and entry bytes and uses the same
strict version-one `sovra-lock` schema as existing package locks. Package metadata,
normalized relative source path, byte counts and SHA-256 hashes are compared;
canonical absolute checkout paths and runtime request identities are excluded.
An ordinary root package produces identical lock bytes through either compiler
pipeline, and unchanged portable package contents can relocate with the same lock.
Whitespace/key order in a valid input lock may vary. Duplicate, unknown or missing
data, malformed JSON, wrong versions/types/hashes/paths and oversized documents
refuse under the existing bounded parser with E4165.

`lock --experimental-v2` publishes through the existing guarded staging/atomic
replacement implementation. Unsafe, read-only, symlink/hard-link targets and
existing malformed/unsupported formats refuse rather than being overwritten.
Generation is an explicit update operation. `lock --experimental-v2 --check`
verifies only and never rewrites a stale document. Status output errors return 1
without a panic, but cannot undo an already completed lock publication.

Every `--locked` compiler command captures the supported root inputs once, reads
the existing lock through the guarded reader, and verifies those same captured
bytes **before** source checking, lowering, test discovery or VM effects. Missing,
unsafe, stale, malformed or over-budget locks return 1 without fallback, source
execution or compiler lock modification. JSON check/test failures retain their
existing schema/kinds, including zero tests on lock refusal. Compiler source and
manifest paths are not reread to build the expected document after capture.
Package admission still precedes lock verification, so unsupported package shapes
may report E3124 or earlier graph/path errors before a lock is considered.

## Meaning and boundaries

Lock integrity does not establish valid source, a trusted publisher, a pinned
compiler/toolchain or an atomic filesystem snapshot. Generation/check-only lock
verification does not parse or execute source. A verified lock can describe an
invalid program: the compiler must still reject it during its ordinary checking.
Runtime host files/streams are not frozen by a source lock. Without `--locked`,
v2 commands continue to ignore existing locks as before.

VM limits, program arguments/status, working directory and host grants are
unchanged. Successful lock verification does not grant stdin/filesystem access.
Completed runtime file effects are not rolled back; an explicitly granted program
can still modify accessible files after verification. This is not process isolation.

Rust APIs `generate_root_source_lock` and `verify_root_source_lock` work directly
on immutable `CapturedRootSource`. `verify_existing_root_source_lock` reads only
the lock for an already captured input; `write_root_source_lock` and
`check_root_source_lock` provide guarded capture/publication or verification.
Existing source-lock APIs retain their formats and behavior.

G13 remains partial: dependency/module linking and its v2 lock qualification,
IDE contexts, JS/native backends and other package-host acceptance remain open.

V2 also supports [source-backed native executable builds](experimental-v2-native.md)
through `--emit exe --output PATH`, with optional package `--locked`. This does not
close dependency/module linking or executable IR codec qualification.
