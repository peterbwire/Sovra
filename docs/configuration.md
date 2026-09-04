# Configuration

`svr check <project-directory>` reads `sovra.toml` from the project root and
validates the production-facing project shell.

Supported sections:

```toml
[project]
name = "fielddesk"
version = "0.1.0"
entry = "app/main.svr"

[runtime]
target = "web"

[services]
maps = "env:MAPS_API_KEY"
```

The required project keys are `project.name` and `project.entry`.
`project.entry` must point to an existing `.svr` source file. Supported runtime
targets are `web` and `cli`.

Unknown sections, unknown keys, duplicate sections, duplicate keys, malformed
assignments, unsupported runtime targets, missing entries, and unreadable
project directories are reported as structured diagnostics.

Service bindings under `[services]` must use valid Sovra identifiers. During
project checks, each manifest service binding must match a `service <name>`
declaration in project source, and each service requested by the app entry's
`services: [...]` list must be both declared and bound.

The project checker also indexes route and page declarations from the app entry.
Routes use `route METHOD "/path" -> target`; pages use `page "/path" -> target`.
Public paths must start with `/`, avoid whitespace, avoid empty path segments,
avoid trailing slashes except for `/`, and use valid Sovra identifiers for
`:parameters`. Route targets must resolve to a `fn` or `task`; page targets must
resolve to a `page` or `view`.

App wiring is also checked. `auth: module.symbol` must resolve to an `auth`
declaration, each `data: [...]` model must resolve to a `model` declaration,
and scheduled `task <schedule> -> module.symbol` entries must resolve to a
known `task` or `fn` target.

Auth policy lines use `allow role to action on Model`, bracketed lists such as
`allow manager to [read, write] on [Customer, Job]`, or the shorthand
`allow role to action Model where ...`. Policy model names must resolve to known
model declarations, and duplicate policy lines are rejected.

