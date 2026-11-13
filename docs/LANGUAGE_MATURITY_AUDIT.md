# Sovra language maturity and contributor readiness

Audit date: 2026-11-11. Scope: the current local working tree of
`peterbwire/Sovra`, based on `a758fba`, including its uncommitted compiler, UI,
tooling and installer work. This is a repository-wide engineering assessment,
not a line-by-line security audit, remote CI verification or release approval.

**Sovra has a useful implemented language core and substantial test/tooling
infrastructure. It is ready for focused experimental contributions, but is not
yet an all-platform, production-qualified application language.** Contributors
can help now without waiting for every future language feature. Published claims
must distinguish checked source, partial application inspection, host prototypes
and actually qualified deployment targets.

The durable goal remains [mobile, desktop, web, servers and other environments](PLATFORM_VISION.md).
Windows foundation qualification is the immediate track; Task Manager enhancements
remain on hold. Keep M0–M15 and the existing [release gates](V1_DELIVERY_GATES.md).
The work items below are task IDs, not replacement milestones.

## What was inspected

- Compiler stages, executable AST/types/IR, interpreter, generated JavaScript,
  native image/linking boundaries, diagnostics, resource limits and fuzz targets.
- Local package resolver/linker, manifest/project scanner and opt-in application
  body inspection; language tests, formatter, doctor and project generation.
- Shared UI protocol/Node bridge, compiled WPF presentation/transport and native
  acceptance fixtures; editor, LSP, VS Code extension and developer-tool bundling.
- Windows package/install/upgrade/rollback scripts, CI and release-candidate
  workflows, runtime payload/distribution documents and release qualification gates.
- Examples/dogfood applications, course/reference/design documents, contributor,
  security, conduct and licensing materials, and public onboarding paths.

Generated builds and historical logs were used as evidence where identified,
not treated as source or proof that a downloaded Windows release works. No Task
Manager business changes, syntax decisions, publication or external issue creation
were performed in this audit.

## Current capability map

