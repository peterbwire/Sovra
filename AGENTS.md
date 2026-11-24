# Sovra project context

Sovra (`svr`, `.svr`) is a general-purpose language and software ecosystem for
humans and AI agents. Prioritize correctness, coherent semantics, simplicity,
developer experience, machine-readable structure, then performance/ecosystem
breadth. Nova is a future AI engineering system; language foundations come first.
Business entities belong in libraries/examples, not core language primitives.

## Long-term platform goal

Sovra must grow into a language for mobile (Android/iOS), desktop
(Windows/macOS/Linux), web/browser, servers, command-line tools and other
environments. Read `docs/PLATFORM_VISION.md` as the durable product direction.
Keep compiler semantics, checked representations, diagnostics and reusable
application logic portable; isolate OS/device/UI/lifecycle/packaging differences
behind explicit backend or host boundaries. Do not let the current Task Manager,
Windows host, browser editor or VS Code extension narrow the language's purpose.
Distinguish intended targets from implemented and verified support: mobile
runtime/UI/packaging remains planned. Preserve established milestones and release
gates while building toward this broader goal.

## Start here

Current experimental G13 checkpoint (2026-11-23): the explicit data profile now
implements executable Byte/Bytes/codecs, Byte keys, strict JSON, word-2047 native
packages and version-3 explicit filesystem/date/clock/timer/process adapters.
`fmt --experimental-generics` now formats generic/data/propagation source and
package graphs; generic LSP/VS Code formatting returns guarded edits. Explicit
`sovra.experimentalGenericPackageTasks` now opts Check/Run into compiler-selected
registered/dependency graph tasks. A separate
`sovra.experimentalGenericPackageDiagnostics` opt-in now checks unsaved registered
package graphs without saving/executing source. Checked generic package function
calls now navigate through canonical bindings and fingerprinted source snapshots;
checked type annotations now navigate through the scoped registry without erasing
alias identities. Checked package locals now retain typed hover and per-function
definitions; explicit local type annotations also navigate through their original
scoped fragments. Explicit call type arguments and record/enum constructor
heads/arguments also navigate through checked expression ranges. User-record
field reads and constructor labels now navigate through nominal record/field
identities. Full frozen-source Linux qualification now passes 2,273 Rust tests
and 16 Node scripts, including exact Rust 1.74.0 and Clippy. Subsequent IDE-only
changes pass all 16 Node scripts and all six actual VS Code 1.106.0 host modes,
including generic packages and restricted workspaces. G13 is complete for the
documented experimental Linux profile. Additional-host/runtime/release acceptance
remains separate. G14 now has an owned Rust loopback socket host,
with explicit port grants, bounded nonblocking I/O and independent TCP-client
tests. An explicit owned version-four network host now lets checked `.svr` data
programs bind/accept/read/write/close, with scoped token custody and cleanup on
VM success/failure. Bounded external v3/v4 policy loading now enables explicit
CLI/native loopback byte servers with no embedded grants, project or Node startup
dependency. A pure borrowed fixed-length HTTP/1.1 request parser now validates
framing, Host/field syntax, partial input and lowerable limits. Bounded response
construction owns framing, HEAD/bodyless rules and field injection refusal.
`http.receive`/`http.respond` now wire these to execution-owned sockets with
aggregate deadlines and one-request state. A real Sovra HTTP/JSON example passes
independent CLI/native client acceptance. A bounded incremental chunked-body Rust
decoder now shares HTTP head work and aggregate socket deadlines through
`http.receive`; independent CLI/native Unicode chunked JSON clients are covered
(see `docs/reference/http-chunked-v4.md`). Trailer sections now use bounded validated discard without header merging,
with known security-field refusal (see `docs/reference/http-trailers-v4.md`).
The host now supports bounded 100-continue with early 417 refusal for unsupported
expectations and shared deadline/I/O accounting (see `docs/reference/http-expect-v4.md`).
Explicit optional HTTP Host/Origin access policies now check exact grants before
100/body handling, with bounded 403 refusal and shared work admission; see
`docs/reference/http-access-v4.md`. Policies without this object retain prior behavior. Optional paired exact
method/raw-target lists now further narrow access before Continue/body handling;
query/escape/case variants do not select a normalization fallback.
Trailer integrity/application access, authentication, full protocol, production
server controls and wider qualification remain open. Continue under `docs/design/G14_COMPLETION_CONTRACT.md`. Consult
`docs/G13_QUALIFICATION.md` and `docs/reference/generic-formatting.md` for current
evidence and remaining gates. Historical checkpoints below do not reopen these
implemented slices or establish G13/release/platform completion.

Use `docs/LANGUAGE_MATURITY_AUDIT.md` as the current repository-wide capability,
remaining-work and contributor-task index. Its linked
`docs/18_TASK_COMPLETION_CHECKLIST.md` expands the same G01–G18 IDs to cover the
user-requested toolchain/native/stdlib/HTTP/UI/package goals. Preserve those IDs
and completed foundation scopes; unchecked product criteria remain open.
Older dated checkpoints are historical
evidence, not a substitute for current source and exact-platform verification.

User workflow rule (2026-11-24): at every G-task transition, push the completed
work to GitHub and open a pull request before starting the next G task. G14 has
its own review branch. PR #3 is merged into main; PR #4 was closed unmerged.
The continued G14 review targets main and includes all unmerged G14 work.
Continue incremental work within that task.

Current sequencing (2026-11-11): review and qualify the compiler/runtime, shared
UI, developer tooling and Windows packaging before further Task Manager feature
work. Task Manager changes are on hold until those foundations meet the Windows
readiness gates in `docs/WINDOWS_FOUNDATION_REVIEW.md`. Existing application
regressions may still run; do not expand or redesign its behavior during this
foundation work. Windows is the immediate qualification target within the broader
platform goal, not the language's only intended host.

Read `docs/CODEX_HANDOFF_ASSESSMENT.md`, `docs/spec.md`, `docs/roadmap.md`, and
`docs/production-upgrade-flow.md` before substantive work. Inspect Git status
and preserve existing changes. Continue the implementation incrementally;
do not restart it, silently change syntax, or reorganize it for appearance.

## Architecture and current milestone

Single Rust 2021 Cargo package `sovra`, binary `svr`; declared minimum Rust 1.74.
`src/main.rs` delegates to `src/cli.rs`. Public compiler stages are in
`src/compiler/`: lexer -> parser/AST -> semantic -> IR -> interpreter/backend,
with diagnostics and Rust-owned stdlib. `project.rs` is a separate manifest and
line-scanning checker, not full application parsing or type-checking.

The repository calls M0-M11 complete and M12 project checking in progress.
The assessment recommends existing-subset correctness hardening first. Module
bodies, numeric widening/overflow, token ranges and straight-line returns now
have regression coverage. ADR 0002 is approved and implemented: every function
parameter requires an explicit annotation (`E3014`); local `let` inference and
the default Unit return type remain. Named-type resolution is still incomplete.
Consult `docs/FULL_DEVELOPMENT_STATUS.md` and `docs/adr/` for active work and decisions.
Do not renumber the established milestones.
`run` executes one source file or local package directory; `build` emits IR or JS;
`check` accepts a source
file or project. `check --format json` emits versioned diagnostic reports; see
`docs/reference/check-json.md` for null locations and the project-scan scope.
`new`, interpreter `test`, `doctor` and standalone `fmt` are implemented too.
`init`, `repl`, `install`, `update` and `doc` remain reserved commands.

