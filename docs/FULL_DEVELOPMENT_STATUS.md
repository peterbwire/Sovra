# Full development status

Updated: 2026-10-04. Status terms: Implemented, Partial, Experimental, Stub, Planned.

Ordinary calls without a unique supported signature now fail with E4133 rather
than disappearing from validation. Additive JSON `ordinary_calls` records expose
ordinary/builtin/unresolved classifications, argument type evidence, declared
return types and source ranges. This advances partial inspection; signature
resolution is not call validity, and cross-file ordinary resolution remains open.

Application ordinary-call resolution now consumes the existing Rust stdlib registry.
Qualified names preserve their identity; stdlib results propagate and invalid
arguments fail checking. Bare print is preserved, user Any annotations remain
unsupported and ordinary builtin collisions produce E4132.

Same-file ordinary function checking now collects primitive signatures before
body analysis, supporting forward references and mutual recursion. Arguments,
duplicate declarations and ordinary return contracts are checked; validated
results flow into conditions, service arguments and locals. Shadowed names, task
names and qualified/cross-file ordinary calls do not acquire ordinary signatures.
`examples/application-functions` demonstrates the checker subset, not execution.

Unresolved condition and service return types now fail opt-in checks with E4125
and E4126. These report incomplete type validation at expression ranges without
claiming a proven mismatch. Syntax coverage can remain complete while typing fails.

Structured if/else-if/else and while inspection now retains calls and lexical
types in each block. E4123 requires both conditional arms to return and treats
loops as potentially skipped. E4124 rejects known non-Bool conditions; unresolved
conditions remain outside full validation. Else-if chain depth is bounded.

Non-Unit service implementations now require an explicit return within the
supported straight-line/unconditional-block syntax (E4123). Unit bodies and
declaration-only contracts are exempt. Branches/loops remain unsupported;
unknown return types are not validated by this presence check.

Known explicit service implementation returns now validate against resolved
contracts with E4122, including bare Unit returns and Int-to-Float widening.
This does not prove return-path completeness or resolve unknown return expressions.

Service call result types now propagate through nested calls, local bindings and
binary expressions using canonical contracts and existing direct import visibility.
Shadowed/ambiguous receivers, wrong arity and invalid/unknown arguments do not
supply result types. Ordinary function calls and implementation typing remain open.

Primitive binary expression typing is now implemented in the opt-in application
checker, including numeric widening, String concatenation and comparisons. E4121
rejects known incompatible operands. Call results and full body typing remain open.

Primitive annotated local initializers now validate known type compatibility
with E4120. Compatible initializers propagate the declared type, including Float
after Int widening. Invalid/unknown initializers do not supply type evidence.
Compound expressions and full application body typing remain unfinished.

Service argument checking now follows primitive parameters and inferred locals
through lexical scopes, including nested shadowing and unknown binding barriers.
Annotated locals and compound expressions remain outside the typed subset.

Literal service-call argument validation is now implemented within the partial
opt-in checker: E4119 reports mismatches, Int may widen to Float, and argument
ranges are preserved. Variables and compound expressions remain unresolved.
The full suite passed 244 library and 42 CLI tests with no skips; an additional
argument-evidence regression verifies unresolved expressions and nested calls.

## 2026-10-03 primitive service contracts: Partial

The opt-in service checker now resolves primitive parameter/return annotations,
defaults omitted returns to Unit and rejects unresolved types with E4117.
The new Rust signature API preserves canonical source/service identity and omits
invalid operations instead of retaining unknown types. E4118 identifies source
owner failures. Ordinary project inspection still retains raw annotation text.
Named application types, argument values and implementation bodies are not yet
typed; application execution and library publishing remain unfinished.
Validation: **243 library + 42 CLI = 285 passed**, no skips, with required Windows
CLI execution; strict Clippy, formatting and the documentation test stage passed.

## 2026-10-03 exported-record package interfaces

