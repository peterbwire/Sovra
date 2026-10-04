# Exported application functions

Status: Implemented checking under ADR 0013; not application execution.

```text
cargo run -- check --service-calls examples/application-imports
cargo run -- check --service-calls --format json examples/application-imports
```

`use app.helpers` imports the source `app/helpers.svr`. Its `export fn format`
is callable as `app::helpers::format(12)`. Its unexported `prefix` helper remains
private to that source. Imported functions are not exposed as bare names and
imports do not re-export other modules. Source snapshots and interface collection
allow cyclic imports without recursively loading bodies.

The JSON call record retains the caller location and a `declaration_file` for
the imported function. Arguments and returns use the currently supported primitive
type rules. The external service binding does not provide a runtime implementation.
