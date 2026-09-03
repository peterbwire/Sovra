# Changelog

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
