# ADR 0016: Pure text decoding primitives

Status: Accepted and implemented (approved by user on 2026-10-09).

## Context

ADR 0015 gives programs bounded UTF-8 file reads and atomic replacement writes. A
program still cannot recover structured state from the resulting string: there is
no substring operation or fallible integer parser. The Task Manager dogfood app
therefore cannot safely reload saved tasks. These operations should be general
stdlib functions, with identical interpreter and JavaScript behavior.

## Proposal

Add two pure functions without changing language syntax or existing operations:

- `std::split_once(text: String, delimiter: String) -> std::SplitOnce`, whose
  nominal result has `found: Bool`, `before: String`, and `after: String`. It
  splits at the first exact occurrence. If absent, `found` is false, `before`
  is the original text, and `after` is empty. An empty delimiter matches at the
  beginning (`found` true, empty `before`, original `after`). Matching respects
  UTF-8 boundaries; the operation does not normalize Unicode.
- `std::parse_int(text: String) -> std::ParsedInt`, whose nominal result has
  `ok: Bool` and `value: Int`. It accepts only ASCII decimal text matching
  `-?(0|[1-9][0-9]*)` whose value fits signed 64-bit `Int`. Invalid text,
  whitespace, a plus sign, leading zeros, and overflow yield `ok: false` and
  `value: 0`. The grammar accepts `-0` as zero.

Both result records have public readable fields and follow existing nominal
record and value semantics. Calls may be made from source and dependency
packages, and both functions are available in interpreter and JS output.

The Task Manager may use these operations to define a versioned, strictly
validated file format in its own application code. This ADR does not prescribe
an application schema or add dynamic arrays, arbitrary slicing, or a general
serialization format. File size remains subject to ADR 0015's read bound.

## Validation required

Test valid and invalid calls, result-field typing, Unicode delimiters, empty
and absent delimiters, signed boundaries, overflow, malformed numbers, and
interpreter/JavaScript parity. Add a restart and malformed-file dogfood case
before claiming Task Manager persistence.
