# Arrays in application inspection

Status: Partial under `svr check --service-calls <project>`.

The opt-in M12 checker now understands array literals, indexing and equality in
inspected bodies. It infers a compatible element type, propagates that type
through local bindings, and uses `Int` indices:

```svr
fn main() {
    let mut values = [1, 2];
    values = [3, 4];
    values[0] = 5;
    let first = values[0];
    let same = values == [1, 2];
    if same { print(first); }
}
```

The checker also validates direct reassignment of a `let mut` local. It retains
the initializer's type, permits Int-to-Float widening (including nested arrays),
and reports E4140 for immutable, missing, unresolved or incompatible assignment
targets. A nested block can shadow a binding without changing the outer one.
Indexed assignment to a directly named mutable array also validates its Int
index and element type. E4141 identifies an immutable/non-array target, invalid
index, incompatible value or unsupported nested write. Runtime bounds checking
still happens only when the executable source runs.

`[1, "two"]` has incompatible elements (E4143), and `[1][false]` has an invalid
index (E4142). The checker points to those expressions even when passed directly
to an `Any` parameter; they never supply validated local type evidence. An
unresolved initializer may also report E4136. Bounds for a runtime `Int` index
are checked during executable program execution, not proved by static inspection.

JSON reports retain null in the legacy scalar type field for arrays. The additive
type descriptor uses `{"kind":"array","element":...}`, nesting descriptors
for nested arrays. An empty array's element descriptor is null. Array type
annotations and application runtime execution remain outside
this partial checker. Executable source files use the full compiler pipeline.
