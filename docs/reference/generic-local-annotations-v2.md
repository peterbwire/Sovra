# Explicit local annotations in symbolic generic bodies

The symbolic copy-body profile supports typed locals, including
[mutable locals](generic-mutable-bodies-v2.md):

```svr
fn copy<T>(value:Box<T>)->Box<T> {
    let local:Echo<Box<T>>=value;
    return local;
}
```

This example requires checked Box/Echo type definitions in the Rust API context;
it is not yet an executable generic CLI program. `check_generic_copy_bodies` now
validates each written local type in the function's type-parameter scope, using
the prepared alias environment. Unknown heads, wrong arities, false Key bounds
and incompatible initializer types refuse without a partial checked body.

The initializer resolves before introducing the local, including when the name
shadows an existing value. Local value names do not become type parameters or
change the type namespace. The binding retains the declared expanded type rather
than silently retaining the initializer's unconverted type. Existing concrete
scalar Int-to-Float widening is explicit on the let statement; aggregate widening
and opaque-parameter numeric assumptions remain unsupported.

## Original-token parser boundary

A private generic block adapter extends only local annotation spelling. It retains
the existing statement shape, original tokens/ranges and ordinary expression
parser. Public legacy Program parsing and the earlier block API keep their strict
original type grammar. Every extended annotation is independently reparsed from
its exact original source fragment and resolved before a body proof can publish.
No extended legacy Program is exposed or passed unchecked to ordinary lowering.

Adjacent `Box<T>=value` is handled only at the root local type-argument terminator:
the first byte of the lexer's >= token ends the type, and the second is its
assignment. The type span excludes the assignment byte. Nested generic closings,
array wrappers, qualified names, supported comments and whitespace retain original
coordinates. Comparison operators in expressions and public legacy parsing are
unchanged. Malformed/empty/trailing type arguments refuse.

`GenericCopyStatement::Let` retains its optional `GenericCopyLocalAnnotation`,
original full-source type range, expanded type proof and explicit initializer
conversion. The original initializer expression still has its actual type.
Grammar/expansion errors preserve the full-source input-fragment range and bounded
inner error. Alias-origin spans retain their own target source, never the local
annotation's origin.

## Cumulative local type admission

`GenericCopyBodyLimits.local_types` supplies independent fragment syntax/expansion
profiles plus table-wide caps: 4,096 annotations, 1 MiB copied original type source,
65,536 fragment tokens including EOFs, 65,536 parsed nodes, 65,536 parsed edges,
1 MiB copied type-name text, 65,536 expanded nodes and 65,536 expanded edges.
Remaining allowances cap every subsequent local across every function. Expansion
also consumes the existing cumulative body semantic work budget; comparison uses
the existing body type-comparison budgets. Syntax parsing/helper CPU remains
separately bounded by source/token/node/depth admission, not a hard deadline.

`local_annotation_usage()` reports each retained/generated dimension. Type syntax
now records its admitted token count including EOF, avoiding an extra tokenization
pass for counting. The private core parser's temporary annotation spelling is
covered by the original body lexer/parser admission, before this fragment stage.
All successful local proofs are retained only within a complete body result.

## Remaining work

Qualified/implicit calls, field mutation, record and inferred-array construction, aggregate coercions,
opaque numeric operations and matches still refuse this initial profile. Concrete
substitution/rechecking, bounded specialization, generic runtime IR/backends and
remaining G13 phases remain open. Existing generic CLI execution stays blocked.

See [symbolic copy bodies](generic-copy-bodies-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md).
