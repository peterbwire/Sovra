# Generic value-argument type matching

`check_generic_call_values` matches supplied checked caller expression IDs to a
sealed instantiated callee signature. The body must own the exact bound caller
signature ID, every expression must belong to that exact body arena, and the supplied value
count must equal the substituted parameter count. Foreign bodies/expressions
and mismatched lists fail without partial successful evidence.

Exact expanded types preserve opaque caller parameter and nominal identity.
Concrete Int-to-Float widening is recorded in each `GenericCallValueArgument`;
no opaque conversion, narrowing or aggregate coercion is introduced. `[Int]`
does not match `[Float]`. The result retains original argument order/IDs,
conversion metadata, the original checked body and instantiated signature.
Mismatch and comparison errors identify the original caller expression range.
The callee's explicit result or implicit Unit convention remains inspectable.

`GenericCallValueLimits` caps argument records and cumulative comparison pairs,
edges and work. Per-comparison limits are also capped by remaining allowances.
Scalar comparisons consume one pair and one comparison work unit, including
widening decisions. Empty argument lists still require owner-checking work.
Exhaustion is never interpreted as equality or an ordinary type mismatch.
These are logical limits, not process isolation or hard CPU deadlines. Future
body integration must meter repeated value matches cumulatively.

This API proves compatibility of **supplied expression IDs**, not that they appear
in an actual source call. It does not parse a call, context-check new literals,
check callee bodies, execute expressions or prove runtime evaluation order.
Repeated IDs are type-matchable; future source-call construction must establish
original syntax edges and evaluation semantics. Body call facts, literal
contextualization, concrete body rechecking, code specialization and executable
integration remain open. Runtime/CLI behavior and ownership semantics are unchanged.

`tests/generic_call_values_v2.rs` covers exact opaque identity, argument order,
explicit scalar widening, aggregate refusal, foreign owners/counts, original
Unicode caller ranges, exact/one-under cumulative and per-comparison admission,
empty-list work and retained owners/result conventions.
See [signature substitution](generic-signature-instantiation-v2.md).

[Symbolic body calls](generic-body-calls-v2.md) now provide a separate
original-function adapter that connects nested explicit root calls and active
caller scopes. The standalone API described here retains its narrower contract.
