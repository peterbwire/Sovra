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
| G13 / experimental Linux foundation complete | General stdlib: collections/errors/data, filesystem, dates and processes | Explicit generic/collection/data profiles implement checked abstractions, Option/Result/Error, collections including Byte keys, immutable bytes/codecs, recursive strict JSON and version-3 filesystem/date/clock/timer/process adapters. Data package Check/Run/Test and word-2047 native execution have independent Linux consumers with source deletion and no Node/PATH. Host authority is external, explicit and denied by default; process cancellation kills/reaps owned children and reports cleanup failure. See [data/host scope and limits](reference/data-host-v3.md). Current frozen-source Linux qualification passes 2,273 Rust tests, exact Rust 1.74.0 and 16 Node scripts. Generic/data CLI/package and LSP formatting is implemented; the formatter and consolidated Linux regression gaps are closed. Newer opt-in [generic package tasks](reference/generic-package-tasks.md) now select registered sources and dependencies for Check/Run with related-only saves and dirty-manifest refusal. A separate [generic package diagnostics](reference/generic-package-diagnostics.md) opt-in now checks unsaved entries/modules/dependencies with source-owned diagnostics and generation invalidation. [Checked package function navigation](reference/generic-package-navigation.md) now provides imported call definitions/hover and function outlines; [type annotation navigation](reference/generic-package-type-navigation.md) also preserves imported and alias identities. Body types, local bindings and [record fields](reference/generic-package-field-navigation.md) now navigate through checked identities. The full result above includes these newer package changes. E2/C1 implementation and Linux compiler/LSP qualification are complete for the documented experimental profiles; actual VS Code 1.106.0 package-host acceptance also passes. G13 is complete for the documented experimental Linux scope. Additional OS runtime, browser/mobile and release gates remain separate. |
| G14 / large, active partial | Resources, networking, HTTP and server lifecycle | A trusted Rust IPv4 loopback adapter now owns denied-by-default exact-port grants, listeners/streams, bounded partial binary I/O, EOF, deadlines/cancellation and cleanup with independent TCP-client tests. Explicit Rust-owned version-four requests now connect checked source execution with tested binary echo and scope cleanup. External v3/v4 policy dispatch now enables independent CLI/native loopback byte serving without embedded grants or Node/project startup dependencies. A bounded borrowed fixed-length HTTP/1.1 request parser now validates framing and partial input; pure bounded final-response construction also enforces framing and HEAD/bodyless status rules. One-request HTTP socket integration and real Sovra CLI/native handlers now pass independent fixed-length/chunked Unicode HTTP/JSON client acceptance with shared head/decode work and deadlines. Full protocol/production and wider supported-host qualification remain open; see [socket foundation](reference/loopback-network-v4.md). |
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

G13 v2 tooling checkpoint (2026-11-14): explicit source and package formatting
now use v2 syntax safety gates with shared guarded file/publication controls;
[v2 formatting](reference/experimental-v2-format.md) documents graph selection
without implying dependency/module compilation. Opt-in standalone v2 IDE mode
is also available. G13 remains partial; linking/backends and later library phases
retain their completion gates.

G13 v2 native checkpoint (2026-11-14): explicit source-backed executable emission
now embeds a versioned bounded source image and rechecks/lowers at startup, with
finite VM limits and no file/stdin services. See [native scope](reference/experimental-v2-native.md).
This is not an executable IR codec or optimizing backend; the remaining generic
collections/error, JSON/codec, filesystem/time/process and linking gates stay open.

G13 v2 function-tooling checkpoint (2026-11-14): successful standalone JSON
checks can explicitly project bounded checked root-function declarations and
canonical call links. The opt-in v2 LSP now supports root function hover,
definition and outline; [navigation scope](reference/program-symbols-v2.md) states
remaining local/type/package facts and exact host qualification. G13 stays partial.

G13 v2 scoped-tooling checkpoint (2026-11-14): standalone symbol output now
includes scope-owned parameters, local declarations, match payloads and their
read/write/scrutinee targets, with shared optional quotas and checked type hover.
See [binding scope](reference/program-symbols-v2.md). Type/field/package navigation
and the remaining library phases remain open; G13 is still partial.

G13 v2 nominal-tooling checkpoint (2026-11-14): standalone full symbol output
now includes enum/variant declarations and constructor/match-arm variant references
from canonical IDs/coverage plans, with correct enum/member outline kinds and
shared quotas. See [nominal navigation](reference/program-symbols-v2.md). Alias/record/
annotation/field/package navigation and the remaining library phases stay open.

G13 nominal alias checkpoint (2026-11-14): validated enum alias chains now work
as constructor/match qualifiers through checked canonical IDs, with existing
payload rules and charged signature/index metadata. See [alias scope](reference/enum-aliases-v2.md).
CLI, formatter, IDE and source-backed Linux native evidence cover this bounded
feature; generics/structured errors and the remaining G13 phases stay open.

G13 type-module checkpoint (2026-11-14): one-level inline modules can own enum,
alias and record declarations used by root functions, with original spans,
qualified nominal IDs and early qualifier-work admission. See [type-only module
scope](reference/type-modules-v2.md). CLI/formatter/IDE and source-backed Linux
native evidence cover this subset; module functions/imports and G13 remain open.

G13 module-body syntax checkpoint (2026-11-14): raw v2 parsing and formatting
retain module function bodies with original spans and namespace paths. Checked
source/CLI execution/native decoding continue to reject those bodies; no checked
facts are fabricated. Scope, visibility, call linking and lowering remain open.
See [syntax-only module bodies](reference/type-modules-v2.md).


G13 checked module-function checkpoint (2026-11-14): the program API, explicit
v2 CLI, VM, IDE and source-backed Linux native host now support qualified
same-file module functions with private/exported call filtering. This supersedes
the syntax-only execution restriction above. Root entry/test discovery and bounded
metadata remain guarded. Conflicting root-type shadows, nested modules, imports
and dependency linking remain open; G13 is partial. See
[checked module functions](reference/module-functions-v2.md).


G13 IDE checkpoint (2026-11-14): checked same-file module containers now appear
in the flat outline with original source/name ranges and shared projection
quotas. Invalid bodies still clear all facts. This does not complete imports,
nested modules, dependency linking or G13. See
[checked navigation](reference/program-symbols-v2.md).


G13 IDE declaration checkpoint (2026-11-14): root/module records and type aliases
now have checked outline entries, original ranges and aggregate quota coverage.
Type-use/field navigation and remaining G13 language/package work stay open. See
[checked navigation](reference/program-symbols-v2.md).


G13 enum header navigation checkpoint (2026-11-14): direct enum parameter/return
annotations now navigate through checked module scope and canonical enum IDs,
including array wrappers. Alias/local/record/field targets and remaining G13
package/language phases stay open. See
[checked navigation](reference/program-symbols-v2.md).


G13 E2 syntax checkpoint (2026-11-14): the additive Rust
[generic annotation parser](reference/generic-type-syntax-v2.md) retains bounded
qualified applications/arrays and original ranges. This is not generic semantic
or executable support. The pushed checkpoint passed 1,294 tests over all 99 Cargo
targets in batches. Later exact-source qualification is recorded separately.
G13 remains partial, including E1 package/backend work and E2/C1/D1/H1/H2/Q1.


G13 E2 parameter checkpoint (2026-11-14): bounded generic parameter lists now
retain distinct source owners, `std::Key` spelling and bounded annotation-use
identities. This is not checked/executable generic declarations. Declaration
attachment, semantic arity/bounds, specialization and the remaining G13 work stay
open. See [parameter ownership](reference/generic-parameters-v2.md).


