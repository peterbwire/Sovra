# Checked opaque collection layout plans

`GenericCollectionLayout` binds checked closed type arguments to a compiler-owned
Map or Set constructor. It consumes [generic type tokens](generic-collection-values-v2.md)
and creates [typed generic map storage](generic-collection-maps-v2.md) or existing
[primitive set storage](runtime-sets-v2.md). This is an explicit Rust API beside
the executable generic layout table, not source or native feature admission.

The current executable table continues to represent primitives, arrays, records
and enums. These collection plans introduce no ordinary record fields, fake enum
variants, physical ABI or reinterpretation of existing layout numbers. The separate
[opt-in graph planner](generic-collection-layout-graphs-v2.md) now publishes opaque
`GenericRuntimeLayoutKind` shapes under cumulative layout/instantiation limits;
this lightweight argument-plan API alone does not publish them. Production annotation/executable specialization/IR/VM and versioned
native integration remain required before source programs can use intrinsic
`std::Map`/`std::Set`.

## Argument and constructor proof

`map(K,V,limits)` requires exactly a checked key token and checked value token;
`set(K,limits)` requires only a key token. Key admission reads the sealed primitive
type and accepts Int, Bool or String. Float, Unit, arrays, records and enums refuse,
including empty records or record names that resemble primitive/collection names.
There is no default value argument, runtime reference or name-based type authority.

`kind` and descriptive `qualified_name` identify the compiler-owned constructor.
`argument_count` and `argument(index)` expose fixed key/value argument order, with
None for unavailable positions. `key_domain` is derived from the checked key token;
callers cannot supply a conflicting primitive storage domain. Private fields
prevent inconsistent kind/arity/argument plans.

Exact value owners and all concrete type arguments remain retained. Aliases agree;
root/dependency homonyms, foreign check owners and phantom generic argument changes
do not. Plan equivalence requires matching constructors and arguments under the
stricter intersection of both plans' type profiles. Primitive key identity remains
portable across separately checked contexts. A user record named Map remains an
ordinary nominal record and can be a map value; it is not an intrinsic collection.

## Bounds and storage factories

Each argument is validated under explicit finite `GenericTypeEquivalenceLimits`
before a usable plan is returned. There are at most two argument comparisons per
plan operation; bounds are per comparison. Exhaustion returns an error without a
partial successful plan or guessed type agreement. `type_limits()` exposes the
retained profile. Cloning a plan shares existing checked metadata, not a runtime
value or additional source authority. This API does not introduce a cumulative
compiler layout/instantiation budget; executable-table publication must use the
existing combined specialization/layout admission separately.

`instantiate_map(fuel)` accepts only a Map plan and derives its key domain,
retained value proof and finite comparison profile for the existing map storage.
`instantiate_set(fuel)` accepts only a Set plan and derives its primitive key domain.
Wrong-kind calls return Kind before materialization. Both successful factories
charge exactly one empty-container node and zero text bytes. Existing complete
snapshot/update/iteration costs apply to populated storage; no defaults or
fabricated entries are created.

Plan checking does not materialize language values. Source spans/visibility and
registration remain the responsibility of their checked argument issuers and
future source integration. Factory and comparison limits are not instruction,
cooperative-cancellation, elapsed CPU or allocator-memory isolation.

## Qualification and remaining work

Tests cover fixed arity, retained argument owners, alias/phantom identity, all
primitive key domains, rejected compound/Float/Unit keys, type-limit refusal,
source deletion, map/set factory behavior, exact container admission and unchanged
same-spelled user-record semantics. Default source compilation tests keep intrinsic
collection types refused until executable support exists. The separate
[opt-in semantic checker](generic-collection-types-v2.md) can now validate their
annotations without authorizing execution. Native feature words,
legacy Value and existing record/layout semantics are unchanged.

Intrinsic annotation registration now exists in an
[opt-in package semantic API](generic-collection-types-v2.md). Next: publish checked
executable layout/IR/VM operations together with result-cost
and versioned native qualification. C1 and G13 remain open.

Actual checked graph shapes can now bind these storage plans through
`from_checked_layout`; ordinary record shapes and invalid local indices refuse.
See [layout graphs and the IR guard](generic-collection-layout-graphs-v2.md).
