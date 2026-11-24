# G13 completion contract

Status: **Complete for the documented experimental Linux scope: implementation, consolidated compiler/runtime/LSP qualification and actual VS Code host acceptance pass. Additional platform and release acceptance remains separate.**
The user requested finishing G13. The existing [18-task checklist](../18_TASK_COMPLETION_CHECKLIST.md)
remains authoritative: arrays/resource controls and enum syntax metadata do not
close generic collections, executable errors, binary/JSON, filesystem/time or
process work. The user approved this coordinated language/host contract for sustained
implementation in dependency order.

## Completion scope and order

| Phase | Deliverable | Required completion proof |
| --- | --- | --- |
| C0 | Useful ordinary array algorithms | Independent package, snapshots, empty/duplicate/order cases, bounded execution and native output. |
| E1 | Nominal enum checking, constructors and exhaustive handling | Typed payloads and binding/return-flow checks across compiler, runtimes, packages and tooling. |
| E2 | Generic functions/records/enums/aliases and explicit errors | Bounded checked specialization, std::Option/std::Result and explicit propagation, independent consumers. |
| C1 | Generic Map/Set | Deterministic snapshot operations/iteration/equality, key constraints and resource refusal. |
| D1 | Bytes/codecs and JSON | Strict encoding/precision/duplicate/error-offset behavior, bounded parsers and independent round trips. |
| H1 | Filesystem/path/directory and time APIs | Explicit authority, portable contracts, errors, limits and supported-host evidence. |
| H2 | Process APIs | G14's minimum owned-resource/cancellation/cleanup contract implemented alongside the API. |
| Q1 | Consolidated qualification | Full compiler/tooling/parity/security regressions, independent locked/native consumers, accurate remaining platform gates. |

Use small reviewable changes within this sequence. Approval covers implementing
these defined choices without approval per builtin. Material departures remain
review boundaries. Do not mark later phases complete from metadata or scaffolding.
Networking/HTTP/TLS/async scheduling stay G14, browser execution G15, mobile G16,
and exact release/platform qualification G17–G18. H2 must implement its actual
resource prerequisites; a future G14 label cannot excuse missing process cleanup.

## E1: executable nominal errors

Current implementation is partial: [versioned enum type facts](../reference/enum-types-v2.md)
resolve root/module payloads and produce constructor-type and exhaustive arm-shape
plans. [Standalone match syntax](../reference/match-syntax-v2.md) now retains
ordinary arm bodies, and [closed-scope checking](../reference/match-bodies-v2.md)
validates immutable payload bindings, core types and conservative per-arm returns.
A [single-function source checker](../reference/enum-function-v2.md) now derives
locals and checks direct matches plus required return coverage. The opt-in
[callable-source API](../reference/callable-function-v2.md) now checks constructors
and self-recursion with bounded call/context facts. The
[typed callable API](../reference/typed-callable-v2.md) now adds core expression
types, call/context value links and array-element conversions. The
[scoped callable API](../reference/scoped-callable-v2.md) adds declaration/read/write/
match-scrutinee identities. A [structural mapper](../reference/callable-mapping-v2.md)
now validates source/fact ownership, shape and coverage.
[Expression register IR](../reference/expression-ir-v2.md) now emits/validates ordered
value operations and operand conversions, retaining deferred statement contexts.
[Function IR](../reference/function-ir-v2.md) now emits stores/scopes/branches/loops/
match dispatch/returns and validates reachable availability/tag/scope flow. The
initial Rust VM below executes this sealed subset with shared limits. Broader
standard/host qualification, modules/imports and backend/package/tooling
integration remain open. Root-program calls and recursive match/control bodies
are implemented as described below.

Use the approved nullary/single-payload enum declaration spelling. Variant names
must be unique and an executable enum must have at least one variant. Capture
exports under the same module/package visibility and nominal ownership principles
as records. Resolve payload primitives, aliases, arrays, records and enum types;
reject alias cycles/unresolved names under bounded graph work. Nominal recursive
references must not trigger unbounded structural expansion, and runtime values
remain owned acyclic snapshots with iterative traversal/cleanup.

Construct variants with `Outcome::Text(value)` and nullary variants with
`Outcome::End()`. Expected payload type drives only existing numeric widening.
Different enums or generic instantiations are not interchangeable, even if payload
layouts match. No implicit conversions between Result<Int,E> and Result<Float,E>.

Add exhaustive statement matching on an already bound scrutinee:

```svr
let outcome = obtain();
match outcome {
    Outcome::Text(text) => { print(text); },
    Outcome::End() => { print("end"); },
}
```

Evaluate the scrutinee once. Require each variant exactly once, with correct
payload binding arity; reject missing/duplicate/foreign variants. Each payload
binding is an immutable, arm-local snapshot, using ordinary lexical shadowing
rules. Initial matching has no guards, wildcard/or/nested patterns or match
expression. All arms must return for the match to prove function return coverage;
otherwise control continues normally. No automatic exceptions or fallthrough.
Keep enum/match contextual where possible and preserve existing ordinary named
identifier programs through compatibility tests.

The [single-entry source-unit API](../reference/entry-source-v2.md) now derives
root type declarations and exactly one checked entry from one admitted source,
retains original token coordinates and lowers/executes its supported subset.
The [root-program checker](../reference/program-source-v2.md) now adds canonical
function IDs and checked forward/mutual call links across every body under explicit
cumulative signature/evidence/link caps. [Linked Rust execution](../reference/program-execution-v2.md)
now executes these root tables with shared policy/services and aggregate lowering
limits. [Matches inside arms](../reference/recursive-match-v2.md) now retain
recursive bodies and execute under the existing scope/fuel contracts.
[Structured controls](../reference/control-match-v2.md) now permit those matches
inside if/else-if/else/while with existing type/scope/IR rules and explicit depth
admission. Modules/imports and broader backend/package/tooling integration
remain open; this
is not complete E1/program/platform qualification.

## E2: bounded generics and propagation

The [bounded generic annotation parser](../reference/generic-type-syntax-v2.md)
now retains qualified applications/arrays and original spans under independent
quotas. [Generic parameter lists](../reference/generic-parameters-v2.md) now
retain distinct owners, approved bound spelling and bounded annotation parameter
uses. [Generic alias attachment](../reference/generic-alias-syntax-v2.md) now
retains that owner and target-use links in one complete source declaration.
[Generic record attachment](../reference/generic-record-syntax-v2.md) now shares
one parameter owner across bounded source-owned field annotations.
[Generic enum attachment](../reference/generic-enum-syntax-v2.md) likewise
shares one owner across nullary/single-payload variants with cumulative admission.
[Generic function headers](../reference/generic-function-syntax-v2.md) now
attach the same ownership to explicit parameter/return annotations and retain
bounded opaque bodies. [Bounded annotation resolution](../reference/generic-type-resolution-v2.md)
now checks registered head names, arity and initial concrete/symbolic Key proofs.
[Prepared alias environments](../reference/generic-alias-expansion-v2.md) now
validate alias target cycles/symbolic bounds and perform bounded transparent
identity-based substitution. [Complete type-definition annotation checking](../reference/generic-declarations-v2.md)
now requires every original definition and checks record fields/enum payloads,
including nonempty enum coverage. [Generic signature checking](../reference/generic-signatures-v2.md)
now checks all supplied function parameter/return annotations against complete
type definitions. [Bounded type equivalence and normalized keys](../reference/generic-type-equivalence-v2.md)
now compare expanded structures while preserving nominal/parameter identities.
[Initial symbolic copy bodies](../reference/generic-copy-bodies-v2.md) now check
immutable locals, returns and equality under cumulative budgets.
[Explicit local annotations](../reference/generic-local-annotations-v2.md) now
resolve generic/alias types and initializer contexts in the original function scope.
Full body/call/control coverage, runtime constructor/match plans, concrete code
specialization and executable integration remain open.

