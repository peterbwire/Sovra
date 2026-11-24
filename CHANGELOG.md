# Changelog

- Add an experimental ordered snapshot map/set reference package and runnable consumer, with bounded package type-argument admission. Intrinsic collection types remain open.

## Unreleased

- Execute typed map insertion/replacement and set insertion in the explicit collection
  profile, with flat entry snapshots, atomic result costs and bounded unordered
  equality. Lookup/removal/iteration and CLI/native qualification remain open.

- Execute typed empty Map/Set creation and length from Sovra source through explicit
  collection package/IR/VM APIs with shared controls. Ordinary execution and native
  encoding retain feature gates; nonempty operations remain open.

- Discover opaque Map/Set layout graphs through an opt-in planner, with recursive
  value edges, shared resource caps and checked storage binding. Collection-bearing
  graphs remain refused at construction/IR until runtime operations are qualified.

- Add opt-in package semantics for opaque std::Map/std::Set annotations, with
  cumulative header admission, exact arity, Key bounds and source visibility.
  Executable layouts explicitly refuse; default CLI/native profiles stay unchanged.

- Add sealed Map/Set layout plans with checked key/value arguments and typed storage
  factories. Source annotations and executable collection layouts remain gated.

- Add ordered map storage for proof-carrying generic package values, enforcing exact
  owners/type arguments and atomic full-result copy admission. Source/IR/VM
  collection instructions remain open.

- Retain checked generic package-owner/type-argument proofs in bounded collection
  value snapshots. Source collection and VM instruction integration remain open.

- Add sealed checked-record runtime construction and collection value ownership,
  with exact field types, full snapshot costs and bounded field-search equality.
  Name-only records retain legacy behavior and cannot forge collection owners.

- Admit sealed checked-enum map value leaves with environment-specific identities,
  complete label/payload copy costs and bounded transitive ingress. Record values
  and source/VM collection integration remain gated.

- Add an explicit opaque map runtime API with exact primitive/array value typing,
  ordered updates and atomic full-descendant copy admission. Nominal values and
  compiler/VM collection integration remain open.

- Add an explicit opaque primitive-key set runtime API with atomic snapshot fuel
  admission and bounded helper work; source and VM integration remain open.

- Execute sealed generic IR in a host-free Rust VM with iterative frames, flat
  owned snapshots, compound operations and shared resource/control limits.
  Source/CLI/package and native/JavaScript integration remain open.

- Lower concrete generic bodies into owned typed stack instructions with resolved
  control flow, short-circuiting, calls, arrays and nominal operations; validate
  slot/target structures and reachable typed-stack joins under finite limits.
  Production generic runtime/backend and Program/CLI integration remain open.

- Link generic nominal constructors, field reads and exhaustive match dispatch to
  concrete layout slots/tags while preserving operand order and explicit conversions.
  Executable instruction and control-flow lowering remain open.

- Plan finite backend-neutral generic layouts with declaration-order fields/tags,
  recursive nominal edges and a shared function/type instantiation budget.
  Constructor/match execution plans and executable lowering remain open.

- Decode exact source-backed generic literal payloads with finite source/token/
  string-copy/reference limits and immutable sharing across concrete instances.
  Conversion edges and executable behavior remain unchanged.

- Recheck concrete generic signatures and body operation/context edges against
  finite specialization targets with bounded type substitution and comparisons.
  Runtime layouts and executable lowering remain open.

- Discover bounded concrete generic call graphs from nongeneric source roots with
  exact argument reuse and finite recursion admission. Concrete body rechecking
  and executable lowering remain open.

- Check exhaustive matches inside symbolic generic bodies with active scopes,
  immutable payload identities, nested arm typing and all-arm return coverage.
  Generic specialization and execution remain open.

- Check and execute qualified v2 same-file module functions with private/exported
  call filtering, canonical identities, scoped types and charged shared metadata.

- Retain source-owned v2 module function syntax and namespace paths for formatting
  drafts; checked compilation and execution still refuse these bodies.

- Admit bounded type-only inline modules in v2, preserving qualified nominal
  ownership, original spans, formatting and checked enum/variant navigation.

- Resolve v2 alias-qualified enum constructors and match arms to canonical IDs,
  with charged signature/index metadata and preserved nominal/payload checks.

- Add checked v2 enum/variant outline kinds and canonical constructor/match-arm
  variant hover/definitions, sharing standalone symbol quotas and refusal policy.

- Extend v2 standalone symbols with checked parameter/local/payload hover and
  definitions, scope-owned read/write targets and shared optional quotas.

- Add explicit v2 standalone checked function symbols and LSP root-function
  hover/definition/outline with all-or-unavailable projection limits.

- Add opt-in v2 source-backed native executable builds, strict private image
  admission, shared finite VM policy, bounded UTF-8 arguments and denied I/O.
  Legacy native images keep their existing runtime path.

- Add opt-in v2 package syntax formatting through existing canonical graph/source
  selection, all-input preflight, byte quotas and guarded staged writes.

- Add explicit v2 LSP/VS Code standalone diagnostics, snapshot formatting and
  source-only tasks, with cancellation/version gates and explicit unavailable symbols.

- Add explicit v2 enum/match source formatting with syntax-only parsing, token/
  comment/AST/fixed-point gates and existing guarded writes.

- Add explicit v2 single-entry lock generation/check and locked commands using
  the existing strict format, guarded publication and captured-input verification.

- Connect v2 commands to guarded single-entry package capture, preserving entry
  diagnostics and explicitly refusing dependency/module/locked modes.

- Add bounded deterministic linked v2 IR inspection through the library and
  explicit `build --experimental-v2 --emit ir`, without source execution.

- Add independent experimental-run process stdin/text-file grants, preserving
  bounded host operations, default denial and completed-effect ordering.

- Add opt-in v2 language tests with single-program lowering, fresh invocation state,
  finite suite admission and existing human/JSON reports, without stdin/file hosts.

- Add opt-in `svr run --experimental-v2` with finite VM limits, arguments/status,
  captured output and no stdin/filesystem authority.

- Add opt-in `svr check --experimental-v2` root-source checking with bounded human/JSON diagnostics and no execution.

