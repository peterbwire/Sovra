# Opaque typed map runtime foundation

`RuntimeMapV2` is an explicit Rust library API for G13 C1. Private storage enforces
unique homogeneous keys and exact value shapes. It complements the
[opaque runtime sets](runtime-sets-v2.md) and is separate from the
[ordinary Sovra reference package](../../stdlib/snapshot-collections/README.md).
This does not admit `std::Map` source types, generic layouts, IR/VM values or native
features. Existing model markers, legacy public Value and command defaults stay
unchanged.

Keys accept Int, Bool or exact UTF-8 String, with no normalization. The flat,
private `RuntimeMapValueType` descriptor admits Unit, Bool, Int, Float, String or
a sealed checked enum or record identity,
optionally wrapped in up to 128 homogeneous array levels. Every inserted value is
checked under the shared finite helper-work allowance. There is no implicit
numeric widening. Empty arrays retain the map's declared value type. Nested collections remain rejected.
[Checked enum leaves](#checked-enum-value-leaves) and
[checked record leaves](runtime-record-owners-v2.md) use sealed identities; names
supplied by Rust callers are not treated as type authority.

Operations provide new, len/is_empty/domain metadata, contains/get, insert/remove,
keys/values/entries snapshots, snapshot and equal. Replacement preserves insertion
position; remove and reinsert appends. Equality compares associations independently
of order, requiring matching key and value domains. Float value equality follows
the existing runtime structural rules, including NaN being unequal to itself.

Insertion, lookup, removal and iteration return independent copies. Missing
get/remove return Rust None. Successful removal returns `RuntimeMapRemoval` with
an updated map and the removed value snapshot; both are admitted together. Returned
`RuntimeMapEntry` fields are mutable Rust snapshots, with no route to forge or
mutate internal map entries. No unchecked collection Clone API is provided.

## Atomic ownership events

Use existing `RuntimeValueFuel` counters. Each complete result is preflighted for
nodes/text, then copy work, before committing fuel and copying any key, label or
value descendant. Refusal preserves inputs and materialization allowances. Already
performed helper work remains consumed; drops never refund fuel. Result lengths
use the shared checked 4,294,967,295-key ceiling.

Let `N(v)` and `T(v)` be all value-descendant nodes and owned UTF-8 bytes, and
`T(k)` the String key's byte count (zero for Int/Bool). Arrays contribute one node
plus their children. Primitive values contribute one node; String bytes contribute
owned text. Enum wrappers contribute one node and own their type/variant labels;
every payload descendant is counted. Intrinsic anonymous entry nodes have no owned labels.

| Event | Nodes | Owned text bytes |
| --- | --- | --- |
| Empty map | 1 | 0 |
| Snapshot or insertion result | 1 + sum over result entries of (entry 1 + key 1 + N(value)) | Sum over result entries of (T(key) + T(value)) |
| Present removal result | Updated map cost + N(removed value) | Updated map text + T(removed value) |
| Present get | N(value) | T(value) |
| Keys array snapshot | 1 + key count | Sum T(key) |
| Values array snapshot | 1 + sum N(value) | Sum T(value) |
| Anonymous entries array snapshot | Same as complete map snapshot | Same as complete map snapshot |
| Missing get/remove; contains/equal; metadata | 0 | 0 |

Replacement copies only the replacement value, rather than charging discarded old
value descendants. Removal does not copy the removed key. Rust Option, Vec and
removal/entry wrappers are API carriers for the stated logical results, not extra
language values. A future VM adapter must admit actual language Option/Result
wrappers and any newly owned nominal labels separately.

## Helper work and typing

Maps share `RuntimeSetWork` with sets. Each key comparison costs
`1 + stored String bytes + input String bytes`. Ingress type checking charges one
per visited value node and one per queued array edge, admitting edges before
extending its iterative work queue. This avoids an unbounded pre-admission value
traversal. Cached node/text footprints belong to private, validated immutable entries.

Result planning charges one per visited entry. Result copying charges
`result nodes + result text bytes` before allocation. Value equality first admits
`left nodes + right nodes + left text + right text` comparison work, then uses the
existing iterative primitive/array structural comparison. Equality also charges
one for its length comparison and ordinary key lookup work.

Ordered storage gives linear lookup scans and quadratic association equality.
Work units do not promise elapsed CPU, allocator memory or cancellation isolation.
This API has no instruction/cooperative-control adapter yet.

## Qualification and remaining work

Tests exercise exact node/text/work boundaries and atomic refusal, full nested
String-array copies, declared empty-array domains, primitive/key mismatches,
Unicode identity, replacement/removal ordering, deep arrays at the 128-level
boundary, and all 4,096 four-operation update histories over two keys and three
values against an ordered association oracle. Existing set/value/VM tests remain
compatibility checks.

Next: generic package nominal-owner adaptation, complete collection type/layout/
IR/VM admission, language result events and versioned native/backend qualification.
C1 and G13 remain open.

## Checked enum value leaves

`RuntimeMapPrimitive::Enum(EnumId)` now admits a sealed enum leaf, including under
the descriptor's homogeneous array levels. `EnumId` can only originate in checked
enum evidence. It includes the immutable environment identity: same-spelled enums
from separately checked environments do not mix, even in empty maps or arrays.
Cloned evidence retains identity. This is local checked-environment authority;
it does not introduce generic package declaration IDs or native serialization.

The existing trusted enum constructor validates variant/payload shape and seals
its identity. Map ingress checks the exact enum ID, then iteratively scans the
entire enum subtree under shared helper work. Every nested enum wrapper contributes
one node plus its owned type and variant labels; String payloads contribute their
bytes, arrays their wrappers, and all payload descendants their full footprint.
Existing enum materialization units are reused unchanged. Copying never transfers
or borrows the original enum's payload. Cached costs include labels and descendants
and apply to get, replacement, remove and all snapshot forms.

Name-only records anywhere in an enum payload tree remain rejected. The old runtime
record builder stores a name and shape; a matching name does not establish record
ownership. [Checked record construction](runtime-record-owners-v2.md) now seals
owner IDs and admits those records both as leaves and transitively within enum
payloads. Nested collections and compiler/layout/IR/VM/native collection integration
remain open.

Work charges one per shape/subtree node and one per queued array or payload edge,
before extending the iterative work queue. The enclosing leaf check charges the
enum root once; the payload scan reuses that charge. Equality preflights complete
node/text comparison work before the existing iterative enum/array comparison.
Record-field searches additionally charge each candidate label comparison; see
[record equality work](runtime-record-owners-v2.md#costs-and-refusal). Recursive enum payload depth is bounded
by construction/materialization and finite traversal work, with flat copying/drop.

Additional tests cover foreign and cloned environment identities, same-context
distinct enums, exact label/payload event boundaries, nominal arrays (including
empty arrays), nested enums, transitive record refusal, payload queue-work refusal
and a 2,048-link recursive enum snapshot/equality/drop.
