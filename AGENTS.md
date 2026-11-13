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

Use `docs/LANGUAGE_MATURITY_AUDIT.md` as the current repository-wide capability,
remaining-work and contributor-task index. Its linked
`docs/18_TASK_COMPLETION_CHECKLIST.md` expands the same G01–G18 IDs to cover the
user-requested toolchain/native/stdlib/HTTP/UI/package goals. Preserve those IDs
and completed foundation scopes; unchecked product criteria remain open.
Older dated checkpoints are historical
evidence, not a substitute for current source and exact-platform verification.

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
`docs/design/G13_SUM_TYPE_SYNTAX.md` proposes the next error foundation: an
additive bounded single-enum syntax API only. It is not yet approved/implemented.
Do not infer approval for executable enums, match, generics, AST/IR changes or
standard-result migration from existing array/resource decisions.
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
