# Concrete propagation rechecking

Two explicit Rust APIs extend [symbolic postfix bodies](generic-propagation-bodies-v2.md):

- `discover_generic_propagation_specializations` discovers the finite concrete call closure.
- `recheck_generic_propagation_concrete_bodies` substitutes and rechecks each reachable instance under ordinary concrete limits and cumulative `GenericPropagationLimits`.

The ordinary specialization and concrete entry points continue refusing
question-bearing source tables. No source/CLI/native feature or executable
propagation is enabled by these APIs.

Call discovery retains existing exact normalized keys, recursive instance reuse,
argument/bound checks, structural-depth and whole-graph generated type/key/call/
work caps. A question node is not a new call edge; its earlier operand calls
remain in the source expression table and are visited normally. No branch or
unreachable-call optimization bypasses existing admission.

Concrete checking first substitutes every signature, binding and expression type.
For each postfix fact it validates the earlier operand identity, original
question range, actual wrapped operand and exact caller return type. It recomputes
the standard wrapper family, nominal identity, success/failure variant indices
and payload contexts from these closed types. Original symbolic success, failure
and return descriptors are independently substituted and compared with actual
concrete facts. Stale descriptors are refused rather than silently repaired.
Result failure types must still match exactly; no numeric, nominal or error
conversion is introduced after specialization.

Each `ConcreteGenericBody::propagations()` row retains an immutable
`ConcreteGenericPropagation` with original expression/operand IDs, original question
span and recomputed closed `GenericPropagationTypes`. IDs belong to the containing
source body's arena; use them with the containing concrete instance, since the
same source may specialize to different payload types. Original sources and
provenance remain owned by the checked graph. Parameter nodes cannot remain in
these concrete contexts.

`CheckedConcreteGenericBodies::propagation_usage()` counts actual concrete sites,
so one symbolic site specialized three ways costs three sites. Context, exact
comparison pair/edge and work allowances are shared across all instances. Existing
`GenericConcreteBodyLimits` also charge retained projections, independently
instantiated descriptors, generated nodes/edges, concrete validation and
comparisons. Independent comparison profiles cap comparison work rather than
unrelated site overhead. Symbolic body checking uses the same rule. Exhaustion
returns no partial facts and never selects an unlimited or ordinary fallback.
Logical controls do not establish physical heap or hard CPU-time guarantees.

Original literals can be decoded and logical type/nominal-operation layouts can
be planned from these checked instances, including literals adjacent to question
expressions. That is metadata only. `lower_generic_ir` has an unconditional
`PropagationUnavailable` gate for any question-bearing source table, including a
root whose questions are all in unreachable functions. Local unknown-operation
refusals remain too. Thus no metadata path can silently drop a propagation node
and publish an executable IR program. Zero-site source tables retain ordinary
lowering/execution behavior.

The [concrete regression target](../../tests/generic_propagation_concrete_v2.rs)
covers distinct generic payloads, closed contexts, recursive reuse, aliases,
structured errors and chains, exact stage/propagation boundaries, literal/layout
retention, ordinary API refusal and the IR gate. Test-only fault injection verifies
rejection of stale symbolic descriptors and cyclic operand IDs. No production
mutation API is added.

Early-return layout/operation planning, typed-stack control flow, operand temporary
cleanup, successful/error payload snapshots and resource accounting, executable
source/native feature integration, explicit adapters and remaining G13 work stay
open. Existing native profiles 1/3/7 and full-source/CLI checking still refuse `?`.
G13 E2 and G13 remain incomplete.
