# ADR 0035: Frontend resource budgets

Status: **Accepted and implemented for executable frontend and package-source defaults.**
The user approved 16 MiB/source, 1,000,000 tokens and 256 frontend diagnostics
on 2026-11-10, then separately approved 64 MiB aggregate package entry sources.
Executable-source reads are bounded. Manifest/edge and semantic/IR quotas remain
separate follow-up work.

## Problem

The existing structural-depth checks constrain recursive shapes, not total input
size or width. A shallow file can allocate arbitrarily many tokens or diagnostics.
Before this ADR, package graphs were capped at 1,024 packages without a total
source-byte budget, and CLI source reads allocated complete files before lexing.

## Implemented mechanism

`compiler::resources::FrontendLimits` lets Rust callers select source-byte,
non-EOF token and lexer/parser diagnostic budgets. `Lexer::tokenize_with_limits`
and `Parser::parse_source_with_limits` enforce them. Normal lexer/parser entry
points and executable CLI compilation now use FrontendLimits::DEFAULT. Explicit
with-limits APIs allow trusted Rust callers to choose their own budgets.
Lexical exhaustion returns E1003; parser diagnostic
exhaustion returns E2008. Exhaustion never returns a partial successful token
stream or program. Diagnostic capacity includes the terminal limit notice;
capacity zero is normalized to one so failure cannot appear successful.

Source size is checked before token allocation. The token check precedes scanning
the next token. EOF, whitespace and comments do not consume the token budget.
These APIs receive already allocated source strings. CLI source check/run/build,
language tests and executable package entry loading additionally use a shared
bounded reader: at most 16 MiB plus one detection byte, without trusting file
metadata. Package reads also honor the remaining aggregate source budget. These
limits do not constrain semantic diagnostics, total AST/IR size or CPU time.

## Approved default policy

1. Maximum **16 MiB UTF-8 per source file** and **1,000,000 non-EOF tokens per file**.
2. Maximum **256 lexer/parser diagnostics per stage/file**, including an exhaustion
   notice. Executable semantic error reports now have their own 256-entry cap;
   valid-input semantic work quotas remain a separate follow-up.
3. Bounded file reads read at most the byte ceiling plus one detection byte,
   regardless of metadata or concurrent file growth.
4. Maximum **64 MiB aggregate entry-source bytes per canonical package graph**,
   including the root. Shared dependencies and aliases count their package once.

Package imports consume tokens before import statements are removed; every
root/dependency entry receives the same per-file byte budget. Package diagnostics
retain the offending file identity. The formatter retains its existing stricter
1 MiB bounded input guard and uses the default token/diagnostic caps. Ordinary
scanner-only application project checks do not acquire a blanket read quota.

These are policy ceilings, not total process-memory or CPU guarantees. Oversized
files that previously compiled may now be rejected. Exact byte/token boundary
and overflow tests pass; broader performance/platform qualification remains open.

## Aggregate source implementation

The caller-selected mechanism is now implemented as
`project::packages::compile_with_source_limit(root, max_source_bytes)`. It counts
UTF-8 entry-source bytes once per canonical package, including the root, and reads
each source within the smaller of the remaining aggregate allowance and the
approved 16 MiB per-file limit, plus one detection byte. Aliases and diamonds do
not count the same package twice. Every source is loaded/parsed before body
checking/lowering, so aggregate exhaustion returns E1003 at the offending file
without producing linked IR. Empty sources consume zero bytes. Ordinary package
compilation and package CLI/language-test paths use the approved 64 MiB aggregate
default. Trusted Rust hosts can still select explicit source budgets. This does
not bound graph resolution, manifests, edges or cloned IR.

Advisory measurement of the existing examples found one declared dependency
graph (examples/local-packages/app): two canonical packages, one dependency edge
and 765 entry-source bytes. This small sample is not production-size evidence
and cannot justify a universal safe CPU/memory budget. Tests use small caller
limits to verify exact/overflow behavior, UTF-8 accounting, deduplication, zero
allowance and preservation of the per-file limit. A command-level fixture tests
exactly 64 MiB with aliases and an empty dependency, then one-byte overflow through
check/run/build/JS/native build/test, with no native publication or source mutation.

## Remaining policy work

The existing 1,024-package ceiling is retained. Manifest/edge defaults are now
accepted under ADR 0036. Manifest/semantic error counts and guarded message
storage are capped separately. Transient formatting and semantic/IR work budgets
still need further design and measurement. These input ceilings are not a complete
compilation memory or CPU quota.

## Decision boundary

[AGENTS.md](../../AGENTS.md) requires: “Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions.” Making these
ceilings the default rejects previously accepted large files. The documented
per-file and aggregate policies were approved explicitly before adoption; no
further approval gate applies to implementing those approved scopes.

## Verification

Byte boundary/boundary-plus-one, multibyte input, token/EOF/comment accounting,
zero capacity, lexical error floods, parser recovery and generous-budget
compatibility are covered by ordinary tests. Deterministic source mutations also
compare generous-budget parsing with existing parsing. Coverage-guided source
fuzzing exercises bounded parsing without executing inputs.
