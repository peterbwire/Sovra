# Development log

## 2026-10-08: Reject unresolved member-call contracts

- Reproduced silently accepted calls on missing and nested unresolved receivers.
  Structured inspection now records these as unresolved ordinary calls and emits
  E4133 at the whole call range. Local/unresolved member metadata stays available.
  Known service and ambiguous service receivers retain existing service checks.
- Updated missing-import and unknown-local regressions to require errors rather
  than treating receiver classification as validation. Added a CLI JSON fixture.
- Validation: **325 library + 42 CLI = 367 passed**, with required Windows CLI
  execution. Library results are in `target/v1-unresolved-members-tests.log`;
  after updating the CLI's expected diagnostics, all CLI tests passed in
  `target/v1-unresolved-members-cli-tests.log`. Strict Clippy, formatting and
  whitespace checks passed.
- M12 remains partial. Next: audit declaration coverage and retained application
  types before enabling full default checking; runtime/publication gates remain.

## 2026-10-08: Reject callable members on scalar receivers

- Reproduced silent acceptance of member calls on scalar locals, literals and
  ordinary function results. Structured inspection now emits E4133 with the whole
  call range and a receiver-type explanation, retaining member-call metadata.
- Service-shadowing tests now assert the new scalar-call errors alongside existing
  service argument, arity and unknown-operation diagnostics. Record-field call
  rejection remains unchanged. Added a CLI JSON regression fixture.
- Validation: **324 library + 42 CLI = 366 passed**, with required Windows CLI
  execution (`target/v1-scalar-member-calls-tests.log`). Strict Clippy, formatting
  and whitespace checks passed.
- Unknown receiver types remain a partial validation boundary. Next: audit
  unresolved member receivers and their diagnostic/inspection coverage under
  ADR 0009; runtime and library publication remain unfinished.

## 2026-10-08: Resolve explicit task return annotations

