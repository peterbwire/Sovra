# Bounded generic type resolution in v2

```rust
use sovra::compiler::v2::*;
let entry = parse_generic_record(
    "struct Entry<K:std::Key,V>{key:K,value:V}", GenericRecordLimits::DEFAULT,
)?;
let registry = register_generic_types(
    &[GenericTypeDeclaration::Record(&entry)], GenericRegistryLimits::DEFAULT,
)?;
let parameters = parse_generic_parameters("<T>", GenericParameterLimits::DEFAULT)?;
let annotation = parse_generic_type("Entry<String,[T]>", GenericTypeLimits::DEFAULT)?;
let resolved = registry.resolve(&annotation, &parameters, GenericResolutionLimits::DEFAULT)?;
```

The additive Rust semantic API indexes source-owned root generic alias/record/enum
headers and resolves a complete admitted annotation. Function declarations are
not type heads. Headers retain their category, declared parameter owner and a
registry-owned identity. Duplicate root type names, primitive-name declarations
and primitive-name type parameters refuse. Root user names such as `Option` and
`Result` remain available; standard qualified types are not fabricated.

Resolution selects the declaration-local parameter first, then an existing
primitive or registered header. Unknown names and wrong argument counts refuse.
Primitives and parameters take no arguments. Registered heads require exactly
their declared parameter count. Arrays retain their element structure. Resolved
nodes align with the original immutable syntax's postorder spans and use earlier
child indices. Registry/parameter/syntax owners are retained by shared immutable
references. Foreign declaration IDs refuse; clones retain their original owner.

## Initial `std::Key` proofs

Each applied header's declared `std::Key` constraints are checked against resolved
argument nodes. Existing Int, Bool and String primitives satisfy Key. A symbolic
parameter satisfies it only when its own declaration explicitly carries the Key
bound. Float, Unit, arrays and nominal records/enums do not satisfy Key.

An alias argument returns `AliasBoundNeedsExpansion`: this stage does not guess
its underlying type. This includes aliases that would eventually expand to a key.
The separate [prepared alias environment](generic-alias-expansion-v2.md) now
proves that case through bounded transparent expansion. Byte is planned in D1 and is not
fabricated as a supported primitive here. General traits/coherence and implicit
conversions are not introduced.

## Admission and error provenance

Registry defaults admit 1,024 declarations, 4,096 referenced parameter records,
64 KiB copied declaration-name bytes and 1 MiB referenced parameter-fragment
source. Each copied name is shared between its header and lookup index; parameter
owners are not deep-copied. Input declaration order defines registry ordinals.
Identity allocation uses checked compare/exchange and cannot wrap.

Resolution defaults admit 4,096 nodes, 4,096 structural edges and 65,536 logical
checking steps. Node/edge totals are admitted before construction. Node visits,
parameter-name comparisons, registry lookups and argument-bound checks consume
work before the operation. Lookup/string comparison cost is not CPU-time or
physical-memory isolation. Exhaustion returns no partial successful table or
unchecked fallback. Exact and one-under tests cover all dimensions.

Declaration errors identify the input declaration ordinal and original full-source
span. Annotation/arity/bound errors use the original type-expression fragment's
coordinates. `ParameterScope` errors instead identify the name in the retained
parameter-list fragment. Limits have no source span. A caller attaching diagnostics
to a complete declaration must use the corresponding fragment's origin range.
No label/identity grants filesystem or runtime authority.

## Limits of the proof

This proves annotation head lookup, arity and the bounds described above only.
Registration does **not** check alias targets/cycles, record fields/layout or enum
payloads/inhabitation. A registered declaration may still have an invalid body.
Only root generic headers and the five existing primitives are included;
nongeneric/module/package/standard-record contexts are not implicitly loaded.
Children are arena-local indices, not a cross-annotation canonical type identity
or alias-normalized equality key.

Full declaration/signature/body checking, bounded code specialization,
generic constructors/matches, Option/Result and
propagation remain open. Existing CLI/source checking still refuses unsupported
generic programs. See the approved [G13 contract](../design/G13_COMPLETION_CONTRACT.md).
