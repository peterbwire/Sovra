# Explicit call and constructor type-argument syntax

`parse_generic_type_arguments` parses exactly one nonempty `::<T,E>` list under
`GenericTypeArgumentLimits`. It reuses the existing approved type grammar for
qualified applications and `[T]` arrays. Empty lists, trailing commas, bounds,
expressions, missing brackets and trailing call/value syntax are rejected.
The API handles the explicit list only. A separate
[call-envelope parser](generic-call-syntax-v2.md) now embeds these arguments in
original call-source coordinates while retaining independently counted list tokens.

`GenericTypeArgumentsSyntax` owns the original source and shared immutable
postorder nodes plus source-order argument root indices. Original byte/line/
column spans remain intact, including Unicode and trivia. No subtree rewrite,
flattened spelling or fabricated type parameter identity is introduced.

`types` limits apply cumulatively across the complete list: source bytes, lexer
tokens including EOF, nodes, edges, copied name bytes and per-type structural
depth. Edges include one list-to-root edge per argument in addition to nested
type edges. `max_arguments` independently bounds the root vector. The fixed type
nesting ceiling remains 128. Type parsing uses the existing bounded parser;
retained graph storage and destruction are flat. No partial list is published.

This is syntax admission, not a checked call or instantiated type. Unknown names
and wrong arity/bounds remain syntactically possible. [Caller-owner type argument checking](generic-argument-checking-v2.md) now
resolves names, aliases and type-head constraints. Callee/visibility resolution,
callee arity/bound proofs, body call facts, concrete
specialization and runtime/backend integration remain open. Existing generic CLI
programs remain refused; no inference or new call execution is enabled.

`tests/generic_type_arguments_v2.rs` covers qualified/nested arguments, original
Unicode source/root order, malformed forms, shared exact/one-under budgets,
independent argument/depth caps, syntax-only unknown names and CLI strictness.
See [generic type syntax](generic-type-syntax-v2.md) and the
[approved G13 contract](../design/G13_COMPLETION_CONTRACT.md).