- Validation: **323 library + 42 CLI = 365 passed**, with required Windows CLI
  execution (`target/v1-task-return-annotations-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
- Reproduced acceptance of unknown explicit task result annotations in structured
  inspection. The checker now retains their resolved types and emits E4131 for
  unknown names, even in unused tasks; omitted task results remain unspecified.
- Project coverage exposed a separate scanner bug: task result arrows were also
  parsed as scheduled-task wiring. Parenthesized task declarations now bypass
  schedule parsing while existing scheduled-task validation remains intact.
- Coverage includes primitive results, aliases, directly imported nominal records,
  declaration diagnostic ranges and CLI JSON failure for an unknown result type.
- Task return-path checking, scheduling execution and runtime result semantics
  remain unfinished. This does not mark M12 or version 1.0 ready.
- Next: define the remaining task result contract before implementing return-path
  checks; continue other approved application validation work independently.

## 2026-10-08: Validate unused task parameter annotations

- Reproduced missing diagnostics for unused task parameters with unknown types.
  Structured application checking now emits E4134 per unresolved task parameter,
  retaining the owning declaration range and source file. Tasks remain excluded
  from ordinary callable signatures.
- Regression coverage includes unused unknown annotations, valid primitives,
  local aliases and directly imported exported records, plus CLI JSON failure.
  Duplicate parameters were already rejected by the project scanner; a regression
  preserves E4027/E4098 across service, function and task declarations.
- Validation: **322 library + 42 CLI = 364 passed**, with required Windows CLI
  execution (`target/v1-task-annotations-tests.log`). Strict all-target/all-feature
  Clippy, rustfmt and whitespace checks passed.
- M12 remains partial; this is annotation validation, not task execution or
  scheduling support. Next: audit remaining task contracts and application
  validation boundaries before runtime and publishing release gates.

## 2026-10-07: Record-bearing exported application functions

- Validation: **319 library + 42 CLI = 361 passed**, with required Windows CLI
  execution (`target/v1-exported-record-functions-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
- Reproduced blanket rejection of public record function interfaces. Replaced it
  with nominal visibility checks against validated public field metadata. E4116
  still rejects private records, including aliases, and invalid private-record
  exports no longer provide a selectable consumer signature.
- Shared public field interfaces now travel with ordinary function signatures.
  Imported results preserve declaring-module identity and readable fields without
  re-exporting dependency type names. Scalar/builtin interfaces remain unchanged.
- Added public/private alias, factory result, consumer-name collision and
  transitive-name isolation tests. Extended the multi-file CLI example and its
  JSON identity/declaration-owner assertions.
- Remaining: broader application syntax/type coverage, recursive cross-file type
  design, native test/runtime support and library publication/release qualification.

## 2026-10-07: Shared application module type resolution

- Validation: **316 library + 42 CLI = 358 passed**, with required Windows CLI
  execution (`target/v1-module-types-tests.log`). Strict Clippy passed; the CLI
  inspection matrix was rerun successfully after extending its example/JSON
  assertions. Formatting and whitespace checks passed.
- Extended the existing iterative executable alias resolver with an internal
  imported-type registry; existing executable callers retain their original API.
  Application modules now resolve direct imported alias targets and record fields.
- Shared resolved module interfaces feed service contracts and body inspection
  from one source snapshot. Bounded propagation follows validated exports; private,
  invalid and partially inspected exporters do not provide usable interfaces.
- Preserved canonical nominal identity and direct-name visibility while carrying
  nested public field metadata through dependency chains. No structural record
  equivalence, alias exports or placeholder records were introduced.
- Added imported alias constructors, exported fields, multi-file chains, invalid
  exporter and unresolved cycle coverage; extended the real CLI example and JSON
  field evidence assertions.
- Remaining: ordinary record-bearing exports, application runtime and publishing.
  Cross-file type-dependent cycles requiring placeholder interfaces remain
  unsupported (E3017); the pass does not claim full recursive-type support.

## 2026-10-07: Nominal JSON type descriptors

- Validation: **312 library + 42 CLI = 354 passed**, with required Windows CLI
  execution (`target/v1-nominal-json-tests.log`). Strict all-target/all-feature
  Clippy, formatting and whitespace checks passed.
- Added schema-one descriptors for local declared/initializer/resolved types,
  ordinary result/argument types and member-call arguments. Existing scalar fields
  and argument counts remain unchanged. Records carry opaque nominal identity;
  scalars carry canonical names; unresolved/unsupported evidence remains null.
- Added same-layout distinct-record, constructor/call identity, scalar field and
  unresolved-value assertions, including real multi-file CLI JSON parsing.
- Imported alias targets and record fields remain unfinished and need a shared
  module type-resolution pass. This slice closes the inspection gap where valid
  records and unresolved values were indistinguishable in scalar-only JSON.

## 2026-10-07: Imported record types in service contracts

- Validation: **311 library + 42 CLI = 353 passed**, with required Windows CLI
  execution (`target/v1-imported-service-contracts-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
- Reproduced unresolved direct-import service parameters/returns. Resolved them
  from immutable declaring-module export interfaces, preserving nominal identity.
- Service results retain exported field metadata even when the caller imports
  only the service module. This does not make the type's source name transitively
  visible. Private/transitive contract names remain E4117 and yield no signature.
- Added service implementation/return, metadata propagation and visibility tests;
  extended the multi-file CLI example with an imported record relay contract.
- Remaining: imported alias targets/record fields, ordinary record-bearing
  exports, nominal JSON evidence and application runtime/publishing.

## 2026-10-07: Qualified application record names and constructors

- Validation: **308 library + 42 CLI = 350 passed**, with required Windows CLI
  execution (`target/v1-qualified-application-records-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.
- Collected exported record interfaces from fully inspected source snapshots and
  exposed qualified names only through direct imports. No source alias/private
  record or transitive name is exported. Repeated imports remain idempotent.
- Qualified record paths now resolve in locals and ordinary function signatures;
  constructors share existing required-field, compatibility and depth checks.
  Field metadata also works for imports from modules without any service.
- Added direct/private/transitive, duplicate-import, declaring-identity and
  same-name nominal mismatch tests. Extended the multi-file CLI example.
- Remaining: imported types in service/record declarations and alias targets,
  record-bearing ordinary exports, nominal JSON metadata, runtime and publishing.

## 2026-10-07: Exported service record field metadata

- Validation: **305 library + 42 CLI = 347 passed**, with required Windows CLI
  execution (`target/v1-exported-service-records-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
- Applied approved explicit export-struct semantics to application service record
  metadata. Direct imported service results retain exported field interfaces and
  nested nominal identity; consumer aliases/declarations cannot reinterpret them.
- Private records remain opaque across files. E4116 rejects exported field types
  resolving to private records, including through aliases. Invalid type graphs
  expose no metadata. Shared immutable interfaces avoid copying a module's record
  map into each operation; scopes deduplicate repeated interfaces.
- Added exported/private nested-field and alias privacy regressions plus a checked
  multi-file example. Qualified imported type spellings/construction and ordinary
  exported functions carrying records remain unfinished, as do runtime/publishing.

## 2026-10-07: Application record construction

- Validation: **303 library + 42 CLI = 345 passed**, with required Windows CLI
  execution (`target/v1-application-constructors-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
- Added file-local and alias constructors with nested values, lexical call
  inspection and existing structural-depth guards. E4139 rejects unknown record
  types, missing/extra/duplicate fields and incompatible/unresolved field values.
- Valid constructors propagate nominal identity. Int values can widen to Float
  fields; invalid constructor types do not propagate to locals or arguments.
- Preserved bare identifier conditions; parentheses enable record expressions in
  conditions, and call arguments accept constructors. Added valid/invalid syntax,
  type, nested, depth and CLI coverage and extended the checked record example.
- Remaining: cross-file field metadata, exported application record types,
  nominal JSON metadata and application runtime/publishing. Construction here is
  statically checked, not executed by an application runtime.

## 2026-10-07: Application record field reads

- Validation: **300 library + 42 CLI = 342 passed**, with required Windows CLI
  execution (`target/v1-application-fields-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
- Retained validated field interfaces with canonical nominal identities and
  exposed them through lexical scopes. File-local field reads now propagate
  primitive/nested nominal types through arguments, returns and local bindings.
- Added E4138 for invalid member reads on known receivers. Record fields are not
  callable service operations; field calls fail with E4133. Invalid field graphs
  do not supply field metadata. Existing service receiver rules are preserved.
- Added nested/aliased field, scope, invalid member/call and CLI regressions;
  extended the record example with a field passed to a typed service operation.
- Remaining: construction, cross-file field metadata, exported application
  records, runtime and publishing. Field reads are implemented for records whose
  declarations are available in the inspected file.

## 2026-10-07: Nominal application record contracts

- Validation: **298 library + 42 CLI = 340 passed**, with required Windows CLI
  execution (`target/v1-application-record-contracts-tests.log`). Strict Clippy,
  formatting and whitespace checks passed. CLI coverage includes a valid record
  flow and rejection of same-shaped distinct records with E4119.
- Added file-local record declaration parsing and field validation through the
  existing executable parser/type checker. Alias and record names share duplicate
  checks; unknown field types and duplicate fields keep E3017/E3008.
- Record annotations now carry declaring-module identity through services,
  ordinary functions and locals. Aliases preserve identity; same layouts or names
  from different files do not imply compatibility. Raw single-file inspection
  uses an isolated synthetic owner; project inspection uses canonical owners.
- Exported ordinary functions exposing private record contracts fail with E4116.
  Added checker examples and unit/CLI nominal-mismatch and invalid-field coverage.
- Remaining: application record construction, field reads, exported type imports,
  nominal JSON metadata, runtime and publishing. This is useful contract checking,
  not complete record execution or complete M12 validation.

## 2026-10-05: Application integer literal bounds

- Validation completed 2026-10-07: **295 library + 42 CLI = 337 passed**, with
  required Windows CLI execution (`target/v1-application-integer-bounds-tests.log`).
  Strict all-target/all-feature Clippy and formatting checks passed. Updated the
  course and production checkpoint to describe the numeric validity boundary.
- Reproduced silent acceptance of three oversized integer literals. Application
  inspection now reports executable diagnostic E3012 at original token ranges,
  including unused initializers, calls, returns and statically skipped branches.
- Preserved Int type evidence as in executable semantic analysis. Diagnostics
  determine validity; type evidence alone does not prove a value fits its domain.
- Added boundary, grouping, Unicode/CRLF, function-isolation and CLI JSON checks.
- Remaining: application records and field typing, runtime and publishing. This
  check does not evaluate arithmetic or prove runtime overflow cannot occur.

## 2026-10-05: File-local scalar aliases in application checking

- Validation: **293 library + 42 CLI = 335 passed**, with required Windows CLI
  execution (`target/v1-application-aliases-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed.
  Optimized build passed the alias example with imported Float JSON evidence;
  the invalid fixture returned exit 1 with E4129/E4120/E4119 as expected.
- Reproduced rejection of scalar aliases across service, function and local
  annotations. Added top-level alias token extraction and reused the executable
  parser and iterative alias resolver rather than defining another type system.
- Forward aliases now resolve to canonical scalar types before ordinary interface
  collection and service/body checks. Imported exported functions retain their
  owner's resolved signature. Alias names stay private to their declaring file;
  no exported aliases, record equivalence or new source syntax was introduced.
- Invalid graphs retain E3008/E3017 with source ownership, even when unused. No
  validated alias map escapes an invalid graph. Malformed/nested declarations
  retain incomplete inspection rather than silently disappearing.
- Signature aliases, imported interfaces and body inspection share one source
  snapshot; a regression changes the on-disk alias after snapshot capture and
  verifies the supplied snapshot still determines the resolved service type.
- Added unit, integration and CLI coverage for valid/invalid aliases, owner-local
  imports, source ranges, a 5,000-alias chain, tasks, service bodies and JSON type
  evidence. Added the application-aliases example and course lesson.
- Remaining: application record types/construction/fields, broader body typing,
  runtime and publishing. Ordinary project checks remain partial wiring checks.

## 2026-10-05: Validate discarded application expressions

- Validation: **282 library + 42 CLI = 324 passed**, with required Windows CLI
  execution (`target/v1-discarded-expressions-tests.log`). The CLI fixture checks
  failure status, complete inspection coverage and exact JSON expression ranges.
  Strict all-target/all-feature Clippy, formatting and whitespace checks passed.
- Reproduced three silently accepted unresolved expression statements before
  fixing inspection. E4137 now rejects unresolved non-call statement types at
  their full expression ranges. Known discarded values remain valid; direct
  calls retain their existing contract diagnostics.
- Added project regression, per-function evidence isolation and nested/skipped
  loop coverage, plus a CLI fixture for structured diagnostics and exit failure.
- Remaining: named application types, broader expression support, application
  runtime and publishing. This is incremental M12 validation, not M12 completion.

## 2026-10-05: Local binding validation and JSON type evidence

- Validation: **280 library + 42 CLI = 322 passed**, with required Windows CLI
  execution (`target/v1-local-type-evidence-tests.log`). Strict all-target,
  all-feature Clippy, formatting and whitespace checks passed. The optimized
  build verified E4135/E4136 failures and Int-to-Float JSON widening evidence;
  the valid application-functions example also passed its service-call check.
- Reproduced silent acceptance of three unused unresolved locals. Added E4135
  at unsupported annotation ranges and E4136 at unresolved initializer ranges.
  An unresolved annotation takes precedence over an unresolved initializer to
  avoid duplicate incomplete-validation diagnostics for the same declaration.
- Retained local binding records with declared/initializer/validated types and
  all three source ranges. Known incompatibilities still use E4120 and invalid
  bindings do not propagate types. Added schema-one local_bindings inspection
  metadata with lexical occurrences, preserving shadowed names and widening.
- Added source-range, Unicode/CRLF, shadowing, unknown-type and CLI regressions.
  Generic metadata fixtures retain their original annotations and now expect
  the corresponding local annotation diagnostic.
- Remaining: implement named application types and unresolved expression support;
  explicit rejection is not a substitute for those features or runtime execution.

## 2026-10-05: Boolean guards in application inspection

- Validation: **278 library + 42 CLI = 320 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-logical-guards-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced rejection of a valid mixed comparison/logical condition. Application
  expressions now support the existing && and || operators with executable
  precedence: ||, &&, comparisons, addition/subtraction, multiplication/division.
- Both operands must have known Bool types to propagate a Bool result. Known
  invalid operands use E4121; unresolved conditions retain E4125. Both sides are
  inspected statically even when runtime short-circuiting could skip one side.
- Added operator precedence, source-range, nested-call ordering, unresolved-call
  and structural-depth regressions, plus valid/invalid CLI coverage. Updated the
  application-functions example with a combined guard. No unary syntax or new
  execution semantics were introduced.
- Remaining: named application interfaces and broader structured application
  syntax/type validation, runtime execution and publication infrastructure.

## 2026-10-04: Exported cross-file application functions (ADR 0013)

- Validation: **275 library + 42 CLI = 317 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-function-imports-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.
  Optimized build passed the exported import example and declaring-file JSON
  assertions; private access and argument mismatch failures returned the expected
  exit 1 with E4133/E4129 in the negative fixture.

- Recorded the user's affirmative continuation as approval of the concrete
  Option A proposal. Added top-level export fn recognition and multi-segment
  qualified calls such as app::helpers::format through existing direct imports.
- Project inspection reads one source snapshot per discovered file, collects
  interfaces from fully parsed files and then checks bodies against direct exported
  interfaces. Cycles do not recursively load bodies; duplicate imports are
  idempotent. Bare helpers remain in their own file and imports are nontransitive.
- Imported signatures retain canonical declaring files. JSON ordinary-call records
  add declaration_file while preserving caller/argument locations. Duplicate or
  partially parsed declarations do not provide selectable exported interfaces.
- Added positive/negative CLI fixtures, examples/application-imports, and regressions
  for private/nontransitive access, cycles, duplicate imports, lexical/module
  namespace separation, owner-correct errors and invalid export interfaces.
- Remaining: named application interfaces, complete application parsing/typing,
  runtime execution and package publication. Existing executable package imports
  and service visibility rules remain separate.

## 2026-10-04: Unused ordinary parameter annotation validation

- Validation: **270 library + 42 CLI = 312 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-ordinary-annotations-tests.log`).
  The pre-fix regression reproduced zero diagnostics instead of two. Formatting
  and whitespace checks passed; strict all-target/all-feature Clippy passed.

- Added E4134 for unresolved ordinary parameter annotations even when no call
  reaches the function, applying accepted ADR 0009. Diagnostics retain the owning
  declaration range and are emitted once per unresolved parameter.
- Added regression and CLI coverage for Text/Any annotations; scope-only project
  fixtures now use canonical String annotations so their original receiver and
  shadowing assertions continue to isolate those rules.
- Reviewed ADR 0006/0009 before cross-file ordinary work. Their existing service
  and type import rules do not specify ordinary callable visibility/spelling.
  Prepared ADR 0013 with exported module-qualified calls and requested approval
  under AGENTS.md; no imported-call syntax or visibility was silently selected.
- Next: implement approved cross-file function import semantics after a decision;
  named application types and full execution remain unfinished.

## 2026-10-04: Explicit ordinary call resolution and JSON inspection

- Validation: **269 library + 42 CLI = 311 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-call-inspection-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed; doc stage
  completed with 0 tests.
  Optimized build checks verified valid stdlib inspection metadata and the expected
  three E4133 failures/unresolved records for the negative call fixture.

- Reproduced silently accepted missing, misspelled stdlib and locally shadowed
  callees (zero instead of three diagnostics). Retained unresolved bare/qualified
  and computed call records; opt-in checking now emits E4133 at whole call ranges.
  Existing member-call receiver boundaries remain separate.
- Added schema-one `ordinary_calls` metadata for resolved ordinary/builtin and
  unresolved calls, containing declared return types, argument evidence, reasons
  and source ranges. Only fully inspected files contribute; resolved signature
  metadata is explicitly not proof that a call's arguments validate.
- Added Unicode/CRLF range, nested/computed-call, mixed-coverage JSON tests and CLI
  checks. Updated shadowing/duplicate tests to require the new unresolved-call
  diagnostic while preserving their original resolution assertions.
- Remaining: implement cross-file ordinary resolution and general callable/type
  semantics rather than treating these explicit rejections as feature completion.
  Application runtime, native testing and registry publication remain unfinished.

## 2026-10-04: Application stdlib contract resolution

- Validation: **267 library + 42 CLI = 309 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-application-stdlib-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed; doc stage
  completed with 0 tests. A compiled stdlib smoke program produced identical
  interpreter/JavaScript output (`value: 12`, `5`).
  Optimized build passed the application-functions check; the invalid stdlib
  fixture returned exit 1 with the expected E4129/E4128/E4130 diagnostics.

- Reproduced lost result evidence for std::len/std::to_string/print calls. Qualified
  expression nodes now retain names as well as spans; the existing Rust-owned
  registry supplies builtin signatures and the bare print compatibility alias.
- Builtin arity, known argument types and unresolved arguments use the ordinary
  checker diagnostics. Builtin Any slots are explicit metadata and never grant
  wildcard behavior to a user annotation. Validated builtin results propagate
  into application expressions, locals and return contracts.
- Added E4132 for ordinary declarations colliding with builtins. Local bindings
  still shadow bare print. Regression coverage includes result types, invalid
  calls, unresolved arguments, shadowing and user Any rejection; added CLI fixture
  and upgraded the application-functions example to use std::to_string.
- Remaining: cross-file ordinary calls, named application types, complete unknown
  name/expression resolution and runtime execution. No new stdlib runtime APIs
  were introduced and this does not complete M12.

## 2026-10-04: Ordinary application functions and contracts

- Validation: **264 library + 42 CLI = 306 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-ordinary-functions-tests.log`).
  Strict all-target/all-feature Clippy, formatting and whitespace checks passed;
  documentation test stage completed with 0 tests.
  The optimized release build checked `examples/application-functions` successfully;
  the negative ordinary-call fixture returned exit 1 with the expected five codes.

- Reproduced missing result types for forward ordinary calls. Reused the existing
  structured parser in signature collection and body inspection passes, retaining
  public entry points. No separate line-scanned function resolver was introduced.
- Added same-file primitive interfaces and ordinary call records. Forward and
  mutually recursive calls propagate validated results through conditions, locals,
  service arguments and returns. Lexical bindings shadow function names; duplicate
  names supply no selectable signature. Task/service names remain separate.
- Added E4127 duplicate declarations, E4128 call arity, E4129 argument mismatches,
  E4130 unresolved call contracts/arguments and E4131 ordinary return failures.
  Ordinary returns use the existing conservative branch/loop completeness rules.
- Added recursion, shadowing, duplicate, call-boundary, source-range and return
  regressions, CLI positive/negative fixtures, and `examples/application-functions`.
  The existing generic metadata fixture now also reports its previously unchecked
  ordinary return expression; its original raw metadata remains intact.
- Remaining: cross-file/qualified ordinary calls, named application types and
  general unresolved expression handling. Same-file body validation does not
  implement the application runtime or complete M12.

## 2026-10-04: Explicit unresolved condition and return diagnostics

- Validation: **259 library + 42 CLI = 301 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-unresolved-types-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced silent acceptance of unresolved conditions/service returns (zero
  instead of three diagnostics). Added E4125 for unresolved condition types and
  E4126 for unresolved service return types, retaining expression source ranges.
- These are incomplete-validation errors, distinct from known mismatches.
  Complete syntax coverage can coexist with failed type validation. Ordinary
  project scans and raw inspection remain separate from opt-in contract checks.
- Added project regression and human/JSON CLI coverage. Ordinary calls, named
  application types and other unresolved expressions still need actual resolution;
  this change does not claim full type checking or runtime readiness.

## 2026-10-04: Structured application control flow

- Validation: **258 library + 42 CLI = 300 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-control-flow-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced rejection of a valid conditional service-call body before extending
  structured inspection to if/else-if/else and while. Calls, local types and
  returns are retained in each lexical block; malformed bodies still reject the
  entire file. Older incomplete-coverage fixtures now use malformed conditionals.
- Function inspection retains a conservative return-path result. E4123 requires
  both conditional arms to return; loops may execute zero times, and a later
  unconditional return can establish completeness. E4124 identifies known
  non-Bool condition expressions. Unknown condition types remain unresolved.
- Added tests for conditional paths, loops, scope isolation, bounded else-if
  chain depth and CLI coverage/diagnostics. No runtime execution is added.
- Next: resolve unknown expressions and ordinary calls; complete application
  parsing/typing and runtime remain open before version 1.0.

## 2026-10-04: Missing returns in supported service bodies

- Validation: **255 library + 42 CLI = 297 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-missing-returns-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced a non-Unit service body accepted without a return. Added E4123 at
  the full function range when a resolved non-Unit implementation has no explicit
  return. Unit bodies and external declaration-only contracts remain exempt.
- The rule relies on the inspector's restricted straight-line statements and
  unconditional nested blocks. Branches/loops still reject full-file inspection
  with E4096; this is not general control-flow analysis. Unknown return expression
  types remain unresolved rather than being treated as compatible.
- Added regression and CLI coverage for missing returns, nested returns, Unit
  bodies, declaration-only contracts, unknown expressions and diagnostic spans.
- Next: structured control-flow inspection, unresolved return expressions and
  ordinary function result types. Full application typing/runtime remain unfinished.

## 2026-10-03: Explicit service implementation returns

- Validation: **254 library + 42 CLI = 296 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-service-returns-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced three missed return mismatches (zero diagnostics instead of three).
  Function inspection now retains explicit return type evidence and source ranges,
  resetting it between declarations and respecting nested lexical scopes.
- Resolved service implementations compare known returns with their owning
  file/service contract. E4122 identifies incompatible expressions or bare return
  keywords. Bare returns are Unit; Int-to-Float widening remains valid.
- Added regressions for return ranges, scope shadowing, unknown expressions,
  declaration isolation, default Unit contracts and CLI reporting.
- Remaining: missing-return/return-path checks, unknown return expression types,
  ordinary function results and full implementation validation. No execution
  support or complete M12 claim is implied.

## 2026-10-03: Service call result propagation

- Validation: **252 library + 42 CLI = 294 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-service-results-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced missing diagnostics for an imported Float-returning service used
  directly and through a local in an Int argument position (zero instead of two).
- Project inspection now supplies canonical resolved contracts to expression
  typing. Existing public source/body inspection entry points retain their
  signatures and remain contract-free unless called through project inspection.
- Call results propagate only for visible unshadowed/unambiguous services with
  correct arity and known compatible arguments. Invalid/unknown calls do not
  produce misleading downstream type errors. Added imported-call, shadowing,
  invalid-inner-call, unknown-argument and CLI regressions.
- Remaining: ordinary function call results, named application types, service
  implementation return/body validation and execution. This does not complete M12.

## 2026-10-03: Primitive compound expression checking

- Validation: **250 library + 42 CLI = 292 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-compound-types-tests.log`). Strict
  all-target/all-feature Clippy, formatting and whitespace checks passed. Added
  `docs/course/service-argument-types.md` for the verified checker subset.

- Reproduced missing Float type evidence for a mixed numeric service argument.
  The application expression representation now retains binary operators and
  propagates primitive results into arguments and inferred/annotated locals.
- Matched existing scalar rules for arithmetic, numeric widening, String `+`,
  numeric/String ordering and numeric/String/Bool equality. Known invalid operands
  produce E4121 at their expression range; unresolved operands remain unknown.
  Invalid inner expressions do not produce cascading outer type errors.
- Added result-type, nested-invalid-expression and CLI coverage for valid and
  invalid compound arguments. This is type validation, not evaluation: overflow,
  division by zero, call-result resolution and complete body typing remain outside
  this application-checking slice.

## 2026-10-03: Primitive annotated local initializers

- Validation: **248 library + 42 CLI = 290 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-local-initializers-tests.log`).
  Strict all-target/all-feature Clippy, formatting and whitespace checks passed.

- Reproduced two missing diagnostics for an incompatible String initializer and
  a widened Float local passed to an Int service parameter (zero instead of two).
- Added retained initializer mismatch records to function inspection and E4120
  diagnostics through opt-in service checking. Known compatible primitive
  initializers propagate the declared type, preserving Int-to-Float widening.
  Invalid initializers do not cause misleading downstream argument errors.
- Added regressions for unknown initializers/annotations, diagnostic spans,
  per-function isolation, valid String initializers and CLI error behavior.
- Remaining: compound expressions, unknown initializer resolution, named
  application types and complete implementation typing. Ordinary project checks
  remain partial wiring checks; `inspect_body` remains a call-inspection API.

## 2026-10-03: Lexical service argument types

- Validation: **246 library + 42 CLI = 288 passed**, no failures/skips, with
  required Windows CLI execution (`target/v1-service-variable-tests.log`).
  Strict all-target/all-feature Clippy, formatting and whitespace checks passed.

- Extended the existing application scope with optional primitive binding types;
  retained the untyped binding API and literal evidence field. Argument records
  expose additional binding evidence through `resolved_type()`.
- Primitive declaration parameters and unannotated locals initialized from known
  literals/bindings now participate in E4119 service argument checks. Reverse
  lexical lookup respects initializer visibility, nested scopes and unknown
  bindings that hide outer known types.
- Added a nested-scope regression and CLI cases for parameter/copy mismatches and
  valid inner shadowing. Annotated locals conservatively remain unresolved until
  initializer compatibility is validated; no unknown type is treated as proof
  of compatibility. Compound expressions and implementation typing remain next.

## 2026-10-03: Literal service-call argument validation

- Reproduced acceptance of `mail.send((true))` for a String parameter (expected
  one diagnostic, received zero). The inspector now retains positional literal
  type evidence and argument ranges, including parentheses and nested calls.
- Resolved service contracts reject literal mismatches with E4119. Existing
  Int-to-Float widening is accepted; arity failures do not cascade into type
  errors. Receiver shadowing and unresolved receivers retain their prior rules.
- Added CLI fixtures for String/Int/Float/Bool mismatches and valid widening,
  plus regressions for argument spans and deliberately unresolved expressions.
- Full Windows suite passed: 244 library and 42 CLI tests, no failures/skips,
  with required CLI execution (`target/v1-service-arguments-tests.log`). A further
  argument-evidence regression was added and checked separately.
- Remaining: variables, compound expressions, implementation return/body types,
  named application types and runtime execution. Literal success is not complete
  argument validation; JSON schema and syntax coverage semantics are unchanged.

## 2026-10-03: Resolved primitive service signatures

- Added `project::service_types` with typed parameters/operations, canonical
  service ownership and declaration spans. Only fully resolved contracts enter
  the typed result. Unit, Bool, Int, Float and String are supported; omitted
  returns resolve to Unit. Missing annotations retain E4097, unresolved types
  use E4117 and owner-path failures use E4118.
- Connected signature diagnostics to opt-in `check --service-calls`. Reproduced
  the missing integration first: the regression expected two unresolved-type
  diagnostics and received zero. Ordinary scanner metadata and JSON schema 1
  remain unchanged. Added `examples/service-contracts` as a checker-only example.
- Coverage includes all five primitives, default returns, invalid-operation
  omission, canonical identities, missing annotations/owners, CRLF/Unicode
  declaration spans and CLI JSON/exit behavior. Existing arity fixtures now use
  supported primitive contracts; the generic metadata fixture remains intact.
- Validation: **243 library + 42 CLI = 285 passed**, no failures or skips, with
  `SOVRA_REQUIRE_CLI_EXECUTION=1`; see `target/v1-service-signatures-tests.log`.
  Strict all-target/all-feature Clippy and formatting passed; doc stage: 0 tests.
- Remaining: retain argument expressions for type checking, connect application
  named types and check implementation bodies. This does not complete M12 or
  supply application execution or registry publishing for version 1.0.

## 2026-10-03: Module-scoped named types and iterative alias resolution

- **Upgrade:** Added a dedicated type-scope resolution pass under accepted ADR
  0009's declaring-module rule. It rewrites module-local type references in
  signatures, record fields, aliases, local annotations and constructors before
  validation. The caller's parsed AST and source spans are preserved; the typed
  program retains qualified identities and resolved scalar alias targets.
- **Compiler/linker:** Removed globally flattened bare module types from semantic
  registries and lowering metadata. Separate modules can independently declare
  `Point`/`Scalar`; root declarations cannot capture their signatures/fields.
  Package interfaces and record relocation now use the validated scoped program,
  preserving module/package nominal identity and private package boundaries.
- **Crash fix:** A 20,000-alias source reproduced Windows stack overflow
  (exit 3221225725) in the prior debug compiler. Replaced recursive alias chasing
  with an iterative cached walk and sorted traversal. Scalar alias targets are
  flattened in validated metadata so lowering does not repeat long chains.
  Root-record alias paths are retained when flattening could rebind a shadowed
  name on re-analysis. Existing unknown/cycle/duplicate codes remain stable.
- **Coverage:** Repeated names across modules and root, Float alias widening,
  cross-module nominal mismatch through a dependency, bare-name leakage,
  duplicates/builtin names/unknowns/cycles, input/span preservation and idempotent
  analysis. Added a 4,096-alias unit regression and 20,000-alias CLI check/run/IR/JS
  build regression. `examples/scoped-types` is executed in both engines and CLI.
- **Validation:** **241 library + 42 CLI = 283 passed**, no failures/skips, with
  strict CLI execution (`target/v1-type-scopes-tests.log`). Strict all-target/
  all-feature Clippy, formatting, doc stage (0 tests) and diff checks passed.
  Debug and optimized builds now check the previously crashing 20,000-alias
  source. The optimized scoped-type example prints `1.5`, `1.5`, `1`, `root`.
- **Compatibility/limits:** Leaked bare module type names must become qualified
  references. Existing qualified same-file record/alias access is preserved;
  this does not add a new same-file visibility policy or exported alias syntax.
  Package consumers still require explicit record exports/direct imports.
  Full retained expression typing, application checking/runtime, native tests,
  registry publication and ADR 0012 compound-value semantics remain unfinished.
- **Next:** Build retained typed application/service declarations on the scoped
  type foundation and continue production correctness work. Version-one scope
  remains the full platform and third-party publishing, not this compiler slice.

## 2026-10-03: Close compound and control-flow depth-guard gaps

- **Reproduced:** A parser regression accepted an array tree beyond depth 128;
  a public-AST regression accepted an over-depth expression in an else body.
  Inspection also found unchecked field/index chains, source block recursion
  and assignment targets in the semantic preflight.
- **Fixed:** Applied accepted ADR 0007's existing limit to square brackets and
  braces before recursive source parsing, and checked array/record/field/index
  node depth before wrapping children. Semantic preflight now uses a borrowed
  statement worklist for both branches and loops, checks targets and values,
  and bounds nested blocks. E2007/E3018 remain the corresponding diagnostics.
- **Coverage:** Boundary tests, public-AST regressions, CLI checks/run/build/JS
  rejection and exact JSON ranges for 2,048-level compound/control-flow inputs.
  Extended the shared CI/candidate probe with arrays and if/while block shapes.
- **Validation:** **235 library + 40 CLI = 275 passed**, no failures/skips, with
  strict CLI execution (`target/v1-compound-depth-tests.log`). Formatting,
  strict all-target/all-feature Clippy, doc stage (0 tests) and whitespace checks
  passed. Debug and optimized Windows GNU builds each passed all **84** depth
  probe cases; logs: `target/v1-depth-debug.log`, `target/v1-depth-release.log`.
- **Limits/next:** This enforces the existing structural bound, not a total
  runtime/allocation budget or unrestricted public-AST destruction guarantee.
  Hosted platform/MSRV evidence and full application/library-publishing gates
  remain open. Compound-copy/equality semantics still await ADR 0012 approval;
  further independent compiler correctness work can proceed meanwhile.

## 2026-10-03: Whole-array numeric conversion correctness

- **Reproduced:** Type checking accepted replacing a Float array with an Int
  array, but both engines retained integer leaves. A regression obtained `1`,
  `2`, `3`, `4` where Float division required `1.5`, `2.5`, `3.5`, `4.5`.
  Nested array construction and row replacement had the same missing conversion.
- **Fixed:** Lowering uses retained destination types for assignments and array
  elements, emitting `WidenFloatArray { depth }` for compound conversion. Both
  engines traverse iteratively, preserve shape and empty arrays, and reject
  invalid public IR with matching errors. JavaScript conversion builds new
  numeric arrays so it cannot change a source Int binding.
- **Coverage:** Whole-array replacement, mixed nested arrays, row replacement,
  empty replacements, source binding preservation and six malformed-IR cases.
  Removed redundant Float flags from lowering bindings now that retained value
  types drive conversion. Added an arrays lesson and updated the IR/spec contract.
- **Validation:** Strict CLI execution full suite: **233 library + 40 CLI = 273
  passed**, zero failures/skips (`target/v1-array-widening-tests.log`). Formatting,
  strict all-target/all-feature Clippy, doc stage (0 tests) and diff checks passed.
- **Pending:** Requested explicit clarification whether “NEXT” approves ADR 0012
  option A. Its compound copy/equality decision remains unimplemented pending
  that answer; numeric widening implements existing compatibility rules only.
  Full application/platform publication gates remain open.

## 2026-10-03: Full-platform v1 scope and retained lowering type facts

- **Release contract:** User explicitly selected full application platform and
  third-party library publishing before version 1.0. Corrected compiler-only
  release claims in README/changelog/draft notes and added V1_DELIVERY_GATES.md.
  Cargo's existing 1.0.0 target version is unchanged. No publication occurred.
- **Confirmed bug/fix:** A new six-case regression reproduced lost Float
  widening through record fields, factory returns, mixed arrays, copied arrays
  and indexing: expected `1.5`/`2.5`, obtained `1`/`2`. Lowering now retains
  alias-resolved field/return/array type facts through local scopes, rather than
  relying only on a set of Float-returning function names. Both execution engines
  pass the regression. Qualified imported factory/field inference is also covered.
- **Verification:** Full strict-execution Windows suite passed **231 library +
  40 CLI = 271 tests**, no failures/skips (`target/v1-type-facts-tests.log`).
  Strict all-target/all-feature Clippy passed. Package regression was extended
  after that run and verified separately; formatting and doc checks also passed.
- **New blocking decision:** An isolated copy/mutation probe printed `1` in the
  interpreter and `2` in JavaScript. ADR 0012 proposes independent compound
  values and nominal record/content equality; explicit approval requested under
  AGENTS.md's memory/type-semantics rule. This was not silently resolved by
  choosing either engine. Ordinary whole-array numeric conversions and broader
  retained semantic typing still require further correctness work.
- **Next:** Implement approved compound semantics with differential regressions,
  then module-local type resolution and the structured M12/application/library
  milestones. Version one is not ready for publication; hosted platform/MSRV,
  native tests, application runtime and registry workflows remain incomplete.

## 2026-10-03: Implemented approved exported-record package interfaces

- **Decision:** User approved ADR 0011 option A: explicit `export struct`, qualified
  consumer types/construction, private unexported records and nominal identity.
  Public fields are readable/constructible now; future explicit field visibility
  and encapsulation remain possible under a compatibility design.
- **Implementation:** Parser/AST retain record exports. Package compilation checks
  dependencies before consumers and transports canonical record identities and
  resolved field metadata. Exported signatures and fields resolve in the library;
  E4116 rejects private-record leaks. Aliases and diamond paths preserve identity,
  while unrelated packages with identical record names/layouts remain distinct.
  Qualified constructors preserve Float field widening in IR and both engines.
- **Regression evidence:** A direct field-division test initially produced `1`
  instead of `1.5`; alias-aware field metadata fixed constructor widening.
  Added valid/invalid export syntax, qualified annotations/construction, field
  validation/access, private annotations/construction/leaks, distinct same-name
  records, cross-package mismatch, alias identity, nested transitive records,
  consumer name collisions and diagnostic file/range coverage. The CLI example
  now checks/runs/builds records and executes emitted JavaScript with Node.
- **Verification:** `SOVRA_REQUIRE_CLI_EXECUTION=1 cargo test --locked --all-targets
  -- --nocapture`: **230 library + 40 CLI = 270 passed**, zero failures or skips;
  evidence `target/exported-records-tests.log`. Strict all-target/all-feature
  Clippy, formatting, doc tests (0 cases) and diff whitespace checks passed.
- **Limits:** Local path packages only. Existing same-file type lookup still
  shares bare module type names and rejects duplicate local module type names.
  No exported aliases, field mutation, new recursive-type support, registry or
  publishing was added. M12 and production readiness remain incomplete.
- **Next:** Harden module-local named-type scope resolution incrementally, then
  continue the library/application roadmap without changing nominal semantics.

## 2026-10-03: Proposed exported-record interface decision

Before extending E4116 into usable package record interfaces, inspected accepted
ADR 0009/0010 and the struct parser/AST. Nominal identity is approved, but record
export syntax, qualified type spellings and consumer field visibility are not.
Created ADR 0011 with a concrete recommended `export struct` rule, qualified
constructor/annotation examples, private-type leakage checks and acceptance tests.
Alternative B permits opaque values accessed only through exported functions.
No syntax or runtime changes were made, and no tests were rerun for this
documentation-only proposal. Next: obtain the required design choice, then
implement the selected boundary while preserving the E4116 guard until verified.

## 2026-10-03: Resolve imported scalar aliases in their owning package

Two regressions reproduced unsound package signature handling: a consumer's
`Shared = Bool` alias changed a library's `Shared = Float` parameter, accepting
a Boolean that failed at runtime; unrelated records with the same name were
accepted across a package boundary. Imported function signature copies now resolve
scalar aliases using the declaring package's type environment before consumer
analysis and lowering. Valid scalar alias calls preserve Float inference and
widening without requiring consumer redeclarations.

Imported non-scalar signatures now fail explicitly with E4116 at the declaring
library annotation until package-qualified nominal identity is implemented.
This does not implement record import/export semantics. Original library bodies
continue through semantic checking; normalized signature copies do not execute.
Public compiler-stage APIs and standalone behavior remain intact.

Validation: both regressions failed before the fix. All 224 library and 40 CLI
tests passed on Windows (264 total), with mandatory CLI execution and no skips.
The scalar alias regression compares interpreter and emitted JavaScript output.
Strict Clippy, formatting and the doc-test stage passed (zero doc tests).
Evidence: target/imported-signature-tests.log. Next: carry package-qualified
record identities and field metadata through imported signatures before enabling
record-valued package interfaces; keep E4116 until that path is complete.

## 2026-10-03: Preserve imported Float return types during lowering

Re-audited the updated workspace before continuing package work; preserved its
records, arrays, control-flow and local numeric-widening changes. Current baseline
was 221 library and 40 CLI tests. A new package regression reproduced a gap:
an inferred mutable local initialized from an imported Float-returning function
lost Float lowering metadata, so reassignment to 3 followed by division by 2
produced 1 instead of 1.5. Inferred array construction had the same problem.

Package lowering now receives imported function signatures and includes explicit
Float returns in its inference context. Standalone `ir::lower` retains its public
API and behavior. The regression verifies both interpreter and generated JavaScript
outputs. Grouped shared statement type information into an internal context to
resolve the strict-Clippy argument-count failure in the updated lowering code.
Corrected stale package status documents; user-defined exported type identity,
lockfiles and publication remain unfinished.

Validation after all code changes: 222 library and 40 CLI tests passed on Windows
(262 total), mandatory CLI execution enabled, no skips. Formatting, strict Clippy
and diff whitespace checks passed; the doc-test stage passed with zero tests.
Evidence: target/package-float-final-tests.log. Next: resolve exported signatures
in their defining package's type environment, without capturing consumer aliases,
before claiming user-defined types cross package boundaries safely.

## 2026-10-03: Numeric widening on reassignment and mixed arrays

Fixed a runtime mismatch where assigning an Int to an existing Float binding
skipped conversion and changed subsequent arithmetic to integer semantics.
Lowering now retains Float-ness for local bindings and inserts widening for
assignments, including aliases and values inferred as Float. Inferred array
elements are unified to Float when any element is Float, including when the
first item is Int. Lowering widens Int elements during construction and Int
values on indexed writes. Float-typed record fields now widen compatible Int
values during record construction. Float-returning calls and indexed reads
participate in local Float inference.

Regressions initially reproduced integer division after Float reassignment and
an Int-first mixed array retaining an Int element type. Coverage now compares
interpreter and JavaScript results for local and indexed writes, mixed array
construction, Float record fields and type rejection when assigning Float to an
Int binding.

## 2026-10-03: Float widening for reassignment and arrays

Fixed another gap between accepted Float compatibility and generated runtime
values. Assignments to Float-typed mutable locals now widen Int values before
storage, including aliases and inferred Float locals. Inferred Float arrays now
widen compatible Int elements during construction and widen Int values written
through indexed assignment. Float-returning calls are tracked so inference
continues across call-to-local and array-element boundaries.

The regression verifies fractional division after annotated, inferred and
indexed reassignment, plus an Int element in an array inferred as Float. The
interpreter and JavaScript backend produce identical results. Formatting,
all 220 library tests and all 40 CLI tests passed on Windows (260 total).
Strict Clippy, `cargo fmt --check` and `git diff --check` passed.

## 2026-10-02: Lexical shadowing across control-flow blocks

Fixed a semantic/runtime mismatch: semantic analysis already checked `if` and
`while` bodies in child scopes, but lowering stored shadowed declarations under
the outer variable's runtime name. The interpreter and JavaScript backend
therefore overwrote the outer value. Lowering now assigns an internal runtime
name when a declaration shadows a visible local and lowers each branch/loop
body with an independent name environment. Assignments to outer mutable names
continue to target the outer binding.

The regression initially reproduced the incorrect outer-value overwrite and
now checks branch and loop shadowing in both execution engines. Formatting,
all 219 library tests and all 40 CLI tests passed on Windows (259 total).
Strict Clippy, `cargo fmt --check` and `git diff --check` passed.

## 2026-10-02: Float alias widening at typed boundaries

Fixed a mismatch where semantic analysis accepted Int-to-Float widening through
aliases, but IR lowering emitted conversion instructions only for annotations
spelled exactly `Float`. Lowering now resolves declared alias chains and emits
the same conversions for Float aliases on parameters, local bindings and return
values. Alias cycles and unresolved aliases remain semantic errors and are not
treated as valid widening types.

A regression reproduces loss of precision with an integer beyond binary64's
exact-integer range, checking chained aliases across all three boundaries in
the interpreter and generated JavaScript backend. Formatting, all 218 library
tests and all 40 CLI tests passed on Windows (258 total). Strict Clippy,
`cargo fmt --check` and `git diff --check` passed.

## 2026-10-02: Short-circuit Boolean expressions and backend branches

Added Boolean-only `&&` and `||` with conventional precedence and branch-based
short-circuit lowering. The JavaScript backend now dispatches IR instructions
by program counter, supporting conditional and loop branches instead of
rejecting them. Interpreter and JavaScript both permit a jump to the
instruction-count boundary, and validate conditional jump targets consistently
whether or not the branch is taken.

Regressions cover precedence, invalid operand types, skipped side effects,
logical combinations, `if`/`while` execution and interpreter/Node output parity.
Formatting passed; all 217 library tests and 40 CLI tests passed on Windows
(257 total), including the Node-backed parity checks. Strict Clippy and
`git diff --check` passed.

## 2026-10-02: Branch-aware non-Unit return completeness

Replaced the prior “contains any return” check with definite-return analysis
over statement sequences. An `if` guarantees a return only when it has an `else`
and both blocks guarantee a return; nested conditionals compose. Loops are
conservatively not treated as guaranteed to execute. Non-Unit functions that
can fall through now report the existing E3013, while Unit behavior and
diagnostic codes remain unchanged.

Regressions cover missing `else`, a non-returning alternate branch, return-only
loops, complete two-branch conditionals, nested branches and `else if`.
Formatting, strict Clippy, all 213 library tests and all 40 CLI tests passed on
Windows; `git diff --check` passed.

## 2026-10-02: Executable record construction and field access

Completed the previously parsed/type-checked-only record path. Lowering now
emits explicit record construction and field-load instructions. The interpreter
stores nominal record values with construction-order fields; the JavaScript
backend emits equivalent records, validates malformed public-IR field accesses,
and renders nested records and arrays consistently with interpreter output.
Records pass through locals, aliases, function parameters and return values.
Field mutation remains unsupported. The added runtime representation exposed
host-stack exhaustion at the documented 256-call limit, so interpreter calls
now use an explicit call-frame stack rather than Rust recursion; language-level
call-depth errors remain unchanged.

Added an executable records example, a course lesson, and tests spanning nested
records, aliases, function boundaries, interpreter/Node output parity, and CLI
check/run/IR/JS commands. Formatting, strict Clippy, all 213 library tests and
all 40 CLI tests passed on Windows. The Node-backed tests executed successfully.

## 2026-10-02: Mutable array write-back and index parity

Hardened the in-progress array/mutability slice. Indexed assignment now requires
a directly named mutable binding; immutable arrays report E3011 and nested
indexed writes report E3003 instead of producing lossy lowering. Lowering stores
the updated array back into its binding and consumes the store result, preventing
stale reads and operand-stack residue. The JavaScript backend now validates
array/index kinds and bounds consistently with the interpreter for both reads
and writes.

Added semantic and execution regressions for successful mutation, immutable
assignment, nested-write rejection, element type mismatch, backend parity and
negative indices. A Unit-returning helper regression also confirms assignment
does not leave a value on the operand stack. Formatting, strict Clippy, all 211
library tests and all 39 CLI tests passed on Windows; the backend parity test
executed with Node.js. Next: continue checking the type information lost between
semantic analysis and lowering before expanding collection behavior.

## 2026-09-28: Accepted local package graph foundation

User approved continuation after the ADR 0010 A–C approval request. Recorded the
decision and implemented `project::packages::resolve`: reuses strict manifest
parsing with explicit dependency sections, canonical package/entry identities,
manifest-relative dependency paths, ordered aliases, iterative traversal,
diamond deduplication, cycle-chain diagnostics and a 1024-node bound. Manifest
and entry symlink escapes are rejected after canonicalization. Equal package
display names do not alias. No package scripts or source programs execute.

Ordinary manifest checking retains its unsupported-dependency error until linking
is implemented. The new Rust API validates metadata/graphs only; its success is
not source validation or usable package consumption. No fake CLI was introduced.

Validation: full Windows suite passed 192 library + 37 CLI tests (229), mandatory
CLI execution and no skips. An additional API-boundary regression passed afterward
(230 distinct Windows tests). Formatting and strict Clippy passed. A Unix-only
symlink escape regression was added but was not executed on this Windows host.
Evidence: target/local-packages-tests.log. Next: implement explicit executable
imports and package-aware semantic linking, preserving source provenance and
private visibility, then connect project check/run/build and a two-package example.

## 2026-09-28: Library ecosystem audit and executable module composition

Read the supplied ecosystem mission and audited source/compiler/runtime/CLI,
manifest/import handling, std registry, tests and examples before modifications.
Confirmed a current 224-test Windows baseline, superseding the supplied historical
158 count. Preserved all existing M12 work. Recorded IMPLEMENTED/PARTIAL/
EXPERIMENTAL/STUB/PLANNED boundaries in LIBRARY_ECOSYSTEM_ASSESSMENT.md and the
dependency chain/functional acceptance for real networking in
REAL_HTTP_APPLICATION_PLAN.md. No architecture rewrite or empty std-module split
was justified by the four-function registry.

Added examples/library-foundations/main.svr and a course lesson using only current
mod/export syntax, private helpers, arithmetic and string composition. Its new
CLI regression validates check, run, IR build, JS emission and actual Node output.
No language feature, builtin, package manager command or dependency capability
was added. Proposed ADR 0010 defines local manifest sections, explicit executable
imports, isolation, graph-cycle policy and end-to-end acceptance for approval.

Final validation: 188 library + 37 CLI tests passed (225 total), no skips, with
mandatory CLI execution. Formatting, strict Clippy and doc-test stage passed.
Logs: target/library-audit-baseline.log and target/library-foundations-tests.log.
Next: obtain the explicitly required approval of ADR 0010 before implementing
compatibility-affecting executable package imports. Package consumption and M12
are not complete; creating these files does not satisfy either milestone.

## 2026-09-28: Multiline application parameter lists

The project scanner now collects balanced parameter-list continuations for
functions, tasks, pages, views and service operations. Annotation enforcement
works across comments, blank lines and CRLF input. Service metadata spans cover
the collected signature; diagnostics still identify its opening line. Recovery
stops before a new declaration or block boundary instead of consuming unrelated
code. Nested function-like annotations retain their parameter grouping.

The multiline regression failed before implementation. Full Windows validation:
187 library and 36 CLI tests passed with mandatory CLI execution. An additional
focused recovery regression passed afterward (224 distinct tests total).
General multiline return annotations, nested application declarations and named
type resolution remain unfinished. M12 remains partial.

## 2026-09-28: Extend application parameter annotation enforcement

Recognized top-level function/task/page/view parameter lists now require explicit
annotations under ADR 0009, reporting E4097. Malformed signatures report E4098.
Scheduled task bindings and page routes are excluded from signature recognition.
Structured function/task/service inspection independently rejects untyped
parameters, so direct Rust API callers cannot bypass that rule.

Both new regressions failed before implementation. Full Windows validation passed
186 library and 36 CLI tests (222 total) with mandatory CLI execution. Strict
Clippy passed. The project scanner remains single-line and partial; this slice
does not establish named-type resolution or complete nested-declaration checking.
Next: structured application declaration parsing and accepted nominal type
resolution. M12 is not complete.

## 2026-09-28: ADR 0009 accepted; service parameter annotations required

The user explicitly approved all six M12 checker-contract decisions. Recorded
acceptance without claiming completion. First implementation slice: project
service operations now require parameter annotations, reporting E4097 at the
original signature line. The structural parser still preserves omitted text for
diagnostics; annotation name resolution remains unfinished. Updated the metadata
fixture with an explicit Int parameter and added Rust/CLI diagnostic regressions.

The Rust regression failed before the fix. Full Windows suite: 184 library and
35 existing CLI tests passed; the newly added CLI regression then passed separately
(220 distinct passing tests). Mandatory CLI execution was enabled. Formatting
and strict Clippy passed. Next: extend annotation enforcement to remaining
application declarations and implement accepted nominal type/name resolution.
Default project checking is still partial until the broader ADR 0009 work lands.

## 2026-09-28: Reject hidden declarations after inline operation bodies

Reproduced the contract scanner silently accepting `fn send() {} fn hidden() {}`
while indexing only `send`. Completed same-line bodies now reject trailing text
with existing E4027 and original file/line provenance. The boundary scan respects
nested braces, quoted strings and escapes, and permits the service closing brace.
Multiline bodies remain tracked by the existing scanner; this is not statement
validation or full application parsing.

The focused regression failed before the fix. Windows validation passed 183
library and 35 CLI tests (218 total), with mandatory CLI execution enabled.
Formatting and strict Clippy passed. Evidence: `target/service-tail-tests.log`.
Next: replace remaining line-oriented service declaration boundaries incrementally;
application signature-type resolution remains unfinished and M12 remains partial.

## 2026-09-28: Service body metadata across line breaks

Fixed service-operation metadata reporting `has_body: false` when an implemented
operation opens its body on the next substantive line. Blank lines and comments
are ignored; a semicolon, another declaration or the service closing brace ends
the pending signature association. Signature source locations remain unchanged.
This aligns scanner metadata with the existing structured inspector and introduces
no new syntax or type rules. The CLI metadata fixture now exercises this layout.

The focused regression failed before the fix. Full Windows validation passed
181 library and 35 CLI tests (216 total), with mandatory CLI execution enabled.
M12 remains partial; application signature types and wider declaration parsing
remain unfinished.

## 2026-09-28: Service-body project and CLI integration coverage

Verified imported service receivers inside operation bodies through the project
checker. Regression coverage checks E4094 argument counts, E4093 missing operations,
original UTF-8/CRLF byte spans and file provenance, qualified operation names,
parameter shadowing and independent sibling scopes. CLI JSON assertions verify
`mail.send` as the enclosing operation, service receiver classification, and exact
agreement between call and diagnostic locations. No implementation changes were
needed for these cases.

Validation: 180 library and 35 CLI tests passed on Windows (215 total), with
mandatory CLI execution enabled. Formatting and strict Clippy passed. Evidence:
`target/service-integration-tests.log`. M12 remains partial; next work is extending
structured application parsing and resolving application signature types under
documented language decisions.

## 2026-09-28: Explicit service implementation scopes and iterative AST ownership

Implemented user-approved ADR 0008. Structured project inspection parses service
operation bodies with file/import service visibility, independent parameter/local
scopes and explicit receivers. Records use qualified operation names. Unsupported
syntax discards the whole file; declaration-only operations emit no body record.
The CLI regression now reports E4093 for a missing operation inside a service body,
instead of rejecting that body with E4096. Source execution is unchanged.

Expression cloning now traverses iteratively. Added explicit consuming
Expression::drop_iterative and Program::drop_iterative APIs with a 10,000-node
regression. Ordinary Rust drop, Debug and equality still have recursive behavior;
this does not establish arbitrary-input resource safety for every public API.

Validation: 179 library and 35 CLI tests passed on Windows (214 total), with
SOVRA_REQUIRE_CLI_EXECUTION=1 and no skips. Formatting, strict all-target/all-feature
Clippy and diff whitespace checks passed. Service-scope regression failed before
the implementation and passed afterward. Evidence: target/service-scopes-tests.log.

M12 remains partial. Next: broaden service-body project integration coverage for
imported receivers, qualified JSON metadata and arity errors, then continue the
structured application parser and named-type resolution under documented decisions.


## 2026-09-28 — Iterative expression lowering

- **Changed:** Lowering now uses explicit visit/emit work items instead of
  recursive expression calls. Argument and binary operand order, widening sites,
  output IR and public stage signatures are preserved. Validated input is still
  required; direct lowering is not a semantic validator.
- **Coverage:** Added exact instruction-order/execution coverage before refactoring.
  A caller-built 10,000-node expression lowers and runs without native expression
  recursion; the test dismantles the owned AST iteratively to isolate lowering
  from the unresolved public AST recursive-drop behavior.
- **Additional fix:** Full testing exposed an existing parallel fixture-name
  collision (Windows AlreadyExists). Project test directories now include an
  atomic per-process sequence in addition to timestamp/PID.
- **Validation:** After the fixture fix, 176 library and 35 CLI tests passed
  (211 total), strict CLI execution enabled, no skips. Formatting and strict
  Clippy passed. The initial full run's fixture failure is not counted as a pass.
- **Next:** Ownership/drop and clone safety of caller-built ASTs remains open;
  runtime budgets and hosted platform verification also remain production gates.

## 2026-09-28 — Public AST semantic-depth preflight

- **Fixed:** Caller-built ASTs bypassed parser depth guards and could enter
  recursive semantic analysis/cloning. A borrow-only iterative preflight now
  returns E3018 for expression depth above 128, including module/private bodies.
  Checked `lower_program` inherits this validation; public signatures are intact.
- **Coverage:** Reproduced acceptance of a manually built depth-129 tree before
  the fix. Tests cover depth 128/129, module/top-level bodies, initializers,
  returns, expression statements, nested arguments and nested callees, including
  retained diagnostic spans and rejection by checked lowering.
- **Validation:** 174 library and 35 CLI tests passed (209 total), with strict CLI
  execution and no skips. Formatting and strict Clippy passed.
- **Limits/next:** Borrowed validation cannot protect caller-owned recursive
  drop/clone, and direct `lower` accepts publicly fabricated TypedProgram values.
  Its validated-input precondition is now explicit. Audit ownership/checked API
  boundaries before claiming complete AST resource safety.

## 2026-09-28 — Approved structural-depth limit implemented

- **Decision:** User explicitly approved the 128-depth policy in ADR 0007.
  Source and application parsers now enforce it; no unrestricted override added.
- **Implementation:** Shared iterative token preflight guards parentheses and
  application block nesting. Iterative child-depth inspection rejects binary,
  call and application member nodes before over-deep tree construction. Source
  parsing returns one E2007 at the relevant token without recovery cascades;
  application failures reject all file call records and surface CLI E4096.
- **Coverage:** Exact boundary/next-depth grouping, calls, binary chains, blocks,
  member chains, mixed block/group nesting and shallow 1024-argument lists.
  Boundary programs survive semantic analysis/lowering/interpreter execution.
  Subprocess tests cover check/run/IR/JS rejection, UTF-8/CRLF JSON locations and
  explicit incomplete application coverage for the former crash inputs.
- **Validation:** 172 library and 35 CLI tests passed (207 total), strict CLI
  execution enabled, no skips. Formatting and strict Clippy passed. Debug and
  optimized Windows GNU builds each passed 42 depth-probe assertions, including
  normal E2007 exits for all previously crashing depth-2048 shapes.
- **CI:** Platform CI now runs the asserting debug probe; candidate verification
  runs the optimized probe. Hosted platform/MSRV execution remains unverified.
- **Limits/next:** Source guards do not protect arbitrarily constructed public
  AST ownership/drop, cap total memory or impose runtime work/output budgets.
  Continue those audits independently; this is not full production readiness.

## 2026-09-27 — Measured compiler depth and proposed policy

- **Added:** Reusable Node subprocess depth probe with timeouts, bounded captured
  output and per-profile JSON results under target. It is diagnostic tooling,
  not a passing safety test; its exit success means only that probing completed.
- **Measured:** 60 check/build invocations across grouping, calls and binary
  chains at depths 32/128/256/512/2048. Windows GNU debug builds passed 24 and
  crashed with stack overflow on all six depth-2048 cases. Optimized builds
  passed all 30. rustc 1.98.1; no platform-independent crash threshold inferred.
- **Proposal:** ADR 0007 recommends a fixed structural-depth bound of 128 with
  diagnostics before recursive parsing or construction of over-deep expression
  trees, including application inspection. Public AST/resource budgets remain
  separate work. No cap is implemented pending compatibility-policy approval.
- **Validation:** Debug/release builds and probe JavaScript syntax checks passed.
  No compiler source changed; the existing 203-test baseline was not rerun.
- **Next:** Obtain the ADR decision required by AGENTS.md, then implement guards
  with boundary, subprocess crash-regression and platform validation.

## 2026-09-27 — Resource audit and iterative source discovery

- **Changed:** Replaced recursive directory descent with a pending-directory
  worklist, retaining sorted source results, extension filtering and E4006 I/O
  errors. This removes native call-stack growth with directory depth; it does
  not impose a directory, memory or program-size budget.
- **Audit:** RESOURCE_BOUNDS.md records parser/grouping/call recursion, deep
  binary AST traversal/cloning/drop, application scopes, and unbounded runtime
  output/data. Existing call-depth enforcement is not a total execution budget.
  Compatibility-affecting caps require a concrete policy and review first.
- **Coverage:** Nested discovery through 48 levels, sibling/root source files,
  non-source filtering, sorted paths and missing-directory diagnostics.
- **Validation:** 169 library and 34 CLI tests passed (203 total), with strict CLI
  execution and no skips. Formatting and strict Clippy passed.
- **Next:** Measure compiler structural-depth failure modes in isolated processes
  and propose a tested limit/API policy covering downstream AST work as well as
  parsing. Production resource guarantees remain unfinished.

## 2026-09-27 — Escaped IR function lookup errors

- **Fixed:** Missing-function errors interpolated raw IR names into JavaScript
  strings. Quotes/newlines could break emitted syntax instead of reporting the
  interpreter's error. Messages now use the existing shared string escaper.
- **Coverage:** Reproduced the malformed JavaScript before the fix. Tests cover
  missing and successful calls for quote/backslash/newline/Unicode names, empty
  names and __proto__/constructor keys. Error assertions compare exact message
  values before Node's Windows newline formatting; wrappers preserve strict mode.
- **Validation:** 168 library and 34 CLI tests passed (202 total), strict CLI
  execution enabled, no skips. All 21 backend tests passed again after tightening
  the error assertion wrapper. Formatting and strict Clippy passed.
- **Next:** Parser/runtime resource-bound audit. Current IR hardening does not
  establish complete verification or production readiness.

## 2026-09-27 — Collision-free IR parameter encoding

- **Fixed:** Distinct public-IR names such as `a-b` and `a_b`, or two Unicode
  characters, were sanitized into identical JavaScript parameters and caused a
  strict-mode syntax error. Emission now uses unique positional parameter names
  and retains original names as escaped runtime-environment keys.
- **Coverage:** Reproduced the duplicate-parameter JavaScript failure before the
  fix. Differential execution checks argument identity for punctuation, Unicode,
  empty names, arguments, __proto__, constructor and quote/backslash/newline keys.
- **Validation:** 167 library and 34 CLI tests passed (201 total), with strict CLI
  execution enabled and no skips. Formatting and strict Clippy passed.
- **Next:** Complete the remaining IR name/error-escaping audit, then address
  compiler/runtime resource bounds. Source identifiers and public APIs are
  unchanged; production readiness remains incomplete.

## 2026-09-27 — Shared IR declaration validation

- **Fixed:** Public IR duplicate function names previously selected the first
  declaration in the interpreter and the last in JavaScript. A shared validator
  now rejects duplicate functions and duplicate parameter names before execution,
  including unused declarations. JavaScript emits an equivalent throwing program
  so its existing String-returning API is preserved.
- **Coverage:** Reproduced duplicate acceptance before implementation. Differential
  regressions cover duplicate main, ordinary and qualified helper names, and
  duplicate parameters in an unused function.
- **Validation:** 166 library and 34 CLI tests passed (200 total), with strict CLI
  execution enabled and no skips. Formatting and strict Clippy passed.
- **Next:** Audit backend parameter-name encoding and remaining IR identity
  invariants, then compiler/runtime resource bounds. No source syntax changed;
  general IR validation and production readiness remain incomplete.

## 2026-09-27 — Binary runtime-kind parity

- **Fixed:** JavaScript binary helpers now reject incompatible runtime kinds
  instead of silently coercing values or emitting host-specific TypeErrors.
  Preserve mixed numeric widening, string concatenation/scalar ordering,
  equality and interpreter division-by-zero precedence. Source semantics and
  public stage signatures remain unchanged.
- **Coverage:** Reproduced the Int-plus-Bool error mismatch before the fix.
  A differential matrix checks 396 combinations: eleven operators (including an
  unknown one) over Int, Float, Bool, String, Unit and zero operands. Both values
  and failure messages are compared against interpreter execution.
- **Validation:** 165 library and 34 CLI tests passed (199 total), strict CLI
  execution enabled, no skips. Formatting and strict Clippy passed.
- **Next:** Audit malformed-IR function identity and resource bounds; Float
  formatting/non-finite policy still needs an explicit decision. These tests
  improve supported-subset correctness, not full production readiness.

## 2026-09-27 — Numeric IR literal parity

- **Fixed:** JavaScript numeric conversion accepted malformed IR spellings that
  Rust rejected. Emission now parses i64/f64 with Rust, normalizes valid values,
  and emits execution-time errors matching the interpreter for invalid literals.
  Stage APIs and source literal syntax are unchanged.
- **Coverage:** Reproduced malformed empty-literal acceptance. Differential cases
  cover whitespace, hexadecimal/underscore spellings, Int bounds and malformed
  floats. Node value assertions cover signed/zero-padded integers, i64 minimum,
  infinity, NaN, signed zero and exponent spellings accepted by public IR.
- **Validation:** 164 library and 34 CLI tests passed (198 total), with strict CLI
  execution and no skips. Formatting and strict Clippy passed.
- **Next:** Audit binary runtime-kind compatibility and error parity. Non-finite
  Float/output policy and broader resource limits remain open production gates.

## 2026-09-27 — Builtin validation and call allocation guard

- **Fixed:** Generated JavaScript checks builtin arity using the Rust registry
  and rejects non-String `std::len` inputs instead of JavaScript coercion. The
  interpreter validates available stack arguments before allocating storage.
- **Reproduced:** Missing builtin arguments silently succeeded in JavaScript;
  `usize::MAX` IR argument count caused a capacity-overflow panic in Rust.
- **Coverage:** Differential cases cover zero/excess arguments for all builtins
  and the print alias, plus Int/Float/Bool/Unit inputs to len. An interpreter
  regression verifies extreme counts return stack-underflow errors without panic.
- **Validation:** 162 library and 34 CLI tests passed (196 total), with strict CLI
  execution and no skips; formatting and strict Clippy passed.
- **Next:** Malformed literal/runtime-kind parity and broader resource bounds.
  These fixes do not establish full IR validation or production readiness.

## 2026-09-27 — IR stack and name parity

- **Fixed:** Generated JavaScript now rejects empty/insufficient stacks for store,
  widening, binary, call and pop instructions with interpreter-compatible errors.
  Missing-name loads check property existence rather than silently yielding Unit.
  Empty-stack returns retain their existing Unit behavior.
- **Coverage:** Reproduced silent empty-stack pop success before the fix. A
  differential regression covers each affected instruction, a binary expression
  with only one operand, and a missing variable.
- **Validation:** 160 library and 34 CLI tests passed (194 total), with strict CLI
  execution enabled and no skips. Formatting and strict Clippy passed.
- **Next:** Audit builtin arity/runtime kinds, malformed IR literals and allocation
  bounds. These corrections do not constitute full public-IR validation or
  production readiness; source syntax and stage APIs are unchanged.

## 2026-09-27 — Production audit and first runtime invariant fix

- **Direction:** User requires production readiness; superseded the preview
  publication plan in README, release checklist/notes and current planning docs.
  Existing verification workflows remain useful but confer no release readiness.
- **Audit:** Added PRODUCTION_READINESS.md with code evidence, blockers,
  dependency-ordered work and testable acceptance gates. Preserve milestone
  numbering, implementation and truthful Partial/Experimental labels. Application
  typing/runtime and independent library publication are still substantial gaps.
- **Fixed:** JavaScript emitted from public IR silently accepted missing/excess
  user-function arguments that the interpreter rejected. Reproduced the missing
  argument bug before adding entry arity guards; no source-language semantics or
  public compiler APIs changed. Guards run before frame entry/parameter setup.
- **Validation:** 159 library and 34 CLI tests passed (193 total), with strict CLI
  execution enabled and no skips. Regression tests cover missing/excess arguments
  and invalid main arity, comparing interpreter and Node failure messages.
  Formatting, strict Clippy and the doc-test stage passed (zero doc tests).
- **Next:** Continue public-IR/backend invariant parity (stack underflow, missing
  names and runtime kinds), then resource-bound analysis and structured project
  typing. Float policies, service-body scope, new syntax and package contracts
  require concrete design review before semantic changes. Production is not yet
  achieved; hosted platform/MSRV evidence remains outstanding.

## 2026-09-27 — Candidate acceptance gates

- **Changed:** Candidate platform jobs now depend on a Rust 1.74 compile job.
  After archive creation, each job extracts a fresh copy and checks exact version,
  hello-world output, source JSON schema/success/diagnostics and generated
  JavaScript execution before uploading artifacts.
- **Validation:** Rehearsed the expanded packaging block on Windows against the
  optimized binary, with all assertions passing. The local rehearsal substitutes
  the Windows selector for PowerShell 5 and labels metadata local-dirty-rehearsal.
  No Rust source changed; the prior 192-test baseline remains applicable.
- **Unverified:** Hosted matrix/MSRV runs still require committed workflows.
  Remote repository/release-history reads were unsuccessful, so version and URL
  decisions remain open; no remote publication state has been inferred.
- **Next:** Confirm public repository and release version, then commit/review the
  candidate and execute hosted checks. No tag, push or publication was performed.

## 2026-09-27 — First developer-preview release preparation

- **Scope:** User selected a developer preview. Added explicit README limitations,
  draft standalone release notes and a release checklist; no publication occurred.
- **Infrastructure:** CI now configures Linux/Windows/macOS and Rust 1.74 checks.
  Strict CLI execution is enabled in CI so OS-blocked tests cannot silently pass.
  A manual candidate workflow runs checks, verifies Cargo packaging and creates
  target-named archives, checksums and commit metadata without publishing.
- **Packaging:** Explicit Cargo include rules exclude the website and repository
  administration while retaining source, fixtures, examples, docs and licenses.
- **Validation:** 192 tests ran successfully with strict CLI execution enabled;
  formatting, strict Clippy and the doc-test stage passed. Offline Cargo package
  verification passed with --allow-dirty. Windows GNU optimized build passed;
  packaging script syntax/rehearsal, archive extraction and hello-world smoke
  execution passed. PowerShell 5 used a local Windows-selector substitution;
  the hosted PowerShell 7 workflow itself remains unexecuted.
- **Release blockers:** Hosted platform/MSRV jobs and clean-commit candidates
  remain unverified. Cargo repository metadata differs from origin. Version
  0.1.0 conflicts with the existing dated changelog history; confirm identity,
  remote release history and distribution/version before tagging or publishing.
- **Next:** Resolve those release decisions, review accumulated changes and run
  the candidate workflow on the exact committed release revision.

## 2026-09-27 — Complete complex-receiver ranges

- **Fixed:** Member references on call results, literals and grouped/binary
  expressions previously lost the receiver and could point only at the member
  name. Every inspected expression now retains its span, so chained member
  locations cover the full receiver through the operation name. Grouping is
  retained without changing receiver resolution.
- **Coverage:** Reproduced the truncated range before the fix. Regression cases
  cover chained/grouped calls, binary and literal receivers, grouped service
  names, CRLF and a preceding Unicode comment.
- **Validation:** 158 library and 34 CLI tests passed (192 total), without skips.
  Formatting and strict all-target Clippy passed.
- **Next:** Continue structural inspection hardening; full application semantics,
  service implementation scopes and package loading remain unfinished.

## 2026-09-27 — Machine-readable member-call references

- **Added:** Opt-in JSON reports expose additive `member_calls` records with
  enclosing function/task, operation, argument count, source range and receiver
  classification. Service identities and ambiguous candidates are retained;
  local/unresolved receivers have no service candidates.
- **Coverage:** A failing JSON regression preceded implementation. Tests cover
  all four receiver classifications, candidate lists, argument count, locations
  and exclusion of unsupported files. CLI tests verify records only reference
  successfully inspected files, including when contract checks fail.
- **Validation:** 157 library and 34 CLI tests passed (191 total), without skips.
  Formatting and strict all-target Clippy passed.
- **Limitations/next:** Records describe inspection rather than application type
  checking or execution. Canonical local module paths are temporary identities;
  package-qualified identities remain planned. Continue improving supported
  inspection coverage before implementing service bodies or package loading.

## 2026-09-27 — Qualified calls in application inspection

- **Added:** Recognition of the existing two-component `module::function` syntax
  in experimental application expressions. Nested call arguments are traversed;
  namespace names cannot accidentally resolve as dotted service receivers.
  Member ranges retain the qualified receiver's full source range.
- **Coverage:** Reproduced rejection of qualified calls before implementation.
  Added namespace/service distinction, nested argument and malformed-path tests.
  CLI success and error fixtures now wrap service calls in `std::println`.
- **Validation:** 156 library and 34 CLI tests passed (190 total), without skips.
  Formatting and strict all-target Clippy passed.
- **Limitations/next:** Recognition does not resolve namespace exports or validate
  qualified call types. Longer paths remain unsupported, matching the executable
  parser's current limit. Continue supported-expression coverage; service-body
  scope and application execution still require separate work.

## 2026-09-27 — Shared coverage diagnostics

- **Changed:** Moved incomplete-inspection E4096 generation from CLI orchestration
  into the experimental Rust service checker. Library and CLI consumers now get
  the same contract and coverage diagnostics. Existing CLI output/order is retained.
- **Coverage:** Updated the mixed complete/partial project regression and observed
  its failure before the fix. It verifies the coverage code, affected filename
  and absence of a fabricated source location alongside precise contract errors.
- **Validation:** 154 library and 34 CLI tests passed (188 total), without skips;
  formatting and strict all-target Clippy passed.
- **Next:** Define service implementation scope rules before extending inspection
  to those bodies. Application type checking and package publication remain planned.

## 2026-09-27 — Honest service implementation coverage

- **Fixed:** File inspection previously skipped nonempty service implementation
  bodies while reporting complete coverage. These files now return an explicit
  unsupported-body reason; opt-in CLI checking reports E4096 and exits 1.
  Service signatures and empty/comment-only bodies remain inspectable.
- **Coverage:** Reproduced the false success with a failing regression. Added
  cases for hidden calls, returns, nested blocks and comment-only bodies, plus
  a project fixture checked through human and JSON CLI output.
- **Validation:** All 154 library and 34 CLI tests passed (188 total), with no
  skips. Formatting and strict all-target Clippy passed.
- **Limitations/next:** Service implementation scope and call inspection remain
  unimplemented. Define those rules before claiming coverage of those bodies;
  ordinary project wiring checks retain their existing partial scope.

## 2026-09-27 — Binary expressions in application inspection

- **Changed:** Experimental service-call inspection accepts arithmetic and
  comparison expressions with the executable subset's precedence and left
  associativity. Calls in both operands and nested arguments are inspected;
  initializer calls still resolve before the new binding shadows a service.
- **Coverage:** Reproduced the previous arithmetic rejection with a failing
  regression, then added operand traversal, argument count, shadowing and
  malformed-expression coverage. The successful CLI fixture now contains calls
  inside a comparison and arithmetic inside a call argument.
- **Validation:** All 152 library and 34 CLI tests passed (186 total), without
  skips. Formatting and strict all-target Clippy passed.
- **Limitations:** Structural inspection only; no application execution or type
  checking. Logical operators, conditionals and closures remain unsupported and
  cause incomplete coverage rather than partial successful inspection.
- **Next:** Continue expanding structured application inspection; package loading
  and publishing remain planned separate work.

## 2026-09-27 — Typed locals in application inspection

- **Changed:** Optional local annotations are structurally consumed before the
  initializer. Balanced generic/tuple/function-like/array-like forms are accepted
  without resolving types. Malformed annotations reject the entire file inspection.
- **Scope:** Initializer receivers still resolve before the new local shadows the
  service name. Subsequent member calls on that binding are ordinary local calls.
- **Coverage:** Nested annotation forms, malformed delimiter/list syntax and
  initializer/shadowing order. Expanded the successful CLI service-metadata fixture
  with a typed local whose later member must not be validated as a service call.
- **Validation:** All 150 library and 34 CLI tests passed (184 total), without
  skips. CLI tests passed again after expanding the fixture. Formatting, strict
  all-target Clippy and diff checks passed.
- **Next:** Expand expression support while keeping scope and coverage guarantees;
  full application type checking and package loading remain separate work.

## 2026-09-27 — Opt-in CLI service-call checking

- **Added:** check --service-calls for project directories, with human/JSON output.
  Runs after successful manifest/import/wiring validation. Contract errors fail;
  incomplete file inspection adds E4096 and fails rather than claiming success.
- **Coverage reporting:** JSON service_coverage retains complete status and every
  file's inspected flag/reason. Completeness refers to supported syntax, not full
  application type checking. Ordinary check reports and execution are unchanged.
- **Tests added:** Complete success, operation/arity failures, partial Fielddesk,
  human summaries, JSON coverage and source/duplicate-option usage errors.
- **Validation:** 148 library tests passed. Updated the existing exact help-text
  assertion for the new flag, then all 34 CLI tests passed (182 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Next:** Expand supported application syntax incrementally while preserving
  explicit partial coverage. Third-party package resolution remains planned.

## 2026-09-27 — Resolved service-call contract checks

- **Added:** Opt-in check_service_calls API consumes successful project metadata
  and per-file structured inspection. E4093 reports unknown operations, E4094
  positional arity mismatches and E4095 ambiguous receivers at member-call spans.
  Only receivers resolved as services are compared with retained contracts.
- **Coverage:** Imported service calls with wrong/matching arity, missing members,
  parameter shadowing, unresolved receivers and discarded partial-file results.
- **Validation:** All 148 library and 32 CLI tests passed (180 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Boundary:** ServiceCheck retains file outcomes and diagnostics separately;
  empty diagnostics never establish full coverage. Ordinary CLI check is unchanged,
  application types remain unresolved, and unsupported files are not enforced.
- **Next:** Expose opt-in service checking with machine-readable coverage so CLI
  consumers cannot confuse a partial inspection with complete application checks.

## 2026-09-27 — Import-aware per-file application inspection

- **Connected:** inspect_project supplies same-file/direct-import service identities
  to structured function inspection. Retained service declarations now include
  empty contracts and source identity. Repeated imports remain idempotent and
  transitive imports do not implicitly expose service names.
- **Parsing:** Recognizes import declarations and balanced service blocks while
  inspecting supported function/task bodies. Each file returns records or an
  explicit error for unsupported syntax; project checking is not made stricter
  based on partial inspection. Operation existence/arity checks remain next.
- **Coverage:** Direct imports, same-file/empty services, parameter shadowing,
  transitive exclusion and partial-file outcomes. The identity adapter uses local
  canonical paths temporarily; future package identities remain a separate boundary.
- **Compatibility:** ProjectCheck literals now need service_declarations. No CLI
  JSON additions or source execution changes were made in this slice.
- **Validation:** Full suite passed 147 library and 32 CLI tests (179 total),
  without skips. The expanded transitive-import regression also passed separately.
  Formatting, strict all-target Clippy and diff checks passed.
- **Next:** Validate operation existence and positional arity only for receivers
  resolved as services, while retaining explicit partial coverage for other files.

## 2026-09-27 — Whole-file function/task receiver inspection

- **Added:** Experimental inspect_functions extracts top-level function/task
  signatures and bodies from lexer tokens, uses the structured signature parser
  for parameter bindings, and invokes scoped body inspection. It returns ordered
  FunctionCalls records with original source spans rather than substring offsets.
- **Coverage:** Parameter shadowing, independent function/task scopes, multiline
  signatures/comments, CRLF and UTF-8 locations, missing bodies, stray terminators
  and unsupported declarations/bodies. Unsupported files return no partial list.
- **Validation:** All 146 library and 32 CLI tests passed (178 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Boundary:** Visible service identities remain an explicit caller input;
  imports and other application top-level forms are not accepted by this API yet.
  Project checking and library/package resolution are unchanged.
- **Next:** Connect validated imports and per-file service declarations to the
  inspector, with explicit partial coverage for unsupported application constructs.

## Third-party library architecture requirement

- **Requirement:** User explicitly requires other developers to publish reusable
  libraries when Sovra reaches full capability. Recorded this as a planned
  ecosystem requirement rather than a claim about the current toolchain.
- **Design:** Added package/library requirements covering separate dependency,
  module and symbol resolution; package-qualified identities; declared dependency
  roots; public exports; local/Git/registry sources; lockfiles; offline builds;
  author/consumer workflows and publish/consume acceptance criteria.
- **Integration:** Updated architecture, roadmap, status and ADR 0006 so current
  root containment is explicitly local-package scope, not a permanent ban on
  third-party libraries. No manifest grammar, version policy, registry endpoint
  or publishing command was silently selected.
- **Validation:** Documentation-only change; checked diff formatting and local
  documentation links. No compiler behavior changed or test rerun required.
- **Next:** Continue application parser/import resolution while preserving the
  package-identity boundary; implement local library consumption before registry
  publication. Detailed package syntax/policies require a separate decision.

## Structured application-body receiver inspection

- **Added:** Experimental application::inspect_body parses a bounded body subset
  using the existing lexer, builds expression nodes and connects traversal to
  nested scopes. Member-call records contain operation, arity, receiver class and
  source range. Initializers are inspected before the local binding takes effect.
- **Coverage:** Parameter/local shadowing, nested block isolation, initializer
  calls, nested call arguments, comments/strings and unsupported/incomplete input.
- **Validation:** All 144 library and 32 CLI tests passed (176 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Boundary:** Callers supply one body, parameters and visible service identities.
  Unsupported syntax fails the whole inspection. Closures, typed locals, binary
  operators and general application statements remain unsupported. No project
  service-call enforcement or full Fielddesk parsing is claimed.
- **Next:** Add structured function/task extraction and import-service wiring,
  preserving explicit partial results until supported bodies can be checked.

## Experimental application receiver-scope resolver

- **Added:** Rust project::scope API with nested scopes, visibility-offset bindings
  and module-qualified service identities. Receiver lookup distinguishes Local,
  Service, Ambiguous and Unresolved. Locals/parameters/closure bindings shadow
  service names; repeated imports of one identity do not create ambiguity.
- **Coverage:** Declaration-order visibility, nested inheritance, sibling/parent
  isolation, duplicate imports, conflicting service identities and unknown names.
- **Validation:** All 141 library and 32 CLI tests passed (173 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Status:** Experimental foundation, not completed application parsing. No
  source scanner feeds this API yet, so project acceptance and service-call
  diagnostics are unchanged. Parser-produced binding/member-call nodes remain
  necessary before enforcing ADR 0006 on application source.
- **Next:** Connect structured application nodes to the resolver with explicit
  partial-syntax handling, then validate resolved service operations.

## ADR 0006 import-validation foundation

- **Decision:** User directed continuation after recommended ADR 0006 Option A.
  Recorded acceptance with partial implementation status.
- **Changed:** Structured top-level use declarations map identifier-separated
  module paths to project-relative .svr files. E4090/E4091/E4092 distinguish
  malformed syntax, inaccessible targets and canonical root escapes. Retained
  ProjectImport records include source, target and location; duplicate imports
  are idempotent per source file. Discovery-based indexing terminates cycles.
- **Coverage:** Import syntax, malformed/missing targets, repeated imports,
  cyclic references and source locations. Existing Fielddesk imports are checked.
- **Validation:** All 138 library and 32 CLI tests passed (170 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Boundary:** This adds file-reference validation, not lexical scope or service
  receiver resolution. ProjectCheck literals must supply the new imports field.
- **Next:** Build application binding/member-call nodes before enforcing imported
  service visibility or argument checks. No heuristic dotted-call matching.

## Service-reference resolution design boundary

- **Inspected:** Fielddesk imports app.services and calls maps.travel_minutes and
  payments.draft_invoice. Current project indexes contain no import or lexical
  binding resolution, so dotted-text matching would misclassify shadowed names.
- **Prepared:** ADR 0006 recommends explicit project-relative imports, lexical
  shadowing and service checks only for resolved receivers. Includes examples,
  parser/resolver implementation order, verification cases and a defer alternative.
- **Boundary:** No new call enforcement or application execution was added.
  AGENTS.md requires a decision before introducing these name-resolution rules.
- **Validation:** Documentation/source review only; prior 167-test baseline stands.
- **Next:** Obtain the ADR 0006 decision, then implement the application syntax
  and import/scope foundations before service-call diagnostics.

## Service contract metadata in project JSON reports

- **Changed:** Successful project JSON checks expose service_operations with
  names, parameter/return annotation text, body flags and source locations.
  Source/error reports and human output retain their existing behavior.
- **Contract:** Additive schema-version-one field; source/error reports omit it.
  Empty arrays mean no operations were found. Annotation nulls mean absent,
  never inferred/resolved types. Documented unknown-field tolerance and scope.
- **Coverage:** Added a CLI fixture checking nested type text, omitted annotations,
  body flags and byte ranges through real JSON parsing.
- **Validation:** All 135 library and 32 CLI tests passed (167 total), with no
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Next:** Service-call resolution needs structured application expressions;
  this inspection output supplies declarations, not executable contracts.

## Service return-annotation delimiter checks

- **Changed:** E4027 rejects unclosed/mismatched return-annotation groups and
  top-level commas. Nested generic/tuple/array-like text remains preserved;
  function-type arrows do not consume closing angle brackets. Names remain
  unresolved and these checks do not add executable type features.
- **Coverage:** Added invalid grouping cases and positive nested/arrow/unknown-name
  cases. Existing metadata and project checks continue through the same parser.
- **Validation:** All 135 library and 31 CLI tests passed (166 total), without
  skips. Formatting, strict all-target Clippy and diff checks passed.
- **Next:** Retain the distinction between structurally valid contracts and
  resolved service types when adding service-reference analysis.

## Service return annotations and operation metadata

- **Changed:** Parsed optional return annotations, terminators and same-line body
  markers separately. E4027 rejects missing return text and unsupported suffixes.
  Return types stay unresolved; body contents and default expressions are not
  validated by this structural parser.
- **Inspection:** Added documented ServiceOperation/ServiceParameter records to
  ProjectCheck, preserving service/name, ordered parameter annotations, optional
  return annotation, body flag and source file/span. Public function signatures
  and CLI diagnostic JSON schema remain unchanged. Rust callers constructing
  ProjectCheck literals must now supply service_operations.
- **Coverage:** Return forms/errors plus an end-to-end project metadata regression
  covering application types, absent annotations, body markers and exact ranges.
- **Validation:** Full run passed 133 library and 31 CLI tests without skips;
  the final metadata regression passed separately (165 distinct passing tests
  across both runs). Formatting, strict all-target Clippy and diff checks passed.
- **Next:** Use retained contracts for structured service-reference checking;
  do not infer runtime implementations from metadata alone.

## Structured service parameter declarations

- **Changed:** Operations retain ordered parameter records with names and
  optional annotation slices. Top-level commas delimit parameters; nested
  generic/tuple/array-like delimiters and quoted text remain intact. Duplicate
  names, empty annotations/items and mismatched delimiters report E4027.
- **Compatibility boundary:** Empty lists and one trailing comma are accepted.
  Unannotated parameters remain representable; no executable primitive-only
  annotation policy is applied to application contracts. Type/default-expression
  resolution and return/body parsing remain unfinished.
- **Validation:** All 132 library and 31 CLI tests passed without skips (163 total),
  including existing Fielddesk checks. Added positive nested-type extraction and
  malformed-list regressions. No Application Control block occurred.
- **Next:** Structure return annotations and retain operation metadata for
  service-reference validation without claiming executable application support.

## Windows baseline and structured operation recognition

- **Baseline:** User reports Windows Application Control resolved and cargo test
  passing with rustc/cargo 1.98.1: 127 library tests plus 31 CLI tests, 158 total,
  zero failures; binary and documentation stages also completed. This records
  the user's run, not a newly executed agent verification.
- **Changed:** Service operations now have structured name, parameter-text and
  suffix fields. E4027 diagnoses missing/invalid names and absent or unclosed
  single-line parameter lists. Duplicate checking consumes the parsed name.
- **Boundary:** Application parameter types and trailing return/body syntax
  remain opaque. Balanced nested parentheses and quoted parentheses are retained;
  this does not introduce executable generics or service type resolution.
- **Coverage added:** Structured extraction, malformed declarations and project
  diagnostic source locations. Next: parse parameter declarations incrementally
  without conflating application types with executable primitive annotations.
- **Verified:** All-target run passed 129 library and 31 CLI tests without skips.
  The subsequently added location regression also passed separately, bringing
  exercised coverage to 130 library plus 31 CLI tests. Formatting and strict
  all-target Clippy passed. Windows execution is working in this agent session.

## 2026-09-26 — Structured service-header parser

- **Changed:** Extracted service header recognition into a private project parser
  module with explicit Header and BodyStart representations. The scanner consumes
  pending/open/empty forms instead of interpreting suffixes inline. Public
  compiler and project APIs remain unchanged.
- **Correctness:** Missing/invalid names report E4026 instead of disappearing.
  Invalid headers no longer create service index entries. Existing binding checks
  may additionally identify a manifest binding without a valid declaration.
- **Coverage added:** Header forms, keyword boundaries, invalid identifiers,
  unsupported inline bodies and scanner rejection without indexing.
- **Boundary:** Only headers are structured. Body tracking, operation signatures
  and application wiring remain incremental scanner work; full service parsing
  and type checking are not claimed.
- **Validation:** Formatting, strict all-target Clippy and diff checks passed.
  The all-target test attempt was blocked at rustc startup by Windows Application
  Control (4551); regression execution remains pending.
- **Next:** Extend structured parsing to operation signatures while preserving
  the separation between project wiring checks and executable type checking.

## 2026-09-26 — Address scanner review findings

- **Fixed:** Task names followed by spaces/tabs before parentheses now enter the
  callable index. Service headers with unexpected suffixes or nonempty inline
  bodies report E4026 instead of bypassing contract checks. Empty inline blocks
  still accept internal whitespace. Updated the stale header-recovery regression.
- **Coverage:** Added task-spacing and malformed/inline service-header cases.
  Removed vertical tab from the ASCII-whitespace acceptance matrix because Rust's
  is_ascii_whitespace excludes it; this does not broaden scanner grammar.
- **Validation:** Formatting and strict all-target Clippy passed. The requested
  all-target test attempt was blocked by Windows Application Control before
  rustc's version query executed (4551). Accumulated runtime regressions remain
  unverified; no OS-policy bypass attempted.
- **Next:** Run the accumulated regression suite in an allowed environment,
  then replace service scanning incrementally with structured application parsing.

## 2026-09-26 — Isolate service contents from application wiring

- **Changed:** Recognized service bodies now contribute only direct operation
  duplicate checks and brace tracking. Nested declaration-looking lines no
  longer create global task/model/service/page/auth symbols or entry-file wiring.
  Closing the service resumes ordinary declaration indexing.
- **Coverage added:** Both supported header layouts, nested function bodies,
  fake declarations and wiring entries, and valid functions/models/routes after
  the block. Isolation does not imply validation of service-body syntax.
- **Validation:** Formatting/diff checks only; compilation and test execution
  remain deferred at the user's request.
- **Next:** Service signature/call checking still requires structured parsing;
  retain these scope boundaries when replacing the line-based scanner.

## 2026-09-26 — Whitespace-consistent service indexing and JSON coverage

- **Changed:** Declaration scanning now splits names on all ASCII whitespace,
  instead of literal spaces alone. Tabs before parentheses/braces no longer
  hide service names, operations or ordinary callable declarations.
- **Coverage added:** Keyword boundaries and invalid identifiers, tabbed service
  operation duplicates and following functions. Added a project CLI fixture
  asserting E4025/E4026 file identities and exact diagnostic source lines in JSON.
- **Validation:** Formatting and diff checks only; compilation and test execution
  remain deferred at the user's request. Updated scanner/report documentation.
- **Next:** Execute accumulated compiler/project regressions when requested;
  further service-call analysis requires structured application parsing.

## 2026-09-26 — Incomplete service-block diagnostics

- **Changed:** E4026 identifies recognized service declarations missing their
  opening brace, and tracked service blocks left open at EOF. Locations retain
  the original service declaration file/line/range. Recovery from a missing
  opening brace continues scanning the following ordinary declaration.
- **Coverage added:** EOF after a header, intervening comments, missing closing
  braces with both supported header layouts, CRLF provenance and recovery.
  Inline or otherwise unrecognized contract syntax remains outside this rule.
- **Validation:** Formatting/diff checks only. Compilation and tests remain
  deferred as requested; the implementation is not yet execution-verified.
- **Next:** Verify the accumulated M12 cases before broader service parsing.

## 2026-09-26 — Service headers across lines

- **Changed:** Recognize a standalone opening brace after a service name, with
  blank lines and source comments permitted between them. Duplicate-operation
  checking and callable exclusion now apply to that layout too. An intervening
  declaration cancels the pending header, preventing unrelated blocks from
  being mistaken for service bodies. Same-line headers require an exact brace
  suffix rather than accepting arbitrary text before a trailing brace.
- **Coverage added:** LF/CRLF layouts, comments, duplicate signatures, following
  free functions and abandoned/malformed header isolation. Full syntax errors
  remain outside this scanner rule; no new language grammar is introduced.
- **Validation:** Formatting/diff checks only; compilation and test execution
  remain deferred at the user's request.
- **Next:** Full application parsing is still needed for reliable service-call
  reference and signature validation; verify accumulated scanner regressions.

## 2026-09-26 — M12 service operation scope

- **Changed:** Track top-level multiline service blocks and direct operation
  names; repeated names within one service report E4025 at the repeated source
  line. Service signatures no longer enter the ordinary callable target index.
  Braces in strings/comments do not change scope; following free functions
  remain indexed. Existing manifest binding rules remain in place.
- **Scope:** Supports the checked-in Fielddesk contract layout. Inline operations,
  later-line opening braces, complete syntax/type validation and service-call
  reference resolution remain unsupported. This is a partial scanner extension.
- **Coverage added:** Per-service duplicates, same names across services, source
  provenance, route-target exclusion, quoted/comment braces and later functions.
- **Validation:** User explicitly requested writing code and testing later.
  Ran formatting only; no compilation or tests were attempted for this slice.
- **Next:** Verify accumulated changes, then add service-call reference validation
  when scope-aware application nodes can distinguish calls reliably.

## 2026-09-26 — Check expressions in excess call arguments

- **Finding:** User and builtin calls zipped arguments with parameters, leaving
  excess argument expressions unchecked after the arity error. Source inspection
  identified the gap; the pre-fix regression attempt was blocked by OS policy.
- **Changed:** Visit excess expressions after checking matched arguments. Keep
  E3006 and all existing type rules; report nested expression errors at their
  own spans. This does not change which source programs are accepted.
- **Coverage:** Added user/builtin/print-alias cases with undefined extra values,
  a zero-parameter function with an invalid nested call, and JSON code/span checks.
- **Validation:** Formatting and strict all-target Clippy passed. Focused and full
  tests, including the pending private-helper suite, were blocked at rustc startup
  by Application Control (4551). No test execution is claimed or bypass attempted.
- **Next:** Run the accumulated regression suite on an allowed environment before
  broadening language semantics; preserve the existing website work.

## 2026-09-26 — Implement qualified private module helpers

- **Decision:** User directed roadmap implementation after the ADR 0005 Option A
  proposal. Added same-module private calls using existing qualified syntax.
- **Code:** Module checking extends the public callable map with only that
  module's private declarations. IR emits all module functions. Bare lookup
  remains top-level/builtin; outside access retains E3004. E3016 now includes
  private builtin collisions, superseding ADR 0003's exemption.
- **Coverage added:** Visibility boundaries, arity/type errors, function values,
  private builtin collisions, bare-name stability, helper chains, numeric widening,
  self/mutual recursion, interpreter/Node parity, CLI acceptance/rejection and
  JSON callee ranges. The module example now uses a private add helper.
- **Validation:** Initial regression and subsequent check/test attempts were
  blocked by Application Control at rustc startup (4551). Strict all-target
  Clippy passed after implementation. Tests are added but execution is pending;
  no policy bypass used. Updated ADRs, specification and module lesson.
- **Next:** Execute regression suites in an allowed environment before expanding
  module loading. Cross-file imports and implicit local lookup remain planned.

## 2026-09-26 — Private-module execution design

- **Verification:** Retried the pending all-target tests. Application Control
  blocked rustc's version query (4551); no compilation or tests executed.
- **Inspection:** Private module declarations are absent from both the semantic
  callable map and IR output. Builtin-first call resolution makes private std
  collisions an explicit compatibility issue when private calls are introduced.
- **Proposal:** ADR 0005 specifies qualified same-module private calls, continued
  external rejection and unchanged bare-name resolution. It recommends extending
  E3016 to private builtin collisions and records the necessary ADR 0003 revision.
  Includes a concrete example and compiler/backend/CLI verification plan.
- **Next:** Obtain the compatibility decision required by AGENTS.md before
  implementation. Existing compiler and website changes remain preserved.

## 2026-09-26 — Complete annotation-location hardening coverage

- **Resumed:** The type-span implementation and CLI JSON assertions were already
  present from the interrupted compiler slice. Reviewed parser-to-AST-to-semantic
  propagation and synchronized the specification, ADR, course and status.
- **Coverage:** Exact-token regression covers parameter, return and local types
  across comments, CRLF and preceding UTF-8 text. Added assertions for absent
  parameter/return spans and a regression for declaration-location fallback in
  manually constructed ASTs without annotation spans. E3014 remains at the name.
- **Validation limitation:** Windows Application Control blocked rustc (4551)
  during cargo check, test compilation and all-target tests. No new Rust tests
  executed and no policy bypass was attempted. This slice remains pending
  executable verification on an allowed toolchain or CI. Formatting, strict
  all-target Clippy and diff checks did pass; Clippy completed after the blocked
  rustc invocations, but does not establish test execution.
- **Next:** Run compiler checks on an allowed environment, then assess remaining
  correctness gaps before private-module/cross-file design. Website work remains
  preserved separately in the working tree.

## 2026-09-26 — Functional Sovra website

- **Changed:** Replaced placeholder links with repository/course destinations,
  added actual Cargo installation commands, and replaced proposed struct syntax
  and reserved CLI demonstrations with implemented source-file workflows.
  Roadmap cards retain M0–M15 labels and distinguish partial/planned features.
- **Interactions:** Added example download and copy controls with manual-copy
  fallback, Escape/outside-click menu dismissal, focus indicators, skip link,
  reduced-motion support, and navigation without JavaScript. Preserved the
  existing visual design and static hosting model.
- **Local workflow:** Added a dependency-free Node preview server, npm start/test
  commands, and hosting instructions. No website deployment was performed.
- **Validation:** Three Node tests passed for links/assets, HTTP delivery/errors,
  and navigation/clipboard interaction logic. Linked repository documents exist
  locally; external availability and visual browser layout were not verified.
  The downloadable example matches the displayed code. Cargo compiled the CLI,
  but Windows Application Control blocked execution (4551); no bypass attempted.
- **Next:** Visual desktop/mobile browser review and deployment to the chosen host.

## 2026-09-21 — Reject unresolved source annotations

- **Decision:** User directed continuation of ADR 0004's recommended Option A.
  Executable annotations now accept only the five implemented primitive types;
  unknown names report E3017. User-defined type declarations remain planned.
- **Regression evidence:** A focused test reproduced acceptance of Strng before
  the fix. Coverage includes parameter, return and local annotations in top-level,
  private and exported functions, Any/Text rejection, declaration-only reporting,
  primitive acceptance, local inference and numeric widening execution.
- **Changed:** Annotation declarations validate once, recovering with Unknown to
  avoid repeated call-site errors and spurious return diagnostics. Existing public
  stage APIs and the separate project scanner are preserved. CLI regressions
  cover check/run/IR/JS rejection and JSON file identity/declaration spans.
- **Validation:** 108 library tests and 27 CLI tests passed without skips.
  The all-target command stopped when Windows Application Control blocked the
  binary test harness (4551); the CLI suite passed separately. Formatting,
  cargo check, test compilation and strict Clippy passed. No policy bypass used.
- **Docs/limitations:** Updated specification, function course, status and ADR.
  Locations use parameter-name, function or let-statement spans; precise type
  token ranges and general named-type resolution remain incomplete.
- **Next:** Improve annotation diagnostic spans without changing language policy;
  private-module execution and user-defined types require separate design work.

## 2026-09-21 — Unresolved annotation audit

- **Evidence:** The CLI accepted `fn identity(value: Strng) -> Strng { return
  value } fn main() {}` with exit 0 and an empty JSON diagnostic array despite
  Strng having no declaration. Unknown names become unresolved Named types.
- **Proposal:** ADR 0004 recommends rejecting unknown source annotations with
  planned E3017 until type declarations exist. Primitive types/local inference
  and the separate Fielddesk project scanner remain unchanged. The alternative
  retains nominal placeholders and their unchecked spelling.
- **Validation/scope:** Documentation-only assessment and one real CLI probe;
  no compiler behavior changed. Temporary probe removed and diff checks passed.
- **Next:** Obtain the type/compatibility decision required by AGENTS.md, then
  implement it with declaration, CLI/JSON and execution regression coverage.

## 2026-09-21 — Builtin collisions and module declaration uniqueness

- **Decision:** User approved Option 1, recorded as accepted ADR 0003: reject
  exact callable-name collisions while permitting other names in `std`.
- **Changed:** E3016 checks top-level names and qualified exported names against
  the builtin registry, including bare print. Noncolliding std exports, unrelated
  same-named functions and private names remain valid. Duplicate module functions
  now produce existing E3008 for all private/exported combinations.
- **Regression evidence:** Duplicate private declarations and builtin collisions
  passed before the fixes. Tests cover all four visibility combinations, every
  registered builtin plus print, exact duplicate spans, permitted names executing,
  CLI check/run/IR/JS rejection, and JSON E3016 locations. Calls through bare print
  and std::len remain working.
- **Validation:** 105 library and 26 CLI tests passed without skips. Formatting,
  compilation, test compilation, strict Clippy and diff checks passed.
- **Compatibility/docs:** Previously accepted colliding declarations must be
  renamed. Public stage signatures and runtime dispatch remain unchanged. Updated
  spec, function/module lessons, development status and the accepted ADR.
- **Next:** Assess remaining named-type validation gaps and document any required
  type-semantics decision before enforcement; private execution remains separate.

## 2026-09-21 — Reject unsupported qualified function values

- **Changed:** A known qualified function name in value position reports E3015
  at the expression and suggests making a call. Normal builtin/exported calls
  are unchanged; unknown qualified names retain E3004. This closes acceptance
  of values with no runtime representation, without implementing function types.
- **Regression evidence:** `let value = std::len` passed checking as Int before
  the fix despite lowering to an unresolved variable load. Six negative cases
  cover builtins/exports in locals, arguments and effect expressions with exact
  spans. CLI tests verify check/run/IR/JS rejection; existing call/execution tests
  continue to pass. Updated spec and function lesson.
- **Validation:** All 102 library and 24 CLI tests passed without skips, including
  the expanded Unicode-ordering test previously blocked by OS policy. Formatting,
  compilation, test compilation and strict Clippy passed (after removing one
  redundant format invocation in the new test); diff checks passed.
- **Next:** Audit function namespace collisions with builtins and duplicate
  private module declarations. Named-type resolution remains a separate design
  boundary; do not introduce function values or inference implicitly.

## 2026-09-21 — Unicode string-ordering parity

- **Changed:** JavaScript ordered string comparisons iterate Unicode scalar
  values instead of using native UTF-16 ordering. This matches existing Rust
  UTF-8 ordering, including prefix handling. Equality, concatenation, numeric
  comparisons and source syntax are unchanged; strings are not normalized.
- **Regression evidence:** Emoji versus U+E000 reversed ordering before the fix.
  Differential tests cover all six comparison operators, reversed operands,
  supplementary-character prefixes, empty/equal strings and composed versus
  decomposed Unicode. Added specification and course guidance.
- **Validation:** 101 library tests passed, including the initial 42 comparison
  cases. All 23 CLI tests passed separately without skips. Three additional pairs
  (18 assertions) were added afterward; that rebuilt library executable compiled
  but was blocked by Application Control twice. The empty binary test target was
  also blocked in the all-target run. Formatting, compilation, test compilation,
  strict Clippy and diff checks passed; no final full-suite pass is claimed.
- **Next:** Execute the expanded boundary cases when OS policy permits, then
  audit remaining accepted-source/runtime mismatches before broader features.

## 2026-09-21 — JavaScript call-depth parity

- **Changed:** Both engines share the existing 256-frame limit. Generated
  functions check before entry and release their count in `finally`, including
  early returns, fallthrough and runtime errors. Main counts; builtins do not.
  Errors name the rejected function using the interpreter's existing message.
- **Regression evidence:** JavaScript reached a 257th user frame before the fix
  while the interpreter rejected it. Differential tests cover success with 256
  frames and a builtin at the deepest frame, and failure on frame 257. Additional
  Node tests cover 300 sequential early/fallthrough calls and repeated recursion
  failures followed by successful calls, verifying frame cleanup.
- **Validation:** 100 library and 23 CLI tests passed without skips. Formatting,
  compilation, test compilation, strict Clippy and diff checks passed.
- **Docs:** Updated runtime specification, function lesson and development status.
  No source syntax or public stage signatures changed; full backend parity is
  still experimental.
- **Next:** Test Unicode string ordering parity: Rust string ordering and native
  JavaScript UTF-16 comparisons may disagree for supplementary characters.

## 2026-09-21 — JavaScript string escaping parity

- **Changed:** JavaScript emission now uses dedicated string escaping rather
  than Rust Debug formatting. Quotes/backslashes and control characters are
  escaped for JavaScript; NUL uses fixed-width `\u0000`, and Unicode line/paragraph
  separators are escaped. Other Unicode scalar values are retained.
- **Regression evidence:** Node rejected generated `\0123` in strict mode before
  the fix. Tests compare raw output bytes against the interpreter for all ASCII
  control characters, NUL followed by digits, quotes/backslashes, non-ASCII text,
  emoji and Unicode separators. A source-to-IR-to-Node test also verifies lexer
  escape decoding and emission together.
- **Validation:** 98 library and 23 CLI tests passed without skips. Formatting,
  compilation, test compilation, strict Clippy and diff checks passed.
- **Scope:** No new Sovra escapes or syntax were introduced. Text IR keeps its
  existing inspection format. Updated the specification and String lesson.
- **Next:** Reproduce and align generated JavaScript's call-depth behavior with
  the interpreter's existing 256-frame limit. Full runtime parity remains partial.

## 2026-09-21 — Preserve concatenation's String type

- **Changed:** Arithmetic result typing now returns String for String operands
  after operator compatibility checks. Previously concatenation returned Unknown,
  accepting invalid Int contracts and rejecting valid subsequent concatenations.
  No syntax, stage API, IR instruction or runtime behavior was changed.
- **Regression evidence:** Both invalid Int annotation acceptance and valid
  chained-concatenation rejection failed before the fix. Coverage includes
  binding/return/parameter contracts, inferred locals, length and equality,
  interpreter/Node output parity, and rejection by check/run/IR/JS commands.
- **Example/docs:** Added executable `examples/strings/main.svr`, a String course
  lesson, specification guidance and the invalid-concatenation CLI fixture.
- **Validation:** All 96 library and 23 CLI tests passed without skips. Formatting,
  compilation, test compilation, strict Clippy and diff checks passed.
- **Next:** Test generated JavaScript string escaping against the interpreter;
  the backend still uses Rust Debug string formatting, whose escapes may differ
  from JavaScript. Named-type and private-module semantics remain separate work.

## 2026-09-21 — Separate source and manifest comments

- **Changed:** Source scanning strips unquoted `//`; manifest parsing strips
  unquoted `#`. Shared quote/escape handling preserves markers within strings.
  Removed the list parser's redundant comment handling now that source scanning
  consistently owns it. Diagnostic ranges still refer to original source lines.
- **Regression evidence:** A valid route with a trailing source comment failed
  before the fix. Added checks for quoted URLs/hash fragments, escaped quotes,
  escaped backslashes, comment-only lines, and wrong comment markers in each
  input format. Source `#` text is no longer silently discarded; `.svr` comments
  must use the existing language's `//` syntax.
- **Validation:** 93 library and 22 CLI tests passed without skips. Formatting,
  compilation, test compilation, strict Clippy and diff checks passed.
- **Limits/next:** Project scanning remains line-based, not full lexical or
  application validation. Next, reproduce the semantic checker's apparent loss
  of String type after concatenation and harden existing-subset inference.

## 2026-09-21 — Reject malformed application lists

- **Changed:** Entry service/data lists validate their complete shape and every
  item. E4024/E4062 identify malformed declarations with existing file/line
  provenance. Rejected lists contribute no partial entries to wiring checks.
  Empty lists, identifier items, a single trailing comma, optional semicolon
  and trailing comments remain accepted. Similar key prefixes are ignored.
- **Regression evidence:** Before the fix, `services: [valid, bad-name]` silently
  dropped the invalid item and diagnosed only the remaining reference. Added
  26 invalid cases across both keys and six positive project cases, including
  Unicode/CRLF locations. CLI JSON fixtures cover both new codes.
- **Validation:** 91 library and 22 CLI tests passed without skips, including
  the previously blocked JSON library regression. Compilation, test compilation,
  formatting, strict Clippy and diff checks passed.
- **Limits/next:** The scanner still accepts only single-line lists and is not
  a full application parser. Next, separate source `//` comment handling from
  manifest `#` handling so source comments cannot distort wiring declarations.

## 2026-09-21 — Manifest-value and wiring diagnostic locations

- **Changed:** Manifest entries retain assignment spans. Project index values
  carry their file/range together through sorting and validation. Name/runtime/
  entry/service manifest errors and every service, route, page, auth, data,
  scheduled-task and policy validator now attach declaration provenance.
  Missing targets point at the reference; duplicates at the repeated declaration.
- **Architecture:** Private located values preserve public project result types,
  ordering, diagnostic codes, stage signatures and JSON schema. Full-line ranges
  match the scanner's scope. Entirely missing required keys and filesystem/
  discovery errors remain unlocated rather than using an invented assignment.
- **Regression evidence:** Missing route-target provenance failed before the
  change. Tests cover eight manifest-value cases, 17 wiring cases, locations
  surviving sorting, duplicate occurrences, Unicode/CRLF offsets, and missing-key
  null provenance. Extended CLI fixtures verify manifest and two source files.
- **Validation:** Formatting, compilation, test compilation, strict Clippy and
  diff checks passed. 88 library tests passed before adding a separate JSON
  library regression. All 22 CLI tests passed afterward with no skips, including
  new JSON location assertions. The final 89-test library executable compiled
  but Application Control blocked it twice (initial attempt and unchanged retry).
  That additional library test is not claimed as executed; no OS policy bypass.
- **Next:** Re-run the final library suite when execution is permitted, then
  harden the scanner's malformed list handling so invalid service/data items do
  not silently disappear. Named-type/private-module decisions remain separate.

## 2026-09-21 — Project scan diagnostic provenance

- **Milestone:** Incremental project-file locations for structured diagnostics.
- **Changed:** Manifest parse errors and per-file source-scan errors retain the
  scanned path and full-line byte range, excluding LF/CRLF. JSON reports use
  explicit file provenance; human diagnostics include file, line and column.
  No file is guessed for later validation or I/O errors.
- **Architecture:** Added optional `Diagnostic::source_file`; Rust callers
  constructing diagnostics must initialize it. Stage function signatures,
  diagnostic codes and the version-one JSON envelope remain unchanged.
- **Regression evidence:** Reproduced a malformed route's empty byte range
  before implementation. Tests cover Unicode/CRLF offsets, first-line manifest
  errors, two distinct source files, JSON locations and human output. Corrected
  the CLI test to compare Windows paths after normalization; reported paths
  intentionally preserve the supplied root rather than being canonicalized.
- **Validation:** Formatting, compilation, test compilation, strict Clippy and
  diff checks passed. Final all-target run passed 84 library and 22 CLI tests
  with no Application Control skips, including Node JSON/backend checks.
- **Limits/next:** Retain declaration locations through the project index and
  manifest entries so later value/wiring validation can identify its source.
  Those errors, discovery and I/O errors currently retain null JSON locations.
  Project success still means a shallow wiring check, not application execution.

## 2026-09-21 — Expression source ranges

- **Milestone:** Structured diagnostic precision for the executable subset.
- **Changed:** AST expressions now contain `kind: ExpressionKind` and `span`.
  The parser retains literal/name, qualified-name, call, binary and grouped
  expression ranges. Semantic diagnostics use the relevant expression rather
  than an enclosing statement or an all-zero fallback; argument mismatches
  identify the argument and call-target errors identify the callee.
- **Architecture:** Compiler stage function signatures, diagnostic codes, JSON
  schema and IR instructions are preserved. Rust consumers matching AST variants
  must now match `expression.kind`; IR lowering was adapted accordingly.
  No Sovra syntax or execution semantics changed. Statement-level type contracts
  and declaration diagnostics keep their existing ranges.
- **Regression evidence:** Reproduced E3001's all-zero location before the fix.
  Coverage checks 22 semantic cases across top-level/private module bodies,
  nested parentheses and precedence, every literal kind, Unicode/escapes,
  multiline/CRLF input, and source-to-CLI JSON byte/line/column locations.
- **Validation:** `cargo check`, test compilation, strict Clippy and all-target
  tests passed: 82 library tests and 21 CLI tests, including Node-backed checks,
  with no skipped subprocess assertions. Formatting and diff checks passed.
- **Limits/next:** Project-file provenance remains the next diagnostic slice.
  Runtime errors still lack expression locations because IR does not retain
  spans. Named-type resolution and private module execution remain unfinished.

## 2026-09-21 — Token-stream boundary hardening

- **Milestone:** Existing-subset correctness prerequisite to M12 expansion.
- **Changed:** `Parser::parse_tokens` validates that its input contains exactly
  one EOF token at the end before entering recursive descent. Empty streams,
  missing EOF and early/repeated EOF now return E2006, preserving the public
  stage signature and source-language behavior.
- **Regression evidence:** The empty-stream test reproduced an index-out-of-bounds
  panic before the fix. Added missing/embedded EOF rejection and EOF-only empty
  program coverage, including diagnostic spans. Missing EOF could otherwise
  fail to terminate; early EOF could silently discard trailing tokens.
- **Validation:** Formatting, `cargo check` and test compilation passed.
  `cargo test --all-targets -- --nocapture` passed 80 library and 20 CLI tests,
  including Node-backed checks, with no skipped subprocess assertions.
  Strict all-target/all-feature Clippy and `git diff --check` also passed.
- **Scope/limits:** This validates EOF boundaries, not arbitrary caller-supplied
  token spellings or spans. No syntax/type decision or AST redesign was made.
- **Next:** Preserve expression spans and project-file provenance in diagnostics;
  named-type resolution and private module execution remain unfinished.

## 2026-09-12 — Structured check reports

- **Milestone:** Initial machine-readable diagnostic interface.
- **Changed:** `svr check --format json` / `--format=json` report source and
  project outcomes on stdout. Human output and 0/1/2 exit conventions remain;
  help and usage errors stay human-readable. Input I/O errors use JSON E0001.
- **Architecture:** Dedicated dependency-free `compiler/check_report.rs`
  serializes a versioned envelope without changing compiler diagnostic structs.
  Source locations retain byte/line/character offsets; unavailable and project
  locations are null rather than assigned to a guessed file.
- **Tests:** Four CLI JSON cases failed before implementation. Added Node
  JSON.parse assertions for source success/errors, parameter locations, project
  success/failure, I/O failures, format errors and serialization of all JSON
  control characters and Unicode. Existing human CLI tests remain in place.
- **Review fixes:** Reproduced and fixed help handling after the `--` option
  terminator. Reproduced and fixed E1000 zero-length invalid-character spans;
  ASCII and multibyte characters now have full byte ranges, including at byte 0.
- **Validation:** Latest library run: 77 passed, including Node JSON parsing and
  direct CLI dispatch for the option-terminator fix. Formatting, compile/test
  compile checks, strict Clippy and `git diff --check` passed. Eight focused CLI
  check tests passed after initial JSON integration. The final full-suite attempt
  and one unchanged CLI retry skipped all 20 subprocess assertions because Windows
  Application Control blocked `svr.exe`; no final CLI pass is claimed.
- **Documentation:** Added `docs/reference/check-json.md` and
  `docs/guides/automated-checks.md`; updated spec, status and agent context.
- **Limitations:** Project checks remain partial wiring scans. Expression spans,
  per-file project provenance and full symbol inspection remain incomplete.
  Success reports do not yet carry project statistics or symbol graphs.
- **Next:** Preserve expression and project-file provenance for better diagnostic
  locations, then close remaining type-resolution gaps. Final subprocess
  assertions need an environment that permits executing the built CLI.

## 2026-09-12 — Required function parameter annotations (ADR 0002)

- **Decision:** User approved Option A. Every function parameter now requires
  an explicit type; inferred local `let` types and default Unit returns remain.
- **Implementation:** Semantic E3014 reports each missing annotation at its
  parameter name and suggests `name: Type`. All top-level/exported/private
  functions are covered, even if unused. Parser/AST retain absent annotations
  for diagnostics; Unknown is only recovery after the error in this path.
- **Regression evidence:** Both missing-annotation tests failed before the fix.
  Afterward `cargo test -- --nocapture` passed 71 library and 14 CLI tests,
  with no failures or skipped assertions. Coverage includes mixed signatures,
  exact spans, the original std::len type hole, local inference, a typed-call
  mismatch, parser recovery and check/run/IR/JS-build rejection.
- **Docs/example:** Accepted ADR, spec, status/handoff/agent context, changelog,
  functions course lesson and executable `examples/functions/main.svr` updated.
- **Compatibility:** Previously accepted untyped declarations now fail source
  validation. Project-directory wiring scans do not enforce source semantics.
  Named-type resolution and general inference remain separate unfinished work.
- **Next:** Versioned JSON reports for source/project checking, with honest
  null locations where the current checker does not retain source provenance.

## 2026-09-12 — Return completeness and token source ranges

- **Milestone:** Foundational semantic and diagnostic hardening.
- **Changes:** Non-Unit functions without an explicit return now report E3013,
  including private/exported module bodies. Successful lexer tokens now include
  their full byte ranges; line/column tracking remains character-based.
- **Reason/tests:** Reproduced both defects with failing tests first. Added
  return-contract coverage for top-level/private/exported functions, allowed
  Unit fallthrough and explicit returns. Token-range tests include multibyte
  UTF-8 strings, punctuation, multiple lines and EOF.
- **Files:** `semantic.rs`, `lexer.rs`, spec and module course lesson.
- **Final verification:** `cargo test -- --nocapture` passed 67 library tests
  and 13 CLI tests, with zero failures/ignored tests and no skip messages.
  The previously blocked executable targets ran successfully this time.
  Formatting, `cargo check`, `cargo test --no-run`, strict Clippy and
  `git diff --check` passed. Generated JavaScript tests executed through Node.
- **Limits:** Return analysis is for the existing straight-line AST, not future
  branch/loop control flow. Expressions still lack individual source spans;
  file identities and rich/JSON diagnostics remain to be implemented.
- **Next decision:** ADR 0002 proposes explicit function parameter types versus
  sound inference. This affects source compatibility/type semantics and needs
  the user's decision under the full-development directive before enforcement.
  `docs/design/MEMORY_MODEL.md` records current behavior and evaluation criteria;
  a final memory model is deliberately not selected ahead of its milestone.

## 2026-09-12 — Full development audit and numeric correctness

- **Milestone:** Foundational correctness before ecosystem expansion.
- **Changes/reason:** Added `FULL_DEVELOPMENT_STATUS.md` and ADR 0001. Preserved
  existing Int/Float rules with explicit `WidenFloat` IR conversions at Float
  local/parameter/return boundaries. Mixed numeric runtime operators widen Ints.
  Semantic checking diagnoses oversized integers with E3012; interpreter
  literal decoding returns errors and integer arithmetic uses checked operations.
- **Backend:** Generated JavaScript uses BigInt/Number to retain numeric kinds,
  preserve large integers, implement correct division and check i64 overflow.
  JS stdlib length now produces a BigInt and counts UTF-8 bytes. Numeric helpers
  live in `src/compiler/numeric_runtime.js`, embedded by the Rust backend.
- **Tests:** Four failures reproduced before implementation: mixed arithmetic,
  overflow panic, oversized literal acceptance and invalid leading-zero JS output.
  Afterward all 65 library tests passed, including real Node output/error
  comparisons with interpreter results. CI now explicitly installs Node 22.
- **Docs/examples:** Added the numeric example, expected-output fixture and
  lesson; updated spec, contributor prerequisites and permanent agent context.
- **Limitations:** Full Float formatting/non-finite parity, typed HIR, source
  spans, return completeness and private module execution remain incomplete.
  Windows Application Control has previously blocked CLI execution; library/Node
  results do not imply CLI assertions ran. No native/WASM or platform-readiness
  claims are made.
- **Next dependency:** Return completeness and diagnostic locations in the
  existing straight-line language, before adding control flow.

## 2026-09-12 — Validate module function bodies

- **Milestone:** Executable-subset correctness prerequisite to M12 expansion.
- **Changed:** Extracted shared function-body validation in
  `src/compiler/semantic.rs` and applied it to every module function, exported
  or not. Each body gets its own scope. Entry-signature restrictions remain
  exclusive to top-level `main`; existing diagnostic codes are reused.
- **Why:** Invalid module bodies previously passed semantic analysis and could
  reach execution or code generation. Two new regression tests failed before
  the fix: invalid module bodies and references outside a function's scope.
- **Tests added:** Four semantic tests cover 16 invalid exported/private body
  cases, scope isolation, ordinary module `main` execution, and the executable
  module example. A CLI regression and `tests/fixtures/invalid-module.svr`
  check rejection by source `check`, `run`, IR build and JS build.
- **Documentation/example:** Added `examples/modules/main.svr` and initial
  `docs/course/` module lesson; updated spec, roadmap, production handoff,
  assessment follow-up and agent briefing. Parser/AST/IR/runtime contracts
  required no representation changes; this fixes traversal of existing nodes.
- **Validation:** Focused module tests passed after reproducing the failure.
  The full test attempt passed all 60 library tests (including the module example
  producing `42`), then Windows Application Control blocked the binary test
  target with OS error 4551. The separate CLI run also encountered the existing
  helper's Application Control skips, so CLI assertions are not verified on
  this machine for this change. Formatting, `cargo check`, `cargo test --no-run`,
  strict all-target/all-feature Clippy and `git diff --check` completed without
  code errors. Cargo printed home-path canonicalization warnings.
- **Remaining limitations:** Private module functions are checked but not
  callable/lowered. No implicit module-local lookup was added; exported calls
  use `module::function`. Numeric conversion, overflow, return completeness,
  source spans and backend consistency remain as documented in the assessment.
- **Next task:** Add regression coverage for numeric widening accepted by the
  checker but unsupported by the interpreter, then fix conversion consistently
  across validation, IR and execution. Run CLI regressions in an environment
  where Application Control permits the built binaries.

## 2026-09-12 — Repository handoff and M12 assessment

- **Milestone:** M12 project checker remains in progress; assessed inherited
  M0-M11 foundations before implementation.
- **Changed:** Added `docs/CODEX_HANDOFF_ASSESSMENT.md` and root `AGENTS.md`;
  established this development log. No compiler, syntax, tests or examples changed.
- **Why:** Preserve the existing Rust architecture, distinguish executable
  functionality from Fielddesk's target syntax, and provide persistent context
  for human and AI contributors.
- **Inspection:** Reviewed documentation, configuration/CI, all compiler and CLI
  modules, tests, examples, repository status/history and unfinished-work markers.
- **Tests added:** None; this is reconnaissance and documentation only.
- **Validation:** `cargo test -- --nocapture` passed 56 library and 12 CLI tests,
  with zero failures/ignored tests and no Application Control skip messages.
  Formatting, `cargo check`, `cargo test --no-run`, strict all-target/all-feature
  Clippy and `git diff --check` completed without errors.
  Cargo initially stalled with a home-path canonicalization warning; an approved
  outside-sandbox test run completed successfully.
- **Limitations:** Findings in the assessment are based on source inspection,
  not newly added regression probes. Existing tests do not establish module-body
  validation, numeric correctness or interpreter/JavaScript equivalence. Course
  materials remain absent. Prior Claude authorship cannot be determined per file
  from the inspected Git author metadata.
- **Next task:** Reproduce and fix missing module-body semantic validation in a
  bounded regression-tested change, then address numeric/runtime/backend contract
  gaps before returning to the documented M12 service-contract expansion.
