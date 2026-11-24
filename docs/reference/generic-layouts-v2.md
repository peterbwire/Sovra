# Logical runtime layouts for generic types

`plan_generic_layouts` consumes [decoded generic literals](generic-literals-v2.md)
and discovers a complete bounded layout graph for the retained concrete types.
The layouts are backend-neutral descriptions of primitives, arrays, record fields
and enum tags/payloads. They select no byte sizes, alignment, pointer format,
allocation strategy or native ABI. This library stage does not execute generic
code or enable generic Program/CLI compilation.

`GenericRuntimeLayoutKind` describes:

- A primitive with its existing value contract.
- An array with an element layout index.
- A nominal record with source-order type-argument and declaration-order field indices.
- A nominal enum with source-order type arguments and declaration-order variants.
  The original variant index is its tag; None is nullary and Some(Unit-layout)
  remains a payload-bearing variant.

Field/variant names stay in the retained checked definitions; no new labels are
copied. A constructor's initialization order remains in its original body facts,
separate from declaration-order storage slots. Lowering must preserve that order.
Layout indices and tags are local compiler metadata, not a persistent codec or a
stable external ABI.

Every function instance maps its concrete type arguments, value parameters,
result, bindings and expressions into one shared table. Phantom function arguments
are included. Nominal type arguments are also layout edges, even when a declaration
does not use them in fields. Array elements and every record field/enum payload
are discovered, including members not directly read or constructed by the source.
Concrete field/payload substitution preserves caller/alias provenance and rechecks
closed types, arity and nominal Key requirements. Int, Bool and String are the
currently supported generic Key primitives.

Exact normalized type identity reuses aliases, shared/duplicated type graphs and
primitive representations while preserving nominal declaration owners and ordered
arguments. No cache decision relies on source spelling or hash-only equality.
Each type is registered before its children are expanded. Recursive records and
enums therefore form finite indexed edges, including through arrays. This does
not allocate recursive values or invent default constructors. A declaration whose
members keep expanding type arguments, such as `Grow<T>` containing `Grow<[T]>`,
must close within the selected instance/depth/work limits or refuse.

`CheckedGenericLayouts` retains the literal/concrete/body/source proofs, complete
`GenericRuntimeLayout` entries and instance-order `GenericInstanceLayouts` tables.
Each layout retains its first-discovered concrete type representative. Use expanded
`node_source` and node origins to inspect composite provenance. Primitive and
compound layout indices are graph-local; lookup must select the enclosing table.
Binding/expression mapping validates the original body arena. Multiple instances
share source IDs, so select the concrete instance before looking up a source fact.
Clones retain all original source/table owners.

`GenericLayoutLimits::DEFAULT` shares the approved 1,024 instantiation cap between
existing function instances (including the root) and new nominal type instances.
`PACKAGE` selects 4,096 combined instances without loading/linking a package.
Primitives and arrays consume layout records but no nominal instantiation slots.
Other cumulative allowances bound records, members, layout edges, source/child
lookup requests, generated substitution nodes/edges, normalized key nodes/edges
and logical work. Duplicate candidates and exact linear cache comparisons consume
work before reuse. Independent type-arena caps apply to inputs and member
templates before temporary depth/mapping storage; per-substitution/key profiles
remain active. Structural depth must be in 1..=128 and is distinct from cycles
through nominal members. Every failure returns no partial/unresolved table and
has no unchecked or execution fallback. These are logical compiler limits, not a
process sandbox or physical heap/CPU deadline policy.

Tests: `tests/generic_layouts_v2.rs`, including recursive closure, expanding
recursion refusal, phantom arguments, discarded alias arguments, source identities,
exact and one-under budgets, nominal identity and nullary/Unit shape.

The separate [nominal operation planner](generic-construction-v2.md) now links
constructors, field reads and match dispatch to these layouts. Remaining work
includes executable generic IR/control-flow lowering and backends, runtime allocation/snapshot/bounds behavior, package
linking, standard Option/Result and structured errors. G13 E2 remains incomplete.

The [opt-in collection semantic profile](generic-collection-types-v2.md) can now
produce checked opaque Map/Set definitions. The original planner explicitly
refuses those definitions before publishing a layout table. A separate
[opt-in collection graph planner](generic-collection-layout-graphs-v2.md) now
publishes opaque shapes and keeps IR/runtime admission gated; no record or enum
layout is substituted for a collection.
