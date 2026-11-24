# Typed collection iteration snapshots (explicit v2 profile)

The explicit `compile_generic_collection_package` and
`execute_generic_collection_package` APIs now admit:

| Source operation | Receiver | Result |
| --- | --- | --- |
| `std::map_keys(map)` | `std::Map<K,V>` | `[K]` |
| `std::map_values(map)` | `std::Map<K,V>` | `[V]` |
| `std::set_values(set)` | `std::Set<K>` | `[K]` |

Each operation returns an independent array snapshot in insertion order. Keys
and values taken from the same unchanged map have matching positions. Replacing
a map value preserves that position; removing/reinserting a key places it last.
Duplicate set insertion preserves its position. Empty collections produce empty
typed arrays without fabricating an element. Strings retain their exact contents;
iteration does not sort, normalize or deduplicate them again.

The arrays support existing indexing, loops, array algorithms, parameter passing
and snapshot equality. Changing a mutable array local derived from a result does
not change the collection. Checked aliases, imported nominal types, phantom
arguments and generic parameter ownership remain exact. No numeric widening is
performed when deriving a snapshot's element type. Wrong receiver constructors,
result types, arity and explicit builtin type arguments refuse.

The compiler derives array types directly from retained checked collection
arguments, preserving source/environment/parameter ownership under finite
node/edge/work/depth admission. Concrete body and typed IR checks independently
verify the exact collection/element relationship. Runtime execution verifies the
receiver shape and selected array element layout before copying.

## Versioned result costs

Ordinary operand evaluation retains its existing snapshot costs. Snapshot
planning debits one shared VM work unit per selected element, with cooperative
control checks. The representation length retains the existing u32 ceiling.

For the result, `N = 1 + sum(selected descendant node counts)` and
`T = sum(selected owned text bytes)`. The one new container is the array. Text
includes all selected String contents and record/enum type, field and variant
labels. Map keys/values do not copy the enclosing private entry nodes or the
unselected side into the result. Nested maps selected as values do copy all of
their own entry/key/value descendants; nested arrays, records, enums and sets
also remain complete.

Before allocating result arrays or copying elements, the VM preflights and admits
all `N` nodes and `T` bytes together and debits `2*N + T` copy work. Attaching the
children to the array does not admit them again. Empty results cost one array
node, zero text bytes and two copy-work units. Earlier operand/planning work is
not refunded. Exhaustion refuses execution rather than publishing a truncated
array; shared cancellation and elapsed-time controls remain active.

The [source example](../examples/intrinsic-collection-iteration.svr) iterates real
map value snapshots with an ordinary indexed loop. It is qualified through the
explicit Rust package API, not ordinary CLI execution. Tests cover exact result
budgets, nested imported and collection values, source array composition,
independent mutation, type refusals and 64 update histories checked against an
ordered reference model.

## Remaining gates

[`std::map_entries`](generic-collection-entries-v2.md) now supplies checked public
key/value entry records with complete wrapper admission. CLI/native admission and consolidated
backend/host/resource qualification also remain open. Ordinary compilation and
execution and native encoding still refuse this collection profile. C1/G13 are
incomplete.

[Collection display](generic-collection-display-v2.md) now renders maps, sets and
nested values in the same explicit profile with bounded work and output costs.

The separate [word-1023 collection image API](generic-collection-native-images-v2.md)
now supports bounded source-backed encoding/rechecking/execution. References above
to native encoding refusals describe the ordinary encoder and installed CLI/native
launch path; those retain their gates. C1/G13 remain open.