- Support checked-program matches in if/else-if/else/while bodies with lexical
  scopes, conservative returns, existing IR/fuel and bounded statement recursion.

- Execute recursive matches directly inside checked-program match arms, preserving
  immutable scopes, exhaustive returns, original spans and existing snapshot fuel.

- Execute checked root-program function links in the opt-in Rust VM with callee-
  owned graphs, shared policy/services and cumulative linked-graph admission.

- Add bounded multi-function source checking and canonical forward/mutual call
  links through IR validation; linked VM execution remains explicitly refused.

- Add opt-in single-entry source-unit checking/execution with source-derived enum/
  record/alias types, original token coordinates and explicit unsupported-form refusal.

- Add independently granted v2 VM file services and explicit ProcessHost delegation,
  preserving standard results, effect ordering and atomic materialization admission.

- Add a narrow explicit v2 VM line-input host with minimum fuel preflight,
  post-read control/full-result admission and oversized-return refusal.

- Add explicit v2 VM program arguments and captured exit status, preserving
  ordinary execute refusals and shared fuel without ambient process access.

- Extend the opt-in v2 Rust VM with pure string/result helpers and println,
  preserving atomic fuel admission; fix its Unicode len to existing UTF-8 bytes.

- Extend the opt-in v2 Rust VM with snapshot array slice/append/concat, atomic
  result admission and iterative nested copying; default CLI/backends unchanged.

- Add opt-in Rust function-IR execution with nominal enums, iterative recursion,
  shared budgets/control and a restricted memory-only standard adapter.

- Add opt-in flat model-v2 runtime values, nominal enum value operations and
  atomic wrapper/label/subtree fuel admission; function execution remains open.

- Add versioned function control-flow emission with stores/scopes/matches/returns
  and bounded definite-availability/tag validation; runtime integration remains open.

- Add checked versioned expression register emission/validation, preserving
  per-operand conversion order and deferring function control flow/runtime work.

- Add bounded checked callable structural mapping, validating source/fact links
  and coverage without fresh inference; executable IR emission remains open.

- Add opt-in scoped callable declaration/read/write/match-scrutinee identities
  with independent binding/frame quotas, preserving prior typed callable behavior.

- Add opt-in typed callable expression facts, unit-owned IDs, call/context links
  and array-element conversion contexts, preserving earlier callable quotas.

- Add opt-in source enum constructors/self-recursion with bounded call-target and
  conversion-context facts. Preserve standard-only APIs; execution remains open.

- Add opt-in source-driven single-function checking with direct exhaustive matches,
  inferred locals, immutable payloads and required return coverage; execution remains open.

- Add opt-in closed-scope match arm type checking, immutable payload bindings,
  nominal scrutinees and conservative return coverage; executable integration remains open.

- Add opt-in versioned standalone match syntax with original spans, qualified
  patterns, ordinary untyped arm bodies and bounded admission; execution remains open.
- Add an opt-in bounded single-enum syntax API with ordered payload variants,
  exact source spans and structured quota refusals. Preserve executable parsing
  and public lexer behavior; enum execution/matching remains unimplemented.

- Add approved snapshot array_append/array_concat with first-directed dependent
  types, recorded numeric widening, checked length limits and atomic fuel.
  Add reusable filtering/record package consumption across Rust/Node/native execution.

- Add std::array_slice with dependent typed snapshot results, strict half-open
  bounds and atomic selected-child fuel admission in Rust and Node/Worker engines.
  Preserve numeric widening and nominal package types; add a reusable package example.

- Add an approved opt-in Node Worker generator with versioned shared-atomic
  cancellation, exact elapsed checks and explicit host/fuel checkpoint ordering.
  Preserve ordinary script/default policies and document shared process stdio.

- Add approved opt-in Rust cooperative cancellation and elapsed execution checks,
  with shared permanent tokens, explicit checkpoints, preserved host errors/effects
  and iterative cleanup. Existing defaults and JavaScript behavior stay unchanged;
  blocked I/O and helper work cannot be preempted.

- Add read-only logical value footprint measurements with checked aggregation,
  borrowed depth-bounded scratch traversal and deep/wide/Unicode regressions.
  Add approved opt-in cumulative node/text fuel in Rust and JavaScript with
  atomic admission, no release refunds and explicit host-result ordering.
  Defaults and native policy remain unchanged; physical heap is not metered.


- Add opt-in Rust/JavaScript execution limits for dispatched IR instructions and
  printed UTF-8 bytes. Share counters across callees, preflight complete lines and
  preserve iterative exhaustion cleanup; existing CLI/native/default policy stays
  unchanged. Heap/work/interruption accounting remains separate.


- Harden deep runtime values with iterative Rust cloning, runtime-owned cleanup
  and Rust/JavaScript display. Preserve public enum destructuring and snapshot
  behavior; add explicit caller cleanup and isolated 16,000-level regressions.


- Implement approved structural array annotations (`[T]`) and `std::array_len`
  across executable/package interfaces, bounded alias resolution, checked numeric
  conversion, formatter syntax and explicit IDE symbols. Preserve nominal element
  privacy and snapshot values; add an independent local library/consumer fixture.


- Implement deterministic local-source lock generation, strict bounded verification,
  guarded Linux publication and explicit CLI locked check/run/build/test using
  exactly the captured manifest/source inputs. Add alias/diamond, mutation,
  malformed-lock and publication-conflict regressions.
- Include registered sources in bounded package formatting, preserving lockfiles;
  support existing public standard result records through registered interfaces.


- Route CLI check/run/build/test through complete checked registered graphs when
  any selected package declares modules, preserving entry-only/application modes,
  source diagnostics and root-only test discovery. Older graph tools still refuse
  module registrations explicitly; explicit locks and package formatting follow below.

- Support visible nominal record-valued fields and aliases through finite
  canonical field references, preserving private type boundaries, source-owned
  errors and existing recursive declaration behavior without layout expansion.

- Add an opt-in nominal primitive-field record compiler for registered modules,
  preserving aliases, exported/private type boundaries, field/call locations and
  checked lowering. Fix internal root-record versus module-named-root identity
  collisions. Complete context/editor integration remains open.

