# Sovra specification

## Status

M11 completes the initial compiler roadmap: Sovra can lex, parse, validate,
lower to explicit IR, interpret, inspect IR, and emit a portable JavaScript
backend.

## Toolchain contract

* The package is named `sovra`; its canonical binary is `svr`.
* The compiler, runtime, and core tooling are implemented in Rust.
* `svr --version` prints `svr <package-version>` and exits successfully.
* `svr --help` documents options and planned commands.
* `run` compiles programs through semantic analysis, IR lowering, and
  interpreter execution.
* `build` compiles programs through semantic analysis and emits human-readable
  IR by default.
* `build --emit js` emits portable JavaScript generated from the lowered IR.
* `check <source.svr>` validates a source file through parsing and semantic
  analysis without executing it.
* `check <project-directory>` validates the project manifest, required
  metadata, runtime target, entry path, source-file discovery, external
  service binding consistency, app routes, page bindings, auth wiring, data
  model references, and scheduled task targets.
* `check --format json <source.svr|project-directory>` emits a versioned JSON
  report for validation/I/O outcomes. Human output remains the default. See
  [the JSON report contract](reference/check-json.md) for schema and usage errors.
* Other commands remain reserved and report that they are not implemented.
* Unknown commands are rejected with a non-zero status and a help hint.

## M1 lexical contract

The lexer recognizes keywords, ASCII identifiers, decimal integer and floating
point literals, quoted strings with common escapes, punctuation, operators,
whitespace, and `//` line comments. Every token carries a byte span and
zero-based line and column. Invalid characters and malformed strings produce
structured error diagnostics.

The lexer lives in [`src/compiler/lexer.rs`](../src/compiler/lexer.rs).

## M2 grammar contract

M2 parses function declarations, parameter names with optional annotations,
optional return types, blocks, `let` bindings, `return` statements, literals,
identifiers, function calls, module-qualified names, and binary expressions with
conventional precedence. Statement semicolons are optional before a closing
block.

A missing parameter annotation is retained in the AST for a semantic diagnostic;
every valid function parameter requires an explicit type under ADR 0002.

The public `Parser::parse_tokens` API requires exactly one EOF token at the end
of the stream. Empty streams and missing, embedded or repeated EOF markers
produce `E2006`; an EOF-only stream parses as an empty program. Source parsing
continues to obtain its EOF marker from the lexer.

## M3-M8 semantic, type, module, and IR contract

The analyzer validates local names, function names, declared return types,
function call arity, parameter types, duplicate declarations, module exports,
and basic operand compatibility. Successful analysis produces a minimal linear
IR containing loads, stores, operators, calls, returns, and value pops.
Diagnostics use stable `E30xx` codes. Function names and parameters must be
unique, and the `main` entry function takes no arguments and returns `Unit`.

Parsed expressions retain byte ranges and zero-based line/character columns.
Undefined variables and out-of-range literals identify that expression;
invalid operators identify the binary expression. Call-target errors identify
the callee, argument type errors identify the argument, and arity errors identify
the full call. Grouping parentheses are included in the grouped expression's
range. Binding/return contract errors still identify the statement, and
declaration errors identify the declaration. These ranges also appear in JSON
source-check reports without changing diagnostic codes or the report schema.

Every function parameter must declare its type, for example `value: String`.
This includes unused parameters and every top-level, exported module and
non-exported module function. A missing annotation produces `E3014` at the
parameter name:

```text
parameter `value` requires an explicit type annotation; write `value: Type`
```

Replace the suggested `Type` with the intended type.
Local `let` bindings continue to infer their type from the initializer unless
annotated. An omitted function return type means `Unit`, not return inference.
Requiring parameter annotations rejects previously accepted untyped declarations;
see [ADR 0002](adr/0002-function-typing.md), the [function lesson](course/functions.md)
and [executable example](../examples/functions/main.svr).

Primitive types are `Unit`, `Bool`, `Int`, `Float` and `String`. Other annotation
names are not yet resolved against type declarations; explicit parameters do not
complete named-type validation or introduce a general typed HIR.

Parameter and body checks apply to every function, including exported and
non-exported module functions that are never called. Each function has its own
local scope. Only top-level `main` has entry-signature restrictions; a module
function named `main` is an ordinary function.
In the current straight-line grammar, a function declared to return a non-Unit
type must contain an explicit `return`; falling through produces `E3013`.
An expression statement is not an implicit return. Unit functions may fall
through. Branch-sensitive return analysis will accompany future control flow.

In the current subset, calls resolve top-level functions by bare name and
exported module functions by `module::function`, including from inside that
module. Non-exported module functions are checked but are not callable or
lowered. Implicit module-local lookup and private helper execution remain
unsupported. See [the module lesson](course/modules.md) and
[executable example](../examples/modules/main.svr).

## M5 runtime contract

### Numeric execution

