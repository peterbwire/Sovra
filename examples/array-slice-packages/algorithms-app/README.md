# Independent array-algorithm consumer

Status: **Executable existing-language library example.** The separate registered
`../sequences` package exports reversed([Int])->[Int] and sorted([Int])->[Int].
The consumer exercises negative/duplicate values, original snapshots, empty and
single inputs, and already sorted/reverse-sorted sequences.

```sh
svr fmt --package --check examples/array-slice-packages/algorithms-app
svr lock examples/array-slice-packages/algorithms-app
svr check --executable-package --locked examples/array-slice-packages/algorithms-app
svr test --locked examples/array-slice-packages/algorithms-app
svr run --locked examples/array-slice-packages/algorithms-app
svr build --locked --emit exe --output array-tools examples/array-slice-packages/algorithms-app
```

Expected main output:

```text
[2, 0, 3, -1, 3]
[-1, 0, 2, 3, 3]
[3, -1, 3, 0, 2]
[]
[]
[7]
[1, 2, 3]
[1, 2, 3]
```

These are ordinary Sovra library functions, not new compiler primitives.
Reversal builds returned append snapshots; sorting uses insertion-sort swaps on
an independent local copy. Both can do quadratic work/copy traffic and are small
correctness/reuse examples, not performance claims. Existing host-selected runtime
fuel applies and exhaustion has no unlimited fallback. They do not provide generic
sorting, comparator callbacks, Map/Set or the rest of G13.

Native building requires Rust/Cargo and a host linker; the generated executable
needs neither Node, Cargo nor svr at runtime. JavaScript output uses Node. See the
[completion contract](../../../docs/design/G13_COMPLETION_CONTRACT.md) for remaining
G13 dependencies and the review boundary on new language/host semantics.
