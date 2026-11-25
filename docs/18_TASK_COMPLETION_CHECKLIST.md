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

## G13 — General standard library and data/error foundations (experimental Linux foundation complete)

The approved [completion contract](design/G13_COMPLETION_CONTRACT.md) is the
acceptance boundary. This task is not finalized by compiler scaffolding or a
Node-free executable alone. Current supported v2 behavior is opt-in; legacy
commands and representations remain the default.

Current closure work and resource review are recorded in
[G13 qualification](G13_QUALIFICATION.md). Historical checkpoints below are
superseded by the current phase table where they describe earlier implementation.

| Phase | Current state | Required work before closure |
| --- | --- | --- |
| C0 — ordinary array algorithms | Implemented bounded slice/append/concat and ordinary reversal/sorting libraries with independent consumers. | Retain supported-host and snapshot/resource evidence as integration expands. |
| E1 — nominal enums/matching | Package visibility/linking, checked bodies, native source-backed executables and compiler/LSP function/type/local/field integration execute in the documented generic profile; actual standalone/package VS Code acceptance passes on Linux. | Linux scope complete. Separate v2-only limitations and additional OS/IDE/runtime claims remain explicit. |
| E2 — generic abstractions and errors | Generic functions/types, Option/Result/Error, propagation, dependency/module graphs and checked IDE navigation execute through the documented experimental CLI/native profile. | Linux scope complete; unsupported JS/default-profile behavior remains refused. |
| C1 — Map/Set | Deterministic typed snapshots, exact Key ownership, source operations/iteration/equality/display, Byte keys and word-1023/2047 native consumers pass Linux resource and independence regressions. | Linux scope complete; additional backend/OS support requires separate evidence. |
| D1 — bytes/codecs/JSON | Byte/Bytes, strict codecs, recursive ordered JSON, exact integers, duplicate/error offsets, formatting and independent data/native consumers pass full Linux qualification. | Linux scope complete; additional host runtime evidence remains separate. |
| H1 — filesystem/time | Version-3 rooted read/write authority, guarded mutations, paths, Gregorian dates, explicit clocks and bounded timers pass supported Linux host/resource regressions. | Linux scope complete. Unsupported adapters refuse; additional OS evidence remains separate. |
| H2 — processes | Explicit executable/cwd/environment grants, owned child/streams, bounded output, cancellation, kill/reap and cleanup reporting pass supported Linux regressions. | Linux scope complete; documented trust/resource limits and additional-host gates remain. |
| Q1 — consolidated qualification | Full frozen-source run: 2,273 Rust tests, 181 integration targets, 16 Node scripts, Clippy and exact Rust 1.74.0. Subsequent IDE-only changes pass all 16 Node scripts and six actual VS Code host modes, with separate source-fingerprinted reports. | Documented experimental Linux scope complete; exact release/platform acceptance remains G17/G18. |

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
- [x] Implement and qualify E1 package checking/execution/native and checked LSP integration in the explicit generic Linux profile.
- [x] Qualify these package IDE workflows in actual VS Code 1.106.0 on Linux; see [host acceptance](reference/generic-package-ide-host.md). Additional OS/IDE acceptance remains separate.
- [x] Add explicit generic package Check/Run task selection from compiler-owned registered-source metadata, related-only source saves and dirty-manifest refusal.
- [x] Check unsaved generic entry/module/dependency buffers through a guarded whole-package overlay path with generation-safe diagnostics.
- [x] Project checked generic package function definitions/hover/outlines through canonical call identities and source fingerprints.
- [x] Project package type annotations in function signatures, record fields, enum payloads and alias targets using the sealed scoped registry; explicit local annotations and constructor/call type arguments are also covered.
- [x] Project user-record field declarations, reads and constructor labels through checked nominal field identities; see [field navigation](reference/generic-package-field-navigation.md). Consolidated Linux compiler/LSP qualification and actual Linux VS Code host acceptance pass.
- [x] Project package parameters, locals and payload bindings from checked owners, with shadowing-safe definitions and typed hover; see [local navigation](reference/generic-package-local-navigation.md).
- [x] E2 bounded source-owned generic annotation grammar (syntax-only Rust API).
- [x] E2 bounded generic parameter-list ownership and annotation parameter-use binding.
- [x] E2 root generic alias syntax with attached parameter ownership and aggregate admission.
- [x] E2 generic record syntax with shared parameter ownership and cumulative field quotas.
- [x] E2 generic enum syntax with shared payload parameter ownership and aggregate quotas.
- [x] E2 generic function header ownership and bounded opaque body retention.
- [x] E2 bounded annotation head/arity resolution and initial concrete/symbolic Key proofs.
- [x] E2 bounded transparent alias substitution, cycle rejection and symbolic target bounds.
- [x] E2 complete root definition coverage and symbolic record-field/enum-payload checks.
- [x] E2 sealed generic function signature checking against complete type definitions.
- [x] E2 bounded expanded-type equivalence and exact normalized in-process type keys.
- [x] E2 initial symbolic immutable copy/return/equality bodies with source-owned facts and cumulative admission.
- [x] E2 explicit immutable generic local annotations with cumulative fragment/expansion budgets.
- [x] E2 concrete numeric symbolic arithmetic/ordering with explicit operand conversions; generic execution remains open.
- [x] E2 concrete Boolean symbolic `&&`/`||` facts; both operands checked, executable short-circuit lowering remains open.
- [x] E2 concrete String concatenation/ordering symbolic facts; runtime allocation admission and generic execution remain open.
- [x] E2 fixed-type mutable symbolic locals/direct assignment; field/index mutation and generic execution remain open.
- [x] E2 checked symbolic expression statements with discarded results; calls and executable integration remain open.
- [x] E2 symbolic `if`/`else`, branch scopes and all-path return checking; calls and executable integration remain open.
- [x] E2 symbolic `while`, loop scopes and conservative return coverage; calls and executable integration remain open.
- [x] E2 read-only symbolic array indexing with concrete Int indexes and exact element types; runtime bounds checking/execution remain open.
- [x] E2 direct mutable symbolic array indexed assignment with fixed element types; executable snapshot/bounds integration remains open.
- [x] E2 context-typed symbolic array literals, nested/empty arrays and explicit item widening; inferred arrays/execution remain open.
- [x] E2 parameter-independent symbolic record field reads; composite substitution and execution remain open.
- [x] E2 direct record-parameter field argument projection with preserved caller owners; execution remains open.
- [x] E2 bounded composite field substitution with caller identities and per-node source provenance; calls/execution remain open.
- [x] E2 bounded explicit `::<T,E>` call/constructor argument-list syntax; callee binding and execution remain open.
- [x] E2 caller-scope explicit type-argument checking with aliases/type-head constraints and cumulative budgets; execution remains open.
- [x] E2 sealed root-callee type matching with exact owners, arity and Key proofs; executable calls remain open.
- [x] E2 bounded caller-scoped callee signature substitution with graph/provenance retention; body calls and execution remain open.
- [x] E2 supplied caller-expression value matching with exact owners/types, explicit widening and comparison budgets; source-call facts/execution remain open.
- [x] E2 source-backed generic call envelopes with explicit type grammar and opaque balanced value ranges; body call integration remains open.
- [x] E2 exact original-token call value expression parsing with AST/work limits and iterative cleanup; caller-scope typing/source-call facts remain open.
- [x] E2 separate body-arena binding/expression identities and fail-closed exhaustion; same-signature cross-arena IDs cannot enter value matching.
- [x] E2 original-source explicit root body calls with active lexical scope, contextual values, sealed type/signature facts and cumulative budgets; specialization/runtime integration remain open.
- [x] E2 explicit symbolic root record construction with complete field/type/bound checks, source-order initialization and cumulative budgets; constructor IR/runtime integration remains open.
- [x] E2 explicit symbolic enum constructors and standalone exhaustive pattern/type coverage with bounded payload substitution; execution remains open.
- [x] E2 symbolic enum match bodies with active scrutinee scope, immutable arm-local payload IDs, exhaustive coverage, nested arm typing and conservative all-arm returns; specialization/execution remain open.
- [x] E2 bounded concrete call-graph discovery from source-owned nongeneric roots with exact normalized argument reuse, recursive closure and cumulative limits; concrete operation rechecking/lowering/execution remain open.
- [x] E2 concrete signature/binding/expression substitution and operation/context rechecking against finite call targets, including nominal fields/payloads and explicit conversions under cumulative budgets; runtime layouts/lowering/execution remain open.
- [x] E2 exact original-source primitive literal payload decoding for concrete instances, shared immutable values, preserved conversions and cumulative copy/token/reference/work limits; runtime layouts/lowering/execution remain open.
- [x] E2 finite logical runtime layouts for primitives, arrays and nominal record/enum members with source fact mappings, recursive edges and a combined function/type instantiation budget; constructor/match plans and executable lowering remain open.
- [x] E2 bounded nominal constructor/field/match plans with source-order initialization, exact slots/tags/conversions, immutable payload destinations and source-contained exhaustive dispatch ranges; executable IR/control-flow lowering remains open.
- [x] E2 owned generic instruction/control-flow lowering with explicit conversions, source-order nominal operations, short-circuit branches, loop backedges, concrete calls, matched tag targets and bounded typed-stack CFG validation; production runtime/source/CLI/backend integration remains open.
- [x] E2 host-free Rust execution of checked generic IR with iterative frames, flat owned snapshots/cleanup, arithmetic/bounds checks and shared instruction/materialization/work/control/active-storage limits; source/CLI/package/native/JS integration remains open.
- [x] E2 complete admitted standalone source collection/checking/execution through the Rust API, ordinary nongeneric calls, original-file primary error mapping and cumulative syntax admission; CLI/package/backend integration remains open.
- [x] E2 explicit standalone CLI generic check/run with original-file diagnostics, JSON checks and finite host-free execution; package/backend/tooling integration remains open.
- [x] E2 standalone host-native generic source images, bounded startup rechecking, fixed host-free VM policy and no-overwrite linking; package/JavaScript/tooling and other-host qualification remain open.
- [x] E2 guarded single-entry generic project check/run/native build with captured-byte lock verification and source-owned diagnostics; dependency/module linking and other-host package capture remain open.
- [x] E2 qualified compiler-owned std::Option/std::Result through checked source, exhaustive matching and host-free source/native execution, with separate user identities and native feature gating; std::Error/adapters/propagation remain open.
- [x] E2 compiler-owned nongeneric std::Error with checked code/message/optional-offset fields, owned snapshots, exact constructor admission and backward-compatible native feature gating; adapters/propagation remain open.
- [x] E2 bounded symbolic propagation compatibility plans over sealed body operands, exact Result error types and retained source/variant/type evidence; postfix syntax/concrete/lowering/runtime integration remains open.
- [x] E2 bounded original-source postfix syntax capture for complete blocks, precise nested/chained marker ranges and speculative-parser rollback, without exposing unwrapped legacy ASTs; checked-body/concrete/lowering/runtime integration remains open.
- [x] E2 opt-in complete symbolic postfix body checking with active operand ownership, exact failure compatibility, contextual success types, original provenance and shared resource/depth admission; specialization/concrete/lowering/runtime integration remains open.
- [x] E2 explicit bounded propagation specialization/concrete rechecking with closed per-instance contracts, descriptor consistency, shared admission and an unconditional IR gate; early-return/runtime/source-native integration remains open.
- [x] Add bounded opt-in propagation operation layouts with per-instance payload/return mappings; execution remains gated.
- [x] Add bounded inspection-only early-return IR for checked propagation sites; runtime integration remains gated.
- [x] Execute checked propagation IR through an explicit VM API with shared fuel and current-frame cleanup; source/CLI/native integration remains open.
- [x] Compile and execute complete original standalone files through an explicit propagation source profile, retaining full-file diagnostics; CLI/native integration remains open.
- [x] Integrate propagation with experimental source/root-package commands and versioned native startup rechecking; qualify a real independent Linux native consumer.
- [x] Provide explicit source adapters for all five legacy standard result records with schema parity, checked snapshots and a qualified native consumer; builtin-call bridging remains open.
- [x] Bridge host-free parse_int/split_once into the checked generic pipeline with existing result records, explicit adapters and a native consumer; other builtin bridging remains open.
- [x] Bridge String len/lines_unique/escape_terminal with core parity, temporary-storage/output admission and a native consumer; polymorphic/array/host builtins remain open.
- [x] Bridge polymorphic to_string and generic array_len with iterative formatting, output/task admission and native qualification; append/concat/slice and host bridging remain open.
- [x] Bridge array append/concat/slice with generic snapshots, explicit recursive widening, half-open bounds and native qualification; host bridging and remaining E2 criteria remain open.
- [x] Add explicit generic LSP/VS Code standalone buffer diagnostics and Check/Run tasks with real-host verification; navigation/formatting and package/JS integration remain open.
- [x] Project compiler-owned generic root declarations into fail-closed JSON symbols and editor outlines/declaration hover; broader semantic references remain open.
- [x] Project checked generic/plain root call bindings into bounded source references and editor definition/hover; type-use navigation remains open.
- [x] Project checked parameter/local/match-payload identities and original read/write tokens into bounded definition navigation and role/name hover.
- [x] Render bounded checked symbolic/inferred binding types in generic local hover, expanding aliases through retained owners; type-use navigation remains open.
- [x] Link user-defined record/enum/alias names in checked function headers to source definitions/hover; other type-use contexts remain open.
- [x] Execute bounded generic source/locked-root-package test suites with all-root compilation before execution and schema-one reports.
- [x] Capture immutable graph-wide dependency/module source bytes and verify schema-compatible locks independently of the legacy parser; generic linking remains open.
- [x] Retain source-owned generic package declarations and explicit file-local import visibility with separate function/type identities; semantic use binding and executable linking remain open.
- [x] Check all package aliases, record fields, enum payloads and function signatures through source-scoped visibility and canonical type identities; executable body linking remains open.
- [x] Check source-aware package calls, constructors, local types and enum patterns, verifying cross-package generic execution through explicit Rust stages; selected-root CLI/native package integration remains open.
- [x] Compile owned selected-root generic graphs and enable guarded package Check/Run/Lock with cross-file diagnostics; multi-file Test/native Build remain open.
- [x] Execute locked multi-file generic package suites with root-only discovery, shared frontend proofs, all-root preflight and aggregate instance admission; native package Build remains open.
- [x] Build locked registered generic graphs into source-backed native executables, with bounded source/topology codec, in-memory rechecking and no project/Node startup dependency; broader qualification remains open.
- [x] Implement E2 generic functions/types and structured error propagation in the documented experimental generic Linux profile.
- [x] Implement C1 generic Map/Set collections in the documented experimental collection/data Linux profiles.
- [x] Execute intrinsic create/len/insert/contains/get/remove through the explicit collection package profile with exact checked types and bounded complete snapshots.
- [x] Add intrinsic source map keys/values and set values as typed insertion-order arrays with complete descendant admission.
- [x] Add intrinsic source map entry-pair arrays with sealed public std::MapEntry<K,V> records and complete wrapper/descendant admission.
- [x] Add bounded insertion-order intrinsic collection display, including nested values and temporary-task/output admission.
- [x] Qualify explicit collection package image encoding/rechecking/execution with exact word 1023, finite budgets and memory-only source custody.
- [x] Wire explicit collection package Check/Run/Test and native Build/runner dispatch, with Linux generated-executable launch after project deletion and no Node/PATH.
- [x] Consolidate the intrinsic collection package/Linux profile with broad regressions, independent native consumers and exact JS/IR/symbol refusal checks.
- [x] Integrate Byte keys with D1 and qualify the Linux collection/data profiles.
- [ ] Qualify additional collection backend/host runtime support before claiming it.
- [x] Implement D1 binary buffers/codecs and bounded strict JSON in the explicit data profile.
- [x] Add checked distinct Rust Byte/immutable Bytes values, full binary-node copies, strict codecs, original offsets and finite shared work/control.
- [x] Register explicit sealed Byte/Bytes annotations and Byte Key proofs with package visibility, bounded type checking and concrete forwarding.
- [x] Publish bounded closed Byte/Bytes layout metadata with sealed buffer-element edges, recursive collection graphs and explicit construction/IR refusal.
- [x] Enable Byte/Bytes construction/IR/VM operations, owned Result/error costs, runtime Byte keys and CLI/native data profiles.
- [x] Implement bounded strict JSON with exact numeric kinds, duplicate-key/error-offset refusal and independent consumers.
- [x] Format generic/data/propagation syntax and package graphs with token/comment/structure/fixed-point checks; return guarded LSP edits and qualify formatted locked/native consumers.
- [x] Implement H1 filesystem/path/directory and date/clock/timer capabilities in the explicit supported host profile.
- [x] Implement H2 owned process resources with explicit authority and cleanup in the explicit supported host profile.
- [x] Complete Q1 independent-consumer/backend/host and full-regression evidence for the documented experimental Linux profile; the current frozen-source run passes 2,273 Rust tests and 16 Node scripts. Actual Linux VS Code acceptance also passes; additional-host runtime claims remain open.
- [x] Review helper-work, physical-memory and default policies separately;
  see [G13 resource review](G13_QUALIFICATION.md). Logical fuel/cooperative controls
  are not process isolation or hard deadlines.

