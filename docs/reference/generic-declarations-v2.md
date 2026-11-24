# Generic type-definition annotation checking

```rust
use sovra::compiler::v2::*;
let record = parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT)?;
let inputs = [GenericTypeDeclaration::Record(&record)];
let registry = register_generic_types(&inputs, GenericRegistryLimits::DEFAULT)?;
let environment = prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT)?;
let checked = check_generic_declarations(&environment, &inputs, GenericDeclarationLimits::DEFAULT)?;
```

The additive semantic API requires exactly one original alias/record/enum definition
for every registered root header, including unused declarations. Definitions must
match category and original parameter-owner IDs; another parse of the same spelling
is not interchangeable. Missing, duplicate, foreign and unregistered definitions
refuse without a partial checked collection. Input ordering may differ from registry
ordering; the returned definitions are aligned with registry identities.

Alias target validity comes from the prepared environment. Every record field and
every enum payload is expanded and checked symbolically under its declaration's
parameter owner. Unknown heads, wrong arities, false/unproven Key bounds and
expansion exhaustion refuse. Aliases may transparently prove Key constraints.
Nominal heads retain their registry IDs even when a caller's type parameter has
the same written name; substitution does not re-resolve canonical names in a
consumer's scope.

Raw empty enums now refuse this semantic stage. Nullary variants remain distinct
from Unit payloads. Recursive nominal record/enum references remain finite type
annotations; this checker does not unfold their definitions into recursive layouts.

## Retained proofs and provenance

`CheckedGenericDeclarations` owns immutable, registry-ordered definitions.
`definition(id)` refuses foreign registry identities. Category-specific checked
proof constructors are private. Each checked alias/record/enum retains its prepared
environment and original syntax, so extracted/cloned proofs remain source-owned.
Record field proofs and enum payload proofs align with original member indices;
nullary payload slots retain None. Expanded nodes retain their root, parameter
identity and input/alias source provenance.

Declaration ownership errors identify the supplied input index and full-source
name span. Missing definitions identify a registered header. Empty-enum errors
retain the full-source enum-name range. Annotation errors retain the registered
owner, field/variant index, full-source input-fragment range and boxed inner
expansion error. Input-local inner spans use that fragment's origin; alias-local
origins still refer to the alias target fragment, never to the field's source.
ParameterScope errors retain their separate parameter-list coordinate context.

## Cumulative admission

Defaults admit at most 1,024 registry/supplied declarations, 8,192 aggregate fields
and variants, 65,536 referenced declaration/parameter/member/type-syntax records,
4 MiB retained original/fragment source and 1 MiB referenced/copied name text.
The syntax-name ledger includes all retained declaration metadata names plus the
new declaration-name copy made by retaining its immutable syntax wrapper.
The prepared environment retains its independent header/alias quotas.

All annotations together admit 65,536 generated type nodes, 65,536 generated edges
and 1,000,000 logical checks/expansion steps. Each expansion's independent limits
are additionally capped by remaining cumulative allowances. Later members cannot
reset a ledger, and dropping earlier results does not refund it. Membership,
metadata/source/name admission precedes field/payload proof retention. The result
is published only after complete header coverage has been verified.

These are logical retention/work contracts, not physical-memory or CPU isolation.
Exact and one-under tests cover each aggregate and independent expansion cap.

## Remaining work

This checks complete root type-definition annotation collections, not generic
function bodies, runtime layouts, constructors, match plans, code
specialization or executable generic programs. Nongeneric/module/package contexts,
normalized type equivalence, Byte, Option/Result and propagation remain open.
The separate [signature checker](generic-signatures-v2.md) now checks supplied
function headers against this complete type context. Existing CLI/source checking
stays strict about unsupported generic programs.

See [alias expansion](generic-alias-expansion-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 remains partial.
