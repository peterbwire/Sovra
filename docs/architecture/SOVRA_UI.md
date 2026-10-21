# Sovra UI architecture

Search/filter checkpoint (2026-10-20): ordinary Sovra input/button nodes apply
case-sensitive title substring search together with All/Open/Completed selection.
Sovra owns matching, preference validation and a separate versioned `.view` sidecar;
the renderer has no filtering rules. Task bytes and flat interfaces are unaffected.
Preferences persist across restarts and are shared per task path, not per window.
Hidden edit controls follow existing removal rules and discard unsaved drafts.
This private application state is not a public UI state/navigation contract.

Editing checkpoint (2026-10-20): Task Manager's grouped view emits ordinary input
nodes with per-task rename actions. Sovra owns validation and persistence, retaining
nominal task IDs and completion status. The renderer retains rejected drafts and
clears successful submissions; unmodified Escape discards a focused local draft
without sending an application event. No protocol, language or public UI contract
extension is needed. This first slice uses always-visible replacement-title fields,
not prefilled modal editors or a general application navigation model.

Keyboard/accessibility checkpoint (2026-10-20): focused inputs submit on unmodified
Enter, with repeat/busy guards. Explicit Tab order follows Sovra view order and
cycles within the window. Repeated buttons receive contextual accessible names
from view text. Busy, result and error status update their accessible names, with
polite live-region events where WPF supports them. Real Sovra execution passes
visible Enter, forward/reverse Tab, focus and automation activation tests. These
are private renderer defaults; public accessibility metadata and manual screen
reader, high-contrast, DPI and clean-host qualification remain pending.

Grouped desktop checkpoint (2026-10-20): an opt-in private `SVR-UI-2` view adds
keyed vertical groups, horizontal wrapping rows and card/plain presentation.
Task Manager emits its layout through `ui-layout-view`; existing `ui-view`
continues emitting v1 unchanged. Profiles explicitly opt in; no unknown command
probing is performed. The renderer uses only generic layout nodes and preserves
stateful controls when they move between groups. Task Manager and the unrelated
message component pass native-control, event and cross-group draft tests.
Interpreter/JavaScript view parity, existing browser tests, nine dogfood slices,
18 native Task Manager cases and visible grouped-window interaction tests pass.
See [ADR 0030](../adr/0030-private-grouped-ui-views.md) for bounded nesting and
validation. This is not a public tree/ABI or a change to Sovra syntax/semantics.

Responsive desktop checkpoint (2026-10-20): application event work now runs in
an isolated PowerShell runspace. Only immutable program/state/event parameters
are passed to it; native controls stay on the WPF dispatcher. The window remains
enabled while forms are disabled, with a busy notice and single event in flight.
A requested close waits for that event to finish and closes on success; failures
remain visible. No automatic replay or rollback is introduced. This is a private
host scheduling change, not Sovra async syntax, callbacks or a public lifecycle
contract. Initial view loading is still synchronous before the window opens.
Dispatcher-heartbeat, timeout/resource cleanup, duplicate-event rejection,
exactly-once close persistence and existing interaction regressions pass.

Desktop interaction checkpoint (2026-10-20): the private Windows renderer now
reconciles stateful controls using existing action IDs and node kinds. Unchanged
fields retain drafts and selection; updates restore focus and scroll. A successful
input submission clears only that field, while a rejected submission retains it.
Removed fields discard their drafts; changing kinds creates a new control. A
focused action that disappears falls back to the first available input. Duplicate
action IDs are rejected before changing the control tree. These are internal
prototype rules, not approval of the public component identity contract.
Real Sovra event tests and a visible-window focus/scroll regression pass.
User feedback confirms the prior desktop app opened successfully; clean-machine,
accessibility and supported-host qualification remain separate evidence.

Status: **Partial private prototype** (updated 2026-10-15). No public UI package,
renderer contract, language feature, or browser target is implemented. The
Task Manager's Node-hosted GUI, `gui-list` stream and new `/ui` route are
internal starting points, not this framework's public API. See
[ADR 0023](../adr/0023-unified-ui-boundary.md). A private Windows WPF desktop
prototype is now the active implementation direction under
[ADR 0029](../adr/0029-desktop-ui-prototype.md); implementation and desktop
qualification must be established by its acceptance tests.

