# Developer tooling implementation

Private compiled desktop qualification: `node scripts/test-compiled-desktop.mjs`
compiles and exercises native WPF controls through real Sovra execution without
a PowerShell test host. Add `--package DIRECTORY` to test the compiled desktop
EXE with the adjacent native Sovra application under restricted PATH, including
persistence across restarts. The package test needs access to one fresh test
profile under LocalAppData, which it removes. See
[compiled adapter](../toolchain/ui/windows/compiled/README.md).
The runner writes `target/compiled-desktop/verification.json` with distinct
running/passed/failed status, completed check count, compiler/source hashes and
optional packaged host/application hashes. Cleanup failure prevents success.

Updated 2026-10-21. Implemented public commands include `new`, `check`, `run`,
`build` with IR/JS/host-native targets, `test`, `doctor` and standalone `fmt`.
JSON reports and installation/native checks support automation. Formatter writes
are enabled on Windows; imports/application syntax and non-Windows writes remain
deferred. See [formatting](reference/formatter.md). Registry/editor commands remain planned.

For separate local Windows MSVC qualification, run `node scripts/test-msvc.mjs`.
It retains compiler hashes, stage logs and pass/fail evidence without changing
the GNU default. See [MSVC qualification](guides/msvc-qualification.md).

## Reusable named-function execution foundation

Rust tooling can use
`compiler::interpreter::run_function_with_host_status(program, name, host)`
to execute a zero-argument lowered user function from checked IR. It uses the
existing interpreter and declaration/frame validation. Original `main`, helpers
and IR stay unchanged; each invocation starts new frames and exit status.
The caller owns host effects, and return values are discarded.

This is a trusted Rust API, not source-level visibility lookup or a new Sovra
call syntax. The ordinary CLI/native application entry remains `main`.

Internal probe from the development checkout:

```text
cargo run --example function_probe -- examples/local-packages/app main
```

The probe compiles/checks the source or package, selects the explicit lowered
function and forwards subsequent arguments to the process host. It is internal
tooling, not a new `svr` command or a test discovery convention.

## Implemented language test runner

[ADR 0028](adr/0028-language-test-command.md) implements `svr test`, with ordinary
root `test_` functions, Unit signatures, sequential isolated frames, explicit
failure status and human/JSON reporting. See the [testing guide](course/testing.md) and [JSON reference](reference/test-json.md) for invocation, effects and report contracts.

Subsequent tooling should extend package formatting, add reproducible package
locks, richer source diagnostics and editor/debugger integration. Their public
contracts need concrete proposals before implementation. Preserve existing
commands and the trusted Rust compiler/runtime boundaries.
