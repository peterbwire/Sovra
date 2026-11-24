# Source-owned generic records in v2

```rust
use sovra::compiler::v2::{parse_generic_record, GenericRecordLimits};
let record = parse_generic_record(
    "struct Entry<K:std::Key,V>{key:K,values:[V]}",
    GenericRecordLimits::DEFAULT,
)?;
```

The additive Rust API attaches one declaration-local parameter owner to every
field annotation in a complete root generic record. It accepts
`[export] struct Name<T>{field:Type,...}` using the approved parameter/type grammar.
Empty record bodies and a trailing field comma follow existing record syntax.
Commas inside nested type applications are not field separators. Duplicate fields,
missing separators/types, malformed nesting, applied parameters (`T<Int>`), extra
declarations and unsupported module wrappers refuse without a partial result.

`GenericRecordSyntax` retains original source, record/name ranges, export intent,
the parameter-fragment range, its parameter owner and immutable source-order fields.
Each field retains its original name/declaration/type ranges and an annotation-use
table sharing the same parameter owner. Repeated parameters across different
fields therefore have one identity. Different records cannot share identities by
spelling. Export intent is syntax metadata, not checked visibility authority.

## Coordinates and admission

Parameter and annotation owners retain exact fragment-local source and spans.
`parameter_span(id)` and `field_type_node_span(field,node)` map to full-source
byte/line/column coordinates and refuse foreign identities or invalid indices.
Fragment errors are likewise translated into the original declaration. Full
source and fields are immutable shared owners; annotation binding does not copy
whole parameter arenas per field.

The default whole-record profile admits 128 KiB input, 32,768 original tokens
including EOF, 256 KiB total retained full/fragment source, 48 KiB copied names,
1,024 fields, 4,096 field-type nodes, 4,096 field-type edges and 4,096 parameter
uses. Independent parameter-list and per-annotation limits remain in force.
Copied names include the record, parameters, fields and annotation type names.
Retained source includes the complete declaration, parameter fragment and every
annotation fragment. Each field is admitted before index/record retention.

Nodes, edges, names and parameter uses consume remaining record-wide allowances
before parsing/binding each field; later fields cannot reset a ledger. Retained
source bytes are admitted before fragment copying. Whole-source and fragment
lexers remain individually bounded. The field boundary scan is iterative; type
parsing retains its fixed depth ceiling. These are logical admission contracts,
not physical-memory or helper CPU isolation.

## Remaining semantics

This is declaration syntax and parameter-use ownership, not a checked record
type, layout, constructor or field-access implementation. The separate
[definition annotation checker](generic-declarations-v2.md) now validates all
registered record fields; this syntax parser does not perform that stage. Unknown type names and
wrong generic arities may parse. Semantic name/arity/visibility/bounds, recursive
layout/alias rules, substitution/specialization and execution remain open.
[Generic enums](generic-enum-syntax-v2.md) now have syntax attachment too;
[generic function headers](generic-function-syntax-v2.md) also have attachment
with bounded opaque bodies. Generic body checking remains open. Existing CLI/source checking still
refuses unsupported generic programs. No default legacy AST/value variant changes.

See [generic alias attachment](generic-alias-syntax-v2.md),
[parameter ownership](generic-parameters-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 remains partial.
