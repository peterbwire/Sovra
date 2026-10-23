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
responsive, nonzero exits stay visible, timeouts terminate the child, and a
malformed refreshed view blocks later stale mutations. These process fixtures
test the host; Task Manager and the reusable component still run through Sovra.

Mobile adapters, public lifecycle/navigation contracts, cancellation,
screen-reader/high-contrast/DPI qualification and clean-machine verification are
not complete. The PowerShell adapter remains available as a compatibility
baseline while the compiled adapter receives broader parity testing.
