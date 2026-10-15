# Changelog

## Unreleased

- Add a private reusable Windows Sovra UI renderer and desktop packaging over
  existing view/event functions; native Task Manager and unrelated component
  controls execute real Sovra behavior. Public UI compatibility remains planned.

- Implement approved ADR 0028 language tests, human/JSON reports, template and reusable-library tests, and installation/CI verification.

- Implement accepted ADR 0027 host-native `svr build --emit exe --output PATH`,
  installed runtime payload validation, offline linking, staged cleanup and
  atomic no-overwrite publication. Preserve IR/JS stdout behavior. Rust/Cargo
  and a compatible linker are required on the developer machine; clean-machine
  release qualification remains pending.
- Add `svr new <path>` with an embedded console template and refusal to
  overwrite existing destinations under approved ADR 0026.
- Add Windows toolchain packaging, integrity checks, user installation/PATH
  setup and a package-contained create/check/run self-test. Clean-machine
  qualification remains outstanding.
- Add executable local dependency packages, exported nominal records and
  compound value isolation, plus process input/status and bounded UTF-8 file
  APIs with interpreter/JavaScript regression coverage.
- Exercise nine dogfood application slices, persistent Task Manager and Notes,
  File Manager/browser UI and HTTP host adapters through real Sovra execution.
- Add the approved internal checked-IR standalone executable proof, bounded
  image validation, build failure isolation and 18-case Task Manager parity.
  Windows proof packaging includes checksums, imports and a self-test. Public
  native build syntax and clean-machine deployment remain outstanding.
- Add production-readiness gates, cross-platform CI, a Rust 1.74 check,
  source-package boundaries and manual verification artifacts. The earlier
  developer-preview publication plan is superseded.
- Add experimental opt-in service-call checking with coverage errors and
  machine-readable member-call references. Application execution remains planned.
- Add versioned JSON reports with `svr check --format json` for source/project
  validation and input I/O errors, preserving default human output.
- Require explicit type annotations on every function parameter, including
  unused and module functions, under approved ADR 0002. Untyped declarations
  now report semantic error `E3014` at the parameter name. This is a source
  compatibility change; local `let` inference and default Unit returns remain.
- Add an executable function example and a course lesson on typed signatures,
  inferred locals and migration from untyped parameters.

## 1.0.0 target metadata - not release evidence

- Set the intended v1.0.0 version metadata for the Sovra toolchain.
- Align the canonical `svr` binary version metadata and project
  manifests for the source compiler, IR, interpreter, and JS backend.
- On 2026-10-09 the user revised v1 scope to a production-quality Task Manager
  CLI and qualified toolchain, with later versions adding platform capacity.
  This supersedes the earlier full-platform/registry requirement. Metadata
  does not establish publication or readiness; see the version-one gates.

## 0.1.0 - 2026-09-03

- Added interpreter support for user-defined function calls, parameters, and
  return values.
- Added semantic diagnostics for function call arity and parameter types.
- Added runtime arithmetic and comparison operations for numeric, string, and
  boolean values, including explicit division-by-zero errors.
- Added validation for duplicate declarations and the `main` entry signature.
- Added a bounded interpreter call depth with an explicit recursion error.
- Promoted `svr run` to the M6 user-facing execution milestone with strict
  `.svr` path validation and end-to-end CLI coverage.
- Created the M0 repository foundation.
- Added the canonical `svr` CLI with version, help, and reserved command
  handling.
- Added compiler-stage extension points and the first `.svr` example.
- Added the M1 lexer with tokens, source spans, comments, literals, and
  structured diagnostics.
- Added the M2 recursive-descent parser and AST.
- Added M3 name resolution, basic type checking, and a minimal IR.
