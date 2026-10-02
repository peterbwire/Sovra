# Records and named types

Status: Implemented for record construction, field reads, typed function
boundaries and display. Record field mutation is not supported.

Declare a struct with typed fields, then construct it by naming each field:

```svr
struct Point {
    x: Int,
    y: Int,
}

fn horizontal(point: Point) -> Int {
    return point.x
}

fn main() {
    let point = Point { x: 3, y: 4 }
    print(horizontal(point))
    print(point.y)
}
```

The checker validates that every declared field appears exactly once and that
each value has a compatible type. Field reads can be chained through nested
records. Records can be passed to and returned from functions. Type aliases
resolve to their declared targets, so a field or function annotated with an
alias still uses the underlying record type.

Records are values. This subset has no field-assignment syntax; a field cannot
be updated with `point.x = 5`. Arrays and `let mut` are separate language
features, with their assignment rules documented in the
[language specification](../spec.md).

Run the executable example with:

```text
cargo run -- check examples/records/main.svr
cargo run -- run examples/records/main.svr
cargo run -- build --emit js examples/records/main.svr
```

The interpreter and JavaScript backend use the same record display form, for
example `Point { x: 3, y: 4 }`.
