# Sovra handoff assessment

Current tooling checkpoint (2026-10-21): the historical command table below is
superseded by implemented `new`, `test`, `doctor` and standalone `fmt` commands.
Formatter Windows writes are enabled; package formatting and other-host writes
remain deferred. Consult the development log and full status for current evidence.

Date: 2026-09-12. Baseline: `89ef9a1` (`auth policy validation`). The working
tree was clean before reconnaissance. This handoff adds documentation only.

Follow-up: the module-body validation fix is now implemented and regression
tested (see `DEVELOPMENT_LOG.md`). All exported and non-exported module function
bodies receive the existing semantic checks, with independent local scopes.
The baseline findings below describe the repository before that fix; private
function execution and implicit module-local resolution remain unsupported.
An executable module example and the first course lesson have also been added.
Follow-up full-development work also fixed numeric widening, Int literal bounds,
arithmetic overflow, JS numeric representation, token byte ranges and
straight-line return completeness. See `FULL_DEVELOPMENT_STATUS.md` for current
status; baseline findings below remain a historical audit. ADR 0002 was then
approved and implemented: every function parameter now requires an explicit
annotation (`E3014`), while local inference and default Unit returns remain.
The function example and course lesson explain the compatibility change.
Unresolved named types, private function execution and project-scanner limits
remain open; this update does not revise the historical baseline findings below.

## Current state

Sovra is a Rust 2021 library and `svr` executable in a single dependency-free
Cargo package, version 0.1.0, declaring Rust 1.74 as its minimum version.
It has a functioning small language implementation, not yet a general-purpose
application runtime. Preserve this foundation and its public boundaries.

The long-term identity is a general-purpose language and ecosystem for humans
and AI agents to build reliable complete systems. Predictable semantics,
inspectable structure, and useful diagnostics come before application breadth.
Nova is a future engineering system; no Nova implementation exists here.
Business concepts in Fielddesk are examples, not core language primitives.

The repository labels M0-M11 complete and M12 (project checker) started.
These are the repository's milestone numbers, which differ from the incoming
handoff's illustrative sequence. Existing milestone labels should remain intact.
The completed labels describe feature slices, not comprehensive correctness.

The existing implementation includes source execution, source validation,
text IR and JavaScript generation, plus manifest and application-wiring checks.
Fielddesk is deliberately target syntax: checking its wiring does not parse,
type-check, test, or execute its business logic.

The user identifies prior Claude contributions. Git history inspected here
attributes commits to Peter Musungu, including agent checkpoint commits; it
does not establish which particular lines Claude wrote. The implementation
described below is inherited work, preserved without attributing unsupported
authorship to individual subsystems.

## Repository and inspection coverage

Read the root README, both architecture documents, specification, roadmap,
production upgrade flow, configuration and developer-experience documents,
contribution guidance, changelog, policy documents, Cargo/configuration files,
CI workflow, compiler and CLI source, unit/integration tests, and both examples.

```text
src/main.rs                 executable entry point
src/lib.rs                  public library modules
src/cli.rs                  arguments, file I/O, pipeline orchestration, output
src/compiler.rs             compiler module exports and command registry
src/compiler/               lexer, parser, AST, semantic, IR, interpreter,
                            backend, diagnostics, stdlib, project checker
tests/cli.rs                subprocess integration tests
examples/hello-world/       executable subset example
examples/fielddesk/         proposed full application and test syntax
docs/                      contracts, architecture, roadmap, handoffs
config/svr.toml             historical reserved configuration example
.github/workflows/ci.yml    Rust formatting, Clippy, tests
```

There is no separate runtime crate/directory, Sovra-written `std/`, language
server, package manager implementation, benchmark suite, or `docs/course/`.
The current compact layout is appropriate; directory reshuffling is unnecessary.

## Compiler pipeline

Statuses refer to the implemented subset; limitations are explicit below.

