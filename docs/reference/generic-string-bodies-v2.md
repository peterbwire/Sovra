# Concrete String symbolic body expressions

`check_generic_copy_bodies` accepts String `+` concatenation and lexical
`<`, `<=`, `>` and `>=` ordering, matching existing compiler/runtime behavior.
Both operands must resolve to concrete `String`, including transparent aliases.
Concatenation produces `String`; ordering produces `Bool`.

`GenericCopyExpressionKind::String` retains `GenericStringOperator`, both
source-owned operand IDs and the original full-source span. No implicit
formatting, numeric coercion or numeric capability is inferred. Opaque `T` and
`T:std::Key` cannot use String operations. String subtraction, multiplication
and division remain refused; equality uses the separate existing equality facts.

The iterative body checker charges every String fact to existing cumulative
expression/work budgets. It neither concatenates nor compares literal contents.
These are symbolic typing facts, not executable IR or a memory-safety proof.
Future execution must retain existing byte/fuel admission before concatenation
allocation and preserve existing lexical ordering. Qualified/implicit calls,
specialization and executable integration remain open. No syntax changes or
unary expression support are introduced.

`tests/generic_string_bodies_v2.rs` covers every supported operator, result and
operand identity, rejection of mixed/opaque operands and unsupported operators,
transparent aliases, Unicode/nested source spans, and exact/one-under quotas.
See [symbolic bodies](generic-copy-bodies-v2.md).