| Area | Actual implemented capability | Remaining gap |
| --- | --- | --- |
| Language core | Typed functions, primitive values, local inference, aliases, nominal structs, arrays, conditions/loops, mutable locals/direct array writes, short-circuit logic | General enums/pattern matching, generic abstractions, closures/function values and broader collection APIs are not implemented. Select additions through reviewed designs, not a feature checklist copied from other languages. |
| Type checking | Checked function calls/returns, explicit parameter annotations, nominal identity and approved value semantics; immutable standalone facts consumed by opt-in IDE projection and an explicit checked-lowering API | Standalone run/build/native/test callers now consume checked evidence with active finite limits; ordinary package callers now use cumulative checked evidence too; final qualification remains. Unsupported annotations must continue to fail rather than masquerade as types. |
| Compiler organization | Rust-owned lexer → parser/AST → semantic → IR, stable diagnostics, deterministic checks and bounded inputs | Unify compiler-owned symbol/type evidence for tooling and reduce repeated semantic work without replacing validated stage APIs casually. |
| Modules/packages | Local dependency graphs, registered multi-file modules, nominal exports and source-hashed local locks | Remaining type/tooling contexts, version selection, verified remote acquisition/cache/offline use, registry ownership and publication remain open under G12. Local locks are not a distributed package manager. |
| Project/application checking | Manifest/wiring scan and opt-in structured inspection with coverage/error reporting | M12 is partial and separate from executable compilation. Scanner success is not proof that services, pages, auth, jobs or whole applications can execute. |
| Runtime/backends | Rust interpreter and Node-targeted JS, checked arithmetic/conversions, snapshot values, optional logical fuel and approved Rust/Node Worker cooperative controls | Owned handles/capabilities, concurrency, structured errors, helper-work/physical-memory policy and wider host services remain open. Ordinary Node scripts have no cancellation transport; current controls cannot preempt blocked I/O or establish a sandbox. |
| Native distribution | Host-native executable embeds checked IR with the Rust interpreter; source/runtime payload checks and existing package tests | This is not an optimizing machine-code backend or general cross-compilation. Exact supported-host artifacts, prerequisites and clean-machine behavior still need qualification. |
| Standard library | Small Rust registry: terminal/arguments, text files, parsing/splitting, array_len and related helpers | Reusable collections, JSON/binary data, paths/directories, dates/clocks/timezones/timers, processes and broader error/resource APIs are needed for ordinary libraries. Avoid app-specific compiler primitives. |
| Servers/networking | Real applications can use the existing private Node HTTP adapter to invoke Sovra behavior | Sovra-owned socket/HTTP lifecycle, partial I/O, bounded parsing, streaming, cancellation/TLS and concurrency are not a production server runtime. Follow the [HTTP dependency plan](REAL_HTTP_APPLICATION_PLAN.md). |
| Browser/web | Browser UI host invokes Sovra through Node; JS code generation is real | Generated JS currently imports Node modules and uses `process`; it is not directly browser-ready. A browser host/backend and its capability contract need independent implementation. |
| Native Windows UI | Compiled C#/WPF host with bounded views/events, process jobs, profiles, focus/accessibility behavior and responsive layout work | Actual current-code Windows interaction, monitor/DPI/screen-reader/theme qualification remain open. Private v1/v2 controls are a limited framework, not a complete public UI toolkit. |
| Other platforms | Portable language/compiler direction and multi-OS CI configuration | No Android/iOS runtime/UI/packaging or native Linux/macOS desktop host is established. WebAssembly is planned. Each target needs real build/run/lifecycle evidence. |
| Editor/IDE | Multi-document browser drafts, check/build/run, JSON diagnostics; compiler-evidence-backed LSP/VS Code standalone locals and checked imported function hover/definitions, package workspace diagnostics | Package editor task targeting/host acceptance, semantic completion, references/rename, type/field navigation, debugging and browser-editor LSP wiring remain planned. Valid-source navigation is not an error-tolerant semantic editor model. |
| Developer commands | `new`, `check`, `run`, IR/JS/host-native `build`, interpreter `test`, `doctor`, standalone `fmt` | Windows/Linux guarded formatter writes are enabled. Import-aware single-file formatting works; read-only package graph checks work; guarded package writes work on Windows/Linux; other-host writes remain deferred. `init`, `repl`, `install`, `update`, `doc` are reserved. Do not confuse the Windows installer with a Sovra package-install command. |
| Installation | Integrity-checked Windows staging, optional developer tools, explicit upgrade/retained backups/receipt-bound rollback | Portable transaction tests are not Windows installer qualification. Uninstall, publisher authentication, support policy and verified clean-machine lifecycle remain open. |
| Quality/security | Rust/CLI tests, backend parity, dogfood, mutation/resource tests, isolated source/native-IR fuzz harnesses and CI gates | Sustained fuzzing, broader property tests, performance baselines, independent review and current exact-target evidence are still required. No claim of being the “most secure language” is supported. |
| Documentation/community | Dual MIT/Apache-2.0 licensing, contributing/conduct/security docs, ADRs, course and examples | Historical/current/aspirational status is mixed; newcomer tasks, maintainership/triage/release policy and reachable private reporting channels need clearer ownership. |

## Highest-priority work

### 1. Make this foundation reviewable and its claims trustworthy

1. Split the accumulated working tree into coherent reviewable changes, include
   new/untracked runtime files, run the exact supported-host checks and make the
   reviewed source available upstream. Outside contributors cannot build on
   unpublished local capabilities. This audit does not itself push or publish.
2. Run Windows/MSVC, legacy PowerShell, WPF, real packaged upgrades/rollback and
   downloaded-artifact checks against the same revision. Qualify a clean Windows
   machine and document the actual supported OS/architecture/framework baseline.
