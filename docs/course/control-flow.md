# Control flow and Boolean logic

Status: Implemented for the current executable subset. `if`, `while`, and
short-circuit Boolean operators are checked and execute in the interpreter and
JavaScript backend.

Conditions and logical operands must be `Bool`:

```svr
fn main() {
    let ready = true
    let authorized = false

    if (ready && authorized) {
        print("allow")
    } else {
        print("deny")
    }

    let mut attempts = 0
    while (attempts < 2) {
        print(attempts)
        attempts = attempts + 1
    }
}
```

`&&` and `||` short-circuit. For `left && right`, `right` is evaluated only
when `left` is true. For `left || right`, `right` is evaluated only when
`left` is false. Both operands are still checked for valid types even if a
particular execution will skip the right operand.

Precedence, from tightest to loosest, is multiplication/division, addition/
subtraction, comparisons, `&&`, and `||`. Operators at the same level associate
left-to-right. Parentheses can make grouping explicit:

```svr
let result = first || second && third
let grouped = (first || second) && third
```

Every condition in `if` and `while` must be Boolean; using an integer or string
is rejected with `E3005`. A non-Unit function must return on every possible
path. An `if` with an `else` satisfies that requirement only if both branches
return; a loop is not assumed to run.

Bindings declared inside a branch or loop body stay local to that block.
Shadowing an outer binding does not overwrite it, while assigning to an outer
mutable variable still updates that variable:

```svr
fn main() {
    let value = 1
    if (true) {
        let value = 2
        print(value)
    }
    print(value)
}
```

This prints `2` and then `1`.

Source files go through parsing and semantic analysis for `check`, `run`, and
`build`. Project-directory checking is a separate wiring scan and does not
validate executable function bodies.
