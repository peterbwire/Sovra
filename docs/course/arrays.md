# Arrays and numeric assignments

Status: Implemented for the executable subset in both execution engines.

Array elements must have compatible types. A Float element promotes compatible
Int elements to Float. A mutable array retains that element type when assigning
a new array or replacing an element:

```svr
fn main() {
    let ints = [3, 5]
    let mut floats = [0.0]
    floats = ints
    print(floats[0] / 2) // 1.5
    print(ints[0] / 2)   // 1: conversion does not change ints
    floats = []
    floats = [7]
    print(floats[0] / 2) // 3.5

    let mut rows = [[1.0], [3]]
    rows[0] = [5]
    print(rows[0][0] / 2) // 2.5
}
```

Indexing requires an Int and checks bounds at runtime. Indexed assignment
requires a direct mutable binding. Replacing `rows[0]` is supported; nested
assignment such as `rows[0][0] = 5` is not currently supported. Float-to-Int
narrowing is not implicit. This does not introduce array annotation syntax.

An empty row does not erase the element type supplied by later compatible rows:

```svr
fn main() {
    let empty = []
    let mut rows = [empty, [1.0]]
    let ints = [5]
    rows[0] = ints
    print(rows[0][0] / 2) // 2.5: inferred Float row retains widening
    print(ints[0] / 2)   // 2: the original Int array is unchanged
}
```

After inference learns a row's element type, later incompatible rows are errors.
For example, `[[], [1], ["text"]]` reports E3007. Empty arrays remain valid, and
using the same empty binding in different array literals does not change its
value or establish a shared mutable element type between those literals.

Under approved [ADR 0012](../adr/0012-compound-value-semantics.md), arrays and
records are values. Reading or passing one creates an independent logical
snapshot. Array equality compares corresponding elements recursively, while
record equality also requires the same nominal record type. For example:

```svr
fn main() {
    let original = [1]
    let mut copy = original
    copy[0] = 2
    print(original[0])  // 1
    print(copy == [2])  // true
}
```


## Typed interfaces and length

Use `[T]` to declare an array element type in a parameter, return, alias, local or
record field. Nest brackets for arrays of arrays. An explicit expected type also
accepts an empty literal:

```svr
type Numbers = [Float];
fn copy(values: Numbers) -> Numbers { return values; }
fn main() {
    let original: Numbers = [1, 2];
    let empty: [[Int]] = [];
    print(copy(original)[0] / 2); // 0.5
    print(std::array_len(empty)); // 0
}
```

Int elements widen to Float at typed boundaries without changing a source Int
array. Element record types stay nominal and private records cannot leak through
exported array signatures/fields. `std::array_len` accepts arrays; `std::len`
continues to accept only String and counts UTF-8 bytes. Arrays do not gain append,
resize or shared-reference mutation. Unknown element names, heterogeneous values,
recursive aliases and type expansion beyond 128 levels are errors.

The [independent package consumer](../../examples/typed-array-packages/README.md)
uses exported array functions and record fields in both execution engines and
has Linux native smoke evidence. Registered-package host limits still apply.


Runtime cloning, runtime-owned cleanup and compound display now use iterative
traversal for deep arrays/records while preserving snapshot semantics. This does
not impose a memory, output or execution budget. Embedding Rust callers should
use `Value::drop_iterative` for their own deep values; see
[runtime value scope](../reference/runtime-values.md).

## Copy a range

`std::array_slice(values, start, end)` returns an independent array snapshot with
the input's element type. Bounds count elements and exclude end; require
`0 <= start <= end <= std::array_len(values)`. Equal bounds return an empty array.
Invalid bounds fail rather than clamp.

```svr
fn main() {
    let original = [2, 3, 4];
    let mut tail = std::array_slice(original, 1, 3);
    tail[0] = 9;
    print(original); // [2, 3, 4]
    print(tail); // [9, 4]
}
```

See the [slice reference](../reference/array-slice.md) and
[independent reusable package](../../examples/array-slice-packages/README.md).

## Build a larger snapshot

`std::array_append(values, item)` returns a new snapshot with one added item.
`std::array_concat(left, right)` returns a new snapshot containing both arrays.
The first argument determines the element type; existing numeric widening can
convert the second argument to it. Inputs are unchanged.

```svr
fn main() {
    let empty: [Int] = [];
    let first = std::array_append(empty, 3);
    print(std::array_concat(first, [4])); // [3, 4]
    print(empty); // []
}
```

An untyped empty first input is rejected. Concat([1.0], [2]) widens the second
array; concat([1], [2.5]) fails rather than changing the first array's type.
See [growth rules](../reference/array-growth.md) and the
[independent filtering consumer](../../examples/array-slice-packages/growth-app/README.md).