| Area | Status | Implementation and boundary |
| --- | --- | --- |
| Source input | Implemented | CLI reads one UTF-8 `.svr` file; project discovery is separate. |
| Lexer | Implemented | `lexer.rs`: tokens, keywords, decimal literals, escapes, comments, EOF; token-span defect below. |
| Parser | Implemented | `parser.rs`: recursive descent and precedence parsing for functions and inline modules. |
| AST | Implemented | `ast.rs`: functions, modules, parameters, three statement forms, expressions; no application nodes. |
| Semantic analysis | Partial | `semantic.rs`: top-level body checks and function/export lookup; module bodies bypass checking. |
| Type system | Partial | Primitive types, declared locals/parameters/returns, permissive unknown types; no retained expression types or conversion instructions. |
| IR | Implemented | `ir.rs`: linear stack instructions for literals, names, stores, binary operators, calls, returns, pops. No branches or typed conversions. |
| Interpreter | Partial | `interpreter.rs`: `i64`, `f64`, bool, string, Unit, function frames, captured output, 256-frame limit; numeric gaps below. |
| Text/JavaScript backend | Partial | `backend.rs`: IR rendering and real JavaScript generation; semantic parity is incomplete. |
| Native/WASM backend | Not Started | No native or WASM code generator. |
| Runtime | Partial | Rust-owned value execution and four builtins; no networking, persistence, scheduler, or application lifecycle. |
| Diagnostics | Partial | Structured Rust code/severity/message/span records; no JSON CLI mode, file identity, suggestions, or source excerpts. |

The normal path is source -> lexer -> parser -> AST -> semantic validation ->
`TypedProgram` -> IR -> interpreter or backend. `TypedProgram` wraps a cloned
AST rather than a fully typed representation. Source `check` stops after semantic
analysis. Project `check` follows a different path: custom manifest parsing,
recursive discovery, line-based declaration indexing, and wiring validation.
It does not invoke the executable parser or semantic analyzer.

## CLI state

| Command | Current behavior |
| --- | --- |
| No arguments, `--help`/`-h` | Help, exit 0. |
| `--version`/`-V` | `svr 0.1.0`, exit 0. |
| `run <source.svr>` | Parses, checks, lowers, interprets; prints captured lines. |
| `build <source.svr>` | Writes inspectable IR to stdout. |
| `build --emit ir\|js <source.svr>` | Writes the selected representation; `--emit=js` also works. |
| `check <source.svr>` | Parses and semantically checks without execution. |
| `check <directory>` | Checks `sovra.toml`, discovery, services, routes/pages, auth/data/task wiring and policy model references. |
| `new`, `init`, `test`, `fmt`, `repl`, `install`, `update`, `doc` | Stub command dispatch: nonzero not-implemented response; their help reports planned status. |

Invalid arguments/unknown commands generally exit 2; compilation, runtime,
project-check failures and reserved commands exit 1. `run` and `build` require
one `.svr` source path, not a project directory. Build emits text, not a native
executable. There is no Sovra-native test runner yet.

## Language features

Implemented syntax includes `fn`, optional parameter annotations, optional
return annotations, `let` with optional type, explicit `return`, effect
expressions, parentheses, positional calls, literals, and binary arithmetic
and comparison. Inline `mod name { export fn ... }` exposes `name::function`.
This is not cross-file module loading; dotted Fielddesk names are a separate
project-scanner convention. The parser supports only a single `::` qualification
step. Statement semicolons are consumed optionally after every statement.

Primitive names are `Unit`, `Bool`, `Int`, `Float`, `String`. Other type names
become `Named` without declaration resolution. Untyped parameters become
`Unknown`; this is not a full inference or generics system. Local redeclaration
overwrites the scope entry. Duplicate top-level functions, exported functions
within modules, modules, and top-level parameters have checks.

Builtins are `std::print`, `std::println`, `std::len`, `std::to_string`, with
bare `print` preserved as an alias. Both print forms capture an output line.
The interpreter's string length counts UTF-8 bytes.

No executable support exists for conditionals, loops, mutation statements,
collections, model/type/enum declarations, generics, pattern matching,
closures, exceptions/results, imports, concurrency, services, pages, auth,
or test declarations. Some related keywords/operators are lexed without
corresponding parser/semantic behavior. Fielddesk uses these proposed forms,
including `Text`, `Money`, `Id<T>`, `?`, `use app.models`, tasks and UI blocks.

