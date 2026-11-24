# Executable collection insertion

The explicit collection package profile now executes typed map insertion/replacement
and set insertion from real Sovra source. See the
[qualified source example](../examples/intrinsic-collection-insertion.svr).

```svr
let empty: std::Map<String, Int> = std::map_new();
let first = std::map_insert(empty, "a", 1);
let replaced = std::map_insert(first, "a", 2);
let set: std::Set<String> = std::set_new();
let member = std::set_insert(set, "a");
```

Use the existing explicit `compile_generic_collection_package` and
`execute_generic_collection_package` APIs. CLI/native collection admission remains
gated. [Typed source lookup](generic-collection-lookup-v2.md) and
[membership](generic-collection-operations-v2.md#typed-membership) now execute too.
Public entry snapshots now execute too; host borrowed views
described below are not those language operations.

## Checked arguments and snapshot semantics

`std::map_insert(map, key, value)` returns a new snapshot of the same exact Map<K,V>
type. `std::set_insert(set, key)` returns a new snapshot of the same Set<K> type.
The receiver determines K/V; its constructor, argument arity, key and value types
are checked symbolically, concretely and in the IR typed stack. Key/value operands
must have the exact declared types. The builtin adds no implicit numeric or
recursive container conversion. Ordinary expression conversions that already
produce the declared type retain their existing rules.

An initial collection must be typed, or the enclosing insertion expression must
supply an explicit result context for a nested creation call. No missing K/V type
or data default is invented. Generic and imported helper functions preserve their
registered key constraints and value declaration owners.

Map replacement retains the original key position; new keys append. Duplicate
set insertion retains the original member and position. Every result is independent
of the receiver and provided values. Mutating a source array/local binding later
cannot alter stored keys or values. Strings compare exact UTF-8 contents without
normalization, and Bool/Int keys remain distinct declared key domains.

## Flat representation and ownership costs

A map root stores insertion-ordered indexes of private entry nodes. Each entry
contains key/value child indexes, one logical node, no owned labels and the cached
complete subtree footprint. Entry wrappers are internal storage rather than
language records; no source record/enum constructor or public entry-wrapper value
can forge them. A set root stores insertion-ordered primitive key indexes.

All values remain flat postorder trees. Rebased copies update map, set, internal
entry, array, record and enum child indexes. Cached counts include real entry nodes,
so physical subtree intervals and logical node footprints remain consistent.
Recursive nominal values containing maps copy, compare and drop iteratively.

For a result map, nodes are `1 + sum(1 entry + N(key) + N(value))`; text is the sum
of copied key/value owned text, including every value descendant and nominal label.
For a result set, nodes are `1 + sum N(key)` with copied key text. Primitive keys
have one node. Replacement counts the replacement value, not discarded old value
descendants. Duplicate set insertion still pays the complete result copy cost.

Insertion premeasures the whole result using sealed cached child footprints, checks
representation/aggregate arithmetic and preflights complete node/text admission
before any output copy. Copy work is admitted next, followed by the complete
materialization charge and actual copying. Node/text/work refusal publishes no
partial result and does not mutate inputs. Cooperative cancellation can refuse
while copying; already admitted cumulative fuel is never refunded.

The shared 4,294,967,295 representation ceiling uses the existing checked length
helper. Runtime node, text, instruction and helper-work limits generally refuse
much earlier. No physical-memory or hard wall-clock isolation is claimed.

A first Int/Int insertion via a local empty map consumes eight nodes: empty
creation 1, receiver read 1, key/value evaluation 2, result map/entry/key/value 4.
A first String/String insertion with key `λ` and value `😀` has the same node count
and consumes twelve text bytes: six in argument evaluation and six in the result.

## Work and equality

Each candidate key comparison admits `1 + left String bytes + right String bytes`
before comparison. Planning charges one per visited input entry/member. The result
copy admits `2N + T` work before allocating output node/entry-index vectors and
uses existing cooperative checkpoints throughout flat copying. All work and
materialization use the same VM budget as surrounding instructions and functions.

Map equality compares key/value associations independently of insertion order; set
equality compares membership. Existing exact type checks and linear node/text
preflight remain. Every candidate key search additionally charges its comparison
work, bounding quadratic association matching. Matched value pairs enter the
existing iterative comparison queue, including nested maps/records/enums/arrays.
Different values or missing associations compare unequal. Compiler/VM and the
proof-carrying snapshot bridge use the same key equality rules.

`GenericValueRef::map_entry(index)` borrows a key/value pair without exposing its
internal wrapper; `set_item(index)` borrows a member. `map_len`/`set_len` now reflect
actual stored lengths. Snapshot selectors MapKey, MapValue and SetItem can copy
individual language subvalues with their original type proof and existing subtree
fuel. These explicit Rust inspection APIs do not add source get/iteration builtins.

## Qualification and remaining work

Tests cover actual insert/replace/duplicate/order semantics, exact node/text budgets,
unchanged inputs, nested mutable-array values, maps of sets, generic/imported
insertion, Unicode/Bool keys, wrong types/arity, shared work refusal, deep recursive
record values and the source example. All 64 three-insertion histories over two
keys and two values match an ordered oracle; reverse-order source equality is
checked for both maps and sets. Existing native execution remains a compatibility
check, while new-profile native encoding still explicitly refuses.

Next: consolidated result qualification,
then versioned native/CLI qualification. C1 and G13 remain open.

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