Support explicit type parameters on functions, records, enums and aliases:
`fn identity<T>(value:T)->T`, `struct Entry<K,V>{key:K,value:V}` and
`enum Result<T,E>{Ok(T),Err(E)}`. Type annotations use `Name<T,E>`; retain `[T]`
array spelling. Initial generic calls/constructors use explicit type arguments,
for example `identity::<Int>(7)` and `std::Result::<Int,std::Error>::Ok(7)`.
Do not introduce backward/implicit generic argument inference in this contract.

Check generic bodies symbolically, including names/visibility and permitted
operations; recheck concrete substitutions before lowering. Opaque parameters can
be copied, returned, compared for existing structural equality and used in typed
containers. Arbitrary numeric operators on unconstrained T fail. The initial
compiler-known qualified `std::Key` bound permits Int, Bool, String and Byte; no general user
trait/coherence system is selected. Keep qualified type arguments, constraints,
coercions and nominal package identities in compiler-owned evidence.

Use bounded specialization: at most 1,024 concrete instantiations per standalone
compilation, 4,096 per package graph, structural depth 128 and cumulative existing
AST/interface/name/IR quotas charged for generated work. Explicit APIs may select
other finite caps. Expanding recursion/exhaustion refuses without legacy/unbounded
fallback, even for functions emitting little IR. Ordinary nongeneric compilation
keeps its existing limits/behavior.

Provide nominal std::Option<T> = Some(T)/None and std::Result<T,E> = Ok(T)/Err(E),
plus std::Error with code/message String fields and offset std::Option<Int>.
Reserve only their qualified standard identities; do not steal existing user
Option/Result names. Existing specialized standard result records remain usable;
provide explicit adapters and do not silently change old builtin return types.

Postfix ? is restricted to Option/Result in a correspondingly returning function.
Successful extraction copies the payload under snapshot/fuel rules. Failure returns
a new compatible failure wrapper in the caller's declared return type, copying its
error payload; error type must match exactly. No implicit error conversion, panic
unwrapping, exception catching or cross-thread propagation. Ordinary explicit match
remains available and is required where propagation is not type-compatible.

## Compatibility and representation migration

Preserve existing public Program/Module field shapes, Statement/Expression/Value
variants and raw stage APIs. Add versioned checked syntax/IR/runtime APIs for the
new constructs; do not project unsupported new nodes as successful old programs.
Legacy APIs remain strict and continue accepting their existing source subset.
Reuse validated old operations/adapters for ordinary constructs incrementally.
CLI selection of the extended pipeline must preserve ordinary diagnostics, limits,
IR/runtime outputs and package/tooling behavior before it becomes the new-source
path. This is a reviewed new-feature integration, not an implicit rewrite of G11.

Use a versioned extended native encoding with explicit feature/version refusal
and retained decoding of old images. Existing old-feature/native outputs and
builder integrity contracts stay compatible. A new public runtime value model
belongs to the versioned API; do not break Rust clients by adding variants to the
old public Value. No new source ownership/reference semantics are selected.

Logical model v2 must inherit every existing event's v1 cost unchanged. Define and
test complete new enum/map/set/byte events before publication: admit entire
snapshot results before copying, count newly owned type/variant/record labels,
never refund drops, and do not double charge transferred children. An enum creates
one wrapper node plus its owned type/variant labels; an existing payload transfers.
Payload bindings/extraction copy the full payload footprint. Control-only variant
selection creates no hidden VM Bool unless one is actually materialized. Enum
formatting is `Type::Variant(payload)` or `Type::Variant()`; equality requires the
same nominal instantiation/variant and existing structural payload equality.
Keep defaults opt-in; these units are not physical-memory or helper-work bounds.

The [runtime value layer](../reference/runtime-values-v2.md) now implements the
enum/value subset of model v2: flat owned snapshots, nominal construction/tag/
extraction/equality/display and atomic wrapper/label/subtree admission. This does
not execute function IR or establish the remaining event/backend/host integration.

The [initial Rust VM](../reference/runtime-vm-v2.md) now executes sealed single-
function IR with enum/array/record operations, iterative self-recursion, shared
component budgets and cooperative control. Its standard adapter is limited to
print/println/to_string/len, split_once/parse_int/lines_unique/escape_terminal and
array_len/slice/append/concat. An explicit borrowed command-context API adds
arg_count/arg/set_exit_code with captured status, without process I/O. File
calls remain refused in earlier APIs. Explicit input/services APIs adapt
read_line and read_text/write_text/write_text_if through independently granted
capabilities, minimum preflight/post-call fuel and cooperative checks. Completed
service effects survive later refusal. Broader host qualification, default CLI,
JavaScript/Worker/native and module/package/tooling integration remain open.

## C1: deterministic Map/Set

Map<K,V>/Set<K> use the Key constraint above; exclude Float/compound keys initially.
Strings compare exact Unicode contents with no normalization. Operations return
snapshots, not references or shared mutation. Replacing a map value keeps its
insertion position; removing/reinserting places it last. Set duplicates retain
the existing entry. Iteration returns snapshot arrays in insertion order.
Map equality compares associations independent of insertion order; Set equality
compares membership. Maps/sets of different nominal type arguments do not mix.

Provide create/len/contains/get/insert/remove and entries/keys/values snapshots;
missing get/remove use Option/Result rather than a fabricated zero/default value.
Length cannot exceed the shared 4,294,967,295 representation ceiling. Charge each
new container/entry and full copied key/value descendants in the versioned cost
table. Document lookup complexity without promising constant time or CPU isolation.

## D1: byte values, codecs and JSON

Byte is distinct from Int, range 0..255, with checked Int conversion returning
Result and explicit conversion back. Bytes is an immutable snapshot byte buffer;
no arbitrary invalid UTF-8 String. Buffer conversion/access/slice/concat and strict
hex/base64/UTF-8 codecs return structured errors with original byte offsets.
Count one logical Byte node per copied byte plus one buffer node; binary data is
not owned UTF-8 text. Permit packed physical storage without changing those units.
Hex output is lowercase; reject odd lengths/nonhex. Base64 uses standard alphabet
with validated padding and no ignored whitespace. UTF-8 decoding never replaces
invalid input. Checked aggregate lengths precede allocation and publication.

JSON uses a recursive explicit value type: Null, Bool, Int, Float, String, Array
and Object<Map<String,JsonValue>>. Parse integers exactly as i64; out-of-range
integer syntax fails rather than becoming an imprecise Float. Fraction/exponent
syntax becomes a finite Float; overflow fails. Reject duplicate decoded object
keys, invalid UTF-8, invalid escapes/lone surrogates, trailing data and non-JSON
number forms. Keep original zero-based UTF-8 byte error offsets, insertion order
and Unicode contents without normalization. Serialization round-trips the value
model, including numeric kind; do not claim canonical signing/hash serialization.
No eval, implicit record deserialization or filesystem authority.

Explicit parser caps cover input bytes, depth, total values/members and decoded
text: initial opt-in profile 16 MiB input, 128 levels, 1,000,000 values/members,
64 MiB decoded UTF-8. Validate caller caps and checked arithmetic before growth;
return no partial successful parse. Test exact/refused boundaries and primary
JSON/UTF-8/base64 standards when implementing codecs, not only happy-path examples.

## H1/H2: explicit host authority and finite resources

Add versioned host adapters rather than required methods on existing RuntimeHost.
New privileged APIs deny by default unless the trusted embedding host supplies an
explicit versioned policy. Existing legacy I/O retains its documented behavior;
this is not a process sandbox. Source cannot grant itself host authority.
New CLI/native policy selection must be explicit and reviewed in the same
implementation, with no change to old defaults. Policy may narrow authority;
requested limits cannot enlarge host-selected ceilings.

