# Sovra UI Windows desktop prototype

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

Limits: 64 KiB per output stream, 256 flat nodes, 30-second child-operation
timeout, serial events, fresh process/view per event. Re-rendering currently
rebuilds controls and clears input/focus. Large Task Manager views may exceed
the protocol bounds. Startup/state failures do not overwrite the state file.
This is not the final layout, navigation, async or focus-preserving UI model.
Native control labels expose accessibility names and normal keyboard behavior;
screen-reader, high-contrast, DPI and clean-Windows qualification remain pending.
No public UI ABI or new language semantics are introduced.
