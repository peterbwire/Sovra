# Strings and concatenation

Status: Implemented in the executable subset.

Adding two `String` values concatenates them and produces another `String`.
Local inference retains that type, so the result can be concatenated again,
passed to a String parameter, compared with another string, or used by
`std::len`. The length is measured in UTF-8 bytes.

```svr
fn main() {
    let text = "a" + "b"
    print(text + "c")
    print(std::len(text))
}
```

This prints `abc` and `2`. String concatenation does not implicitly convert
numbers: use `std::to_string` when a number should become text.

`let number: Int = "a" + "b"` is rejected with E3002. Returning that string
from an Int function also produces E3002; passing it to an Int parameter
produces E3007.

Run `cargo run -- run examples/strings/main.svr` for an example covering
inferred locals, typed parameters/returns, length and equality. It prints
`abc!`, `2`, and `true`. Both the interpreter and generated JavaScript execute
this example in the regression suite.

Quoted source strings support `\n`, `\r`, `\t`, `\"` and `\\` escapes.
JavaScript output uses its own escaping to preserve decoded values, including
control characters and Unicode; emitted `\u0000` does not imply that Sovra
source accepts JavaScript Unicode escapes or `\0`.

String ordering compares Unicode scalar values lexicographically in both
engines. A shorter prefix comes first. This is not dictionary or locale-aware
ordering, and no Unicode normalization occurs: `"é"` and `"é"` are distinct.
