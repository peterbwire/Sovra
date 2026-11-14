# Typed callable expression evidence

Status: **Partial E1 lowering foundation.**
`CheckedEnumTypes::check_typed_callable_function_source` extends the
[callable-source checker](callable-function-v2.md) with core evaluated-expression
types and explicit links between checked values, calls and conversion contexts.
Existing callable/source APIs and their quota behavior remain unchanged.

`CheckedTypedCallableFunction::types()` retains immutable `TypedExpressionFact`
records. Each identity belongs to one typed checked unit; another unit refuses
it through `expression(id)`, even for identical source/environment input. Clones
preserve identities. Ordinals follow semantic completion order, not source order.
Original spans are locations, not identity keys. The recorder uses borrowed AST
addresses only during checking; those transient maps are discarded before the
checked syntax is moved into its result. No addresses are retained in public facts.

Expression facts come from the core checker's value-expression callbacks. Calls
complete after their arguments and retain their checked result types. Function
name heads are resolved targets, not first-class typed values. Direct write
identities and match scrutinee binding uses are separate unfinished work; they
are not manufactured as value expressions. Existing empty-array Unknown holes
remain explicit rather than being presented as inferred concrete types.

`TypedCallLink` connects each resolved callable-table entry to its result
expression. `TypedContextLink` connects every retained context to its actual
value identity. These table indexes belong to the wrapped callable result.
`ArrayElement` contexts now record each literal item's actual type against its
final inferred element type, including identity, scalar Int-to-Float and recursive
array widening. For `[[1, 2.0], [3, 4]]`, facts retain both the conversion of `1`
to Float and the conversion of `[3, 4]` to `[Float]`. An enclosing constructor or
return context separately supplies its required payload/result type.

`ExpressionEvidenceLimits` independently caps unique value expressions,
cumulative copied scalar/named/array type nodes and copied nominal-name text.
`TypedCallableLimits` also includes earlier source/signature/call/context limits.
Array-element contexts share the callable context/type-copy counters; links share
their corresponding call/context caps. Transient lookup maps are bounded by those
entry counts. Each retained type copy is charged, while temporary canonicalization
and core checker scratch remain outside retained accounting. These logical quotas
do not measure physical heap or bound all semantic work. Zero limits are real.
Exhaustion refuses without partial typed facts or fallback; ordinary semantic and
required-return errors retain precedence.

This result is **not complete lowering-ready evidence**. Its scoped declaration,
read/write and match-scrutinee facts are supplied by a separate
[scoped callable API](scoped-callable-v2.md), which now retains checked value identities.
Structural IR mapping/validation and executable lowering remain open.
Nested matches, other user functions, complete programs, executable IR/runtime,
backends/native, packages and tooling remain open. Existing public v1 types,
formats and CLI behavior are unchanged. See the
[approved completion contract](../design/G13_COMPLETION_CONTRACT.md).