- Compile registered primitive-function modules through source-owned checked
  semantics/lowering, with direct file-local imports, private helpers and bounded
  interfaces. An independent consumer executes in interpreter/Node JavaScript;
  record-capable compilation, CLI routing and locks are implemented separately.

- Load registered sources into bounded, file-owned buffers/ASTs/imports and check
  module headers/duplicates through an additive Linux-guarded API. General record/
  type linking, locks and executable CLI integration are implemented separately.

- Begin approved G12 with an opt-in bounded module source inventory, deterministic
  ownership/order, portable path checks and Unix file-alias detection. Existing
  entry-only Rust APIs still reject module manifests.

- Document the approved G12 multi-file module and local source-lock contracts.
  Implementation is incremental; G11 remains partial.

- Add optional same-snapshot project route checks with file-owned outcomes and
  cumulative route/placeholder/message limits, counting nested diagnostic copies.
  Existing checker diagnostics, E4096 and default APIs remain unchanged.

- Compose existing local route target/path/placeholder checks beside opaque
  model/auth/render headers, preserving source identities, coverage gaps and
  unavailable contexts. Default checking and runtime routing are unchanged.

- Add optional same-snapshot project ordinary-function checks with file-owned
  diagnostics and cumulative function/message limits. Preserve default checking,
  unavailable contextual scopes and all original incomplete-check diagnostics.

- Add opt-in ordinary function checking beside opaque model/auth/render headers,
  reusing core semantics and original diagnostic spans. Missing contextual scopes
  remain explicit; older APIs, default checking and runtime behavior are unchanged.

- Add optional same-snapshot project model-field checks with file-owned outcomes
  and cumulative field/diagnostic quotas. Keep default checking and E4096 unchanged.

- Add the approved source-only primitive model-field checker: newline/comma
  fields, local primitive aliases, exact duplicate/unknown diagnostics and bounded
  whole-request refusal. Model runtime and default project checking are unchanged.

- Integrate partial auth/model/enum/page/view headers into opt-in source/project
  coverage with original-token boundary checks and shared quotas. Preserve every
  opaque body/guard gap and all existing incomplete-check diagnostics.

- Require explicit same-name Page/View parameters for local page-route placeholders
  (E4145), preserving target/path failures. Render typing, decoding, implicit
  context injection and complete argument binding remain separate.

- Retain page/view headers with explicit unresolved annotations and opaque bodies,
  without default Unit results. Resolve local page-route names only to unique
  Page/View declarations using shared E4042 rules; runtime rendering is unchanged.

- Retain model/enum declaration headers with opaque body gaps and check local
  policy model-name references using shared E4081 rules. No field/variant typing,
  model operations or authorization behavior is introduced.

- Add auth-aware source/project syntax coverage using original tokens and shared
  checker snapshots/quotas. Preserve every untyped condition gap and existing
  incomplete-check diagnostics; older APIs remain strict and no access is granted.

- Retain bounded auth containers and policy-header syntax while preserving every
  where clause and unsupported context field as unparsed. Header coverage never
  establishes policy validity or runtime permission; default checking is unchanged.

- Check local scheduled-declaration targets against retained ordinary/task names,
  sharing scanner E4071/E4072 rules and quotas. Preserve identity and source spans;
  task body contracts and runtime scheduling remain separate.

- Require explicit same-name local handler parameters for route placeholders
  through an additive scoped API (E4144). Retain target/shape errors; HTTP value
  typing, decoding, injection and complete argument binding remain separate.

- Resolve unqualified local route targets to retained ordinary declarations with
  exact identifier ranges. Keep missing-target E4033 diagnostics and explicitly
  reject unavailable or ambiguous contexts; argument/body validation is separate.

- Add structured route/page path validation sharing existing scanner rules and
  diagnostic codes. Retain exact member spans and bounded diagnostic results;
  target/argument binding and full application validation remain separate.

- Add scoped core-semantic validation for self-contained ordinary functions
  beside application entries. Preserve core diagnostics/source ranges and
  explicitly reject missing import/service/task context. Entry validation
  remains separate; default checking is unchanged.

- Add opt-in combined project syntax coverage from checker source snapshots,
  sharing bounded quota accounting. Preserve semantic and incomplete-check
  diagnostics even when syntax gaps are empty or metadata is unavailable.

- Combine application entry/declaration syntax coverage through private original-
  token filtering. Preserve Unicode spans and unknown entry ranges; existing
  checker APIs remain strict and semantic validation remains separate.

- Retain structured app entry containers and existing binding members through an
  entry-only syntax API. Unknown fields remain explicit gaps; scanner grammar,
  default checking and runtime behavior remain unchanged.

- Retain dotted application import nodes and add opt-in same-snapshot syntax
  coverage beside service checking. Bounded metadata failures remain separate
  from semantic diagnostics; default checker/CLI behavior is unchanged.

- Retain bounded application callable/body ASTs and structured service containers/
  prototypes through library syntax APIs. Preserve original spans and explicit
  coverage gaps; these nodes do not establish semantic validity or execution.

- Retain bounded application alias/record AST nodes through a partial Rust syntax
  API, with original byte spans and explicit unparsed ranges. Reuse executable
  parsing; metadata exhaustion never bypasses ordinary application inspection.
  Correct record annotation spans in the shared parser.

- Validate formatter read handles before consuming bytes. Linux x86/x86_64/aarch64
  opens reject FIFO/link substitutions without blocking; Windows rejects opened
  reparse handles. New execution evidence is Linux-only.

- Retain exact-host formatter qualification reports and command logs in three-OS
  CI. Missing/wrong compilers cannot qualify; unsupported write hosts explicitly
  report verified refusal rather than supported writes.

- Add guarded `fmt --package --write DIR`: validate and stage every graph entry
  before sequential atomic file replacements. Late failures report partial
  completion and retain earlier writes. Manifests stay unchanged; dependencies
  outside the root are included. New execution qualification is Linux-only.

- Add bounded read-only `fmt --package --check DIR` over compiler-selected local
  graph entries, with snapshot conflict checks and source-owned diagnostics.
  Package writes remain deferred.

- Format individual package sources containing imports through the compiler’s
  shared import parser. Preserve ordered import identities and comments without
  loading dependencies; directory formatting remains deferred.