Filesystem APIs cover paths, metadata, bounded reads/writes, directory enumeration,
create/remove/rename and explicit errors. Input Strings are UTF-8 paths; reject NUL
and report nonrepresentable returned paths without lossy replacement. Use path
components and canonical/handle validation, not string-prefix containment. Keep
read/write roots separate; test link/reparse/race refusal on claimed hosts. If a
safe host operation is unavailable, report Unsupported rather than grant broader
access. Document cooperative policy limitations against external filesystem races.

Initial time support: validated Gregorian dates in years 1..9999, UTC instants and
fixed-offset time zones, formatting/parsing/arithmetic, explicit wall/monotonic
clocks and bounded timers. Leap-second/named-IANA-zone support is explicitly outside
this first supported scope, not silently approximated using host local time.
Wall-clock values never stand in for elapsed deadlines. Timer cancellation is
cooperative; scheduling/concurrency belongs to G14.

Process APIs use explicit executable/argument vectors, environment allowlists,
working directory and captured input/output. Never interpret a shell string or
inherit unrestricted environment by accident. An executable grant permits that
program to act with the host OS authority: file roots do not sandbox child code. The adapter owns each spawned child,
closes streams, bounds both outputs, checks cancellation/deadlines, and kills/reaps
the owned child before successful cleanup. Preserve earlier effects and surface
failed cleanup; no descendant-process termination guarantee without OS support.
Versioned policy includes explicit byte/entry/argument/time ceilings. At minimum,
start with 16 MiB/file or stream, 4,096 directory entries, 1,024 arguments/64 KiB
aggregate argument text and 30 s per owned child/timer, all lowerable by the host.
Do not claim H2 done while its G14 resource ownership/cancellation prerequisites
are only planned. Real sockets, HTTP/TLS, async and server concurrency stay G14.

## Q1: definition of done

For every capability, require an independently consumed package, source diagnostics,
checked/legacy or explicit versioned compatibility evidence, Rust/Node/worker
behavior and exact/exceeded resource/error cases. Execute locked formatter/check/
test/run and actual Node-free Linux native outputs. Include malformed/Unicode/
truncated/duplicate/extreme inputs, deep/wide snapshots, cleanup and prior effects.
Keep CI/MSRV, formatter/IDE/import/package coverage and full regression evidence
current. Other host/browser/mobile/release support requires its own G15–G18 gates.

Mark G13 complete only when the expanded checklist's functional items are backed
by these tests on explicitly supported hosts, with remaining advanced/platform
scope stated accurately. Review physical-memory/helper-work/default policy limits
separately; optional logical counters and cooperative checks are not hard process
isolation. Do not invent a sandbox or declare G11/G12/G14 complete to close G13.

## Review boundary

[AGENTS.md](../../AGENTS.md) requires: "Document and pause before fundamental syntax,
memory-model, type-semantics or compatibility decisions." The approved enum syntax
slice explicitly excludes executable enums, generics, matching and AST/IR changes.
The user approved these coordinated phases before implementation. Continue
within this contract without requesting approval per builtin. Material departures
remain review boundaries. Approval authorizes implementation of this contract
as a whole; it does not declare G13 done.

The opt-in `svr check --experimental-v2` command now checks v2 root sources with
human/JSON diagnostics and no execution. Default commands are unchanged; G13 remains partial.
Package/backend/tooling integration remains open. See [experimental checking](../reference/experimental-v2-check.md).

`svr run --experimental-v2` now explicitly executes root sources with finite VM
limits, arguments/status and success-only output publication. No stdin/file hosts
are granted by default. See [experimental execution](../reference/experimental-v2-run.md).

`svr test --experimental-v2` now runs parameterless Unit root test_ functions in
source order with isolated invocation state, bounded suite output/test count and
no input/file hosts. See [experimental testing](../reference/experimental-v2-test.md).
Default testing is unchanged; packages/backends/tooling and G13 completion remain open.

Experimental `run` now accepts independent `--allow-stdin` and `--allow-files`
grants for existing bounded ProcessHost services. Without flags, v2 execution
denies both; experimental check/test remain host-free. File grants use process permissions without path confinement and
completed effects survive later refusal. See [explicit hosts](../reference/experimental-v2-hosts.md).

`svr build --experimental-v2 [--emit ir]` now emits deterministic bounded linked
IR inspection without requiring main or executing source. IDs are local, source
coordinates retained and controls escaped. See [IR inspection](../reference/experimental-v2-ir.md).
This is not an executable image/codec; v2 native/JS/package/tooling gates remain open.

Experimental v2 check/run/test/IR commands now accept guarded single-entry package
directories. Captured entry bytes and canonical diagnostic identity are retained;
dependencies/registered modules still refuse. See [root packages](../reference/experimental-v2-packages.md). Linux/procfs package capture is qualified;
other package hosts and full linking/locks/backends/tooling remain open.

Single-entry v2 packages now support `lock --experimental-v2 [--check]` and
`--locked` check/run/test/IR over the same captured bytes, using the existing
strict schema and guarded publication. See [v2 locks](../reference/experimental-v2-locks.md). Lock integrity is not source validity/authentication;
dependency/module and other-host/backend/tooling gates remain open.

`svr fmt --experimental-v2` now formats root enum/match syntax without requiring
semantic validity, with token/comment/AST/idempotence gates and the existing guarded
source-write path. See [v2 formatting](../reference/experimental-v2-format.md). Default formatting is unchanged; opt-in package formatting and standalone
editor selection are now available. Syntax-only parsing does not establish checked success.

Opt-in v2 IDE mode now provides standalone buffer diagnostics, snapshot formatting
and source-only check/IR/run tasks. See [v2 editor mode](../reference/experimental-v2-ide.md).
V2 package overlays, type/field navigation, semantic completion and other-host
qualification remain open; G13 remains partial. Legacy mode remains the default.

`fmt --experimental-v2 --package --check|--write` now reuses bounded package
selection, all-source preflight and guarded staged publication with the v2 syntax
formatter. See [package formatting](../reference/experimental-v2-format.md). It does not link or execute
dependencies/registered modules; v2 compilation and G13 completion gates remain open.

`build --experimental-v2 --emit exe --output PATH` now builds a private
source-backed native runner with a version/model/feature-checked image, finite
shared CLI policy and no stdin/file services. See [v2 native scope](../reference/experimental-v2-native.md).
The binary rechecks/lowers embedded source at startup; it is not an executable IR
codec or optimizing backend. Legacy images retain their path. G13 remains partial;
module/dependency integration and the E2/C1/D1/H1/H2 library phases remain open.

Standalone v2 JSON checks can now request `--symbols` for bounded checked root-
function hover/definition/outline. The v2 LSP requests it automatically; failed
checks and optional projection refusals clear facts. See [root-function navigation](../reference/program-symbols-v2.md).
No constructor/standard/type/field/package navigation is implied. G13 stays partial.

V2 standalone symbols now also project scope-owned parameters, locals and match
payloads, including shadowing, indexed writes and match-scrutinee identities.
Checked type labels are bounded; unresolved array holes remain explicitly unavailable.
See [binding navigation](../reference/program-symbols-v2.md). Type/field/package navigation remains open.

V2 standalone symbols now also expose checked enum/variant outline entries and
constructor/match-arm variant definitions, selected by nominal IDs and coverage
plans. See [nominal navigation](../reference/program-symbols-v2.md). Alias/record/annotation/field and
package navigation still require their own evidence; G13 remains partial.

