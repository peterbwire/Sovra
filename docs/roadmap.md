# Roadmap

Production readiness is the release target; the developer-preview publication
plan is superseded. Follow [the production audit and gates](PRODUCTION_READINESS.md)
without renumbering these milestones. Completed foundation slices do not imply
that the full language or application platform is production-ready.

On 2026-10-03 the user explicitly required the **full application platform and
third-party library publishing before version 1.0**. The compiler-only release
scope is superseded. [Version-one delivery gates](V1_DELIVERY_GATES.md) track
the required evidence; setting Cargo's version to 1.0.0 does not satisfy them.

The [library ecosystem assessment](LIBRARY_ECOSYSTEM_ASSESSMENT.md) records the
actual executable module/stdlib boundary and package prerequisites. Local package
graph resolution and entry-module consumption are implemented under accepted
ADR 0010, including source validation, private helpers and both execution engines.
Locks, general multi-file loading and publication remain planned. This package
track retains the roadmap's established milestone numbers and does not complete M12.

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

### Third-party package track (planned)

Independent developers must be able to publish reusable Sovra libraries.
Project-local imports are a first stage, not a permanent ecosystem restriction.
Preserve package-qualified module identities and public/private exports while
finishing cross-file resolution. Then deliver local dependencies and lockfiles,
Git/registry resolution and caching, followed by publishing and ownership flows.
Integrate package-aware testing and inspection with M13 and M15 without
renumbering existing milestones. Exact manifest/import syntax and registry
policies remain open design decisions. See the
[package/library requirements](design/PACKAGES_AND_LIBRARIES.md).

The next roadmap turns the foundation into the application language shown in
[`examples/fielddesk`](../examples/fielddesk):

## M12 — Project Checker (started)

Make `svr check <project>` validate project manifests, modules, app routes,
models, auth policies, standard-library calls, service contracts, and page
bindings before execution.

Current implementation validates the project manifest, required project
metadata, runtime target, configured entry path, source-file discovery, external
service binding consistency between the manifest, source declarations, and app
entry, app route and page path/target bindings, auth target wiring, app data
model references, scheduled task targets, and auth policy model references.
Full service contracts and richer page/model checks remain next.

The partial multiline service scanner now includes duplicate-operation E4025
checks and excludes service signatures from route/task callable targets.
Code and regression cases have been executed successfully on Windows.
Structured headers, parameter records and return annotations now feed retained
Rust service-operation metadata. This does not implement service calls, signature
type resolution or full application parsing. Experimental opt-in
`check --service-calls` now checks operation existence and argument counts in
supported function/task and service-operation bodies, with explicit per-file
coverage. ADR 0008 defines explicit service calls and lexical shadowing. Unsupported
syntax produces E4096;
complete coverage does not mean full application type checking.

Before expanding this surface, the handoff assessment recommends hardening the
existing executable subset. Module bodies, numeric widening/bounds, Int
overflow, token ranges and straight-line return completeness are now covered.
ADR 0002 is approved and implemented: every function parameter requires an
explicit annotation, while local inference and default Unit returns remain.
Named-type resolution, full runtime/backend equivalence and rich diagnostics
remain incomplete. Project checking still does not validate function bodies
or parameter annotations through the executable compiler.

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