`tests/cli.rs` covers subprocess behavior; unit tests sit beside Rust modules.
`examples/hello-world` is executable. `examples/fielddesk` is proposed application
syntax with partial wiring validation. Passing its check does not make it runnable.
`examples/modules` and `docs/course/modules.md` cover inline modules;
`examples/functions` and `docs/course/functions.md` cover typed signatures and
local inference. There is no separate runtime crate or package manager yet.
An experimental Node stdio language server and VS Code extension live in
`toolchain/ide/`; they reuse standalone compiler JSON checks for unsaved buffers.
Opt-in standalone `check --format json --symbols` supplies checked function
hover/definitions/outlines, inferred-local hover and scope-aware parameter/local navigation. Semantic
completion, type navigation and
package-root loading are planned.
The additive `compiler::checked::CheckedUnit` API owns immutable standalone
checked syntax/structural IDs and can opt into bounded parameter/local/value
type facts recorded during semantic checking. Flow-enabled constructors also
record scoped function/value uses and numeric coercion edges with explicit
limits. See ADR 0041. Opt-in CLI `--symbols` and LSP/VS Code now project those
standalone identities/types, including inferred-local hover. Explicit lowering-ready
constructors and `ir::lower_checked` now consume retained contexts/types/targets;
Opt-in `compile_with_checked_evidence` also supports imports with canonical
package identities and cumulative metadata/shared emission budgets.
Standalone CLI run/build/native emission and language tests now use
`compiler::compile_program` with active bounded evidence and E3025 exhaustion.
Ordinary package callers now use cumulative `PackageEvidenceLimits::DEFAULT`
and checked lowering too; explicit legacy package lowering is a reference path,
never an exhaustion fallback. G08 portable behavior/Linux qualification is complete (ADR 0041); next is G09
package-aware LSP/navigation. Its first compiler-only slice supports bounded
borrowed existing-entry overlays and bounded `check --format json --overlays-stdin`
transport plus compiler-owned package context (ADR 0042). LSP workspace
diagnostics now invalidate all related document generations on edits/close/watch
events. Checked imported function navigation now uses source-owned declarations
and fingerprint-verified buffers. Package tasks now resolve compiler-owned roots,
save graph sources and refuse dirty manifests; full package Check explicitly uses
`--executable-package`. G09 bounded existing-entry scope is Linux-qualified,
including real VS Code package tasks. G10 now enables guarded Linux formatter writes with failure/permission/identity
qualification. Import-aware single-file formatting now shares the compiler package parser.
Read-only `fmt --package --check DIR` now checks bounded canonical graph entries
with manifest/source conflict checks. Guarded `fmt --package --write DIR` now preflights/stages every selected entry
before sequential per-file publication. Late failures report partial completion
without rollback. G10 package behavior is Linux-qualified; macOS writes and
Windows execution of new package/import slices remain platform gates.
`scripts/qualify-formatter.mjs` now retains exact-host compiler/source hashes,
Rust/CLI logs and failure results; CI uploads reports on all three hosts. macOS
reports verify disabled writes, not supported writes. Collect actual platform
reports before closing those gates. Formatter snapshot/conflict reads now
validate opened handles before reading; Linux x86/x86_64/aarch64 additionally
use nonblocking/no-follow opens, and Windows rejects opened reparse handles.
Linux FIFO/link/regular substitution tests cover the open race. G11 now retains bounded executable alias/record AST nodes with explicit unparsed
ranges through `application::syntax::parse`; this library API never establishes
full application validation. `syntax::parse_callables` now retains source-owned
executable body ASTs, and `syntax::parse_services` retains containers/prototypes
with explicit remaining coverage gaps. These are library-only APIs, never type
checking or runtime authorization. `syntax::parse_file` now retains dotted
application imports; opt-in `check_service_calls_with_syntax` returns separate
bounded coverage from the same source snapshots, preserving semantic diagnostics.
`syntax::parse_entry_blocks` now separately retains app containers and existing
route/page/schedule/service/data/auth binding forms through scanner grammar
helpers; unknown members/nested forms remain explicit gaps. Default APIs/CLI
remain partial and unchanged. `syntax::parse_combined` now composes entry and
declaration coverage using private original-token filtering, preserving unknown
ranges and Unicode spans. Additive `check_service_calls_with_combined_syntax`
now shares source snapshots and quota accounting with the declaration-only
projection, preserving all checker diagnostics including E4096. Additive
`check_local_functions` now uses core semantic checking for self-contained
ordinary functions beside entries; imports/services/tasks explicitly require
other context. Entry bindings and unknown fields remain outside that local-function scope.
Additive `check_entry_paths` now shares scanner route/page shape and duplicate
rules with exact member spans and bounded diagnostics; targets/argument binding
remain outside that path-only scope. Additive `check_local_route_targets` now
resolves unqualified same-source ordinary function names to retained indices and
exact target tokens; missing names use E4033. Ambiguous/import/service/task or
qualified context is explicitly unavailable. Body/argument/path validity remains
separate. Additive `check_route_placeholders` now requires explicit same-name
parameters for local route placeholders (E4144), retaining missing-target/shape
errors and bounded diagnostics. Extra parameters, HTTP types/decoding and repeated
placeholder semantics remain outside scope. Additive `check_local_schedule_targets`
now shares scanner duplicate/missing rules (E4071/E4072) and retains ordinary/task
declaration identities without scheduler execution or body/return validation.
Independent `syntax::parse_auth_blocks` now retains bounded auth/policy-header
nodes while every where clause and provider/user/nested field remains explicitly
unparsed. Additive `syntax::parse_with_auth` and
`check_service_calls_with_auth_syntax` now compose this metadata using original
tokens and shared source snapshots/quotas, preserving every condition gap and
all existing semantic diagnostics. Older APIs remain strict; no policy is
authorized. Independent `parse_data_declarations` now retains model/enum headers
with all nonempty bodies unparsed; `check_local_policy_models` resolves explicit
same-source model references with shared E4081 diagnostics, without typing bodies
or granting model operations/permissions. Independent `parse_render_declarations`
now retains page/view headers, explicit unresolved annotations and opaque bodies,
never default Unit results. `check_local_page_targets` links unique local Page/
View identities with shared E4042 errors; ordinary functions do not satisfy them.
Additive `check_page_placeholders` now requires explicit same-name Page/View
parameters (E4145), retaining missing-target/shape failures without decoding or
render typing. Additive `parse_structured` and
`check_service_calls_with_structured_syntax` now integrate auth/data/render headers
with entries/declarations using original tokens and shared snapshot quotas,
preserving all opaque body/guard gaps and E4096. Earlier APIs remain strict.
The user approved option A in `docs/design/M12_MODEL_FIELDS.md`. Additive
`check_model_primitive_fields` now retains model-owned newline/comma fields and
checks primitives/local primitive aliases with bounded duplicate/unknown diagnostics.
Unsupported contexts expose no partial facts; default checking remains unchanged.
Additive `check_service_calls_with_model_fields` now exposes same-snapshot,
file-owned field outcomes under cumulative file/source/token/field/diagnostic
bounds, preserving the original checker and E4096. Additive
`check_structured_local_functions` now reuses core ordinary function semantics
beside opaque auth/data/render headers; excluded-name/import/service/task context
is explicitly unavailable and older APIs remain strict. Additive
`check_service_calls_with_local_functions` now retains these file-local outcomes
from shared checker snapshots under cumulative function/diagnostic bounds,
without cross-file linking or suppressing E4096. Additive
`check_structured_route_placeholders` now composes existing local ordinary route
target/path/placeholder rules beside opaque auth/data/render headers, preserving
all gaps and refusing excluded-header target collisions. Older APIs stay strict.
Additive `check_service_calls_with_routes` now retains file-owned route outcomes
from shared snapshots under cumulative route/placeholder/diagnostic bounds,
counting nested target diagnostic copies without changing E4096/default checking.
Current sequencing: the user moved to G12 while keeping G11 partial, and approved
`docs/design/G12_MODULES_AND_LOCKS.md` (recommended A/C/D and scope/limits).
Additive `packages::inspect_source_inventory` now admits bounded module mappings
and retains deterministic source ownership/path/observed-size metadata. It does
not read/parse module sources, compile them or implement locks. Unix identity
checks reject hard-link aliases; other hosts explicitly report unavailable checks.
Existing compile/tooling modes still reject module manifests rather than ignore
registered files. Additive `packages::load_package_sources` now captures bounded
source-owned bytes/ASTs/imports, checks registered headers and duplicate module
names, and preserves original spans. Its guarded reader currently requires Linux
x86/x86_64/aarch64 plus procfs; other capture hosts fail explicitly. It does not
resolve semantic imports/types or link/execute modules by itself. Additive
`compile_registered_primitive_functions` now checks all source-owned bodies and
links primitive/local-primitive-alias callable interfaces through checked lowering.
The independent multi-file consumer prints 49 in interpreter/Node JS. Imports stay
file-local/direct, private helpers stay private and resource exhaustion has no
unchecked fallback. User record interfaces remain explicitly unavailable; default
Strict context/overlay APIs still refuse module maps. Additive
`compile_registered_records` now
supports nominal records, public record imports, aliases, construction/returns
and checked coercions. Fields now reference visible declared records through
canonical IDs without layout expansion; public private-type leaks are rejected.
Other nonregistered field contexts remain unavailable (E4164).
Root declarations and a module literally named root now have distinct internal
record IDs. Recursive record declarations retain existing core behavior without
new allocation/default/ownership semantics. CLI check/run/build/test now detect
module registration anywhere in the declared dependency graph and use checked
registered compilation; entry-only/application behavior remains separate. Test
discovery stays root-only. Context/overlay/navigation/editor integration still explicitly refuses registered
graphs. Parsed manifest snapshots now share guarded capture with sources; explicit
source locks, guarded Linux publication and locked CLI check/run/build/test consume
those same owned inputs. Package formatting now includes registered inventory files
within its existing 128-source profile, leaving locks untouched. Existing public
standard result records cross registered signatures/aliases/fields. Next: complete
context/overlay/navigation/editor integration, remaining type contexts and exact-host
qualification under the approved plan. The user approved the first G13 contract in
`docs/design/G13_COLLECTIONS_AND_DATA.md`: structural `[T]` annotations and
`std::array_len`, preserving ADR 0012 value semantics. It is now implemented across
core parsing/checking/lowering, primitive and visible record package interfaces,
formatter syntax and explicit local symbol display. Type/alias expansion is bounded
at 128 structural levels; written/copied wrappers consume AST/interface budgets.
General collections, sum types/errors, binary and JSON APIs remain planned; their
fundamental decisions still require a documented pause. Do not ask again for the
approved array contract. Older partial application type inspection still refuses
array declarations until its own identity/privacy contract is integrated.
G13 runtime hardening now uses iterative Rust value clone/runtime-owned cleanup and
both engines' compound display. Private guards handle frame/temporary/argument and
replacement/error cleanup without adding Drop to the public Value enum. Explicit
Value::drop_iterative supports caller-owned cleanup; ordinary external Drop/Debug/
PartialEq keep their previous contracts. Deep child regressions cover 16,000 levels,
and source-level recursive records preserve copy/equality/display. See
`docs/reference/runtime-values.md`. Allocation/output/work/interruption policy and
host qualification remain open; no process sandbox or complete G13 is claimed.
G13 now also has additive ExecutionLimits Rust APIs and bounded JS generation for
optional per-invocation IR instruction and printed UTF-8 byte allowances. Counters
are shared across callees; LF/Unicode count before output, and exhaustion cleans
runtime values iteratively without fallback/rollback. Existing APIs/CLI/native
policy remain unchanged. See `docs/reference/execution-limits.md` and
`docs/design/G13_RUNTIME_BUDGETS.md`. Physical heap accounting,
full-work/hard interruption, native serialized policy and default profiles remain open;
review a concrete accounting contract before implementing those broader policies.
Read-only Value::logical_footprint and checked ValueFootprint aggregation now
measure value nodes, owned UTF-8 text (record labels included), and display bytes
without copying payloads or formatting full compound output. No allocation fuel
is enforced by this API. `docs/design/G13_MATERIALIZATION_FUEL.md` now contains
recommended cumulative accounting, complete operation/host-effect costs and an
explicit accounting decision. The user has now approved option A and the complete
cost/effect table. Model-v1 cumulative logical node/text fuel is now implemented
through opt-in MaterializationLimits/RuntimePolicy APIs in Rust and JavaScript.
See docs/reference/materialization-limits.md. Do not ask again for this contract. Preserve existing defaults and do not equate logical fuel with
physical heap/RSS or alter native image/default policy implicitly.
The user approved the Rust-only cooperative cancellation/elapsed-control contract
in `docs/design/G13_COOPERATIVE_INTERRUPTION.md`. Implement and verify that
contract without asking again. Opt-in APIs are now implemented; see
`docs/reference/execution-control.md`. Preserve checkpoint/error order, existing defaults
and JavaScript behavior; hard interruption and JS transport remain separate.
`docs/design/G13_JAVASCRIPT_WORKER_CONTROL.md` now contains a concrete Node-only
worker/atomic-flag transport contract. The user has explicitly approved it;
its opt-in generator is now implemented; see
`docs/reference/javascript-worker-control.md`. Do not ask again. Preserve existing defaults and
inherited process-stdio boundaries. Browser transport and hard isolation remain separate.
The user directed continuation of `docs/design/G13_ARRAY_SLICE.md` after review.
Its half-open checked bounds, dependent snapshot-array result and explicit fuel
charge are now implemented; see `docs/reference/array-slice.md`. Do not ask again
for this operation. Preserve earlier array/fuel/control contracts.
The user approved `docs/design/G13_ARRAY_GROWTH.md` for append/concat together.
Its first-directed type/coercion, length and fuel contract is now implemented;
see `docs/reference/array-growth.md`. Do not ask again for these operations.
Existing slicing/resource approvals remain valid.
The user approved `docs/design/G13_SUM_TYPE_SYNTAX.md`: an additive bounded
single-enum syntax API only. Its grammar and admission table are now implemented;
see `docs/reference/enum-syntax.md`. Do not ask again for this syntax API.
Executable enums, match, generics, public AST/IR changes and
standard-result migration remain separate decisions.
The user requested finishing G13. `docs/design/G13_COMPLETION_CONTRACT.md` groups
the remaining executable enum/generic/container/data/host choices. The user has
now approved that completion contract. Implement its phases in dependency order
without requesting approval per builtin. Preserve existing public APIs, checked
budgets and scope limits; do not count metadata/scaffolding as executable features.
Initial E1 now has compiler::v2 nominal enum/constructor/coverage facts and
standalone match syntax and closed-scope ordinary arm checking; see
`docs/reference/enum-types-v2.md`, `docs/reference/match-syntax-v2.md` and
`docs/reference/match-bodies-v2.md`. Payload immutability, type checks and
conservative arm returns are checked against supplied locals. The separate
`docs/reference/enum-function-v2.md` API now derives one function's locals from
source and validates direct-match return coverage. The opt-in
`docs/reference/callable-function-v2.md` API adds canonical constructors, self-calls
and bounded call/context facts using private signatures and core semantic hooks.
Older source/closed-body APIs retain standard-only calls. The separate
`docs/reference/typed-callable-v2.md` opt-in now records unit-owned core expression
IDs/types, call/context value links and array-element conversion contexts under
additional expression/type quotas. Older callable quotas remain unchanged. The
`docs/reference/scoped-callable-v2.md` opt-in now records declaration/read/write/
match-scrutinee identities and follows core/arm lexical scopes with separate binding
quotas. The additive `docs/reference/callable-mapping-v2.md` mapper now builds a
borrowed preorder syntax/fact graph with mapping-local IDs and refuses ambiguous
locations, missing facts, shape/type/ownership or coverage disagreements. It does
not itself emit IR instructions. `docs/reference/expression-ir-v2.md` now adds
ordered expression register emission/validation, checked operand conversions and
explicit deferred statement contexts. Its roots are not an executable function
stream. `docs/reference/function-ir-v2.md` now emits single-function stores, scopes,
branches/loops, match snapshot/dispatch/payload/release and returns. Bounded
definite-availability validation intersects availability/tag facts, unions possibly
live resources and checks tag guards/scope joins; the separate runtime API is described below.
`docs/reference/runtime-values-v2.md` now provides flat owned value arenas, nominal
enum construction/tag/extraction/equality/display and the enum/value subset of
model-v2 atomic logical fuel. Deep ordinary drop is safe without altering v1 Value.
`docs/reference/runtime-vm-v2.md` now executes sealed function IR with an opt-in
Rust VM, checked arithmetic/bounds, iterative self-recursion, shared budgets and
cooperative checkpoints. Print/println/to_string/UTF-8-byte len, split_once/parse_int/lines_unique/escape_terminal
and array_len/slice/append/concat are adapted; unsupported
standard calls refuse before execution, including unreachable calls. Output is
memory capture only. Additive execute_with_arguments explicitly borrows program
arguments and captures exit status for arg_count/arg/set_exit_code; ordinary execute
still refuses them. No process arguments/status or I/O are accessed automatically. Additive
execute_with_input enables read_line via a narrow explicit RuntimeVmInput host,
minimum preflight and full post-read admission/checkpoints. Reads may be consumed
before later refusal; blocking input is not interrupted. Additive
execute_with_services/RuntimeVmServices now independently grants input/files via
narrow traits. RuntimeVmFiles adapts read_text/write_text/write_text_if; explicit
ProcessHost delegates its existing filesystem operations. Minimum preflight and
post-call control/full-result admission preserve effects on later refusal; no
default file authority, confinement, rollback or hard interruption is introduced.
Earlier APIs keep file refusals. `docs/reference/entry-source-v2.md` now adds
a source-owned root unit with aliases/records/enums and exactly one checked entry,
original token coordinates and bounded lowering/VM execution. Additional functions,
modules/imports, nested matches and CLI/backend/package/tooling integration remain
explicit refusals/open; do not describe this as complete program support.
`docs/reference/program-source-v2.md` now adds checked root multi-function source
and canonical per-request function IDs/call links, including forward/mutual/self
calls. All bodies are checked; cumulative signature/text/evidence/link caps apply
alongside per-function type/syntax quotas. IR mapping validates those identities;
standalone VM preflight-refuses program calls. The separate
`docs/reference/program-execution-v2.md` API now lowers/executes the whole checked
root table with callee-owned graphs/scopes and one shared invocation policy,
services/status and iterative frame stack. All functions are preflighted before
effects; cumulative graph quotas apply before emission. Lowering precedes the
VM clock. Modules/imports, nested matches, CLI/backend/package/tooling and broader
qualification remain open; G13 is partial.
`docs/reference/recursive-match-v2.md` now enables recursive matches directly
inside program match arms, with separate full versioned bodies, cumulative
arm/header/scope admission, preorder facts and nested snapshot/release IR. Earlier
standalone/single-entry APIs stay strict. `docs/reference/control-match-v2.md`
now extends checked-program bodies through if/else-if/else/while with original
headers, complete recursive bodies, Bool checks, lexical scopes and existing IR.
Long unbraced else-if chains have an explicit E2007 statement-depth guard.
Broader CLI/backend/package/tooling integration and qualification remain open.
Full host/standard adapter and runtime qualification,
backend parity and cross-function calls,
executable lowering,
nested matches,
complete programs, lowering/runtime and package/tooling remain open. These APIs execute enums in the opt-in Rust VM; default CLI/backend enum support remains open.
G11 contextual/model/enum/policy/render checking remains open. G11 completion is
not a prerequisite to beginning G12. Do not ask again for the approved contracts.
Typed/context validation remains under ADR 0009; render-result/UI, qualified binding/HTTP and
policy-context designs, model/page bodies and platform gates stay open.
Windows/MSRV/release qualification remains separate. Preserve existing public
`TypedProgram`/stage behavior while continuing that migration.