G13 E2 alias checkpoint (2026-11-15): root generic aliases now retain attached
parameter owners and bound target annotation uses, with original full-source
coordinates and cumulative admission. This is syntax-only; generic semantics,
function/record/enum attachment and remaining G13 phases stay open. See
[generic alias syntax](reference/generic-alias-syntax-v2.md).


G13 E2 record checkpoint (2026-11-15): generic record syntax now shares one
parameter owner across source-owned fields, with original-coordinate mapping
and cumulative field/type/source/name/use admission. Semantic record checking,
generic function/enum attachment, specialization and remaining G13 phases stay
open. See [generic records](reference/generic-record-syntax-v2.md).


G13 E2 enum checkpoint (2026-11-15): root generic enum syntax now retains one
parameter owner across nullary/single-payload variants with cumulative resource
admission and original coordinates. This is not generic enum typing or execution.
Function attachment, semantic checking/specialization and remaining G13 phases
stay open. See [generic enums](reference/generic-enum-syntax-v2.md).


G13 E2 function checkpoint (2026-11-15): generic function headers now share one
parameter owner across explicit value/return annotations and retain bounded opaque
body ranges. Root syntax attachment now covers all four declaration categories;
this is not semantic/body checking or generic execution. See
[generic function headers](reference/generic-function-syntax-v2.md).


G13 E2 resolution checkpoint (2026-11-15): source-owned generic type headers now
support bounded annotation name/arity and initial concrete/symbolic Key proofs.
Unexpanded alias Key arguments refuse explicitly. Whole declaration/body validity,
module/package contexts, substitution and generic execution remain open. See
[generic type resolution](reference/generic-type-resolution-v2.md).


G13 E2 alias expansion checkpoint (2026-11-15): prepared alias environments now
check target names/arity/cycles/symbolic bounds and expand arguments by owner-scoped
identity under cumulative metadata/work and fixed depth limits. Transparent alias
Key proofs work without changing runtime ownership. Complete declaration/body
checking and generic execution stay open. See
[alias expansion](reference/generic-alias-expansion-v2.md).


G13 E2 declaration checkpoint (2026-11-15): complete registered root type
collections now check every original record field and enum payload, including
unused definitions, under cumulative budgets. Empty enums and missing/foreign
owners refuse. This does not establish function/body checking, runtime layout,
constructors or generic execution. See
[generic definition annotations](reference/generic-declarations-v2.md).


G13 E2 signature checkpoint (2026-11-15): all supplied root generic function
headers now have sealed signature proofs against complete checked type definitions,
with table-owned IDs and cumulative annotation budgets. Implicit Unit stays
source-absent. Bodies remain unchecked and cannot be lowered/executed from this
proof. See [generic signatures](reference/generic-signatures-v2.md).


G13 E2 type-equivalence checkpoint (2026-11-15): bounded iterative comparison
and exact normalized in-process keys now handle alias-expanded roots without
confusing graph indices or source names. This is a prerequisite for body checking
and specialization, not executable generic support. See
[type equivalence](reference/generic-type-equivalence-v2.md).


G13 E2 symbolic body checkpoint (2026-11-15): a sealed initial body profile now
checks every signature's immutable inferred copies, returns and equality against
symbolic types, preserving source scopes and finite cumulative admission. Other
constructs refuse; this is not specialization or executable generic support. See
[generic copy bodies](reference/generic-copy-bodies-v2.md).


G13 E2 local-annotation checkpoint (2026-11-15): symbolic immutable locals now
accept and check explicit generic/alias types with original ranges and cumulative
fragment/expansion admission. The private adapter does not enable generic types
in public legacy Program parsing. Calls, mutation, broader bodies, specialization
and generic execution remain open. See
[local annotations](reference/generic-local-annotations-v2.md).

Concrete numeric arithmetic and ordering now have bounded symbolic body facts;
see [numeric body profile](reference/generic-numeric-bodies-v2.md).
Opaque numeric assumptions and generic execution remain open.

[Boolean symbolic body expressions](reference/generic-logical-bodies-v2.md)
now check both operands and retain short-circuit operator identity. Executable
short-circuit lowering and broader generic execution remain open.

[Concrete String symbolic expressions](reference/generic-string-bodies-v2.md)
now cover concatenation and lexical ordering without implicit formatting.
Runtime allocation admission and broader generic execution remain open.

[Mutable symbolic locals](reference/generic-mutable-bodies-v2.md) now support
fixed-type direct assignment with explicit widening and binding identity.
Field/index mutation, calls, control flow and generic execution remain open.

[Symbolic expression statements](reference/generic-expression-statements-v2.md)
now retain checked discarded results without conversions or fabricated bindings.
Calls, control flow, specialization and executable integration remain open.

[Symbolic conditional checking](reference/generic-conditional-bodies-v2.md)
now checks concrete Bool conditions, isolated branch scopes and all-path return
coverage using a flat proof/worklist. Loops, calls and generic execution remain open.

[Symbolic while checking](reference/generic-loop-bodies-v2.md) now validates
concrete Bool conditions, isolated loop scopes and fixed parent assignments
without adding guaranteed returns. Calls, specialization and execution remain open.

[Read-only symbolic array indexing](reference/generic-index-bodies-v2.md)
now preserves exact element types and original parameter/provenance owners.
Bounds remain runtime checks; construction, indexed writes and execution remain open.

[Symbolic indexed assignment](reference/generic-index-assignment-v2.md)
now checks direct mutable array locals, concrete Int indexes and exact element
replacement types. Runtime bounds/snapshot ownership and execution remain open.

[Context-typed symbolic array literals](reference/generic-array-literals-v2.md)
now check nested/empty arrays from typed local, return and assignment contexts
with explicit item widening. Inferred arrays, calls and execution remain open.

[Parameter-independent symbolic record fields](reference/generic-field-bodies-v2.md)
now preserve nominal field/receiver identity after a bounded reachable-type walk.
Dependent fields require checked substitution; field writes and execution remain open.

[Direct field-parameter projection](reference/generic-field-arguments-v2.md)
now selects validated receiver arguments while preserving caller parameter and
provenance owners. Composite substitution, calls and execution remain open.

[Composite record field substitution](reference/generic-field-substitution-v2.md)
now preserves caller parameters and per-node input/alias sources under independent
and cumulative node/edge/work/depth limits. Calls and execution remain open.

[Explicit call/constructor type-argument syntax](reference/generic-type-arguments-v2.md)
now admits bounded source-owned `::<T,E>` lists in one shared arena.
Callee binding, checked call facts, specialization and execution remain open.

[Caller-scope explicit type arguments](reference/generic-argument-checking-v2.md)
now resolve names, aliases and type-head Key constraints with exact caller owners
and cumulative copy/expansion budgets. Callee binding and execution remain open.

[Generic callee type matching](reference/generic-call-binding-v2.md) now verifies
exact table/environment/caller owners, arity and callee Key requirements under
matching budgets. Value matching, body calls and execution remain open.

[Bounded callee signature substitution](reference/generic-signature-instantiation-v2.md)
now produces caller-scoped value-parameter/return types with per-node provenance
and cumulative/per-type graph limits. Value/body calls and execution remain open.

[Generic value-argument type matching](reference/generic-call-values-v2.md)
now verifies exact caller body/expression owners, value arity and substituted
parameter compatibility with explicit widening. Source body calls/execution remain open.

[Source-backed generic call envelopes](reference/generic-call-syntax-v2.md) now
retain original callee/type/value ranges with grammar-checked explicit types and
opaque balanced values. Body expression/call integration and execution remain open.

