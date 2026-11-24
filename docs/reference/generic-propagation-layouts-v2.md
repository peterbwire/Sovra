# Generic propagation operation layouts (V2)

`plan_generic_propagation_layouts` is an opt-in library API over
`CheckedGenericLayouts` produced from [concrete propagation checks](generic-propagation-concrete-v2.md).
It returns an immutable layout owner and a complete list of operations, ordered
by concrete instance and source fact order. Indices belong to the retained owner.

Each operation records its instance, source expression and operand, original
question-mark range, input wrapper, extracted success payload, enclosing return
wrapper, success/failure tags and optional failure payload layout. Option failure
is nullary; Result failure retains the exact shared payload layout. The input
success type may differ from the enclosing return success type. Nested questions
remain separate operations; distinct specializations receive distinct mappings.

The planner rechecks mapped operand/result/return types with exact bounded type
comparison and verifies standard wrapper declarations, arities and variant payloads.
The concrete checker supplies sealed source provenance and standard identity.
Instance, plan and logical work admission applies across the whole request;
comparison pair, edge and work allowances are shared across all sites. Exhaustion
returns no partial plan table. These are logical quotas, not CPU time limits.

This metadata does not execute `?`. IR still refuses any source table containing
questions, even when the question-bearing function is unreachable. The ordinary
source, CLI and native-image profiles remain strict. Early-return control flow,
payload ownership/cleanup, runtime fuel and executable source/native integration
remain open, together with adapters and the remaining G13 work. E2/G13 remain
incomplete. Task Manager is outside this change.
