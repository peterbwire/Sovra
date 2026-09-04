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

M2 parses function declarations, typed or untyped parameters, optional return
types, blocks, `let` bindings, `return` statements, literals, identifiers,
function calls, module-qualified names, and binary expressions with
conventional precedence. Statement semicolons are optional before a closing
block.

## M3-M8 semantic, type, module, and IR contract

The analyzer validates local names, function names, declared return types,
function call arity, parameter types, duplicate declarations, module exports,
and basic operand compatibility. Successful analysis produces a minimal linear
IR containing loads, stores, operators, calls, returns, and value pops.
Diagnostics use stable `E30xx` codes. Function names and parameters must be
unique, and the `main` entry function takes no arguments and returns `Unit`.

## M5 runtime contract

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

## Application-language direction

The Fielddesk example documents the intended full application surface: typed
models, service declarations, auth policies, routes, pages, background tasks,
structured concurrency, tests, and project-level commands. Those constructs are
the product direction for the next compiler milestones, not the current parser
contract.