- Enable guarded `fmt --write` on Linux, with permissions, links, no-op identity,
  conflict and cleanup qualification. Package formatting and other-host writes
  remain deferred.

- Add compiler-owned package editor task targets, bounded related-source saves
  and dirty-manifest refusal. Package Check uses full entry compilation through
  `--executable-package`. Real Linux VS Code workspace tasks/navigation and
  relocated installed tools pass; global IDE save policy remains user-controlled.

- Add checked cross-file package function definitions, hover and source-owned
  outlines. Aliases retain canonical targets; closed/unsaved source fingerprints
  and UTF-8 ranges are verified before navigation. Projection limits return
  unavailable metadata without changing valid compilation or falling back to
  name guesses. Package task acceptance is now covered by the subsequent completion checkpoint.

- Add LSP package workspace diagnostics through compiler-owned graph context and
  bounded unsaved overlays. Dependency edit/close/rejection and watched-file
  events invalidate stale generations; diagnostics retain correct file ownership.
  Imported-package navigation and package tasks are covered by subsequent checkpoints.

- Add JSON-only package checking with `--overlays-stdin`: bounded versioned
  unsaved-source frames, request-ID correlation, strict input rejection and capped
  failure reports. No sources are written and no package code executes. LSP
  workspace integration remains pending.

- Add a compiler-only unsaved package-source overlay API with canonical existing
  entry validation, independent buffer quotas and unchanged package compilation
  budgets. No source files are written. Package-aware LSP integration is pending.

- Compile ordinary executable package graphs from retained checked evidence with
  cumulative metadata limits and unchanged shared IR budgets. Valid graphs beyond
  metadata limits now fail with E3025, including package checks. An explicit
  legacy reference API remains available; no exhaustion fallback is performed.

- Compile standalone run/build/native output and language tests from retained
  checked evidence with finite metadata limits. Admitted programs preserve IR
  and runtime behavior. Previously valid programs exceeding metadata limits can
  now fail compilation with E3025; plain semantic check remains separate. Package
  compilation and IDE evidence fallback retain their existing paths.

- Improve compiled native Windows UI resizing: constrain horizontal rows, wrap
  labels/action captions, bound row inputs and fit initial window dimensions to
  the primary work area. Add responsive native interaction regression coverage.

- Add opt-in Windows `install.ps1 -Upgrade`, retained backups, activation-failure
  restoration and explicit receipt-checked rollback. Preserve default overwrite
  refusal; reject changed/unmanaged files and links in existing installations.

- Include optional editor/LSP/extension sources and an installed editor launcher
  in Windows toolchains. Check Node 22+ and adjacent compiler version, verify
  supplied VSIX sources, and add relocated/installed developer-tool acceptance.
  Ordinary compiler and application execution retains its Node-free boundary.

- Extend IDE evidence with parameter/local declarations and references for hover
  and definition navigation. Preserve lexical shadowing, initializer lookup and
  separate variable/function namespaces. Show explicit annotations; leave inferred
  types unavailable. Bound local collection within the shared metadata allowance.

- Add opt-in checked function evidence via `check --format json --symbols` and
  IDE function hover, same-file go-to-definition and outline. Preserve plain JSON
  reports and suppress stale/invalid-source evidence. Bound metadata and derive
  standard-library hover signatures from the trusted registry.

- Add an experimental standard stdio language server and VS Code extension with
  live standalone compiler diagnostics, UTF-16 positions, syntax highlighting,
  basic keyword/type suggestions and explicit check/build/run process tasks.
  Gate VS Code compiler processes on Workspace Trust. Include Neovim/Eglot setup;
  semantic completion, hover, definitions and package-root checking remain planned.

- Extend the editor with independent file drafts, file switching with cursor
  retention, and validated workspace import/export. Bound workspaces to 64 files
  and 512 KiB combined source; keep compiler actions scoped to the selected file.

- Add an experimental local Sovra editor: source open/download, browser drafts,
  clickable compiler diagnostics, IR/JS builds and interpreter Run/Stop. Bound
  requests, output and execution time; authenticate local mutation requests.
  Single-file preview only; execution retains the user's real OS permissions.

- Add primary project scan discovery/read quotas with E4150 on exhaustion and
  fail-fast bounded reads. Enable approved defaults of 16 MiB/source, 64 MiB total,
  4,096 source paths, 4,096 directories and 100,000 yielded entries (ADR 0040).
  Optional body inspection remains separate.

- Stream delegated formatter/doctor diagnostic escaping without temporary escaped
  copies. Encode DEL/C1 controls in check/test/doctor JSON strings while preserving
  decoded values, ordinary Unicode, help/report layout and schema version 1.

- Escape terminal controls in main CLI-generated human errors, including source
  diagnostics, file labels, paths and argument failures. Stream the existing
  C0/DEL/C1 display policy; preserve structured data and program output.

- Stream guarded CLI check I/O and language-test configuration messages into
  bounded storage while preserving JSON path identity and declaration ranges.
  Leave public raw application inspection/error-string contracts unchanged.

- Stream direct project/manifest/package diagnostics into bounded messages.
  Format cycle paths and route/page path problems from borrowed context without
  joined/full intermediate error strings; preserve text, locations and rules.

- Stream semantic diagnostic formatting into a bounded text buffer, stopping
  Display/Debug output on exhaustion while preserving existing clipped messages
  and source metadata. Avoid constructing full oversized formatted messages.

- Share lowering record identity metadata across inferred local/array facts,
  avoiding full record-name copies per fact while preserving runtime behavior.

- Resolve canonical alias names once per lowering invocation with shared terminal
  strings. Preserve checked record identities and defensive cycle behavior.

- Replace repeated IR alias scans with reverse-dependency worklists and share
  immutable record-field widening metadata across aliases. Preserve checked
  source behavior and emitted IR.

- Reuse one immutable type registry per prepared package across export/interface
  normalization and consumer imports. Preserve owner type scope, per-copy quota
  charges and fresh resolution on each compilation.

- Add caller-selected cumulative interface string materialization limits (E3024),
  charging canonical metadata and qualified label copies before retention.
  Normalize record fields directly without synthetic signature copies; enable
  the approved 128 MiB package default (ADR 0039).

