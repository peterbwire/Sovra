# ADR 0018: Float text rendering across execution engines

Status: Accepted and implemented (approved by user on 2026-10-09).

## Context

`print` and `std::to_string` render Float through Rust `f64::to_string` in the
interpreter and JavaScript `String(number)` in generated code. Numeric values
can agree while visible output differs: `0.0 / (0.0 - 1.0)` prints `-0` in
Rust and `0` in JavaScript. JavaScript also chooses exponent notation at
different thresholds. The production audit leaves Float formatting policy
unresolved; the new runtime matrix excludes this undefined boundary.

## Recommended decision: canonical Rust-style decimal rendering

- Define Float display as Rust `f64` Display text for `print`, `std::to_string`,
  and nested array/record display in every supported execution engine. This
  preserves the existing Rust-facing output contract.
- Preserve the sign of negative zero (`-0`). Use the same spelling for
  non-finite values as the interpreter (`NaN`, `inf`, `-inf`) if arithmetic
  produces them. Render finite values as shortest round-trippable decimal text
  without exponent notation, matching Rust Display.
- Implement a JavaScript formatter and validate it against Rust across signed
  zeros, powers of ten, threshold cases, subnormals, large finite values,
  infinities and NaNs. Check scalar and nested display, including `to_string`.
  A verified mismatch blocks acceptance of the formatter; do not normalize
  numeric values to conceal it. The initial differential corpus covers 131
  finite literals plus signed zero and arithmetic non-finite values.
- This changes generated JavaScript's visible text for some existing programs.
  It does not change Float arithmetic, comparison, parsing, or source syntax.

Alternative: leave each backend's native formatting in place and document
backend-specific text output. That avoids a compatibility change but leaves
observable behavior divergent, contrary to the production parity target.