V2 enum aliases now work as constructor and match-arm qualifiers under the same
canonical nominal IDs, payload rules and standard-call collision refusals. Alias
signatures consume existing count/text quotas; the canonical lookup index is
charged to retained type metadata. See [enum aliases](../reference/enum-aliases-v2.md).
Default legacy behavior is unchanged; G13 remains partial.

V2 now admits type-only inline modules containing enums/aliases/records, with
qualified use by root functions, original spans and qualifier-work admission.
See [type modules](../reference/type-modules-v2.md). Module functions, nested modules, imports and
dependency linking remain refused; same-file lookup is not package visibility.
G13 remains partial.

V2 raw syntax/formatting now retains module function bodies with original names,
spans and parallel module paths under shared admission. Checked source, execution
and native decoding still explicitly refuse those bodies. See [syntax-only module
bodies](../reference/type-modules-v2.md). Scope/visibility/call linking remain open; G13 is partial.

V2 program checking/execution now supports qualified same-file module functions,
private/exported call filtering and module-owned type views with charged aggregate
metadata. See [checked module functions](../reference/module-functions-v2.md). Root entry/test selection
remains root-only. Conflicting root-type shadows, nested modules/imports/dependency
linking remain refused; the single-entry API stays strict. G13 is partial.

Concrete numeric arithmetic and ordering now have bounded symbolic body facts;
see [numeric body profile](../reference/generic-numeric-bodies-v2.md).
Opaque numeric assumptions and generic execution remain open.

[Boolean symbolic body expressions](../reference/generic-logical-bodies-v2.md)
now check both operands and retain short-circuit operator identity. Executable
short-circuit lowering and broader generic execution remain open.

[Concrete String symbolic expressions](../reference/generic-string-bodies-v2.md)
now cover concatenation and lexical ordering without implicit formatting.
Runtime allocation admission and broader generic execution remain open.

[Mutable symbolic locals](../reference/generic-mutable-bodies-v2.md) now support
fixed-type direct assignment with explicit widening and binding identity.
Field/index mutation, calls, control flow and generic execution remain open.

[Symbolic expression statements](../reference/generic-expression-statements-v2.md)
now retain checked discarded results without conversions or fabricated bindings.
Calls, control flow, specialization and executable integration remain open.

[Symbolic conditional checking](../reference/generic-conditional-bodies-v2.md)
now checks concrete Bool conditions, isolated branch scopes and all-path return
coverage using a flat proof/worklist. Loops, calls and generic execution remain open.

[Symbolic while checking](../reference/generic-loop-bodies-v2.md) now validates
concrete Bool conditions, isolated loop scopes and fixed parent assignments
without adding guaranteed returns. Calls, specialization and execution remain open.

[Read-only symbolic array indexing](../reference/generic-index-bodies-v2.md)
now preserves exact element types and original parameter/provenance owners.
Bounds remain runtime checks; construction, indexed writes and execution remain open.

[Symbolic indexed assignment](../reference/generic-index-assignment-v2.md)
now checks direct mutable array locals, concrete Int indexes and exact element
replacement types. Runtime bounds/snapshot ownership and execution remain open.

[Context-typed symbolic array literals](../reference/generic-array-literals-v2.md)
now check nested/empty arrays from typed local, return and assignment contexts
with explicit item widening. Inferred arrays, calls and execution remain open.

[Parameter-independent symbolic record fields](../reference/generic-field-bodies-v2.md)
now preserve nominal field/receiver identity after a bounded reachable-type walk.
Dependent fields require checked substitution; field writes and execution remain open.

[Direct field-parameter projection](../reference/generic-field-arguments-v2.md)
now selects validated receiver arguments while preserving caller parameter and
provenance owners. Composite substitution, calls and execution remain open.

[Composite record field substitution](../reference/generic-field-substitution-v2.md)
now preserves caller parameters and per-node input/alias sources under independent
and cumulative node/edge/work/depth limits. Calls and execution remain open.

[Explicit call/constructor type-argument syntax](../reference/generic-type-arguments-v2.md)
now admits bounded source-owned `::<T,E>` lists in one shared arena.
Callee binding, checked call facts, specialization and execution remain open.

[Caller-scope explicit type arguments](../reference/generic-argument-checking-v2.md)
now resolve names, aliases and type-head Key constraints with exact caller owners
and cumulative copy/expansion budgets. Callee binding and execution remain open.

[Generic callee type matching](../reference/generic-call-binding-v2.md) now verifies
exact table/environment/caller owners, arity and callee Key requirements under
matching budgets. Value matching, body calls and execution remain open.

[Bounded callee signature substitution](../reference/generic-signature-instantiation-v2.md)
now produces caller-scoped value-parameter/return types with per-node provenance
and cumulative/per-type graph limits. Value/body calls and execution remain open.

[Generic value-argument type matching](../reference/generic-call-values-v2.md)
now verifies exact caller body/expression owners, value arity and substituted
parameter compatibility with explicit widening. Source body calls/execution remain open.

[Source-backed generic call envelopes](../reference/generic-call-syntax-v2.md) now
retain original callee/type/value ranges with grammar-checked explicit types and
opaque balanced values. Body expression/call integration and execution remain open.

[Original-token call value expressions](../reference/generic-call-expressions-v2.md)
now parse exact original ranges with bounded AST/work admission and iterative
cleanup. Caller-scope typing, source-call facts and execution remain open.

[Symbolic arena identities](../reference/generic-arena-identities-v2.md) now prevent
IDs from separate checks of the same signature being interchanged. Clones retain
their arena; future call-source expression facts must use distinct arena owners.

[Source-backed symbolic body calls](../reference/generic-body-calls-v2.md) now
connect explicit root syntax, active lexical scopes, type/bound/signature stages
and contextual values under cumulative budgets. Runtime/specialization remain open.

[Explicit symbolic record construction](../reference/generic-record-construction-v2.md)
now checks nominal arguments, complete field coverage and contextual values with
shared source/provenance and cumulative budgets. Layout/runtime integration remains open.

[Explicit symbolic enum constructors](../reference/generic-enum-construction-v2.md)
now preserve nominal/variant identity and nullary versus Unit payloads under
shared input/context budgets. Match plans, specialization and execution remain open.

[Generic enum pattern headers](../reference/generic-enum-patterns-v2.md) now prove
exact selected enum coverage/payload contexts under bounded source/graph/work
admission. Active match-site scope, arm typing/return coverage and execution remain open.

[Symbolic generic match bodies](../reference/generic-match-bodies-v2.md) now connect active lexical
scope, one scrutinee read, immutable payload IDs, exhaustive arm typing and
conservative return coverage under shared body/graph/work budgets. Nested matches
and explicit calls retain original source ranges. Generic specialization, executable
lowering/runtime/backend integration and structured error propagation remain open.

[Finite concrete generic call discovery](../reference/generic-specialization-v2.md) now admits
source-owned nongeneric roots and discovers exact argument instances with bounded
substitution, normalized reuse and recursive closure. Defaults cap standalone
instances at 1,024 and the explicit package profile at 4,096; no package linking
is performed. Concrete signature/body rechecking, layout plans, executable
lowering/runtime/backend integration and structured error propagation remain open.

[Concrete generic body rechecking](../reference/generic-concrete-bodies-v2.md) now substitutes
instance signatures/bindings/expressions and rechecks operation, conversion, call,
constructor/field and match-payload contexts under cumulative generation/comparison/
work budgets. Original lexical ownership/return-flow proofs remain sealed; concrete
lookup selects a graph instance before using symbolic IDs. Literal decoding,
nominal/runtime layouts, executable lowering/backend integration and structured
error propagation remain open.

[Decoded generic literal payloads](../reference/generic-literals-v2.md) now retain exact
Int/finite Float bits/Bool/decoded UTF-8 values from original source ranges, sharing
immutable facts across instances while charging each reference. Existing conversion
edges remain explicit; source/token/string-copy/work admission is finite. Runtime
layouts, constructor/match plans, executable lowering/backend integration and
structured errors remain open.

