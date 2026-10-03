# Changelog

## Unreleased

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
- The user confirmed on 2026-10-03 that the full application platform and library
  publishing are required before 1.0.0. Metadata does not establish publication
  or readiness; the earlier compiler-only release scope is superseded.

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
