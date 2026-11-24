# Source-backed symbolic generic body calls

`check_generic_copy_bodies` now checks original explicit root calls such as
`identity::<U>(value)` in the active lexical scope. A private generic-body parser
adapter retains the exact `::<...>` range beside existing call AST nodes. The
public Program grammar and CLI execution remain unchanged. Qualified/module,
implicit-type and function-value calls remain outside this body profile.

The checker refuses an active local/parameter shadowing the callee name, resolves
the root in the complete signature table, checks explicit types in the original
caller scope, matches callee arity/Key bounds and substitutes the callee signature.
It then checks original value ASTs against those parameter contexts, including
nested explicit calls and nested/empty array literals. Exact types and existing
Int-to-Float scalar widening are allowed; aggregate coercion remains refused.
Initializers resolve before their new binding, and branch/loop scope restoration
continues to determine root-callee visibility.

`GenericCopyExpressionKind::Call` retains sealed instantiated signature evidence,
the original full-function type-list range and source-order arena-owned value IDs
with conversions. Result types preserve exact caller parameters/nominal identities;
implicit Unit is represented without fabricating a source annotation. Callee
identifiers are not fabricated as value-binding reads. Expression facts remain
flat and checking uses an explicit worklist, including nested calls.

Type-list parsing uses the exact original fragment without text edits. Nested
input errors retain fragment-local ranges and alias provenance under the outer
`GenericCopyBodyError::Call.type_range`; callers must not interpret inner spans
as full-function coordinates. `ExpandedGenericType::node_source` selects every
retained annotation/argument/alias source. Original value and whole-call spans
remain in the original function source.

`GenericBodyCallLimits` caps cumulative calls, value edges and generated argument/
signature nodes/edges across every body. Its `arguments` profile supplies cumulative
type-argument count, source bytes, tokens (including each fragment EOF), copied
input nodes/edges/names; its generated/work limits also independently cap each
argument-checking stage. Independent syntax, matching and signature profiles remain
active. All stage work is additionally charged to shared body work; value matching
uses existing cumulative pair/edge/work admission. `call_usage()` reports each
call/input/generated dimension. No call may reset these ledgers, including across
functions or unreachable paths. Original lexer/parser/AST bounds still cover
pre-semantic temporary metadata. Speculative index probes roll metadata back on
fallback, enforce structural depth and clean discarded expressions iteratively.

Every body and unreachable statement is checked. Recursive and growing-type
recursive calls are checked against signatures once per source call, without
following or executing the callee. This is symbolic call evidence, **not executable
IR or a finite code-specialization proof**. Concrete body substitution/rechecking,
bounded specialization graphs, module visibility, runtime calls/fuel/ownership
and backend integration remain open. Runtime short-circuiting, branches and loops
must not be replaced with eager sequential execution of the flat facts.

`tests/generic_body_calls_v2.rs` covers original source/callee/value identity,
nested/contextual-array calls, active shadowing/restoration, counts/bounds/errors,
unused/unreachable bodies, scalar versus aggregate conversion, symbolic recursion,
every exact/one-under cumulative call dimension, cross-function budgets, independent stage-profile refusal,
fragment diagnostics, qualified-call refusal and speculative index/depth handling.

[Explicit generic record construction](generic-record-construction-v2.md) now
shares the type-input/generated ledgers and value-edge budget described here;
function-call count stays separate and constructor field contexts also consume
field-substitution budgets.