[Logical generic runtime layouts](../reference/generic-layouts-v2.md) now map concrete
source facts to finite primitive/array/record/enum graphs with exact normalized
identity, declaration-order slots/tags and recursive edges. Function and nominal
type instances share the 1,024 standalone / 4,096 package-profile cap. Input/member
arenas, generated metadata, references and lookup work stay bounded. No physical ABI
or executable path is selected; constructor/match plans, lowering/backends, package
linking and structured errors remain open.

[Generic nominal operation plans](../reference/generic-construction-v2.md) now link source-order
record initializers, enum tags/payloads and field reads to exact instance layouts.
Match dispatch retains one source read, immutable payload destinations, source-order
arms and exhaustive tag mappings with source-contained statement ranges. Planning
is cumulative and bounded; executable IR/control-flow lowering, runtime allocation/
snapshot behavior, backend/package integration and structured errors remain open.

[Typed generic instruction lowering](../reference/generic-ir-v2.md) now emits owned
stack instructions with resolved control flow, source-order operands, concrete
calls, short-circuit branches and enum dispatch. Function/slot/target structures
and reachable typed-stack joins are checked under finite emission/pending/state/
work budgets. Scalar test probes verify behavior; production runtime/backend
execution, resource/value policy and source/program/package/CLI integration remain
open alongside structured errors.

[Host-free generic Rust execution](../reference/generic-runtime-v2.md) now runs sealed
IR from a zero-argument root with iterative calls and flat owned compound values.
Snapshots, indexed replacement, enum payloads, equality and deep cleanup retain
value semantics under shared existing materialization/instruction/control policy
and explicit work/frame/slot/operand caps. Borrowed result inspection grants no
host services. Source/program/package/CLI/native/JS integration, validated external
arguments and structured errors remain open.


G13 E2 source checkpoint (2026-11-15): [standalone generic source execution](../reference/generic-source-v2.md)
now collects complete admitted files and checks all declarations/bodies before
specializing and executing a zero-argument root. Ordinary nongeneric calls share
the bounded pipeline; original whole-file error locations and cumulative syntax
quotas are retained. This is a Rust library API. CLI/package/native/JavaScript
integration, standard Option/Result, propagation and later G13 phases remain open.


G13 E2 CLI checkpoint (2026-11-15): [explicit standalone generic commands](../reference/generic-cli-v2.md)
now expose bounded `check` and host-free `run` under `--experimental-generics`.
The CLI requires a parameterless Unit main for execution, retains original-file
errors, supports existing check JSON and refuses unavailable hosts, packages,
program arguments and engine combinations. Package/backend/tooling integration,
standard Option/Result and propagation, and later G13 phases remain open.


G13 E2 native checkpoint (2026-11-15): [generic host-native packaging](../reference/generic-native-v2.md)
now links explicit standalone generic builds as Rust executables with bounded
embedded-source rechecking and host-free VM execution. Separate image dispatch
preserves legacy/plain-v2 formats; source/IR quotas and command policy remain
finite. This is source-backed execution, not direct generic machine-code lowering.
Package/JavaScript/tooling integration, standard errors/propagation and later
G13 phases remain open; other-host qualification remains a gate.


G13 E2 root-package checkpoint (2026-11-15): [guarded generic root packages](../reference/generic-packages-v2.md)
now connect check/run/native build to captured single-entry projects and optional
source-lock verification, with generic lock generation/checking. Source-read caps
can reduce existing guarded admission; canonical diagnostic provenance is retained.
Runtime generic errors use E3125, reserving existing E3124 for unsupported package
shapes. Dependency/module linking, JavaScript/tooling, standard errors/propagation
and later G13 phases remain open. Package capture still requires supported Linux/procfs.


G13 E2 standard-enum checkpoint (2026-11-15): [qualified Option/Result](../reference/generic-standard-v2.md)
now use compiler-owned nominal identities through complete source checking,
constructors, exhaustive matches, specialization and host-free execution. User
Option/Result names stay available; standard templates consume semantic budgets
and have separate provenance. Native feature word 3 advertises standard enums;
old word-1 images retain their prior environment. std::Error, adapters, postfix
propagation, dependency/module linking and remaining G13 work stay open.


G13 E2 standard-error checkpoint (2026-11-15): [structured std::Error](../reference/generic-error-v2.md)
now has compiler-owned code/message Strings and optional Int offset, checked
qualified construction, fields and owned snapshots through Result payloads.
Public generic record parsing stays strict; ordinary user Error types stay free.
New native feature word 7 includes Error while retaining old word-1/word-3 profiles.
Explicit adapters, postfix propagation, dependency/module/JavaScript/tooling and
later G13 work remain open. Other-host/installed qualification remains separate.


G13 E2 propagation-contract checkpoint (2026-11-15):
[sealed propagation compatibility](../reference/generic-propagation-v2.md) now validates selected body-owned standard Option/Result
operands against enclosing return wrappers under cumulative context/comparison/
work budgets. Successful types may differ; Result error types match exactly.
Original body/provenance and variant/type plans are retained. This is a Rust
planning API only: source question marks, concrete/executable lowering, native
feature integration, adapters and remaining G13 work stay open.


G13 E2 propagation-syntax checkpoint (2026-11-15):
[original-source postfix capture](../reference/generic-propagation-syntax-v2.md) now retains bounded operand/question/
result spans for complete blocks, with nested/chained applications and parser
rollback coverage. Private capture publishes no unwrapped legacy AST. Ordinary
lexer/parser, checked source, CLI and existing native profiles remain strict.
Source/body binding, concrete rechecking, early-return/runtime/native integration,
adapters and remaining G13 work stay open.


G13 E2 checked-postfix checkpoint (2026-11-15):
[opt-in symbolic postfix bodies](../reference/generic-propagation-bodies-v2.md) now bind original questions to active
body operands and sealed success/failure types. Complete checking covers aliases,
scopes, calls, fields/indexes, control flow and unreachable code under shared
comparison/work/virtual-node/depth admission. Specialization and lowering refuse
question-bearing tables; full-source/CLI/native profiles remain strict. Concrete/
layout/early-return/runtime/native integration, adapters and remaining G13 work stay open.


G13 E2 concrete-postfix checkpoint (2026-11-15):
[opt-in concrete propagation](../reference/generic-propagation-concrete-v2.md) now discovers finite closures and recomputes
closed operand/success/failure/return contracts per instance, checking original
symbolic descriptors under shared generated-type/context/comparison/work caps.
Ordinary entry points remain strict; IR rejects every question-bearing source
table, including unreachable questions. Layout/early-return/runtime/native
integration, adapters and remaining G13 work stay open.


G13 E2 propagation-layout checkpoint (2026-11-15):
[bounded propagation operation layouts](../reference/generic-propagation-layouts-v2.md)
now retain input, extracted payload, failure tag/payload and caller-return layouts
per concrete source site under shared admission. Early-return lowering and execution
remain gated; executable integration, adapters and remaining G13 work stay open.


G13 E2 propagation-IR checkpoint (2026-11-15):
[inspection-only early-return IR](../reference/generic-propagation-ir-v2.md)
now lowers each checked question once, validates standard wrapper/failure contracts
and typed success continuations under shared IR admission. Its separate owner
cannot enter the public VM. Ordinary lowering and runtime remain gated for every
question-bearing source table. Payload execution/cleanup/fuel, source/native
integration, adapters and remaining G13 work stay open.


