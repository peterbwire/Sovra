# Completion checklist for the existing 18 tasks

Updated 2026-11-13 at the user's request. This expands the existing G01–G18
acceptance criteria without renumbering tasks or M0–M15 milestones. Current
implementation evidence remains in the [maturity audit](LANGUAGE_MATURITY_AUDIT.md).
A completed bounded foundation stays completed; it does not establish all-platform
support. Unchecked items are remaining work, not approved syntax/ABI/host policies.
Task Manager feature expansion remains paused.

## Six product goals and task ownership

| Goal | Primary tasks | Completion evidence |
| --- | --- | --- |
| Install svr.exe, create/check/run projects | G05, G08–G12, G18 | Installed out-of-checkout workflow on clean supported machines. |
| Applications run without Node.js | G18 and the existing native-execution plan | Exact native artifacts execute without Node, Cargo or svr on the user's machine. |
| Useful general standard library | G13; resource/process/network lifecycle in G14 | Independent packages exercise documented APIs, failures and limits. |
| Sovra-written HTTP servers/APIs | G11, G13, G14 | Sovra owns a real listening/request/response lifecycle; independent client acceptance. |
| Unified desktop/web/mobile UI | G04, G15, G16 | Reusable shared contracts and real target-specific rendering/lifecycle evidence. |
| Reusable distributed package ecosystem | G12, G17, G18 | Resolve, verify, cache, consume and publish independently owned packages. |

## G01 — Accurate documentation (completed foundation)

- [x] Current/proposed classification, example catalog and local-link validation.
- [ ] Maintain accurate feature/support claims as later tasks land.

## G02 — Diagnostic locations (completed foundation)

- [x] Combined Unicode/CRLF CLI, JSON and LSP location regressions.
- [ ] Apply these location guarantees to new syntax/APIs as they arrive.

## G03 — LSP request lifecycle (completed foundation)

- [x] Edit/cancel/close/reopen isolation and exactly-once response coverage.
- [ ] Preserve lifecycle guarantees during package/editor growth.

## G04 — Native desktop UI and qualification (Windows execution deferred)

- [ ] Qualify the current compiled Windows host on real Windows: interaction,
  keyboard/focus, accessibility, DPI, themes, responsiveness and recovery.
- [ ] Define/version public shared UI/event/lifecycle contracts, with an unrelated
  reusable component and independently authored graphical application.
- [ ] Implement and qualify Linux/macOS native desktop adapters where promised;
  shared semantics do not establish native rendering on those hosts.
- [ ] Connect desktop packaging/installed artifacts to G05/G18. Browser UI belongs
  to G15; mobile UI to G16. Windows execution remains deferred, not passed.

## G05 — Independent installed toolchain and lifecycle (partial)

- [x] Installable compiler/templates and new/check/run/test/fmt command foundations.
- [ ] On clean supported Windows, install svr.exe and create/check/execute .svr
  projects outside the checkout using the documented prerequisites.
- [ ] Verify distinct-version upgrade, rollback, uninstall, PATH/profile handling,
  retained user data, damaged-package refusal and installation integrity.
- [ ] Qualify installed editor/IDE integration and publish supported-host requirements.

## G06 — Performance evidence (completed initial corpus)

- [x] Repeatable checked corpus with outputs, raw measurements and machine identities.
- [ ] Extend baselines to new library/server/UI/package workloads and supported hosts.

## G07 — Properties, fuzzing and robustness (completed initial campaigns)

- [x] Frontend/native-image properties and bounded sanitizer campaigns with evidence.
- [ ] Sustain campaigns for new parsers/protocols/data APIs and retain minimized findings.

## G08 — Checked compiler evidence (portable/Linux foundation complete)

- [x] Bounded compiler-owned type/symbol facts, checked lowering and active quotas.
- [ ] Preserve stable stages, diagnostic ownership and limits during later extensions.

## G09 — Package IDE integration (bounded/Linux foundation complete)

- [x] Existing-entry overlays, checked import navigation, workspace diagnostics/tasks.
- [ ] Complete broader registered-module integration with G12, references/rename,
  completion/type-field navigation and debugging through reviewed increments.
- [ ] Qualify supported IDE/OS combinations; browser-editor integration joins G15.