- Add caller-selected cumulative package AST/interface and linked IR quotas,
  counting final qualified names before retention. Import signatures without
  cloning their bodies. Enable approved graph defaults of 1,000,000 AST nodes,
  250,000 interface nodes, 4,000,000 instructions and 256 MiB linked strings
  (ADR 0038).

- Add caller-selected AST admission and retained IR instruction/string budgets,
  with incremental emission accounting and safe numeric jump backpatching.
  Enable approved standalone defaults of 250,000 AST nodes, 1,000,000 IR
  instructions and 64 MiB retained IR string payload (ADR 0037).

- Bound guarded diagnostic messages to 4 KiB with UTF-8-safe truncation notices
  and released oversized backing buffers. Preserve codes, severity, owning files,
  full source ranges and valid long identifiers; keep the raw diagnostic API intact.

- Bound malformed-manifest and executable semantic error reports to 256 entries
  as produced, with E4018/E3019 stopping notices and original source ranges.
  Abort malformed classification early and skip exhausted validation without
  accepting partial programs; valid-source acceptance is unchanged.

- Enable approved graph defaults: 1 MiB per manifest, 8 MiB total canonical
  manifests and 8,192 alias edges. Bound initial classification/project manifest
  reads and stop excess edges before target resolution; retain explicit graph
  budgets for trusted Rust hosts (ADR 0036).

- Enable approved 64 MiB aggregate entry-source defaults for package compilation
  and package command/test paths. Count canonical packages once and bound each
  read by its remaining allowance; retain explicit budgets for trusted Rust hosts
  and the existing 16 MiB per-file ceiling.

- Enable approved executable frontend defaults: 16 MiB/source, 1,000,000 tokens
  and 256 lexer/parser diagnostics. Bound CLI/test/package-entry file reads to
  the byte ceiling plus one detection byte; preserve formatter's stricter 1 MiB
  guard and apply token budgets before package import removal. Aggregate package
  and semantic/IR quotas remain follow-up work.

- Add caller-selected Rust frontend source-byte, non-EOF token and diagnostic
  budgets. Reject exhausted inputs without partial success; accepted ADR 0035
  defines their approved default policy and remaining resource work.

- Add isolated coverage-guided source and native IR fuzz targets using libFuzzer
  and AddressSanitizer, reviewed seeds, reproducible corpus generation, a separate
  dependency lockfile and private failure-triage guidance. Normal compiler
  dependencies and source semantics are unchanged.

- Reject malformed/out-of-range native IR numeric literals at decode time,
  including in unreachable instructions. Preserve supported runtime numeric forms
  and add deterministic source/image mutation regressions to CI without executing
  mutated programs.

- Close a parser structural-depth bypass through unbraced `else`/`else if`
  chains. Count statement nesting before recursive parsing, add isolated
  adversarial CI regressions, and document compiler security trust boundaries
  and remaining work without claiming certification or sandbox isolation.

- Check field values in unknown-record and duplicate-field literals instead of
  hiding independent errors. Report missing required fields in stable alphabetical
  order and avoid rescanning the literal for each required field.

- Preserve independent semantic errors in assignment values and computed-call
  expressions after an invalid target. Retain original ranges in JSON reports
  and continue rejecting invalid source before execution or code emission.

- Resolve aliases when validating the `main` return type. Reject non-Unit
  aliases and records with E3010 before execution or code emission; retain valid
  Unit aliases and existing unresolved-type diagnostics.

- Preserve a record declaration's nominal identity when constructing through a
  type alias, including alias chains and module-local aliases. Fix equality and
  displayed type names in both engines and standalone builds.

- Refine nested-array element inference after leading empty rows. Reject
  incompatible later rows and preserve Float widening through replacement rows
  without mutating the source Int array. Extend depth-boundary regressions.

- Preserve leading UTF-8 BOM characters in generated JavaScript standard-input
  lines, matching Rust. Add BOM-only, CRLF/EOF, byte-boundary and persisted-title
  regressions; distinguish character data from empty input and EOF.

- Validate Windows toolchain, native-app and desktop ZIP members before
  extraction. Share path/type checks across verifiers and reject traversal,
  Windows path aliases, duplicate/conflicting members and link metadata.

- Extend installed-toolchain self-tests with conditional-write and busy-save
  checks. Run same/mixed-engine conflict tests against an explicitly selected
  installed compiler before standalone native-build acceptance.

- Implement approved ADR 0034 `std::write_text_if` for cooperative conditional
  text replacement in the interpreter and JavaScript backend. Migrate Task
  Manager saves to reject stale/busy writes. Retained crash markers require
  deliberate recovery; unconditional writes remain unchanged.
- Preserve UTF-8 BOM bytes in generated JavaScript text-file reads, matching
  Rust and exact snapshot comparisons.

- Preserve other writers' files when `svr new` fails after reserving its directory.
  Rollback now tracks successful file reservations instead of deleting template
  pathnames unconditionally; collision and early-failure regressions cover cleanup.

- Disarm formatter staging cleanup after successful publication so a reused
  staging pathname cannot cause deletion of another writer's file. Retain cleanup
  on failed publication and add Windows attribute-change and comment-placement tests.

- Implement approved ADR 0032 `svr fmt`: comment/token/AST preservation, fixed-point
  checks, bounded output and guarded atomic Windows writes. Package imports and
  non-Windows write qualification remain open.

- Implement approved ADR 0031 `svr doctor`: target-specific environment checks,
  bounded probes, installed inventory/hash validation and human/JSON reports.
- Preserve install.ps1 during installation so the installed file inventory
  matches its integrity manifest; add an installation regression check.

- Avoid duplicate parser diagnostics when malformed indexed statements are probed
  as assignment targets; preserve valid indexed reads and assignments.

- Recover malformed statements within nested blocks, retaining `else` branches,
  later statement keywords and original error ranges without executing invalid code.

- Reduce parser declaration-error floods by recovering at the next declaration
  or module boundary, retaining original diagnostic codes and source ranges.

- Add a private reusable Windows Sovra UI renderer and desktop packaging over
  existing view/event functions; native Task Manager and unrelated component
  controls execute real Sovra behavior. Public UI compatibility remains planned.