[Original-token call value expressions](reference/generic-call-expressions-v2.md)
now parse exact original ranges with bounded AST/work admission and iterative
cleanup. Caller-scope typing, source-call facts and execution remain open.

[Symbolic arena identities](reference/generic-arena-identities-v2.md) now prevent
IDs from separate checks of the same signature being interchanged. Clones retain
their arena; future call-source expression facts must use distinct arena owners.

[Source-backed symbolic body calls](reference/generic-body-calls-v2.md) now
connect explicit root syntax, active lexical scopes, type/bound/signature stages
and contextual values under cumulative budgets. Runtime/specialization remain open.

[Explicit symbolic record construction](reference/generic-record-construction-v2.md)
now checks nominal arguments, complete field coverage and contextual values with
shared source/provenance and cumulative budgets. Layout/runtime integration remains open.

[Explicit symbolic enum constructors](reference/generic-enum-construction-v2.md)
now preserve nominal/variant identity and nullary versus Unit payloads under
shared input/context budgets. Match plans, specialization and execution remain open.

[Generic enum pattern headers](reference/generic-enum-patterns-v2.md) now prove
exact selected enum coverage/payload contexts under bounded source/graph/work
admission. Active match-site scope, arm typing/return coverage and execution remain open.

[Symbolic generic match bodies](reference/generic-match-bodies-v2.md) now connect active lexical
scope, one scrutinee read, immutable payload IDs, exhaustive arm typing and
conservative return coverage under shared body/graph/work budgets. Nested matches
and explicit calls retain original source ranges. Generic specialization, executable
lowering/runtime/backend integration and structured error propagation remain open.

[Finite concrete generic call discovery](reference/generic-specialization-v2.md) now admits
source-owned nongeneric roots and discovers exact argument instances with bounded
substitution, normalized reuse and recursive closure. Defaults cap standalone
instances at 1,024 and the explicit package profile at 4,096; no package linking
is performed. Concrete signature/body rechecking, layout plans, executable
lowering/runtime/backend integration and structured error propagation remain open.

[Concrete generic body rechecking](reference/generic-concrete-bodies-v2.md) now substitutes
instance signatures/bindings/expressions and rechecks operation, conversion, call,
constructor/field and match-payload contexts under cumulative generation/comparison/
work budgets. Original lexical ownership/return-flow proofs remain sealed; concrete
lookup selects a graph instance before using symbolic IDs. Literal decoding,
nominal/runtime layouts, executable lowering/backend integration and structured
error propagation remain open.

[Decoded generic literal payloads](reference/generic-literals-v2.md) now retain exact
Int/finite Float bits/Bool/decoded UTF-8 values from original source ranges, sharing
immutable facts across instances while charging each reference. Existing conversion
edges remain explicit; source/token/string-copy/work admission is finite. Runtime
layouts, constructor/match plans, executable lowering/backend integration and
structured errors remain open.

[Logical generic runtime layouts](reference/generic-layouts-v2.md) now map concrete
source facts to finite primitive/array/record/enum graphs with exact normalized
identity, declaration-order slots/tags and recursive edges. Function and nominal
type instances share the 1,024 standalone / 4,096 package-profile cap. Input/member
arenas, generated metadata, references and lookup work stay bounded. No physical ABI
or executable path is selected; constructor/match plans, lowering/backends, package
linking and structured errors remain open.

[Generic nominal operation plans](reference/generic-construction-v2.md) now link source-order
record initializers, enum tags/payloads and field reads to exact instance layouts.
Match dispatch retains one source read, immutable payload destinations, source-order
arms and exhaustive tag mappings with source-contained statement ranges. Planning
is cumulative and bounded; executable IR/control-flow lowering, runtime allocation/
snapshot behavior, backend/package integration and structured errors remain open.

[Typed generic instruction lowering](reference/generic-ir-v2.md) now emits owned
stack instructions with resolved control flow, source-order operands, concrete
calls, short-circuit branches and enum dispatch. Function/slot/target structures
and reachable typed-stack joins are checked under finite emission/pending/state/
work budgets. Scalar test probes verify behavior; production runtime/backend
execution, resource/value policy and source/program/package/CLI integration remain
open alongside structured errors.

[Host-free generic Rust execution](reference/generic-runtime-v2.md) now runs sealed
IR from a zero-argument root with iterative calls and flat owned compound values.
Snapshots, indexed replacement, enum payloads, equality and deep cleanup retain
value semantics under shared existing materialization/instruction/control policy
and explicit work/frame/slot/operand caps. Borrowed result inspection grants no
host services. Source/program/package/CLI/native/JS integration, validated external
arguments and structured errors remain open.


G13 E2 source checkpoint (2026-11-15): [standalone generic source execution](reference/generic-source-v2.md)
now collects complete admitted files and checks all declarations/bodies before
specializing and executing a zero-argument root. Ordinary nongeneric calls share
the bounded pipeline; original whole-file error locations and cumulative syntax
quotas are retained. This is a Rust library API. CLI/package/native/JavaScript
integration, standard Option/Result, propagation and later G13 phases remain open.


G13 E2 CLI checkpoint (2026-11-15): [explicit standalone generic commands](reference/generic-cli-v2.md)
now expose bounded `check` and host-free `run` under `--experimental-generics`.
The CLI requires a parameterless Unit main for execution, retains original-file
errors, supports existing check JSON and refuses unavailable hosts, packages,
program arguments and engine combinations. Package/backend/tooling integration,
standard Option/Result and propagation, and later G13 phases remain open.


G13 E2 native checkpoint (2026-11-15): [generic host-native packaging](reference/generic-native-v2.md)
now links explicit standalone generic builds as Rust executables with bounded
embedded-source rechecking and host-free VM execution. Separate image dispatch
preserves legacy/plain-v2 formats; source/IR quotas and command policy remain
finite. This is source-backed execution, not direct generic machine-code lowering.
Package/JavaScript/tooling integration, standard errors/propagation and later
G13 phases remain open; other-host qualification remains a gate.


G13 E2 root-package checkpoint (2026-11-15): [guarded generic root packages](reference/generic-packages-v2.md)
now connect check/run/native build to captured single-entry projects and optional
source-lock verification, with generic lock generation/checking. Source-read caps
can reduce existing guarded admission; canonical diagnostic provenance is retained.
Runtime generic errors use E3125, reserving existing E3124 for unsupported package
shapes. Dependency/module linking, JavaScript/tooling, standard errors/propagation
and later G13 phases remain open. Package capture still requires supported Linux/procfs.


G13 E2 standard-enum checkpoint (2026-11-15): [qualified Option/Result](reference/generic-standard-v2.md)
now use compiler-owned nominal identities through complete source checking,
constructors, exhaustive matches, specialization and host-free execution. User
Option/Result names stay available; standard templates consume semantic budgets
and have separate provenance. Native feature word 3 advertises standard enums;
old word-1 images retain their prior environment. std::Error, adapters, postfix
propagation, dependency/module linking and remaining G13 work stay open.


G13 E2 standard-error checkpoint (2026-11-15): [structured std::Error](reference/generic-error-v2.md)
now has compiler-owned code/message Strings and optional Int offset, checked
qualified construction, fields and owned snapshots through Result payloads.
Public generic record parsing stays strict; ordinary user Error types stay free.
New native feature word 7 includes Error while retaining old word-1/word-3 profiles.
Explicit adapters, postfix propagation, dependency/module/JavaScript/tooling and
later G13 work remain open. Other-host/installed qualification remains separate.


