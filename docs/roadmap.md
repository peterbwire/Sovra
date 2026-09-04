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

## M6 — `svr run` (complete)

Expose the complete source-to-execution pipeline through the `svr run
<source.svr>` command with user-facing diagnostics and end-to-end coverage.

## M7 — Core Type System (complete)

Typed locals, parameter declarations, numeric widening, and type-directed diagnostics.

## M8 — Modules (complete)

Source modules, exports, and module-aware name resolution for namespaced calls.

## M9 — Standard Library (complete)

Define stable standard-library boundaries for I/O, conversions, and utility
functions under a `std` namespace.

## M10 — IR (complete)

Make the intermediate representation explicit, extensible, and independently
inspectable.

## M11 — Compiler Backend (complete)

Add a native or portable compiled backend over the stable IR.

## Product experience track

The next roadmap turns the foundation into the application language shown in
[`examples/fielddesk`](../examples/fielddesk):

## M12 — Project Checker (started)

Make `svr check <project>` validate project manifests, modules, app routes,
models, auth policies, standard-library calls, service contracts, and page
bindings before execution.

Current implementation validates the project manifest, required project
metadata, runtime target, configured entry path, source-file discovery, and
external service binding consistency between the manifest, source declarations,
and app entry. Application-level routes, models, auth policies, full service
contracts, and page bindings remain next.

## M13 — Sovra Tests

Make `svr test <project>` run Sovra-native unit, integration, service-contract,
policy, and page tests.

## M14 — Application Runtime

Make `svr run <project>` start an integrated application with APIs, pages,
auth, background tasks, concurrency, and external services.

## M15 — Agent Inspection

Expose structured project metadata, diagnostics, tests, routes, models, and
dependency boundaries so AI agents can inspect and modify Sovra projects
without reverse-engineering the codebase.