3. Review language/tool/runtime version policy before publication. Cargo's
   `1.0.0` does not establish stable language/API guarantees or release approval.
   Preserve existing tags/milestones; decide policy explicitly rather than
   silently changing them. Version equality alone cannot distinguish edited
   compiler/runtime/tool builds sharing one version.
4. Keep one current capability/qualification summary and link historical
   checkpoints from it. Label proposal examples conspicuously, especially
   Fielddesk and the aspirational command flow in developer-experience docs.

Gate: a new developer can reproduce the advertised subset from a clean checkout
or exact supported artifact, and each support claim points to matching evidence.
Task Manager feature work stays paused until its foundation gates are satisfied.

### 2. Make external contribution practical now

Provide a short clone/build/test/run path, a component map, small tasks with
acceptance criteria, and clear separation between platform-independent tests
and Windows-only qualification. Maintainers should create/triage public issues
from the task board, select review owners and explain expected review/release
cadence. Do not require someone to understand every application prototype to
fix a diagnostic, test or documentation defect.

Before wider promotion, publish reachable private security/conduct reporting
channels and supported-version/response policies. Existing documents say
“maintainers” but do not provide a concrete private contact or confirm GitHub
private-reporting configuration. Do not invent addresses or assume the feature
is enabled. Clarify review authority, decision/ADR process and third-party
contribution licensing; do not introduce an unsolicited CLA or new license.

Gate: a first contributor can pick one bounded issue, run relevant checks and
submit a focused PR without guessing which statements describe current behavior.
Experimental contributions can begin before production deployment is approved.

### 3. Complete the compiler/project foundation before growing application syntax

Retain checked symbol identities, expression types and source provenance in a
deliberate internal representation; reuse that evidence in lowering and IDEs.
Migrate supported project declarations from scanning toward structured checked
nodes, incrementally sharing executable semantics. Extend actual multi-file
package loading and cross-file diagnostics before pretending a project check is
an application runtime. Keep unresolved/unsupported constructs visibly incomplete.

Finish resource coverage for optional service/body reads, aggregate reports and
metadata, and repeated semantic work; the existing per-file/graph limits do not
establish a total process budget. Keep input rejection and operational failures
distinct, and review compatibility before changing accepted resource policies.

Gate: representative multi-file projects have consistent check/run/build/test/
IDE results, with negative visibility/type/import cases and backend parity.
Fundamental type/memory/syntax/compatibility choices still need reviewed decisions.

### 4. Build the library/runtime/package capabilities ordinary apps require

Prioritize an explicit error/resource model, useful collections and JSON/binary
data APIs, deterministic local dependency locks, then host I/O/lifecycle needed
by real services. Design cancellation/cleanup before promising async networking.
Add independently authored local libraries with stable exports and examples;
then design registry ownership, integrity, caching, offline operation and
publication. A registry is not a substitute for reliable module resolution.

Preserve approved array/record value semantics. This audit does not select GC,
borrowing, exceptions, a generic syntax, a UI ABI or an async model. Databases,
auth providers and business entities belong in libraries/platform APIs rather
than compiler keywords.

Gate: independent libraries can be reused reproducibly, and a real server can
own a socket, process bounded requests, report failures and shut down reliably
on each claimed host. Merely checking route metadata does not meet the gate.

### 5. Qualify platforms as separate tracks on the same portable language core

- **Windows first:** native UI interaction, accessibility/DPI, failure handling,
  installed toolchain/IDE compatibility and upgrade/rollback evidence. Retain
  compatibility hosts until replacements are qualified.
- **Web:** execute a real program in a browser without Node imports/process
  assumptions; define browser-host capabilities and unsupported-operation errors.
  Choose a browser JS/Wasm approach through design and evidence.
- **Servers:** implement runtime-owned networking/lifecycle rather than treating
  the private Node request adapter as a production server platform.
- **Linux/macOS:** qualify compiler/tools/artifacts and develop native desktop
  adapters where required; working CLI/Node code is not native GUI support.
