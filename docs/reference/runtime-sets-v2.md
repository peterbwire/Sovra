# Opaque primitive-key set runtime foundation

`RuntimeSetV2` is an explicit Rust library API for the G13 C1 implementation. Its
private storage enforces homogeneous, unique keys. It is separate from the
[ordinary Sovra reference package](../../stdlib/snapshot-collections/README.md).
It does **not** expose `std::Set` source syntax, compiler types, generic layouts,
IR instructions, VM values or native image features. No existing runtime model,
CLI default, legacy public Value, or native feature word changes.

`RuntimeKeyType` selects Int, Bool or String, including on empty sets.
`RuntimeKey` admits only those primitive values. Wrong-domain inputs/equality
return `KeyType`. Exact UTF-8 contents determine string identity; there is no
normalization. Inputs cannot forge stored duplicates, reorder storage, mutate
members or bypass snapshot admission through a Clone implementation.

The API provides new, len/is_empty/key_type metadata, contains/get, insert/remove,
keys, snapshot and equal. Insert duplicates retain position. Removing and
reinserting appends at the end. Equality compares membership independently of
order. All results are independent owned snapshots; inputs remain unchanged even
on refusal. Missing get/remove return Rust None. Rust Option, bool and metadata
results do not secretly materialize Sovra wrappers or primitive values. A future
VM adapter must charge the actual language result wrappers/values separately.

## Logical materialization events

All costs use the existing `RuntimeValueFuel` node/text counters. No old event
cost changes. Drops never refund fuel. Every whole result is admitted before
copying its first key or UTF-8 byte. Text and node refusal are atomic together.
Representation length is bounded by 4,294,967,295 keys using checked arithmetic.

| Event | Nodes | Owned UTF-8 bytes |
| --- | --- | --- |
| Empty set | 1 | 0 |
| Set snapshot, insert result, successful remove result | 1 + result key count | Sum of result String key byte lengths |
| Keys array snapshot | 1 + key count | Sum of String key byte lengths |
| Present get | 1 | Stored key String byte length, otherwise 0 |
| Missing get/remove; contains/equality; metadata | 0 | 0 |

Duplicate insertion still creates a complete new snapshot and pays its complete
cost. Removed keys contribute no result ownership. There are no map-entry nodes
in a set. String keys are charged again on every independent copy.

## Bounded helper work

Callers supply a shared finite `RuntimeSetWork`. Comparison charges
`1 + stored String bytes + input String bytes` before each key comparison.
Snapshot planning charges one per visited input key. Copy preflight then charges
`result nodes + result text bytes` before any allocation/copy. Get charges
`1 + stored text bytes` after successful search. Equality charges one for its
length comparison and uses the same bounded membership searches.

Completed work stays consumed after refusal; a refused single preflight charge
leaves its own amount unconsumed. Materialization remains unchanged if helper
work refuses. Helpers use linear ordered storage; equality has quadratic lookup
work. These units bound admitted traversal/copy activity, not elapsed CPU,
allocator memory or cancellation latency. This isolated API has no VM instruction
or cooperative-control integration yet.

## Qualification and remaining work

Tests cover exact node/text/work boundaries, atomic refusals, immutable results,
Unicode distinctions, key-domain refusal, extreme Int/Bool keys and all 1,296
four-operation insert/remove histories over three keys against an ordered oracle.
The representation ceiling and arithmetic-overflow paths are checked without
allocating enormous collections.
The old runtime value suite verifies existing materialization events remain intact.

The [opaque map foundation](runtime-maps-v2.md) now admits primitive/array values
with complete descendant costs. Next: nominal value ownership; sealed generic
type/layout/IR/VM integration; language-level Option/Result/event admission;
versioned native codec/backend qualification. C1 and G13 remain open.
