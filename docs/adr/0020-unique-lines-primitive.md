# ADR 0020: Exact line-uniqueness text primitive

Status: Accepted and implemented (2026-10-09).

## Problem

The v1 Task Manager validates unique task IDs and titles when it loads a
versioned text file. Its current Sovra implementation compares every decoded
row with all earlier rows. On Windows, both execution engines exceed ten
seconds when listing 1,000 tasks. A temporary diagnostic copy that omits both
uniqueness checks handles 1,000 tasks in under a second and 5,000 in a few
seconds, but it permits malformed state and cannot be shipped.

The language has `std::split_once` and `std::parse_int`, but no collection or
text-set operation for checking uniqueness efficiently. Do not introduce
Task Manager-specific compiler behavior or weaken corruption detection.

## Recommended contract

Add pure `std::lines_unique(text: String) -> Bool`. It treats `text` as a
sequence of exact LF-terminated UTF-8 lines. A final LF terminates the last
line and does not add another empty line; an unterminated final line counts
as one line. An empty string has zero lines. CR is an ordinary character.
Two lines are equal only when their Unicode scalar sequences are identical;
there is no normalization, case folding or trimming. Empty lines participate
in duplicate detection. Both engines return true exactly when every line is
distinct.

The implementation scans once using a Rust `HashSet<&str>` in the interpreter
and a JavaScript `Set<string>` in generated code. It does not persist a set,
expose shared references, mutate the input or change array/record value
semantics. Interpreter and backend limits remain bounded by the input String
and host memory, as with other pure text operations.

The Task Manager will gather decoded IDs and titles into two LF-separated
strings, call this primitive once for each after validating individual rows,
and reject duplicate data exactly as before. The existing file format,
commands, error text and nominal type behavior stay intact. The primitive
itself is general-purpose for any line-oriented data.

## Alternatives

- General `Set<T>` collections. This is more flexible but requires new
  generic type, equality, construction and value-semantics contracts.
- Permit duplicate titles or defer corruption checks until a specific title
  is used. This weakens the current application contract.
- Keep quadratic checking and document a low capacity. This conflicts with
  the practical-capacity v1 gate.

## Acceptance

Cover empty, trailing-LF, empty-line, CR, Unicode and duplicate cases at
source, semantic, interpreter and generated-JavaScript levels. Benchmark the
unchanged Task Manager format at 100, 1,000 and 5,000 tasks in both engines;
assert malformed duplicate-ID/title files are still rejected. Update the
specification, text-decoding lesson, stdlib registry, dogfood status and log.

## Compatibility boundary

AGENTS.md requires review before compatibility decisions. This adds a public
`std::` callable, so existing user declarations with the same exact name
become builtin collisions under ADR 0003. The user approved the proposed
contract by directing implementation in the following turn.