- **Android/iOS:** define the minimal runtime/UI/device/lifecycle/packaging
  boundary, run shared logic on an emulator/device, then test suspend/resume,
  storage/permissions, packaging and distribution on each target. Framework and
  backend selection remain design decisions.

Gate per target: build/install/run a real application, exercise failure/cleanup
and platform capabilities, and reproduce the result from the published artifact.
Do not require simultaneous completion of all targets before a scoped release.

### 6. Establish sustainable language quality and stability

Expand differential/property/fuzz coverage with minimized regressions; keep
unresolved security findings private. Measure parse/check/build time, runtime,
startup, memory and package size using a documented corpus/toolchain/machine.
The existing Task Manager benchmark is not a language performance baseline.
Optimize measured bottlenecks before selecting a new optimizing backend.

Maintain explicit compatibility/deprecation and release notes, migration guides,
known limits, reproducible CI/artifacts and a rolling support matrix. Publish
benchmark/security claims only within the evidence actually collected.

## Contributor task board

The user-requested [18-task completion checklist](18_TASK_COMPLETION_CHECKLIST.md)
now makes installed tooling, native distribution, general stdlib, HTTP, unified
UI and distributed packages explicit acceptance criteria within these same IDs.
Completed foundation scopes remain completed; unchecked product criteria remain open.

Progress (2026-11-11): **G01–G03 and G06–G07 are complete.**
The [examples catalog](../examples/README.md) labels all example directories and
has real compiler/backend smoke coverage. Repository-local Markdown targets are
checked in CI; external URLs and heading fragments are explicitly outside that
checker. Platform-specific commands retain their prerequisites/qualification
limits. G02 adds CLI/JSON/human and LSP regressions combining astral/combining
Unicode, tabs, escaped source and CRLF with valid/invalid cases and stable codes.
G03 adds deterministic pending-query cancellation isolation, close/reopen with
reset versions, exactly-once responses and rejected-buffer evidence clearing.
G04 is **deferred at the user's request** because Windows is unavailable.
G05 is **partial**: portable transaction evidence and a
[clean-machine checklist](INSTALLER_QUALIFICATION.md) are available; distinct-version
Windows/PATH/profile execution remains pending. G06 adds the versioned
[compiler/runtime corpus](../benchmarks/compiler-runtime/README.md), with checked
outputs, raw startup/check/build/run timings, POSIX process peak RSS and machine/
toolchain/input identities. Initial release measurements pass on Linux; other
hosts, isolated stage profiling and larger workloads remain future work.
G07 adds checked-IR trivia invariance and structural/canonical native-image
properties, four passing ordinary security tests and bounded sanitizer campaigns
with retained private local evidence. No compiler failure was found; sustained
campaigns and independent review remain ongoing work.
G08 follows [ADR 0041](adr/0041-retained-checked-evidence.md). Slice 1 implements
private immutable checked ownership and structural identities. Slice 2 now
records checked expression/parameter/local types and scoped function/value uses
plus numeric coercion edges with explicit retained-record/name allowances.
Standalone slices 1–3 are implemented: opt-in CLI/IDE projection now consumes
checked identities/types with strict bounded LSP decoding and inferred-local
hover. Slice 4 now adds an explicit standalone checked-lowering API with retained
contexts/results and exact IR/emission-budget parity. As of 2026-11-12, opt-in
checked package compilation also preserves imported identities and cumulative
metadata/shared emission budgets. Fourteen-case Linux overhead/shape observations and exact quota/retry
regressions are recorded. A concrete production-limit policy is proposed in
[the evidence policy](reference/checked-evidence-policy.md); qualification remains
partial. Initial standalone node/type saturation passes, and additive E3025
diagnostic conversion preserves existing errors without invented locations.
Combined metadata/lowering and five-unit graph observations now pass exact IR
parity in a 16-case candidate-profile corpus. Aggregate source/actual-IR boundary
checks now cover a 1,000,000-node graph, exact linked budgets, one-below IR budgets
and cumulative evidence exhaustion. Next: active-limit/compatibility decision, then
standalone migration is now implemented with active limits and E3025 exhaustion.
Ordinary package callers now also use active cumulative evidence limits.
G08 portable behavior/Linux qualification is now complete: ordinary Rust,
property/fuzz, performance, relocated tooling and real VS Code host evidence are
recorded in ADR 0041. G09 package-aware LSP/navigation is now Linux-qualified for bounded existing entries. Next: **G10 formatter qualification**. Separate Windows
G04/G05 and release-platform gates remain unchanged.
Previously implemented pieces of later tasks do not substitute for their stated
platform/acceptance gates.

