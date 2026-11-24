# Explicit symbolic generic enum construction

`check_generic_copy_bodies` accepts registered root enums using approved
`Choice::<U>::One(value)` and nullary `Choice::<U>::End()` spelling. It checks the
nominal enum/arguments, Key bounds, variant identity and exact zero/one payload
count. Type arguments follow the enum head, not the variant. Bare nullary names,
implicit types, alias/qualified enum heads and function-value variants refuse
this profile. A local value with the enum's name does not replace its type name.

Nullary variants retain `payload: None`. A declared Unit payload still requires
one Unit-valued expression and retains `Some(GenericEnumPayloadValue)`. No Unit
value or missing payload is fabricated. Different enums or type applications
remain incompatible even when their payload layouts match. Existing scalar
Int-to-Float payload widening is explicit; aggregate coercions remain refused.

Payload contexts use the checked enum's exact substituted type, including opaque
caller parameters, aliases, nested arrays/records/enums and generic calls. The
private parser adapter retains original explicit-list metadata beside existing
qualified call ASTs; public Program/CLI grammar and execution remain unchanged.
Nominal head expansion is shared with record construction under the same bounds.

`GenericCopyExpressionKind::Enum` retains declaration ID, original variant index,
original full nominal-head and short variant ranges, and optional arena-owned
payload/conversion. Nominal type nodes share the original function source; copied
payload contexts retain their original input/alias sources via `node_source`.
`GenericCopyBodyError::EnumType` keeps full-function input coordinates and separate
alias provenance. Standalone explicit-list syntax errors retain their existing
fragment range under the outer call/type-range wrapper.

`max_enum_constructions` independently caps enum facts across every body, reported
by `enum_constructions()`. Shared call-input ledgers count nominal constructor
heads/arguments and generated types; value edges include actual enum payloads.
Function-call and record-constructor counts stay separate. Payload context nodes/
edges/work/depth consume the existing field-substitution profiles and cumulative
ledgers. Nullary constructors need no payload/context budget. Existing AST,
expression, comparison and shared body-work bounds remain active. No constructor
or function resets these ledgers; no partial checked body is published on failure.
Logical admission is not physical process isolation or a hard execution deadline.

These facts do not create runtime enum tags or layout/payload storage. [Enum pattern/type coverage](generic-enum-patterns-v2.md) is now a separate
bounded stage. Arm body typing and executable match plans, concrete body rechecking,
bounded specialization graphs and runtime/
backend integration remain open. Future lowering must preserve payload evaluation,
variant identity, snapshot ownership, allocation/fuel/error rules and nullary versus
Unit distinction. The flat expression worklist does not execute constructors.

`tests/generic_enum_construction_v2.rs` covers original nominal/variant/value IDs
and Unicode ranges, nullary/Unit and preceding-sibling preservation, nested contexts,
wrong enums/type applications, bounds, wrong argument placement, aliases/qualified
heads, unreachable errors, alias provenance, exact/one-under budgets, independent
payload caps, cross-function ledgers and public generic CLI strictness.
See [record construction](generic-record-construction-v2.md).
