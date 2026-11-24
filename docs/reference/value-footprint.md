# Logical value footprint measurement

Status: **Implemented read-only Rust API; separate opt-in logical fuel is available.**
`Value::logical_footprint()` observes an existing runtime value without cloning
payloads, building its complete display string or changing execution policy.
It returns `Option<ValueFootprint>`; overflow returns None without partial data.

```rust
use sovra::compiler::interpreter::{Value, ValueFootprint};
let value = Value::Array(vec![Value::String("λ".into()), Value::String("😀".into())]);
assert_eq!(value.logical_footprint(), Some(ValueFootprint {
    value_nodes: 3,
    owned_text_bytes: 6,
    display_bytes: 10,
}));
value.drop_iterative();
```

The independent u64 measurements are:

| Field | Meaning |
| --- | --- |
| `value_nodes` | One per scalar, Unit, array or record, including descendants. |
| `owned_text_bytes` | UTF-8 String payload bytes plus record type/field-label bytes. |
| `display_bytes` | UTF-8 bytes of the existing runtime display, including delimiters/separators but excluding any printed LF. |

A Packet with `items: ["λ", "😀"]` and `ok: true` measures five nodes, 19 owned
text bytes and 38 display bytes. Display size is not owned String storage size.
Unit measures one node and zero display bytes; an empty array measures one node
and two display bytes. Empty records retain their existing two interior spaces.
Labels are counted as stored, including canonical compiler-owned record names.

`ValueFootprint::checked_add` sums independent measured roots without adding an
enclosing container or separators. Any component overflow rejects the aggregate.
This can describe a collection of snapshots; it does not decide which future
runtime events consume fuel or whether release credits should exist.

Traversal uses borrowed iterator frames with scratch space proportional to depth,
not a queue proportional to a wide container. It copies no String/record payloads
and builds no compound display string. Small scalar number-formatting temporaries
and work-list allocation remain. Tests cover 50,000-element width, 16,000-level
arrays/records, checked aggregation, Unicode labels/payload accounting and display
byte agreement with actual Rust/Node execution. Value clone/drop behavior stays
under the existing [runtime value contract](runtime-values.md).

These numbers are logical observations, not allocator capacity, pointers, padding,
GC storage, process RSS, peak live memory or an enforced limit. Default execution,
CLI and native policy are unchanged. The separate [materialization policy](materialization-limits.md)
uses these measurements for approved cumulative admission; calling this observer
alone does not enable limits.