## G14 — Resources, networking, HTTP and server lifecycle (active, partial)

- [x] Add a trusted Rust loopback socket foundation with exact port grants, unique resource owners, nonblocking bounded I/O, deadlines/cancellation and independent TCP-client tests; see [socket scope](reference/loopback-network-v4.md).
- [x] Add strict version-four network requests with execution-scoped owned handle custody and explicit Rust checked-data execution; actual `.svr` binary echo and failure cleanup are tested. See [network host](reference/network-host-v4.md). CLI/native network policy is now supported through an external versioned policy; HTTP remains open.
- [x] Load bounded external v3/v4 policies and dispatch explicit CLI/native data execution; prove independent native binary echo after project deletion with no Node/PATH, and preserve v3 authority behavior. See [network policy](reference/network-policy-v4.md).
- [x] Add a bounded borrowed fixed-length HTTP/1.1 request parser reviewed against RFC 9110/9112, including ambiguous framing/refusal, partial input and raw byte preservation; see [parser scope](reference/http-request-framing-v4.md). Socket integration, response construction and handlers remain open.
- [x] Add pure bounded final HTTP response construction with renderer-owned framing, HEAD/no-content handling, field injection/override refusal and exact byte/work limits; see [response scope](reference/http-response-framing-v4.md). Socket and handler integration remain open.
- [x] Integrate one-request HTTP socket ownership and aggregate deadline/I/O policy with checked Sovra handlers; qualify CLI/native GET/HEAD, JSON echo, 404 and recovered handler 500 through an independent HTTP client. See [exchange scope](reference/http-exchange-v4.md). Full protocol/production and additional-platform gates remain open.
- [x] Add a bounded incremental Rust chunked-body decoder with sticky errors, extension validation and fragmentation-independent work; see [decoder scope](reference/http-chunked-v4.md). Checked head/socket integration and CLI/native chunked Unicode JSON clients are now covered; trailer retention/integrity and full protocol remain open.
- [x] Validate and discard bounded chunked trailers without merging handler headers, refuse known security fields and malformed declarations, and test CLI/native clients, shared limits, cancellation and cleanup. See [trailer scope](reference/http-trailers-v4.md); retention/integrity and full HTTP/production gates remain open.
- [x] Support bounded 100-continue after checked heads and before body waits, early 417 for unsupported expectations, shared interim output/deadline/I/O limits, and independent CLI/native handshakes plus per-connection receive recovery. See [expectation scope](reference/http-expect-v4.md); full HTTP/production gates remain open.
- [x] Add explicit external Host/Origin allowlists with bounded exact matching, missing-Origin rules and shared work; refuse before Continue/body/handler publication and qualify CLI/native 403 clients. See [access scope](reference/http-access-v4.md). Application authentication, absent-policy defaults and production qualification remain separate.
- [x] Narrow configured HTTP access with paired exact method/raw-target lists, bounded grants/shared work, case/query/percent-escape refusal and CLI/native pre-body 403 tests. See [request permissions](reference/http-access-v4.md#optional-exact-method-and-target-grants); application authentication/per-route identity permissions remain open.
- [x] Define and exercise unique owned handles, explicit capabilities, partial I/O, scoped cleanup, cancellation/deadlines and unsupported-host refusal for the documented Linux profile. Scheduling/concurrency remain separate.
- [x] Implement the documented restricted Sovra-owned loopback HTTP lifecycle, request/response framing, application routing/JSON and malformed/truncated/oversized refusal; full HTTP conformance remains open.
- [x] Retain repeatable independent client, port-conflict, partial I/O, disconnect, deadline and cleanup evidence through the [frozen-source G14 qualifier](G14_QUALIFICATION.md), including CLI/native and v3 compatibility. Hosted/additional-platform results must be collected separately.
- [x] Add a trusted Rust Bearer-verifier embedding boundary with bounded checked credentials, provider outcomes, validated principal publication, Authorization redaction, shared controls and an actual Sovra consumer. See [provider scope](reference/http-authentication-v4.md). No CLI/native credential backend or cryptographic verification is bundled.
- [x] Supply admitted Host/Origin and actual transport to trusted verifiers; test audience-bound refusal, exact context and denial of forged secure forwarding claims. See [audience context](reference/http-authentication-v4.md#audience-and-actual-transport-context); TLS/proxy trust and deployable credential backends remain open.
- [x] Delegate CLI/native Bearer verification to an explicit captured external command with strict bounded protocol, private process authority, cooperative cancellation/deadline cleanup and independent valid/401/403 native clients. See [command contract](reference/http-command-authentication-v4.md). No production backend/store/TLS is bundled.
- [x] Add optional per-domain fixed-window HTTP attempt admission before parsing/verifier work, counting failures across listeners and returning bounded non-cacheable 429; qualify CLI/native verifier-spawn avoidance. See [admission scope](reference/http-admission-v4.md). Per-identity/distributed quotas and production scheduling remain open.
- [ ] Select and qualify reviewed production credential backends, issuance/storage/rotation/revocation and per-identity/per-route permissions; delegated verification does not itself provide those systems.
- [x] Add cooperative `net.try_accept` with single-syscall idle return, capacity-before-dequeue admission and cancellation-safe ownership; checked Sovra source and socket tests cover the foundation. HTTP scheduling remains open.
- [x] Add cooperative `net.try_read` with bounded single-syscall reads, distinct idle/data/EOF results and pre-read cancellation; prove idle-peer yielding and checked Sovra consumption. Resumable HTTP and fair scheduling remain open.
- [x] Add cooperative `net.try_write` with exact partial progress, explicit not-ready output and pre-write cancellation; prove actual send-buffer backpressure and progress on another connection. HTTP scheduling remains open.
- [x] Extract bounded resumable HTTP head accumulation with retained scan state, sticky failures and body-prefix preservation; existing receive uses it before strict one-time parsing. Yielding HTTP and scheduling remain open.
- [x] Add `http.poll_head` pending-owner custody with one nonblocking read, retained head/deadline/I/O state and admission-once transition into checked receive. Body/verifier/output phases and fair scheduling remain open.
- [x] Extract bounded resumable fixed-length HTTP body state with exact/zero lengths, sticky excess refusal, opaque debug and complete-only publication; integrate after authentication. Yielding body phases remain open.
- [x] Retain owned checked HTTP metadata and verifier outcome with fixed/chunked pending-body custody; add `http.poll_body` with readiness-only publication and one nonblocking read after preparation. Verifier/interim/final-output scheduling remains open.
- [x] Add cooperative final-response preparation/polling with immutable framed output, exact partial progress, shared deadlines/I/O and close-on-completion; the Sovra example uses this path. Fair scheduling and interim/verifier phases remain open.
- [x] Schedule two HTTP connections in cyclic bounded phase turns with full-slot accept backpressure, fixed pacing and finite idle rounds; independent CLI/native clients verify delayed head/body interleaving. Production/preemptive fairness remains open.
- [x] Queue bounded Continue output in pending-body state and advance one exact partial write per body-poll turn; blocking receive resumes the suffix without duplication. Verifier and denial/error output remain open.
- [x] Retain cooperative admission/head-preparation refusals under owned bounded output custody, yielding exact partial writes then returning the original error; forbid handler/raw/application-response transitions. Initial verifier execution and production fairness remain open.
- [x] Extract private bounded owned-process polling state with retained pipe/deadline/exit progress, opaque debug and kill/reap cleanup; existing synchronous runner uses it. Pending HTTP verification remains open.
- [x] Bind owned checked metadata to pending command-verifier jobs, poll bounded child I/O without duplicate spawn, cap live jobs at two and kill/reap on close/cancellation/timeout; qualify independently gated CLI/native clients. Legacy trusted-provider fallback and production controls remain separate.
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

Keep G11/G12 incomplete status explicit. G13's documented experimental Linux
foundation is complete: data/errors/collections, explicit filesystem/time/process
hosts, source-backed native consumers, formatting and checked package IDE
integration are implemented and qualified. The full compiler/runtime run passes
2,273 Rust tests and 16 Node scripts; the subsequent IDE-only checkpoint also
passes all 16 Node scripts and all six actual Linux VS Code host modes.
[Host acceptance](reference/generic-package-ide-host.md) and
[G13 qualification](G13_QUALIFICATION.md) retain the scope and evidence.
Next is G14 resource/network/HTTP implementation under its own contract.
Windows/macOS, browser/mobile, default-profile expansion and release qualification
remain tracked separately; this closure does not complete the language ecosystem.

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
