# Checked data layout graphs

`plan_generic_data_layouts(literals, limits)` now discovers complete closed data
and collection layout graphs from the [checked data type profile](generic-data-types-v2.md).
The explicit data construction/IR/VM profile consumes these graphs; ordinary and
collection-only profiles retain their independent refusal gates. See the
[data/host workflow](data-host-v3.md) for CLI/native admission and JSON.

| Layout | Retained proof and edges |
| --- | --- |
| `Byte { declaration }` | Exact sealed standard Byte declaration; distinct from Int and user records |
| `Bytes { declaration, byte }` | Exact sealed standard Bytes declaration plus one edge to checked Byte |

Bytes is not `[Byte]`, a record, String or a container of Int. Buffer-only source
annotations still discover the exact Byte edge. Derivation uses the receiver's
checked environment and sealed Byte definition, retains provenance and applies
bounded generated-type admission. It does not guess a declaration from a name,
attach foreign proof or pretend that the caller wrote a Byte annotation. Missing
Byte schema refuses even if a user record is named Byte.

Canonical interning shares aliases and explicit/derived Byte types. Record,
array, enum/Option/Result and collection arguments preserve data edges, including
phantom nominal arguments. Recursive records/maps close through indexed edges
without recursive physical values or erased Byte-key types. No native ABI,
alignment, packed/flat representation or data opcode is selected by this API.

## Shared bounded admission

The existing cumulative instantiation, layout, member, graph-edge,
generated-node/edge, canonical-key node/edge, depth and work profiles remain in
force. Internal Bytes-to-Byte derivation charges copied proof arenas, generated
nodes/edges, schema-search/expansion work and its graph edge before publication.
Both generated type proofs and the final graph remain bounded. Exhaustion returns
no partially usable layout table.

`CheckedGenericLayouts::has_data()` reports actual discovered Byte/Bytes/JsonValue shapes,
including phantom arguments. Merely registering unused data headers does not set
the marker. Data-free graphs retain prior usage and VM behavior.

`GenericCollectionType` retains data declaration IDs and recognizes Byte's Key
proof while preserving exact nominal/type arguments. The data execution profile
uses a distinct Byte key domain; older storage bridges refuse data operations
rather than fabricating Int keys. Bytes and user Byte records remain nonkeys.

## Execution gates

The existing default and collection-only layout planners continue to reject data
shapes. The ordinary and collection-only construction planners reject any data-bearing table with
`GenericConstructionError::DataOperationsUnavailable`, including tables with
phantom data arguments or no data constructors. Ordinary IR and collection
propagation lowering retain the same independent gate. Metadata cannot authorize
unchecked byte values, operations or uncharged Result wrappers.

Existing compiler/CLI/native profiles still do not register data source types.
Words 511/1023, noncollection behavior and value-model-v1 events are unchanged.
Tests cover exact/refused counters, aliases, Byte/Int/array/record distinctions,
sealed buffer elements, recursive Byte-key maps, missing-schema refusal,
phantom/data construction/IR refusal and data-free compatibility.

Byte operations, source Result/error admission, runtime Byte keys, CLI/native data
profiles and strict JSON are implemented. The [consolidated qualification](../G13_QUALIFICATION.md)
remains the open G13 acceptance gate.
