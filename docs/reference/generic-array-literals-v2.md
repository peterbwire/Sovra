# Context-typed symbolic array literals

`check_generic_copy_bodies` accepts array literals when an explicit expected
array type is available from a typed local, function return, mutable local
assignment or indexed replacement. Examples include `let items:[T]=[value]`,
`return []` from a `[T]` function, and nested literals in a `[[T]]` context.
The expected element type flows into nested array literals only; it does not
change numeric arithmetic or infer opaque capabilities.

Each item must match the exact expanded element type. Existing scalar Int-to-
Float widening is recorded in `GenericCopyArrayElement::conversion`; the item
expression retains its original type. An existing `[Int]` value is not coerced to
`[Float]`. Empty literals need the same explicit context. Inferred local arrays,
discarded array literals and contexts that are not arrays remain unsupported.

`GenericCopyExpressionKind::Array` stores flat source-order item IDs and their
conversions. The literal retains the expected expanded array type and original
full-source span. No new type arena or parameter owner is fabricated: validated
array-edge projection shares existing immutable annotation/provenance owners.
Typed local annotations are resolved before checking their initializer, while
value lookup still occurs before introducing the new local binding.

Checking uses the existing iterative expression worklist. Item expression facts
bound retained item edges through cumulative expression admission; AST/token
limits also bound temporary worklists. Every item consumes semantic work and
type-comparison admission, and every literal consumes expression admission,
including an empty literal. No array allocation, value copying or evaluation
occurs in this stage. Future lowering must preserve item order, conversions,
snapshot ownership, allocation admission and runtime failure/fuel rules.
Generic calls, inferred array construction, specialization and execution remain open.

`tests/generic_array_literals_v2.rs` covers opaque/empty/nested literals,
assignment contexts, explicit item widening, lack of aggregate coercion,
context/type/name errors including unreachable items, Unicode source/item order,
and exact/one-under cumulative expression/comparison/work quotas.
See [symbolic bodies](generic-copy-bodies-v2.md).
