# Development log

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
