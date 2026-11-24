# Generic propagation early-return IR (V2)

`lower_generic_propagation_ir` accepts [checked propagation layout operations](generic-propagation-layouts-v2.md),
nominal-construction limits and IR limits. It produces
`CheckedGenericPropagationIr`, a separate opt-in owner with function instructions,
layout ownership and usage. It does not expose a `CheckedGenericIr` and cannot be
passed to `execute_generic_ir`. The explicit [propagation VM](generic-propagation-vm-v2.md)
now executes this separate owner.

A `GenericIrOperation::Propagate` evaluates after its operand exactly once. On
success it consumes the input wrapper and pushes its successful payload, keeping
pending operands. On failure it constructs the caller's failure wrapper and exits
the current frame, abandoning that frame's pending operands and local slots.
Caller frames are not abandoned. This is an implicit terminal control-flow edge;
it is not a normal `Return`, which requires exactly one remaining operand.
Runtime implementation is available through that explicit library entry point.

Each instruction carries input, success and caller-return layouts, standard tags
and an optional exact failure payload. Structural validation independently checks
compiler-owned standard identity, wrapper declarations, arities, variant payloads,
failure equality and the enclosing result. The typed flow pass checks the consumed
wrapper and successful continuation at existing stack joins. Failure has no local
continuation; its return-layout contract is established structurally. Question
ranges retain original source provenance. Every retained operation must be emitted
exactly once. Nested questions and concrete instances keep their own source order.

Construction admission remains independent. IR instance/instruction/operand/target/
pending-task/stack/work quotas apply to the whole request. Each propagation charges
one operand and two control outcomes; retained source-to-operation mappings also
consume operand admission. Existing labels and branch targets continue to resolve
normally. Exhaustion or inconsistent proofs return no partial IR.

Ordinary `lower_generic_ir` still rejects every source table containing questions,
even when all such functions are unreachable. The ordinary VM additionally refuses these
source tables before dispatch and defensively rejects the new opcode. Full-source,
CLI, native-image and ordinary Program profiles remain strict. No runtime, native
or platform support is claimed by this inspection API.

The explicit VM now covers payload extraction, failure reconstruction, bounded
value traversal, frame/operand cleanup and fuel tests. Executable source/native
integration remains open. Adapters and remaining E2/G13 requirements stay open. Task Manager is
unchanged.
