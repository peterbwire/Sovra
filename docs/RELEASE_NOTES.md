# Sovra release notes

Draft for the intended 1.0.0 release; **not ready for publication**.
The user-approved release scope requires Sovra to build a production-quality
Task Manager CLI. The current variable-length dogfood application is useful
validation but does not yet satisfy every release gate. These notes describe implemented foundations,
not a completed release.

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
- Value semantics and nominal equality for arrays and records
- Bounded UTF-8 text-file reads and replacement writes with explicit result records
- Portable application-selected process status for scripted CLI failures
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
- Registry publication and installation remain planned for later versions
- A local Windows benchmark completes 5,000-task list, add and restart-list
  operations in under four seconds each in both engines; exact-artifact and
  supported-host release qualification remain required before version 1.0
- Interactive Task Manager commands refresh external file changes and detect
  stale edits before saving. Simultaneous writers remain unsupported without
  atomic conditional replacement or locking. Unix creation and replacement
  request owner-only `0600` mode, narrowing previously group-shared files;
  Windows custom ACLs are not preserved. A write can
  replace a symlink rather than its target. Use a regular file in a private
  directory for sensitive titles
- Candidate archives bundle the Task Manager source for extracted-artifact
  verification; hosted qualification is still pending

The repository can be used for compiler development; it is not a qualified 1.0
release. Follow
[docs/PRODUCTION_READINESS.md](PRODUCTION_READINESS.md) for the current acceptance
and follow-on roadmap.
