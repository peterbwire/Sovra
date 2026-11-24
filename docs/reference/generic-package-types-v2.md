# Checked generic package types and signatures

`check_generic_package_types(declarations, GenericPackageTypeLimits)` checks a
sealed package declaration/import table through the existing generic registry,
alias, type-definition and signature checkers. It checks every alias target,
record field, enum payload and function signature, including unused declarations.
Function bodies remain outside this proof.

The registry retains a canonical header identity for every source-owned type.
Same-spelled records/enums in separate files remain nominally distinct. Original
names and source text are not renamed or concatenated. Parameter owners select
the original annotation's source/import context. Local type parameters shadow
nominal names; one-part module and two-part dependency/module qualifiers obey the
existing file-local visibility table. Unknown, private or unimported type names
refuse; wrong arity and unsupported Key arguments use existing core checking.

Transparent aliases resolve targets in their definition's source scope, preserving
private local helpers and preventing caller imports from supplying missing target
names. Alias cycles, symbolic bounds and argument substitution retain existing
checks. Compiler-owned Option/Result/Error and legacy result schemas are injected
with their established identities. The `std::` namespace remains compiler-owned.

Function signatures share one checked table but duplicate names are checked per
source scope. Separate package entries can each have main, and independent modules
can use the same helper name without collapsing identities. The proof maps owned
package declaration IDs to nominal type IDs and function signature IDs. Foreign
package-table or parameter-scope identities refuse. Expanded alias type equality
continues to use canonical registry IDs, not displayed names.

`GenericPackageTypeLimits` applies registry, alias, declaration and signature
limits cumulatively to the whole graph. Context records are independently hard
capped at 8192 with shared context-mapping work. Existing capture/declaration
budgets still apply. Any failure returns diagnostics without a partial proof.
E3123 errors identify original source owners and safe declaration/annotation
ranges; aggregate admission failures can have no source location. Nested alias
error details retain their separate provenance rather than being relabeled as a
caller token.

This is semantic type/signature checking, not executable package compilation.
Unscoped function-name lookup returns unavailable for scoped signature tables.
Flat generic body-checking profiles explicitly refuse those tables. The separate
[package body profile](generic-package-bodies-v2.md) now links source-aware call/type contexts. Checked type/signature evidence cannot be
passed into the flat body pipeline to bypass module visibility. Generic CLI
commands still refuse dependencies/modules; native image features are unchanged.

Package bodies now feed the explicit specialization/layout/IR/VM pipeline,
[selected-root commands](generic-package-compilation-v2.md),
[package suites](generic-package-testing-v2.md) and
[native graph images](generic-package-native-v2.md). Independent Linux consumers
are qualified; remaining platform/backend and E1/E2/G13 requirements stay open.
No editor polish or Task Manager work is included.
