# Exported service record fields

Run `svr check --service-calls examples/application-record-imports`.
The direct service import returns an exported `Snapshot`; its nested exported
`Point` fields retain their declaring file's types. The consumer's same-named
`Point` cannot reinterpret those fields.

This is static checking, not service execution. Private records remain opaque
outside their file. The example also constructs `app::store::Point` and checks a
qualified local annotation. The `relay` service also accepts and returns the
imported Point contract. `Position` aliases the imported record, and the envelope
module embeds it in an exported field. The factory module exports a record-returning
function and a record-taking function; their types retain the store module's
identity. Application execution remains unfinished.
