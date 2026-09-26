# ADR 0006: Resolve service calls through application scopes

Status: Accepted option A; implementation is partial. The user directed
continuation after the proposal. Project-relative import records and target
validation are implemented. A tested experimental scope-resolution API now
handles explicit bindings and service identities; application parser integration
and service-call validation remain planned.

## Evidence and boundary

Fielddesk imports `app.services` and calls `maps.travel_minutes(...)` and
`payments.draft_invoice(...)` in `app/jobs.svr`. The project checker retains
service declarations and operations, but its callable index is not an import
graph or lexical symbol table. The executable parser does not accept this
application syntax. Project success is therefore not service-call validation.

Matching `maps.` text is insufficient: it may appear in comments or strings,
or `maps` may be a parameter, local variable or closure binding. Unknown dotted
names also include model operations and ordinary objects; they cannot all be
diagnosed as undeclared services.

## Decision: Option A, scoped resolution before enforcement

1. Introduce an application syntax layer that retains imports, source modules,
   service declarations, function/task bodies, binding scopes and member calls.
   Keep it separate from the executable subset until supported application forms
   can be lowered and executed correctly. Preserve compiler stage entry points.
2. For the initial import model, resolve `use app.services` relative to the
   project root as `app/services.svr`. Resolve only explicit imports; do not
   expose every discovered service globally. Reject paths escaping the project
   root and diagnose missing or ambiguous modules. This is the initial local
   package boundary, not a permanent ban on third-party dependencies. Future
   package resolution authorizes declared dependency roots; imports stay within
   the selected package root. Packages and re-exports require their own detailed
   design; see [package requirements](../design/PACKAGES_AND_LIBRARIES.md).
3. An import makes that file's service declarations available by service name.
   Repeated imports of the same source are idempotent. Distinct imported services
   with the same name are ambiguous rather than resolved by file scan order.
4. Parameters, locals and closure bindings take precedence in their lexical
   scope. Resolve a dotted call as a service call only when its receiver actually
   resolves to a service declaration. Ordinary member calls remain outside this
   service rule; they are not silently validated as object operations either.
5. For resolved service calls, check the manifest binding, operation existence
   and positional argument count. Do not claim argument/return type checking
   until application types are resolved. Preserve omitted annotations as absent.
   Named arguments, overloads and service values need separate specified support.
6. Unsupported syntax or unresolved scopes must remain explicitly partial;
   do not infer a service receiver by scanning substrings. Existing project JSON
   continues to describe wiring scope rather than executable validation.

This proposal changes how proposed application names are resolved. It does not
alter bare `print`, executable `module::function` calls or ADR 0005 privacy.
Global duplicate service declarations remain rejected by existing E4020 until
a separately reviewed service-identity change permits scoped duplicate names.

## Concrete examples after implementation

With `use app.services`, `maps.travel_minutes(from, to)` resolves to the imported
maps contract and checks its operation and two positional arguments.
`maps.missing(from)` receives an operation-not-found diagnostic.

Inside `fn example(maps: SomeObject) { maps.travel_minutes(from, to) }`, the
parameter shadows the imported name. This is not validated as a service call.
Similarly, quoted `"maps.missing()"` and source comments create no references.

## Implementation sequence and verification

First implement source-module/import records and lexical application nodes with
valid/invalid parsing cases. Then build resolution on those records, finally
enable service diagnostics and retained reference metadata. Cover missing
imports/files, duplicate imports, cycles, same-file declarations, parameter/local/
closure shadowing, nested scopes, comments/strings, unknown operations, argument
counts, binding failures and file-accurate JSON locations. Same-file services
are visible without an import; cycles must terminate without recursive rescanning.
Do not equate signature metadata tests with execution of application programs.

## Third-party library requirement

Independent developers must be able to publish reusable libraries. Module and
service identities must eventually incorporate resolved package identity;
global service-name checks cannot become an accidental ecosystem-wide namespace.
The current E4020 project rule remains unchanged until that migration is designed.
This ADR's service visibility rules do not automatically export private library
symbols. Local/Git/registry dependency selection and publication are planned,
not implemented by the existing file-reference checks.

## Alternative B: Defer call validation

Keep exposing service declarations and operation metadata while concentrating on
the executable language. This preserves current application checking semantics
but leaves service references unchecked. Do not add heuristic call diagnostics.

## Approval boundary

AGENTS.md requires: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." Import visibility and shadowing are
new application name-resolution rules. The user's subsequent continuation
selects option A. Implementation proceeds incrementally; import validation
does not imply that service receivers or application expressions are resolved.
