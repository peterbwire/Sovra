# Checked generic function signatures

```rust
use sovra::compiler::v2::*;
let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT)?;
let environment = prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT)?;
let types = check_generic_declarations(&environment, &[], GenericDeclarationLimits::DEFAULT)?;
let syntax = parse_generic_function("fn identity<T>(value:T)->T{return value;}", GenericFunctionLimits::DEFAULT)?;
let signatures = check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT)?;
```

The additive semantic API checks every supplied root generic function signature
against a complete checked type-definition collection. Explicit parameter and
return annotations are expanded symbolically and checked for names, arity and Key
bounds under the function's own parameter owner. Unused signatures are checked too.
Duplicate function names, existing standard-callable conflicts and primitive-name
type parameters refuse. A root generic `main` refuses because the application entry
has no explicit generic-argument selection; no implicit entry inference is added.

An omitted return annotation becomes `GenericSignatureReturn::ImplicitUnit`.
It creates no fabricated type-expression node or source reference. A written Unit
annotation stays Explicit with its original expanded proof and coordinates.
Annotations retain their original parameter identities and registered nominal IDs,
including transparent alias expansion and consumer-name capture avoidance.

## Sealed ownership and provenance

`CheckedGenericSignatures` retains functions in supplied source order and allocates
an opaque table-owned `GenericSignatureId` only after the whole table succeeds.
IDs cannot wrap or address another signature table. `find` performs root-name
lookup within the admitted function count; `signature(id)` validates ownership.
Clones retain their original table owner.

Each `CheckedGenericSignature` has a private constructor and retains the complete
checked type-definition context, original source syntax, symbolic parameter types
and explicit/implicit return convention. Extracted cloned proofs keep those source
and type owners alive. Annotation-use tables now share immutable storage as well,
so retaining explicit return annotations does not deep-copy their use tables.

Definition errors carry the input function index and original full-source name or
type-parameter range. Annotation errors retain the input function, value-parameter
index or return position, full-source input-fragment range and bounded boxed inner
expansion error. Alias target origins retain their separate source context; callers
must not translate those spans through a function annotation's range.

## Cumulative admission

Defaults admit 128 functions, 4,096 aggregate value parameters, 65,536 referenced
function/parameter/annotation records, 4 MiB retained full/fragment source and
1 MiB referenced/copied names. Full source includes opaque body text once; its
lexical body metadata was admitted by the earlier source parser. Retained name
accounting includes syntax metadata and the function-name wrapper copy.

All explicit annotations together admit 65,536 generated nodes, 65,536 generated
edges and 1,000,000 logical signature/expansion steps. Independent per-annotation
caps are reduced by remaining aggregate allowances; later annotations/functions
cannot reset a ledger. The complete checked type context retains its own earlier
admission. These are logical budgets, not CPU/RSS isolation.

## Bodies remain unchecked

This is a checked **signature**, not a checked generic function. Its retained body
is still only lexed and delimiter-balanced by the source stage. Unknown calls,
invalid statements, invalid operators and incorrect return coverage can remain in
that body. The signature result must never be used as lowering/execution evidence. A
separate [initial symbolic body checker](generic-copy-bodies-v2.md) now checks
immutable copies, returns and equality and refuses everything outside its profile.

The separate [equivalence/key API](generic-type-equivalence-v2.md) now supplies
bounded exact comparison and in-process normalization. Symbolic body
parsing/checking, call target/type-argument plans, bounded code specialization,
constructor/match IR, nongeneric/module/
package integration, Byte, Option/Result and propagation remain open. Existing
CLI/source execution still refuses unsupported generic programs. See the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 remains partial.

The separate [specialization root adapter and concrete call graph](generic-specialization-v2.md)
now admits nongeneric source headers into this checked library pipeline. This
does not enable generic Program/CLI compilation or execution.