G13 E2 propagation-contract checkpoint (2026-11-15):
[sealed propagation compatibility](reference/generic-propagation-v2.md) now validates selected body-owned standard Option/Result
operands against enclosing return wrappers under cumulative context/comparison/
work budgets. Successful types may differ; Result error types match exactly.
Original body/provenance and variant/type plans are retained. This is a Rust
planning API only: source question marks, concrete/executable lowering, native
feature integration, adapters and remaining G13 work stay open.


G13 E2 propagation-syntax checkpoint (2026-11-15):
[original-source postfix capture](reference/generic-propagation-syntax-v2.md) now retains bounded operand/question/
result spans for complete blocks, with nested/chained applications and parser
rollback coverage. Private capture publishes no unwrapped legacy AST. Ordinary
lexer/parser, checked source, CLI and existing native profiles remain strict.
Source/body binding, concrete rechecking, early-return/runtime/native integration,
adapters and remaining G13 work stay open.


G13 E2 checked-postfix checkpoint (2026-11-15):
[opt-in symbolic postfix bodies](reference/generic-propagation-bodies-v2.md) now bind original questions to active
body operands and sealed success/failure types. Complete checking covers aliases,
scopes, calls, fields/indexes, control flow and unreachable code under shared
comparison/work/virtual-node/depth admission. Specialization and lowering refuse
question-bearing tables; full-source/CLI/native profiles remain strict. Concrete/
layout/early-return/runtime/native integration, adapters and remaining G13 work stay open.


G13 E2 concrete-postfix checkpoint (2026-11-15):
[opt-in concrete propagation](reference/generic-propagation-concrete-v2.md) now discovers finite closures and recomputes
closed operand/success/failure/return contracts per instance, checking original
symbolic descriptors under shared generated-type/context/comparison/work caps.
Ordinary entry points remain strict; IR rejects every question-bearing source
table, including unreachable questions. Layout/early-return/runtime/native
integration, adapters and remaining G13 work stay open.


G13 E2 propagation-layout checkpoint (2026-11-15):
[bounded propagation operation layouts](reference/generic-propagation-layouts-v2.md)
now retain input, extracted payload, failure tag/payload and caller-return layouts
per concrete source site under shared admission. Early-return lowering and execution
remain gated; executable integration, adapters and remaining G13 work stay open.


G13 E2 propagation-IR checkpoint (2026-11-15):
[inspection-only early-return IR](reference/generic-propagation-ir-v2.md)
now lowers each checked question once, validates standard wrapper/failure contracts
and typed success continuations under shared IR admission. Its separate owner
cannot enter the public VM. Ordinary lowering and runtime remain gated for every
question-bearing source table. Payload execution/cleanup/fuel, source/native
integration, adapters and remaining G13 work stay open.


G13 E2 propagation-VM checkpoint (2026-11-15):
[explicit propagation VM execution](reference/generic-propagation-vm-v2.md)
now copies success/error payloads, rebuilds compatible failure returns and cleans
current-frame slots/operands under shared snapshot/work/control admission. The
ordinary VM remains gated. Executable source/CLI/native integration, adapters
and remaining E2/G13 work stay open.


G13 E2 propagation-source checkpoint (2026-11-15):
[explicit complete-source propagation](reference/generic-propagation-source-v2.md)
now collects original complete files and composes symbolic/concrete propagation,
operation layouts, IR and VM execution under independent shared stage admission.
Ordinary source/CLI/native profiles remain strict; native encoding refuses the
new profile. CLI/native integration, adapters and remaining E2/G13 work stay open.


G13 E2 propagation-command checkpoint (2026-11-15):
[experimental CLI/package/native propagation](reference/generic-propagation-cli-v2.md)
now selects the complete checked propagation source profile. Native feature word
15 enables bounded startup rechecking; words 1/3/7 remain strict. Existing root
package/lock/authority and command-entry restrictions remain. Explicit adapters,
remaining E2/G13 work and other-host qualification stay open.


G13 E2 standard-result adapter checkpoint (2026-11-16):
[explicit source-level legacy result adapters](reference/generic-result-adapters-v2.md)
now admit five immutable legacy schemas and provide ordinary Sovra conversion
functions under existing shared quotas. The experimental CLI uses this profile;
native feature 31 retains it. Builtin-call bridging, other-host execution and
remaining E2/G13 requirements stay open. Existing builtin return types are unchanged.


G13 E2 host-free builtin checkpoint (2026-11-16):
[checked parse_int/split_once calls](reference/generic-builtin-calls-v2.md)
now preserve legacy result records through symbolic/concrete/IR/VM checking and
explicit adapter/question composition. Experimental CLI/native feature 63 selects
this profile; words 1/3/7/15/31 remain unchanged. Other builtins, remaining E2/G13
work and other-host qualification stay open. No host I/O is enabled.


G13 E2 String-builtin checkpoint (2026-11-16):
[checked len/lines_unique/escape_terminal calls](reference/generic-string-builtins-v2.md)
now preserve core semantics with primitive result checks, temporary line-storage
admission and escape-output preflight. Experimental CLI/native feature 127 selects
this profile; older feature environments remain unchanged. to_string, array/host
builtin bridging, remaining E2/G13 work and other-host qualification stay open.


G13 E2 value-builtin checkpoint (2026-11-16):
[checked to_string/array_len calls](reference/generic-value-builtins-v2.md)
now support polymorphic iterative display and generic array lengths. Output and
temporary formatting tasks are admitted under shared budgets. Experimental
CLI/native feature 255 selects the new profile; previous environments stay strict.
Append/concat/slice, host builtins, remaining E2/G13 work and platform gates stay open.


G13 E2 array-builtin checkpoint (2026-11-16):
[checked append/concat/slice calls](reference/generic-array-builtins-v2.md)
now preserve snapshots, generic element identity, recursive numeric widening and
half-open bounds. Explicit IR conversion targets and whole-result copy preflight
use shared admission. Experimental CLI/native feature 511 selects the new profile;
previous environments stay strict. Host bridging and remaining E2/G13 gates stay open.


G13 E2 generic-editor checkpoint (2026-11-16):
[explicit generic editor diagnostics/tasks](reference/generic-ide-v2.md)
now check unsaved original buffers and dispatch standalone Check/Run with the
generic compiler flag. Modes are mutually exclusive; unsupported navigation,
formatting and IR/JS tasks remain gated. Dependency/module/JS integration, broader
editor evidence and remaining E2/G13 gates stay open. Task Manager is unchanged.


G13 E2 generic-symbol checkpoint (2026-11-16):
[checked root declaration symbols](reference/generic-symbols-v2.md)
now project complete original declarations into optional bounded JSON evidence and
generic editor outlines/declaration hover. Exhaustion returns no facts without
changing check success. Call/type/local references, formatting, modules/dependencies,
JavaScript and remaining E2/G13 gates stay open. Task Manager is unchanged.


G13 E2 call-navigation checkpoint (2026-11-16):
[checked direct-call references](reference/generic-call-navigation-v2.md)
now retain original callee tokens and resolved function identities through optional
JSON evidence and generic editor Go to Definition/hover. References cover original
symbolic bodies once, not specialization duplicates. Local/type-use references,
formatting, package overlays and remaining E2/G13 gates stay open.

2026-11-16 checkpoint: [checked generic local navigation](reference/generic-local-navigation-v2.md) adds parameter/local/match-payload definition and role hover. Type rendering and broader E2/G13 gates remain open.

2026-11-16 type-hover checkpoint: [checked generic binding types](reference/generic-local-navigation-v2.md) now display symbolic/inferred types from retained owners with bounded iterative alias-expanded rendering. Type-use navigation and broader E2/G13 requirements remain open.

2026-11-16 header-type checkpoint: [checked generic annotation navigation](reference/generic-type-navigation-v2.md) links user type names in function parameters/returns through sealed registry identities. Broader type-use navigation and E2/G13 requirements remain open.

