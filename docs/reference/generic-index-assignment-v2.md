# Symbolic indexed array assignment

`check_generic_copy_bodies` accepts `local[index] = value` only when `local`
is a direct mutable array local. Parameters and ordinary immutable locals cannot
be written. Nested targets such as `rows[0][0]`, field targets, non-arrays and
unknown names are rejected. Indexes must resolve to concrete `Int`.

Replacement types must match the exact expanded element type, including opaque
parameter identities and nested array values. Existing concrete Int-to-Float
widening is recorded explicitly. The binding's array type never changes.

`GenericCopyStatement::IndexAssign` retains the mutable binding ID, checked index
and replacement IDs, scalar conversion, original indexed-target range and full
statement range. No fake read of the element or extra local binding is inserted.
Element projection shares the validated immutable type arena and provenance.
Statements, expressions, comparisons and work consume existing cumulative quotas.
Every write is checked, including unreachable writes and writes in false branches.
Branch/loop shadowing still determines which binding is mutable.

This is a symbolic value-replacement plan, not executable mutation or a reference
semantic. Existing snapshot ownership must be preserved by future lowering; other
array values must not acquire shared mutable storage. Future execution must retain
index/replacement evaluation rules, bounds checks, allocation admission, runtime
fuel and failure behavior. A huge Int index can type-check; this does not prove
that the write will succeed. Inferred array construction, generic calls, specialization
and executable integration remain open.

`tests/generic_index_assignment_v2.rs` covers source/binding/operand identity,
opaque and nested array replacements, explicit widening, fixed-type errors,
immutable/nested/non-array/unknown targets, bad indexes, branch shadowing,
unreachable errors, bounds non-evaluation and exact/one-under resource caps.
See [read-only indexing](generic-index-bodies-v2.md).
