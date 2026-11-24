# Parameter-independent symbolic record fields

`check_generic_copy_bodies` accepts record field reads when the checked field
type has no reachable record type parameters. For example, `Box<T>.count:Int`
and `Box<T>.labels:[String]` qualify. Chained record fields and subsequent array
indexing use existing checked expression rules.

`GenericCopyExpressionKind::Field` retains the checked receiver ID, nominal
record declaration ID, source-order field index and original full-source range.
The receiver must resolve to a registered checked record in the signature's type
definitions. Unknown fields and non-record receivers fail. Field lookup and an
iterative visited-node type walk consume existing semantic work; each field fact
consumes expression admission. Types retain their original field-annotation
owners and alias provenance, without substitution or fabricated arenas.

Only the reachable expanded type matters; unused eager alias argument nodes do
not change the result. [Composite field types](generic-field-substitution-v2.md) now use bounded
receiver substitution. [Direct parameter fields](generic-field-arguments-v2.md), including
`Box<Int>.value:T`, are now handled by validated receiver-argument projection. This restriction is deliberate: the checker
must not confuse the record's parameter owner with the calling function's owner.

These are symbolic field facts, not runtime layout, field-copy or reference plans.
Field writes, record construction, generic calls, specialization
and execution remain open. Future lowering must preserve snapshot ownership and
runtime limits. No generic execution is enabled by this stage.

`tests/generic_field_bodies_v2.rs` covers nominal/receiver/source identity,
array and chained record fields, dependent nested types, concrete-instantiation
refusal, invalid receivers/names, unreachable errors, field write refusal,
and exact/one-under cumulative expression/work quotas.
See [symbolic bodies](generic-copy-bodies-v2.md).
