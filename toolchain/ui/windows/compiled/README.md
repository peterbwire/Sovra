# Sovra UI compiled Windows adapter

This private adapter renders Sovra views directly in a compiled C#/WPF executable.
It does not launch PowerShell, a browser, Node.js or a local web server.
Application rules, interface descriptions, events and persistence execute in the
adjacent native `application.exe`, compiled from Sovra.

Extract the complete package and open **Sovra UI.exe**. Keep `application.exe`
and `app.json` beside it. Compatible Windows x64 and .NET Framework/WPF are
required. Rust, Node.js, PowerShell and the Sovra compiler are not application
runtime prerequisites. This is a managed Windows host, not a new native-code
backend or a completed cross-platform UI framework.

The existing private profile chooses a stable application ID. Data remains at
`%LOCALAPPDATA%\Sovra\Desktop\<id>\state.txt`, preserving the previous adapter's
location and Sovra file format. Do not run two writers against the same data.
The host never interprets task rows or implements task-management operations.

Compiled hosts now acquire an exclusive `.desktop.lock` handle in the profile
directory before launching the application. A second cooperating host for that
same directory shows a startup error and exits nonzero. Different profiles are
independent. Closing releases the handle; process termination also releases it.
The empty marker stays on disk and must not be deleted as a way to unlock a
running profile. A leftover marker alone is not a stale lock.

This protects cooperating compiled desktop hosts only. CLI, browser and older
PowerShell hosts do not acquire this lease and must not write the same data
concurrently. Before starting children, the host joins a private Windows job
with kill-on-close and no breakaway. Its non-inheritable handle belongs to the
host for the process lifetime; Windows terminates descendants on normal exit
or forced termination. If the guard cannot be installed, startup fails before
application execution. This requires nested-job support on Windows 8 or newer.
The lease itself is not inherited: shutdown is not an atomic handoff between
writers, nor a transaction/rollback guarantee.

Each invocation re-enters this executable as a private worker. Before starting
the application, the worker creates its own nested kill-on-close job. Timeout
or output-limit failure terminates the worker and its application descendants,
leaving the GUI alive. Worker exit also cleans up descendants left after normal
application exit. Output bytes and exit status pass through unchanged; no shell
or additional installed executable is involved. This internal mode is not a
public Sovra CLI or cancellation API. Committed writes are not rolled back.

The current Task Manager payload uses approved `std::write_text_if` for task,
filter and page saves. Cooperating current CLI/browser/desktop programs reject
stale or busy writes. This host does not add locking to arbitrary applications.
A crash or cleanup failure can leave `<file>.svr-write-lock`; stop all writers,
inspect the data, then deliberately remove that marker before retrying. Never
delete it based only on age. An old app, unconditional write or external editor
does not participate. Keep state in a private local directory; conditional writes
reject symlink destinations and do not form multi-file transactions.

Both `SVR-UI-1` and opt-in `SVR-UI-2` are supported: headings, text, labelled
inputs, actions, vertical stacks, wrapping rows and card/plain groups. Existing
limits remain: strict UTF-8, 64 KiB per stream, 256 leaf nodes, 256 groups, eight
group levels, 30 seconds per child invocation and one event at a time. The latest
Sovra view authorizes each event. Timeouts are not retried because a write may
already have committed. Closing during an event waits for success; errors keep
the window open. Rejected submissions retain drafts. Unchanged action controls
retain drafts, focus and selection; removed or kind-changed controls discard them.

Status, accessible names, Tab order, unmodified Enter and Escape remain host
behavior. The compiled renderer uses Windows system colors in high-contrast mode
and refreshes its palette when Windows reports a high-contrast change. Theme
updates do not send application events or replace input controls. Layout rounding
and pixel snapping are enabled. Tests exercise the system-color palette without
changing the user's Windows settings; actual OS theme switching and manual
assistive-technology/DPI qualification remain separate checks.

The compiled acceptance suite also covers task rename, combined search/status
filters, keyboard submission, forward/reverse focus traversal, contextual action
names, and controls moving between or changing groups. Removed or kind-changed
controls never inherit discarded drafts. `target/compiled-desktop/verification.json`
records pass/fail status, completed checks, compiler/source hashes and optional
package executable hashes. Failed cleanup leaves a failed report.

Startup profiles are read into a bounded snapshot (16 KiB maximum); schema,
application identity and view command are validated before execution. Startup
failures display a diagnostic and return nonzero status. Tests cover malformed
profiles and missing application executables without changing existing state.
Controlled transport fixtures also verify that slow work leaves the dispatcher
responsive, nonzero exits stay visible, timeouts terminate invocation trees, and a
malformed refreshed view blocks later stale mutations. These process fixtures
test the host; Task Manager and the reusable component still run through Sovra.

Mobile adapters, public lifecycle/navigation contracts, cancellation,
screen-reader/high-contrast/DPI qualification and clean-machine verification are
not complete. The PowerShell adapter remains available as a compatibility
baseline while the compiled adapter receives broader parity testing.
