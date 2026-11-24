# Generic package-owner value bridge

`GenericCollectionSnapshot` is an explicit Rust API for retaining real checked
generic VM values with their enclosing concrete layout proof. It connects the
[generic package compiler](generic-package-compilation-v2.md) to future collection
adapters without converting package-owned records/enums into name-only values or
reinterpreting local layout numbers as global type identities.

This is a bridge, not source-level `std::Map`/`std::Set` admission. Existing
[local runtime maps](runtime-maps-v2.md) still use their own versioned value API;
there is no implicit conversion to or from `RuntimeValueV2`, `EnumId` or
`RuntimeRecordId`. [Generic map storage](generic-collection-maps-v2.md) now consumes this proof directly.
Type/layout/IR instructions, VM collection execution and native/backend feature
qualification remain open.

## Proof custody

`GenericVmOutcome::collection_snapshot(path, fuel, work)` selects from its own
returned value and retains **that outcome's** checked layouts. Selector steps are
record `Field(index)` in declaration order, array `Item(index)` or enum `Payload`.
Missing fields/items, wrong selector kinds and nullary payload requests return
`Missing`. An empty path selects the whole result.

No constructor accepts a borrowed `GenericValueRef`, caller-supplied layout table,
raw node vector, declaration name or numeric layout ID. A caller cannot attach
one program's type evidence to another program's value. The owned snapshot has
no unmetered Clone API. `select` and `snapshot` retain the original proof and admit
complete owned results. Child indexes are rebased within contiguous flat subtrees;
copying and ordinary destruction remain flat for recursive nominal values.

`value()` exposes an immutable borrowed view. `ty()` retains a
`GenericCollectionType` token. Its `layout()` exposes checked metadata and its
`declaration()` exposes the original source-owned nominal ID. Metadata token
cloning shares existing compiler proof; it does not copy a runtime value.

`equivalent(other, limits)` compares exact semantic types with the existing bounded
alias-expanded type comparer. Nominal declaration owners and **all** concrete type
arguments remain significant, including unused arguments of empty records. Aliases
resolve to the same type. Raw layout indices remain local to their enclosing table
and are not used for cross-table type identity. Fresh checks establish fresh
nominal authorities; repeated execution of one retained checked program preserves
its owners. Primitive identities remain independent of source owners.

`is_key()` admits only checked primitive Int, Bool and String types. Float, Unit,
arrays, records and enums cannot become keys through their shape or display text.

## Ownership and work events

If the selected subtree has `N` cached logical nodes and `T` owned String/nominal
label bytes, capture and copying admit **N nodes and T bytes** together before
copying any output node or text. These are the existing generic VM snapshot units,
including record type/field labels and enum type/variant labels. Drops do not
refund fuel. Selector failure and whole-result node/text/work refusal do not
commit any materialization fuel; already completed selector work remains spent.

The caller supplies shared finite `RuntimeSetWork`:

| Operation | Helper work |
| --- | --- |
| Each selector step | 1 |
| Full selected-subtree copy | 2N + T, admitted before output allocation |
| Equality | 1, then N(left) + N(right) + T(left) + T(right) after exact root type agreement |

Equality first checks exact root type under independent
`GenericTypeEquivalenceLimits`. Type-limit exhaustion returns an error, not false.
These type-comparison bounds are separate from the shared value-work counter.
After type agreement, compiler-sealed declaration order permits linear record
field pairing; no field-name search is needed. Array order and enum tags/payloads
remain significant. Different exact types return false. Primitive equality uses
the generic VM's existing rules without Int/Float widening; Float NaN remains
unequal to itself.

Rust selectors, views, type tokens and bool results do not materialize hidden
language values. A future adapter must admit actual language Option/Result and
other result wrappers separately. This API does not dispatch VM instructions,
perform cooperative cancellation or promise physical heap/elapsed CPU isolation.
It changes no default CLI policy, legacy public Value or native feature word.

Proof retention keeps original source/type evidence alive after source deletion;
it does not reread project files or grant new filesystem capture authority.

## Qualification

Real captured package graphs exercise imported record owners, root/dependency
homonyms, aliases, phantom generic arguments, arrays and enum payloads. Tests also
cover fresh versus retained check identity, exact atomic node/text/work limits,
missing selectors, changed values under equal types, array order, primitive key
admission and a 128-link recursive generic record with flat snapshot/select/drop.

[Generic map storage](generic-collection-maps-v2.md) now consumes these proof-carrying
values. C1 and G13 remain open pending source/layout/IR/VM and versioned native
qualification.

Nonempty source values now support [typed insertion](generic-collection-insertion-v2.md),
association/membership equality and bounded MapKey/MapValue/SetItem snapshot selectors.
Internal map entry wrappers remain unavailable as language values.
