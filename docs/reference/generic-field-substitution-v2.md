# Bounded composite record field substitution

`check_generic_copy_bodies` now resolves composite record fields such as `[T]`
and `Other<T>` using the checked receiver's type arguments. Nested field reads,
array indexing and repeated composition retain exact caller parameter identities.
The helper verifies shared environment, nominal declaration/arity and every
substituted record parameter owner before publishing a complete type graph.
Already-checked symbolic field bounds and checked receiver argument bounds remain
in force; this does not create new numeric capabilities or runtime specialization.

Independent fields and direct parameter fields retain their existing projection
paths. Composite substitution copies the admitted receiver arena, then maps the
expanded field's postorder nodes, replacing parameter nodes with the exact
receiver argument roots. It preserves unused eager nodes and charges them too.
New child indices always point to earlier validated nodes. A bounded iterative
pass computes generated depth; no recursive substitution or proof graph is added.

`GenericFieldSubstitutionLimits` caps per-field generated nodes, edges, work and
structural depth (at most 128). `GenericCopyBodyLimits.max_field_nodes` and
`max_field_edges` apply cumulatively across every body; existing body work also
caps each substitution. `field_nodes()`/`field_edges()` report usage. Each generated
node has one retained input-source metadata slot; temporary field-node mappings
are charged before reservation. These are logical limits, not physical memory
isolation or hard time deadlines. Refusal returns `GenericCopyBodyError::FieldType`
without a partial field type.

Composed nodes may originate in either the receiver annotation, the field
annotation or an alias target. **Use `ExpandedGenericType::node_source(index)`
with that node's origin span**, rather than assuming every node belongs to the
primary `syntax()`. Argument nodes preserve caller provenance and field-generated
nodes preserve declaration/alias provenance. The retained parameters are the
caller's owner. Array/direct argument projections retain this source metadata.

No field writes, record construction, generic calls, executable IR or runtime
layout/ownership changes are introduced. Future lowering must preserve existing
snapshot semantics and runtime allocation/fuel/error rules.

`tests/generic_field_substitution_v2.rs` covers composite arrays/nominals,
repeated composition, caller identity and wrong-owner return refusal, mixed input
and alias source provenance, exact/one-under generated node/edge/work limits,
independent depth/node caps and cumulative repeated-expression admission.
See [direct parameter fields](generic-field-arguments-v2.md).