For library/package work, read `docs/LIBRARY_ECOSYSTEM_ASSESSMENT.md` and
`docs/REAL_HTTP_APPLICATION_PLAN.md`. ADR 0010 proposes local dependencies and
executable imports and is accepted. `project::packages::resolve` implements the
local graph only; `project::packages::compile` parses, checks and links package
entry files. `use alias::module;` exposes exported members of direct dependencies;
calls use `alias::module::function`. CLI dependency-package check/run/build use
this compiler. Directories without dependencies retain partial application checks.
See `docs/course/local-packages.md` and `examples/local-packages` for tested usage.
`examples/library-foundations` is tested
inline composition, not cross-file/package consumption. Preserve these boundaries.

## Commands and verification

```text
cargo build
cargo fmt --all -- --check
cargo check
cargo test --no-run
cargo test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- run examples/hello-world/main.svr
cargo run -- check examples/fielddesk
cargo run -- check --format json examples/functions/main.svr
cargo run -- build --emit js examples/hello-world/main.svr
```

CI is configured for Linux, Windows and macOS checks plus Rust 1.74 compile
qualification. It includes formatting, strict Clippy and full-target tests.
Install Node.js for backend execution tests (CI uses Node 22).
Windows CLI tests can return early on Application Control error 4551: inspect
output and report actual execution separately from compilation or skipped checks.
Do not weaken tests or bypass OS policy to obtain a green result.

## Change discipline

- Preserve compiler stage APIs and the Rust-owned trusted implementation.
- Use rustfmt (100-column setting), four-space indentation and LF. Unsafe code
  is forbidden; document public APIs and maintain Debug implementations.
- Reproduce bugs with focused regression tests before fixes. New language
  features need valid/invalid syntax, semantic and execution coverage as relevant.
- Syntax/semantics changes must synchronize specification, parser, AST,
  semantic analysis, IR/runtime/backends as applicable, tests, examples, docs,
  and course materials. Create `docs/course/` incrementally for verified features.
- Label proposals as planned. Preserve the bare `print` compatibility alias.
- Keep diagnostic codes stable; improve file identity/spans and structured
  output deliberately. Never equate scanner success with complete validation.
- Record significant work, validation, limitations and the next task in
  `docs/DEVELOPMENT_LOG.md`. Inspect dependencies/tests/design before replacing
  a subsystem and explain why incremental replacement is necessary.
