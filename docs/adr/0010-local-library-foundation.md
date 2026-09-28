# ADR 0010: First executable local library dependencies

Status: Accepted (2026-09-28). The user approved continuation after the explicit
request to approve A–C. Implementation is incremental; acceptance alone does not
make the examples executable. ADR 0009 did not select package syntax.

## Boundary

Language modules own names and visibility; standard-library modules ship with
the toolchain; packages own versions and modules; dependencies are declared graph
edges. Preserve existing `mod`, `export fn`, qualified calls, private helpers,
exact builtin collision rules and the bare `print` alias. No `module`/`pub` rewrite.

## A. Manifest and library root

Recommend retaining `[project]` with name/version/entry, and adding one explicit
section per local dependency:

```toml
[project]
name = "consumer"
version = "0.1.0"
entry = "main.svr"

[dependencies.utilities]
path = "../utilities"
```

The dependency directory contains its own `sovra.toml`. Resolve paths relative
to the declaring manifest, not the shell working directory. A library's entry
file initially contains ordinary inline `mod` declarations; exported functions
form the callable API. A library need not have `main`. Its top-level `main`, if
present, is not executed or exposed by loading it. No installation scripts run.
All loaded declarations receive semantic checks, including private bodies.

This first slice intentionally does not invent implicit filename modules or
general filesystem module discovery for execution. Multi-file packages follow
separately. Project application scanning remains a distinct partial facility.

Alternative: inline-table dependencies or replacing `[project]` with `[package]`.
The latter needlessly breaks existing manifests; neither is selected here.

## B. Explicit executable imports

Recommend `use utilities::arithmetic;` to import the dependency's named inline
module, with calls spelled `utilities::arithmetic::square(7)`. Dependency imports
remain fully qualified, avoiding filename/name-order winners. Existing application
`use app.services` retains its project-file meaning; do not silently reinterpret
it as executable package loading. The new form must be wired through parser,
file-aware semantic resolution, IR and both execution engines before release.

Only exported members are callable by consumers. Module-local private calls keep
their current qualified spelling and resolve in the owning package. Library
top-level helpers resolve within that package, never to consumer functions.
Importing a dependency does not expose its transitive dependencies. Duplicate
imports may be deduplicated; duplicate dependency aliases and conflicting names
must fail. Reserve `std` as a dependency alias without changing the accepted
noncolliding inline `std` member rule.

Alternative: bring dependency modules into an unqualified global namespace.
This introduces collision/alias rules and is not recommended.

## C. Graph, containment and acceptance

Recommend deterministic traversal ordered by dependency alias. Use canonical
manifest roots for local node identity; separate declared aliases from identities.
Deduplicate a shared canonical node. Reject package cycles with the dependency
chain (existing project source-import cycles remain a separate policy). Different
roots with identical display names remain distinct nodes. Two aliases can refer
to the same node, but neither may expose undeclared transitive dependencies.

An explicitly declared sibling directory is allowed. Entry/module paths must stay
inside their selected package root after canonicalization, including symlinks.
Reject missing roots/manifests/entries, unknown dependency fields, duplicate
aliases, invalid names and malformed manifest values. Bound graph traversal and
avoid host recursion. Do not claim local mutable directories are reproducibly
locked artifacts; lockfiles/content integrity are the next package slice.

No fetch, Git, registry, caching, checksum, publish or new package command is
implemented by this decision. Extend actual `check`/`run`/`build` project behavior
only when they share the same resolver and executable pipeline. A project-source
scan must not substitute for semantic validation of a linked consumer.

Acceptance: a consumer in one root uses a library in another and passes source
checking, interpreter execution and JS execution, including a private helper.
Tests must reject private access, missing/malformed/duplicate dependencies,
undeclared transitive access, cycles and path escapes; cover relocation,
deterministic resolution, diamond graphs and equal display names. Keep original
single-file behavior and all builtin collision regressions.

## Approval and implementation status

The user approved A, B and C. The graph-only Rust API is implemented: canonical
local nodes, dependency aliases, deterministic traversal, cycle detection,
manifest validation and entry containment. Source imports, semantic linking and
project run/build integration remain unfinished. The ordinary CLI checker still
rejects dependency sections to avoid claiming executable validation.

The original approval boundary was required because the user request
explicitly says to stop before an undecided import/export compatibility change.
AGENTS.md also says: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." This is the blocker to implementing
local consumption, not a claim that documentation completes the package milestone.
