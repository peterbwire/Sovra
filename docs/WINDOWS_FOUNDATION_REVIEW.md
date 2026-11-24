# Windows foundation readiness review

Reviewed 2026-11-11 on Linux, branch `compiler/interface-string-budgets`, base
commit `a758fba` with substantial uncommitted compiler/UI/editor changes.
This is a focused source and readiness review, not an exhaustive security audit
or Windows release qualification.

**Current direction: qualify the foundations before further Task Manager feature
work.** The user explicitly set this sequence. Existing Task Manager regression
tests may run, but its application behavior stays on hold. Windows is the next
qualification target within the [broader platform goal](PLATFORM_VISION.md).

## Findings, ordered by priority

1. **Release blocker: current Windows execution and exact artifacts are unqualified.**
   The dirty working tree contains newer compiler, UI and IDE code than retained
   archives. Previous candidate results cannot qualify this snapshot.
   [CI](../.github/workflows/ci.yml) defines Windows compiler, WPF, installed
   toolchain and downloaded-artifact checks, but configuring a job is not execution
   evidence. This environment has no Windows/WPF runtime; attempting
   `node scripts/test-compiled-desktop.mjs` stops at `Compiled WPF adapter requires
   Windows`. Next: pass exact-commit MSVC checks and native desktop tests, rebuild
   packages, verify the downloaded packages, then qualify a clean Windows machine.

2. **Desktop qualification blocker: real accessibility/theme/DPI checks remain open.**
   The [compiled host](../toolchain/ui/windows/compiled/README.md) implements
   keyboard traversal, accessible names, high-contrast system colors, responsive
   event handling and process-tree cleanup. Its documentation explicitly leaves
   actual screen-reader, Windows theme-switching, DPI and clean-machine checks
   incomplete. Source-level tests and browser screenshots cannot close those
   gates. Exercise keyboard-only use, screen reader, scaling and multi-monitor
   transitions on the intended Windows baseline before declaring the UI ready.
   Native layout follow-up now constrains horizontal rows, wraps long labels and
   captions, preserves inputs across resize and fits initial size to the primary
   work area. Both actual C# host and acceptance harness compile against Microsoft
   WPF references as C# 5 on Linux; source-view parity passes in both engines.
   Native resize assertions are added to the existing Windows suite, not locally
   executed here. This does not close real Windows DPI/accessibility qualification.

3. **Opt-in replacement/recovery implemented; Windows lifecycle qualification remains.**
   [install.ps1](../toolchain/install.ps1) now accepts `-Upgrade` while retaining
   default no-overwrite behavior. It validates source/staged/current packages,
   checks the compiler before/after activation, retains backups and restores the
   old snapshot on activation failure. An explicit rollback helper validates
   target-bound recovery receipts. Unknown/modified files and links block upgrade;
   application data outside the prefix is untouched. Real PowerShell transaction
   tests pass on Linux; Windows 5.1, real packaged upgrade/rollback and clean-machine
   checks are wired into CI but remain unverified locally. Uninstall/removal,
   publisher authentication and power-loss/registry atomicity remain outside this
   slice. No backup is automatically deleted.

4. **Developer distribution implemented in source; Windows qualification pending.**
   Follow-up packaging now includes editor, LSP and extension sources under
   `developer-tools/`, an editor wrapper and a fixed-mode Node launcher using the
   adjacent compiler. The installer preserves them under its integrity manifest.
   A supplied VSIX is compared with bundled sources and identity; CI now builds
   and supplies it. Node 22+ is needed only for these optional tools. Relocated
   Linux acceptance passes full LSP and editor checks from a spaced installation
   path/unrelated cwd, including prerequisite/version failures. Installed Windows/
   PowerShell and downloaded-package tests are wired into CI but still need
   actual execution evidence.

5. **Host scope: browser development transport is not a desktop responsiveness model.**
   [shared/program.mjs](../toolchain/ui/shared/program.mjs) uses synchronous child
   execution with finite but larger build/run output buffers. A browser server
   using it blocks its event loop during execution. Native WPF already uses a
   different worker/job model; do not substitute the synchronous Node helper for
   that host. An interactive browser host still needs asynchronous execution,
   cancellation/lifecycle tests and appropriate UI transport limits.

6. **Compiler/tooling completeness remains partial.**
   Existing [security limits](COMPILER_SECURITY.md) do not establish complete
   process memory/CPU limits or a runtime sandbox; optional application body/service
   inspection is distinct from primary scan quotas. IDE navigation checks
   standalone snapshots and does not load package roots. Inferred-type hover,
   semantic completion and type/field navigation remain planned. Close these
   according to the intended developer workflow; do not treat every IDE feature
   as a prerequisite for executing a simple packaged Windows application.

## Readiness evidence and boundaries

| Area | Evidence available | Still required |
| --- | --- | --- |
| Compiler/runtime | Previous current-source Linux run: 570 Rust tests, formatting and strict Clippy; bounded frontend/AST/IR/diagnostic work | Current exact-commit Windows/MSVC execution, native/installed parity and supported-host qualification |
| Shared UI protocol | This review: 115 protocol admission checks; grouped/flat parity in both engines for existing app and unrelated component | Fresh native Windows execution of the changed bridge/profile/protocol tests |
| Shared browser renderer | This review: actual Chromium save/reset, grouped layout, literal markup/Unicode, stale-action rejection and narrow layout; screenshots inspected | Browser results do not establish WPF behavior |
| Editor/LSP/VS Code | Previous current-source Linux compiler/LSP and real VS Code 1.106 tests, including local shadowing navigation | Installed Windows tools, supported Node configuration and distribution compatibility |
| Windows UI/package | Source and CI coverage exist; compiled-host profile, UTF-8, action, timeout and process-job guards reviewed | Exact packages, Windows native interaction, real accessibility/DPI and clean-machine lifecycle evidence |

Review scope included compiler security/stage contracts and recent evidence,
shared Node protocol/transport, WPF bridge/profile/process-host boundaries,
toolchain/desktop packaging, installer behavior, CI and release gates. It did not
re-audit every compiler operation or run Windows-only code on Linux.

One reproducible test defect was fixed during review:
`test-ui-framework-browser.mjs` attempted a page-script fetch despite its own
`default-src 'none'` policy. The stale-action probe now uses Playwright's HTTP
client; the page policy and server admission assertion remain intact. Full browser
acceptance passes. Evidence is in `target/ui-framework/verification.json` and
the desktop/narrow screenshots. Task Manager source and behavior were not changed
by this review; earlier working-tree changes were preserved.

## Work order before Task Manager enhancements resume

1. Stabilize the reviewed compiler/shared-UI changes and obtain Windows/MSVC and
   compiled-WPF evidence for the exact source revision. Use the unrelated reusable
   UI component for foundation changes; retain existing application regressions.
2. Qualify the compiled host's responsiveness, failure recovery, keyboard/focus,
   screen-reader, contrast and scaling behavior on real Windows. Keep the
   PowerShell compatibility default until a separately reviewed migration is
   justified; this review does not change host defaults or public UI contracts.
3. Integrate developer-tool distribution and define a safe installation lifecycle;
   rebuild and verify the exact toolchain/desktop artifacts on a clean machine.
4. Record supported Windows/runtime prerequisites and remaining limits accurately.
   Once these foundations satisfy the applicable existing
   [release gates](V1_DELIVERY_GATES.md), resume Task Manager enhancements against
   that qualified foundation. No milestone renumbering or release approval is
   implied by this document.