These are repository notes, not GitHub issues or assigned ownership. Small tasks
can start now; architecture/release tasks require maintainer decisions. “Done”
means the listed behavior is verified, not that a placeholder file exists.

| ID / size | Task and starting point | Acceptance criterion |
| --- | --- | --- |
| G01 / small / complete | Fix current-vs-proposed documentation links (`docs/`, examples) | Catalog/classification, current command/status corrections, local-link checks and executable example smoke tests are implemented; platform execution limits remain explicit. |
| G02 / small / complete | Add a focused diagnostic/Unicode range regression (`tests/cli.rs`, LSP) | Valid/invalid combined-Unicode/CRLF cases verify exact byte/scalar/UTF-16 locations and diagnostic clearing without code drift. |
| G03 / small / complete | Add LSP edit/cancellation/close cases (`scripts/test-lsp.mjs`) | Batched pending edits/cancellation/close/reopen verify fresh symbols, exactly-once responses, diagnostic clearing and no symbol evidence for rejected buffers. |
| G04 / desktop UI; Windows qualification deferred | Native desktop UI contracts/adapters and Windows responsive qualification | Actual Windows narrow/wide layout, drafts, focus and failure evidence attached with source/compiler hashes. |
| G05 / installed toolchain, Windows / partial | Qualify independent installed tools and upgrade/rollback lifecycle | Portable fixture report and procedure ready; distinct real old/new versions, user PATH/profile and clean-machine evidence still required. |
| G06 / medium / complete | Add a repeatable compiler/runtime benchmark corpus | Versioned five-case corpus, verified outputs, 21 measurements with five raw time/memory samples each, hardware/toolchain/source identities and explicit measurement limits; no speed thresholds. |
| G07 / medium / complete | Extend frontend/native-image property/fuzz regressions (`fuzz/`) | Unicode-trivia IR/diagnostic properties and structural image mutations pass ordinary tests; bounded ASan/LSan source/image campaigns pass with hashed logs/binaries. No compiler finding required minimization; infrastructure-failed logs retained separately. |
| G08 / medium / complete (portable behavior/Linux qualification) | Retain checked inferred type/symbol evidence ([ADR 0041](adr/0041-retained-checked-evidence.md)) | Immutable facts and bounded IDE projection feed ordinary standalone/package lowering with active quotas. Exact IR/runtime/diagnostic and resource-boundary tests, fuzz/property checks, versioned overhead corpus, installed tooling and real VS Code Linux host pass. Separate release-platform gates remain. |
| G09 / medium / complete: portable/Linux bounded scope | Package-aware LSP and navigation ([ADR 0042](adr/0042-unsaved-package-sources.md)) | Borrowed bounded unsaved entry buffers compile through canonical checked graphs without disk writes. Compiler-owned context and bounded overlay transport now drive workspace diagnostics with dependency edit/close/rejection invalidation and correct file ownership. Standalone/relocated LSP and pure host-path regressions pass. Checked imported function navigation now uses canonical targets and verified source fingerprints, with bounded unavailable evidence. Compiler-owned package tasks and real VS Code package-workspace acceptance now pass on Linux; Windows/macOS, MSRV and release gates remain separate. |
| G10 / medium / partial: package behavior/Linux qualification | Package-aware and other-host formatter qualification | Linux guarded writes now qualify mode/identity/parent conflicts, link/read-only/special-mode refusal and no-op timestamps, alongside execution preservation. Import-aware single-file formatting now shares package parsing and checks import structure/order. Bounded package checks and guarded writes now preflight/stage canonical entries, verify snapshots/staging and report late partial publication without rollback. Exact-host qualification reports now retain compiler/source hashes, command results and logs in three-OS CI, including failures. Opened-handle validation and Linux nonblocking/no-follow reads now cover FIFO/link substitution races. macOS disabled-write reports do not qualify writes. macOS writes and actual Windows evidence remain gates. |
| G11 / large, partial: supported syntax + opt-in coverage | Structured M12 application nodes | Bounded alias/record AST nodes now reuse executable parsing and retain original spans, with explicit unparsed ranges and optional metadata exhaustion. This is a library-only syntax API, not full checking. Callable/body ASTs and service containers/prototypes now have bounded library syntax APIs and explicit coverage gaps; signatures are not resolved contracts. Dotted import nodes and opt-in same-snapshot checker coverage now retain separate semantic results and explicit metadata failures. Entry-only app container/binding nodes now reuse existing route/page/schedule/list/auth scanner grammar, retaining unknown fields as gaps. Combined entry/declaration syntax now uses original-token filtering and preserves unknown ranges; existing APIs remain strict. Opt-in combined project coverage now shares source snapshots/quotas while preserving semantic and incomplete-check diagnostics. Self-contained ordinary functions beside entries now reuse core semantic analysis through a scoped source API; imports/services/tasks explicitly require additional context. Structured route/page method/path/duplicate checks now share scanner rules through a scoped source API with exact member spans and diagnostic quotas. Unqualified same-source route names now resolve to retained ordinary function identities, with precise E4033 ranges and explicit unavailable ambiguous/import/service/task contexts. Local route placeholders now require explicit same-name handler parameters (E4144), without HTTP decoding, injection or complete argument typing. Local scheduled-declaration targets now retain ordinary/task identity using shared E4071/E4072 rules, without validating task bodies or schedule execution. Independent auth syntax now retains containers/policy headers while every where clause and provider/user/nested form remains unparsed; it grants no permissions. Additive auth-aware source/project coverage now composes these nodes with original-token filtering and shared snapshots/quotas, preserving all guard gaps and E4096 diagnostics. Independent model/enum headers now retain all nonempty bodies as gaps; same-source policy model references resolve only explicit unique Model declarations using shared E4081 rules. Independent page/view headers now retain explicit unresolved annotations and opaque bodies without Unit defaults; local page-route targets resolve only unique Page/View identities using shared E4042 rules. Page-route placeholders now require explicit same-name local Page/View parameters (E4145), preserving target/shape errors without render or HTTP type assumptions. Integrated partial source/project header coverage now composes auth/data/render/entry/declaration nodes under original-token boundary checks and shared quotas, preserving all opaque gaps and E4096. The user-approved [first model-field slice](design/M12_MODEL_FIELDS.md) now has an additive source checker for newline/comma fields, primitives/local primitive aliases, exact duplicate/unknown diagnostics and bounded whole-request refusal. Model identity stays separate from executable records; optional project model-field checks now share checker snapshots, retain file-owned errors and enforce cumulative metadata/diagnostic bounds. Ordinary function checks now also compose beside opaque auth/data/render headers through an additive source API with core diagnostic parity and explicit unavailable excluded-name/import/service/task contexts. Optional project ordinary-function outcomes now retain file-owned diagnostics from shared checker snapshots under cumulative function/diagnostic bounds, without implicit cross-file linking or suppressing E4096. Local route target/path/placeholder rules now also compose beside opaque auth/data/render headers through a source API, preserving gaps and refusing excluded-header target collisions without new namespace semantics. Optional project route outcomes now retain file-owned binding diagnostics from shared checker snapshots under cumulative route/placeholder/diagnostic bounds, including nested diagnostic copies. Default checking remains partial. Extra-parameter/qualified binding, policy/render typing, model/page bodies and default full validation remain. |
| G12 / large, partial / active | Modules, locks and distributed package ecosystem | User approved the [G12 design](design/G12_MODULES_AND_LOCKS.md). Additive [source inventory](reference/package-source-inventory.md) now admits explicit module mappings with bounded deterministic selection, original manifest ownership, canonical containment, observed-size budgets and Unix alias identity checks. Other hosts report unavailable identity checks. Older Rust/context/overlay APIs remain strict; executable CLI commands now select the registered compiler when needed. The additive [source-unit loader](reference/package-source-units.md) now captures bounded owned buffers/ASTs/file-local imports and checks matching headers/module uniqueness with guarded Linux/procfs handle validation. An additive [primitive callable compiler](reference/registered-package-functions.md) now checks all file-owned bodies, resolves direct file-local imports and links bounded checked IR; an independent consumer prints 49 in interpreter/Node JS. Private access, alias coercions, distinct package targets and relocation have regressions. A separate [record-capable API](reference/registered-package-records.md) now compiles nominal records with aliases/imports/construction/returns, private boundary checks and source-owned diagnostics; visible record-valued field edges retain canonical IDs without recursive layout expansion and nested values execute in both engines. CLI check/run/build/test now detect registrations across the declared graph, preserve root-only test discovery and fail without scanner/entry-only fallback; IR/Node execution regressions pass. Exact parsed manifest snapshots now feed deterministic bounded source locks; explicit lock generation/check and locked check/run/build/test consume the same owned inputs with guarded Linux publication. Package formatting now includes registered sources without rewriting locks, and existing standard result records cross registered interfaces. Remaining type contexts, complete context/overlay/navigation/editor integration and exact-host qualification remain open; this does not complete G12 or G11. |
| G13 / large, partial / active | General stdlib: collections/errors/data, filesystem, dates and processes | The user approved the [first structural array/interface contract](design/G13_COLLECTIONS_AND_DATA.md). `[T]` annotations and `std::array_len` now cover executable aliases, locals, parameters, returns and record fields, including package interfaces. Checked conversion preserves recursive numeric widening and snapshot values; privacy, nominal identity, malformed/cyclic/excess-depth types and wrapper admission have regressions. An independent library/consumer executes in interpreter, Node and Linux native output, including locked compilation. Formatter syntax and explicit local IDE type display work; existing inferred JSON labels remain compatible. [Runtime hardening](reference/runtime-values.md) now removes recursive Rust cloning/runtime-owned cleanup and both-engine display, with 16,000-level child regressions and compatible explicit caller cleanup. [Optional instruction/output limits](reference/execution-limits.md) now share exact counters across callees and reject the next instruction/printed line without fallback; UTF-8/LF, large-u64, prior-effect, prologue and deep-cleanup parity has regressions. Existing CLI/native/default policy stays unchanged. [Read-only footprint measurements](reference/value-footprint.md) now report checked logical nodes/text/display bytes with deep/wide/Unicode regressions. The approved [materialization model v1](design/G13_MATERIALIZATION_FUEL.md) now has opt-in Rust/JS cumulative node/text enforcement, atomic admission, no release refunds and explicit host-effect ordering. Physical heap/RSS remains unmetered. Approved [cooperative Rust control](reference/execution-control.md) now adds permanent shared cancellation and elapsed checks at explicit boundaries, with host effects and iterative cleanup retained. An approved [Node Worker generator](reference/javascript-worker-control.md) now adds shared-atomic cancellation and exact elapsed checks with explicit process-stdio boundaries; ordinary script defaults stay unchanged. [Snapshot array_slice](reference/array-slice.md) now preserves dependent element/nominal types across checked and legacy lowering, with strict half-open bounds, iterative selected-child copies, exact fuel and independent package evidence. Approved [snapshot append/concat](reference/array-growth.md) now retain first-directed types and second-argument widening, with checked length limits, full logical charges and independent filtering/package evidence. Browser control transport, full-work/hard interruption, generics, mutating resizing, Map/Set, Option/Result, binary/JSON APIs and exact-platform qualification remain planned. |
| G14 / large, design | Resources, networking, HTTP and server lifecycle | Real loopback requests, partial/malformed input, cancellation, cleanup and supported-host failure tests. |
| G15 / medium/large, design | Browser execution and shared web UI | A real Sovra program runs in-browser through a documented host; no hidden Node dependency. |
| G16 / large, design | Mobile runtime, UI and packaging | Shared checked logic plus real Android/iOS build/lifecycle/device tests; platform-specific limits stated. |
| G17 / maintainer | Publish accurate support/version/reporting policy | Concrete support scope, private reporting channels, review ownership and release authority; no unsupported “stable 1.0” claim. |
| G18 / maintainer + native distribution | Native artifact qualification and reviewed release | Focused reviewed commits, exact-revision CI/MSRV, checksummed artifacts and clean-machine evidence; explicit release decision. |

