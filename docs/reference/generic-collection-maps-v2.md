# Maps of checked generic package values

`GenericCollectionMap` is explicit Rust collection storage consuming
[proof-carrying generic snapshots](generic-collection-values-v2.md). It retains
actual generic package declaration owners and complete concrete type arguments.
It does not convert records/enums into the separate local `RuntimeValueV2` model
or infer owners from names, shapes or raw layout numbers.

This API is not source-level `std::Map` admission. Compiler type/layout/IR and VM
collection instructions, language result wrappers and native feature qualification
remain open. Existing default commands and native feature words are unchanged.

## Types and operations

Create an empty map with an Int/Bool/String `RuntimeKeyType`, a retained
`GenericCollectionType` value token and explicit finite
`GenericTypeEquivalenceLimits`. Construction checks that the type profile can
admit the declared type before materializing one map node. The token can outlive
the value from which it originated; no default/fabricated value is stored.

Insertion checks exact semantic agreement between the input snapshot and the
retained value type. Aliases agree; foreign declaration owners, different nominal
type arguments (including phantom arguments) and wrong key domains refuse.
Every stored value retains its own original layout proof, rather than interpreting
its local indexes in the declared token's table. The declared type and finite
comparison profile survive every update, including removal of the final entry.

The API provides new, len/is_empty/key_type/value_type metadata, contains/get,
insert/remove, snapshot, keys/values/entries and equal. Storage is private and has
no unmetered Clone or unchecked entry-ingress API. Returned entry fields are
independent owned snapshots and cannot mutate or forge internal entries.

Replacement retains position. Removal followed by insertion appends. Missing
get/remove return Rust None. Successful removal returns an updated map and copied
removed value. Association equality ignores insertion order, but requires matching
key/value domains. It checks declared types once under the stricter intersection
of both maps' comparison profiles, then compares compiler-sealed value fields in
declaration order. Rust bool/Option/Vec/entry/removal carriers do not introduce
hidden language wrappers. String keys compare exact Unicode contents without
normalization. Float and compound keys remain excluded.

## Atomic result costs

Let N(v)/T(v) be a snapshot's complete logical node/text footprint, including
String contents, record type/field labels, enum type/variant labels and every
payload descendant. Let T(k) be a String key's byte length, otherwise zero.

| Event | Logical nodes | Owned text |
| --- | --- | --- |
| Empty map | 1 | 0 |
| Map snapshot or insertion result | 1 + sum over result entries of (entry 1 + key 1 + N(value)) | Sum over result entries of (T(key) + T(value)) |
| Present removal | Updated map cost + N(removed value) | Updated map text + T(removed value) |
| Present get | N(value) | T(value) |
| Keys array snapshot | 1 + key count | Sum T(key) |
| Values array snapshot | 1 + sum N(value) | Sum T(value) |
| Anonymous entries array snapshot | Same as complete map snapshot | Same as complete map snapshot |
| Missing get/remove; metadata; contains/equal | 0 | 0 |

Whole-result node/text preflight and copy-work admission precede any entry/key/value
copy. Updated map and removed value are admitted together. Replacement counts the
replacement subtree rather than discarded old descendants. Input maps remain
unchanged on success and refusal; dropping outputs never refunds fuel. Checked
arithmetic and the shared 4,294,967,295-key ceiling precede result allocation.
Future language wrappers and newly owned nominal entry labels require their own
admission; these anonymous Rust-entry snapshots do not charge absent labels.

## Finite work

Maps share `RuntimeSetWork` with the snapshot bridge and primitive sets. Key
comparison costs `1 + stored String bytes + input String bytes`. Planning costs
one per visited input entry, including an entry selected for removal. No result
value is traversed merely to recompute an already sealed cached footprint.

After whole-result materialization preflight, copy work is
`result nodes + result text bytes + sum N(each copied generic value)`.
Thus each copied generic subtree keeps its existing `2N + T` bridge copy units;
container/entry/key overhead contributes its nodes and key bytes. Removal includes
the removed value's nodes in this additional sum as well.

Equality costs one for length comparison, ordinary key lookup work and each
matched pair's `N(left) + N(right) + T(left) + T(right)` comparison work. An exact
matching declared type, maintained at insertion, authorizes linear positional
record pairing without repeated field-name or root-type searches per entry.

Semantic type comparison has its own explicit finite profile, independently of
the shared value-work counter. Type exhaustion is an error, never a guessed match
or false result. Ordered scans yield linear lookup and quadratic association
matching. No elapsed CPU, allocator memory, instruction or cooperative-cancellation
isolation is claimed for this explicit library API.

## Qualification and remaining work

Tests use real captured multi-file Sovra package outputs and cover imported owners,
root/dependency homonyms, alias agreement, phantom argument refusal, fresh-check
owner refusal, exact atomic node/text/work boundaries, source deletion, independent
outputs, Unicode/Bool keys, enum/array values, missing keys and insertion order.
All 1,296 four-operation histories over two keys and two values match an ordered
association oracle, including reversed-order logical equality.

Next: compiler collection types/layouts, intrinsic IR/VM operations with shared
execution controls, actual language result admission and versioned native/backend
qualification. C1 and G13 remain open.

[Checked collection layout plans](generic-collection-layouts-v2.md) can now bind
the key/value type arguments and instantiate this storage through a sealed Map
constructor. Source/executable-table/IR/VM publication remains open.
