# Sealed record ownership for runtime collections

This explicit Rust library extension lets [opaque runtime maps](runtime-maps-v2.md)
admit checked record leaves and checked records within enum/array payload trees.
It does not introduce source collection types, generic package declaration IDs,
IR/VM collection instructions, or native image features.

`CheckedEnumTypes::find_record` resolves a canonical record header to an opaque
`RuntimeRecordId`. The identity includes the immutable checked environment and a
stable ordinal in its already retained immutable record inventory. Clones preserve identity;
separately checked environments remain distinct even with identical source.
Unknown names and enum declarations cannot produce record IDs. No second name
inventory is allocated or retained; existing checker metadata costs stay unchanged.
Inventory ordering is local to the checked environment, not a cross-check or
serialization identity.

`RuntimeValueV2::checked_record(env, id, fields, fuel, work)` is the only new path
to sealing an owned runtime record. It validates ID custody, complete unique field
coverage, exact primitive/array types and exact nested enum/record owners. It
performs no implicit numeric widening. A transitive scan rejects unsealed records
anywhere under field arrays or enum payloads. Existing checked child values
transfer into the result, with flat indexes rebased without recursive destruction.
The Rust constructor consumes supplied children, including on refusal; their
previous materialization charges are never refunded.

The existing `record` and legacy import builders continue to create name-only
records, with no owner. Such records retain their legacy mutual structural
comparison and display, but cannot forge a collection record leaf or nested owned
field. `record_owner()` exposes optional root owner metadata. Snapshots, array
operations and field extraction retain sealed owners. Owned record equality
requires matching owner IDs and field associations, independent of field order;
owned versus name-only records compare unequal. The legacy public Value enum is
unchanged.

Existing enum construction keeps accepting legacy name-only record payloads for
its established APIs. When it receives a sealed record, its owner must match the
expected checked environment. Collection ingress rejects any remaining name-only
record transitively, while admitting sealed record payloads.

## Costs and refusal

A checked record constructor charges one new wrapper node plus all newly owned
type/field-label UTF-8 bytes. Already owned children transfer without another
materialization charge. It validates everything before atomically admitting the
whole wrapper and labels, then allocates result labels. Checked aggregate flat
node sizes precede admission and merging.

Collection snapshots charge the entire record wrapper, labels and descendants
again. Maps retain their [complete-result cost table](runtime-maps-v2.md#atomic-ownership-events):
insert/get/remove/entries/values and map snapshots all include copied ownership;
removal admits both the updated map and removed value together. No model marker,
legacy materialization event, key constraint or default command policy changes.

Shared finite helper work charges field admission `1 + label bytes`, one per type
node and queued array edge, and transitive ownership scans. Nominal scans charge
one per visited node, each array/enum edge, and `1 + field-label bytes` before
queuing a record field. Construction additionally preflights
`all transferred nodes + wrapper node + owned wrapper/label bytes` for merging
and output creation. Checked schema lookups use already bounded immutable checker
metadata; these logical units are not elapsed CPU or physical heap guarantees.

Map equality preserves its existing linear node/text preflight. It additionally
charges `1 + left-label bytes + right-label bytes` before **each** candidate field
name comparison in a record association search. A finite allowance can therefore
refuse during a quadratic field lookup without fabricating an equality decision.
The old unbudgeted Rust structural-comparison API keeps its established role.

## Qualification and remaining work

Tests cover same/foreign owner environments, cloned evidence, unknown IDs/names,
legacy-name forgery, missing/duplicate/wrong fields, exact typing without widening,
nested record arrays, sealed/unsealed enum record payloads, owner retention through
snapshots/extraction, full copy costs, atomic fuel refusal and exact field-search
work boundaries. Existing enum, runtime, module and value tests provide compatibility
coverage.

This is local checked-record evidence rather than generic package owner integration.
Source collection types, generic layout/IR/VM admission, package-owner adaptation,
language result wrappers and versioned native/backend qualification remain open.
C1 and G13 remain incomplete.

The [generic package-owner value bridge](generic-collection-values-v2.md) now
retains actual generic layout proofs in a separate snapshot API. It does not
reinterpret these local runtime record IDs as generic package declarations.
