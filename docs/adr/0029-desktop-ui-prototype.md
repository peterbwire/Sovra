# ADR 0029: Private Windows desktop UI prototype

Status: **Private implementation direction authorized (2026-10-15).** The user
requested that Sovra UI power the Task Manager in a desktop window. WPF is
selected for this internal Windows prototype. Implementation evidence must be
recorded separately; this decision does not approve the public UI contract in
[ADR 0023](0023-unified-ui-boundary.md).

## Context and inspected evidence

The existing Task Manager GUI is a Node-hosted browser application in
`dogfood/01-task-manager/gui/server.mjs`. It is not a desktop window. The native
executable produced under [ADR 0027](0027-native-build-cli.md) is a CLI; opening
it without arguments prints usage and exits. A terminal launcher does not
satisfy the user's desktop UI request.

The Task Manager already defines its interface and event behavior in Sovra.
`dogfood/01-task-manager/main.svr` emits a private `SVR-UI-1` view through
`ui-view STATE` and handles `ui-event STATE ACTION [VALUE]`. Headings, text,
Add inputs, Complete/Delete actions, task validation and persistence come from
Sovra. The existing generic decoder in `gui/ui-view.mjs` provides the current
64 KiB, 256-node bounds and heading/text/button/input grammar. These are useful
prototype inputs, not a published framework ABI.

There is no implemented native widget renderer or mobile toolchain in the
existing compiler. [ADR 0025](0025-standalone-executable-strategy.md) approved
the standalone CLI direction without selecting a GUI toolkit. Extending that
CLI packaging alone cannot supply a desktop interface.

## Private implementation decision

Use Windows Presentation Foundation through Windows PowerShell and .NET for
the smallest genuine desktop window. Keep reusable renderer functions in
`toolchain/ui/windows/renderer.ps1` and process/window startup in the adjacent
`host.ps1`. This adds no Rust dependencies, language syntax, compiler target,
public host API or runtime ABI. It does not launch a browser, require Node,
or introduce an HTTP server.

The renderer consumes the existing bounded private stream and creates native
WPF controls. Headings/text become text controls; buttons expose application
action identifiers; an input becomes a labelled text field and submit button.
The renderer owns generic layout, control creation, keyboard interaction,
focus and status display. It must not recognize task IDs, task commands,
storage rows or application-specific labels.

During development the host may invoke the real interpreter. The distribution
path invokes the existing standalone Sovra executable directly. Both paths
request `ui-view`, dispatch `ui-event`, then obtain a fresh view. An unrelated
Sovra component in `examples/ui-desktop/component.svr` must use the same
renderer and host. Source parsing, view construction and business execution
remain in the actual Sovra toolchain; the desktop host does not emulate Sovra.

Application state and file writes remain in Sovra. The host passes one
configured state path rather than accepting a path from a control. The
Task Manager retains its existing file format and single-writer limitations.
Host controls retain only temporary input/focus and the latest validated view.
Process startup cost is accepted for the first slice; persistent application
sessions and asynchronous execution need later design and measurement.

## Transport, validation and failure boundaries

- Validate the complete UTF-8 stream, version, node count, node grammar, text
  and action keys before rendering. Enforce the existing 64 KiB/256-node
  bounds. Reject unknown or malformed nodes rather than partially rendering.
- Treat view text, labels, file paths and input as data. Do not evaluate
  generated PowerShell, command-shell source or XAML. Launch a specified
  executable directly and preserve argument boundaries. On Windows PowerShell
  5.1, any required Windows argument encoding must correctly preserve quotes
  and backslashes; joining argument strings is insufficient.
- Serialize event dispatch and disable conflicting controls while an event
  runs. Only actions represented by the validated view may be dispatched;
  Sovra must still validate stale or invalid events. Do not retry a mutation
  automatically after an uncertain process result.
- Bound process output and execution, retain useful diagnostics, and surface
  launch failures, invalid views, rejected events and persistence errors in
  the window. A failed operation must not be displayed as success. Do not
  replace corrupt stored data with an empty successful view.
- Set accessible names from Sovra labels, preserve logical keyboard order and
  expose visible status/error messages. Verify these through the actual WPF
  controls. The current stream has no complete accessibility schema; full
  screen-reader, heading, live-region and focus behavior remain to qualify.

## Platform and distribution limits

This prototype requires compatible Windows, Windows PowerShell and WPF/.NET.
The packaged native application does not require Node, Rust or the Sovra
compiler at execution time, but the desktop host has those Windows platform
dependencies. Detect missing host components and show an actionable failure.
Document every launcher, renderer script and native executable required by a
desktop package; a CLI executable alone is not a desktop application package.

Keep the working CLI and browser GUI available. Do not change the native
compiler command, no-argument CLI behavior or standard library merely to
launch this adapter. Do not advertise Linux, macOS, Android, iOS, WebAssembly,
a public Sovra UI package or a general native desktop backend from this work.
The current node limit also restricts how many Task Manager rows this view can
display; a scrolling window does not remove the protocol bound.

## Acceptance and staged implementation

1. Decode valid/invalid fixture views and create native controls without
   application-specific renderer branches. Verify literal text, Unicode,
   labels, keyboard access and invalid-view rejection.
2. Run the unrelated Sovra component through the interpreter and standalone
   executable. Drive a WPF event through the same event handler used by a
   user click and verify the resulting view through real Sovra execution.
3. Render Task Manager, add a task, complete it and delete it through the
   shared desktop renderer. Restart the host and verify persisted state.
   Include quotes, backslashes and shell metacharacters in input to prove
   argument transport preserves data.
4. Exercise startup failure, application error, corrupt state, stale events,
   malformed/oversized views and process bounds. Verify errors remain visible
   and stored files are preserved. Retain existing interpreter, JavaScript,
   CLI, browser-GUI and native-executable tests.
5. Package the exact desktop artifacts and test them with Node, Rust and
   `svr` absent from PATH. Perform a visible window smoke test and later the
   clean compatible Windows VM test. Automated control tests alone are not
   proof of visual quality, screen-reader behavior or clean-machine support.

Mark only demonstrated slices implemented and record exact tests in the
development log. Public release qualification remains separate from this
private prototype.

## Alternatives and future decisions

A local browser host retains the browser workflow and does not satisfy this
desktop-window request. A WebView2/Wry wrapper could provide a desktop window,
but introduces a browser engine and a separate deployment decision. A Rust
widget toolkit could support more platforms, but introduces dependencies and
larger renderer work before this slice is proven. These remain alternatives,
not selected public backends.

Publishing `SVR-UI-1` or a replacement tree, adding component/lifecycle APIs,
native build-target flags, async callbacks, new platform capabilities,
cross-platform deployment contracts or a native ABI requires a separate
concrete approval under `AGENTS.md`. The user authorized this private desktop
prototype; it does not silently settle those compatibility decisions.
