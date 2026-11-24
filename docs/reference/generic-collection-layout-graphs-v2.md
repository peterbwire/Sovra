# Closed collection layout graphs

`plan_generic_collection_layouts(literals, limits)` extends the existing bounded
layout planner with opaque Map and Set shapes from the
[opt-in collection semantic checker](generic-collection-types-v2.md). It returns
a complete checked layout table. This is layout publication, **not** permission
to lower collection operations, execute collection values or encode new native
features.

The original `plan_generic_layouts` preserves its strict profile and refuses an
intrinsic collection before returning a table. Production compiler/CLI/native
paths keep that profile. Existing record/enum layouts, commands, old feature words
and logical value events are unchanged.

## Shapes and identity

`GenericRuntimeLayoutKind::Map` retains the compiler-owned collection declaration
ID and table-local key/value layout edges. `Set` retains its declaration ID and
key edge. Neither has record fields, enum variants, fabricated entries or a chosen
physical ABI. Key arguments are checked against the same primitive Int/Bool/String
constraint during semantic/concrete checking and layout discovery.

Layout interning uses the existing exact normalized type key: constructor owners
and all concrete arguments matter. Transparent aliases deduplicate. Changed key
or value arguments, different constructor IDs and phantom generic arguments do
not collapse because runtime field counts happen to match. Argument edges remain
local to the enclosing table and are not standalone type identities.

Collections participate in the same closure as records, enums and arrays. Each
type is interned before discovering children. A Node<Int> field of type
Map<String,Node<Int>> therefore closes back to its existing Node layout instead of
expanding recursively forever. Expanding substitutions such as Grow<T> containing
Map<Int,Grow<[T]>> still discover distinct instances and refuse at the shared
instantiation/depth/work limits.

## Existing cumulative limits

The existing combined function/type instantiation cap includes Map and Set
instances. They consume ordinary layout records, argument edges, key-generation,
reference and work allowances. They consume no record-field/enum-variant members.
There is no per-collection or per-file reset. Limit failure returns no partial
checked table. Collection-free planning under the opt-in function has the same
usage as the original planner.

`has_collections()` records whether any opaque collection shape was discovered,
including through phantom generic arguments. Merely enabling the opt-in planner
or registering unused collection headers does not set it. The marker is derived
by the private complete-table builder; callers cannot override it.

## Type proof and storage binding

`GenericCollectionType::from_layout(table, index)` retains metadata from an admitted
layout after checking the local index. It does not attach a caller-supplied type
to an existing runtime value. Snapshot construction still couples each value to
its own original proof.

`GenericCollectionLayout::from_checked_layout(table, index, type_limits)` accepts
only an actual Map/Set shape, derives its checked argument tokens and reuses the
[typed storage factories](generic-collection-layouts-v2.md). Bad indices refuse;
ordinary records, even records named Map, cannot impersonate a collection shape.
Storage preserves exact value-owner checking across distinct layout tables.
Tests compile a record-producing helper and a Map-parameter root from the same
checked package bodies: their local value-layout numbers differ, but the correct
nominal value inserts successfully, while a same-shaped different-owner record
refuses.

## IR and runtime gate

The original `plan_generic_construction` explicitly returns
`CollectionOperationsUnavailable` for any table marked with collection shapes.
The ordinary propagation-IR lowering entry also passes through this guard. The
separate [initial collection operation profile](generic-collection-operations-v2.md)
now opts into qualified empty creation and length operations. No fields,
enum constructors or hidden fallback values are synthesized to bypass it. An
otherwise empty record with a phantom Map argument still triggers the guard.

A collection-free root continues through existing IR/VM execution normally, even
if its checked semantic context registered unused intrinsic headers. Layout publication alone adds no runtime authority. The separate initial operation
profile now owns empty collection VM values under shared controls; native encoding
remains gated.

## Qualification and next work

Tests cover opaque shapes, exact argument edges, no fake members, alias interning,
constructor/phantom identity, recursive value graph closure, expanding recursion
refusal, exact cumulative limits and one-under refusals, strict old planner,
phantom-feature IR refusal, preserved ordinary execution and checked storage
binding across different layout tables.

[Empty creation and length](generic-collection-operations-v2.md) now execute through
an explicit package API. Next: nonempty operations, complete language result-wrapper
admission and versioned native/backend qualification. C1 and G13 remain open.
