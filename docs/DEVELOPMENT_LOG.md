# Development log

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