- Implement approved ADR 0028 language tests, human/JSON reports, template and reusable-library tests, and installation/CI verification.

- Implement accepted ADR 0027 host-native `svr build --emit exe --output PATH`,
  installed runtime payload validation, offline linking, staged cleanup and
  atomic no-overwrite publication. Preserve IR/JS stdout behavior. Rust/Cargo
  and a compatible linker are required on the developer machine; clean-machine
  release qualification remains pending.
- Add `svr new <path>` with an embedded console template and refusal to
  overwrite existing destinations under approved ADR 0026.
- Add Windows toolchain packaging, integrity checks, user installation/PATH
  setup and a package-contained create/check/run self-test. Clean-machine
  qualification remains outstanding.
- Add executable local dependency packages, exported nominal records and
  compound value isolation, plus process input/status and bounded UTF-8 file
  APIs with interpreter/JavaScript regression coverage.
- Exercise nine dogfood application slices, persistent Task Manager and Notes,
  File Manager/browser UI and HTTP host adapters through real Sovra execution.
- Add the approved internal checked-IR standalone executable proof, bounded
  image validation, build failure isolation and 18-case Task Manager parity.
  Windows proof packaging includes checksums, imports and a self-test. Public
  native build syntax and clean-machine deployment remain outstanding.
- Add production-readiness gates, cross-platform CI, a Rust 1.74 check,
  source-package boundaries and manual verification artifacts. The earlier
  developer-preview publication plan is superseded.
- Add experimental opt-in service-call checking with coverage errors and
  machine-readable member-call references. Application execution remains planned.
- Add versioned JSON reports with `svr check --format json` for source/project
  validation and input I/O errors, preserving default human output.
- Require explicit type annotations on every function parameter, including
  unused and module functions, under approved ADR 0002. Untyped declarations
  now report semantic error `E3014` at the parameter name. This is a source
  compatibility change; local `let` inference and default Unit returns remain.
- Add an executable function example and a course lesson on typed signatures,
  inferred locals and migration from untyped parameters.

## 1.0.0 target metadata - not release evidence

- Set the intended v1.0.0 version metadata for the Sovra toolchain.
- Align the canonical `svr` binary version metadata and project
  manifests for the source compiler, IR, interpreter, and JS backend.
- On 2026-10-09 the user revised v1 scope to a production-quality Task Manager
  CLI and qualified toolchain, with later versions adding platform capacity.
  This supersedes the earlier full-platform/registry requirement. Metadata
  does not establish publication or readiness; see the version-one gates.

## 0.1.0 - 2026-09-03

- Added interpreter support for user-defined function calls, parameters, and
  return values.
- Added semantic diagnostics for function call arity and parameter types.
- Added runtime arithmetic and comparison operations for numeric, string, and
  boolean values, including explicit division-by-zero errors.
- Added validation for duplicate declarations and the `main` entry signature.
- Added a bounded interpreter call depth with an explicit recursion error.
- Promoted `svr run` to the M6 user-facing execution milestone with strict
  `.svr` path validation and end-to-end CLI coverage.
- Created the M0 repository foundation.
- Added the canonical `svr` CLI with version, help, and reserved command
  handling.
- Added compiler-stage extension points and the first `.svr` example.
- Added the M1 lexer with tokens, source spans, comments, literals, and
  structured diagnostics.
- Added the M2 recursive-descent parser and AST.
- Added M3 name resolution, basic type checking, and a minimal IR.

- Experimental v2 checked IDE outlines now include same-file module containers,
  including empty modules, under existing all-or-nothing symbol quotas.

- Experimental v2 IDE outlines now include checked root/module record and alias
  declarations, with source-owned ranges and shared optional projection limits.

- Experimental v2 IDE navigation now resolves direct enum parameter/return
  annotations, including arrays, through checked module-owned enum identities.

- Add the approved v2 generic annotation syntax API with source-owned spans,
  flat postorder nodes and independent resource limits. Generic execution stays
  unsupported pending checked declaration/substitution/specialization work.

- Add bounded v2 generic parameter-list ownership, approved bound spelling and
  source-owned annotation parameter-use binding; generic execution stays unsupported.

- Add bounded root generic alias syntax with attached parameter ownership,
  original-source span mapping and aggregate source/name admission.

- Add generic record syntax attachment with shared parameter identities,
  original field/error coordinates and cumulative multi-field resource limits.

- Add generic enum syntax attachment with shared payload parameter ownership,
  original source ranges and cumulative variant/type admission.

- Add generic function header attachment with shared annotation parameter owners,
  cumulative signature admission and explicitly opaque bounded body retention.

- Add bounded generic type-header registration and annotation name/arity/Key
  resolution, with registry-owned IDs and explicit unresolved-alias bound refusal.

- Add bounded transparent generic alias environments with cycle rejection,
  symbolic target checks, identity-based substitution and expanded Key proofs.

- Add complete generic type-definition annotation checking with sealed source-owned
  proofs, unused-definition validation and cumulative field/payload admission.

- Add sealed generic function signature checks with complete type contexts,
  table-owned IDs and cumulative annotation admission; bodies remain unchecked.

- Add bounded iterative generic type equivalence and exact normalized in-process
  keys, preserving nominal/parameter identities across separate annotation graphs.

- Add an initial symbolic generic body checker for immutable inferred copies,
  returns and equality, with source-owned facts and cumulative resource admission.

- Check explicit immutable generic local annotations with original-token source
  ranges, initializer conversions and cumulative syntax/expansion admission.

- Add bounded concrete numeric arithmetic/ordering facts to symbolic generic bodies,
  preserving explicit operand widening; generic execution remains pending.

- Add bounded Boolean symbolic `&&`/`||` facts with both operands checked;
  executable short-circuit lowering remains open.

- Add bounded concrete String concatenation/ordering facts to generic bodies,
  without implicit formatting or execution.

- Check fixed-type mutable generic locals and direct assignment with original
  binding identity and explicit widening; executable generic integration remains open.

- Check generic expression statements while preserving result types and source
  identity; discarded results do not bypass type checks or return requirements.

- Check symbolic generic conditionals with concrete Bool conditions, isolated
  branch scopes, flat branch facts and conservative all-path return coverage.

