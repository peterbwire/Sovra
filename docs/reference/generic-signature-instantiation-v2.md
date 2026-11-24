# Bounded callee signature substitution

`instantiate_generic_signature` accepts a sealed caller/callee type binding and
substitutes every original callee value-parameter annotation and explicit result.
`InstantiatedGenericSignature` retains that binding, caller-scoped parameter types,
return convention and generated annotation/node/edge/work counts. No unchecked
parameter list or foreign signature can be supplied through this public API.

The private substitution pass copies every admitted argument arena with rebased
indices, then maps callee annotation nodes in postorder. Exact callee-owned
parameter IDs select argument roots; argument parameter IDs must belong to the
original caller. Nominal declaration identities, type order and aliases' expanded
meaning remain intact. Unused eager arguments remain retained and charged for
each instantiated annotation. This is intentionally not code specialization.

Every generated node retains its original argument, callee annotation or alias
source. Use `ExpandedGenericType::node_source(index)` with the node's origin span.
The result's primary syntax remains the callee annotation and its parameter owner
is the caller. No source rewriting or synthetic parameter identity is introduced.
Implicit Unit returns remain implicit; explicit Unit remains an admitted annotation.

`GenericSignatureInstantiationLimits` caps total explicit annotations, generated
nodes/edges/work and independent per-type nodes/edges/work plus structural depth
(up to 128). Even an annotation-free signature consumes admission work and rejects
invalid depth profiles. Temporary postorder mapping entries are charged before
reservation; source metadata has one slot per generated node. Storage and
substitution traversal are flat; input graphs and owners are retained immutably.
Failures identify the original parameter/return position and publish no partial
instantiated signature. Logical limits are not physical process isolation or
hard time deadlines. Future call integration must meter repeated instantiations
cumulatively across the enclosing compilation.

This is substituted signature evidence, not a checked value call or checked body.
[Value-argument matching](generic-call-values-v2.md) is now a separate bounded
stage. Body call facts, concrete body substitution/rechecking,
recursion/specialization planning and executable integration remain open. No
runtime layout, snapshot ownership, CLI execution or reference semantics change.

`tests/generic_signature_instantiation_v2.rs` covers caller parameter identity,
composite arrays/nominals, alias arguments, mixed-source provenance, implicit/
explicit Unit, exact/one-under cumulative and per-type caps, invalid empty-signature
profiles and retained sealed owners.
See [callee type matching](generic-call-binding-v2.md).
