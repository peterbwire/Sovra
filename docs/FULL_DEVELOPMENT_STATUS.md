# Full development status

Updated: 2026-09-21. Status terms: Implemented, Partial, Experimental, Stub, Planned.

## Repository state and audit

The comprehensive audit is in `CODEX_HANDOFF_ASSESSMENT.md`; the subsequent
module validation fix is recorded in `DEVELOPMENT_LOG.md`. The foundation pass reviewed
the current diff, numeric semantics, IR, interpreter, JavaScript generator,
test infrastructure and existing architecture. Prior edits remain in place.
The repository is a single Rust 2021 package with no third-party dependencies.
No restart or crate/layout migration is needed.

## Current capabilities

| Area | Status | Working scope / remaining work |
| --- | --- | --- |
| Lexer/parser/AST | Implemented subset | Functions, locals, literals, binary operations, inline modules; no control flow or structured data. |
| Name/type analysis | Partial | All function bodies, required parameter annotations and straight-line return completeness checked; named-type and private resolution remain incomplete. |
| Typed representation/IR | Partial | Validated AST wrapper, linear stack IR and explicit Float widening; no general typed HIR, branches, optimizer or native ABI. |
| Interpreter/runtime | Partial | Primitive values, functions, builtins, depth bound, numeric widening and checked Int arithmetic. |
| Backend | Experimental | Text IR and JavaScript emission with tested numeric parity; full Float formatting and general runtime parity remain incomplete. No native/WASM target. |
| CLI | Partial | Help/version, source run/build/check, JSON check reports and shallow project check work. Other recognized commands are stubs. |
| Standard library | Partial | print/println, len, to_string only; filesystem, collections, networking and other modules are planned. |
| Project system | Partial | Manifest, source discovery and application wiring scanner; not application type-checking or execution. |
| Tooling | Partial | Versioned JSON check outcomes implemented. Formatter, REPL, Sovra test runner, package resolution/registry, debugger, LSP and symbol inspection remain planned. |
| Application libraries | Planned | Web/data/auth/cloud libraries follow a stable core; Fielddesk is target syntax. |

Working features include `Int`, `Float`, `Bool`, `String`, `Unit`, typed
parameters/locals/returns, functions and exported inline module calls. Every
function parameter requires an explicit type under approved ADR 0002; local
`let` inference and omitted return annotations meaning Unit are preserved. Char,
Never, collections, closures, generics, structs/enums, traits, imports,
pattern matching, Result/Option, mutation and concurrency remain planned.
Nova integration follows inspectable compiler/tooling interfaces; no custom
foundation model is planned in this development pass.

## Verification and documentation

The public token parser now rejects malformed EOF boundaries with `E2006`
instead of panicking, failing to terminate or silently ignoring trailing tokens.
See the latest development-log entry for this incremental hardening validation.

The builtin-collision slice passed 105 library and 26 CLI tests without
skips, including all previously blocked Unicode-ordering boundary cases.
Formatting, compilation, test compilation and strict Clippy passed. Windows
Application Control has intermittently blocked earlier runs; inspect skip/block
messages rather than trusting counts alone. CI explicitly installs Node 22.

Specification, architecture, roadmap, handoff, examples, agent context and
function, module and numeric course lessons exist. JSON check reports have a
reference and automation guide; the complete curriculum is unfinished.
New stable behavior needs executable examples and tests, not just prose.
Numeric implementation decisions were recorded in ADR 0001 before code.
The user approved ADR 0002 before enforcement of required function parameter
annotations. The diagnostic is `E3014` at the parameter name; the parser retains
missing annotations for this semantic error. This rejects formerly accepted
untyped declarations. See `DEVELOPMENT_LOG.md` for validation of that later slice.

## Blockers and debt

- Numeric widening, Int bounds/overflow and JS numeric kinds are now fixed and
  tested. Non-finite Float/output policy and structured runtime errors remain.
- String concatenation retains its String result type through local inference,
  typed calls and returns; invalid numeric annotations no longer pass checking.
- Qualified function references used as values now report E3015 instead of
  being typed as the function's return value and failing at runtime.
- Approved ADR 0003 rejects exact callable builtin collisions with E3016,
  preserving the print alias and noncolliding std members. E3008 now covers
  duplicate module functions regardless of visibility.
- JavaScript string escaping now preserves control characters and Unicode,
  including NUL followed by digits, with interpreter/Node output comparisons.
- Both engines enforce the same 256 active user-frame limit, counting main and
  excluding builtins. JavaScript releases frames on returns and thrown errors.
- JavaScript string ordering now follows Unicode scalar values like the
  interpreter, including supplementary characters; no normalization is applied.
- Token and expression byte ranges now cover the full spelling, including
  grouping parentheses. Semantic expression diagnostics and JSON source reports
  retain these ranges. Project parsing, scanning and declaration-based value/wiring
  errors retain file/line ranges. Missing keys, discovery and I/O errors still
  use null JSON locations; related-location notes remain unimplemented.
- Private module functions are checked but not lowered/callable; no import graph.
- Straight-line non-Unit fallthrough and missing parameter annotations are
  rejected. ADR 0002 is approved and implemented; named-type resolution and a
  general typed HIR remain incomplete.
- Project scanning is line-based, scope-insensitive and not full validation.
  Malformed entry service/data lists now fail with E4024/E4062 instead of
  silently dropping invalid items; multiline application parsing remains absent.
  Source comments use `//`, separately from manifest `#` comments, with quoted
  markers preserved. This does not make scanning full lexical validation.
- Tests need backend execution comparisons and meaningful platform coverage.
- Windows execution policy has intermittently blocked CLI subprocess assertions;
  the latest run executed all of them. Do not bypass policy or count future skips
  as validation.

## Development sequence

1. Completed: numeric widening/Int correctness, real differential tests, token
   ranges, straight-line return completeness and explicit function parameter
   typing under approved ADR 0002, followed by versioned JSON check reports.
2. Close remaining type holes and add
   expression/file-aware structured diagnostics.
3. Resolve/document private modules and cross-file loading before package work.
4. Add control flow and structured data incrementally, with design records for
   syntax/type decisions and a reviewed memory-model decision before references
   and concurrency. Do not select a final memory model from scaffolding alone.
5. Project creation/run/build, official formatter, REPL and Sovra test runner.
6. Stdlib, packages/lockfiles, LSP/VS Code and structured agent inspection.
7. Benchmark infrastructure, interop/native/WASM targets, cross-platform releases.
8. Application libraries, deployment integration and Nova workflows.

Keep existing M0-M15 labels; this sequence records dependencies rather than
renumbering completed slices. Continue authorized routine engineering between
milestones. Document and pause before fundamental syntax, memory, type or
compatibility changes. Do not label the overall platform production-ready.
