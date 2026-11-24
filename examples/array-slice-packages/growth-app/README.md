# Independent array growth consumer

Status: **Executable registered local package.** Requires the current G13 append/
concat builtins. Its separate `../sequences` library exports keep_positive,
join_float and appended_sample. Filtering grows a typed empty snapshot through a
loop; concatenation records dependent Int-to-Float conversion; Sample retains
its package-owned nominal identity. This is reusable collection behavior, not an
app-specific compiler primitive or a registry distribution claim.

```sh
svr fmt --package --check examples/array-slice-packages/growth-app
svr lock examples/array-slice-packages/growth-app
svr check --executable-package --locked examples/array-slice-packages/growth-app
svr test --locked examples/array-slice-packages/growth-app
svr run --locked examples/array-slice-packages/growth-app
svr build --locked --emit exe --output growth-app examples/array-slice-packages/growth-app
```

Expected main output:

```text
[-2, 0, 3, 4]
[9, 4]
1.5
7
```

The first line confirms that changing the filtered result does not modify input.
Native building needs Rust/Cargo and a host linker; the compiled executable needs
neither Node, Cargo nor svr. Ordinary JavaScript output needs Node. See the
[growth reference](../../../docs/reference/array-growth.md) for type, length,
fuel and supported-host boundaries.