## Source map for contributors

| Work | Start here |
| --- | --- |
| Lexing, syntax, AST | `src/compiler/lexer.rs`, `parser.rs`, `ast.rs`; executable examples and course |
| Types/visibility and IDE evidence | `semantic.rs`, `semantic/`, `ide_symbols.rs`, `ide_symbols/` |
| Lowering/runtime/JS/native image | `ir.rs`, `ir/`, `interpreter.rs`, `backend.rs`, `numeric_runtime.js`, `native_image.rs` |
| Packages/project inspection | `src/compiler/project/`, `project.rs`; `docs/reference/check-json.md` |
| Stdlib and host file behavior | `stdlib.rs`, `text_files.rs`; parity scripts and file-I/O course |
| CLI/toolchain | `src/cli.rs`, `doctor/`, `formatter/`, `native_build/`, `toolchain/` |
| UI/editor/IDE | `toolchain/ui/shared`, `ui/windows/compiled`, `editor`, `ide`; shared/native fixtures in `scripts/` |
| Packaging/qualification | `.github/workflows`, `scripts/build-*`, `test-windows-*`, `toolchain/install*.ps1`, `rollback.ps1` |

## Validation for this audit

See the development-log entry for fresh command outcomes. Linux validation does
not qualify Windows/WPF, macOS, browsers as a language target, or mobile. Current
MSRV execution is assigned to CI: Rust 1.74 is declared but is not installed in
this workspace. Earlier VS Code/browser/WPF-reference results are separate
evidence, not newly rerun host tests in this audit. No release or marketplace
publication is implied by a green local suite.

Fresh Linux results for this snapshot:

- `cargo test --locked --all-targets`: **570 passed, zero failed/ignored**;
  formatting and strict all-target/all-feature Clippy pass. The doc-test command
  succeeds with zero doctests; it adds no executable documentation coverage.
- Real dogfood corpus: **nine applications pass**; runtime differential matrix
  passes, including the 131-value Float corpus.
- Formatter: eight real programs/both engines, 261 comment placements and
  resource/fixed-point cases pass. Language test-runner integration passes.
- Relocated editor/LSP, VSIX admission, compiler symbol evidence, 115 UI protocol
  checks and the responsive component's two-engine source parity pass.
- Actual PowerShell installer transaction tests pass on Linux. No fresh Windows
  package/GUI, macOS, Rust 1.74, mobile or browser-language execution is asserted.
- The contribution quickstart's greeting, JSON source check, project creation
  and generated language test were exercised locally successfully.
