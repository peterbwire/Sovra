# G13 implementation and qualification

Status (2026-11-23): consolidated Linux compiler/runtime/LSP qualification now
passes on the current frozen source: **2,273 Rust tests**, **181 integration
targets**, **16 Node scripts** and **204 commands**, including formatting,
all-target/all-feature Clippy and exact Rust 1.74.0. All commands passed and the
source fingerprint remained unchanged. This supersedes the formatter-era full
run for compiler/LSP evidence, including the newer package tasks, unsaved graph
checks and function/type/local/field navigation.

Subsequent IDE-only changes pass all 16 Node scripts and all six actual Linux
VS Code 1.106.0 headless Electron modes, including generic packages and restricted
workspaces. See [host acceptance](reference/generic-package-ide-host.md). The host
checkpoint fixes Field outline mapping and updates acceptance infrastructure;
Rust compiler/runtime source is unchanged. It has its own frozen source report in
`target/checked-evidence/g13-vscode-host/verification.json`; the full Rust report
below remains the preceding compiler/runtime checkpoint, not a second full run.

G13 is complete for the documented experimental Linux scope. Additional OS/runtime,
browser/mobile, default-profile expansion and release gates remain separate.
The [completion contract](design/G13_COMPLETION_CONTRACT.md) remains authoritative.

The complete report and per-command logs are retained in
`target/checked-evidence/g13-package-navigation-consolidated/verification.json`.
The older formatter and targeted checkpoint reports are preserved separately.

```text
source_sha256 = source_sha256_end =
7ba5866b4d89fcd4fe779cbe1677930120c68a06d0f059c9faea8ec37e36e4d1
```

| Requested area | Implementation and acceptance coverage |
| --- | --- |
| Strict JSON | Ordered recursive values, signed exact integers, finite floats, strict Unicode/UTF-8/numbers, decoded duplicate-key refusal, original byte offsets, bounded parse/stringify and source Result admission. |
| Data CLI/native | Explicit package check/run/test/build; exact word-2047 images; independent native launch after project deletion, cleared environment and unavailable Node/PATH; old profiles retain strict admission. |
| Filesystem/time/process | Version-3 deny-by-default policy, captured separate read/write roots, no-follow traversal, bounded guarded operations, Gregorian dates, explicit clocks/timers, pinned ELF executable and fixed cwd/environment, owned streams/child, cancellation/deadlines, kill/reap and cleanup error reporting. |
| Consolidated qualification | Reproducible source-fingerprinted Rust/Node/MSRV harness and retained per-command logs; focused Linux CI artifact job alongside existing full multi-OS checks. |

The [data and host reference](reference/data-host-v3.md) defines the supported
commands, requests, policy schema, limits and host restrictions. Current evidence
does not establish Windows runtime execution, browser/mobile parity, an optimized
machine-code backend, a public package registry or HTTP/server support. Those
remain tracked in the established G11/G12/G14–G18 gates.

## Current package navigation scope

Checked package navigation now covers function calls, declaration/local type
annotations, explicit call/constructor types, parameters/locals/payload bindings,
and user-record field reads and constructor labels. The linked checkpoint logs
below retain their original test scopes. Newer slices supersede the older lists
of missing navigation contexts. Consolidated Linux compiler/LSP qualification
and actual Linux VS Code host acceptance pass; additional host qualification remains open.

## Earlier checked package type annotations

[Package type annotation navigation](reference/generic-package-type-navigation.md)
now preserves record/enum/alias definitions and references in function signatures,
record fields, enum payloads and alias targets. Scoped registry resolution preserves
written aliases, imported ownership and private/import boundaries; original
annotation fragments and terminal token ranges are validated before publication.

Targeted checks pass 145 distinct Rust tests, six Node regression scripts, stable
formatting, all-target/all-feature Clippy and exact Rust 1.74.0. Evidence is retained
in `target/checked-evidence/g13-generic-package-type-navigation/verification.json`
and adjacent logs. This is not a new full-suite or Electron/additional-host run.
At that checkpoint body type/field/local references remained open. Later slices
below implement them and the current full run qualifies their Linux compiler/LSP behavior.

## Newer checked package function navigation

[Generic package function navigation](reference/generic-package-navigation.md) now
projects imported/same-scope function calls from canonical checked signature
bindings, with original headers and source fingerprints. Real LSP tests verify
function definitions/hover/outlines, mismatch refusal and clearing facts after a
failed check. Rust tests separate identical names in different source files,
reject inaccessible private calls and refuse optional quotas without turning a
valid check into an error. Type/field/local package references remain open.

Targeted qualification passes 745 distinct Rust tests and six Node regression
scripts, stable all-target checking, all-target/all-feature Clippy and exact Rust
1.74.0. Evidence is retained in
`target/checked-evidence/g13-generic-package-navigation/verification.json` and
adjacent logs, separately from the earlier full-suite checkpoint;
actual Electron/additional-host acceptance was not run for this slice.

