# Generic package declaration identities and visibility

`bind_generic_package_declarations(captured, limits)` consumes a sealed captured
graph and returns a complete immutable `GenericPackageDeclarations` table. It
parses generic aliases, records, enums and function headers in original root and
registered-module scopes. Function bodies remain opaque under the existing
bounded declaration grammar. This is not semantic checking or executable linking.

Each source and declaration has a private table-owned identity. Identical names
in different packages/modules remain distinct even when manifest display names
match. Functions and types occupy separate namespaces. Duplicates inside one
source namespace refuse. A source/declaration identity from another table refuses
lookup. Declaration spans remain original file offsets, including Unicode/CRLF
line/column positions; wrappers/imports are neither concatenated nor rewritten.

Entry sources admit leading `use dependency::module;` forms followed by root
declarations. Registered module sources admit those imports followed by exactly
their matching `mod name { ... }` wrapper. Imports must name a direct dependency
alias and one of its registered modules. Missing aliases/modules, malformed or
duplicate imports, mismatching wrappers and trailing unrelated forms refuse.

`resolve(source_id, qualifier, namespace, name)` performs explicit visibility
lookup. An empty qualifier selects only the current source. A one-part qualifier
selects a registered module in the same canonical package. A two-part qualifier
selects a dependency module explicitly imported in that same source file.
Imports never leak to sibling files or expose undeclared transitive dependencies.
Private declarations are visible in their own source only; cross-file requests
require an explicit export. Function, record and enum export flags are retained.
Aliases remain private because the admitted generic grammar has no exported alias
form. Standard builtins and type parameters are outside this declaration lookup.

The table retains original source bytes and a snapshot-origin view. Guarded
captures keep exact manifests; embedded native inputs have logical locations and
no filesystem capture authority or manifest snapshots. Shared generic
syntax quotas cumulatively admit declarations, retained fragments, parameters,
metadata, names, tokens and work across the graph. An additional import count is
hard capped at 8192. Module/import/index name copies and original-source position
mapping consume admission. Per-file frontend limits remain active. Refusal emits
source-owned E3124 diagnostics and exposes no partial table. Existing captured
graph limits bound selected source bytes and registrations separately.

This table binds declaration ownership and explicit imports; it does not bind
actual body/type use sites, validate signatures/types/bodies, specialize functions,
lower IR, execute code or produce native images. An unresolved call inside an
opaque function body does not make declaration collection fail. Callers must not
interpret table construction or visibility lookup as execution authorization.

Follow-on [selected-root compilation](generic-package-compilation-v2.md),
[package suites](generic-package-testing-v2.md) and
[native package images](generic-package-native-v2.md) now consume these identities.
Their Linux consumer evidence does not make this declaration table an executable
proof or close E1/E2/G13 and broader platform/backend qualification. Editor polish
is deferred and Task Manager is unchanged.