- Continue authorized roadmap work through tested milestones. Document and
  pause before fundamental syntax, memory-model, type-semantics or compatibility
  decisions. Label features Implemented, Partial, Experimental, Stub or Planned;
  do not count scaffolding or skipped tests as completed functionality.
- Every new dogfood application must expose at least one reusable capability
  (a local Sovra package or host adapter) that another application can consume.
  Declare its path and contract in `dogfood/suite.json`, document its limits in
  the application README, and test both the application and capability through
  real Sovra execution. Do not count an app-specific command as reuse.

The opt-in `svr check --experimental-v2` command now checks v2 root sources with
human/JSON diagnostics and no execution. Default commands are unchanged; G13 remains partial.
Package/backend/tooling integration remains open. See [experimental checking](docs/reference/experimental-v2-check.md).

`svr run --experimental-v2` now explicitly executes root sources with finite VM
limits, arguments/status and success-only output publication. No stdin/file hosts
are granted by default. See [experimental execution](docs/reference/experimental-v2-run.md).

`svr test --experimental-v2` now runs parameterless Unit root test_ functions in
source order with isolated invocation state, bounded suite output/test count and
no input/file hosts. See [experimental testing](docs/reference/experimental-v2-test.md).
Default testing is unchanged; packages/backends/tooling and G13 completion remain open.

Experimental `run` now accepts independent `--allow-stdin` and `--allow-files`
grants for existing bounded ProcessHost services. Without flags, v2 execution
denies both; experimental check/test remain host-free. File grants use process permissions without path confinement and
completed effects survive later refusal. See [explicit hosts](docs/reference/experimental-v2-hosts.md).

`svr build --experimental-v2 [--emit ir]` now emits deterministic bounded linked
IR inspection without requiring main or executing source. IDs are local, source
coordinates retained and controls escaped. See [IR inspection](docs/reference/experimental-v2-ir.md).
This is not an executable image/codec; v2 native/JS/package/tooling gates remain open.

Experimental v2 check/run/test/IR commands now accept guarded single-entry package
directories. Captured entry bytes and canonical diagnostic identity are retained;
dependencies/registered modules still refuse. See [root packages](docs/reference/experimental-v2-packages.md). Linux/procfs package capture is qualified;
other package hosts and full linking/locks/backends/tooling remain open.

Single-entry v2 packages now support `lock --experimental-v2 [--check]` and
`--locked` check/run/test/IR over the same captured bytes, using the existing
strict schema and guarded publication. See [v2 locks](docs/reference/experimental-v2-locks.md). Lock integrity is not source validity/authentication;
dependency/module and other-host/backend/tooling gates remain open.

`svr fmt --experimental-v2` now formats root enum/match syntax without requiring
semantic validity, with token/comment/AST/idempotence gates and the existing guarded
source-write path. See [v2 formatting](docs/reference/experimental-v2-format.md). Default formatting is unchanged; opt-in package formatting and standalone
editor selection are now available. Syntax-only parsing does not establish checked success.

Opt-in v2 IDE mode now provides standalone buffer diagnostics, snapshot formatting
and source-only check/IR/run tasks. See [v2 editor mode](docs/reference/experimental-v2-ide.md).
V2 package overlays, type/field navigation, semantic completion and other-host
qualification remain open; G13 remains partial. Legacy mode remains the default.

`fmt --experimental-v2 --package --check|--write` now reuses bounded package
selection, all-source preflight and guarded staged publication with the v2 syntax
formatter. See [package formatting](docs/reference/experimental-v2-format.md). It does not link or execute
dependencies/registered modules; v2 compilation and G13 completion gates remain open.

`build --experimental-v2 --emit exe --output PATH` now builds a private
source-backed native runner with a version/model/feature-checked image, finite
shared CLI policy and no stdin/file services. See [v2 native scope](docs/reference/experimental-v2-native.md).
The binary rechecks/lowers embedded source at startup; it is not an executable IR
codec or optimizing backend. Legacy images retain their path. G13 remains partial;
module/dependency integration and the E2/C1/D1/H1/H2 library phases remain open.

Standalone v2 JSON checks can now request `--symbols` for bounded checked root-
function hover/definition/outline. The v2 LSP requests it automatically; failed
checks and optional projection refusals clear facts. See [root-function navigation](docs/reference/program-symbols-v2.md).
No constructor/standard/type/field/package navigation is implied. G13 stays partial.

V2 standalone symbols now also project scope-owned parameters, locals and match
payloads, including shadowing, indexed writes and match-scrutinee identities.
Checked type labels are bounded; unresolved array holes remain explicitly unavailable.
See [binding navigation](docs/reference/program-symbols-v2.md). Type/field/package navigation remains open.

V2 standalone symbols now also expose checked enum/variant outline entries and
constructor/match-arm variant definitions, selected by nominal IDs and coverage
plans. See [nominal navigation](docs/reference/program-symbols-v2.md). Alias/record/annotation/field and
package navigation still require their own evidence; G13 remains partial.

V2 enum aliases now work as constructor and match-arm qualifiers under the same
canonical nominal IDs, payload rules and standard-call collision refusals. Alias
signatures consume existing count/text quotas; the canonical lookup index is
charged to retained type metadata. See [enum aliases](docs/reference/enum-aliases-v2.md).
Default legacy behavior is unchanged; G13 remains partial.

V2 now admits type-only inline modules containing enums/aliases/records, with
qualified use by root functions, original spans and qualifier-work admission.
See [type modules](docs/reference/type-modules-v2.md). Module functions, nested modules, imports and
dependency linking remain refused; same-file lookup is not package visibility.
G13 remains partial.

V2 raw syntax/formatting now retains module function bodies with original names,
spans and parallel module paths under shared admission. Checked source, execution
and native decoding still explicitly refuse those bodies. See [syntax-only module
bodies](docs/reference/type-modules-v2.md). Scope/visibility/call linking remain open; G13 is partial.

V2 program checking/execution now supports qualified same-file module functions,
private/exported call filtering and module-owned type views with charged aggregate
metadata. See [checked module functions](docs/reference/module-functions-v2.md). Root entry/test selection
remains root-only. Conflicting root-type shadows, nested modules/imports/dependency
linking remain refused; the single-entry API stays strict. G13 is partial.


The additive v2 `parse_generic_type` Rust API now parses approved E2 annotation
syntax under finite source/token/node/edge/name/depth limits with original spans.
See [generic type syntax](docs/reference/generic-type-syntax-v2.md). It is not
semantic type evidence: unknown names/arities can parse. Generic declarations,
bounds/substitution/specialization and executable Option/Result/propagation remain
open; existing source checkers/CLI must continue refusing unsupported generic
programs. G13 remains partial under its approved completion contract.


E2 also has bounded source-owned parameter lists and annotation parameter-use
binding, with distinct owner IDs, original ranges and only the approved
`std::Key` bound spelling. See [parameter ownership](docs/reference/generic-parameters-v2.md).
This is not declaration-body checking or concrete bound validation. Immutable
annotation/parameter owners are shared rather than deep-copied by binding.
Generic executable programs remain unsupported until semantic and specialization
phases are implemented.


E2 now attaches parameter ownership to complete root generic alias syntax under
aggregate source/name and independent fragment/use quotas. See
[generic alias attachment](docs/reference/generic-alias-syntax-v2.md). Fragment
coordinates remain local internally, with explicit full-source mapping methods
and error translation. This is not alias resolution or generic semantic checking;
function/record/enum attachment and specialization remain open.


E2 generic record syntax now attaches one parameter owner across all field types,
with cumulative field/node/edge/name/source/use allowances. See
[generic record attachment](docs/reference/generic-record-syntax-v2.md).
This is not checked layout/constructor/field-access or generic execution support.
Function/enum attachment and semantic checking/specialization remain open.


E2 generic enum syntax now attaches a shared parameter owner across variant
payloads, with original source mapping and cumulative variant/type/source/name/use
admission. See [generic enum attachment](docs/reference/generic-enum-syntax-v2.md).
This is not checked generic constructors/matches or Option/Result. Raw empty
enums may parse, but executable enum checking must still require variants.
Generic function attachment, semantic checking and specialization remain open.


E2 generic function headers now retain source-owned parameter/return annotations
and bounded opaque body ranges, completing the initial four root attachment
profiles. See [generic function headers](docs/reference/generic-function-syntax-v2.md).
The body is only lexed and delimiter-balanced: even invalid statements may be
retained. Never treat this result as checked function syntax or lowering evidence.
Semantic name/arity/bound/body checking and specialization remain open.


E2 now has a bounded root generic header registry and annotation head/arity/Key
resolver. See [generic type resolution](docs/reference/generic-type-resolution-v2.md).
A resolved annotation is not a checked declaration/body: header registration may
retain declarations with invalid fields/targets/payloads. Alias Key proofs require
expansion and currently refuse explicitly; Byte and nongeneric/module/package
contexts are not fabricated. Complete declaration/body checking and specialization
remain open before generic source execution can be enabled.


E2 prepared alias environments now reject cycles, check all symbolic alias targets
and expand transparent substitutions under cumulative admission and fixed depth.
See [generic alias expansion](docs/reference/generic-alias-expansion-v2.md).
The result is an immutable compiler type DAG with explicit root/provenance, not a
runtime ownership change or code specialization. The public header-only resolver
still refuses alias Key proofs; only the prepared environment expands them.
Complete record/enum/function/body checking and generic execution remain open.


E2 now has complete registered root type-definition annotation checking. See
[generic declaration annotations](docs/reference/generic-declarations-v2.md).
Every original definition is required; unused fields/payloads are checked,
empty enums refuse and foreign owners cannot substitute. Sealed proofs retain
syntax/expanded annotations under cumulative member/metadata/source/name/work
budgets. Function/body checking, runtime constructor/match plans and specialization
remain open; this result cannot enable generic CLI execution by itself.


E2 generic function signatures now check all supplied headers against complete
checked type definitions with owner-scoped IDs and cumulative budgets. See
[generic signature checking](docs/reference/generic-signatures-v2.md).
This is signature evidence only: retained bodies can still be invalid and must
not be lowered or executed. Generic main is rejected because entry cannot select
explicit type arguments. Body checking and specialization remain open.