Implemented private desktop slice (2026-10-15): `renderer.ps1` validates the
existing view, builds WPF controls and invokes Sovra events. Native-control
Add/Complete/Delete tests pass with both interpreter and compiled Task Manager;
the unrelated message component also passes. A visible window smoke test passes
and captures a rendered screenshot. The package includes a C# Windows launcher,
the platform host and native Sovra application. Review caught and fixed silent
startup failures, hidden nonzero event failures and literal-label mnemonic
interpretation. Public API, richer layout/lifecycle, other platforms, full
accessibility and clean Windows qualification remain pending.

## Goal and present boundary

Application authors should write components, view state, event transitions and
business rules in `.svr` files. A host supplies platform capabilities; a renderer
turns a platform-neutral view into native UI elements. Renderers must not contain
task rules, persistence formats or application navigation decisions.

The compiler currently has typed functions, inline modules, nominal records,
arrays, local mutation and `if`/`while`. Local dependency packages can expose
exported functions and records to an executable package. The interpreter runs
IR. `build --emit js` emits a Node CommonJS program with Node process, input and
file APIs; it is not a browser module and has no DOM binding. The CLI starts
`main`; the Rust tooling API can invoke a selected parameterless lowered
function, but there is no published UI callback or component API.
There are no closures, general callback/event types, browser runtime,
public native UI bridge, async scheduler or mobile toolchain. The project checker is
not a substitute for executable package compilation.

Today `dogfood/01-task-manager/gui/bridge.mjs` compiles a Sovra program once and
starts it for each request. `server.mjs` owns loopback HTTP, form security and
HTML. The Sovra Task Manager owns validation, state-file parsing and mutations;
its private `gui-list` stream supplies rows. The Node host still chooses the
page layout, labels, forms and button actions. Moving those decisions into Sovra
is the first migration target.

## Proposed architecture

```text
Sovra application (.svr)
  domain functions + persisted model
  component functions: view(model) -> UiTree
  event functions: update(model, UiEvent) -> UiResult
                 |
            UI package contracts
                 |
      versioned host transport / capability boundary
          /             |              \
    web renderer    desktop adapter        mobile adapters
    (existing)      (private Windows)      (Android/iOS later)
```

`UiTree` is a platform-neutral *concept*, not an existing Sovra type. A
component produces an immutable tree with stable keys. The minimal proposed
node kinds are text, container, heading, label, text input, button and list.
Containers carry layout intent (vertical, horizontal, spacing and alignment),
not CSS or native widget classes. Inputs have a stable field key; buttons have
an action key. Accessibility data includes accessible name, role when the
element's native role is insufficient, heading level, label association,
disabled state, and status/live-message intent. A renderer may refuse an
unsupported node or property; it must not silently discard an action or
accessibility requirement. Text is data and must be escaped by the renderer.

An event identifies its component/key, action and bounded input values. The
application validates the event and returns a new state plus a new view or an
explicit error; the host never edits the domain model itself. State has three
distinct lifetimes: temporary input state in the renderer, application session
state in Sovra, and persisted domain state behind Sovra business functions.
Persisted state must be reloaded or conflict-checked before a mutation, as the
current Task Manager already does. Stable component keys support focus and
input preservation across renders. Event processing is serial per session in
the first web adapter; concurrency, optimistic updates and cancellation need a
separate contract before implementation.

Lifecycle is explicitly host-driven: initialize a session, render, dispatch
events, render the resulting view, then dispose. A later effect phase may
request bounded capabilities (file picker, notifications, network, clipboard)
through an allowlisted host interface and return typed results to Sovra.
Neither rendering nor component construction should perform hidden I/O.
Navigation is an application-selected route state and event transition;
renderers map it to browser history, desktop views or mobile navigation only
after a reviewed routing contract. The first prototype stays on one route.

## Renderer and host responsibilities

The proposed renderer interface consumes one validated tree version and
returns either a rendered view or a typed unsupported/invalid-tree error. It
must preserve sibling order, keys, text, actions and accessibility metadata.
Its event adapter returns bounded, untrusted event data; the Sovra application
owns authorization and validation. The host owns process lifecycle, scheduling,
transport limits, capability grants and platform failure reporting. Host-side
escaping, CSP and CSRF checks remain defense in depth; they do not replace
application validation.

