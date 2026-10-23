# Sovra UI Windows desktop prototype

Compiled-host migration (2026-10-23): the internal packager now accepts
`-CompiledHost` to produce a direct C#/WPF executable with no PowerShell runtime
scripts or shell child. Application behavior still executes in Sovra. See the
[compiled adapter](compiled/README.md) and [ADR 0033](../../../docs/adr/0033-compiled-desktop-host.md)
for dependencies, tests and remaining qualification. The description below
documents the retained PowerShell compatibility baseline.

This private reusable renderer turns the current `SVR-UI-1` Sovra view into WPF
headings, text, labelled inputs and buttons. It opens a desktop window, with no
browser, local HTTP server or Node process. Application rules and persistence
execute in the Sovra program through `ui-view` and `ui-event`.

The prototype requires compatible Windows x64 with Windows PowerShell 5.1 and
.NET Framework/WPF. A packaged native Sovra application needs neither Rust,
Node nor the Sovra compiler on the user's machine. The small C# EXE launches
the platform host; it does not implement application rules or compiler behavior.
This is an internal Windows adapter, not a published cross-platform framework.

For a packaged app, extract the entire ZIP and double-click **Sovra UI.exe**.
Keep `application.exe`, `app.json` and the `ui` folder beside it. The window title
and controls are supplied by the Sovra view. A profile's data lives under
`%LOCALAPPDATA%\Sovra\Desktop\<id>\state.txt`. Existing CLI task files are not
moved or deleted. Close the window to exit the host. Startup errors appear in a
dialog. The CLI's no-argument behavior is unchanged.

For development, from the Sovra repository root:

```powershell
powershell -NoProfile -STA -ExecutionPolicy Bypass -File toolchain/ui/windows/host.ps1 -Program target/release/svr.exe -Source dogfood/01-task-manager/main.svr -State tasks.txt
```

The same renderer accepts the unrelated `examples/ui-desktop/component.svr`
through `-Source`. Native distribution instead passes an existing compiled
program with `-Program`; `-Source` is only the interpreter development bridge.
Treat sources and native programs as trusted executable code.

Application events run in a background PowerShell runspace. The dispatcher owns
all native controls; the worker receives only program/state/event data. While
busy, forms are disabled and the status says Working; the window and scrolling
remain responsive. A second event is rejected rather than queued. Closing waits
for the current event to finish once, then closes on success. Failures keep the
window open with an error. There is no cancellation/rollback or automatic retry;
a timeout can leave an uncertain application outcome, so inspect state before retrying.
Initial view loading still happens before the window opens.

Optional private grouped views put layout choices in Sovra. An opted-in profile
sets `view_command` to `ui-layout-view`; other profiles continue using `ui-view`
without probing unknown commands. The private `SVR-UI-2` stream adds keyed
vertical stacks, horizontal wrapping rows and card/plain panels. Task Manager
emits its input and task-list cards and one action row per task. The host has
no task-specific grouping rules. The unrelated message example uses the same
renderer. Existing `SVR-UI-1` browser/desktop views remain available.

Limits: 64 KiB per output stream, 256 leaf nodes, at most 256 groups/eight nested
levels in v2 (768 total records), 30-second timeout per child
invocation, serial events per window, fresh process/view per event. Unchanged action IDs and
control kinds retain their native controls, drafts and text selection. Updates
restore keyboard focus and scroll position. Only a successfully submitted input
is cleared; failed submissions retain the text. Removed fields discard drafts,
and a different control kind does not inherit an old field's state. Duplicate
action IDs are rejected. These are private renderer rules, not a published UI ABI.
Large Task Manager views may exceed
the protocol bounds. Startup/state failures do not overwrite the state file.
This is not the final layout, navigation or public async UI model.
Enter submits the focused input when no modifier is pressed; key repeats and
busy submissions cannot start duplicate work. Escape clears only the focused
draft without invoking Sovra or changing persisted state. Tab and Shift+Tab follow view order,
including each input's submit button, and cycle within the window. Buttons expose
contextual accessibility names derived from Sovra text, distinguishing repeated
actions. Status names reflect working, completion and errors; compatible WPF hosts
also announce changes through a polite live region. These naming defaults are
private renderer behavior, not a public accessibility metadata contract.
Native control labels expose accessibility names and normal keyboard behavior;
screen-reader, high-contrast, DPI and clean-Windows qualification remain pending.
No public UI ABI or new language semantics are introduced.
