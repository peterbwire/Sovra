# Single-entry source units

Status: **Partial G13 E1 source integration; opt-in Rust API.**
`compiler::v2::check_entry_source(label, source, EntrySourceLimits)` checks a complete
supported root unit containing aliases, records, nominal enums and exactly one
function. Types and the entry body come from the same source; callers no longer
supply a separate context AST or enum fragments. The sole function is the entry;
its name need not be main and its parameters remain explicit caller-supplied values.

The [independent enum example](../../examples/enum-entry/main.svr) prints 7 through
this API. The example includes an alias, record payload, constructor and exhaustive
match. The checked unit retains the original source/opaque label, ordered declaration
ranges and scoped callable facts. Entry checking reuses the existing bounded
[type environment](enum-types-v2.md) and [scoped callable](scoped-callable-v2.md).
Export intent is retained without establishing module/package visibility.

Lexing admits the whole file first. Disjoint token projections preserve original
byte/line/scalar-column coordinates, including Unicode strings/comments and CRLF.
No whitespace masking, line scanning, reconstructed syntax or ignored unknown body
establishes success. Per-enum ordinal labels distinguish internal declarations;
all ranges in this source-unit API refer to its retained original source. Earlier
fragment APIs retain their existing fragment-coordinate contracts.

`EntrySourceLimits` separately bounds whole-source bytes/tokens/diagnostics,
declaration count, label bytes, aggregate/per-fragment enum source and metadata,
and the existing function/call/type/scope evidence dimensions. Function fragment
bytes/tokens retain their own caps too. Source tokens are projected into disjoint
declaration/context streams, with one additional EOF per enum/entry fragment;
there is no full-file copy per declaration. Metadata/syntax limits continue to
apply before admission in their original layers. These are logical input/retention
bounds, not physical memory or complete-work limits. Zero limits are real.

`CheckedEntrySource::execute(arguments, EntryLoweringLimits, RuntimeVmPolicy)`
builds the structural map, expression graph and function IR, then executes the
[VM](runtime-vm-v2.md) with its existing memory-only default boundary.
`execute_with_services` adds explicit program arguments and independently optional
input/files. Compilation/lowering quotas remain separate from runtime fuel; the
VM elapsed clock begins after lowering, not before source checking. Lowering is
currently repeated for each execution rather than retained in a self-referencing
compiled object. Errors distinguish source/type/body checking from mapping,
expression/function emission and execution. No exhaustion selects another engine.

Additional functions, imports, modules, app/service declarations, nested matches,
generics and other unsupported forms refuse the whole unit. Missing entry also
refuses. This does not provide cross-function linking, packages/tooling integration,
JavaScript/Worker/native support, or extended enum CLI parsing/execution. Public
v1 AST/stage APIs and default CLI behavior remain unchanged. Full program support
and the rest of the [G13 contract](../design/G13_COMPLETION_CONTRACT.md) remain open.

A separate [root-program checker](program-source-v2.md) now checks multiple root
functions and canonical forward/mutual call links. It does not relax this API's
single-entry scope; its checked tables can now use the separate
[linked-program runtime](program-execution-v2.md).

The source projector also admits [type-only inline modules](type-modules-v2.md),
with original module/type spans and parallel enum module paths. This does not
add module functions, nested modules or imports to the single-entry API.

The separate syntax-only API also retains module functions and parallel
`function_modules()` paths for formatting drafts. The single-entry API stays strict; the program API and native source decoding
now support [checked qualified module functions](module-functions-v2.md). Syntax
ownership alone still establishes no semantic or executable proof.
