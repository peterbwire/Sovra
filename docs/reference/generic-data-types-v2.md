# Compiler-owned Byte/Bytes annotations (explicit semantic API)

`check_generic_data_package_types(table, limits)` now registers sealed opaque
`std::Byte` and `std::Bytes` headers alongside the explicit collection schema and
checks real package source annotations. The complete data profile also registers
the sealed zero-parameter `std::JsonValue` enum. These type proofs feed the explicit
[data construction/IR/VM and CLI/native profile](data-host-v3.md).

| Type | Arguments | Constraint/representation |
| --- | --- | --- |
| `std::Byte` | none | Distinct opaque scalar; satisfies `std::Key` |
| `std::Bytes` | none | Distinct opaque buffer; does not satisfy `std::Key` |

Neither type is a record, enum, Int, String or `[std::Byte]`. Record literals,
field projections, numeric widening and implicit buffer/array conversions refuse.
Their runtime values are constructed through checked operations, without exposing
storage. The [Rust Byte/Bytes foundation](runtime-bytes-v2.md) already enforces
range/codec/copy rules, but it cannot attach arbitrary runtime values to these
compiler-owned type proofs.

Qualified names preserve existing user `Byte<T>`/`Bytes<T>` record names. Aliases,
arrays, records, enum/Option/Result payloads, imported signatures and collection
arguments retain exact source/environment identities. Generic forwarding and
concrete substitutions recheck under the same rules. Transparent aliases of Byte
satisfy Key; Bytes, Float, arrays and user record lookalikes do not. Phantom nominal
arguments remain distinct. Public `std::MapEntry<std::Byte,std::Bytes>` construction
and field access type-check with existing values without making either data type
constructible as a record.

## Proof and admission

`GenericTypeDeclaration::Data`, `GenericTypeDeclarationKind::Data(kind)` and
`CheckedGenericDefinition::Data` retain the sealed category instead of guessing
from caller names. Header/checked-proof constructors are private. The standard
factory creates only Byte/Bytes with fresh empty parameter scopes. Definition
checking verifies those scopes even though there are no parameter IDs to compare;
a fresh foreign definition cannot attach to an older header.

The same cumulative declaration, scope, name, source, member, annotation,
expansion and work limits apply. Byte/Bytes contribute two declarations and
two empty scopes; JsonValue contributes a third declaration/scope and its seven
variants. All three have zero type parameters. Input arity is checked;
`std::Byte<Int>` and `std::Bytes<T>` refuse. Raw resolution, alias expansion,
generic-call binding and concrete nominal/function Key rechecks recognize the
sealed Byte category without admitting arbitrary same-shaped records.

Default package and collection-only type APIs do not inject these headers.
Collection/default compilation, CLI and native image decoders therefore still
reject data annotations. Both existing layout planners explicitly refuse actual
Byte/Bytes layout discovery, including collection arguments and phantom types;
the explicit data planner publishes executable data tables. Existing collection
feature word 1023 and ordinary profile 511 remain unchanged; data images use
exact word 2047.

Tests cover nested/aliased/imported annotations, Key constraints, generic calls,
concrete substitution, public entry typing, distinct primitive/buffer/phantom
identities, unforgeable opaque construction, exact declaration/scope boundaries,
foreign empty-scope rejection and strict old-profile/layout refusal.

## Execution and qualification

The [data layout API](generic-data-layouts-v2.md) now feeds checked construction,
IR/VM operations, complete owned Result/error wrappers and runtime Byte keys.
Explicit package CLI/native admission and strict JSON are implemented; older
profiles keep their refusal gates. See [G13 qualification](../G13_QUALIFICATION.md)
for consolidated acceptance evidence and the remaining qualification gate.