2026-11-16 functional checkpoint: [generic source and locked-package tests](reference/generic-testing-v2.md) adds bounded test discovery, all-root preflight and fresh sequential VM execution. Editor polish is deferred; dependency/module linking and remaining E1/E2/G13 requirements remain open.

2026-11-16 package-input checkpoint: [syntax-independent graph capture](reference/captured-package-graphs-v2.md) retains guarded per-file dependency/module source snapshots and verifies compatible source locks without the legacy parser. This is not generic import linking; CLI dependency/module refusal remains active. Next: source-owned declarations, import visibility and cross-file identities.

2026-11-16 package-binding checkpoint: [generic declaration ownership and visibility](reference/generic-package-declarations-v2.md) assigns table-owned per-file identities, validates registered module scopes and binds file-local direct dependency imports with private/export boundaries. This is not semantic/executable linking; generic CLI dependency/module refusal remains active.

2026-11-16 package-type checkpoint: [checked package types and signatures](reference/generic-package-types-v2.md) feeds per-source visibility into existing registry/alias/definition/signature semantics with canonical nominal identities and cumulative admission. Scoped body compilation and unscoped function lookup remain gated; generic CLI dependencies/modules remain refused until executable linking is qualified.

2026-11-16 package-body checkpoint: [source-aware package calls and nominal operations](reference/generic-package-bodies-v2.md) checks every body under its own visibility/type context. Explicit-library specialization/IR/VM tests execute imported generic calls, transitive helpers, records, enums/matches and propagation. Selected-root package/diagnostic and CLI/native integration remain open; generic CLI dependencies/modules remain refused.

2026-11-16 selected-root checkpoint: [owned generic package compilation and Check/Run/Lock](reference/generic-package-compilation-v2.md) now executes registered local dependency graphs with locked snapshot custody, root-owned entry selection and cross-file runtime diagnostics. Multi-file Test/native Build and editor package integration remain gated; G13 is not complete.

2026-11-16 package-suite checkpoint: [generic package tests](reference/generic-package-testing-v2.md) now run root-entry tests with registered module/dependency helpers over one immutable checked graph. Every selected specialization is preflighted; shared instance reservation bounds both passes, with fresh per-test VM policy and original-file failures. Locked test reports preserve lock priority and schema-one output. Multi-file native Build remains gated; G13 is not complete.

2026-11-16 native-package checkpoint: [versioned source/topology images](reference/generic-package-native-v2.md) now build and run registered generic package graphs without project-file or Node.js startup dependencies. Embedded inputs retain logical ownership but cannot become guarded filesystem captures. Old single-entry image families remain unchanged. Linux native consumer execution and codec refusals are qualified; broader E1/E2/G13 and other-host/backend gates remain open.

2026-11-16 collections checkpoint: the [snapshot collections reference package](../stdlib/snapshot-collections/README.md) supplies ordered generic maps/sets, explicit logical equality, missing-key Options and structured duplicate-record refusal. Locked Check/Run/Test and native consumer coverage exercise the actual library. This uses ordinary records/arrays and existing VM copy/work limits; raw record equality remains structural. Intrinsic std::Map/std::Set representation, operators and cost qualification remain open; C1 and G13 are not complete. Package body type-argument admission is bounded at 4096 aggregate; standalone and per-list limits remain unchanged.

2026-11-16 opaque-set checkpoint: [primitive-key runtime sets](reference/runtime-sets-v2.md) now enforce private homogeneous storage, ordered snapshots, membership equality and atomic full-result node/text admission with finite helper work. This is an explicit Rust library foundation, not compiler/IR/VM/source/native admission. Opaque maps, collection type integration and full C1/G13 qualification remain open.

2026-11-16 opaque-map checkpoint: [typed runtime maps](reference/runtime-maps-v2.md) now enforce private unique keys and exact primitive/array value shapes, preserve update order, compare associations and atomically admit complete descendant copies. Nominal record/enum ownership and compiler/layout/IR/VM/native collection admission remain open. This is an explicit Rust library API; C1/G13 are not complete.

