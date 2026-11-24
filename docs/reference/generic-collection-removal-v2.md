# Typed collection removal (explicit v2 profile)

`compile_generic_collection_package` and `execute_generic_collection_package`
now admit immutable removal from real Sovra source:

| Operation | Result |
| --- | --- |
| `std::map_remove(map, key)` | `std::Option<std::Map<K,V>>` |
| `std::set_remove(set, key)` | `std::Option<std::Set<K>>` |

A present key produces `Some(updated collection)`; a missing key produces a
nullary `None`. Removing the only key produces `Some(empty collection)`, which is
distinct from a miss. The original remains a complete independent snapshot.
Survivors retain their insertion order. Reinserting a removed key appends it at
the end. Strings compare exact UTF-8 contents without Unicode normalization.

The receiver fixes the exact key and complete result collection type. Wrong
constructors, key types, result type arguments, arity and explicit builtin type
arguments refuse. The compiler derives the sealed standard Option from the
checked receiver under finite generated-node/edge/work/depth budgets. Concrete
body and typed IR checks independently verify that Option's argument is exactly
the receiver type. Matching and generic `?` propagation work normally.

These source builtins return the updated collection, without a separate removed
value. Call `std::map_get` before removal when both are needed. The separate Rust
storage API and reference package retain their existing richer removal results;
this adds source builtins without changing those APIs.

## Full-result admission and complexity

Operand evaluation keeps its existing snapshot costs. Searches scan keys in
insertion order and stop at the first match. Each Int/Bool comparison debits one
shared VM work unit. Each String comparison debits one plus both UTF-8 byte
lengths before comparing. This is linear search, not a constant-time or CPU
isolation guarantee.

For a hit, `N` is one Option wrapper plus one updated collection container and
all surviving descendants. A surviving map entry contributes its private entry
node and every key/value descendant. A surviving set entry contributes its full
key. `T` includes all surviving owned text/type/variant/field labels and the
Option labels. Removed entries contribute no new result ownership. There are no
refunds for earlier receiver/argument snapshots.

The VM preflights the whole result's `N` nodes and `T` bytes together, charges
`2*N + T` copy work, and admits the complete result before allocating/copying it.
Descendants copied into the container and then attached to Option are admitted
once. Internal map entry nodes remain private. On a miss, the result costs just
one `None` node and 15 bytes for `std::Option` plus `None`; no updated collection
or default payload is fabricated. On a hit, Option's labels also cost 15 bytes.
Shared work/materialization, cancellation and elapsed-time limits remain active.
Exhaustion refuses execution instead of returning a false miss or partial result.

The [executable source example](../examples/intrinsic-collection-removal.svr)
is qualified through the explicit Rust package API. Tests cover survivor order,
old snapshot preservation, reinsert order, nested collection and imported nominal
values, full-result boundaries, generic propagation and all 64 three-operation
insert/remove histories over two keys against an ordered reference model.

## Remaining gates

Ordinary compiler/CLI execution and native encoding still refuse the collection
profile. CLI/native qualification remains pending. C1 and G13 remain open.

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
