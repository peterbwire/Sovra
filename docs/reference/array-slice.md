# Snapshot array slicing

Status: **Implemented in checked/legacy compilation, Rust and Node runtimes.**
The [reviewed contract](../design/G13_ARRAY_SLICE.md) adds
`std::array_slice(values, start, end)`, returning the same array element type.
This is a compiler-known dependent result, not source-level generic declarations.
Aliases, nested arrays and nominal record/package identities remain intact.

```svr
fn main() {
    let original = [1, 2, 3];
    let mut selected = std::array_slice(original, 1, 3);
    selected[0] = 9;
    print(original); // [1, 2, 3]
    print(selected); // [9, 3]
}
```

Bounds are half-open element indices: require `0 <= start <= end <= length`.
Equal bounds, including length..length, return a typed empty array. Full bounds
return a full independent snapshot. Negative/reversed/out-of-range bounds fail;
there is no clamping, negative-offset interpretation or partial result.
Both indices must be Int. Strings and Float indices are rejected by the checker.
Runtime malformed-IR type errors use
`std::array_slice expects Array, Int, Int arguments`; range errors use
`array slice bounds out of range`. Bounds are validated before result admission.
JS checks BigInt bounds before Number conversion; Rust host-size conversion is checked.

The result copies selected nested children under existing value semantics.
Existing argument evaluation can copy the complete input first; this operation
is not zero-copy. Typed empty inputs and existing empty-literal inference rules
remain. Int-to-Float array assignments/returns retain existing widening behavior.
The inferred result is retained in checked facts/IDE output rather than exposing
the registry's dependent-result descriptor.

With [materialization limits](materialization-limits.md), charge one result-array
node plus complete node/owned-text footprints of selected descendants. Nested
String payloads and record labels count; unselected payloads are not copied or
charged again for the result. Empty results cost one node and zero text. Premeasure
checked sums and admit node/text atomically before constructing the output.
Iterative copies and runtime-owned cleanup handle deep values. Existing dispatch
fuel/control order remains; the slice helper is not preempted midway.

The design's original `[1,2,3]` slice-and-print main requires exactly 18 logical
nodes, 6 text bytes and independently 7 output bytes. Tests verify one-under
limits, prior output, malformed/type/range priority and Unicode payload costs.
These are logical cumulative units, not heap/RSS measurements.

The [independent registered package](../../examples/array-slice-packages/README.md)
exports reusable tail/record operations. Tests compare legacy/checked widening,
Rust/ordinary Node/controlled Worker results and errors. Isolated 16,000-level
array/record cases cover snapshot admission/refusal; worker copies run with a
0.5 MiB stack. Locked formatter/check/test/run and actual Linux native output
are qualified separately; no other-platform/browser/mobile support is inferred.
G13 remains partial: Map/Set, generic abstractions, structured errors, binary/JSON,
filesystem/time/process APIs and broader host policies remain open.