## Newer unsaved generic package diagnostics

[Generic package diagnostics](reference/generic-package-diagnostics.md) now check
unsaved entry/module/dependency buffers through guarded capture and bounded logical
overlays. The check-only API returns no executable/capture/lock authority. LSP
results preserve original-file ownership, validate UTF-8 byte boundaries and
invalidate on related changes, close/rejection and disk/watch events. Standalone
symbol facts are cleared in checked package graphs; generic package function navigation is now available as described above;
type/field/local references remain open. The package-task setting remains independent.

Targeted validation passes 730 Rust tests: 600 library, 77 CLI, 16 generic CLI,
18 package-image regressions, eight package-compile tests, four context tests,
four new overlay tests and three wire-decoder tests. Six real Node regression
scripts cover the new generic flow and existing LSP/package/navigation/task/
installed-tool behavior. Stable all-target checking, all-target/all-feature Clippy, formatting and exact
Rust 1.74.0 checks pass. Evidence is retained in
`target/checked-evidence/g13-generic-package-diagnostics/verification.json` and
adjacent logs. Actual Electron host acceptance was not run. The earlier full-suite result below is not a new full rerun.

## Newer generic package-task integration

The next integration slice adds opt-in compiler-selected generic package Check/Run
through LSP/VS Code. See [generic package tasks](reference/generic-package-tasks.md).
Metadata selection includes registered modules/dependencies without claiming a
source check. Versioned context, source-only saves, dirty-manifest refusal,
unregistered-source refusal and broken-graph refusal are tested through the real
compiler/LSP and task preparer. Package diagnostics/navigation remain standalone.
Targeted qualification passes **97 Rust tests** (77 CLI, 16 generic CLI and four
new package-context tests), **seven Node regression scripts**, all-target checking,
all-target/all-feature Clippy, stable formatting and exact Rust 1.74.0. Evidence
is retained in `target/checked-evidence/g13-generic-package-tasks/verification.json`
and adjacent logs. Actual Electron host acceptance was not run. The full result
below belongs to the preceding formatter checkpoint and is not a full rerun of
these newer changes.

## Full qualification at the formatter checkpoint

The updated Linux x86_64 source passed **2,258 Rust tests** across the library,
all **179 integration targets** and doctests, plus **15 Node scripts** covering
compiler, editor, LSP, symbols, VSIX admission and installed developer tools.
All **201 commands** passed, including stable formatting, all-target checking,
all-target/all-feature Clippy, exact Rust **1.74.0** and the CLI build.
The formatted data consumer passed locked check/test/run/build and actual native
launch after source deletion with an empty environment and unavailable Node/PATH.

The compiler/tooling/test/example/CI fingerprint remained unchanged throughout:
`4f7a70f3548ebbbfe9b030e39da5733bfb226e0ad2842493df7aaacb02da0564`.
Evidence is retained in `target/checked-evidence/g13-closure/verification.json`
and adjacent per-command logs. Fresh all-target ARM Linux and Windows GNU
cross-checks passed; their logs are retained alongside the report. These are
compile checks, not runtime acceptance on those hosts. Optional VS Code Electron
acceptance was not run and is explicitly recorded in the report. CI is configured
for x86_64 and ARM Linux; its upstream run remains separate evidence.

The formatter gap and consolidated Linux regression gap are closed. The checklist
still tracks broader E1 package/backend/tooling integration, including package
editor contexts and remaining type/field symbol coverage. Those capabilities
must not be inferred from successful standalone formatting. Additional host,
browser/mobile, registry, HTTP and release gates retain their existing task IDs.

## Earlier focused result (before generic formatting)

At the preceding Linux x86_64 checkpoint, source passed 926 Rust tests (library, ten focused
integration targets and doctests), stable formatting, all-target checking,
all-target/all-feature Clippy, exact Rust 1.74.0, native source-deletion consumers
and 15 Node compiler/editor/LSP/developer-tool scripts. All 32 commands passed.
The compiler/test/runtime source fingerprint remained unchanged:
`853541e56040de84f459407300d2e0a31294b6c4298b50be54bb13bca3d8bd42`.
That report is archived at `target/checked-evidence/g13-pre-format-focused/verification.json`;
current qualification writes to `target/checked-evidence/g13-closure/`. Documentation links and CI/script syntax also pass.
This focused result does not turn the earlier changing-source broad run into a
frozen-source full qualification, or resolve the formatter gap below.

## Resource and authority review

Compiler-owned standard identities and independently checked IR shapes prevent
ordinary source records/enums from impersonating Byte, Bytes, JSON or Result.
Old layout/image profiles reject data rather than silently weakening checks.
The source profile shares VM work/control and preflights complete owned results
before publication. JSON helper trees/output and host I/O staging have separate
finite ceilings; logical node/text/work counters are not a hard heap/RSS limit.