E2 now has bounded expanded-type equivalence and exact normalized in-process keys.
See [generic type equivalence](docs/reference/generic-type-equivalence-v2.md).
Keys preserve nominal/parameter owner IDs and ignore graph sharing/unused nodes.
They are not cross-process cache formats or standalone executable type proofs.
Symbolic body checking, call plans and code specialization remain open.


E2 now has an explicit initial symbolic copy/return/equality body profile. See
[generic copy bodies](docs/reference/generic-copy-bodies-v2.md). It parses original
body tokens, checks every statement/function (including unreachable code) and
retains sealed flat source-owned facts. Calls, controls, mutation,
construction/index/field access, aggregate conversions and matches still refuse.
This proof is not executable IR or specialization; generic CLI execution remains
blocked until the remaining stages are implemented and qualified.


E2 symbolic immutable locals now support explicit generic/alias annotations. See
[generic local annotations](docs/reference/generic-local-annotations-v2.md).
A private original-token block adapter extends local type spelling; public legacy
Program parsing stays strict. Every fragment is independently parsed/resolved
under function type ownership before proof publication. Cumulative local type
budgets and body work/comparison budgets apply. Mutation, calls, broader bodies,
code specialization and generic execution remain open.

Concrete numeric arithmetic and ordering now have bounded symbolic body facts;
see [numeric body profile](docs/reference/generic-numeric-bodies-v2.md).
Opaque numeric assumptions and generic execution remain open.

[Boolean symbolic body expressions](docs/reference/generic-logical-bodies-v2.md)
now check both operands and retain short-circuit operator identity. Executable
short-circuit lowering and broader generic execution remain open.

[Concrete String symbolic expressions](docs/reference/generic-string-bodies-v2.md)
now cover concatenation and lexical ordering without implicit formatting.
Runtime allocation admission and broader generic execution remain open.

[Mutable symbolic locals](docs/reference/generic-mutable-bodies-v2.md) now support
fixed-type direct assignment with explicit widening and binding identity.
Field/index mutation, calls, control flow and generic execution remain open.

[Symbolic expression statements](docs/reference/generic-expression-statements-v2.md)
now retain checked discarded results without conversions or fabricated bindings.
Calls, control flow, specialization and executable integration remain open.

[Symbolic conditional checking](docs/reference/generic-conditional-bodies-v2.md)
now checks concrete Bool conditions, isolated branch scopes and all-path return
coverage using a flat proof/worklist. Loops, calls and generic execution remain open.

[Symbolic while checking](docs/reference/generic-loop-bodies-v2.md) now validates
concrete Bool conditions, isolated loop scopes and fixed parent assignments
without adding guaranteed returns. Calls, specialization and execution remain open.

[Read-only symbolic array indexing](docs/reference/generic-index-bodies-v2.md)
now preserves exact element types and original parameter/provenance owners.
Bounds remain runtime checks; construction, indexed writes and execution remain open.

[Symbolic indexed assignment](docs/reference/generic-index-assignment-v2.md)
now checks direct mutable array locals, concrete Int indexes and exact element
replacement types. Runtime bounds/snapshot ownership and execution remain open.

[Context-typed symbolic array literals](docs/reference/generic-array-literals-v2.md)
now check nested/empty arrays from typed local, return and assignment contexts
with explicit item widening. Inferred arrays, calls and execution remain open.

[Parameter-independent symbolic record fields](docs/reference/generic-field-bodies-v2.md)
now preserve nominal field/receiver identity after a bounded reachable-type walk.
Dependent fields require checked substitution; field writes and execution remain open.

[Direct field-parameter projection](docs/reference/generic-field-arguments-v2.md)
now selects validated receiver arguments while preserving caller parameter and
provenance owners. Composite substitution, calls and execution remain open.

[Composite record field substitution](docs/reference/generic-field-substitution-v2.md)
now preserves caller parameters and per-node input/alias sources under independent
and cumulative node/edge/work/depth limits. Calls and execution remain open.

[Explicit call/constructor type-argument syntax](docs/reference/generic-type-arguments-v2.md)
now admits bounded source-owned `::<T,E>` lists in one shared arena.
Callee binding, checked call facts, specialization and execution remain open.

[Caller-scope explicit type arguments](docs/reference/generic-argument-checking-v2.md)
now resolve names, aliases and type-head Key constraints with exact caller owners
and cumulative copy/expansion budgets. Callee binding and execution remain open.

[Generic callee type matching](docs/reference/generic-call-binding-v2.md) now verifies
exact table/environment/caller owners, arity and callee Key requirements under
matching budgets. Value matching, body calls and execution remain open.

[Bounded callee signature substitution](docs/reference/generic-signature-instantiation-v2.md)
now produces caller-scoped value-parameter/return types with per-node provenance
and cumulative/per-type graph limits. Value/body calls and execution remain open.

[Generic value-argument type matching](docs/reference/generic-call-values-v2.md)
now verifies exact caller body/expression owners, value arity and substituted
parameter compatibility with explicit widening. Source body calls/execution remain open.

[Source-backed generic call envelopes](docs/reference/generic-call-syntax-v2.md) now
retain original callee/type/value ranges with grammar-checked explicit types and
opaque balanced values. Body expression/call integration and execution remain open.

[Original-token call value expressions](docs/reference/generic-call-expressions-v2.md)
now parse exact original ranges with bounded AST/work admission and iterative
cleanup. Caller-scope typing, source-call facts and execution remain open.

[Symbolic arena identities](docs/reference/generic-arena-identities-v2.md) now prevent
IDs from separate checks of the same signature being interchanged. Clones retain
their arena; future call-source expression facts must use distinct arena owners.

[Source-backed symbolic body calls](docs/reference/generic-body-calls-v2.md) now
connect explicit root syntax, active lexical scopes, type/bound/signature stages
and contextual values under cumulative budgets. Runtime/specialization remain open.

[Explicit symbolic record construction](docs/reference/generic-record-construction-v2.md)
now checks nominal arguments, complete field coverage and contextual values with
shared source/provenance and cumulative budgets. Layout/runtime integration remains open.

[Explicit symbolic enum constructors](docs/reference/generic-enum-construction-v2.md)
now preserve nominal/variant identity and nullary versus Unit payloads under
shared input/context budgets. Match plans, specialization and execution remain open.

[Generic enum pattern headers](docs/reference/generic-enum-patterns-v2.md) now prove
exact selected enum coverage/payload contexts under bounded source/graph/work
admission. Active match-site scope, arm typing/return coverage and execution remain open.

[Symbolic generic match bodies](docs/reference/generic-match-bodies-v2.md) now connect active lexical
scope, one scrutinee read, immutable payload IDs, exhaustive arm typing and
conservative return coverage under shared body/graph/work budgets. Nested matches
and explicit calls retain original source ranges. Generic specialization, executable
lowering/runtime/backend integration and structured error propagation remain open.

[Finite concrete generic call discovery](docs/reference/generic-specialization-v2.md) now admits
source-owned nongeneric roots and discovers exact argument instances with bounded
substitution, normalized reuse and recursive closure. Defaults cap standalone
instances at 1,024 and the explicit package profile at 4,096; no package linking
is performed. Concrete signature/body rechecking, layout plans, executable
lowering/runtime/backend integration and structured error propagation remain open.

[Concrete generic body rechecking](docs/reference/generic-concrete-bodies-v2.md) now substitutes
instance signatures/bindings/expressions and rechecks operation, conversion, call,
constructor/field and match-payload contexts under cumulative generation/comparison/
work budgets. Original lexical ownership/return-flow proofs remain sealed; concrete
lookup selects a graph instance before using symbolic IDs. Literal decoding,
nominal/runtime layouts, executable lowering/backend integration and structured
error propagation remain open.

[Decoded generic literal payloads](docs/reference/generic-literals-v2.md) now retain exact
Int/finite Float bits/Bool/decoded UTF-8 values from original source ranges, sharing
immutable facts across instances while charging each reference. Existing conversion
edges remain explicit; source/token/string-copy/work admission is finite. Runtime
layouts, constructor/match plans, executable lowering/backend integration and
structured errors remain open.

[Logical generic runtime layouts](docs/reference/generic-layouts-v2.md) now map concrete
source facts to finite primitive/array/record/enum graphs with exact normalized
identity, declaration-order slots/tags and recursive edges. Function and nominal
type instances share the 1,024 standalone / 4,096 package-profile cap. Input/member
arenas, generated metadata, references and lookup work stay bounded. No physical ABI
or executable path is selected; constructor/match plans, lowering/backends, package
linking and structured errors remain open.

[Generic nominal operation plans](docs/reference/generic-construction-v2.md) now link source-order
record initializers, enum tags/payloads and field reads to exact instance layouts.
Match dispatch retains one source read, immutable payload destinations, source-order
arms and exhaustive tag mappings with source-contained statement ranges. Planning
is cumulative and bounded; executable IR/control-flow lowering, runtime allocation/
snapshot behavior, backend/package integration and structured errors remain open.

[Typed generic instruction lowering](docs/reference/generic-ir-v2.md) now emits owned
stack instructions with resolved control flow, source-order operands, concrete
calls, short-circuit branches and enum dispatch. Function/slot/target structures
and reachable typed-stack joins are checked under finite emission/pending/state/
work budgets. Scalar test probes verify behavior; production runtime/backend
execution, resource/value policy and source/program/package/CLI integration remain
open alongside structured errors.

[Host-free generic Rust execution](docs/reference/generic-runtime-v2.md) now runs sealed
IR from a zero-argument root with iterative calls and flat owned compound values.
Snapshots, indexed replacement, enum payloads, equality and deep cleanup retain
value semantics under shared existing materialization/instruction/control policy
and explicit work/frame/slot/operand caps. Borrowed result inspection grants no
host services. Source/program/package/CLI/native/JS integration, validated external
arguments and structured errors remain open.