Subsequent compiler upgrade: module-local record/alias scopes are implemented,
with qualified references retained through semantic analysis, lowering and
package linking. Same-named types in different modules no longer collide or
capture each other's fields/signatures. Iterative alias resolution fixes a
reproduced 20,000-alias stack overflow. Latest suite: **241 library + 42 CLI =
283 passed**, no skips; strict Clippy and formatting passed. The runnable
`examples/scoped-types` example executes in both engines. This does not finish
M12, application execution, compound-copy semantics or registry publication.

Implemented accepted ADR 0011 option A for local dependency packages:
`export struct`, qualified annotations/construction, public field reads and
nominal identity across packages. Unexported records cannot cross public
signatures or public record fields. Multiple aliases for one package retain the
same identity; identical layouts in different packages do not. Public fields are
the current milestone rule, not a prohibition on future explicit encapsulation.
Both execution engines are covered. Latest Windows validation: 230 library and
40 CLI tests passed, no skips; strict Clippy and formatting passed at that earlier
checkpoint. Registry publication and full M12 remain unfinished.

## 2026-10-02 language-core follow-up

The in-progress executable subset now includes arrays, indexing, `if`/`while`,
`let mut`, and assignment. Direct indexed writes require a mutable array binding;
the compiler writes the modified array back to that binding. Nested indexed
writes remain unsupported. Boolean `&&` and `||` are typed, short-circuit during
execution, and use conditional branches in the JavaScript backend. User-defined
aliases and structs are executable: record literals construct runtime values,
field reads work across local and function boundaries, and both engines display
nested records consistently. Interpreter and JavaScript array reads/writes
agree on index type and bounds errors. Float aliases preserve numeric widening
at local, parameter and return boundaries. Branch and loop local declarations
now preserve lexical shadowing at runtime. Float widening also applies to
mutable assignment and inferred Float array elements and indexed writes. This
incremental work does not establish a complete semantic type system or
application runtime; see the corresponding entries in `DEVELOPMENT_LOG.md`
and `spec.md`.

Current release direction: **production readiness, not yet achieved**. The
developer-preview plan is superseded by [the production audit](PRODUCTION_READINESS.md).
The latest pre-audit Windows baseline is 192 executed tests with no skips;
formatting, strict Clippy, source packaging and optimized artifact smoke checks
passed. Historical blocked/pending notes below describe earlier checkpoints and
are superseded for tests in that baseline. Hosted platform and Rust 1.74 jobs
remain unverified. Project service-call inspection and JSON member references
are implemented for a limited subset; full application typing/runtime and package
publication remain unfinished. Consult the development log for subsequent counts.

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
| Lexer/parser/AST | Implemented subset | Functions, locals, literals, binary operations, inline modules, control flow, arrays, aliases and records. |
| Name/type analysis | Partial | All bodies and annotations checked; definite returns include nested conditionals; aliases and nominal records implemented; no general typed HIR. |
| Typed representation/IR | Partial | Validated AST wrapper, stack IR, explicit Float widening, branches, arrays and record operations; no general typed HIR, optimizer or native ABI. |
| Interpreter/runtime | Partial | Primitive and record values, arrays, functions, builtins, depth bound, numeric widening and checked Int arithmetic. |
| Backend | Experimental | Text IR and JavaScript emission with tested numeric parity; full Float formatting and general runtime parity remain incomplete. No native/WASM target. |
| CLI | Partial | Help/version, source run/build/check, JSON check reports and shallow project check work. Other recognized commands are stubs. |
| Standard library | Partial | print/println, len, to_string only; filesystem, collections, networking and other modules are planned. |
| Project system | Partial | Manifest, source discovery and application wiring scanner; not application type-checking or execution. |
| Tooling | Partial | Versioned JSON check outcomes implemented. Formatter, REPL, Sovra test runner, package resolution/registry, debugger, LSP and symbol inspection remain planned. |
| Application libraries | Planned | Web/data/auth/cloud libraries follow a stable core; Fielddesk is target syntax. |

