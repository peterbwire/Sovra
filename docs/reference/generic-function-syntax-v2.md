# Generic function header ownership in v2

```rust
use sovra::compiler::v2::{parse_generic_function, GenericFunctionLimits};
let syntax = parse_generic_function(
    "fn identity<T>(value:T)->T{return value;}",
    GenericFunctionLimits::DEFAULT,
)?;
```

The additive Rust API attaches one type-parameter owner to a complete root
`[export] fn name<T>(value:Type,...)->Return{body}` source range. The generic list
must be explicit and nonempty. Every value parameter requires an annotation;
duplicate value names refuse. Generic argument commas remain inside annotations,
and trailing value-parameter commas follow existing function syntax. An omitted
return annotation stays absent, preserving the existing implicit Unit convention
without fabricating a type node or reference.

The immutable result retains the full original source, function/name ranges,
export intent, parameter-list owner, source-order value parameters, optional return
annotation and original body range. Every annotation's parameter-use table shares
the function's owner. Other functions' parameter identities cannot match by
spelling. `parameter_span`, `value_type_node_span` and `return_type_node_span` map
fragment-local metadata into original full-source coordinates; foreign identities,
missing annotations and invalid indices refuse. Fragment errors are translated
into the original byte/line/column coordinates too.

## Explicitly opaque body

`body_source()` borrows the original body including braces without another copy.
The body is lexed and checked only for balanced/matching `{}`, `[]` and `()`
delimiters under token/depth limits. String/comment delimiters are handled by the
existing lexer. Its statements are **not parsed or semantically checked**. A body
with unknown names or invalid but delimiter-balanced statement syntax can be
retained. This API must never be treated as a checked function or passed directly
to lowering/execution. No inferred body types, call targets, control flow or
return-coverage facts are invented.

## Admission

Defaults admit 128 KiB original source, 32,768 original tokens including EOF,
256 KiB total retained full/fragment source, 48 KiB aggregate copied names,
128 value parameters, 4,096 annotation nodes, 4,096 type edges and 4,096 annotation
parameter uses. The independent parameter-list and per-annotation profiles remain
in force. Type-parameter/value/return fragments and names consume shared source
and name allowances; value and return annotations consume the same remaining
node/edge/use ledgers. Later annotations cannot reset these allowances.

Body admission independently permits 16,384 tokens including braces and delimiter
nesting 128 including the outer body. The fixed body-depth ceiling is 128 even
for explicit callers. Tokens and delimiter stack growth are admitted before
retention/advancement. Empty bodies require two tokens and one delimiter level.
The bounded body scan is iterative. Logical limits do not establish process
CPU/RSS isolation. Whole-source/fragment lexers remain separately bounded.

## Remaining semantics

This completes the first root attachment profiles for function headers, records,
enums and aliases. It does not resolve names, generic arity/bounds or visibility;
check generic bodies; substitute/specialize; or implement generic execution.
The separate [annotation resolver](generic-type-resolution-v2.md) now proves
registered head names/arity and initial Key constraints; it does not check a whole
function signature or body. The separate
[signature checker](generic-signatures-v2.md) now validates the complete supplied
header set against checked type definitions, while leaving bodies opaque.
Unknown annotation names and incorrect concrete generic arities may parse.
Module/package attachment, semantic/body checking, Option/Result and propagation
remain open. Existing CLI/source checking still refuses unsupported generic
programs; no unchecked or legacy fallback is added.

See [generic parameters](generic-parameters-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 remains partial.

The separate [specialization root adapter and concrete call graph](generic-specialization-v2.md)
now admits nongeneric source headers into this checked library pipeline. This
does not enable generic Program/CLI compilation or execution.
