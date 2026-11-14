# Scoped callable value identities

Status: **Partial E1 lowering foundation.**
`CheckedEnumTypes::check_scoped_callable_function_source` extends the
[typed callable checker](typed-callable-v2.md) with value declarations and
scope-resolved reads/writes, including direct match scrutinees. Earlier APIs,
their syntax boundaries and quota behavior remain unchanged.

`CheckedScopedCallableFunction` retains typed/call/source facts and immutable
`ScopedCallableEvidence`. Declaration and use IDs belong to the same checked unit
as its expression facts. `declaration(id)` and `usage(id)` refuse foreign unit IDs,
even for identical input; clones preserve identities. Source spans locate facts,
while IDs distinguish declarations and uses. Ordinals follow semantic introduction
or visitation order rather than necessarily source order.

Declarations distinguish immutable source parameters, ordinary locals with checked
mutability, and immutable payload snapshots. Payload roles identify the direct
match and declaration variant ordinal, so arm reordering does not change variant
meaning. Local declaration spans cover the original core statement; parameter and
payload spans use their original name locations. Declaration types are canonical
core types, retaining existing empty-array holes.

The recorder follows the core checker's declaration/use and branch/loop scope
callbacks. An initializer is checked before its declaration replaces a same-named
local. Branch and loop frames restore outer names on exit; each match arm has its
own frame, so payload/local names cannot leak into another arm or continuation.
The containing function's parameters and preceding locals seed those frames.
Function-call names remain in the separate callable namespace.

Reads link to the selected declaration and typed expression. Direct writes and
indexed writes have their own use-site IDs; indexed locations include the complete
array target, and evaluated receiver/index expressions retain ordinary type facts.
A `MatchScrutinee` use records its selected binding and match ordinal once. Writes
and scrutinees are not fabricated value expressions. No later name-inference pass
is used to construct this evidence.

`BindingEvidenceLimits` independently caps declarations, uses, retained declaration
name bytes, copied declaration type nodes/nominal text and active scope frames.
The function root counts as one frame; a match arm and each nested core branch/loop
add frames. Declaration names use shared allocations between retained facts and
scope maps; each declaration is charged once, including shadowed names. Every
copied type leaf/array wrapper and nominal byte is charged. Temporary
canonicalization and core checker scratch remain outside retained accounting;
these are logical quotas, not physical memory or complete-work accounting.
Zero allowances are real. Exhaustion refuses without partial evidence or fallback,
while core semantic/required-return diagnostics keep precedence.

The result supplies binding facts for subsequent versioned IR work; it does not
implement instruction emission or execution. A separate
[structural mapper](callable-mapping-v2.md) now validates source/fact attachments.
IR operation emission/validation, nested matches,
other user functions/complete programs, runtime/backend/native, packages and
tooling remain open. Existing v1 stages and CLI behavior remain unchanged. See the
[approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
