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
are enabled on Windows and Linux; application syntax and other-host writes
remain deferred. See [formatting](reference/formatter.md). Registry commands and a public
`svr editor` CLI remain planned. An experimental browser editor, stdio LSP and
VS Code extension now exist under `toolchain/editor` and `toolchain/ide`; see
[IDE integration](../toolchain/ide/README.md) and the
[current maturity audit](LANGUAGE_MATURITY_AUDIT.md).

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

G12 now extends package formatting to registered inventory sources and implements
[explicit local source locks](reference/package-source-locks.md) on guarded Linux
hosts. Full registered context/overlay/navigation/editor integration and richer
source diagnostics remain open. New public contracts need concrete proposals
before implementation. Preserve existing
commands and the trusted Rust compiler/runtime boundaries.

G09 initial foundation checkpoint (2026-11-12; superseded by later checkpoints below): the compiler now exposes bounded borrowed
package entry-source overlays without disk writes. This is an API foundation,
not activated package-aware LSP/editor behavior. CLI `check --format json --overlays-stdin` now carries bounded framed requests
and correlated JSON reports. Manifest/new-file overlays, LSP dependency generations
and cross-file navigation remain pending; see [ADR 0042](adr/0042-unsaved-package-sources.md).

Workspace diagnostics checkpoint (2026-11-12): initialized workspace folders now
use compiler-owned package context and unsaved-buffer checks. Dependency edits,
close/rejection and source/manifest notifications conservatively invalidate all
workspace generations. Diagnostics preserve open-file URI/version and UTF-16
ownership; closed dependencies produce explicit summaries. Single-entry local hover remains available. Imported function hover/definition/
outlines now use checked targets and fingerprint-verified source; package editor
tasks and package-specific host acceptance remain pending. See the [IDE limits](../toolchain/ide/README.md) and ADR 0042.

## Package task qualification (2026-11-12)

The VS Code package workflow now resolves compiler-owned roots/cwd and saves
related graph sources, refusing dirty manifests. Package Check explicitly uses
full entry compilation. See [task behavior](reference/editor-package-tasks.md),
including VS Code’s independent `task.saveBeforeRun` preference. Linux real-host
and relocated VSIX/source-byte qualification pass; other platform and release
gates remain separate.

`svr fmt --package --check DIRECTORY` performs bounded read-only formatting
checks of compiler-selected entries across the local dependency graph, including
dependencies outside the root. It never writes sources or manifests. See the
[formatter reference](reference/formatter.md) for quotas and conflict checks.

`svr fmt --package --write DIRECTORY` now applies the same bounded graph on
Windows/Linux, including dependencies outside the root. It preflights and stages
all selected entries before sequential per-file replacement. Late failures report
partial completion; earlier replacements remain. Manifests are never written.
Linux execution qualification does not establish new Windows/macOS evidence.

Formatter host evidence is reproducible with `cargo build --locked` followed by
`node scripts/qualify-formatter.mjs`. CI retains per-host JSON and command logs
under `target/formatter/`, even when qualification fails. Missing/wrong compiler
regressions prohibit false success. Inspect actual reports rather than treating
CI configuration as platform evidence; macOS currently verifies disabled writes.
See the [formatter contract](reference/formatter.md#reproducible-host-evidence).
