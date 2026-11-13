# Snapshot array append and concatenation

Status: **Implemented in checked/legacy compilation, Rust and Node/Worker runtimes.**
The user-approved [growth contract](../design/G13_ARRAY_GROWTH.md) adds:

| Operation | Result |
| --- | --- |
| std::array_append(values, item) | Independent snapshot of values followed by item. |
| std::array_concat(left, right) | Independent snapshot of left followed by right. |

The first array fixes the result element type. Aliases, nested arrays and nominal
package records retain their identities. The second argument must match that type
(append: one element; concat: another array). Existing Int-to-Float widening is
allowed recursively on the second argument; no Float-to-Int narrowing or backward
common-type inference is introduced.

```svr
fn main() {
    let empty: [Int] = [];
    let values = std::array_append(empty, 3);
    let joined = std::array_concat(values, [4, 5]);
    print(empty); // []
    print(joined); // [3, 4, 5]
    let floats = std::array_concat([1.0], [3]);
    print(floats[1] / 2); // 1.5
}
```

An empty first argument needs a known type, such as the annotated binding above.
`std::array_append([], 1)` is rejected even if the result binding has an annotation.
Unknown nested leaves are also rejected. Existing empty inference is allowed on
the second side: concat([1], []) and append([[1]], []) work. Known Unit elements
are supported. Type/arity failures use established semantic diagnostics; dependent
registry descriptors are not published as inferred IDE/interface types.

The checked compiler records expected second-argument types/coercions and retains
the first-directed result. Checked and legacy lowering apply conversion before
the builtin; converted values are the ones copied into its output.

Runtime container validation precedes result length and fuel admission. Malformed
IR reports `std::array_append expects an array argument` or
`std::array_concat expects array arguments`. Those checks do not dynamically prove
homogeneous element types: raw IR lacks static T metadata, as for MakeArray.

New results may contain at most 4,294,967,295 elements. Checked length arithmetic
rejects excess or overflow with `array result length exceeds limit`, before result
fuel or allocation/copying. This is a shared representation ceiling for these two
operations, not a retroactive array/slice limit or a physical memory guarantee.
Actual allocation can fail much earlier; no recoverable allocator failure promise
is introduced. Boundary tests use length helpers without giant allocations.

With optional [logical fuel](materialization-limits.md), admit one new container
node plus complete child node/text footprints. Append includes all original
children and the converted item; concat includes children of both converted arrays
without charging input containers as output children. Nested String payloads and
record labels count. Empty concat costs one new node. Checked aggregation and
atomic admission precede iterative clones, which are not charged a second time.
Argument/parameter/LoadName copies remain separately charged; there are no refunds.
Scratch and physical sharing are outside the logical model.

A complete main storing append([1],2) then printing it requires 11 nodes, 6 text
bytes and independently 7 output bytes. Concat([1],[2]) requires 12 nodes with the
same text/output. Tests verify exact and one-under behavior, including output that
precedes failure at root completion. Widening itself consumes zero logical fuel.

[The independent growth consumer](../../examples/array-slice-packages/growth-app/README.md)
uses reusable filtering, numeric concatenation and nominal record results from a
separate registered library. Qualification compares Rust, ordinary Node and Node
workers, rejects incompatible/unknown types, retains inferred IDE facts and covers
Unicode/labels, 16,000-level copying/refusal and 50,000-element width. Worker deep
copies use a 0.5 MiB stack; isolated Rust children verify iterative cleanup.
Locked formatter/check/test/run and actual Linux native output are separate gates.

These operations leave inputs unchanged. Repeated append can copy repeatedly;
there is no amortized-growth, zero-copy, helper preemption, heap/RSS cap or process
sandbox guarantee. Existing source syntax, control defaults and CLI/native policy
remain unchanged. Other-platform/MSRV/release/browser/mobile support is not inferred.
G13's general generics, Map/Set, errors, binary/JSON and filesystem/time/process
work remains open.
