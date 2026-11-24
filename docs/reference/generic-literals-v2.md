# Decoded literal payloads for generic bodies

`decode_generic_literals` consumes [rechecked concrete generic bodies](generic-concrete-bodies-v2.md)
and produces sealed primitive payload tables in specialization instance order.
It reads exact ranges from the retained original function sources and reuses the
existing lexer. It does not evaluate expressions, emit instructions, apply
conversions or enable generic Program/CLI execution.

`GenericLiteralValue` retains signed 64-bit Int values, finite binary64 Float bits,
Bool values and lexer-decoded UTF-8 strings. The current source grammar still uses
its existing decimal literal and escape spellings. No Unit literal is invented;
implicit Unit results remain part of function/control-flow handling. Strings keep
Unicode, supported escapes and embedded NUL bytes without added normalization.
Float bits avoid a later textual reparse or loss of representation. Payload equality
is exact representation equality, not a replacement for runtime language equality.

A source range must contain exactly one literal token surrounded only by balanced
grouping parentheses and lexer trivia. Grouped literals can include comments and
line breaks between tokens. Calls, names, operators, extra statements, unmatched
groups, out-of-range integers, nonfinite floats and mismatching primitive kinds
refuse. Lexer errors are translated back into original function-source byte,
line and column locations, including multiline/Unicode ranges.

For example, `let value:Float = (7);` retains `GenericLiteralValue::Int(7)`. The
original explicit Int-to-Float conversion edge remains in the checked statement.
The decoder does not silently replace the source payload with a Float value.

`CheckedGenericLiterals` retains the complete concrete checking proof and an
instance-order `GenericLiteralBody` list. Each body exposes its original concrete
proof and source-order `GenericDecodedLiteral` facts with original expression ID,
full expression span and primitive value. `literal(id)` first checks the original
body arena and returns None for foreign IDs or nonliteral expressions. Clones
retain all source/table/body owners.

Multiple instances of the same original function share immutable decoded facts
and String payloads. Sharing uses function identity within the retained signature
table; it does not merge unrelated literals merely because their text matches.
Every instance reference is checked against its concrete expression type. All
retained literal expressions are decoded, including unreachable statements.

`GenericLiteralLimits` bounds concrete instances, unique source literal facts,
instance references, relexed source bytes/tokens, copied decoded UTF-8 bytes and
logical work. Fragment tokens include EOF, unlike the underlying lexer frontend
profile. Per-fragment source/token caps and a grouping depth in 1..=128 also apply.
Decoded string bytes are charged once per unique source literal; repeated instance
references are charged separately. Temporary lexer storage is bounded by admitted
fragment source/tokens; the decoded string allowance bounds retained payload
copies. Empty Unit bodies can use zero source/token/payload quotas. Every failure
returns no partial decoded graph. These are logical compiler limits, not CPU
execution deadlines or a process sandbox.

Tests: `tests/generic_literals_v2.rs` and private source-shape/range/diagnostic
regressions in `generic_literals.rs`. The separate
[logical layout planner](generic-layouts-v2.md) now maps the concrete type graph.
Constructor/match plans,
executable lowering/backends, package linking and structured errors remain open.
G13 E2 remains incomplete.
