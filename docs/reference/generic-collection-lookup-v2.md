# Typed collection lookup (explicit v2 profile)

`compile_generic_collection_package` and `execute_generic_collection_package`
now admit these source operations:

| Operation | Result |
| --- | --- |
| `std::map_get(map, key)` | `std::Option<V>` for `std::Map<K,V>` |
| `std::set_get(set, key)` | `std::Option<K>` for `std::Set<K>` |

The receiver fixes the exact key type. Lookup accepts no explicit builtin type
arguments and performs no numeric widening. Present keys return a complete,
independent `Some` snapshot; absent keys return a nullary `None`. No default
value is synthesized, including for recursive or otherwise uninhabited payload
types. Strings compare exact UTF-8 contents without normalization. Lookup neither
changes the collection nor its insertion order.

The compiler derives the sealed standard Option type directly from the checked
payload type. Alias transparency, imported nominal identities, phantom arguments
and generic parameter ownership survive this derivation. Its copied type arena,
edges, work and structural depth use finite per-call and cumulative body limits.
Concrete body checking and typed IR independently verify the exact receiver/key
and standard Option/payload relationship. The result supports ordinary enum
matching, `?` propagation and checked generic calls.

## Runtime costs

Operand evaluation retains its existing snapshot costs. Searches visit keys in
insertion order, stopping at the first match. Each Int/Bool comparison costs one
shared VM work unit; String comparisons cost one plus both UTF-8 byte lengths,
charged before inspecting contents. Cancellation and elapsed-time checks remain
active through shared budget checkpoints.

For the returned value, let `N` be one Option wrapper plus every payload node
(or just one for `None`), and `T` be all owned type/variant labels plus all payload
text bytes. The VM preflights and admits **all `N` nodes and `T` bytes together**
before allocating or copying the result. Copy work costs `2*N + T`. Payload nodes
are copied once, then attached to the new wrapper without further admission.
`std::Option` plus `Some`/`None` costs 15 label bytes in either case. `None` has no
payload nodes or fabricated payload bytes. Earlier operand/search work is never
refunded. Work exhaustion refuses execution rather than returning a false miss.

Nested records, enums, arrays, maps and sets preserve complete descendant
ownership. Internal map entry nodes remain private and are copied when they are
part of a returned nested map. A returned value can be changed independently
without changing the stored snapshot.

See the [executable source example](../examples/intrinsic-collection-lookup.svr)
and [collection operations](generic-collection-operations-v2.md). The example is
qualified through the explicit Rust package API, not ordinary CLI execution.

## Remaining gates

Default compiler/CLI execution and native image encoding still refuse the
collection profile. Broad CLI/native qualification remains pending. C1 and G13 remain open.

[Typed removal](generic-collection-removal-v2.md) now returns standard Options
of updated immutable collections in the same explicit profile.

[Typed iteration snapshots](generic-collection-iteration-v2.md) now provide
map keys/values and set values as insertion-order arrays in the explicit profile.

[Public map entry snapshots](generic-collection-entries-v2.md) now return typed
standard key/value records in the explicit profile.

[Collection display](generic-collection-display-v2.md) now renders maps, sets and
nested values in the same explicit profile with bounded work and output costs.

The separate [word-1023 collection image API](generic-collection-native-images-v2.md)
now supports bounded source-backed encoding/rechecking/execution. References above
to native encoding refusals describe the ordinary encoder and installed CLI/native
launch path; those retain their gates. C1/G13 remain open.