## Test state

`cargo test -- --nocapture` completed successfully during this assessment:
56 library tests and 12 CLI integration tests passed; zero failures and zero
ignored tests. Binary-unit and doc-test targets contain zero tests. No
Application Control skip messages appeared in this successful run.

`cargo fmt --check`, `cargo check`, `cargo test --no-run`, and
`cargo clippy --all-targets --all-features -- -D warnings` also completed
without formatting, compile or lint errors. The sandboxed command sequence
printed home-path canonicalization warnings and eventually completed; the
approved outside-sandbox test run also passed. `git diff --check` was clean.

Unit distribution: CLI 3, lexer 4, parser 5, semantic 8, IR 1, interpreter 7,
backend 2, stdlib 2, project checker 24. Project tests use temporary filesystem
fixtures. CLI tests launch `svr`, exercising help/version, source run/check,
IR/JS emission, Fielddesk checking, and invalid/reserved commands.

Important qualification: `tests/cli.rs::output_or_skip` returns early on Windows
OS error 4551. Such tests would appear passed without exercising assertions.
Keep `--nocapture` when diagnosing local execution and inspect skip output.
The existing CI runs formatting, strict Clippy and all-target tests on Linux.

Missing coverage includes invalid module bodies, runtime widening, numeric
bounds/overflow, missing returns, accurate diagnostic ranges, malformed scanner
inputs, parser robustness, and actual JavaScript execution against interpreter
results. Backend tests assert generated text only. Fielddesk `.svr` tests are
specimens and are not discovered/executed by Cargo or a Sovra test runner.

## Documentation state

The spec and Fielddesk README correctly distinguish target application syntax
from the executable subset. Configuration documentation describes the current
project-wiring checks. Important discrepancies remain:

- M0-M11 completion wording overstates module/type/backend reliability given
  the gaps below. Semantic analysis also does not itself produce IR, despite
  wording in the spec; lowering is a separate stage.
- The spec describes semicolons as optional before a closing block, whereas
  the parser makes them optional after all statement forms.
- The architecture diagram points diagnostics from semantic analysis, though
  lexing, parsing and project checks also produce them.
- README's statement about remaining commands being unimplemented overlooks
  the implemented `check` command.
- CONTRIBUTING still says to keep M0 free of lexer behavior. This is historical
  guidance, superseded by the implemented compiler and current roadmap.
- `config/svr.toml`, several module comments, CLI status strings and changelog
  retain older milestone wording. The actual project manifest is `sovra.toml`.
- `TokenKind::String` says escapes are preserved, but the lexer decodes them.
- There is no official course yet. Add `docs/course/` incrementally alongside
  verified features; proposed application examples must stay clearly labeled.

## Technical debt and correctness findings

These are source-inspection findings, not new regression tests executed during
this documentation-only handoff. Reproduce each in a focused test before fixing.

1. **Module bodies escape validation.** `SemanticAnalyzer::analyze` indexes
   exported signatures but traverses only `program.functions` for statement
   checking. For example, an exported function returning an undefined name can
   pass `check`. Private module functions are neither checked nor lowered, and
   unqualified internal calls are not rewritten to module-qualified symbols.
2. **Numeric acceptance and execution disagree.** Semantic compatibility accepts
   `Float <- Int` and mixed numeric operators. IR discards binding annotations
   and emits no conversion. `let scaled: Float = 2; print(scaled + 3.5)` passes
   analysis but reaches an unsupported Int/Float interpreter operation. Mixed
   equality compares value variants rather than widened numeric values.
3. **Numeric input can panic.** Integer text is not range-checked before
   interpreter `parse().expect(...)`. A decimal beyond `i64` is therefore a
   panic risk despite the expectation message asserting earlier validation.
   Integer arithmetic is unchecked, so overflow can differ by build profile.
4. **Return and type guarantees are incomplete.** A function declared `-> Int`
   can fall through with no value; a program without `main` can pass checking,
   with entry absence detected at execution. Unknown and arbitrary
   named types are permissive; they are not validated type declarations.
