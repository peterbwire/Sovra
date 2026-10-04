# Ordinary helpers in application checking

Status: Implemented checker subset; this is not an executable application.

```text
cargo run -- check --service-calls examples/application-functions
cargo run -- check --service-calls --format json examples/application-functions
```

The checker resolves primitive ordinary function signatures throughout this file,
including forward references and mutual recursion. Service implementations can
call these helpers explicitly. Validated call results feed local annotations,
conditions, service arguments and returns. Int arguments may widen to Float.
The label helper uses `std::to_string`; builtin contracts come from the existing
Rust registry and are checked alongside ordinary calls. Bare `print` is retained.

Parameters and local bindings shadow function names. Tasks and service operation
names do not become bare ordinary functions. Direct exported ordinary imports are
demonstrated in `../application-imports`. Named application types and general
unresolved names remain unfinished. The `external`
service binding does not provide a runtime implementation.