G13 E2 source checkpoint (2026-11-15): [standalone generic source execution](docs/reference/generic-source-v2.md)
now collects complete admitted files and checks all declarations/bodies before
specializing and executing a zero-argument root. Ordinary nongeneric calls share
the bounded pipeline; original whole-file error locations and cumulative syntax
quotas are retained. This is a Rust library API. CLI/package/native/JavaScript
integration, standard Option/Result, propagation and later G13 phases remain open.


G13 E2 CLI checkpoint (2026-11-15): [explicit standalone generic commands](docs/reference/generic-cli-v2.md)
now expose bounded `check` and host-free `run` under `--experimental-generics`.
The CLI requires a parameterless Unit main for execution, retains original-file
errors, supports existing check JSON and refuses unavailable hosts, packages,
program arguments and engine combinations. Package/backend/tooling integration,
standard Option/Result and propagation, and later G13 phases remain open.


G13 E2 native checkpoint (2026-11-15): [generic host-native packaging](docs/reference/generic-native-v2.md)
now links explicit standalone generic builds as Rust executables with bounded
embedded-source rechecking and host-free VM execution. Separate image dispatch
preserves legacy/plain-v2 formats; source/IR quotas and command policy remain
finite. This is source-backed execution, not direct generic machine-code lowering.
Package/JavaScript/tooling integration, standard errors/propagation and later
G13 phases remain open; other-host qualification remains a gate.


G13 E2 root-package checkpoint (2026-11-15): [guarded generic root packages](docs/reference/generic-packages-v2.md)
now connect check/run/native build to captured single-entry projects and optional
source-lock verification, with generic lock generation/checking. Source-read caps
can reduce existing guarded admission; canonical diagnostic provenance is retained.
Runtime generic errors use E3125, reserving existing E3124 for unsupported package
shapes. Dependency/module linking, JavaScript/tooling, standard errors/propagation
and later G13 phases remain open. Package capture still requires supported Linux/procfs.


G13 E2 standard-enum checkpoint (2026-11-15): [qualified Option/Result](docs/reference/generic-standard-v2.md)
now use compiler-owned nominal identities through complete source checking,
constructors, exhaustive matches, specialization and host-free execution. User
Option/Result names stay available; standard templates consume semantic budgets
and have separate provenance. Native feature word 3 advertises standard enums;
old word-1 images retain their prior environment. std::Error, adapters, postfix
propagation, dependency/module linking and remaining G13 work stay open.


G13 E2 standard-error checkpoint (2026-11-15): [structured std::Error](docs/reference/generic-error-v2.md)
now has compiler-owned code/message Strings and optional Int offset, checked
qualified construction, fields and owned snapshots through Result payloads.
Public generic record parsing stays strict; ordinary user Error types stay free.
New native feature word 7 includes Error while retaining old word-1/word-3 profiles.
Explicit adapters, postfix propagation, dependency/module/JavaScript/tooling and
later G13 work remain open. Other-host/installed qualification remains separate.


G13 E2 propagation-contract checkpoint (2026-11-15):
[sealed propagation compatibility](docs/reference/generic-propagation-v2.md) now validates selected body-owned standard Option/Result
operands against enclosing return wrappers under cumulative context/comparison/
work budgets. Successful types may differ; Result error types match exactly.
Original body/provenance and variant/type plans are retained. This is a Rust
planning API only: source question marks, concrete/executable lowering, native
feature integration, adapters and remaining G13 work stay open.


G13 E2 propagation-syntax checkpoint (2026-11-15):
[original-source postfix capture](docs/reference/generic-propagation-syntax-v2.md) now retains bounded operand/question/
result spans for complete blocks, with nested/chained applications and parser
rollback coverage. Private capture publishes no unwrapped legacy AST. Ordinary
lexer/parser, checked source, CLI and existing native profiles remain strict.
Source/body binding, concrete rechecking, early-return/runtime/native integration,
adapters and remaining G13 work stay open.


G13 E2 checked-postfix checkpoint (2026-11-15):
[opt-in symbolic postfix bodies](docs/reference/generic-propagation-bodies-v2.md) now bind original questions to active
body operands and sealed success/failure types. Complete checking covers aliases,
scopes, calls, fields/indexes, control flow and unreachable code under shared
comparison/work/virtual-node/depth admission. Specialization and lowering refuse
question-bearing tables; full-source/CLI/native profiles remain strict. Concrete/
layout/early-return/runtime/native integration, adapters and remaining G13 work stay open.


G13 E2 concrete-postfix checkpoint (2026-11-15):
[opt-in concrete propagation](docs/reference/generic-propagation-concrete-v2.md) now discovers finite closures and recomputes
closed operand/success/failure/return contracts per instance, checking original
symbolic descriptors under shared generated-type/context/comparison/work caps.
Ordinary entry points remain strict; IR rejects every question-bearing source
table, including unreachable questions. Layout/early-return/runtime/native
integration, adapters and remaining G13 work stay open.


G13 E2 propagation-layout checkpoint (2026-11-15):
[bounded propagation operation layouts](docs/reference/generic-propagation-layouts-v2.md)
now retain input, extracted payload, failure tag/payload and caller-return layouts
per concrete source site under shared admission. Early-return lowering and execution
remain gated; executable integration, adapters and remaining G13 work stay open.


G13 E2 propagation-IR checkpoint (2026-11-15):
[inspection-only early-return IR](docs/reference/generic-propagation-ir-v2.md)
now lowers each checked question once, validates standard wrapper/failure contracts
and typed success continuations under shared IR admission. Its separate owner
cannot enter the public VM. Ordinary lowering and runtime remain gated for every
question-bearing source table. Payload execution/cleanup/fuel, source/native
integration, adapters and remaining G13 work stay open.


G13 E2 propagation-VM checkpoint (2026-11-15):
[explicit propagation VM execution](docs/reference/generic-propagation-vm-v2.md)
now copies success/error payloads, rebuilds compatible failure returns and cleans
current-frame slots/operands under shared snapshot/work/control admission. The
ordinary VM remains gated. Executable source/CLI/native integration, adapters
and remaining E2/G13 work stay open.


G13 E2 propagation-source checkpoint (2026-11-15):
[explicit complete-source propagation](docs/reference/generic-propagation-source-v2.md)
now collects original complete files and composes symbolic/concrete propagation,
operation layouts, IR and VM execution under independent shared stage admission.
Ordinary source/CLI/native profiles remain strict; native encoding refuses the
new profile. CLI/native integration, adapters and remaining E2/G13 work stay open.


G13 E2 propagation-command checkpoint (2026-11-15):
[experimental CLI/package/native propagation](docs/reference/generic-propagation-cli-v2.md)
now selects the complete checked propagation source profile. Native feature word
15 enables bounded startup rechecking; words 1/3/7 remain strict. Existing root
package/lock/authority and command-entry restrictions remain. Explicit adapters,
remaining E2/G13 work and other-host qualification stay open.


G13 E2 standard-result adapter checkpoint (2026-11-16):
[explicit source-level legacy result adapters](docs/reference/generic-result-adapters-v2.md)
now admit five immutable legacy schemas and provide ordinary Sovra conversion
functions under existing shared quotas. The experimental CLI uses this profile;
native feature 31 retains it. Builtin-call bridging, other-host execution and
remaining E2/G13 requirements stay open. Existing builtin return types are unchanged.


G13 E2 host-free builtin checkpoint (2026-11-16):
[checked parse_int/split_once calls](docs/reference/generic-builtin-calls-v2.md)
now preserve legacy result records through symbolic/concrete/IR/VM checking and
explicit adapter/question composition. Experimental CLI/native feature 63 selects
this profile; words 1/3/7/15/31 remain unchanged. Other builtins, remaining E2/G13
work and other-host qualification stay open. No host I/O is enabled.


G13 E2 String-builtin checkpoint (2026-11-16):
[checked len/lines_unique/escape_terminal calls](docs/reference/generic-string-builtins-v2.md)
now preserve core semantics with primitive result checks, temporary line-storage
admission and escape-output preflight. Experimental CLI/native feature 127 selects
this profile; older feature environments remain unchanged. to_string, array/host
builtin bridging, remaining E2/G13 work and other-host qualification stay open.


G13 E2 value-builtin checkpoint (2026-11-16):
[checked to_string/array_len calls](docs/reference/generic-value-builtins-v2.md)
now support polymorphic iterative display and generic array lengths. Output and
temporary formatting tasks are admitted under shared budgets. Experimental
CLI/native feature 255 selects the new profile; previous environments stay strict.
Append/concat/slice, host builtins, remaining E2/G13 work and platform gates stay open.


G13 E2 array-builtin checkpoint (2026-11-16):
[checked append/concat/slice calls](docs/reference/generic-array-builtins-v2.md)
now preserve snapshots, generic element identity, recursive numeric widening and
half-open bounds. Explicit IR conversion targets and whole-result copy preflight
use shared admission. Experimental CLI/native feature 511 selects the new profile;
previous environments stay strict. Host bridging and remaining E2/G13 gates stay open.


G13 E2 generic-editor checkpoint (2026-11-16):
[explicit generic editor diagnostics/tasks](docs/reference/generic-ide-v2.md)
now check unsaved original buffers and dispatch standalone Check/Run with the
generic compiler flag. Modes are mutually exclusive; unsupported navigation,
formatting and IR/JS tasks remain gated. Dependency/module/JS integration, broader
editor evidence and remaining E2/G13 gates stay open. Task Manager is unchanged.


G13 E2 generic-symbol checkpoint (2026-11-16):
[checked root declaration symbols](docs/reference/generic-symbols-v2.md)
now project complete original declarations into optional bounded JSON evidence and
generic editor outlines/declaration hover. Exhaustion returns no facts without
changing check success. Call/type/local references, formatting, modules/dependencies,
JavaScript and remaining E2/G13 gates stay open. Task Manager is unchanged.


