# Task Manager GUI execution boundary

Status: implemented local adapter and reusable execution module; a stable data
protocol and dedicated Sovra UI package remain planned.

The local browser GUI in `dogfood/01-task-manager/gui/server.mjs` hosts HTTP and
HTML with Node.js. Its reusable `bridge.mjs` accepts a compiler, Sovra source
and engine, invokes `build --emit js` at startup, and manages the generated
program's temporary file and process lifetime. Each browser action invokes
that generated program with the same `list`, `add`, `complete` or `delete` arguments as the
CLI. `--engine interpreter` selects `svr run` for parity checks. Both paths use
the same versioned task file and Sovra application rules. Node reads the file
only to render task rows after Sovra has validated it with `list`.

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

The next bridge step should give the host a stable data protocol from Sovra
code so it can avoid
reading the Task Manager's private file format; that protocol needs a separate
compatibility review before it becomes public. A dedicated Sovra UI package
can then use the execution module and common HTTP/form helpers. The bridge is
currently repository-internal and has no published compatibility promise.
Native runtime scheduling, browser DOM APIs and an
application server remain outside this slice.
