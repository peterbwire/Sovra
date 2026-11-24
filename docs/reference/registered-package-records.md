# Registered nominal record interfaces

Status: **Partial G12; additive Rust API.**
`packages::compile_registered_records(root)` extends the source-owned registered
callable pipeline to nominal records whose fields resolve to canonical primitives
or visible declared records, including source-local aliases. It returns the same read-only
`RegisteredFunctionPackage` result, retaining captured inputs, checked units and
complete linked IR. Executable [CLI routing](registered-package-cli.md) now uses
this pipeline; [explicit locks](package-source-locks.md) and inventory-aware package formatting are implemented. Full context/editor integration remains open.
The older primitive-only compiler keeps refusing user records; ordinary entry-only
record compilation retains its wider existing type behavior.

The [independent shapes example](../../examples/multi-file-records/README.md)
constructs, passes and returns Point across a local package boundary. Interpreter
and Node JavaScript verify `25` followed by `1.5`, including integer-to-Float field
coercion and a provider-local constructor alias.

## Type identity and visibility

Canonical identities combine owning package, declaration scope and record name.
Two aliases of one canonical package share record identity; equal-shaped records
from distinct packages do not unify. A package-root Point and Point inside a
module literally named `root` are distinct. The shared internal identity helper
now escapes that module category, fixing its former collision without reserving a
new source name or changing other module labels. Both old entry-only and new
registered pipelines have regressions for this case.

Source-local primitive/record aliases resolve through core rules. Same-package
record spellings follow existing inline-module scope. Explicit dependency imports
bind only exported records in the selected module; imports remain file-local and
direct. Private types do not acquire public constructor names. Public function
signatures exposing private record types fail with existing E4116 diagnostics.
Private root helpers remain internal to their package as in the callable pipeline.

Record fields resolve to Unit/Bool/Int/Float/String or a visible declared record
after source-local alias resolution. Duplicate/unknown fields retain core
E3008/E3017 errors in the provider source. A public record field exposing a private
record type fails E4116 at the original annotation. Nonregistered field contexts
(such as standard record interfaces not integrated into this registry) still
return E4164, never a truncated valid layout. Unknown/undeclared type contexts
fail; names grant no capabilities.

Field edges store canonical record IDs without recursively expanding layouts.
Existing recursive record declarations remain admissible, as in core executable
checking; this does not add default values, cyclic value allocation or ownership
semantics. Metadata is a finite graph bounded by source/record/field/interface
allowances. An early lower-bound check rejects graph-wide shape copies that
cannot fit the remaining interface budget before repeated source/record context
scans. Actual copies still consume the allowance individually. Alias cycles retain
existing errors.

Nested constructors, returns, field access and argument checks retain nominal
identity and coercions in both engines. Returning a public nested record supports
inferred field access without granting the caller transitive source import or
constructor aliases. Explicit imports remain direct and file-local.

## Source ownership and checking

Original source ASTs remain unchanged in the retained source-unit collection.
Checked ASTs normalize own record declarations to canonical shapes and retain
source-spelling aliases; imported aliases/shapes are attached as checking metadata.
This uses the existing alias/record semantics, not synthetic source text. Imported
metadata spans use the consumer import or file-level zero context, never foreign
source offsets labeled as consumer text. Provider declarations/field validation
and caller construction/argument errors retain their own original source paths.

Canonical shape metadata alone grants no source-visible name. The compiler also
retains inaccessible shapes needed to understand returned values; only permitted
source aliases make constructors/types addressable. Source cannot spell internal
`@package...` identifiers. Public type facts/IDE navigation must not treat attached
checking metadata as original source declarations: checked units still have no
raw-source identity claim, and original ASTs/buffers stay separately owned.

Every loaded/private/unused function body still uses checked semantic evidence
and checked lowering. Field access, construction, returns, calls and numeric
coercions consume consistent canonical types. No partial IR or unchecked fallback
is returned on a semantic, interface, metadata or emission failure.

## Resource and host scope

Existing source capture/AST, interface, checked-evidence and emission budgets
remain cumulative. Record registry entries/fields, aliases, attached shape copies,
string labels and runtime-type mappings consume the relevant allowances before
retention. Repeat import bindings count again even if their names deduplicate.
The independent fixture accepts exactly 35 interface nodes and rejects 34, including standard-record registry entries; zero
string/evidence/emission allowances also refuse compilation explicitly. These are
component budgets, not complete process memory/CPU or filesystem sandbox bounds.

The pipeline retains its Linux/procfs guarded-reader scope; actual other-host,
MSRV/native/release qualification is separate. Explicit locked compilation is a
separate wrapper that verifies captured manifests/sources before this pipeline.
The ordinary record compiler does not enforce a lock. Complete context/editor
integration remains open. G11 and Task Manager behavior are unchanged.

## Standard result records

Existing public standard records (`std::InputLine`, `std::TextRead`,
`std::TextWrite`, `std::SplitOnce`, `std::ParsedInt`) retain their global nominal
identities through registered callable signatures, aliases and record fields.
Their existing core field schemas and value semantics apply. No user layout or
new permission is attached to those identities. The primitive-only API retains
its earlier refusal rules. Approved structural `[T]` array interfaces are now supported, including nested
arrays and visible nominal/standard element records. Private leaf types remain
forbidden in exported signatures and public fields. Array wrappers consume copied
interface node/string budgets; checked lowering preserves recursive numeric
widening and existing value snapshots. See the [G13 contract](../design/G13_COLLECTIONS_AND_DATA.md).