2026-11-16 nominal-enum map checkpoint: [checked enum map leaves](reference/runtime-maps-v2.md#checked-enum-value-leaves) now retain sealed environment-specific enum identities and copy all type/variant labels and payload descendants under atomic fuel and finite work. Foreign owners, forged text and transitive record payloads refuse. Record ownership and compiler/layout/IR/VM/native collection integration remain open; C1/G13 are not complete.

2026-11-16 sealed-record checkpoint: [checked record runtime ownership](reference/runtime-record-owners-v2.md) now retains environment-specific record IDs, validates exact field/child owners before sealing, preserves IDs through snapshots/extraction and admits owned records in maps and enum payloads. Name-only records remain refused transitively. Record equality charges every candidate field-name search. Generic package-owner adaptation and collection type/layout/IR/VM/native integration remain open; C1/G13 are incomplete.

2026-11-16 generic-owner bridge checkpoint: [proof-carrying generic snapshots](reference/generic-collection-values-v2.md) now capture actual package VM values with their own checked layout/declaration/type-argument evidence. Raw layout numbers and caller names cannot attach foreign proof. Imported homonyms, aliases and phantom arguments remain exact; full subtree copies use existing atomic generic snapshot units and finite work. Generic collection storage/source/layout/IR/VM/native integration remains open; C1/G13 are incomplete.

2026-11-16 generic-map storage checkpoint: [maps of checked generic package values](reference/generic-collection-maps-v2.md) now enforce exact retained value types, preserve insertion order and owners across updates/get/remove/iteration, compare associations, and atomically admit all result descendants before copying. Root/dependency homonyms and phantom/foreign owners refuse. Type profiles remain finite and value copy units inherit the generic snapshot bridge. Source collection types/layouts/IR/VM and native qualification remain open; C1/G13 are incomplete.

2026-11-16 opaque-layout plan checkpoint: [checked collection layout plans](reference/generic-collection-layouts-v2.md) now bind compiler-owned Map/Set constructors to exact checked arguments, enforce the primitive Key bound and instantiate the existing typed storage without caller-supplied key-domain guesses. This separate Rust plan API does not publish executable layout variants or enable source annotations. Native words and ordinary records remain unchanged; C1/G13 stay open pending source/layout/IR/VM/native qualification.

2026-11-16 intrinsic annotation checkpoint: [opt-in package collection semantics](reference/generic-collection-types-v2.md) now registers opaque compiler-owned Map/Set headers and checks real source annotation arity, Key bounds, aliases/nesting and per-file dependency visibility under cumulative limits. Collection definitions are not records/enums; record literals/field projections refuse. Concrete forwarding rechecks, then executable layout publication explicitly refuses. Default compiler/CLI/native profiles remain unchanged; C1/G13 stay open for executable layouts/IR/VM/native qualification.

2026-11-18 collection layout-graph checkpoint: [opt-in closed Map/Set layouts](reference/generic-collection-layout-graphs-v2.md) now retain opaque argument edges, intern aliases/exact owners and close recursive map-value graphs under the existing cumulative instantiation/layout/work limits. Real checked shapes bind typed storage even when producer layout numbers differ. Old planning remains strict; construction/IR explicitly refuses collection-bearing graphs, including phantom arguments. Actual collection builtin/IR/VM operations and native qualification remain open; C1/G13 are incomplete.

2026-11-18 initial collection execution checkpoint: [typed empty collection operations](reference/generic-collection-operations-v2.md) now compile and execute real Sovra map_new/set_new/map_len/set_len through explicit package APIs, checked builtin IR and shared VM budgets/control. Contextual types preserve K/V without defaults; distinct flat empty values copy/compare/propagate normally. Ordinary execution and native encoding explicitly refuse the new profile. Nonempty operations, full language result admission and CLI/native qualification remain open; C1/G13 are incomplete.

2026-11-18 collection insertion checkpoint: [real-source map/set insertion](reference/generic-collection-insertion-v2.md) now validates exact receiver/key/value types through body/concrete/IR checks, creates ordered flat nonempty snapshots, replaces without moving keys and retains duplicate set positions. Whole-result ownership/copy work is admitted before output copying, including real internal entry nodes. Equality compares associations/membership with charged key searches. Source lookup/removal/iteration and CLI/native qualification remain open; C1/G13 are incomplete.

2026-11-18 membership checkpoint: the explicit collection package profile now
checks and executes `std::map_contains` and `std::set_contains` with exact key
types, primitive Bool results and shared metered key searches. See
[collection operations](reference/generic-collection-operations-v2.md).
Value lookup/Option ownership, removal, iteration, formatting and CLI/native
qualification remain open; C1/G13 are incomplete.

2026-11-18 typed lookup checkpoint: the explicit collection package profile now
checks and executes `std::map_get` and `std::set_get`, deriving sealed standard
Option result types from checked payload ownership. Missing keys produce real
None values; present values copy complete independent snapshots. The whole
wrapper/payload node and text cost is admitted before output copying. See
[typed collection lookup](reference/generic-collection-lookup-v2.md).
Removal, iteration, formatting and CLI/native qualification remain open;
C1/G13 are incomplete.

2026-11-19 typed removal checkpoint: the explicit collection package profile now
checks and executes `std::map_remove` and `std::set_remove`, returning Some(updated
collection) on a hit and None on a miss. Surviving entries keep their order and
full ownership; remove/reinsert appends. Whole Option/container/descendant node
and text costs are admitted before copying. See
[typed collection removal](reference/generic-collection-removal-v2.md).
Source iteration, formatting and CLI/native qualification remain open;
C1/G13 are incomplete.

2026-11-19 iteration snapshot checkpoint: the explicit collection package profile
now checks and executes `std::map_keys`, `std::map_values` and `std::set_values`,
returning exact typed insertion-order arrays with independent complete copies.
Generated array proofs and whole-result node/text/copy work remain bounded; see
[collection iteration snapshots](reference/generic-collection-iteration-v2.md).
Map entry-pair snapshots, formatting, CLI/native admission and consolidated
backend/host/resource qualification remain open; C1/G13 are incomplete.

2026-11-19 public entry snapshot checkpoint: the explicit collection package
profile now checks and executes `std::map_entries` as insertion-order arrays of
sealed `std::MapEntry<K,V>` records with public key/value fields. Actual record
wrappers and all descendant/label copies are admitted together; typed ownership,
Key bounds and source/default-profile/native gates remain explicit. See
[public map entries](reference/generic-collection-entries-v2.md).
Collection formatting, CLI/native admission and consolidated backend/host/resource
qualification remain open; C1/G13 are incomplete.

2026-11-19 collection display checkpoint: explicit collection package execution
now formats maps/sets and nested values in insertion order with checked private
entry ownership and exact two-pass output sizing. Existing temporary-task/node,
text, work and cooperative controls remain active; raw String/scalar and public
record display conventions remain unchanged. See
[collection display](reference/generic-collection-display-v2.md).
CLI/native admission and consolidated backend/host/resource qualification remain
open; C1/G13 are incomplete.

2026-11-19 collection image checkpoint: separate explicit encoding/decoding and
execution APIs now admit exact package feature word 1023, rechecking all embedded
collection source with finite compiler/VM policies and memory-only ownership.
Word 511 and the ordinary compiler/encoder/decoder remain strict; feature flips,
source errors and resource refusal never select a fallback. See
[collection package images](reference/generic-collection-native-images-v2.md).
Installed CLI/native backend/runtime dispatch and generated-executable launch
qualification remain open, along with consolidated backend/host/resource gates;
C1/G13 are incomplete.

2026-11-19 collection command/launch checkpoint: `--experimental-generics
--experimental-collections` now opts package Check/Run/Test/native Build into
collection semantics and exact word-1023 runner dispatch. Collection suites retain
all-root preflight and finite fresh per-test VM policies. Single-entry packages
use the new package profile; default and word-511 paths remain unchanged. See
[collection CLI/native workflow](reference/generic-collection-cli-v2.md).
Linux collection launch, 71 generic integration targets, all 600 library tests
and eight IDE scripts pass. Byte-key/additional-host integration and
E2/D1/H1/H2/Q1 completion remain open. G13 is incomplete.

2026-11-19 D1 byte-runtime checkpoint: explicit Rust checked Byte/immutable Bytes
APIs now provide typed scalar conversion, complete buffer/array snapshots,
index/slice/concat, strict lowercase-hex/standard-base64/UTF-8 codecs and original
input offsets. Packed physical bytes retain one logical Byte node each; whole
outputs and finite helper work/cooperative controls are admitted explicitly. See
[byte runtime foundation](reference/runtime-bytes-v2.md).
Source type/layout/IR/VM and Result-wrapper integration, Byte keys, CLI/native
admission and JSON remain open; this does not complete D1 or G13.

2026-11-19 compiler data-type checkpoint: the explicit data package semantic API
now registers sealed zero-argument std::Byte/std::Bytes headers, checks Byte Key
bounds through aliases/calls/concrete forwarding and preserves exact primitive,
buffer, phantom and imported ownership. Empty parameter scopes are explicitly
validated against registered headers. See
[compiler data annotations](reference/generic-data-types-v2.md).
Default/collection-only APIs still reject data annotations; actual data layouts
explicitly refuse. Source operations, owned Result costs, runtime Byte keys,
CLI/native data admission and JSON remain open; D1/G13 are incomplete.

2026-11-19 data layout checkpoint: the explicit data layout planner now publishes
closed Byte/Bytes metadata with exact declaration identities, a sealed Byte edge
inside Bytes, canonical alias interning and bounded recursive/phantom collection
graphs. Existing construction and IR paths reject data-bearing tables independently;
Byte storage plans refuse instead of selecting Int keys. See
[checked data layout graphs](reference/generic-data-layouts-v2.md).
Checked byte operations, owned Result/error admission, runtime Byte keys, CLI/native
data profiles and JSON remain open; D1/G13 are incomplete.

2026-11-19 scalar Byte execution checkpoint: the explicit data package compiler/VM
now executes checked Int-to-Byte Result conversion and explicit Byte-to-Int,
with complete owned success/error preflight, propagation and Byte collection keys.
Default/collection-only execution and existing native encoders refuse this profile.
Bytes execution, CLI/native data admission, JSON and the remaining G13 phases
stay open. See [checked Byte execution](reference/generic-byte-execution-v2.md).

2026-11-19 Bytes execution checkpoint: the explicit data package compiler/VM
now constructs immutable buffers from original UTF-8 text or checked Byte arrays,
executes byte length and ordered equality/display, and retains nested/generic
copy ownership with complete N+1-node output admission. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source get/slice/concat/to-array/codecs, CLI/native data admission, JSON and
remaining G13 phases stay open; G13 is incomplete.

2026-11-19 Bytes access/concat checkpoint: the explicit data package compiler/VM
now executes `std::bytes_get(Bytes,Int) -> Result<Byte,Error>` with structured bounds
errors carrying the attempted index and complete owned wrapper/payload costs, and `std::bytes_concat -> Result<Bytes,Error>` with ordered independent output,
structured capacity failure and whole-result node admission. Exact checked
operand/result identities are independently rechecked before execution. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source slice/to-array/codecs, CLI/native data admission, JSON and remaining G13
phases stay open. G13 remains incomplete.

D1 follow-up: qualify recoverable Result/error construction for source text/array
buffers; the existing construction helpers still use VM capacity refusal.

2026-11-19 constructor Result checkpoint: `bytes_from_text` and `bytes_from_array`
now return sealed `Result<Bytes,Error>`, including recoverable owned capacity
errors. Explicit VM callers may lower the shared `max_byte_buffer_bytes` ceiling
from 16 MiB to zero. Empty buffers still succeed; all constructor/concat success
and error paths preflight full ownership before output allocation. Source
fixtures now use explicit propagation, and constructor Result/error qualification
listed at the preceding checkpoint is complete. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source slicing/to-array/codecs, CLI/native data admission, JSON and remaining
G13 phases remain open. G13 is incomplete.

2026-11-19 byte slice checkpoint: the explicit data package compiler/VM now
executes `std::bytes_slice(Bytes,Int,Int) -> Result<Bytes,Error>` for half-open
raw byte ranges, including empty slices and slices within UTF-8 sequences.
Negative/reversed/out-of-range bounds return owned errors carrying the original
signed index. Source/concrete/IR stages independently validate exact operands
and sealed results; whole success/error ownership is admitted before allocation.
See [checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source to-array/codecs, CLI/native data admission, JSON and remaining G13 phases
stay open. G13 is incomplete.

2026-11-19 Byte array conversion checkpoint: the explicit data package
compiler/VM now executes `std::bytes_to_array(Bytes) -> Result<[Byte],Error>`
with exact sealed Byte element ownership, independent mutable array snapshots,
raw-byte preservation and complete N+2-node/13-label-byte output admission.
Source/concrete/IR validation and generic forwarding retain distinct buffer,
array and Byte/Int identities. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source codecs, CLI/native data admission, JSON and remaining G13 phases stay
open. G13 is incomplete.

2026-11-19 source hex checkpoint: the explicit data package compiler/VM now
executes `bytes_to_hex(Bytes) -> Result<String,Error>` and
`bytes_from_hex(String) -> Result<Bytes,Error>`, using lowercase encoding, strict
case-insensitive hex digits, original byte offsets and recoverable capacity
errors. It shares the Rust helper digit rules/capacity-validation order while
preflighting complete language Result/error ownership before output allocation.
Sealed String/Bytes Result schemas are independently rechecked. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source base64/UTF-8 codecs, CLI/native data admission, JSON and remaining G13
phases stay open. G13 is incomplete.

2026-11-20 source Base64 checkpoint: the explicit data compiler/VM now executes
`bytes_to_base64(Bytes) -> Result<String,Error>` and
`bytes_from_base64(String) -> Result<Bytes,Error>`. Standard RFC 4648 alphabet,
canonical padding/discarded bits, original byte offsets and complete owned
Result/error preflight are enforced. The packed helper and flat VM share one
strict validator while preserving each caller's control/work errors. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source UTF-8 decoding, CLI/native data admission, JSON and remaining G13 phases
stay open. G13 is incomplete.

2026-11-20 source UTF-8 checkpoint: the explicit data compiler/VM now executes
`bytes_to_utf8(Bytes) -> Result<String,Error>` with RFC 3629 validation, original
invalid-sequence byte offsets and no replacement/normalization. Valid BOM, NUL,
combining sequences and encoded U+FFFD are preserved. Stack-only validation
precedes complete Result ownership admission and exactly sized String copying;
both passes retain cooperative controls. See
[checked immutable Bytes execution](reference/generic-bytes-execution-v2.md).
Source byte-codec execution is covered; CLI/native data admission, JSON and
remaining G13 phases stay open. G13 is incomplete.

Package local-navigation checkpoint: [checked package bindings](reference/generic-package-local-navigation.md) now supply typed hover and definitions for parameters and local/payload bindings, with per-function ownership, source fingerprints and whole-batch quota refusal. Package body type-use/field navigation and consolidated qualification remain open.

Explicit package local-type checkpoint: checked `let` annotation ranges now supply alias-preserving and dependency-aware type hover/definitions, including nested arrays, through the sealed function scope. Constructor/call type arguments, field navigation and consolidated G13 qualification remain open. See [package type navigation](reference/generic-package-type-navigation.md).

Package expression-type checkpoint: explicit call arguments and record/enum constructor heads and arguments now supply checked scoped hover/definitions while preserving alias destinations and source offsets. Field navigation and consolidated G13 qualification remain open. See [package type navigation](reference/generic-package-type-navigation.md).

Package field checkpoint: [checked field navigation](reference/generic-package-field-navigation.md) now supplies field outlines, declaration hover and definitions for user-record reads and constructor labels through exact nominal/field identities. Grouped/chained reads, same-named package fields, reordered labels and whole-batch quotas are covered. Consolidated G13 and external IDE/host qualification remain open.

Consolidated package-navigation qualification (2026-11-23): all 2,273 Rust tests
across 181 integration targets and 16 Node scripts pass, with unchanged source
fingerprint, Clippy, formatting and Rust 1.74.0. Evidence:
`target/checked-evidence/g13-package-navigation-consolidated/verification.json`.
E2 and C1 implementation checklist entries now reflect the implemented explicit
Linux profiles. G13 remains open for actual IDE-host acceptance; additional OS,
browser/mobile and release acceptance are not established by this run.

G13 Linux closure (2026-11-23): all six actual VS Code 1.106.0 host modes pass,
including generic package navigation/unsaved diagnostics/Check/Run and restricted
workspaces. IDE Field outline mapping is corrected; VS Code formatting acceptance
allows host-normalized edit arrays. All 16 Node tooling scripts pass after these
IDE-only changes; Rust compiler/runtime source remains unchanged from the full
2,273-test checkpoint. Separate host fingerprints/logs are retained in
`target/checked-evidence/g13-vscode-host/`. The documented experimental Linux G13
foundation is complete; next is G14 networking/HTTP. Other-host, browser/mobile,
default-profile and release qualification remain separate.

G14 owned socket foundation (2026-11-23): trusted Rust embedders can grant exact
IPv4 loopback ports and own bounded nonblocking listeners/accepted streams with
partial binary I/O, EOF, per-operation deadlines, cancellation and drop cleanup.
See [socket scope](reference/loopback-network-v4.md) and the
[G14 contract](design/G14_COMPLETION_CONTRACT.md). Source/CLI/native network host
integration, Sovra-owned serving, HTTP framing/handlers and independent API
acceptance remain open. This Rust-only slice does not establish HTTP support.

G14 network execution checkpoint (2026-11-24): `NetworkHostV4` validates exact
version-four requests and keeps process-unique tokens tied to private socket
owners. Additive Rust checked-data entry points consume the host and close its
owners on VM success, runtime failure or control refusal. Real `.svr` binary echo
is tested against an independent TCP client. Existing v3/default entry points
remain separate; CLI/native network policy, HTTP and additional-host acceptance
remain open. See [network host requests](reference/network-host-v4.md).

G14 external-policy checkpoint (2026-11-24): the existing data-profile
`--host-policy` option now dispatches exact v3 file/process/time or v4 loopback
policy documents. Regular-file reads are bounded; v4 grants exact ports and only
lowerable limits. CLI/native network echo runs with an independent TCP client;
native startup is tested after project deletion with an empty environment and
no Node/PATH. No grant is embedded or accepted by check/test/build. Existing v3
source/native tests remain regression coverage. HTTP/TLS and wider platform
qualification remain open; see [network policy](reference/network-policy-v4.md).

G14 HTTP request checkpoint (2026-11-24): a pure borrowed fixed-length HTTP/1.1
parser now checks exact CRLF/request lines, Host/field syntax, declared binary
bodies, partial input/EOF and lowerable byte/count/work limits. Ambiguous framing,
unsupported transfer coding/contexts and buffered trailing data refuse. RFC 9110
and 9112 primary sections were reviewed. This is a restricted Rust parser, not
full HTTP conformance or socket/Sovra handler integration. See
[request framing](reference/http-request-framing-v4.md) for the explicit profile
and required aggregate connection controls.

G14 HTTP response checkpoint (2026-11-24): a pure Rust renderer now validates and
sizes final HTTP/1.1 responses before allocation, owns Content-Length/Connection,
handles HEAD and 204/205/304, rejects field injection/framing overrides and
preserves separate Set-Cookie lines. Exact serialized byte/count/work caps and
primary RFC rules are tested. Response/socket/Sovra-handler wiring and full
protocol conformance remain open. See [response framing](reference/http-response-framing-v4.md).

G14 HTTP exchange checkpoint (2026-11-24): version-four HTTP receive/respond
operations now own one-request socket state, aggregate accept-to-send deadlines,
I/O admission and partial-write completion. Framing refusals never publish a
handler request; HEAD context is retained by the host. A real Sovra example
passes independent Python HTTP-client checks for GET/HEAD, JSON echo, 404 and
Result-based handler 500 through CLI/native launch, with no Node/source startup
dependency. Chunked/full protocol, production controls and additional platforms
remain open; see [HTTP exchange scope](reference/http-exchange-v4.md).


2026-11-24 G14 incremental chunked-decoder checkpoint: the experimental Rust
helper now validates binary chunk frames and ignored extensions with aggregate
wire/body/line/chunk/work limits, sticky failures and no partial body exposure.
[Scope and next integration](reference/http-chunked-v4.md): nonempty trailers
refuse; request-head/socket/CLI/native chunked admission remains open. The
existing fixed-length Sovra HTTP serving checkpoint is unchanged.


2026-11-24 G14 chunked serving checkpoint: `http.receive` now admits exactly one
chunked coding through checked head framing and bounded incremental decoding.
Head work, byte/line limits, socket deadline, I/O count and cancellation remain
shared; partial/malformed bodies never publish a handler request. Independent
CLI/native clients exercise chunked Unicode JSON, including splits inside UTF-8.
The public borrowed parser retains its fixed-length contract. Nonempty trailers,
remaining protocol/production and additional-host gates stay open.
See [chunked host scope](reference/http-chunked-v4.md).


2026-11-24 G14 trailer checkpoint: the HTTP host validates and discards bounded
chunked trailers under inherited byte/line/count and shared work/deadline/I/O
limits. No trailer field is merged into the handler header map. Known framing,
routing/authentication/content-processing fields refuse. Independent CLI/native
clients cover declared trailers; integrity verification and application retention
remain outside this discard contract. Full protocol/production gates stay open.
See [trailer scope](reference/http-trailers-v4.md).


2026-11-24 G14 expectation checkpoint: checked HTTP heads now permit one
100-continue expectation; bounded interim output precedes body waits under the
same deadline/I/O accounting. Unsupported expectations receive early 417;
duplicate/invalid framing refuses before interim output. The Sovra example
recovers ordinary receive errors per connection and independent CLI/native
clients verify headers-only waiting and subsequent requests. Full protocol,
Host/origin authorization and production gates remain open.
See [expectation scope](reference/http-expect-v4.md).


2026-11-24 G14 Host/Origin checkpoint: optional trusted HTTP access policies now
validate bounded exact Host/Origin grants and explicit missing-Origin rules
before Continue/body processing. Admission consumes shared request work; denial
returns bounded 403 and drops the owner. The example enables the policy and
independent CLI/native clients test early denial without body transfer followed
by successful requests. Existing policies without the object retain their HTTP
behavior; this is not authentication, CORS or complete CSRF protection.
See [access scope](reference/http-access-v4.md).


2026-11-24 G14 request-permission checkpoint: optional paired method/raw-target
lists now narrow external HTTP access before Continue/body processing. Finite
exact matching preserves method case, encoded targets and queries; comparisons
consume existing HTTP work. Empty lists deny all, incomplete configurations
refuse and omitting both preserves Host/Origin-only behavior. Independent CLI/
native clients exercise unlisted methods and query variants without sending
bodies. This is coarse request admission, not application authentication or
per-route identity permissions. See [access contract](reference/http-access-v4.md).


2026-11-24 G14 qualification consolidation: a [frozen-source runner](G14_QUALIFICATION.md)
now covers library/network/HTTP, actual CLI/native clients and v3 compatibility
with strict lint/MSRV and Node checks. It retains stage failures and rejects
source drift or missing/skipped Rust targets. The checklist now closes the
implemented restricted loopback lifecycle/client foundation while keeping client
authentication, full protocol/production and additional-platform gates explicit.
CI jobs retain x86_64/ARM64 evidence; actual hosted passes require their reports.


2026-11-24 G14 qualifier source-capture hardening: bounded no-follow Linux
directory/file reads now refuse symlinks, missing tracked files and nonregular
substitutions; read/reopen identity checks reject mutation and replacement.
Initial capture errors retain unsuccessful reports. Eighteen Python regressions
cover these cases and existing qualifier failures. Runtime HTTP/authentication
capabilities are unchanged. See [qualification limits](G14_QUALIFICATION.md).


2026-11-24 G14 trusted authentication boundary: a Rust embedding caller can now
register a synchronous Bearer verifier before socket creation. The framework
checks bounded credential syntax, gates body/Continue, maps fixed provider
outcomes, publishes a validated public principal and redacts Authorization from
VM descriptions. Eight tests include real Sovra consumption and post-verifier
controls/cleanup. This is not a CLI/native credential backend or cryptographic
verification implementation; those deployment gates remain open.
See [provider contract](reference/http-authentication-v4.md).


2026-11-24 G14 verifier audience/transport context: trusted providers now receive
exact admitted Host, optional Origin and actual loopback plaintext transport.
Three additional cases verify per-audience refusal, exact/missing context and
refusal despite forged secure forwarding claims. Context scans/validation share
HTTP work and preserve credential/URI debug redaction. This does not implement
TLS, trusted proxies or CLI/native credential backends.
See [audience context](reference/http-authentication-v4.md#audience-and-actual-transport-context).


2026-11-24 G14 external verifier delegation: CLI/native policies can now select
an exact captured native program/cwd and fixed arguments for a bounded JSON
verification protocol. Credentials use stdin; no environment inheritance or
source-visible general process authority is added. Strict replies fail closed,
and child/HTTP timeout or cancellation reaps the direct child. Independent
CLI/native clients prove principal/redaction, wrong-token 401, permission 403 and
continued service without embedded credentials, project files or Node startup.
The test backend is not a shipped production credential system; TLS and reviewed
credential lifecycle/deployment remain open.
See [command contract](reference/http-command-authentication-v4.md).


2026-11-25 G14 aggregate HTTP admission: optional bounded fixed-window budgets
now govern attempts across a host domain before head/parser/authentication work.
Failures consume admitted attempts; listener closure does not replenish credits.
Exhaustion returns best-effort non-cacheable 429 and drops the owner. Three
deterministic clock and four policy/socket cases cover bounds/counters/cleanup;
authenticated CLI/native clients prove denied requests do not spawn verifiers.
This is aggregate attempt limiting, not per-identity/distributed fairness or
production DoS protection. See [admission contract](reference/http-admission-v4.md).