G13 E2 propagation-VM checkpoint (2026-11-15):
[explicit propagation VM execution](../reference/generic-propagation-vm-v2.md)
now copies success/error payloads, rebuilds compatible failure returns and cleans
current-frame slots/operands under shared snapshot/work/control admission. The
ordinary VM remains gated. Executable source/CLI/native integration, adapters
and remaining E2/G13 work stay open.


G13 E2 propagation-source checkpoint (2026-11-15):
[explicit complete-source propagation](../reference/generic-propagation-source-v2.md)
now collects original complete files and composes symbolic/concrete propagation,
operation layouts, IR and VM execution under independent shared stage admission.
Ordinary source/CLI/native profiles remain strict; native encoding refuses the
new profile. CLI/native integration, adapters and remaining E2/G13 work stay open.


G13 E2 propagation-command checkpoint (2026-11-15):
[experimental CLI/package/native propagation](../reference/generic-propagation-cli-v2.md)
now selects the complete checked propagation source profile. Native feature word
15 enables bounded startup rechecking; words 1/3/7 remain strict. Existing root
package/lock/authority and command-entry restrictions remain. Explicit adapters,
remaining E2/G13 work and other-host qualification stay open.


G13 E2 standard-result adapter checkpoint (2026-11-16):
[explicit source-level legacy result adapters](../reference/generic-result-adapters-v2.md)
now admit five immutable legacy schemas and provide ordinary Sovra conversion
functions under existing shared quotas. The experimental CLI uses this profile;
native feature 31 retains it. Builtin-call bridging, other-host execution and
remaining E2/G13 requirements stay open. Existing builtin return types are unchanged.


G13 E2 host-free builtin checkpoint (2026-11-16):
[checked parse_int/split_once calls](../reference/generic-builtin-calls-v2.md)
now preserve legacy result records through symbolic/concrete/IR/VM checking and
explicit adapter/question composition. Experimental CLI/native feature 63 selects
this profile; words 1/3/7/15/31 remain unchanged. Other builtins, remaining E2/G13
work and other-host qualification stay open. No host I/O is enabled.


G13 E2 String-builtin checkpoint (2026-11-16):
[checked len/lines_unique/escape_terminal calls](../reference/generic-string-builtins-v2.md)
now preserve core semantics with primitive result checks, temporary line-storage
admission and escape-output preflight. Experimental CLI/native feature 127 selects
this profile; older feature environments remain unchanged. to_string, array/host
builtin bridging, remaining E2/G13 work and other-host qualification stay open.


G13 E2 value-builtin checkpoint (2026-11-16):
[checked to_string/array_len calls](../reference/generic-value-builtins-v2.md)
now support polymorphic iterative display and generic array lengths. Output and
temporary formatting tasks are admitted under shared budgets. Experimental
CLI/native feature 255 selects the new profile; previous environments stay strict.
Append/concat/slice, host builtins, remaining E2/G13 work and platform gates stay open.


G13 E2 array-builtin checkpoint (2026-11-16):
[checked append/concat/slice calls](../reference/generic-array-builtins-v2.md)
now preserve snapshots, generic element identity, recursive numeric widening and
half-open bounds. Explicit IR conversion targets and whole-result copy preflight
use shared admission. Experimental CLI/native feature 511 selects the new profile;
previous environments stay strict. Host bridging and remaining E2/G13 gates stay open.


G13 E2 generic-editor checkpoint (2026-11-16):
[explicit generic editor diagnostics/tasks](../reference/generic-ide-v2.md)
now check unsaved original buffers and dispatch standalone Check/Run with the
generic compiler flag. Modes are mutually exclusive; unsupported navigation,
formatting and IR/JS tasks remain gated. Dependency/module/JS integration, broader
editor evidence and remaining E2/G13 gates stay open. Task Manager is unchanged.


G13 E2 generic-symbol checkpoint (2026-11-16):
[checked root declaration symbols](../reference/generic-symbols-v2.md)
now project complete original declarations into optional bounded JSON evidence and
generic editor outlines/declaration hover. Exhaustion returns no facts without
changing check success. Call/type/local references, formatting, modules/dependencies,
JavaScript and remaining E2/G13 gates stay open. Task Manager is unchanged.


G13 E2 call-navigation checkpoint (2026-11-16):
[checked direct-call references](../reference/generic-call-navigation-v2.md)
now retain original callee tokens and resolved function identities through optional
JSON evidence and generic editor Go to Definition/hover. References cover original
symbolic bodies once, not specialization duplicates. Local/type-use references,
formatting, package overlays and remaining E2/G13 gates stay open.

2026-11-16 checkpoint: [checked generic local navigation](../reference/generic-local-navigation-v2.md) adds parameter/local/match-payload definition and role hover. Type rendering and broader E2/G13 gates remain open.

2026-11-16 type-hover checkpoint: [checked generic binding types](../reference/generic-local-navigation-v2.md) now display symbolic/inferred types from retained owners with bounded iterative alias-expanded rendering. Type-use navigation and broader E2/G13 requirements remain open.

2026-11-16 header-type checkpoint: [checked generic annotation navigation](../reference/generic-type-navigation-v2.md) links user type names in function parameters/returns through sealed registry identities. Broader type-use navigation and E2/G13 requirements remain open.

2026-11-16 functional checkpoint: [generic source and locked-package tests](../reference/generic-testing-v2.md) adds bounded test discovery, all-root preflight and fresh sequential VM execution. Editor polish is deferred; dependency/module linking and remaining E1/E2/G13 requirements remain open.

2026-11-16 package-input checkpoint: [syntax-independent graph capture](../reference/captured-package-graphs-v2.md) retains guarded per-file dependency/module source snapshots and verifies compatible source locks without the legacy parser. This is not generic import linking; CLI dependency/module refusal remains active. Next: source-owned declarations, import visibility and cross-file identities.

2026-11-16 package-binding checkpoint: [generic declaration ownership and visibility](../reference/generic-package-declarations-v2.md) assigns table-owned per-file identities, validates registered module scopes and binds file-local direct dependency imports with private/export boundaries. This is not semantic/executable linking; generic CLI dependency/module refusal remains active.

2026-11-16 package-type checkpoint: [checked package types and signatures](../reference/generic-package-types-v2.md) feeds per-source visibility into existing registry/alias/definition/signature semantics with canonical nominal identities and cumulative admission. Scoped body compilation and unscoped function lookup remain gated; generic CLI dependencies/modules remain refused until executable linking is qualified.

2026-11-16 package-body checkpoint: [source-aware package calls and nominal operations](../reference/generic-package-bodies-v2.md) checks every body under its own visibility/type context. Explicit-library specialization/IR/VM tests execute imported generic calls, transitive helpers, records, enums/matches and propagation. Selected-root package/diagnostic and CLI/native integration remain open; generic CLI dependencies/modules remain refused.

2026-11-16 selected-root checkpoint: [owned generic package compilation and Check/Run/Lock](../reference/generic-package-compilation-v2.md) now executes registered local dependency graphs with locked snapshot custody, root-owned entry selection and cross-file runtime diagnostics. Multi-file Test/native Build and editor package integration remain gated; G13 is not complete.

2026-11-16 package-suite checkpoint: [generic package tests](../reference/generic-package-testing-v2.md) now run root-entry tests with registered module/dependency helpers over one immutable checked graph. Every selected specialization is preflighted; shared instance reservation bounds both passes, with fresh per-test VM policy and original-file failures. Locked test reports preserve lock priority and schema-one output. Multi-file native Build remains gated; G13 is not complete.

2026-11-16 native-package checkpoint: [versioned source/topology images](../reference/generic-package-native-v2.md) now build and run registered generic package graphs without project-file or Node.js startup dependencies. Embedded inputs retain logical ownership but cannot become guarded filesystem captures. Old single-entry image families remain unchanged. Linux native consumer execution and codec refusals are qualified; broader E1/E2/G13 and other-host/backend gates remain open.