The first web renderer can remain a Node loopback server and static HTML/form
renderer. It runs generated JavaScript through the existing bridge, with
`svr run` parity tests. It does not require browser-executable JavaScript or
new compiler syntax. A future browser-only target would require explicit
runtime and capability work because generated code currently imports Node.
The private Windows desktop prototype selects native WPF controls through
Windows PowerShell/.NET, as described below. A system webview or a different
widget adapter remains a future option for a public desktop framework.
Android and iOS may use a webview first or native adapters later; no mobile
adapter or release target is selected. All adapters must pass the same
tree/event conformance fixtures and document platform-specific behavior such
as focus, touch, back navigation and accessibility mapping.

Platform APIs are requested by named, versioned capabilities. An application
declares what it needs; the host grants or rejects each capability and returns
a typed result. No renderer receives direct access to application files or
secrets. The current Task Manager's file I/O stays in Sovra through its existing
standard library. Browser DOM, native windows, mobile sensors and background
tasks require separate capability and lifecycle decisions.

## First working prototype: Task Manager web component

Implemented private slice: `ui-fixture.svr` produces a heading, text and button;
`ui-view.mjs` strictly decodes a bounded `SVR-UI-1` line stream and escapes HTML.
The fixture's Complete event runs through both execution engines and a real
loopback HTTP request in `scripts/test-ui-prototype.mjs`. The Task Manager now
emits task heading/count/rows/Complete actions from Sovra via `ui-view` and
handles a `ui-event` completion by stable task ID. `/ui` renders it through the
same generic Node renderer. Browser tests verify completion and stale-event
rejection in both engines. The existing `/` route remains the full GUI.
The Task Manager now also defines an Add input, Delete buttons and outcome
notices in Sovra. The host maps these generic nodes and status text to HTML;
it does not decide task labels or action rules on `/ui`. The private view
format permits heading, text, button and single-line input nodes, with a
64 KiB/256-node bound. It has no layout, navigation, lifecycle or richer
accessibility schema beyond native controls and a status region. The existing
`/` route remains the production-facing GUI. This stream is insufficient for
public package use.

The unnumbered File Manager dogfood app is a second consumer of the private
renderer. Sovra emits its heading, file labels and Preview/Copy/Create controls from
directory names supplied by the Node host. Node still owns enumeration and
path validation, since Sovra has no directory API. Per-page listing snapshots
keep index-based actions attached to the names visible when their forms were
rendered. Sovra also emits Previous/Next controls for 80-file pages. This shows
renderer reuse across two applications, but it does not
make the protocol public or solve native/browser runtime contracts.

The smallest useful vertical slice is one reusable Sovra `TaskList` component
that chooses its heading, count, rows, completed labels and action identifiers.
The existing Task Manager remains the domain owner. A private `ui-view` command
could emit a bounded, versioned tree stream after loading the state file; a
private `ui-event` command could dispatch `add`, `complete` or `delete` through
the existing Sovra functions. Existing `gui-list` can remain during migration.
The host decodes the tree, validates its schema, escapes text and maps nodes to
semantic HTML. It sends form events back to Sovra and re-renders from a fresh
view. The host must not contain task labels, row markup or task-specific action
branches once this slice is complete. A generic host may still own standard
control rendering, CSRF, HTTP errors and its launch page.

Do not publish the prototype stream as a compatibility contract. Its wire
format, size bounds, key identity, error categories and version negotiation
must be reviewed before other applications depend on it. Existing Sovra string
and record features are sufficient to *experiment* with serialization; the
absence of first-class tagged variants, callbacks and structured IPC means a
public `UiTree` API cannot be assumed to work without additional design.

Implementation order:

1. Add a small, standalone Sovra view fixture using existing syntax. Run it
   through both engines and verify identical tree output. Keep it private.
2. Add a generic Node decoder/renderer with strict length, depth, node-kind,
   key and action validation. Test text escaping, Unicode, malformed input and
   accessibility output without involving Task Manager storage.
3. Move one Task Manager list-and-complete flow into Sovra view/event functions.
   Exercise browser interaction, restart persistence and generated-code proof.
4. Move add/delete, notices and empty states. **Implemented in the private
   `/ui` route** with interpreter/JavaScript browser parity tests; continue
   hardening before replacing the existing `/` route.
5. Review the public UI package and protocol decisions in ADR 0023, then add
   local-package consumption and cross-app conformance tests. Do not advertise
   desktop or mobile support before an adapter passes those tests.

