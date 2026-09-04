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

