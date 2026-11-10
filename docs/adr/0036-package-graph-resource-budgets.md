# ADR 0036: Package manifest and dependency-edge budgets

Status: **Accepted and implemented.** The user approved the 1 MiB per-manifest,
8 MiB total-manifest and 8,192 alias-edge defaults on 2026-11-10. The previously
approved source/frontend and 64 MiB aggregate source limits remain.

## Problem and scope

The existing 1,024-package ceiling does not bound manifest size or alias count.
Multiple dependency aliases can produce substantial path-resolution and pending
traversal work while naming few canonical packages. Entry-source byte limits
apply after graph loading and do not protect this earlier phase.

## Implemented mechanism

`compiler::resources::PackageGraphLimits` has per-manifest bytes, total manifest
bytes and total dependency-edge allowances. Rust callers use
`packages::resolve_with_limits(root, limits)` or
`packages::compile_with_graph_limits(root, limits)`. The latter retains the
approved frontend and aggregate entry-source defaults.

Manifest reads consume at most the smaller of the per-file and remaining total
allowance plus one detection byte, without trusting file metadata. Overflow
returns E4112 with the owning manifest. IO failures retain E4000. Actual UTF-8
bytes, including comments/whitespace, consume the allowance before parsing.
Each canonical package manifest is counted once, including the root; aliases
and shared dependencies do not charge the same manifest again.

Each dependency alias declaration consumes an edge even when another alias
targets the same package. The edge limit is checked before resolving the next
target path or scheduling traversal. Overflow reports E4112 at that declaration's
manifest-header range. Existing cycle, path containment, import visibility and
1,024-package checks remain. Failure returns no partial successful graph or IR.

Ordinary graph entry points use PackageGraphLimits::DEFAULT. Explicit-limit APIs
remain available to trusted Rust hosts. These are graph-input budgets, not total
CPU/memory quotas. Manifest error reports now have a separate 256-entry cap;
guarded messages also have a 4 KiB storage ceiling. Transient formatting,
valid-input semantic analysis, imported-interface duplication and
IR work still need further bounds.

## Approved defaults

1. **1 MiB per manifest**, including the initial project/dependency classification
   read, to avoid an unbounded read before the graph budget can take effect.
2. **8 MiB total manifest bytes** during one canonical package-graph resolution,
   counting each package once, including the root.
3. **8,192 dependency alias edges** across those unique package manifests.

The initial classifier may separately read the root manifest within its per-file
cap; the total manifest allowance measures canonical manifests during resolution,
not every OS read across a CLI invocation. The per-file manifest read cap
also applies to ordinary scanner-only project checks, whose source-file scan
budgets otherwise remain separate work. Existing 1,024-package, 16 MiB/source
and 64 MiB aggregate source ceilings remain unchanged.

Advisory measurement of examples/local-packages/app found two manifests of 121
and 69 bytes (190 total) and one edge. This small example is not representative
production-size performance or evidence that the approved defaults guarantee
safe process memory/CPU. Larger and other-host qualification remains necessary.

## Decision boundary

[AGENTS.md](../../AGENTS.md) requires: “Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions.” Applying these
new ceilings by default rejects some previously accepted manifests/graphs. The
policy was documented and explicitly approved before adoption; no further gate
applies to implementing this approved scope.

## Verification

Tests cover exact/overflow per-file and total bytes, UTF-8 accounting, zero
allowances, alias/diamond deduplication, distinct alias-edge accounting, cycle
rejection, and stopping before resolution of an over-budget missing target.
Command tests cover actual default boundaries and one-byte/one-edge overflow,
including initial classification, canonical manifest deduplication, missing
excess targets and package check/run/build/test rejection without native output.
Existing approved source limits retain regression coverage. Larger and other-host
qualification and semantic/IR quotas remain open.
