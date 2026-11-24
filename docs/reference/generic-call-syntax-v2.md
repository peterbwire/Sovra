# Source-backed explicit generic call envelopes

`parse_generic_call` admits one complete `callee::<T,E>(values...)` source
fragment. It retains a canonical root/qualified callee path and original full
callee/call ranges. Qualified paths are syntax only, not module visibility proofs.
There is no optional statement semicolon in this fragment API.

Explicit type arguments use the approved bounded type grammar. Their original
nodes/spans are relocated into the retained original call source without editing
text. The embedded argument list shares the complete call source; its token count
is the independently parsed list-fragment count including EOF. The call's own
`token_count()` reports the whole-call count including EOF. Existing caller-scope
argument checking accepts this embedded list with original-call diagnostics.

Value arguments remain **opaque token fragments**. They are nonempty, lexed and
delimiter-balanced, with source-order original ranges. Commas inside arrays,
records, nested calls and explicit nested type arguments do not split the outer
list. String/comment contents do not act as delimiters. Ordinary comparisons do
not open type-argument brackets. Empty value lists are allowed; trailing commas,
mismatched/unclosed delimiters and trailing input refuse the envelope.

`GenericCallSyntaxLimits` independently caps whole source/tokens, copied callee
name bytes, value-range count and value delimiter nesting (including the outer
call, fixed maximum 128). Type-list source/token/node/edge/name/argument/depth
limits remain independent. Retained type-node relocation uses bounded flat copies;
source references are shared. Lexing/scanning and retained syntax are bounded by
these logical profiles, not physical process isolation or hard time deadlines.
No partial successful envelope is returned on failure.

This proves an envelope, **not valid value expressions or a checked call**.
For example, an opaque fragment containing invalid expression grammar can pass
this stage and must fail later expression parsing. [Value expression parsing](generic-call-expressions-v2.md) is now available
as a separate bounded stage. Callee resolution, caller-local scope association,
body expression contextualization, source-call edges, cumulative body
call budgets, concrete body rechecking, specialization and runtime integration
remain open. Existing public Program/CLI parsers are unchanged and do not execute
generic calls.

`tests/generic_call_syntax_v2.rs` covers original Unicode ranges and qualified
paths, nested commas/literals/comparisons, opaque syntax boundaries, malformed
forms, exact/one-under whole-call quotas, independent type/depth limits and
caller-scope type checking of relocated argument nodes.
See [explicit type arguments](generic-type-arguments-v2.md).