G13 E2 call-navigation checkpoint (2026-11-16):
[checked direct-call references](docs/reference/generic-call-navigation-v2.md)
now retain original callee tokens and resolved function identities through optional
JSON evidence and generic editor Go to Definition/hover. References cover original
symbolic bodies once, not specialization duplicates. Local/type-use references,
formatting, package overlays and remaining E2/G13 gates stay open.

2026-11-16 checkpoint: [checked generic local navigation](docs/reference/generic-local-navigation-v2.md) adds parameter/local/match-payload definition and role hover. Type rendering and broader E2/G13 gates remain open.

2026-11-16 type-hover checkpoint: [checked generic binding types](docs/reference/generic-local-navigation-v2.md) now display symbolic/inferred types from retained owners with bounded iterative alias-expanded rendering. Type-use navigation and broader E2/G13 requirements remain open.

2026-11-16 header-type checkpoint: [checked generic annotation navigation](docs/reference/generic-type-navigation-v2.md) links user type names in function parameters/returns through sealed registry identities. Broader type-use navigation and E2/G13 requirements remain open.

2026-11-16 functional checkpoint: [generic source and locked-package tests](docs/reference/generic-testing-v2.md) adds bounded test discovery, all-root preflight and fresh sequential VM execution. Editor polish is deferred; dependency/module linking and remaining E1/E2/G13 requirements remain open.

2026-11-16 package-input checkpoint: [syntax-independent graph capture](docs/reference/captured-package-graphs-v2.md) retains guarded per-file dependency/module source snapshots and verifies compatible source locks without the legacy parser. This is not generic import linking; CLI dependency/module refusal remains active. Next: source-owned declarations, import visibility and cross-file identities.

2026-11-16 package-binding checkpoint: [generic declaration ownership and visibility](docs/reference/generic-package-declarations-v2.md) assigns table-owned per-file identities, validates registered module scopes and binds file-local direct dependency imports with private/export boundaries. This is not semantic/executable linking; generic CLI dependency/module refusal remains active.

2026-11-16 package-type checkpoint: [checked package types and signatures](docs/reference/generic-package-types-v2.md) feeds per-source visibility into existing registry/alias/definition/signature semantics with canonical nominal identities and cumulative admission. Scoped body compilation and unscoped function lookup remain gated; generic CLI dependencies/modules remain refused until executable linking is qualified.

2026-11-16 package-body checkpoint: [source-aware package calls and nominal operations](docs/reference/generic-package-bodies-v2.md) checks every body under its own visibility/type context. Explicit-library specialization/IR/VM tests execute imported generic calls, transitive helpers, records, enums/matches and propagation. Selected-root package/diagnostic and CLI/native integration remain open; generic CLI dependencies/modules remain refused.

2026-11-16 selected-root checkpoint: [owned generic package compilation and Check/Run/Lock](docs/reference/generic-package-compilation-v2.md) now executes registered local dependency graphs with locked snapshot custody, root-owned entry selection and cross-file runtime diagnostics. Multi-file Test/native Build and editor package integration remain gated; G13 is not complete.

2026-11-16 package-suite checkpoint: [generic package tests](docs/reference/generic-package-testing-v2.md) now run root-entry tests with registered module/dependency helpers over one immutable checked graph. Every selected specialization is preflighted; shared instance reservation bounds both passes, with fresh per-test VM policy and original-file failures. Locked test reports preserve lock priority and schema-one output. Multi-file native Build remains gated; G13 is not complete.

2026-11-16 native-package checkpoint: [versioned source/topology images](docs/reference/generic-package-native-v2.md) now build and run registered generic package graphs without project-file or Node.js startup dependencies. Embedded inputs retain logical ownership but cannot become guarded filesystem captures. Old single-entry image families remain unchanged. Linux native consumer execution and codec refusals are qualified; broader E1/E2/G13 and other-host/backend gates remain open.

2026-11-16 collections checkpoint: the [snapshot collections reference package](stdlib/snapshot-collections/README.md) supplies ordered generic maps/sets, explicit logical equality, missing-key Options and structured duplicate-record refusal. Locked Check/Run/Test and native consumer coverage exercise the actual library. This uses ordinary records/arrays and existing VM copy/work limits; raw record equality remains structural. Intrinsic std::Map/std::Set representation, operators and cost qualification remain open; C1 and G13 are not complete. Package body type-argument admission is bounded at 4096 aggregate; standalone and per-list limits remain unchanged.

2026-11-16 opaque-set checkpoint: [primitive-key runtime sets](docs/reference/runtime-sets-v2.md) now enforce private homogeneous storage, ordered snapshots, membership equality and atomic full-result node/text admission with finite helper work. This is an explicit Rust library foundation, not compiler/IR/VM/source/native admission. Opaque maps, collection type integration and full C1/G13 qualification remain open.

2026-11-16 opaque-map checkpoint: [typed runtime maps](docs/reference/runtime-maps-v2.md) now enforce private unique keys and exact primitive/array value shapes, preserve update order, compare associations and atomically admit complete descendant copies. Nominal record/enum ownership and compiler/layout/IR/VM/native collection admission remain open. This is an explicit Rust library API; C1/G13 are not complete.

