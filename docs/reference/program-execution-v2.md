# Linked root-program Rust execution

Status: **Partial G13 E1 executable root programs; opt-in Rust API.**
[Checked programs](program-source-v2.md) can now execute their canonical function
links. Select an owned ProgramFunctionId, then call CheckedProgramSource::execute
with explicit entry arguments, ProgramLoweringLimits and RuntimeVmPolicy.
execute_with_services additionally supplies explicit program arguments and
independently granted input/files. No default process host or CLI selection changes.

Each iterative frame selects its function's own expression graph, instruction
stream, scopes, binding IDs, roots and match snapshots. Forward calls, self-calls
and mutual recursion use canonical IDs and callee parameter/return validation.
Caller evaluation resumes at its retained waiting register. Callee arguments
snapshot under the shared fuel; returns transfer values. Every frame shares one
instruction/output/materialization budget, cooperative clock/token, call-depth
allowance, command status and explicitly supplied services. No counter resets or
refunds occur on a call/return. Deep owned value/frame cleanup remains flat.

Before entry snapshots or source effects, the engine checks all function owners,
validates every IR and preflights every call/service capability, including unused
functions. A foreign entry refuses before lowering; mismatched runtime function
IDs/owners refuse without string dispatch or another engine. Standalone FunctionIr
execution still refuses program targets: only checked-program execution supplies
the complete owned table. Captured output/status are returned only on success;
completed explicit host effects survive later cancellation or admission refusal.
Blocking hosts and helper walks are not hard-interrupted.

ProgramLoweringLimits inherits existing per-function compiler quotas and adds
cumulative mapping-node, fact-link, expression-register and instruction allowances.
The instruction allowance counts both retained expression and function streams,
including unused functions. Each stage receives the remaining aggregate allowance
before emission; results are charged before retention. Zero and one-under limits
refuse without partially executed source or fallback. Per-function validation/work
limits remain per function; they are not a new physical heap/complete-work bound.
Lowering all functions occurs before starting the VM elapsed clock and is currently
repeated for each invocation. The VM validation/control clock then spans all function
validation, preflight and execution. Instruction fuel counts dispatched versioned
instructions; unused functions' instructions do not consume execution fuel.

The [independent enum-functions fixture](../../examples/enum-functions/main.svr)
prints 14 using a forward enum/record factory and numeric helper. Tests cover mutual
recursion, local scope/snapshot isolation, numeric coercion, legacy common-cost/
output parity, exact aggregate lowering allowances, shared limits/services/status,
unused-service preflight and post-write cancellation without rollback.

This establishes root multi-function Rust execution, not completed E1/G13 or a
release/toolchain qualification. [Recursive matches inside arms](recursive-match-v2.md)
now execute through this engine, including [if/while bodies](control-match-v2.md).
[Explicit CLI execution](experimental-v2-run.md) now selects this engine with finite
limits and no stdin/file services. Modules/imports/packages/tooling,
JavaScript/Worker/native encoding and broader host/platform
qualification remain open under the [completion contract](../design/G13_COMPLETION_CONTRACT.md).

The same complete linked lowering now supports [bounded deterministic IR
inspection](experimental-v2-ir.md) without execution or host preflight.
