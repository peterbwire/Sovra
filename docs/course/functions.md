# Functions and local inference

Status: Implemented for the current executable subset, following approved
[ADR 0002](../adr/0002-function-typing.md). Run
`cargo run -- run examples/functions/main.svr` to print `5`.

Every function parameter needs an explicit type. A local `let` binding can
infer its type from its initializer:

```svr
fn length(value: String) -> Int {
    let count = std::len(value)
    return count
}

fn main() {
    let text = "Sovra"
    let count = length(text)
    print(count)
}
```

`value: String` tells callers what `length` accepts. `-> Int` states what it
returns. `std::len` returns the number of UTF-8 bytes in a string, so `count`
is inferred as `Int`. The inferred type of `text` is `String`. The bare `print`
call remains a compatibility alias for `std::print`.

These primitive types are available: `Unit`, `Bool`, `Int`, `Float`, `String`.
Explicit annotations preserve numeric widening: an Int argument can be passed
to a Float parameter. A value of another incompatible type is rejected; for
example, `length(5)` produces `E3007` because `length` expects a String.

## Missing annotations

This declaration is invalid even when nobody calls it:

```svr
fn length(value) -> Int {
    return std::len(value)
}
```

Source checking reports `E3014` at the name `value`:

```text
parameter `value` requires an explicit type annotation; write `value: Type`
```

Replace the placeholder `Type` with the intended type; here, write
`value: String`. The compiler does not infer function parameter types from
their bodies or callers. This rule also applies to unused parameters, exported
module functions and non-exported module functions. It is a compatibility
change for older Sovra code that omitted parameter annotations.

## Return types

Omitting a return annotation means `Unit`. It does not infer a return type.
For example, `fn main()` has a Unit return type and may end without `return`.
A function declared `-> Int` must use an explicit return statement; an
expression at the end of the body does not implicitly return its value.
Returning a value of an incompatible type produces `E3002`; a non-Unit function
that falls through produces `E3013` in the current straight-line grammar.

## Check the source file

```text
cargo run -- check examples/functions/main.svr
cargo run -- run examples/functions/main.svr
cargo run -- build --emit js examples/functions/main.svr
```

Source `check`, `run` and `build` all apply these semantic rules. Project
directory checking remains a manifest and wiring scan; its success does not
validate function annotations or bodies. User-defined type declarations,
named-type resolution and general parameter inference remain unfinished.

The interpreter and JavaScript backend limit simultaneously active user-function
calls to 256, counting `main`. Builtins do not add a frame. Recursive calls beyond
that boundary report a call-depth error; sequential calls release their frames
as they return.

Function values are not implemented. `let value = std::len` produces E3015;
`let value = std::len("hello")` calls the function and infers an Int result.
The same restriction applies to exported module functions used without a call.

Declaring a top-level `fn print` produces E3016 because `print` is the builtin
compatibility alias. Rename the user function; calling `print(value)` remains
supported. Exact exported builtin collisions follow the same rule under
[ADR 0003](../adr/0003-builtin-name-collisions.md).
