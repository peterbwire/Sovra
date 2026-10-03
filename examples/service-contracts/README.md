# Primitive service contract checks

This is a checker example, not an executable service implementation:

```text
cargo run -- check --service-calls examples/service-contracts
cargo run -- check --service-calls --format json examples/service-contracts
```

The explicit check resolves service signature primitives, operation names and
argument counts. Literal arguments are checked, allowing Int-to-Float widening;
mismatches produce E4119. The omitted return annotation on `log` means Unit in
the typed contract. Primitive parameters and inferred literal/binding locals also
contribute argument types. Primitive annotated locals validate known initializers
with E4120 before contributing types. Primitive binary arithmetic/comparisons also
contribute types; invalid known operands produce E4121. Valid resolved service calls
contribute contract return types. Implementations, ordinary call result types and external service
execution are not validated or provided by this example. Named/generic application
contract types remain unsupported and produce E4117.