## G10 — Formatter and developer-tool qualification (partial)

- [x] Import-aware/package formatting and guarded publication foundations.
- [ ] Collect exact-revision Windows evidence and qualify macOS writes before enabling.
- [ ] Maintain package lock/source consistency and installed-tool behavior.

## G11 — Complete application checking (partial)

- [x] Bounded structured syntax and selected opt-in local function/binding checks.
- [ ] Complete context/import/type resolution, service/task bodies, model/page bodies,
  policy/render typing and qualified handler/argument bindings.
- [ ] Integrate complete supported application checks into ordinary commands;
  reject unsupported constructs explicitly. Scanner success is insufficient.

## G12 — Modules, reproducibility and distributed packages (partial)

- [x] Local dependency/module compilation, stable nominal exports and source-hashed locks.
- [ ] Finish remaining type contexts and registered-module overlay/navigation/editor integration.
- [ ] Specify versions/compatibility, deterministic dependency selection and conflicts.
- [ ] Implement verified remote acquisition, integrity/authenticity, bounded archives,
  cache/offline operation and reproducible lock updates without execution scripts.
- [ ] Implement registry identities/ownership, publication/download policy and useful
  package CLI/documentation. Reserved install/update commands are not implementation.
- [ ] Prove an independently owned library can be published, fetched, verified and
  consumed from the installed toolchain; test tampering, unavailable/offline and
  conflicting dependencies. Local examples alone do not close ecosystem work.

## G13 — General standard library and data/error foundations (active, partial)

The approved [completion contract](design/G13_COMPLETION_CONTRACT.md) is the
acceptance boundary. This task is not finalized by compiler scaffolding or a
Node-free executable alone. Current supported v2 behavior is opt-in; legacy
commands and representations remain the default.

| Phase | Current state | Required work before closure |
| --- | --- | --- |
| C0 — ordinary array algorithms | Implemented bounded slice/append/concat and ordinary reversal/sorting libraries with independent consumers. | Retain supported-host and snapshot/resource evidence as integration expands. |
| E1 — nominal enums/matching | Root and type-only-module aliases/records/enums, constructors, exhaustive recursive statement matching, linked forward/mutual functions, bounded Rust VM, commands, single-entry locks, syntax formatting and standalone IDE integration are implemented. Source-backed native construction is implemented; exact host evidence is separate. | Module/dependency visibility/linking, executable backend/codec compatibility, v2 package editor contexts and type/field symbol evidence and consolidated parity/host qualification. |
| E2 — generic abstractions and errors | Approved contract; not implemented. | Bounded symbolic checking and concrete specialization for functions/records/enums/aliases, qualified std::Option/std::Result/std::Error, explicit adapters and compatible postfix propagation. |
| C1 — Map/Set | Approved contract; not implemented. | Generic snapshot collections, Key constraints, deterministic operations/equality/iteration and full copy/fuel admission. |
| D1 — bytes/codecs/JSON | Approved contract; not implemented. | Byte/Bytes, strict UTF-8/hex/base64, recursive JSON, exact integers, duplicate/error-offset policy and complete resource/error qualification. |
| H1 — filesystem/time | Existing narrow text services; general capability API not implemented. | Explicit-authority paths/metadata/directories, guarded mutations and portable errors; Gregorian dates, UTC/fixed offsets, wall/monotonic clocks and bounded timers. |
| H2 — processes | Existing program arguments/status; owned child-process API not implemented. | Explicit executable/argument/environment/cwd policy, bounded streams, cancellation/deadlines, kill/reap and failed-cleanup reporting with G14 prerequisites. |
| Q1 — consolidated qualification | Focused tests and host artifacts exist; full completion proof remains open. | Independent locked consumers, backend parity, malformed/extreme/cleanup tests, exact native/platform artifacts, current full regression and policy review. |

Implemented evidence:

- [Root-program checking](reference/program-source-v2.md),
  [linked VM execution](reference/program-execution-v2.md),
  [recursive controls/matches](reference/control-match-v2.md) and
  [flat runtime snapshots](reference/runtime-values-v2.md).
- Opt-in [check](reference/experimental-v2-check.md),
  [run](reference/experimental-v2-run.md), [test](reference/experimental-v2-test.md),
  [bounded IR inspection](reference/experimental-v2-ir.md) and
  [explicit narrow host grants](reference/experimental-v2-hosts.md).
