# Sovra specification

## Status

M5 adds runtime execution of the lowered IR, including user-function calls and
return values, over the M3 semantic and IR foundation.

## Toolchain contract

* The package is named `sovra`; its canonical binary is `svr`.
* `svr --version` prints `svr <package-version>` and exits successfully.
* `svr --help` documents options and planned commands.
* `run` and `build` compile programs through semantic analysis and M5
  execution/text lowering. Other commands remain reserved and report that they
  are not implemented.
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
function calls, and binary expressions with conventional precedence. Statement
semicolons are optional before a closing block. Semantic validation and
execution beyond M4 remains deferred to later milestones.

## M3 semantic and IR contract

M3 validates local names, function names, builtin `print`, declared return
types, and basic operand compatibility. Successful analysis produces a minimal
linear IR containing loads, stores, operators, calls, returns, and value pops.
Diagnostics use stable `E30xx` codes, including function call arity and
parameter-type errors. Function names and parameters must be unique, and the
`main` entry function takes no arguments and returns `Unit`.

## M5 runtime contract

The interpreter executes `main` and user-defined functions. Function arguments
are bound to parameters in declaration order, return values are passed back to
the caller, and `print` captures one rendered value per call. Integer and
floating-point arithmetic, string concatenation, equality, and ordered
comparisons are supported at runtime. Calls are limited to a maximum depth of
256 to prevent runaway recursion from crashing the interpreter.
