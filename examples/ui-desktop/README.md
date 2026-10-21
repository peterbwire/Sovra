# Reusable Sovra desktop component

This small message component exercises the same private `SVR-UI-1` view and event
adapter as the Task Manager, with unrelated state and actions. Sovra defines the
heading, text, labelled input, buttons, validation and persistence. The desktop
host creates controls and forwards events without knowing these application rules.

Run it through the actual interpreter from the repository root:

```powershell
.\target\release\svr.exe check examples/ui-desktop/component.svr
.\target\release\svr.exe run examples/ui-desktop/component.svr -- ui-view .\message.txt
.\target\release\svr.exe run examples/ui-desktop/component.svr -- ui-event .\message.txt save-message 'Hello from Sovra'
.\target\release\svr.exe run examples/ui-desktop/component.svr -- ui-view .\message.txt
.\target\release\svr.exe run examples/ui-desktop/component.svr -- ui-event .\message.txt reset-message
```

`ui-view STATE` emits a complete view. `ui-event STATE save-message VALUE` saves
the message; `ui-event STATE reset-message` saves the default. A successful event
emits a notice; the host obtains the new view with a separate `ui-view` call.
Restarting the host retains the saved message. A missing file displays
`Ready from Sovra` without creating a file.

Messages must contain 1 to 1024 UTF-8 bytes and no C0, DEL or C1 controls. Unicode,
quotes, backslashes, pipes and shell metacharacters are literal data. Argument
errors, unsupported actions, invalid stored text and file failures return status
1. Invalid input never replaces existing state; reset also refuses corrupt state.
The existing bounded UTF-8 read and atomic replacement write APIs perform storage.

This is an internal adapter fixture, not a published UI package or new language
syntax. Its fixed four-node view remains within the private 64 KiB/256-node
protocol bounds. It has no history, concurrent writer coordination, navigation,
custom styling or asynchronous state. Standalone builds use the existing native
build command and require the documented developer build tools. Windows desktop
execution additionally requires the private PowerShell/WPF host; this source
alone does not create a window.
# Grouped desktop view

The same component optionally supports `ui-layout-view STATE` and emits a private
`SVR-UI-2` message card. `ui-view STATE` retains its existing flat output. Select
`-ViewCommand ui-layout-view` in the development host to exercise grouped rendering.
The generic renderer and event path are shared with Task Manager; this is not a
published UI package or compatibility contract.
