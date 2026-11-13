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

- [x] Typed arrays/array_len, iterative value handling, optional fuel and approved
  Rust/Node Worker cooperative controls, with independent package evidence.
- [x] Add snapshot array slicing with typed results, strict bounds, explicit fuel
  and an independent registered package consumer.
- [x] Add snapshot append/concat with first-directed typing, checked length,
  complete fuel and independent filtering/record-package consumption.
- [ ] Add further array operations, Map/Set and generic abstractions under explicit
  element/equality/iteration/snapshot contracts.
- [ ] Define and implement structured errors, payload sum types, Option/Result,
  exhaustive matching and propagation before depending on them in public APIs.
- [ ] Add binary buffers/codecs and bounded JSON with explicit Unicode, precision,
  duplicate-key, error-location and depth/size policies.
- [ ] Expand filesystem/path/directory operations and dates/clocks/timezones/timers.
- [ ] Add process argument/environment/launch/exit/stream APIs with G14's resource,
  authority, cancellation and cleanup contracts; do not expose ambient authority silently.
- [ ] For each capability: independent package consumer, documented valid/error cases,
  exact/exceeded limits and actual supported-backend evidence.
- [ ] Review helper-work/physical-memory/default policies separately. Current fuel
  and cooperative checks do not establish process isolation or hard deadlines.

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
Next review: the [bounded payload-enum syntax slice](design/G13_SUM_TYPE_SYNTAX.md)
starts error-handling foundations without claiming enum execution. Continue the
remaining collection/error/data work; checklist expansion does not
approve future type, bounds, error, memory, authority or compatibility semantics.
G14 follows reviewed error/binary/resource foundations. Windows-specific evidence remains deferred until a
Windows host is available; portable work can continue independently.