Working features include `Int`, `Float`, `Bool`, `String`, `Unit`, typed
parameters/locals/returns, functions, exported inline module calls, aliases,
records, arrays, basic control flow and local mutation. Every function parameter
requires an explicit type under approved ADR 0002; local `let` inference and
omitted return annotations meaning Unit are preserved. Char, Never, general
collections, closures, generics, enums, traits, pattern matching, Result/Option,
field mutation and concurrency remain planned.
Nova integration follows inspectable compiler/tooling interfaces; no custom
foundation model is planned in this development pass.

## Verification and documentation

The structured service-parameter slice passed 132 library and 31 CLI tests on
Windows (163 total, no skips/failures). Service parameter names and annotation
text are now parsed structurally; application types remain unresolved.

The user reports a successful Windows baseline after resolving Application
Control: 127 library tests and 31 CLI tests passed (158 total, zero failures),
plus successful binary/doc-test stages, using rustc/cargo 1.98.1. This supersedes
the historical Windows blocking notes below for that baseline. Later changes
require their own validation results.

The annotation-location follow-up adds exact-token and missing-span fallback
coverage. Its latest local compilation/test attempts were blocked by Windows
Application Control (rustc, 4551); execution verification remains pending.

The public token parser now rejects malformed EOF boundaries with `E2006`
instead of panicking, failing to terminate or silently ignoring trailing tokens.
See the latest development-log entry for this incremental hardening validation.

The unresolved-annotation slice passed 108 library and 27 CLI tests without
skips. The combined all-target run was interrupted by Application Control
blocking the binary test harness; the CLI suite passed separately.
Formatting, compilation, test compilation and strict Clippy passed. Windows
Application Control has intermittently blocked earlier runs; inspect skip/block
messages rather than trusting counts alone. CI explicitly installs Node 22.

Specification, architecture, roadmap, handoff, examples, agent context and
function, module, numeric and record course lessons exist. JSON check reports
have a reference and automation guide; the complete curriculum is unfinished.
New stable behavior needs executable examples and tests, not just prose.
Numeric implementation decisions were recorded in ADR 0001 before code.
The user approved ADR 0002 before enforcement of required function parameter
annotations. The diagnostic is `E3014` at the parameter name; the parser retains
missing annotations for this semantic error. This rejects formerly accepted
untyped declarations. See `DEVELOPMENT_LOG.md` for validation of that later slice.

## Blockers and debt

Accepted ADR 0004 rejects unresolved executable-source annotations with E3017.
Unit, Bool, Int, Float, String, declared aliases and structs resolve. E3017
identifies precise type-token spans in parsed source; manually constructed ASTs
without those spans use declaration locations.
Project scanning is unaffected.

- Numeric widening, Int bounds/overflow and JS numeric kinds are now fixed and
  tested. Non-finite Float/output policy and structured runtime errors remain.
- String concatenation retains its String result type through local inference,
  typed calls and returns; invalid numeric annotations no longer pass checking.
- Qualified function references used as values now report E3015 instead of
  being typed as the function's return value and failing at runtime.
- Excess call arguments are now checked after E3006, retaining nested expression
  diagnostics. Regression execution remains pending due to local Windows policy.
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
- ADR 0005 adds qualified private calls within the declaring module and lowers
  private functions. External access remains rejected. Execution verification
  is pending due to Windows policy; cross-file loading remains planned.
-   Non-Unit fallthrough across nested conditionals and missing parameter
  annotations are rejected. Loops conservatively do not establish definite
  return. ADR 0002 is approved and implemented; a retained expression-level typed
  HIR remains incomplete.
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

Third-party library publishing and consumption are explicit planned requirements.
The current local import boundary must evolve into per-package containment with
declared dependencies, package-qualified identities and public exports. See
`design/PACKAGES_AND_LIBRARIES.md`. There is no package resolver, lockfile,
registry client or publisher yet; existing import checks do not provide them.

1. Completed: numeric widening/Int correctness, real differential tests, token
   ranges, branch-aware return completeness and explicit function parameter
   typing under approved ADR 0002, followed by versioned JSON check reports.
2. Close remaining type holes and add
   expression/file-aware structured diagnostics.
3. Resolve/document private modules and cross-file loading before package work.
4. Continue hardening control flow and structured data, with design records for
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