2026-11-16 collections checkpoint: the [snapshot collections reference package](../../stdlib/snapshot-collections/README.md) supplies ordered generic maps/sets, explicit logical equality, missing-key Options and structured duplicate-record refusal. Locked Check/Run/Test and native consumer coverage exercise the actual library. This uses ordinary records/arrays and existing VM copy/work limits; raw record equality remains structural. Intrinsic std::Map/std::Set representation, operators and cost qualification remain open; C1 and G13 are not complete. Package body type-argument admission is bounded at 4096 aggregate; standalone and per-list limits remain unchanged.

2026-11-16 opaque-set checkpoint: [primitive-key runtime sets](../reference/runtime-sets-v2.md) now enforce private homogeneous storage, ordered snapshots, membership equality and atomic full-result node/text admission with finite helper work. This is an explicit Rust library foundation, not compiler/IR/VM/source/native admission. Opaque maps, collection type integration and full C1/G13 qualification remain open.

2026-11-16 opaque-map checkpoint: [typed runtime maps](../reference/runtime-maps-v2.md) now enforce private unique keys and exact primitive/array value shapes, preserve update order, compare associations and atomically admit complete descendant copies. Nominal record/enum ownership and compiler/layout/IR/VM/native collection admission remain open. This is an explicit Rust library API; C1/G13 are not complete.