`Int` uses signed 64-bit values; out-of-range literals produce `E3012` during
semantic checking. Integer arithmetic overflow is a runtime error in every
build profile. Integer division truncates toward zero. `Float` uses binary64.
Float annotations on locals, parameters and returns widen Int values before
use, and mixed Int/Float operators widen the Int operand before evaluation,
including equality and ordering. Large Int values may round when widened;
Int-only arithmetic and comparison remain exact. Division by zero is an error
for both numeric types. No implicit Float-to-Int narrowing is provided.

The IR carries `widen-float` at annotated boundaries. The JavaScript backend
uses BigInt for Int and Number for Float and requires a runtime supporting
BigInt and TextEncoder. `std::len` counts UTF-8 bytes in both engines.
Complete non-finite Float and Float-to-string parity remains experimental.
See [ADR 0001](adr/0001-numeric-execution.md), the
[numeric lesson](course/numbers.md), and [example](../examples/numbers/main.svr).

### Function execution

The interpreter executes `main` and user-defined functions. Function arguments
are bound to parameters in declaration order, return values are passed back to
the caller, and `print` captures one rendered value per call. Integer and
floating-point arithmetic, string concatenation, equality, and ordered
comparisons are supported at runtime. Calls are limited to a maximum depth of
256 to prevent runaway recursion from crashing the interpreter.

## M9 standard-library contract

M9 exposes a stable `std` namespace containing `std::print`, `std::println`,
`std::len`, and `std::to_string`. The legacy bare `print` call remains accepted
as an alias for `std::print` so early examples continue to run.

`std::print` and `std::println` accept any single value and return `Unit`.
`std::len` accepts a `String` and returns `Int`. `std::to_string` accepts any
single value and returns `String`.

The initial standard-library registry is Rust-owned. Future standard-library
modules can be written in Sovra once module loading, packaging, and bootstrap
behavior are stable enough to support that without changing the trusted core.

## M10 IR contract

The current IR is backend-neutral, typed at the literal boundary, and
inspectable through `svr build`.

## M11 backend contract

The initial compiler backend emits portable JavaScript through
`svr build --emit js <source.svr>`. The generated program preserves the current
IR execution model, including stack-based local execution, user-function calls,
standard-library output capture, arithmetic, comparison, and runtime division
by zero checks.

## M12 project-checker contract

Project checking is a line-based manifest and wiring scan. It does not invoke
the executable parser or semantic analyzer, so a successful project check does
not verify function parameter types or bodies. Use `svr check <source.svr>` for
semantic validation of an executable source file.

Manifest parsing and source-scan diagnostics retain the actual file path and
full-line byte range (excluding LF/CRLF). Human output includes file/line/column,
and JSON reports include the same provenance. Manifest-value errors identify
their assignment. Service, route, page, auth, data, task and policy validation
errors identify the declaration or reference being checked; duplicates identify
the repeated declaration. Missing manifest keys, discovery and I/O errors keep
null JSON locations. This location support does not expand the scanner's
validation scope.

The initial project checker reads `sovra.toml` from a project directory. It
requires `project.name` and `project.entry`, accepts `project.version`,
`runtime.target`, and arbitrary service bindings under `[services]`, and emits
stable `E40xx` diagnostics for malformed or unsupported project metadata.
Manifest service bindings must correspond to `service <name>` declarations in
project source, and app-entry service references must be both declared and
manifest-bound. App entry route declarations must use
`route METHOD "/path" -> target`; methods are limited to common HTTP verbs,
paths must be slash-rooted without whitespace, empty segments, trailing slashes
outside `/`, or invalid `:parameter` names, and targets must resolve to a
function or task symbol. Page route declarations use `page "/path" -> target`
with the same path rules and must resolve to a page or view symbol. App
`auth: module.symbol` bindings must resolve to an `auth` declaration, app
`data: [...]` entries must resolve to known models, and scheduled
`task <schedule> -> module.symbol` declarations must resolve to known task or
function symbols. Auth policies use `allow role to action on Model` with single
items or bracketed action/model lists, plus `allow role to action Model where
...` shorthand for conditional policies; policy model references must resolve
to known model declarations and duplicate policies are rejected.

Entry-file `services: [...]` and `data: [...]` lists must contain comma-separated
bare identifiers on one line. Empty lists and a single trailing comma are
accepted, as are an optional trailing semicolon and line comment. Invalid items,
empty interior items, missing delimiters, or trailing non-comment text produce
`E4024` (services) or `E4062` (data) at the declaration line. A malformed list
contributes no entries to wiring validation. This hardens the existing scanner;
it does not add multiline lists or executable application syntax.

The project scanner follows source `//` line comments and manifest `#` line
comments separately, ignoring either marker inside quoted strings (including
escaped quotes/backslashes). Trailing source comments do not become part of
wiring targets. `#` is not a source comment and is no longer silently removed;
use `//` in `.svr` files. Diagnostic ranges still cover the original full line,
including comments. This scanner fix does not provide full lexical validation.

## Application-language direction

The Fielddesk example documents the intended full application surface: typed
models, service declarations, auth policies, routes, pages, background tasks,
structured concurrency, tests, and project-level commands. Those constructs are
the product direction for the next compiler milestones, not the current parser
contract.
