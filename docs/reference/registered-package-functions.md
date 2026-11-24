# Registered package primitive-function compilation

Status: **Partial G12; additive Rust compiler API.**
`packages::compile_registered_primitive_functions(root)` now captures registered
sources, checks each original unit and links complete IR for callable interfaces
using canonical primitives and local primitive aliases. It is not the general
module/record compiler by itself. Executable CLI commands now select the separate
record-capable compiler for registered graphs; older Rust/context/overlay/formatter
APIs remain strict rather than silently selecting an incomplete graph.

The [independent example](../../examples/multi-file-packages/README.md) prints
`49` through its registered application module and a separate utilities library,
in both the interpreter and Node-executed JavaScript. Its library keeps a private
helper and uses a local primitive alias in the exported signature.

## Ownership and checking

`RegisteredFunctionPackage` provides read-only `sources()`, `checked_units()`,
`ir()` and `remaining()` accessors. Captured buffers and original AST/imports stay
in the [source-unit collection](package-source-units.md); each checked-unit index
is its source-unit owner ID. Checked ASTs qualify local type spellings and have
`CheckedUnit::source() == None`; they do not claim raw-text identity. The enclosing
collection retains original buffers/ownership. No files are concatenated or given
foreign byte ranges. Ordinary body/type/call errors use core codes and the actual
owning source path. Provider interface errors point to the provider; call errors
point to the calling unit.

Every loaded unit, private helper and unused body is checked. A failure returns
no partial program. Interfaces are registered before body checking, allowing
ordinary same-package mutual recursion without treating call cycles as package
cycles. Existing package cycles still fail in graph resolution.

Same-package qualified module calls expose exported functions only. Private
module helpers remain accessible only within their defining module under existing
qualified-call rules. Entry top-level functions remain package-root helpers,
visible to other units of that package as with existing inline modules. They are
not implicitly exposed to dependency consumers. Dependency entry points are
checked but never automatically invoked.

Explicit dependency imports are file-local, resolve only direct declared aliases
and select the requested existing module's public functions. Entry imports do not
leak into registered sibling files; undeclared/transitive aliases and missing
modules use E4115. Duplicate identical imports preserve behavior but consume
interface materialization budgets again. Unknown/private function calls and wrong
arguments use the core executable diagnostics, at original caller spans.

## Identities and interfaces

Canonical graph roots determine local package identity, independently of display
names. Package IDs use deterministic source-inventory traversal; linked symbols
include package and module names, with only the root entry's `main` kept as the
execution entry. Distinct roots with equal display/module/function names do not
merge. Multiple aliases of one canonical package share targets. Relocating a
relative dependency layout preserves linked IR.

Primitive interface annotations and aliases are normalized through the shared
core type-scope/alias rules before checked calls/lowering. Imported Float coercions
retain their existing behavior in both execution engines. Function return/body
validity is checked independently; a declared signature never substitutes for a
valid implementation.

User record declarations currently return E4163 context unavailable in this
additive mode. Nonprimitive exported signatures retain existing E4116 failures;
qualified cross-file type contexts are not integrated yet. These are limits of
this primitive callable API, not new restrictions on the older executable record
language. General nominal record interfaces, type visibility and source-aware
record metadata are required later G12 work. Model/enum/UI/policy semantics remain
part of still-partial G11.

## Budgets and host scope

The reader retains its guarded Linux/procfs host limits and bounded source/token/
AST admission. All calls are checked through `CheckedUnit` package evidence and
checked lowering, without a legacy fallback. Existing cumulative evidence limits
apply across source units; repeated interface bindings consume the allowance.

Interface materialization uses the existing 250,000-node and 128 MiB string
budgets, counting provider signatures/parameters, per-consumer copies (including
repeated imports), labels and linked-symbol copies. Linked emission shares the
existing 4,000,000-instruction/256 MiB string budget across all units. E3023/E3024
report interface exhaustion; checked-evidence failure remains structured
`CheckedPackageError::Evidence` (E3025 on conversion), and emission failures retain
E3021/E3022. No exhaustion path publishes partial IR or retries unchecked lowering.
These component limits do not claim a full process-memory/CPU sandbox.

This API has no manifest-content lock verification, lock commands, new
formatter/editor graph support, registry/network access or native-host
qualification. Source capture is not an atomic filesystem snapshot. The compiler
consumes the already captured unit buffers/ASTs without reopening source paths.


The separate [record-capable API](registered-package-records.md) now adds nominal
primitive-field record interfaces, construction, arguments and returns through
this same checked pipeline. The primitive-only API deliberately retains its
original scope/refusal. Explicit [source locks](package-source-locks.md) wrap the record-capable pipeline; remaining type/context/editor integration stays open. See [CLI routing](registered-package-cli.md)
for the now-implemented executable command integration.


The approved G13 slice now permits primitive array annotations (`[Int]`,
`[[Float]]`) through this API's existing interface scope. Copied normalized array
wrappers consume interface node/string budgets. The record-capable pipeline
additionally supports arrays of its visible nominal/standard records. No unchecked
fallback, source concatenation or broader record support is added to this API.
