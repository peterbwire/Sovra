# ADR 0004: Unresolved type annotations

Status: Accepted and implemented. User directed continuation of recommended
Option A on 2026-09-21.

## Verified problem

Before this decision, `fn identity(value: Strng) -> Strng { return value } fn main() {}` passed source
checking with success and no JSON diagnostics. Unknown names become Type::Named
without declaration resolution. The executable parser has no type/model/struct/
enum/alias declarations yet, so none of those names can be resolved today.

## Decision: Option A, reject unresolved annotations now

Accept only Unit, Bool, Int, Float and String in executable source annotations
until user-defined types are implemented. Report E3017 at every unknown
parameter, return or local annotation, including unused/private/exported bodies.
Validate declarations once rather than repeating errors at each call site; use
Unknown only for recovery after reporting the error. Do not silently alias Text,
Money or the builtin registry's internal Any marker to an implemented type.

Preserve local inference, required parameter annotations, default Unit returns,
numeric widening, parser syntax and compiler stage signatures. This is a temporary
resolution boundary, not a permanent primitive-only language. Future declaration
resolution can accept declared user types while retaining E3017 for unknown names.

This rejects previously accepted named annotations. Correct typos to the intended
implemented type. Project-directory checking remains a separate manifest/wiring
scan; Fielddesk's proposed application types remain outside executable validation.

Tests cover positive/negative annotations in all declaration contexts, source
check/run/IR/JS rejection, JSON locations and existing execution behavior.
E3017 diagnostics identify the exact type token for parsed source. AST values
constructed without annotation spans fall back to parameter-name, function
(return annotation), or let-statement spans.

## Alternative: Keep placeholders until type declarations exist

Keep accepting arbitrary annotation names as opaque Named types, with spelling
and resolution explicitly unchecked. This avoids an immediate compatibility
change but preserves misleading successful checks. A later declaration milestone
must choose resolution and migration rules.

## Approval boundary

AGENTS.md says to document and pause before fundamental type-semantics or
compatibility decisions. ADR 0002 approved explicit parameters; ADR 0003 approved
builtin collisions. The user's subsequent continuation selects Option A for this
separate policy.