5. **JavaScript semantics differ.** All numbers become JS Number; integer-sized
   floating values trigger integer division (`5.0 / 2.0` emits division logic
   yielding 2 rather than the interpreter's 2.5). Large integers lose precision.
   `std::len` counts UTF-16 code units versus Rust UTF-8 bytes. Generated calls
   lack the interpreter's depth limit. Rust Debug string formatting is used
   as JS escaping; escapes such as Rust `\0` before a digit need JS validation.
6. **Locations are incomplete.** The lexer saves `current_span()` before scanning
   and uses it unchanged for successful tokens, leaving zero-length spans.
   Expressions have no spans; effect-expression errors use a zero span.
   Many project diagnostics use synthetic positions and carry no source path.
   `run`/`build` print fewer location details than `check`.
7. **Project checks are shallow and scope-insensitive.** The scanner indexes
   line prefixes rather than blocks/imports. Service operation signatures can
   enter the global callable set. File stems and bare symbols can collide.
   Invalid `services:`/`data:` list items can silently disappear. Source scanning
   reuses the manifest's `#` comment stripper rather than the lexer's `//` rules.
8. **Auth metadata loses meaning.** Policy parsing drops the `where` condition
   and auth-block ownership. Duplicate detection compares only role/actions/
   models, so distinct conditional policies may be considered duplicates.
   No enforcement, role validation or predicate checking exists.
9. **Parser boundary assumptions need tests.** `parse_tokens` requires EOF but
   does not validate empty/malformed caller-provided streams. Generic operator
   precedence defaults to zero, admitting operators into ASTs that semantics
   later rejects. Keep lexer, parser and semantic support lists aligned.

The requested TODO/FIXME/HACK/unimplemented/panic/todo/placeholder/stub scan
found no explicit unfinished compiler macros. The explicit `panic!` is in the
CLI test helper; Fielddesk's `.stub` is proposed service mocking syntax.
Absence of markers does not mean completeness: unchecked `expect` calls and
the stub CLI commands remain relevant. No second compiler or conclusively dead
subsystem was found. Duplication exists in CLI pipeline/error rendering,
quote parsing, and interpreter/backend builtin behavior; consolidate only
incrementally with behavioral coverage.

## Architecture risks

- An untyped AST wrapper and string-based IR names/operators allow validation
  assumptions to disappear before execution. Retain the stage APIs while
  introducing resolved symbols/types only where correctness requires them.
- Two independent interpretations of source syntax (parser versus scanner)
  can produce false confidence. Avoid treating project success as full program
  validation; migrate application declarations to structured nodes in slices.
- Duplicated runtime semantics in Rust and generated JS need differential
  execution tests before portability claims become stronger.
- Diagnostics without file identity and precise spans limit both human fixes
  and future Nova/agent inspection.
- Filesystem discovery recursively scans all subdirectories without project
  exclusions, and entry containment is lexical rather than canonical. Specify
  links, exclusions and module identity before scaling project loading.
- A custom limited TOML reader is intentional today; document its subset rather
  than assuming support for general TOML syntax.

## Recommended next milestone

**Stabilize correctness of the existing executable subset before expanding M12.**
Keep M12 as the active roadmap milestone; add a prerequisite hardening slice,
not a replacement roadmap or broad rewrite. The existing production handoff
recommends service contracts next; this assessment recommends delaying that
expansion because unchecked module bodies and numeric mismatches undermine
the existing `check -> run/build` contract.

The first bounded implementation should validate every module function body
using the existing semantic machinery, with negative tests for undefined names,
wrong returns and duplicate parameters plus a positive exported-call test.
Specify module-local resolution/private-function behavior before extending it.
Then close numeric conversion/bounds and backend parity gaps with regression
tests and synchronize spec/examples/course guidance. Completion means supported
programs have consistent checked and executed behavior, invalid programs yield
diagnostics rather than panics, and both execution paths have meaningful tests.

Do not build Nova, add business primitives, or expand the application runtime
as part of this prerequisite. This handoff intentionally implements none of
those semantic changes; it provides the evidence and next concrete task.
