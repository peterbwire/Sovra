# Inline modules and function validation

A module groups functions in one source file. Mark a function `export` and call
it as `module::function`. Run the checked-in example:

```text
cargo run -- run examples/modules/main.svr
```

It prints `42`. In [the example](../../examples/modules/main.svr), `math::double`
calls `math::add`. The qualified spelling is required even inside `math`.
Top-level functions remain callable by bare name, and builtins use `std::`.
This is inline grouping, not cross-file imports.

Function names must be unique within a module, including private/exported
combinations. A repeated name produces E3008 at the repeated declaration.
Exported names must also avoid exact builtin collisions: exporting `std::len`
produces E3016, while `std::extra` or `other::len` are allowed. Private module
functions remain uncallable and do not enter this builtin collision check.

Every function body is checked, including unused and non-exported module
functions. For example, this is invalid:

```svr
mod math {
    export fn answer() -> Int {
        return missing
    }
}
fn main() {}
```

`svr check`, `svr run`, and `svr build` reject this source with `E3001` for the
undefined variable. Wrong return/local types, invalid calls and operators, and
duplicate parameter names are also checked. Parameters and locals belong only
to their function.

A function annotated with a non-Unit return type needs an explicit `return`.
For example, `fn answer() -> Int { 42 }` fails with `E3013`; write
`fn answer() -> Int { return 42 }`. Unit functions can finish without `return`.

Only top-level `main` is the entry point and must take no parameters and return
`Unit`. An exported `math::main(value: Int) -> Int` is an ordinary function.
Non-exported module functions are currently validated but not callable or
emitted into IR. Private helpers and implicit module-local lookup require
future work. Project-directory checks use a separate, partial application
scanner; they do not provide these source-file semantic guarantees.