- Guarded [single-entry packages](reference/experimental-v2-packages.md) and
  [captured-byte locks](reference/experimental-v2-locks.md). Dependencies and
  registered modules remain refused by v2 compilation.
- [Source/package formatting](reference/experimental-v2-format.md) preserves
  tokens/comments/AST/fixed points and reuses guarded publication. Formatting a
  selected graph does not compile or link it.
- [Standalone IDE mode](reference/experimental-v2-ide.md) supplies diagnostics,
  formatting, source tasks, checked function and parameter/local/payload navigation;
  unavailable type/field/package facts are not guessed.
- [Source-backed native builds](reference/experimental-v2-native.md) embed bounded
  source and recheck/lower at startup, with finite runtime limits and no stdin/file
  services. This is not an executable IR codec or optimizing backend.

- [x] Typed arrays/array_len, iterative values and reviewed runtime controls.
- [x] Snapshot slice/append/concat with typed results, strict bounds and copy fuel.
- [x] Independent ordinary-library reversal/sorting and snapshot/error cases.
- [x] Source-owned enum syntax and executable root-program enum/match foundations.
- [x] Explicit v2 source-backed native construction and default-denied I/O.
- [ ] Complete E1 package/backend/tooling integration and qualification.
- [ ] Implement E2 generic functions/types and structured error propagation.
- [ ] Implement C1 generic Map/Set collections.
- [ ] Implement D1 binary buffers/codecs and bounded strict JSON.
- [ ] Implement H1 filesystem/path/directory and date/clock/timer capabilities.
- [ ] Implement H2 owned process resources with explicit authority and cleanup.
- [ ] Complete Q1 independent-consumer/backend/host and full-regression evidence.
- [ ] Review helper-work, physical-memory and default policies separately;
  logical fuel/cooperative controls are not process isolation or hard deadlines.

## G14 — Resources, networking, HTTP and server lifecycle (planned/design)

- [ ] Define owned handles, explicit capabilities, partial I/O, lifetime/cleanup,
  cancellation/timeouts, scheduling/concurrency and unsupported-host behavior.
- [ ] Implement sockets and a bounded Sovra-owned loopback server, then HTTP framing,
  requests/responses, routing, JSON APIs and malformed/truncated/oversized refusal.
- [ ] Prove real requests with an independent client; test port conflicts, partial
  reads/writes, disconnects and reliable shutdown/resource cleanup.
- [ ] Add reviewed concurrency/backpressure, streaming, TLS and deployment controls
  before claiming production server support. A private Node adapter is insufficient.

## G15 — Browser execution and web UI (planned/design)

- [ ] Run a real Sovra program in-browser without hidden Node/process/fs dependencies.
- [ ] Define browser host capabilities and shared UI contracts with G04, then qualify
  rendering/events/state, accessibility, failures and package/application consumption.
- [ ] Integrate browser editor/compiler services deliberately; no unsupported host fallback.

## G16 — Mobile runtime and UI (planned/design)

- [ ] Choose explicit Android/iOS runtime/host boundaries without narrowing portable semantics.
- [ ] Execute shared checked/package logic with real native UI, device APIs and
  application lifecycle/resource/permission handling on each claimed target.
- [ ] Qualify actual build/install/run, suspend/resume, failure and packaging workflows.

## G17 — Support, security and ecosystem policy (maintainer)

- [ ] Publish an accurate per-target support matrix, compatibility/versioning policy,
  private security reporting route, review ownership and release authority.
- [ ] Define registry ownership/reporting/revocation and package compatibility rules
  with G12. Avoid unsupported stable/security guarantees.

## G18 — Native distribution and reviewed release qualification (open)

- [x] Public host-native executable build foundation embeds checked IR and a Rust
  runtime; resulting console applications do not require Node, Cargo or svr.
- [ ] Qualify installed native builds outside the checkout. Document build-machine
  Rust/Cargo/linker requirements; review any future bundled builder separately.
- [ ] Test exact checksummed artifacts on clean supported systems without developer
  runtimes, with failure/exit/I/O parity, source-payload integrity and provenance.
