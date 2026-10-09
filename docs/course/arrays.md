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
