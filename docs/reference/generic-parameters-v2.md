# Generic parameter ownership in v2

```rust
use sovra::compiler::v2::{
    parse_generic_parameters, parse_generic_type,
    GenericParameterLimits, GenericTypeLimits,
};
let parameters = parse_generic_parameters("<T, K:std::Key>", GenericParameterLimits::DEFAULT)?;
let annotation = parse_generic_type("Map<K,[T]>", GenericTypeLimits::DEFAULT)?;
let uses = parameters.bind_type(&annotation, 2)?;
```

The additive Rust API parses exactly one nonempty explicit parameter list.
Parameters retain their written name, declaration/name spans, optional bound and
original qualified bound span. Only the approved initial `std::Key` bound spelling
is admitted; unqualified `Key`, arbitrary user traits, multiple bounds, defaults,
qualified parameter names, duplicate names and trailing input refuse. Whitespace
and comments retain original UTF-8 source coordinates.

Each successfully parsed list owns distinct opaque `GenericParameterId` values.
Ordinals follow source order but are insufficient for cross-list identity.
`find` resolves within the list, and `parameter(id)` refuses an unrelated owner's
identity. Clones retain the same owner. Identity allocation never wraps; failed
parses publish no owner or partial successful list.

## Annotation parameter uses

`bind_type` accepts an admitted [type expression](generic-type-syntax-v2.md) and an
explicit finite use allowance. It links exact unqualified parameter-name nodes
to this list's identities. Arrays and application arguments participate through
the flat syntax arena. Repeated occurrences keep distinct node positions and
share their declaration identity. Qualified `pkg::T` does not bind to parameter
`T`. Applying type arguments to a parameter, such as `T<Int>`, refuses.

The returned `GenericParameterUses` table retains immutable owners for both the
annotation and parameter declarations. These owners share source/metadata through
immutable reference-counted storage; binding does not copy whole annotations or
parameter lists. Node indices address the retained syntax only. Exhausting the
use allowance returns no partial successful table. Lookup is a linear scan of
parameters per named annotation node, bounded by their admitted input sizes;
this is not a helper CPU or physical-memory isolation guarantee.

## Limits and remaining semantics

Default parameter limits are 64 KiB input, 16,384 tokens including EOF, 128
parameters and 4,096 cumulative copied parameter-name bytes. Each record/name
is admitted before retention. The bound is a fixed tag; its path is not copied
into every record. Input source, lexer tokens and parameter metadata have
independent quotas. Zero record/name allowances refuse when needed.

This is declaration-list ownership and syntactic parameter-use binding, not a
checked generic function/type. Other annotation names remain unresolved. Bound
spelling does not establish whether a concrete type satisfies `std::Key`.
[Root generic aliases](generic-alias-syntax-v2.md) now attach parameter owners
to declaration syntax, as do [generic records](generic-record-syntax-v2.md).
Function/enum attachment, semantic
name/arity/visibility checks, body checking,
bounded substitution/specialization, Option/Result and propagation remain open.
Existing source/CLI checking stays strict; there is no unchecked execution path.
No default legacy AST/runtime variants or memory semantics change.
