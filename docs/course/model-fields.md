# Inspect primitive model fields

Status: **Partial, additive Rust API.** Model values cannot yet be constructed or
executed through this feature. The default CLI/project checker remains partial.

This source passes the first field checker:

```svr
type Title = String;
model Item {
    id: Int
    title: Title,
    active: Bool,
}
```

A field needs its own name and explicit primitive annotation or local primitive
alias. Separate fields with a newline or comma; one trailing comma is allowed.
Whitespace/comments can occur between a field's name, colon and annotation.
`Unit`, `Bool`, `Int`, `Float` and `String` are the canonical primitive names.
`Text` needs an explicit alias such as `type Text = String;`.

Compiler library callers inspect the source without executing it:

```rust
use sovra::compiler::project::application::check_model_primitive_fields;

let checked = check_model_primitive_fields("model Item{id:Int}").unwrap();
assert!(checked.diagnostics.is_empty());
assert_eq!(checked.models[0].fields[0].name, "id");
assert_eq!(checked.resolved.len(), 1);
```

Duplicate fields and unknown simple annotations produce diagnostics at the
original source tokens and leave the primitive table empty. Unsupported syntax,
imports, declared nonprimitive field types or exhausted limits return `Err`
without a partial table. Defaults, optional/generic annotations, model-valued
relationships, enum variants, storage and generated APIs remain future work.
Header/body coverage gaps stay visible in the independent metadata; successful
field checking neither validates other declarations nor grants permissions.
See the [API reference](../reference/application-type-syntax.md#primitive-model-field-checking)
for ownership, diagnostics and exact resource bounds.


For a scanned project, library callers may use
`check_service_calls_with_model_fields(&project)`. Inspect both its original
`check` and every optional `model_fields` file result, including each file's
semantic diagnostics. The wrapper shares source snapshots with service checking;
it does not resolve imported field types, suppress E4096 or add a CLI mode.
