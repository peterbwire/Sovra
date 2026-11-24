# Generic nominal operation and dispatch plans

`plan_generic_construction` consumes [logical generic layouts](generic-layouts-v2.md)
and links every concrete record constructor, enum constructor, record field read
and exhaustive match to that layout table. Plans retain original source identities
and explicit conversions. They are sparse operation metadata, not executable
instructions, runtime values, jump addresses or a native ABI.

Record plans keep initializers in source evaluation order while naming each
field's declaration-order destination slot. Each `GenericPlannedOperand` retains
its original expression ID, pre-conversion layout, target layout and original
conversion. Every slot must occur exactly once. For example, a source initializer
`Pair::<Int,Float>{right:2,left:1}` plans slots `[1,0]`; its first operand keeps an
Int source layout and explicit Int-to-Float conversion. Planning does not reorder
initializers to match storage order or infer a replacement conversion.

Enum plans retain the exact declaration-order tag and zero/one payload operand.
A Unit payload remains Some and a nullary variant remains None. Field-read plans
retain the original receiver, receiver record layout and declaration-order slot;
the slot's layout must equal the result layout. Constructor operands and field
receivers must belong to the selected body arena and precede their expression.

A `GenericMatchDispatch` records one original scrutinee binding read, its nominal
enum layout, source-order arms and an exhaustive tag-to-arm vector. Reordered
source arms preserve their order while dispatch uses declaration-order tags.
Each payload destination must be the original immutable `MatchPayload` binding
with the matching scrutinee/tag origin and exact payload layout. Nullary and Unit
arms remain distinct. Source arm statement ranges are contiguous, bounded and
checked against each arm's original source span, including nested facts. These
ranges identify original statements, not executable labels. Future execution must
evaluate the scrutinee once and create an immutable payload snapshot before
running the selected arm; this stage does not perform that evaluation/allocation.

`CheckedGenericConstruction` retains all original layout/literal/concrete/source
proofs and instance-order `GenericConstructionBody` tables. `expressions()` holds
only nominal constructor/field plans; `matches()` holds match plans in original
flat statement order. Other expressions/statements remain in the concrete body
facts and require their own instruction lowering. `expression(id)` validates the
original arena and returns None for foreign or non-nominal expressions.
`match_statement(index)` uses a statement index local to the selected body.
Multiple instances share source IDs but can have different layouts: select an
instance before looking up a source expression. Clones retain all original owners.

Every retained nominal operation is planned, including unreachable statements
and branches. Collection order does not authorize executing all plans as a linear
stream: conditional, loop and short-circuit control flow still needs lowering.
Original lexical scopes and return coverage remain supplied by the retained sealed
body proof.

`GenericConstructionLimits` bounds instance wrappers, plan records, record field
mappings, source operands/receivers/scrutinees, arm records, dispatch entries and
logical work across the entire graph. Field coverage and tag coverage storage are
admitted before allocation. Work includes source scans and arm-range visits.
Exact-layout Identity and primitive Int-to-Float are the only accepted retained
conversions. Empty explicit records and nullary constructors can use zero operand/
field allowances. Failure publishes no partial plan and has no execution fallback.
These are logical compiler limits, not runtime heap/CPU policies.

Tests: `tests/generic_construction_v2.rs` and private negative regressions in
`generic_construction.rs` for duplicate slots/tags, incorrect conversions,
inconsistent/escaping arm ranges and missing payload destinations.

The separate [typed generic IR lowerer](generic-ir-v2.md) now emits nominal, array
and other expression instructions with resolved control flow and typed joins.
Remaining work includes backend/runtime integration, value allocation and
snapshot/bounds behavior, package linking and structured errors. G13 E2 remains
incomplete; generic Program/CLI execution stays unavailable through these APIs.
