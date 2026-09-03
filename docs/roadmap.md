# Roadmap

## M0 — Repository Foundation (complete)

Cargo package, canonical `svr` binary, repository documentation, and CI.

## M1 — Token & Lexer System (complete)

Source locations, token kinds, comments, literals, identifiers, and lexical
diagnostics.

## M2 — Parser (complete)

Recursive-descent parsing for the Sovra grammar with structured diagnostics.

## M3 — AST (complete)

Validated function, statement, and expression abstract syntax trees.

## M4 — Semantic Analysis (complete)

Name resolution, type checking, declaration validation, and stable diagnostics.

## M5 — Minimal Interpreter (complete)

IR execution with values, operators, function calls, output capture, and runtime
errors.

## M6 — `svr run` (current)

Expose the complete source-to-execution pipeline through the `svr run
<source.svr>` command with user-facing diagnostics and end-to-end coverage.

## M7 — Core Type System (complete)

Typed locals, parameter declarations, numeric widening, and type-directed diagnostics.

## M8 — Modules (complete)

Source modules, exports, and module-aware name resolution for namespaced calls.

## M9 — Standard Library (current)

Define stable standard-library boundaries for I/O, conversions, and utility
functions under a `std` namespace.

## M10 — IR

Make the intermediate representation explicit, extensible, and independently
inspectable.

## M11 — Compiler Backend

Add a native or portable compiled backend over the stable IR.
