# G13 completion contract

Status: **Approved by the user; implementation is in progress and G13 is not complete.**
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
