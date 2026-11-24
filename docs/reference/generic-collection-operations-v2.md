# Initial executable collection operations

The explicit Rust package profile now compiles and executes this Sovra source:

```svr
fn main() -> Int {
    let map: std::Map<Int, String> = std::map_new();
    let set: std::Set<String> = std::set_new();
    return std::map_len(map) + std::set_len(set);
}
```

Use `compile_generic_collection_package` with guarded captured package inputs,
then `execute_generic_collection_package` with an explicit `GenericVmPolicy`.
The [complete example](../examples/intrinsic-empty-collections.svr) is qualified
through this library profile. **Ordinary CLI and native profiles do not enable
these operations yet.** This page describes the initial empty-value subset.
[Insertion/replacement and nonempty equality](generic-collection-insertion-v2.md)
now also execute. [Typed lookup](generic-collection-lookup-v2.md) returns standard
Options. The complete C1 standard library remains open.

## Checked source, IR and VM path

The collection package profile uses the existing guarded declarations, visibility,
[collection annotation checker](generic-collection-types-v2.md), exact specialization,
[opaque layout graphs](generic-collection-layout-graphs-v2.md) and source diagnostics.
Every original body is checked before selecting the reachable execution closure.

New creation calls require an expected collection result type from an annotation,
function return or typed constructor field. There are no fabricated generic defaults
and no inference of missing K/V arguments from an empty container. Explicit
builtin turbofish arguments are not admitted in this initial subset. Wrong arity,
wrong constructor, record impostors, unsupported key types and invalid length
operands refuse before successful IR publication.

| Operation | Result | Requirement |
| --- | --- | --- |
| `std::map_new()` | Empty Map<K,V> | Explicit expected Map type context |
| `std::set_new()` | Empty Set<K> | Explicit expected Set type context |
| `std::map_len(map)` | Int | Exact checked Map operand |
| `std::set_len(set)` | Int | Exact checked Set operand |

Dedicated concrete rechecking proves builtin constructor/operand types. IR
structure validation checks creation result shapes; typed stack validation checks
length operand layouts and Int results. The VM dispatches real collection builtin
instructions. Empty maps and sets have distinct private value kinds and preserve
their original checked layout IDs; they are not empty records, arrays or payload
placeholders. Their entire owned representation is currently one flat node, with
no entries or values.

Collections can be copied, stored in mutable local slots, passed through generic
and cross-file functions, embedded in arrays/records/enums and used with existing
Option/Result propagation. Equality of same-typed empty collections is supported.
Borrowed `GenericValueRef::map_len`/`set_len` inspect the actual empty value without
creating a language Int. Generic snapshot capture and equality preserve the
collection's type/layout proof.

## Explicit admission boundaries

The old body checker, layout planner, construction planner, IR lowerers and VM
executors retain their collection gates. New library entry points opt into the
qualified subset: `check_generic_collection_package_bodies`,
`plan_generic_collection_construction`, `lower_generic_collection_propagation_ir`
and `execute_generic_collection_ir` support the owned package compile/execute API.

IR produced by the collection profile carries `has_collection_operations()`,
including when collection annotations are unused by the selected root. It is an
admission marker, rather than merely an opcode count. Ordinary execution refuses
that IR before instruction dispatch. Ordinary package/CLI compilation still rejects
intrinsic collection annotations/calls. No implicit fallback to the legacy compiler,
empty arrays or name-only records is used.

Existing native encoding explicitly refuses collection-profile programs with
`native collection feature is not enabled`, even for Unit main and unused
annotations. Old feature words cannot encode the new source/IR profile. Adding
versioned native support requires separate qualification; this change does not
weaken the old native decoder or introduce a new image word prematurely.

## Logical costs and shared controls

Creating an empty Map or Set charges one new container node and zero text bytes.
It materializes no keys, default V values, empty String placeholders or type-argument
payloads. A length operation charges one Int result node. Passing a local collection
into a length query first pays the existing independent snapshot-read cost: one
node for the current empty container. Thus creation, one local read and one length
result consume exactly three nodes and zero text bytes.

Each dispatched instruction consumes the existing instruction/work allowance and
cooperative checkpoint. Compound copies use the existing `2N + T` helper work and
whole-result materialization admission. All functions/frames share the same VM
instruction, value, work, slot, operand, frame-depth and cancellation/elapsed policy;
no separate unlimited collection budget is introduced. Refusal returns no success
value and no alternate backend. These logical units do not promise physical memory
or hard wall-clock isolation.

## Limits and remaining work

The initial empty subset has since expanded with
[typed insertion](generic-collection-insertion-v2.md), lookup, removal and
[typed key/value arrays](generic-collection-iteration-v2.md). [Public entry snapshots](generic-collection-entries-v2.md) now execute too. [Collection display](generic-collection-display-v2.md) now includes nested
values under the shared two-pass budget.
The richer [Rust map storage API](generic-collection-maps-v2.md) is separate and has
not been silently relabeled as completed source/VM functionality.

Tests cover actual source execution, strict contexts and operands, generic/imported
factories, copied collections in arrays/records/local slots, equality and propagation,
exact node budgets, instruction/work/cancellation refusal, profile/native gates and
the published source example. Existing native package execution remains qualified
on Linux with the new compiler code.

Next: consolidated result qualification,
followed by versioned native and CLI qualification. C1 and G13 remain incomplete.

## Typed membership

The explicit collection package profile also admits
`std::map_contains(map, key)` and `std::set_contains(set, key)`, returning `Bool`.
The receiver fixes the exact primitive key type; wrong constructors, key types,
arity and explicit builtin type arguments refuse during checking. Concrete bodies
and typed IR recheck the same operand/result contract.

Membership scans insertion-order keys and stops at the first match. Each Int or
Bool key comparison costs one shared VM work unit; each String comparison costs
one plus both UTF-8 byte lengths, charged before comparison. Unicode strings
compare exactly without normalization. Empty and missing keys return false.
The result admits one Bool node and no text bytes; membership makes no additional
payload snapshot, while ordinary operand evaluation retains its existing copy costs.
Cancellation, elapsed-time and cumulative work/materialization limits remain active.

Default compiler/CLI/native admission remains gated; C1 and G13
are open.

Typed value lookup is now available in the same explicit profile; see
[standard Option lookup and full-result costs](generic-collection-lookup-v2.md).

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
