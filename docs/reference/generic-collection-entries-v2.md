# Typed map entry snapshots (explicit v2 profile)

`std::map_entries(map)` now returns `[std::MapEntry<K,V>]` from a checked
`std::Map<K,V>` through `compile_generic_collection_package` and
`execute_generic_collection_package`.

`std::MapEntry<K:std::Key,V>` is a compiler-owned standard record with two fields,
`key:K` and `value:V`. Entries are ordinary checked records: field access, explicit
record construction, generic parameter passing, array operations and structural
record equality work normally. Their sealed standard identity differs from any
same-spelled user record. Float and compound keys remain excluded by `std::Key`.

The result array preserves insertion order. Replacement keeps an entry's position;
remove/reinsert appends. Empty maps return empty typed arrays without inventing a
key, value or record. Ordered entry-array equality remains order-sensitive even
when the original maps compare equal by associations. Keys and values retain their
exact checked types, including imported identities, aliases and phantom arguments.
Changing a mutable local derived from a returned value preserves the map snapshot.

The standard record header is registered only by the explicit collection package
profile, consuming one declaration, two parameters and all ordinary record
source/member/annotation admission. Default type registration remains unchanged.
The compiler derives the entry application and enclosing array from the retained
map proof in one bounded expansion worker, charging generated nodes, edges, depth
and work cumulatively. Concrete bodies independently check the sealed standard
identity and exact key/value arguments. Structural and typed IR checks verify the
array element's sealed entry identity, type arguments and field layouts against
the map receiver. User lookalikes and swapped arguments refuse.

## Ownership and cost

The VM's private map-entry node is never exposed as a language value. Each public
entry is a new ordinary record wrapper around complete copied key/value subtrees.
No private layout identity is attached to a public record.

For `n` entries, result node cost is
`N = 1 + sum(1 + key descendant nodes + value descendant nodes)`.
The first node is the array; each additional wrapper is one public entry record.
Text cost `T` includes all copied descendant text/type/variant/field labels and
**21 bytes per entry** for `std::MapEntry` plus `key` and `value`. Empty results
cost one array node and zero text bytes. Nested maps/sets/records/enums/arrays keep
all their descendants and owners; no transferred child is admitted twice.

Operand evaluation retains its existing receiver snapshot costs. Planning checks
the shared u32 length ceiling, debits one work unit per entry and performs the
ordinary record-label schema checks. It preflights all `N` nodes and `T` bytes,
charges `2*N + T` result copy work and admits the entire result before allocating
or copying it. Ordinary label inspection contributes two work units, including
for empty results. Cancellation/elapsed-time checks and shared work/materialization
limits remain active. Exhaustion refuses rather than exposing a partial array.

The [executable source example](../examples/intrinsic-collection-entries.svr)
uses a generic entries helper, an indexed loop and public field access. Tests
cover ordered updates, nested imported payloads, source construction, homonyms,
phantom arguments, exact result/proof limits, default-profile/native gates and
64 update histories against an ordered reference model.

## Remaining gates

CLI/native admission and consolidated
backend/host/resource qualification remain open. Native image encoding and
ordinary execution still explicitly refuse this collection profile, including
entry-only programs. C1/G13 remain incomplete.

[Collection display](generic-collection-display-v2.md) now renders maps, sets and
nested values in the same explicit profile with bounded work and output costs.

The separate [word-1023 collection image API](generic-collection-native-images-v2.md)
now supports bounded source-backed encoding/rechecking/execution. References above
to native encoding refusals describe the ordinary encoder and installed CLI/native
launch path; those retain their gates. C1/G13 remain open.