2026-11-16 nominal-enum map checkpoint: [checked enum map leaves](../reference/runtime-maps-v2.md#checked-enum-value-leaves) now retain sealed environment-specific enum identities and copy all type/variant labels and payload descendants under atomic fuel and finite work. Foreign owners, forged text and transitive record payloads refuse. Record ownership and compiler/layout/IR/VM/native collection integration remain open; C1/G13 are not complete.

2026-11-16 sealed-record checkpoint: [checked record runtime ownership](../reference/runtime-record-owners-v2.md) now retains environment-specific record IDs, validates exact field/child owners before sealing, preserves IDs through snapshots/extraction and admits owned records in maps and enum payloads. Name-only records remain refused transitively. Record equality charges every candidate field-name search. Generic package-owner adaptation and collection type/layout/IR/VM/native integration remain open; C1/G13 are incomplete.

2026-11-16 generic-owner bridge checkpoint: [proof-carrying generic snapshots](../reference/generic-collection-values-v2.md) now capture actual package VM values with their own checked layout/declaration/type-argument evidence. Raw layout numbers and caller names cannot attach foreign proof. Imported homonyms, aliases and phantom arguments remain exact; full subtree copies use existing atomic generic snapshot units and finite work. Generic collection storage/source/layout/IR/VM/native integration remains open; C1/G13 are incomplete.

2026-11-16 generic-map storage checkpoint: [maps of checked generic package values](../reference/generic-collection-maps-v2.md) now enforce exact retained value types, preserve insertion order and owners across updates/get/remove/iteration, compare associations, and atomically admit all result descendants before copying. Root/dependency homonyms and phantom/foreign owners refuse. Type profiles remain finite and value copy units inherit the generic snapshot bridge. Source collection types/layouts/IR/VM and native qualification remain open; C1/G13 are incomplete.

2026-11-16 opaque-layout plan checkpoint: [checked collection layout plans](../reference/generic-collection-layouts-v2.md) now bind compiler-owned Map/Set constructors to exact checked arguments, enforce the primitive Key bound and instantiate the existing typed storage without caller-supplied key-domain guesses. This separate Rust plan API does not publish executable layout variants or enable source annotations. Native words and ordinary records remain unchanged; C1/G13 stay open pending source/layout/IR/VM/native qualification.

2026-11-16 intrinsic annotation checkpoint: [opt-in package collection semantics](../reference/generic-collection-types-v2.md) now registers opaque compiler-owned Map/Set headers and checks real source annotation arity, Key bounds, aliases/nesting and per-file dependency visibility under cumulative limits. Collection definitions are not records/enums; record literals/field projections refuse. Concrete forwarding rechecks, then executable layout publication explicitly refuses. Default compiler/CLI/native profiles remain unchanged; C1/G13 stay open for executable layouts/IR/VM/native qualification.

2026-11-18 collection layout-graph checkpoint: [opt-in closed Map/Set layouts](../reference/generic-collection-layout-graphs-v2.md) now retain opaque argument edges, intern aliases/exact owners and close recursive map-value graphs under the existing cumulative instantiation/layout/work limits. Real checked shapes bind typed storage even when producer layout numbers differ. Old planning remains strict; construction/IR explicitly refuses collection-bearing graphs, including phantom arguments. Actual collection builtin/IR/VM operations and native qualification remain open; C1/G13 are incomplete.

2026-11-18 initial collection execution checkpoint: [typed empty collection operations](../reference/generic-collection-operations-v2.md) now compile and execute real Sovra map_new/set_new/map_len/set_len through explicit package APIs, checked builtin IR and shared VM budgets/control. Contextual types preserve K/V without defaults; distinct flat empty values copy/compare/propagate normally. Ordinary execution and native encoding explicitly refuse the new profile. Nonempty operations, full language result admission and CLI/native qualification remain open; C1/G13 are incomplete.

2026-11-18 collection insertion checkpoint: [real-source map/set insertion](../reference/generic-collection-insertion-v2.md) now validates exact receiver/key/value types through body/concrete/IR checks, creates ordered flat nonempty snapshots, replaces without moving keys and retains duplicate set positions. Whole-result ownership/copy work is admitted before output copying, including real internal entry nodes. Equality compares associations/membership with charged key searches. Source lookup/removal/iteration and CLI/native qualification remain open; C1/G13 are incomplete.

2026-11-18 membership checkpoint: the explicit collection package profile now
checks and executes `std::map_contains` and `std::set_contains` with exact key
types, primitive Bool results and shared metered key searches. See
[collection operations](../reference/generic-collection-operations-v2.md).
Value lookup/Option ownership, removal, iteration, formatting and CLI/native
qualification remain open; C1/G13 are incomplete.

2026-11-18 typed lookup checkpoint: the explicit collection package profile now
checks and executes `std::map_get` and `std::set_get`, deriving sealed standard
Option result types from checked payload ownership. Missing keys produce real
None values; present values copy complete independent snapshots. The whole
wrapper/payload node and text cost is admitted before output copying. See
[typed collection lookup](../reference/generic-collection-lookup-v2.md).
Removal, iteration, formatting and CLI/native qualification remain open;
C1/G13 are incomplete.

2026-11-19 typed removal checkpoint: the explicit collection package profile now
checks and executes `std::map_remove` and `std::set_remove`, returning Some(updated
collection) on a hit and None on a miss. Surviving entries keep their order and
full ownership; remove/reinsert appends. Whole Option/container/descendant node
and text costs are admitted before copying. See
[typed collection removal](../reference/generic-collection-removal-v2.md).
Source iteration, formatting and CLI/native qualification remain open;
C1/G13 are incomplete.

2026-11-19 iteration snapshot checkpoint: the explicit collection package profile
now checks and executes `std::map_keys`, `std::map_values` and `std::set_values`,
returning exact typed insertion-order arrays with independent complete copies.
Generated array proofs and whole-result node/text/copy work remain bounded; see
[collection iteration snapshots](../reference/generic-collection-iteration-v2.md).
Map entry-pair snapshots, formatting, CLI/native admission and consolidated
backend/host/resource qualification remain open; C1/G13 are incomplete.

2026-11-19 public entry snapshot checkpoint: the explicit collection package
profile now checks and executes `std::map_entries` as insertion-order arrays of
sealed `std::MapEntry<K,V>` records with public key/value fields. Actual record
wrappers and all descendant/label copies are admitted together; typed ownership,
Key bounds and source/default-profile/native gates remain explicit. See
[public map entries](../reference/generic-collection-entries-v2.md).
Collection formatting, CLI/native admission and consolidated backend/host/resource
qualification remain open; C1/G13 are incomplete.

2026-11-19 collection display checkpoint: explicit collection package execution
now formats maps/sets and nested values in insertion order with checked private
entry ownership and exact two-pass output sizing. Existing temporary-task/node,
text, work and cooperative controls remain active; raw String/scalar and public
record display conventions remain unchanged. See
[collection display](../reference/generic-collection-display-v2.md).
CLI/native admission and consolidated backend/host/resource qualification remain
open; C1/G13 are incomplete.

2026-11-19 collection image checkpoint: separate explicit encoding/decoding and
execution APIs now admit exact package feature word 1023, rechecking all embedded
collection source with finite compiler/VM policies and memory-only ownership.
Word 511 and the ordinary compiler/encoder/decoder remain strict; feature flips,
source errors and resource refusal never select a fallback. See
[collection package images](../reference/generic-collection-native-images-v2.md).
Installed CLI/native backend/runtime dispatch and generated-executable launch
qualification remain open, along with consolidated backend/host/resource gates;
C1/G13 are incomplete.

2026-11-19 collection command/launch checkpoint: `--experimental-generics
--experimental-collections` now opts package Check/Run/Test/native Build into
collection semantics and exact word-1023 runner dispatch. Collection suites retain
all-root preflight and finite fresh per-test VM policies. Single-entry packages
use the new package profile; default and word-511 paths remain unchanged. See
[collection CLI/native workflow](../reference/generic-collection-cli-v2.md).
Linux collection launch, 71 generic integration targets, all 600 library tests
and eight IDE scripts pass. Byte-key/additional-host integration and
E2/D1/H1/H2/Q1 completion remain open. G13 is incomplete.

2026-11-19 D1 byte-runtime checkpoint: explicit Rust checked Byte/immutable Bytes
APIs now provide typed scalar conversion, complete buffer/array snapshots,
index/slice/concat, strict lowercase-hex/standard-base64/UTF-8 codecs and original
input offsets. Packed physical bytes retain one logical Byte node each; whole
outputs and finite helper work/cooperative controls are admitted explicitly. See
[byte runtime foundation](../reference/runtime-bytes-v2.md).
Source type/layout/IR/VM and Result-wrapper integration, Byte keys, CLI/native
admission and JSON remain open; this does not complete D1 or G13.

2026-11-19 compiler data-type checkpoint: the explicit data package semantic API
now registers sealed zero-argument std::Byte/std::Bytes headers, checks Byte Key
bounds through aliases/calls/concrete forwarding and preserves exact primitive,
buffer, phantom and imported ownership. Empty parameter scopes are explicitly
validated against registered headers. See
[compiler data annotations](../reference/generic-data-types-v2.md).
Default/collection-only APIs still reject data annotations; actual data layouts
explicitly refuse. Source operations, owned Result costs, runtime Byte keys,
CLI/native data admission and JSON remain open; D1/G13 are incomplete.

2026-11-19 data layout checkpoint: the explicit data layout planner now publishes
closed Byte/Bytes metadata with exact declaration identities, a sealed Byte edge
inside Bytes, canonical alias interning and bounded recursive/phantom collection
graphs. Existing construction and IR paths reject data-bearing tables independently;
Byte storage plans refuse instead of selecting Int keys. See
[checked data layout graphs](../reference/generic-data-layouts-v2.md).
Checked byte operations, owned Result/error admission, runtime Byte keys, CLI/native
data profiles and JSON remain open; D1/G13 are incomplete.

2026-11-19 executable data checkpoint: the explicit data package profile now
executes Byte Result conversions, immutable Bytes construction from UTF-8 text
and Byte arrays, length/equality/display, checked Result<Byte,Error> access and ordered
concatenation. Full result ownership, source/concrete/IR identity checks and
shared VM admission/control apply. See
[checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source slice/to-array/codecs, CLI/native data admission, JSON and remaining
phases are open. This checkpoint does not close D1 or G13.

D1 follow-up: qualify recoverable Result/error construction for source text/array
buffers; the existing construction helpers still use VM capacity refusal.

2026-11-19 constructor Result checkpoint: `bytes_from_text` and `bytes_from_array`
now return sealed `Result<Bytes,Error>`, including recoverable owned capacity
errors. Explicit VM callers may lower the shared `max_byte_buffer_bytes` ceiling
from 16 MiB to zero. Empty buffers still succeed; all constructor/concat success
and error paths preflight full ownership before output allocation. Source
fixtures now use explicit propagation, and constructor Result/error qualification
listed at the preceding checkpoint is complete. See
[checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source slicing/to-array/codecs, CLI/native data admission, JSON and remaining
G13 phases remain open. G13 is incomplete.

2026-11-19 byte slice checkpoint: the explicit data package compiler/VM now
executes `std::bytes_slice(Bytes,Int,Int) -> Result<Bytes,Error>` for half-open
raw byte ranges, including empty slices and slices within UTF-8 sequences.
Negative/reversed/out-of-range bounds return owned errors carrying the original
signed index. Source/concrete/IR stages independently validate exact operands
and sealed results; whole success/error ownership is admitted before allocation.
See [checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source to-array/codecs, CLI/native data admission, JSON and remaining G13 phases
stay open. G13 is incomplete.

2026-11-19 Byte array conversion checkpoint: the explicit data package
compiler/VM now executes `std::bytes_to_array(Bytes) -> Result<[Byte],Error>`
with exact sealed Byte element ownership, independent mutable array snapshots,
raw-byte preservation and complete N+2-node/13-label-byte output admission.
Source/concrete/IR validation and generic forwarding retain distinct buffer,
array and Byte/Int identities. See
[checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source codecs, CLI/native data admission, JSON and remaining G13 phases stay
open. G13 is incomplete.

2026-11-19 source hex checkpoint: the explicit data package compiler/VM now
executes `bytes_to_hex(Bytes) -> Result<String,Error>` and
`bytes_from_hex(String) -> Result<Bytes,Error>`, using lowercase encoding, strict
case-insensitive hex digits, original byte offsets and recoverable capacity
errors. It shares the Rust helper digit rules/capacity-validation order while
preflighting complete language Result/error ownership before output allocation.
Sealed String/Bytes Result schemas are independently rechecked. See
[checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source base64/UTF-8 codecs, CLI/native data admission, JSON and remaining G13
phases stay open. G13 is incomplete.

2026-11-20 source Base64 checkpoint: the explicit data compiler/VM now executes
`bytes_to_base64(Bytes) -> Result<String,Error>` and
`bytes_from_base64(String) -> Result<Bytes,Error>`. Standard RFC 4648 alphabet,
canonical padding/discarded bits, original byte offsets and complete owned
Result/error preflight are enforced. The packed helper and flat VM share one
strict validator while preserving each caller's control/work errors. See
[checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source UTF-8 decoding, CLI/native data admission, JSON and remaining G13 phases
stay open. G13 is incomplete.

2026-11-20 source UTF-8 checkpoint: the explicit data compiler/VM now executes
`bytes_to_utf8(Bytes) -> Result<String,Error>` with RFC 3629 validation, original
invalid-sequence byte offsets and no replacement/normalization. Valid BOM, NUL,
combining sequences and encoded U+FFFD are preserved. Stack-only validation
precedes complete Result ownership admission and exactly sized String copying;
both passes retain cooperative controls. See
[checked immutable Bytes execution](../reference/generic-bytes-execution-v2.md).
Source byte-codec execution is covered; CLI/native data admission, JSON and
remaining G13 phases stay open. G13 is incomplete.
