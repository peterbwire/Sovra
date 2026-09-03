# Roadmap

## M0 — foundation (current)

* Cargo package and canonical `svr` binary
* CLI version/help behavior
* Compiler stage boundaries
* Documentation, example, tests, and CI

## M1 — tokens and lexer

Define source locations, token kinds, comments, literals, identifiers, and
lexical diagnostics. This is the first milestone allowed to add lexer
functionality.

## M2 — parser and AST

Specify the grammar and produce a validated AST with useful diagnostics.

## M3 — semantics and IR

Add name resolution, types, validation, and a stable intermediate
representation.

## M4 — execution and backends

Implement the interpreter and at least one compiled backend, then enable the
reserved CLI commands incrementally.

