# ADR 0019: Explicit process exit status for executable programs

Status: Accepted and implemented (2026-10-09). The user approved the
recommended explicit process-status API.

## Problem

Sovra programs can read arguments and files and report application failures,
but `svr run` exits successfully when a program only prints an error. Generated
JavaScript has the same limitation. The Task Manager's direct commands are
therefore awkward to use in shell scripts: a failed save or malformed state
cannot be detected through the process status. This is a general executable
program contract, not a Task Manager primitive.

## Recommended contract

- Add `std::set_exit_code(code: Int) -> Unit`. It sets the eventual process
  status without ending execution. The initial status is 0; the last successful
  call wins. Valid codes are 0 through 125 inclusive on every supported host.
  An out-of-range code is a runtime error in both execution engines.
- `svr run` returns that status after successful program completion and after
  flushing already produced output. Generated JavaScript sets
  `process.exitCode` and allows ordinary completion and stream flushing.
- An uncaught runtime error remains a nonzero tool failure regardless of the
  last registered status. Compile/check errors keep their existing CLI behavior.
  The existing interpreter `run` and `run_with_host` APIs retain their
  output-only contracts; an additive status-returning API serves the CLI.
- The Task Manager sets status 1 for invalid commands, missing/duplicate titles,
  malformed files and I/O failures, leaving successful operations at 0.
  Output wording and the existing bare `print` alias remain unchanged.
- The builtin is registered under the exact `std::` name and participates in
  the established builtin-collision rule. It does not change `main`'s return
  type or reinterpret existing return values.

The 0–125 range is deliberately portable across POSIX and Windows, reserves
larger conventional shell statuses, and avoids host-specific truncation. It is
an application-selected status, not an exception or a way to bypass cleanup.

## Alternatives

- Interpret an `Int` result from `main` as the process status. This changes
  the meaning of existing non-Unit programs and requires a compatibility rule.
- Add `std::exit(code)` that terminates immediately. This can skip normal
  flushing and application cleanup, so it is less suitable for the current
  synchronous runtime.
- Leave error text as the only signal. Shell automation cannot reliably
  distinguish failures from successful output.

## Acceptance after approval

Test valid/invalid codes, last-call behavior, output before nonzero completion,
runtime errors after status selection, source diagnostics, interpreter host
isolation, generated JavaScript, Windows/Linux/macOS subprocess results, and
Task Manager failures through direct commands. Update the spec, stdlib lesson,
CLI reference, release notes and version-one gates. Preserve the existing
interpreter public APIs and all successful programs' default status 0.

## Approval boundary

AGENTS.md requires review before fundamental compatibility decisions. Process
exit status is externally observable behavior for every executable program and
adds a public standard-library name. The user approved this contract on
2026-10-09. Source, runtime, CLI and JavaScript tests cover the implementation.