- [ ] Qualify desktop/server/package artifacts as those capabilities land; record
  host prerequisites and actual target support, not inferred cross-compilation.
- [ ] Review focused upstream commits, exact-revision CI/MSRV and release evidence;
  make an explicit release decision. Release checks cannot replace missing features.

## Current continuation

Keep G11/G12 incomplete status explicit and continue G13 toward useful libraries.
The reviewed [array_slice](design/G13_ARRAY_SLICE.md) operation is now implemented.
Reviewed [snapshot append/concat](design/G13_ARRAY_GROWTH.md) are now implemented.
The approved [bounded payload-enum syntax slice](design/G13_SUM_TYPE_SYNTAX.md)
is now implemented without enum execution. Next: nominal enum/type checking,
constructors and exhaustive handling under explicit contracts. Continue the
remaining collection/error/data work; checklist expansion does not
approve future type, bounds, error, memory, authority or compatibility semantics.
G14 follows reviewed error/binary/resource foundations. Windows-specific evidence remains deferred until a
Windows host is available; portable work can continue independently.

Opt-in v2 IDE mode now provides standalone buffer diagnostics, snapshot formatting
and source-only check/IR/run tasks. See [v2 editor mode](reference/experimental-v2-ide.md).
V2 package overlays, type/field navigation, semantic completion and other-host
qualification remain open; G13 remains partial. Legacy mode remains the default.

`fmt --experimental-v2 --package --check|--write` now reuses bounded package
selection, all-source preflight and guarded staged publication with the v2 syntax
formatter. See [package formatting](reference/experimental-v2-format.md). It does not link or execute
dependencies/registered modules; v2 compilation and G13 completion gates remain open.

`build --experimental-v2 --emit exe --output PATH` now builds a private
source-backed native runner with a version/model/feature-checked image, finite
shared CLI policy and no stdin/file services. See [v2 native scope](reference/experimental-v2-native.md).
The binary rechecks/lowers embedded source at startup; it is not an executable IR
codec or optimizing backend. Legacy images retain their path. G13 remains partial;
module/dependency integration and the E2/C1/D1/H1/H2 library phases remain open.

Standalone v2 JSON checks can now request `--symbols` for bounded checked root-
function hover/definition/outline. The v2 LSP requests it automatically; failed
checks and optional projection refusals clear facts. See [root-function navigation](reference/program-symbols-v2.md).
No constructor/standard/type/field/package navigation is implied. G13 stays partial.

V2 standalone symbols now also project scope-owned parameters, locals and match
payloads, including shadowing, indexed writes and match-scrutinee identities.
Checked type labels are bounded; unresolved array holes remain explicitly unavailable.
See [binding navigation](reference/program-symbols-v2.md). Type/field/package navigation remains open.

V2 standalone symbols now also expose checked enum/variant outline entries and
constructor/match-arm variant definitions, selected by nominal IDs and coverage
plans. See [nominal navigation](reference/program-symbols-v2.md). Alias/record/annotation/field and
package navigation still require their own evidence; G13 remains partial.

V2 enum aliases now work as constructor and match-arm qualifiers under the same
canonical nominal IDs, payload rules and standard-call collision refusals. Alias
signatures consume existing count/text quotas; the canonical lookup index is
charged to retained type metadata. See [enum aliases](reference/enum-aliases-v2.md).
Default legacy behavior is unchanged; G13 remains partial.

V2 now admits type-only inline modules containing enums/aliases/records, with
qualified use by root functions, original spans and qualifier-work admission.
See [type modules](reference/type-modules-v2.md). Module functions, nested modules, imports and
dependency linking remain refused; same-file lookup is not package visibility.
G13 remains partial.

V2 raw syntax/formatting now retains module function bodies with original names,
spans and parallel module paths under shared admission. Checked source, execution
and native decoding still explicitly refuse those bodies. See [syntax-only module
bodies](reference/type-modules-v2.md). Scope/visibility/call linking remain open; G13 is partial.

V2 program checking/execution now supports qualified same-file module functions,
private/exported call filtering and module-owned type views with charged aggregate
metadata. See [checked module functions](reference/module-functions-v2.md). Root entry/test selection
remains root-only. Conflicting root-type shadows, nested modules/imports/dependency
linking remain refused; the single-entry API stays strict. G13 is partial.


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
