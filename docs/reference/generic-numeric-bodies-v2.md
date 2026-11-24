# Concrete numeric symbolic body expressions

`check_generic_copy_bodies` accepts `+`, `-`, `*`, `/`, `<`, `<=`, `>` and
`>=` when both checked operands resolve to concrete `Int` or `Float`.
`Int`/`Int` arithmetic produces `Int`; mixed or `Float` arithmetic produces
`Float`. Ordering produces `Bool`. Transparent aliases use their expanded type.

`GenericCopyExpressionKind::Numeric` records the operator, source-owned operand
IDs, original full-source span, and explicit operand conversions. Mixed operands
mark only the `Int` edge as `IntToFloat`. Expected return/local types do not
change operand arithmetic: an `Int + Int` returned as `Float` retains integer
arithmetic followed by a return conversion. Nested expressions use the existing
parser precedence and bounded iterative checking; numeric nodes consume the
existing cumulative expression and work budgets.

Opaque parameters, including `T:std::Key`, do not prove numeric capabilities.
Booleans, arrays and nominal values are refused as numeric operands.
Strings use their [separate concrete profile](generic-string-bodies-v2.md).
The existing equality profile remains separate. [Boolean logical operators](generic-logical-bodies-v2.md) are also supported.
Qualified/implicit calls, unary operators, field mutation and generic execution remain outside this profile.

This is type checking, not evaluation or an executable proof. `1/0` and an
integer addition that would overflow can type-check; future lowering/execution
must preserve Sovra runtime error and finite-number rules. No arithmetic is
constant-folded and no runtime checks are removed.

Verification: `tests/generic_numeric_bodies_v2.rs` covers every operator and
numeric operand pairing, widening placement, original Unicode source spans,
nested precedence, opaque/nonnumeric rejection, non-evaluation, and exact versus
one-under resource caps. See [symbolic body checking](generic-copy-bodies-v2.md).