- Check symbolic generic while loops with concrete Bool conditions, isolated scopes
  and conservative return coverage; executable backedge lowering remains open.

- Check read-only generic array indexing with concrete Int indexes and exact
  projected element types; bounds checks and execution remain future integration.

- Check direct mutable generic array indexed replacement with exact element
  types and explicit widening; runtime snapshot/bounds integration remains open.

- Check context-typed generic array literals, including nested/empty arrays and
  explicit scalar item widening, without aggregate coercion or execution.

- Check parameter-independent generic record field reads with nominal field
  identity; dependent substitution, field writes and execution remain open.

- Project direct generic record-parameter fields from validated receiver arguments
  while preserving caller parameter ownership; composite substitution remains open.

- Substitute composite generic record field types under bounded graph/work/depth
  admission, preserving caller identities and per-node receiver/field/alias sources.

- Parse bounded source-owned explicit `::<T,E>` type-argument lists for future
  generic call/constructor binding; no checked call or execution is enabled.

- Resolve explicit generic type arguments in their caller scope under cumulative
  copy/expansion limits; callee matching and executable call integration remain open.

- Match checked generic arguments to root callees with exact owner, arity and
  Key-bound proofs under finite matching budgets; executable calls remain open.

- Instantiate bound generic callee signatures into caller-scoped parameter/return
  types under cumulative/per-type graph limits, retaining per-node provenance.

- Match supplied generic caller value expressions to instantiated parameter types
  with exact owners and explicit scalar widening; source-call/executable integration remains open.

- Parse bounded source-backed explicit generic call envelopes, retaining original
  ranges and grammar-checked types while leaving value expressions opaque.

- Parse original-token generic call value expression ranges with exact boundaries,
  bounded AST/work admission and iterative cleanup; typed body calls remain open.

- Scope symbolic binding/expression IDs to exact immutable arenas as well as
  signatures; reject cross-arena references and identity exhaustion without reuse.

- Check original explicit root generic body calls, including nested/contextual-array
  arguments, lexical shadowing and cumulative pipeline budgets; execution remains open.

- Check explicit symbolic generic record construction with complete field/type/bound
  validation, source-order values, shared source provenance and cumulative budgets.

- Check explicit symbolic generic enum construction with exact nominal variants,
  nullary/Unit distinction, contextual payload types and cumulative budgets.

- Check exhaustive generic enum pattern headers and exact payload contexts for
  selected body-arena bindings; arm typing/source-site integration remains open.

G13 E2: additive bounded standalone generic source compilation/execution and
ordinary nongeneric calls, with original-file diagnostics. CLI/backend/package
integration remains open. See `docs/reference/generic-source-v2.md`.

G13 E2: explicit standalone `check/run --experimental-generics`, JSON checking,
original-file error locations and finite host-free execution. Packages, hosts,
arguments and native/JavaScript builds remain unavailable in this mode.

G13 E2: standalone `build --experimental-generics --emit exe --output PATH`
packages the host-free Rust VM and checked embedded generic source without a
Node.js runtime dependency. Adds a separate bounded image format, startup
rechecking and existing staged/no-overwrite linking. Direct machine-code lowering,
packages, JavaScript and other-host qualification remain open.

G13 E2: guarded root-package generic check/run/native build, optional captured-byte
`--locked` verification and `lock --experimental-generics`. Adds reduced guarded
source admission; dependency/module linking remains refused. Generic runtime
diagnostics now use E3125 to avoid existing package E3124. Linux/procfs package
capture only; standalone source commands remain separate.

G13 E2: compiler-owned qualified std::Option/std::Result through complete generic
source, exhaustive matching and host-free execution. User names remain available;
semantic-stage budgets include standard definitions. Generic native feature word 3
advertises standard enums while retaining old word-1 decoding without them.
std::Error, adapters, postfix propagation and later G13 work remain open.

G13 E2: compiler-owned std::Error with checked String code/message and optional
Int offset, qualified record construction and Result payload execution. Adds
bounded zero-argument nominal-head admission and native feature word 7, retaining
old word-1/word-3 environments. Adapters and postfix propagation remain open.

G13 E2: additive bounded symbolic Option/Result propagation compatibility plans
with exact failure types, opaque owner checks and original source provenance.
This Rust API does not enable source `?` or executable propagation.

G13 E2: bounded syntax-only postfix question-mark capture for complete blocks,
with original spans, chained/nested applications and parser rollback. No legacy
AST is exported; ordinary lexer/parser/source/CLI/native modes remain strict.
Executable propagation integration remains open.

G13 E2: opt-in original-source postfix function/body checking with typed expression
nodes, shared Option/Result compatibility, active scopes and cumulative virtual-
node/depth/comparison/work admission. Specialization/concrete/lowering explicitly
refuse propagation; executable source/CLI/native support remains disabled.

G13 E2: opt-in finite specialization and concrete propagation rechecking with
closed per-instance payload/return contracts, original descriptor validation and
shared stage/propagation budgets. Instruction emission remains unconditionally
gated for question-bearing tables; source/CLI/native `?` stays disabled.

- G13 E2: add opt-in bounded propagation operation layouts over closed concrete
  sites, preserving distinct input/caller success types and exact failure payloads.
  Execution remains gated.

- G13 E2: add inspection-only typed propagation IR with standard-wrapper validation,
  explicit frame-exit semantics and shared IR quotas. Runtime execution stays gated.

- G13 E2: add explicit propagation VM execution with bounded payload snapshots,
  compatible failure reconstruction and current-frame cleanup. Source/CLI/native
  profiles remain strict.

- G13 E2: add opt-in complete-source propagation compilation/execution with original
  file diagnostics and independent stage quotas. Ordinary CLI/native profiles stay
  strict; native encoding explicitly refuses the new source profile.
- Correct propagated failure-wrapper nominal label-byte accounting to match
  ordinary enum constructors and retained value footprints.

- G13 E2: enable postfix propagation under existing experimental generic
  source/root-package commands and native feature word 15, preserving historical
  image profiles and bounded startup rechecking.

- G13 E2: add immutable legacy standard-result schemas and reusable explicit Sovra
  adapters, with native feature word 31. Existing builtin return types remain
  unchanged; generic host/builtin-call bridging remains open.

