# Sovra release notes

Draft for the intended 1.0.0 release; **not ready for publication**.
The user-approved release scope requires the full application platform and
third-party library publishing. Current compiler functionality alone does not
satisfy that scope. These notes describe implemented foundations, not a completed
release or a production application runtime.

Sovra provides the canonical `svr` toolchain for a small, explicitly typed
language subset. The public scope includes source-file checking and execution,
typed function parameters, inferred locals, inline modules, a Rust-owned standard
library, text IR output, JavaScript generation, and versioned JSON diagnostics.
Parameters require annotations; bare `print` remains supported as a compatibility
alias.

## Implemented foundations

- Source parsing, semantic validation, and IR lowering
- Interpreter execution for the supported source subset
- JavaScript backend emission for supported programs
- Records, arrays, branches, loops, mutable locals and local dependency packages
- Explicit record exports with qualified types and nominal package identity
- CLI version/help/check/run/build behavior and JSON diagnostic output
- Project manifest and wiring checks for the current partial application scope
- A documented separation between shipped compiler behavior and planned runtime
  and ecosystem features

## Current limits

- Fielddesk is a target example, not a runnable application runtime
- Sovra-native `test`, package installation, registry publication, formatter,
  REPL, and language server remain planned
- Full application typing, structured runtime services, and broad
  platform semantics remain incomplete
- Registry publication and installation, locked resolution and clean-machine
  publish/download/consume verification remain required before version 1.0

The repository can be used for compiler development; it is not a qualified 1.0
release. Follow
[docs/PRODUCTION_READINESS.md](PRODUCTION_READINESS.md) for the current acceptance
and follow-on roadmap.