Acceptance for the first slice: an unrelated fixture and the Task Manager
render through the same Node renderer; a browser click invokes real Sovra code;
the changed task persists and re-renders; malformed trees/events fail safely;
the same event yields the same state and view in interpreter and generated
JavaScript execution. Tests must run in CI and from an extracted candidate.

## Private Windows desktop prototype

The next concrete UI slice opens a desktop window rather than a browser or
terminal. [ADR 0029](../adr/0029-desktop-ui-prototype.md) records the authorized
private direction and its acceptance gates. Reusable WPF rendering functions
live in `toolchain/ui/windows/renderer.ps1`; the adjacent `host.ps1` owns
startup and application execution. This is an internal Windows adapter,
not a published Sovra UI framework or a new compiler backend.

The same `SVR-UI-1` heading/text/button/input stream drives native controls.
The host asks the actual Sovra application for `ui-view STATE`, dispatches
`ui-event STATE ACTION [VALUE]`, then reloads the view. Development can use
the interpreter; a distributed desktop package uses the existing standalone
application executable. Sovra chooses the interface text/actions, validates
events and owns persistence. The WPF renderer owns generic layout, temporary
input, focus, native controls and error display. It must contain no Task
Manager command or task-file parsing branches.

Views retain their 64 KiB/256-node bounds and must validate completely before
rendering. Control labels and input remain literal data: the host must not
generate PowerShell, command-shell source or XAML from them. Direct process
execution must preserve quotes, backslashes, Unicode and argument boundaries.
Events run serially with bounded process output/execution; errors remain
visible and uncertain mutations are not retried automatically. WPF controls
need accessible names and usable keyboard navigation. A full accessibility
tree, live announcements and focus preservation remain qualification work
because the private stream does not yet define those contracts.

The first acceptance runs both `examples/ui-desktop/component.svr`, an
unrelated component, and Task Manager through the same renderer. Native
control events must execute real Sovra code, update the displayed view and
preserve Task Manager changes across a restart. Test malformed views,
application/startup failures, corrupt state, stale actions and argument
transport as well as successful add/complete/delete. Preserve the browser
and CLI regression suites. Only actual test evidence establishes implemented
behavior; a decoder or empty window alone does not meet these gates.

The platform dependency is Windows PowerShell plus WPF/.NET. The native
application path needs no Node, Rust or installed Sovra compiler at runtime.
Package the host, renderer, launcher and application together and document
these dependencies; the CLI executable by itself supplies no desktop UI.
Visible window testing and a clean Windows VM remain separate qualification
steps. This work does not establish cross-platform desktop, mobile or browser
execution, and a scrollable window does not remove the view-size limit.

## Errors, testing and build targets

Distinguish compile/semantic errors, host transport errors, invalid UI trees,
rejected events, domain validation errors, capability denial and renderer
failures. Hosts must surface safe user-facing messages and retain diagnostic
details in local logs; an invalid view must not be partially rendered or cause
an event to be applied. No mutation should be replayed automatically after a
transport timeout without an idempotency contract.

Use component golden fixtures for tree shape, renderer conformance tests for
HTML/accessibility/escaping, event-to-state behavioral tests in both Sovra
engines, and end-to-end browser tests against persisted Task Manager state.
Cross-platform adapters later reuse the same view/event fixtures plus native
accessibility and lifecycle tests.

Proposed package layout after protocol approval: `packages/sovra-ui-core/` for
Sovra component/data helpers, `packages/sovra-ui-web/` for the Node/web host,
and adapter packages for desktop, Android and iOS only when implemented. The
private web prototype remains under `dogfood/01-task-manager/gui/`; its Windows
counterpart uses `toolchain/ui/windows/` and an unrelated example component.
Neither location implies package publication.
Potential build targets are `svr build --emit js` for the Node web host first,
then explicitly designed browser, desktop and mobile targets. Today's CLI has
no such targets, so no command names or target triples are reserved here.

## Decisions requiring approval

ADR 0023 records the proposed public UI tree and event/lifecycle boundary.
Publishing it would establish protocol versioning, component identity,
accessibility obligations, state ownership and error behavior. Any new Sovra
syntax, callback/closure semantics, async model, memory/resource lifetime,
browser runtime, native ABI, direct platform APIs or compatibility change also
requires a separate approved ADR under `AGENTS.md`. The private prototype may
use existing syntax and commands without making those decisions public.
ADR 0029 records the user-authorized private Windows prototype; it does not
approve the public protocol or a desktop build/distribution contract.
