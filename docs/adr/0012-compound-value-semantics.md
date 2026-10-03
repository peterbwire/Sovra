# ADR 0012: Array and record value semantics

Status: Proposed; requires explicit approval before implementation.

## Release blocker

The interpreter clones compound values when loading a binding; generated
JavaScript currently loads an object reference. Consequently this accepted
program prints `1` in the interpreter and `2` in JavaScript:

```svr
fn main() {
    let original = [1]
    let mut copy = original
    copy[0] = 2
    print(original[0])
}
```

Backend parity cannot be claimed until the language owns one rule. This also
affects passing arrays through functions and nesting arrays inside records.

## Recommended option A: compound values

Arrays and records have value semantics. Binding, assignment, argument passing
and return transfer a logical value snapshot. Mutating a mutable array binding
does not change another binding or the caller's argument. `let mut` permits
replacement and supported indexed writes, not shared mutable ownership.
Implementations may optimize copies only when observable behavior is preserved.
No references, borrowing, field mutation or resource-handle semantics are added.

Array equality compares lengths and corresponding elements recursively. Record
equality requires the same nominal declaration and equal corresponding fields;
constructor field order does not affect equality. Equal layouts from different
packages do not make records compatible. Primitive equality retains the existing
numeric/Boolean/String/Unit rules, including existing Float behavior.

The interpreter already supplies independent compound snapshots. Adjust both
engines where required for equality and bring JavaScript copying into parity.
Document behavior and test copies, nested values, function boundaries, mutation
isolation, record constructor order and nominal identity in both engines.

## Alternative B: shared compound references

Choose shared identity and mutation for arrays/records and change the interpreter
to match. This changes existing interpreted behavior and needs additional rules
for aliasing, equality, immutable bindings and future concurrency before coding.

## Approval and limits

AGENTS.md requires: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." Choosing one engine's behavior as
the language rule is such a decision; production intent alone does not select it.
Recommend option A. This does not select a general ownership model for sockets,
files, asynchronous tasks or other future resources. Allocation/output limits
remain a separate design and release gate.
