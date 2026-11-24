# Snapshot collections reference package

This ordinary Sovra package provides ordered association-list maps and sets under
`--experimental-generics`. It is a usable reference implementation for G13 C1;
opaque intrinsic `std::Map`/`std::Set` and their cost contract remain unfinished.
See the [consumer example](../../examples/snapshot-collections/README.md) and the
[opaque runtime set foundation](../../docs/reference/runtime-sets-v2.md).

Keys accept `Int`, `Bool` and `String` through `std::Key`. Strings compare exactly,
without Unicode normalization. Values can be any admitted generic value.

| Module | Operations |
| --- | --- |
| `maps` | `map_new`, `map_len`, `map_contains`, `map_get`, `map_insert`, `map_remove`, `map_entries`, `map_keys`, `map_values`, `map_equal` |
| `sets` | `set_new`, `set_len`, `set_contains`, `set_get`, `set_insert`, `set_remove`, `set_values`, `set_equal` |

Supply explicit type arguments, for example `map_new::<String,Int>()` and
`set_new::<String>()`. Constructors return empty records directly. Other operations
return `std::Result<T,std::Error>` and support `?`. Get operations return an Option
inside the Result. Map removal returns `Option<Removal<K,V>>`, with `map` and
removed `value` fields; set removal returns `Option<SnapshotSet<K>>`. Missing keys
return None. Insertion returns a new collection snapshot.

Insertion order is deterministic. Replacement retains position; removal followed
by insertion moves the key to the end. Duplicate set insertion retains position.
Entries, keys and values are independent array snapshots. Explicit `map_equal`
compares associations and `set_equal` compares membership, independently of order.
**Raw record `==` remains structural and order-sensitive.**

Records have public fields. Every public consuming operation validates uniqueness
before returning a result, including equality on both inputs. Forged duplicate-key
records return `invalid_snapshot_map` or `invalid_snapshot_set`. Inserts check the
4,294,967,295 representation ceiling and return `collection_capacity`; practical
VM work and copy limits generally refuse much earlier.

This implementation scans arrays, validates duplicate keys pairwise, and uses
ordinary snapshot copies. Copying descendants consumes existing VM admission and
work budgets; loops and repeated field reads incur additional costs. No constant-time
lookup or intrinsic collection cost model is claimed. It is intended for bounded
workloads and semantic qualification before the intrinsic implementation.

Whole-package compilation permits at most 4096 explicit body type arguments in
aggregate. Standalone compilation retains 128, and the per-list limit remains 128.