Host grants are supplied by the trusted embedding application or an explicit
external CLI policy. Source and native images cannot manufacture grants. Pure
date/path operations need none. Clock/timer authority is separate from filesystem
and executable authority. External executable grants convey ordinary OS process
authority and must be reviewed accordingly; they are not confined by the API's
read/write directory grants.

Handle-rooted traversal and final no-follow checks prevent symlink-following
escapes for supported operations. External tree mutation and existing hard links
remain deployment considerations. Cancellation does not roll back completed
filesystem effects. File synchronization and kernel calls can block independently
of cooperative VM checks: these are not hard real-time deadlines.

Child ownership is synchronous and private to an invocation. Early control,
timeout and stream-limit refusals trigger kill/wait; normal completion reaps the
child. The observed-PID test verifies that cancellation leaves no owned child in
procfs. Cleanup errors remain visible, including through the VM boundary.
Trusted embedders must not reap these children independently. Descendant
termination, cgroups/seccomp and hard process CPU/memory limits are not promised.
Networking, TLS and asynchronous scheduling remain G14.

## Qualification procedure

Run:

```sh
python scripts/qualify-g13.py --all-rust --node
```

The harness checks stable formatting, all-target checking, all-target/all-feature
Clippy, library tests, every Rust integration target, doctests, exact Rust 1.74.0,
CLI build and compiler/editor/LSP/developer-tool Node regressions. It retains
logs and `verification.json` in `target/checked-evidence/g13-closure/`, records
the source fingerprint before/after, and refuses success if it changes. It only
removes known generated passed-test executables to bound disk consumption.

Native/process tests require permission to launch local executables via procfs.
Execution restrictions in a hosted sandbox must not be reported as a passing
native qualification. A Windows GNU all-target cross-check passed locally; this
is compile evidence only. The CI workflow itself still needs an upstream run.

Optional VS Code Electron host acceptance requires `@vscode/test-electron` and a
usable display/headless host. Select it with `--vscode-host`; the report explicitly
records when it was not run. Portable Node regression success does not establish
Electron host acceptance.

The broader run recorded 2,253 passing Rust tests, including every integration
target, and passed exact Rust 1.74.0 before stopping at the unavailable optional
Electron dependency. Its logs are retained separately under
`target/checked-evidence/g13-broad-before-arm-fix/`. During that run, a portability
review identified and fixed AArch64 no-follow/directory flag values in the host
adapter and guarded package-source reader. Its source fingerprint therefore
changed: it is not a final frozen-source qualification. A fresh focused run
checks the final source and includes final-symlink outside-file preservation,
root races, child cleanup, data/native consumers and package capture. ARM CI is
configured to exercise these behaviors on actual ARM Linux; it has not run locally.
An all-target `aarch64-unknown-linux-gnu` cross-check passed after the flag fix;
this establishes compilation, not ARM runtime behavior.

Closure requires reviewing the final report and resolving any failed command;
historical passing logs or this implementation inventory cannot substitute for it.

## Generic/data formatter integration

[Generic/data formatting](reference/generic-formatting.md) is implemented through
`fmt --experimental-generics`, including package graphs, propagation and standard
data annotations. CLI tests pass for fixed points, comments/literals, structural
refusal, guarded writes, all-source preflight and locked consumer behavior. LSP
formatting returns edits without workspace writes or execution and retires stale
requests; VS Code registration uses this path. The native data consumer now
formats before lock/build. Full frozen-source Rust/Node/MSRV qualification passed as recorded above; the
earlier formatter rejection is resolved. New Electron/Windows/ARM
runtime evidence remains separate, with no release or cross-platform claim.

Package local-navigation checkpoint: [checked package bindings](reference/generic-package-local-navigation.md) now supply typed hover and definitions for parameters and local/payload bindings, with per-function ownership, source fingerprints and whole-batch quota refusal. Package body type-use/field navigation and consolidated qualification remain open.

Explicit package local-type checkpoint: checked `let` annotation ranges now supply alias-preserving and dependency-aware type hover/definitions, including nested arrays, through the sealed function scope. Constructor/call type arguments, field navigation and consolidated G13 qualification remain open. See [package type navigation](reference/generic-package-type-navigation.md).

Package expression-type checkpoint: explicit call arguments and record/enum constructor heads and arguments now supply checked scoped hover/definitions while preserving alias destinations and source offsets. Field navigation and consolidated G13 qualification remain open. See [package type navigation](reference/generic-package-type-navigation.md).

Package field checkpoint: [checked field navigation](reference/generic-package-field-navigation.md) now supplies field outlines, declaration hover and definitions for user-record reads and constructor labels through exact nominal/field identities. Grouped/chained reads, same-named package fields, reordered labels and whole-batch quotas are covered. Consolidated G13 and external IDE/host qualification remain open.
