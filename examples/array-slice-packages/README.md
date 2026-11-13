# Reusable snapshot slicing package

Status: **Executable registered local library and independent consumer.** Requires
this compiler's G13 `std::array_slice` support; earlier compilers do not provide it.

`sequences/` exports alias-typed `tail([Int]) -> [Int]` and a sliced nominal
`Sample` array. `app/` imports them through its local manifest dependency.
The library deliberately returns an empty tail for empty input; builtin slice
bounds remain strict. The consumer demonstrates independent snapshots, return
widening to Float arrays and a package-owned record result.

```sh
svr fmt --package --check examples/array-slice-packages/app
svr lock examples/array-slice-packages/app
svr check --executable-package --locked examples/array-slice-packages/app
svr test --locked examples/array-slice-packages/app
svr run --locked examples/array-slice-packages/app
svr build --locked --emit exe --output slice-app examples/array-slice-packages/app
```

Expected main output:

```text
[2, 3, 4]
[9, 4]
1.5
[]
9
```

Native building requires Rust/Cargo and a host linker on the build machine.
The resulting executable does not require Node, Cargo or svr at runtime.
JavaScript emission uses Node; a separate worker generator is an embedding API,
not a new CLI mode. Registry distribution, mobile/browser hosts and other-platform
qualification are outside this local example. See the [slice contract](../../docs/reference/array-slice.md).


The same reusable library also supplies an independent
[growth consumer](growth-app/README.md) for append-based filtering, typed
concatenation and nominal results. Existing slice consumer output is unchanged.
