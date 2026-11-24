# Mutable symbolic locals and direct assignment

`check_generic_copy_bodies` accepts `let mut`, including explicit local type
annotations, and direct reassignment to mutable locals. Parameters and ordinary
`let` locals remain immutable. `GenericCopyBindingKind::MutableLocal` records
mutability while each binding retains a fixed symbolic type.

`GenericCopyStatement::Assign` retains the existing binding ID, original target
range, checked replacement expression ID, full statement range and explicit
replacement conversion. Exact symbolic types qualify, including opaque owner-
scoped parameters, arrays and nominal values. Existing concrete `IntToFloat`
widening is allowed; narrowing and opaque numeric assumptions are refused.
Assignment does not infer a new binding type. Unknown names and field targets
and writes to immutable bindings are rejected.
[Indexed array assignment](generic-index-assignment-v2.md) is separately supported
for direct mutable array locals.

Initializers resolve before the new declaration. Shadowing selects the current
binding for assignment, even when an earlier binding of the same name was mutable.
Every statement is checked, including unreachable assignments. Existing cumulative
statement, expression, binding, source, annotation, comparison and work budgets
remain active. No extra binding or read expression is fabricated for an assignment
target; its identity and source range are retained directly.

These are immutable symbolic proof records, not mutable runtime storage or
executable IR. Direct whole-value replacement introduces no reference semantics,
field mutation, runtime copy optimization or change to snapshot ownership.
Generic calls, specialization and executable integration remain open.

`tests/generic_mutable_bodies_v2.rs` covers opaque and typed-array replacement,
explicit widening, original Unicode source ranges, fixed-type errors, immutable
and unknown targets, aggregate target refusal, shadowing, unreachable errors,
and exact/one-under cumulative quotas.
See [symbolic bodies](generic-copy-bodies-v2.md) and
[local annotations](generic-local-annotations-v2.md).
