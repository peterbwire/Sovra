# Boolean symbolic body expressions

`check_generic_copy_bodies` accepts `&&` and `||` with concrete `Bool` operands,
including transparent aliases expanded to `Bool`. The result is `Bool`.
`GenericCopyExpressionKind::Logical` records `GenericLogicalOperator::And` or
`Or`, both source-owned operand IDs, and the original expression span.
There is no truthiness conversion, and a `std::Key` bound does not prove `Bool`.

Both sides are checked, including `false && missing` and `true || missing`.
Runtime short-circuiting does not excuse invalid names or types. The checker
uses the existing parser precedence and iterative expression worklist; every
logical node consumes cumulative expression and work admission.

These are symbolic typing facts, not executable control flow. Future lowering
must evaluate the right operand only when required (`&&`: left true; `||`: left
false). Postorder fact storage must not be interpreted as eager evaluation.
No generic execution, specialization, call support or control statements are
introduced by this stage. Unary operators remain outside the profile.

`tests/generic_logical_bodies_v2.rs` covers operators and binding identity,
non-Bool/opaque refusal on either side, invalid skipped operands, mixed numeric
comparisons and logical precedence, Unicode source spans, transparent aliases,
and exact/one-under cumulative quotas.
See [symbolic bodies](generic-copy-bodies-v2.md) and
[numeric expressions](generic-numeric-bodies-v2.md).