2026-11-16 nominal-enum map checkpoint: [checked enum map leaves](docs/reference/runtime-maps-v2.md#checked-enum-value-leaves) now retain sealed environment-specific enum identities and copy all type/variant labels and payload descendants under atomic fuel and finite work. Foreign owners, forged text and transitive record payloads refuse. Record ownership and compiler/layout/IR/VM/native collection integration remain open; C1/G13 are not complete.

2026-11-16 sealed-record checkpoint: [checked record runtime ownership](docs/reference/runtime-record-owners-v2.md) now retains environment-specific record IDs, validates exact field/child owners before sealing, preserves IDs through snapshots/extraction and admits owned records in maps and enum payloads. Name-only records remain refused transitively. Record equality charges every candidate field-name search. Generic package-owner adaptation and collection type/layout/IR/VM/native integration remain open; C1/G13 are incomplete.

2026-11-16 generic-owner bridge checkpoint: [proof-carrying generic snapshots](docs/reference/generic-collection-values-v2.md) now capture actual package VM values with their own checked layout/declaration/type-argument evidence. Raw layout numbers and caller names cannot attach foreign proof. Imported homonyms, aliases and phantom arguments remain exact; full subtree copies use existing atomic generic snapshot units and finite work. Generic collection storage/source/layout/IR/VM/native integration remains open; C1/G13 are incomplete.

2026-11-16 generic-map storage checkpoint: [maps of checked generic package values](docs/reference/generic-collection-maps-v2.md) now enforce exact retained value types, preserve insertion order and owners across updates/get/remove/iteration, compare associations, and atomically admit all result descendants before copying. Root/dependency homonyms and phantom/foreign owners refuse. Type profiles remain finite and value copy units inherit the generic snapshot bridge. Source collection types/layouts/IR/VM and native qualification remain open; C1/G13 are incomplete.

2026-11-16 opaque-layout plan checkpoint: [checked collection layout plans](docs/reference/generic-collection-layouts-v2.md) now bind compiler-owned Map/Set constructors to exact checked arguments, enforce the primitive Key bound and instantiate the existing typed storage without caller-supplied key-domain guesses. This separate Rust plan API does not publish executable layout variants or enable source annotations. Native words and ordinary records remain unchanged; C1/G13 stay open pending source/layout/IR/VM/native qualification.

2026-11-16 intrinsic annotation checkpoint: [opt-in package collection semantics](docs/reference/generic-collection-types-v2.md) now registers opaque compiler-owned Map/Set headers and checks real source annotation arity, Key bounds, aliases/nesting and per-file dependency visibility under cumulative limits. Collection definitions are not records/enums; record literals/field projections refuse. Concrete forwarding rechecks, then executable layout publication explicitly refuses. Default compiler/CLI/native profiles remain unchanged; C1/G13 stay open for executable layouts/IR/VM/native qualification.

2026-11-18 collection layout-graph checkpoint: [opt-in closed Map/Set layouts](docs/reference/generic-collection-layout-graphs-v2.md) now retain opaque argument edges, intern aliases/exact owners and close recursive map-value graphs under the existing cumulative instantiation/layout/work limits. Real checked shapes bind typed storage even when producer layout numbers differ. Old planning remains strict; construction/IR explicitly refuses collection-bearing graphs, including phantom arguments. Actual collection builtin/IR/VM operations and native qualification remain open; C1/G13 are incomplete.

2026-11-18 initial collection execution checkpoint: [typed empty collection operations](docs/reference/generic-collection-operations-v2.md) now compile and execute real Sovra map_new/set_new/map_len/set_len through explicit package APIs, checked builtin IR and shared VM budgets/control. Contextual types preserve K/V without defaults; distinct flat empty values copy/compare/propagate normally. Ordinary execution and native encoding explicitly refuse the new profile. Nonempty operations, full language result admission and CLI/native qualification remain open; C1/G13 are incomplete.

2026-11-18 collection insertion checkpoint: [real-source map/set insertion](docs/reference/generic-collection-insertion-v2.md) now validates exact receiver/key/value types through body/concrete/IR checks, creates ordered flat nonempty snapshots, replaces without moving keys and retains duplicate set positions. Whole-result ownership/copy work is admitted before output copying, including real internal entry nodes. Equality compares associations/membership with charged key searches. Source lookup/removal/iteration and CLI/native qualification remain open; C1/G13 are incomplete.

2026-11-18 membership checkpoint: the explicit collection package profile now
checks and executes `std::map_contains` and `std::set_contains` with exact key
types, primitive Bool results and shared metered key searches. See
[collection operations](docs/reference/generic-collection-operations-v2.md).
Value lookup/Option ownership, removal, iteration, formatting and CLI/native
qualification remain open; C1/G13 are incomplete.

2026-11-18 typed lookup checkpoint: the explicit collection package profile now
checks and executes `std::map_get` and `std::set_get`, deriving sealed standard
Option result types from checked payload ownership. Missing keys produce real
None values; present values copy complete independent snapshots. The whole
wrapper/payload node and text cost is admitted before output copying. See
[typed collection lookup](docs/reference/generic-collection-lookup-v2.md).
Removal, iteration, formatting and CLI/native qualification remain open;
C1/G13 are incomplete.

2026-11-19 typed removal checkpoint: the explicit collection package profile now
checks and executes `std::map_remove` and `std::set_remove`, returning Some(updated
collection) on a hit and None on a miss. Surviving entries keep their order and
full ownership; remove/reinsert appends. Whole Option/container/descendant node
and text costs are admitted before copying. See
[typed collection removal](docs/reference/generic-collection-removal-v2.md).
Source iteration, formatting and CLI/native qualification remain open;
C1/G13 are incomplete.

2026-11-19 iteration snapshot checkpoint: the explicit collection package profile
now checks and executes `std::map_keys`, `std::map_values` and `std::set_values`,
returning exact typed insertion-order arrays with independent complete copies.
Generated array proofs and whole-result node/text/copy work remain bounded; see
[collection iteration snapshots](docs/reference/generic-collection-iteration-v2.md).
Map entry-pair snapshots, formatting, CLI/native admission and consolidated
backend/host/resource qualification remain open; C1/G13 are incomplete.

2026-11-19 public entry snapshot checkpoint: the explicit collection package
profile now checks and executes `std::map_entries` as insertion-order arrays of
sealed `std::MapEntry<K,V>` records with public key/value fields. Actual record
wrappers and all descendant/label copies are admitted together; typed ownership,
Key bounds and source/default-profile/native gates remain explicit. See
[public map entries](docs/reference/generic-collection-entries-v2.md).
Collection formatting, CLI/native admission and consolidated backend/host/resource
qualification remain open; C1/G13 are incomplete.

2026-11-19 collection display checkpoint: explicit collection package execution
now formats maps/sets and nested values in insertion order with checked private
entry ownership and exact two-pass output sizing. Existing temporary-task/node,
text, work and cooperative controls remain active; raw String/scalar and public
record display conventions remain unchanged. See
[collection display](docs/reference/generic-collection-display-v2.md).
CLI/native admission and consolidated backend/host/resource qualification remain
open; C1/G13 are incomplete.

2026-11-19 collection image checkpoint: separate explicit encoding/decoding and
execution APIs now admit exact package feature word 1023, rechecking all embedded
collection source with finite compiler/VM policies and memory-only ownership.
Word 511 and the ordinary compiler/encoder/decoder remain strict; feature flips,
source errors and resource refusal never select a fallback. See
[collection package images](docs/reference/generic-collection-native-images-v2.md).
Installed CLI/native backend/runtime dispatch and generated-executable launch
qualification remain open, along with consolidated backend/host/resource gates;
C1/G13 are incomplete.

2026-11-19 collection command/launch checkpoint: `--experimental-generics
--experimental-collections` now opts package Check/Run/Test/native Build into
collection semantics and exact word-1023 runner dispatch. Collection suites retain
all-root preflight and finite fresh per-test VM policies. Single-entry packages
use the new package profile; default and word-511 paths remain unchanged. See
[collection CLI/native workflow](docs/reference/generic-collection-cli-v2.md).
Linux collection launch, 71 generic integration targets, all 600 library tests
and eight IDE scripts pass. Byte-key/additional-host integration and
E2/D1/H1/H2/Q1 completion remain open. G13 is incomplete.

2026-11-19 D1 byte-runtime checkpoint: explicit Rust checked Byte/immutable Bytes
APIs now provide typed scalar conversion, complete buffer/array snapshots,
index/slice/concat, strict lowercase-hex/standard-base64/UTF-8 codecs and original
input offsets. Packed physical bytes retain one logical Byte node each; whole
outputs and finite helper work/cooperative controls are admitted explicitly. See
[byte runtime foundation](docs/reference/runtime-bytes-v2.md).
Source type/layout/IR/VM and Result-wrapper integration, Byte keys, CLI/native
admission and JSON remain open; this does not complete D1 or G13.

2026-11-19 compiler data-type checkpoint: the explicit data package semantic API
now registers sealed zero-argument std::Byte/std::Bytes headers, checks Byte Key
bounds through aliases/calls/concrete forwarding and preserves exact primitive,
buffer, phantom and imported ownership. Empty parameter scopes are explicitly
validated against registered headers. See
[compiler data annotations](docs/reference/generic-data-types-v2.md).
Default/collection-only APIs still reject data annotations; actual data layouts
explicitly refuse. Source operations, owned Result costs, runtime Byte keys,
CLI/native data admission and JSON remain open; D1/G13 are incomplete.

2026-11-19 data layout checkpoint: the explicit data layout planner now publishes
closed Byte/Bytes metadata with exact declaration identities, a sealed Byte edge
inside Bytes, canonical alias interning and bounded recursive/phantom collection
graphs. Existing construction and IR paths reject data-bearing tables independently;
Byte storage plans refuse instead of selecting Int keys. See
[checked data layout graphs](docs/reference/generic-data-layouts-v2.md).
Checked byte operations, owned Result/error admission, runtime Byte keys, CLI/native
data profiles and JSON remain open; D1/G13 are incomplete.

2026-11-19 scalar Byte execution checkpoint: the explicit data package compiler/VM
now executes checked Int-to-Byte Result conversion and explicit Byte-to-Int,
with complete owned success/error preflight, propagation and Byte collection keys.
Default/collection-only execution and existing native encoders refuse this profile.
Bytes execution, CLI/native data admission, JSON and the remaining G13 phases
stay open. See [checked Byte execution](docs/reference/generic-byte-execution-v2.md).

2026-11-19 Bytes execution checkpoint: the explicit data package compiler/VM
now constructs immutable buffers from original UTF-8 text or checked Byte arrays,
executes byte length and ordered equality/display, and retains nested/generic
copy ownership with complete N+1-node output admission. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source get/slice/concat/to-array/codecs, CLI/native data admission, JSON and
remaining G13 phases stay open; G13 is incomplete.

2026-11-19 Bytes access/concat checkpoint: the explicit data package compiler/VM
now executes `std::bytes_get(Bytes,Int) -> Result<Byte,Error>` with structured bounds
errors carrying the attempted index and complete owned wrapper/payload costs, and `std::bytes_concat -> Result<Bytes,Error>` with ordered independent output,
structured capacity failure and whole-result node admission. Exact checked
operand/result identities are independently rechecked before execution. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source slice/to-array/codecs, CLI/native data admission, JSON and remaining G13
phases stay open. G13 remains incomplete.

2026-11-19 constructor Result checkpoint: `bytes_from_text` and `bytes_from_array`
now return sealed `Result<Bytes,Error>`, including recoverable owned capacity
errors. Explicit VM callers may lower the shared `max_byte_buffer_bytes` ceiling
from 16 MiB to zero. Empty buffers still succeed; all constructor/concat success
and error paths preflight full ownership before output allocation. Source
fixtures now use explicit propagation, and constructor Result/error qualification
listed at the preceding checkpoint is complete. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source slicing/to-array/codecs, CLI/native data admission, JSON and remaining
G13 phases remain open. G13 is incomplete.

2026-11-19 byte slice checkpoint: the explicit data package compiler/VM now
executes `std::bytes_slice(Bytes,Int,Int) -> Result<Bytes,Error>` for half-open
raw byte ranges, including empty slices and slices within UTF-8 sequences.
Negative/reversed/out-of-range bounds return owned errors carrying the original
signed index. Source/concrete/IR stages independently validate exact operands
and sealed results; whole success/error ownership is admitted before allocation.
See [checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source to-array/codecs, CLI/native data admission, JSON and remaining G13 phases
stay open. G13 is incomplete.

2026-11-19 Byte array conversion checkpoint: the explicit data package
compiler/VM now executes `std::bytes_to_array(Bytes) -> Result<[Byte],Error>`
with exact sealed Byte element ownership, independent mutable array snapshots,
raw-byte preservation and complete N+2-node/13-label-byte output admission.
Source/concrete/IR validation and generic forwarding retain distinct buffer,
array and Byte/Int identities. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source codecs, CLI/native data admission, JSON and remaining G13 phases stay
open. G13 is incomplete.

2026-11-19 source hex checkpoint: the explicit data package compiler/VM now
executes `bytes_to_hex(Bytes) -> Result<String,Error>` and
`bytes_from_hex(String) -> Result<Bytes,Error>`, using lowercase encoding, strict
case-insensitive hex digits, original byte offsets and recoverable capacity
errors. It shares the Rust helper digit rules/capacity-validation order while
preflighting complete language Result/error ownership before output allocation.
Sealed String/Bytes Result schemas are independently rechecked. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source base64/UTF-8 codecs, CLI/native data admission, JSON and remaining G13
phases stay open. G13 is incomplete.

2026-11-20 source Base64 checkpoint: the explicit data compiler/VM now executes
`bytes_to_base64(Bytes) -> Result<String,Error>` and
`bytes_from_base64(String) -> Result<Bytes,Error>`. Standard RFC 4648 alphabet,
canonical padding/discarded bits, original byte offsets and complete owned
Result/error preflight are enforced. The packed helper and flat VM share one
strict validator while preserving each caller's control/work errors. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source UTF-8 decoding, CLI/native data admission, JSON and remaining G13 phases
stay open. G13 is incomplete.

2026-11-20 source UTF-8 checkpoint: the explicit data compiler/VM now executes
`bytes_to_utf8(Bytes) -> Result<String,Error>` with RFC 3629 validation, original
invalid-sequence byte offsets and no replacement/normalization. Valid BOM, NUL,
combining sequences and encoded U+FFFD are preserved. Stack-only validation
precedes complete Result ownership admission and exactly sized String copying;
both passes retain cooperative controls. See
[checked immutable Bytes execution](docs/reference/generic-bytes-execution-v2.md).
Source byte-codec execution is covered; CLI/native data admission, JSON and
remaining G13 phases stay open. G13 is incomplete.
