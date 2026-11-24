# Checked v2 standalone symbol navigation

```sh
svr check --experimental-v2 --format json --symbols main.svr
```

The opt-in successful standalone check report can now include the existing
`symbols` envelope for root and qualified module functions, module containers,
scope-owned value bindings, records, aliases and enum/variant declarations with constructor/match references. Every function body must check first;
semantic/read/syntax failures publish no symbol table. Without `--symbols`, check
reports remain unchanged. The option requires JSON and a standalone source file;
package directories, duplicate flags and use with other commands refuse.

`CheckedProgramSource::render_symbols(ProgramSymbolLimits)` projects the
same immutable source and checked program identities. The earlier
`render_function_symbols` API retains its function-only output and quotas. Function declarations come
from checked bodies and original tokens. References come only from canonical
`ProgramCallLink` caller/callee IDs; source spelling validates those retained
ranges rather than selecting a target. Forward calls, mutual recursion and self
calls therefore use the same identities as linked execution. Comments, strings,
constructor calls and standard calls do not create function references. Constructor
and match variant targets are projected separately from their nominal evidence.

The projection includes original function/body/name byte spans and written source
signatures, including annotations and header trivia. It does not invent resolved
or inferred type spelling. The language server's existing strict envelope decoder
validates tables, indices, strings and UTF-8 boundaries, then maps spans to UTF-16
positions for the open snapshot. Hover remains plaintext, and the extension
escapes Markdown display. Outline exposes checked functions, modules, enums and variants; definitions point to the
matching declaration in the same document. Hosts may sort the flat outline by
source range independently of compiler table order.

Default optional projection limits are 8192 total declarations/references across
functions and bindings, and
512 KiB complete serialized metadata. Names/signatures must fit the existing
4096-byte string envelope; source tokenization also retains frontend limits. Any
quota or ownership/range/projection refusal returns empty tables with
`truncated: true, unavailable: true`. No partial successful table is exposed and
ordinary checking remains successful. Explicit caller caps can be smaller; even
a zero cap can return the fixed unavailable envelope. These logical limits do
not establish hard time or physical-memory isolation.

The opt-in v2 LSP mode requests this projection automatically and advertises
function/binding/variant hover, definition and outline. Existing document generations, request
retirement and check invalidation prevent old symbols surviving source errors,
edits or close/restart. Missing/unavailable evidence yields no navigation facts.
Package overlays/linking, annotation/field/type-use navigation, semantic completion,
references/rename and error-tolerant semantics
remain open. No v1 symbol metadata is fabricated for unsupported v2 constructs.

Regression evidence includes exact/exceeded projection limits, source comments
and Unicode, canonical recursive/forward links, opt-in/failed-check CLI behavior,
protocol version/failure retirement and the actual VS Code development host.
Other installed/platform editor acceptance remains a separate gate. G13 stays
partial; this closes only the documented standalone navigation subset.

## Parameters, locals and match payloads

The full projection includes checked parameter, local and immutable match-payload
bindings and their read/write/indexed-write/match-scrutinee uses. Each use resolves
through its owning function's `ScopedDeclarationId`; function-local ordinals are
translated to a shared output table with checked offsets. Token spelling validates
retained ranges after target selection. No name search selects a declaration.
Initializer reads, nested shadows, sibling arms and same-named parameters in
other functions therefore keep their own targets. Indexed writes publish only
the root identifier span; index expressions retain their separate read targets.

Hover signatures use retained canonical checked types, with bounded iterative
array display such as `Array<Array<Int>>`. Mutable locals display `let mut`, and
payloads display `match payload`. Payloads use the existing local table category;
payload bindings remain outside the document outline. Empty-array holes and unavailable
labels display `checked type unavailable`, never a fabricated Any/Unknown type.
Function signatures still preserve written annotations and header trivia.

Binding records and all references share the global optional record/byte quotas.
A total-count, output or range/ownership refusal returns no function or binding
facts. Failed checks and newer document generations also retire the whole old
projection. Navigation does not run code or grant a runtime capability.

## Enum and variant declarations

