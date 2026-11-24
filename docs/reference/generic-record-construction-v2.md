# Explicit symbolic generic record construction

`check_generic_copy_bodies` accepts registered root records using explicit
`Box::<U>{value: item, weight: 1}` syntax. It validates nominal type identity,
argument arity and Key bounds, complete field coverage, known/unique field names
and each substituted field type. Missing, duplicate, unknown, incompatible and
unreachable initializers are rejected. Alias/qualified/grouped type heads, inferred
record type arguments and field mutation remain outside this profile.

A private original-token parser adapter retains the explicit-list range beside
the existing record AST. A private type-syntax factory combines its admitted
argument nodes with the original nominal head, without rewriting source text.
The head/argument node spans address the original function source, shared from
its existing immutable source owner. Parenthesized literals retain their full expression ranges while the constructor
head range still identifies `Box::<U>`; grouping the type head itself remains
outside this profile. The resulting semantic type syntax may
spell its source application as `Box::<U>`; it is a constructor-source view, not
a separately parsed standalone `Box<U>` annotation or a public arbitrary-node API.
`node_source(index)` selects the input or alias source for every expanded node.

Field contexts use bounded checked substitution from the constructor's validated
nominal arguments. This supports nested records, contextual nested/empty arrays,
exact opaque caller parameter identities and concrete scalar Int-to-Float item
conversion. Existing aggregate values do not gain coercions. Type names use the
type namespace: a local value named `Box` does not replace the record declaration.

`GenericCopyExpressionKind::Record` retains nominal declaration identity, original
full constructor-head range and source-order `GenericRecordFieldValue` entries.
Each entry records its declaration field index, original arena-owned value ID and
conversion. Source evaluation order therefore remains distinct from declaration/
future layout order. The expression result is the exact expanded nominal type;
existing field reads and generic call arguments can consume it symbolically.

`max_record_constructions` independently caps constructor facts, reported by
`record_constructions()`. Shared `calls` input ledgers count explicit type arguments,
complete constructor heads (bytes, prefix/list tokens including EOF, nodes, root
edges and names), initializer edges and generated nominal types alongside call
metadata. Actual function-call count remains separate. Argument expansion profiles
bound each nominal type; function binding/signature profiles do not run for records.
All substituted field contexts consume existing cumulative/per-field node/edge/
work/depth limits, including retained receiver nodes even for concrete fields.
Existing AST/expression/comparison/body-work admission remains active. No ledger
resets per record or function. Logical quotas are not physical process isolation.

These are symbolic construction facts, **not runtime records or layout code**.
Future lowering must preserve initializer order, field mapping, conversions,
snapshot ownership, allocation/fuel/error rules and concrete body rechecking.
Executable constructor IR, bounded code specialization and runtime/backend
integration remain open. Public Program/CLI generic execution is unchanged.

`tests/generic_record_construction_v2.rs` covers nominal/source/arena identity,
shared source ownership, source versus declaration field order, nested/array/empty
contexts, field errors, bounds/namespaces, exact/one-under constructor/input/
generated/field/work caps, prefix token/name admission and cross-function ledgers.
See [symbolic calls](generic-body-calls-v2.md).

[Enum construction](generic-enum-construction-v2.md) now shares nominal head and
input/generated budgets; its payload contexts share field-substitution admission.
Enum and record constructor counts remain independent.
