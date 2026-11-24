# Bounded generic type syntax in v2

The approved G13 E2 grammar now has a source-owned Rust parser:

```rust
use sovra::compiler::v2::{parse_generic_type, GenericTypeLimits};
let syntax = parse_generic_type(
    "std::Result<[pkg::Entry<Int, String>], std::Error>",
    GenericTypeLimits::DEFAULT,
)?;
```

This API parses exactly one annotation expression: a qualified name, a nonempty
explicit type-argument list (`Name<T,E>`) or an array (`[T]`). Applications and
arrays may nest. Whitespace/comments preserve original source coordinates; names
retain canonical `::` separators without trivia. Empty arguments, missing commas,
unbalanced delimiters, trailing input, function declarations, bounds and explicit
call spelling `name::<T>` refuse. The latter belongs to a future call parser,
not the annotation grammar.

`GenericTypeSyntax` retains the original input and an immutable postorder arena.
`GenericTypeKind::Named` exposes the written path and ordered argument indices;
`Array` exposes its element index. Every child precedes its parent; the last node
is the root. Node spans refer to original UTF-8 byte offsets with original line
and column information. The flat arena avoids recursive ownership/drop of nested
types. Indices are syntax-local positions, not checked nominal identities.

## Admission

The default profile admits 64 KiB of original input, 16,384 tokens (including EOF),
4,096 nodes, 4,096 structural edges, 32 KiB copied qualified-name bytes and depth
128. The depth ceiling is fixed at 128 even for explicit callers. Zero allowances
refuse when the corresponding resource is needed; a bare name requires no edge.

The existing bounded lexer runs before syntax construction. Parent-node admission
counts pending ancestors before descent; each edge/name copy is admitted before
retaining it. Node and width/depth limits therefore remain independent. All
arithmetic uses checked growth. Malformed or exhausted input returns no partial
successful syntax. Limits are logical admission, not process CPU/RSS isolation.

## Remaining E2 work

This is syntax preparation, not semantic checking or generic execution. Unknown
names and incorrect generic arities may parse successfully. No substitution,
specialization, bounds, type inference, generic function/record/enum/alias
checking, Option/Result or propagation is claimed. Existing source checkers and
CLI paths still refuse unsupported generic programs. They never accept this
syntax as checked evidence or fall back to unchecked execution.

See the approved [G13 completion contract](../design/G13_COMPLETION_CONTRACT.md).
[Parameter-list ownership](generic-parameters-v2.md) and bounded syntactic
parameter-use binding are now available. The next E2 work is declaration
attachment, resolved type-application arity and bounded symbolic/concrete
substitution before lowering. E1 package/
backend qualification and the rest of G13 remain open.
