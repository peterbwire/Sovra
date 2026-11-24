# Original-source generic call value expressions

`parse_generic_call_expressions` parses every opaque value range in an admitted
explicit generic call envelope using the existing expression grammar. A private
core parser suffix entry uses original EOF-terminated tokens; each parse must
consume exactly its original range. No synthetic function, rewritten source,
replacement tokens or coordinate translation is used for value expressions.

`ParsedGenericCallExpressions` retains original envelope/source and source-order
expression roots. Invalid expression grammar, statements, assignments and trailing
expression tokens fail. A postparse operator check refuses unsupported binary
operators admitted by the legacy raw AST parser, including assignment-like forms. Parse failures retain argument index and original call
coordinates; boundary errors identify the remaining original token. Unknown
names and ordinary nested calls may parse because this stage proves syntax only.
Nested explicit generic calls still refuse the current core expression profile.

`GenericCallExpressionLimits` bounds relexed whole-call source/tokens, value roots,
cumulative postparse AST nodes/edges and logical token/root/AST traversal work.
The existing parser depth and lexer diagnostic bounds remain active; the parser
has its own selected diagnostic cap (minimum one). Empty value lists still
consume original token admission/work. AST admission happens after bounded parsing;
source/token/parser limits bound temporary construction before retention.

Retained AST ownership is shared. Unique temporary and final arenas drain roots
with iterative expression cleanup on success, failure and last-owner drop. Node/
edge admission traverses a heap worklist. Work is logical admission, not parser
CPU time, hard deadlines or physical memory isolation.

These are parsed expressions, **not typed values or checked source calls**.
Caller-local scope association, expected-type literal checking, callee resolution,
body call facts, recursive generic expression integration, concrete body rechecking,
specialization and execution remain open. The public Program grammar is unchanged.
Future body integration must meter repeated parses and copies cumulatively.

`tests/generic_call_expressions_v2.rs` covers original Unicode source/ranges,
precedence, exact boundary refusal, malformed expressions, syntax-only names and
plain calls, unsupported nested generics, argument diagnostics, every exact/
one-under input/AST/work quota, empty-list work and shared AST lifetime cleanup.
See [call envelopes](generic-call-syntax-v2.md).

[Symbolic body calls](generic-body-calls-v2.md) now provide a separate
original-function adapter that connects nested explicit root calls and active
caller scopes. The standalone API described here retains its narrower contract.
