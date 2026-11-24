# Source-owned generic enums in v2

```rust
use sovra::compiler::v2::{parse_generic_enum, GenericEnumLimits};
let syntax = parse_generic_enum(
    "enum Outcome<T,E>{Ok(T),Err(E),End}",
    GenericEnumLimits::DEFAULT,
)?;
```

The additive Rust API attaches one parameter owner to all payload annotations in
one complete root `[export] enum Name<T>{Variant,Payload(Type),...}` declaration.
Variants preserve original order, written names and full/name ranges. Export intent
is retained as syntax metadata. Nullary variants have no payload object; a payload
such as `UnitValue(Unit)` remains distinct. Every payload retains its exact type
fragment and a parameter-use table sharing the enum's declaration-local owner.

A trailing variant comma follows existing enum syntax. Type arguments nested in
a payload may contain commas; tuple payloads, named fields, empty parentheses,
explicit discriminants and extra declarations refuse. Duplicate variant names
and applications of a parameter such as `T<Int>` refuse the entire declaration.
The enum keyword remains contextual under the existing lexer. Module wrappers
and generic source execution are not enabled.

## Original coordinates and limits

`GenericEnumSyntax` retains complete original source and parameter/payload
fragments. Fragment spans stay local to their immutable owners. `parameter_span`
and `payload_type_node_span` map them into the complete source's byte/line/column
coordinates. Foreign parameter identities, invalid indices and nullary payload
lookups refuse. Fragment errors are translated into the original source as well.
Different declarations cannot share parameter identities by spelling.

Defaults admit 128 KiB full input, 32,768 original lexer tokens including EOF,
256 KiB retained full/fragment source, 48 KiB aggregate copied names, 1,024 variants,
4,096 payload type nodes, 4,096 type edges and 4,096 parameter-use records.
Parameter-list and per-payload annotation profiles retain their independent caps.
Full/fragment source copies and enum/parameter/variant/type names consume aggregate
allowances. Nullary variant names/records are charged even without type nodes.
Payloads consume the remaining node/edge/name/use allowances before parsing and
binding; later variants cannot reset those ledgers.

Annotation/parameter owners are shared without deep-copying whole arenas per
payload. Variant retention and duplicate-index entries are width-bounded. Payload
boundaries are scanned iteratively; type parsing keeps its fixed depth ceiling.
Logical admission is separate from process CPU/physical-memory isolation.

## Remaining semantics

This is syntax and parameter-use attachment, not a checked nominal generic enum.
The separate [definition checker](generic-declarations-v2.md) now validates
registered payload annotations and rejects empty enums at its semantic boundary.
Unknown names/incorrect application arities can parse. An empty generic enum body
can parse, as in the earlier raw enum syntax API; executable enum checking must
still require at least one variant. Generic type/constructor/match checking,
bounds, recursive-type rules, substitution/specialization, Option/Result and
propagation remain open. [Generic function headers](generic-function-syntax-v2.md) now have attachment
with explicitly opaque bodies; generic body parsing/checking remains open.
Default CLI/source checking still refuses unsupported generic programs.

See [generic parameters](generic-parameters-v2.md),
[generic records](generic-record-syntax-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 remains partial.