The full projection now adds checked enum declarations and their variants to the
same declaration table, after functions. Enum and variant entries carry LSP kinds
10 (Enum) and 22 (EnumMember); missing kinds preserve the legacy function default
12. The strict decoder rejects unsupported or incorrectly typed kind values. The
extension maps these kinds to VS Code enum/member glyphs. Function-only library
projection remains unchanged.

Enum/variant declaration positions come from the checked type environment and
retained root declaration ranges. Constructor targets come from `CallableTarget`
nominal IDs; match targets come from retained exhaustive match plans and their
arm-to-variant mapping. An iterative preorder syntax walk pairs recursive matches
and controls with those plans. Reordered arms and same-named variants in different
enums retain their own targets. Original tokens validate ranges after selection.
References cover the variant identifier, keeping payload bindings and qualified
type prefixes separate. No declaration is selected by searching for its name.

Constructor signatures display canonical enum/variant identity and checked
payload type, including arrays, records and recursive nominal leaves. Nullary
variants remain distinct from Unit payloads. Display uses bounded iterative
traversal; recursive enum edges do not structurally unfold. Enum/variant records,
constructor references and match-arm references share the same global optional
record/byte ceilings with functions and bindings. A refusal clears every table.

This projection adds no source syntax. The checker now also supports
[transparent enum alias qualifiers](enum-aliases-v2.md): alias constructor and match
variant references retain canonical targets. Alias/record declarations now appear in the outline as described below.
Fields and qualified type prefixes still have no navigation link. Direct enum
function-header annotations now have the scoped navigation described below.
The projection does not guess those missing facts.

[Type-only inline modules](type-modules-v2.md) now retain qualified enum/variant
outline names and canonical variant targets. Module container/function/import
navigation is not inferred from those type facts.


## Module containers

Successful checked v2 programs expose same-file module declarations with LSP
Module kind (2), `mod name` detail, the full original declaration range and the
written module-name selection range. Empty modules are included. Containers are
appended after nominal entries to preserve constructor/pattern target indices;
editor hosts may sort the flat outline by source position. This adds no module-use
references, import resolution, hierarchical outline or package navigation.

Containers consume the same aggregate item and serialized-byte limits as all
other symbol tables. Either limit refusing the projection clears the entire
optional envelope. Invalid module bodies likewise publish no symbols. The
function-only projection remains unchanged.


## Record and alias declarations

The checked v2 outline now includes root and one-level module record/type-alias
headers, including unused declarations. Module-owned names are qualified (for
example `data::Point`). Records use Struct kind (23); aliases use TypeParameter
kind (26), since the LSP has no distinct type-alias kind. Alias detail retains the
original written `type` header, without an `export` prefix; record detail displays
`struct` and its canonical name. Selection ranges cover the original short name,
and declaration ranges retain the complete original declaration.

These entries follow existing nominal/container entries, preserving retained
function and variant reference indices. Source-order checked declaration spans
establish module ownership; name spelling does not select a type-use target.
The scan is linear over declarations and does not inspect function-body text.
All entries consume shared item/serialized-byte quotas. Names and display headers
are limited to 4096 bytes; an oversized header clears the complete optional
envelope while checking can still succeed. The function-only projection stays
unchanged. This does not add alias expansion hover, type-use definitions, record
field navigation, semantic completion or package linking.


## Explicit enum function-header annotations

Direct enum names in checked function parameter and return annotations now support
hover/definition to their canonical enum declaration. Array wrappers are traversed
iteratively, and the reference covers only the final written enum identifier.
Qualified names use the checked environment's enum identity; module-local short
names resolve in the defining function's module. Same-named enums in another
module cannot redirect the target. The original annotation/name tokens validate
the retained ranges after identity selection.

This deliberately covers explicit direct enum header names only. Alias spellings
are not redirected to their underlying enum declaration; alias definition targets,
local annotations, payload annotations, record types and fields remain open.
Qualified module prefixes have no new reference. These optional references share
all item/byte limits with the rest of the symbol envelope; the function-only
projection remains unchanged. No executable/type semantics change.
