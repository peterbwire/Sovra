# Sovra release notes

This is the initial public 1.0.0 release of the Sovra compiler foundation.
The shipped scope is the source compiler, interpreter, IR, machine-readable
check reports, CLI surface, and compiler-focused project checks. It is not a
complete application platform or production runtime.

Sovra provides the canonical `svr` toolchain for a small, explicitly typed
language subset. The public scope includes source-file checking and execution,
typed function parameters, inferred locals, inline modules, a Rust-owned standard
library, text IR output, JavaScript generation, and versioned JSON diagnostics.
Parameters require annotations; bare `print` remains supported as a compatibility
alias.

## Included in this release

- Source parsing, semantic validation, and IR lowering
- Interpreter execution for the supported source subset
- JavaScript backend emission for supported programs
- CLI version/help/check/run/build behavior and JSON diagnostic output
- Project manifest and wiring checks for the current partial application scope
- A documented separation between shipped compiler behavior and planned runtime
  and ecosystem features

## Current limits

- Fielddesk is a target example, not a runnable application runtime
- Sovra-native `test`, package installation, registry publication, formatter,
  REPL, and language server remain planned
- Full application typing, control flow, structured runtime services, and broad
  platform semantics remain incomplete
- Local package and library publication work is intentionally kept separate from
  the compiler foundation release

The release is appropriate for compiler-focused evaluation and early adoption, but
it should not be described as a complete application platform. Users should follow
[docs/PRODUCTION_READINESS.md](PRODUCTION_READINESS.md) for the current acceptance
and follow-on roadmap.
