# Application record contracts

Run `svr check --service-calls examples/application-record-contracts`.
This validates record declarations and nominal contracts; it does not execute
the external service or start an application runtime.

`Position` aliases the local `Point` record. A value returned by `store.load`
can pass through an ordinary function and back into `store.save`. A same-shaped
record declared in another file is a different type.

Field reads such as `point.x` propagate the declared field type into service
arguments. Local construction now validates required fields and widening, as in
`Point { x: 1, y: 2 }`. Exported application records and field
mutation remain unsupported by this checker slice. Executable package records
retain their existing separate implementation.
