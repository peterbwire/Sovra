# Direct generic record field argument projection

`check_generic_copy_bodies` now accepts fields whose expanded type is exactly
one record parameter. For `struct Box<T>{value:T}`, reading `Box<U>.value`
produces the original caller-owned `U`; reading `Box<[U]>.value` produces `[U]`.
A record with multiple parameters selects the exact declared parameter ordinal.
Chained parameter fields and subsequent array reads compose with existing rules.

The checker verifies that the field parameter belongs to its checked record,
then privately projects the corresponding argument from the receiver's validated
nominal root. It verifies declaration identity and argument availability before
publishing a result. `GenericCopyExpressionKind::Field::parameter_argument`
records `Some(ordinal)` for this case and `None` for independent field types.

Projection shares the receiver's immutable expanded arena, original annotation,
parameter owner and alias provenance. No nodes, substitutions, replacement
sources or synthetic parameter identities are generated. Subsequent equivalence
compares the selected root with the original caller's parameter identity intact.
Lookup/projection consumes cumulative work, field facts consume expression
admission, and subsequent comparisons consume existing pair/edge/work budgets.

This handles direct parameter fields only. [Composite field types](generic-field-substitution-v2.md), such as `[T]`
or `Other<T>`, now use a separate bounded substitution path. Opaque projected values
gain no numeric or other capabilities. Field writes remain rejected under the
existing language rules. Record construction, generic calls, specialization and
executable integration remain open; these are not runtime layout/reference plans.

`tests/generic_field_arguments_v2.rs` covers caller ownership, shared arenas,
concrete/array/nominal arguments, nonfirst parameter positions, nested field reads,
foreign-parameter and numeric-assumption refusal, unsupported composites/writes,
original Unicode source/provenance and exact/one-under quotas.
See [independent field reads](generic-field-bodies-v2.md).