- G13 E2: bridge host-free std::parse_int/std::split_once into checked generic
  source/IR/VM execution with legacy result parity and shared resource admission.
  Native feature word 63 preserves startup checking; host I/O remains gated.

- G13 E2: bridge String len/lines_unique/escape_terminal with byte/line/control
  parity, primitive result checking and shared temporary/output admission. Native
  feature word 127 preserves startup checking; host I/O remains gated.

- G13 E2: add polymorphic std::to_string and generic std::array_len with checked
  iterative formatting, exact output preflight and shared temporary task admission.
  Native feature word 255 preserves startup checking; host I/O remains gated.

- G13 E2: bridge array append/concat/slice with owned snapshots, explicit recursive
  Int-to-Float widening, half-open bounds and shared copy preflight. Native feature
  word 511 preserves startup checking; host I/O remains gated.

- G13 E2: add explicit generic LSP/VS Code buffer diagnostics and standalone
  Check/Run tasks; declare the shared task type and gate unsupported editor actions.
  Default/v2 modes remain separate; generic navigation/formatting remain open.

- G13 E2: add bounded optional generic declaration JSON symbols and editor
  outlines/declaration hover. No references are guessed; optional exhaustion emits
  an unavailable envelope while preserving successful compilation.

- G13 E2: add checked direct function-call references and generic editor definition/
  hover using retained callee tokens and resolved identities. Source sites are
  deduplicated across specialization; optional exhaustion remains fail closed.

### 2026-11-16 — Generic local binding navigation

Retain bounded checked read/write binding identities and project original parameter, local and match-payload definitions for optional generic editor navigation. Hover reports role/name; inferred type display remains open. Task Manager unchanged.

### 2026-11-16 — Checked generic local type hover

Parameter/local/match-payload hover now renders retained symbolic types, arrays and ordered nominal arguments, transparently expanding aliases. Shared work/string/task limits refuse the full optional evidence envelope on exhaustion. Execution and Task Manager behavior unchanged.

### 2026-11-16 — Generic function-header type navigation

Optional generic symbols now link checked parameter/return annotation names to user records, enums and aliases, preserving written alias and type/function namespace identities. Shared reference/work/byte limits discard incomplete evidence. Broader type contexts remain open. Runtime and Task Manager unchanged.

### 2026-11-16 — Generic source/package test command

`svr test --experimental-generics [--locked] [--format human|json]` now runs nongeneric parameterless Unit `test_` roots without invoking main. Every selected specialization is checked before any run, suites have finite count/source allowances, runtime failures continue with fresh VM state, and package capture/lock failures use empty test reports. Dependency/module linking remains open. No editor polish or Task Manager changes.

### 2026-11-16 — Versioned linker input capture

Add immutable syntax-independent graph capture across registered modules and local dependencies, preserving canonical package ownership, original per-file bytes and exact manifests. Reduced source count/aggregate/per-file quotas retain guarded read safeguards. Captured graph locks reuse the strict existing schema and match parsed-source locks. Generic import linking and CLI enablement remain open; capture/lock success does not authorize execution. No editor or Task Manager changes.

### 2026-11-16 — Generic package declaration ownership and import visibility

Add table-owned source/declaration identities over captured graph bytes, original module/root declaration spans, duplicate namespace refusal and direct dependency module imports scoped to the declaring file. Explicit lookup preserves private/export boundaries and refuses foreign table IDs. Cumulative syntax/import quotas apply across files. This is declaration/visibility metadata, not semantic checking or executable linking; CLI dependencies/modules remain refused. No editor or Task Manager changes.

### 2026-11-16 — Semantic generic package types and signatures

Add package-aware lookup to the existing generic registry and reuse transparent alias, declaration and signature checking across captured source scopes. Same-spelled nominal/function declarations retain distinct owners, aliases use definition imports, and private/unimported types or invalid arity/Key bounds refuse. Semantic admission is cumulative across the graph. All flat body profiles reject package-scoped signatures until executable contexts are linked. CLI dependencies/modules and native feature contracts remain unchanged. No editor or Task Manager work.

### 2026-11-16 — Source-aware generic package bodies and calls

Add an explicit package body profile that binds qualified/unqualified calls and nominal constructors/patterns through per-source visibility and canonical IDs. Every original body is checked under shared limits, with source-owned diagnostics; flat body profiles remain gated. Existing explicit specialization/IR/VM stages execute imported generic calls, transitive helpers, remote records, enums/matches and standard propagation in tests. CLI dependency/module enablement, selected-root compilation and package native-image integration remain open. No editor or Task Manager work.

### 2026-11-16 — Owned generic package compilation and CLI graph consumers

Add selected-root compilation over retained source/type/body proofs through bounded specialization, concrete checking, layouts and IR, with original-file runtime diagnostics. Generic package Check/Run and Lock now consume registered local dependency/module graphs and verify locks against the same captured bytes before compilation. Source mutation/removal does not alter compiled execution. Multi-file Test/native Build remain explicit refusals without output side effects; single-entry paths/native feature words remain unchanged. No editor or Task Manager work.

### 2026-11-16 — Shared-snapshot generic package test suites

Generic package Test now consumes registered module/dependency graphs after lock verification, discovers only source-order root-entry Unit `test_` functions, and preflights every selected specialization before execution. Replay shares immutable frontend/body proofs with bounded two-pass instance reservations and fresh VM state/policy; runtime outcomes retain original dependency-file locations. Dependency/module test functions and main are not auto-invoked. Standalone source suites remain separate; multi-file native Build remains gated. Runnable package example includes two consumer tests. No editor or Task Manager work.

### 2026-11-16 — Source-backed generic package native builds

Add a distinct SVRGP version-one source/topology image with strict structural/resource admission and full in-memory semantic rechecking. Embedded package inputs carry logical locations and cannot yield guarded filesystem capture authority. Locked generic native Build now embeds registered graphs; runners reject arguments and retain finite host-free policy without reading original projects. Source-only/single-entry builds retain the old SVRG2 image contract. A native generic record/propagation consumer runs after project deletion with no usable PATH and ignores conflicting logical-path files. Existing builder publication/cleanup is reused. No editor or Task Manager changes.

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
