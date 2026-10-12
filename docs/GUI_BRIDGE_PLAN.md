# Task Manager GUI execution boundary

Status: implemented local adapter, reusable execution module and private
Task Manager view stream. A public data protocol and dedicated Sovra UI
package remain planned.

The first private Sovra UI prototype is now available at `/ui`. The Task
Manager emits bounded heading/text/button/input nodes and handles Add,
Complete and Delete events in Sovra; `ui-view.mjs` validates and renders those
nodes for the Node host. Sovra also supplies operation notices.
`ui-fixture.svr` exercises the same renderer outside Task Manager. The normal
`/` GUI remains the full-featured route. This prototype is not a public UI
contract; see `docs/architecture/SOVRA_UI.md` and proposed ADR 0023.

The local browser GUI in `dogfood/01-task-manager/gui/server.mjs` hosts HTTP and
HTML with Node.js. Its reusable `bridge.mjs` accepts a compiler, Sovra source
and engine, invokes `build --emit js` at startup, and manages the generated
program's temporary file and process lifetime. Each browser action invokes
that generated program with the same `list`, `add`, `complete` or `delete` arguments as the
CLI. `--engine interpreter` selects `svr run` for parity checks. Both paths use
the same versioned task file and Sovra application rules. For display, the
Sovra program emits a private `SVR-GUI-1` stream through `gui-list`: a header
followed by `id|done|title` rows. Titles cannot contain LF or CR in the task
file; the host splits only the first two separators so pipes and trailing
spaces remain intact. The host renders this stream and never opens the task
file. This command is internal to the dogfood adapter and has no public
compatibility promise.

The GUI test executes the same workflow through both engines, compares their
persisted files, and checks rejection of duplicate/control-bearing titles,
HTML escaping and local form protections. Candidate and Cargo source-package
verifiers run this test against the packaged source and compiler. A separate
bridge test runs an unrelated hello-world source in both engines. This proves
the Task Manager's business operations run in Sovra-generated code without
adding a browser or native window runtime to the language.
In both generated-JavaScript tests, the source copy is removed after build and
before execution. Operations still succeed, so they cannot be falling back to
`svr run` or reparsing that source on each request.

Legacy titles containing terminal controls are shown in printable notation
and marked CLI-only in the GUI, because browsers may normalize controls in
form fields and break exact title lookup. The Sovra CLI remains able to edit
those records. A public, general data protocol needs a separate compatibility
review before it becomes part of a reusable UI package. A dedicated Sovra UI package
can then use the execution module and common HTTP/form helpers. The bridge is
currently repository-internal and has no published compatibility promise.
Native runtime scheduling, browser DOM APIs and an
application server remain outside this slice.
