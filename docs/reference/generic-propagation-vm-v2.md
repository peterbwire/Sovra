# Generic propagation VM (V2)

`execute_generic_propagation_ir` explicitly executes the separate
[checked early-return IR](generic-propagation-ir-v2.md) with `GenericVmPolicy`.
This library API has no host I/O. Ordinary `execute_generic_ir`, ordinary full-source, CLI and native-image
profiles remain strict for question-bearing source tables. The explicit
[propagation source profile](generic-propagation-source-v2.md) now composes this API.

Each executed question dispatches one instruction after evaluating its operand
once. Success consumes its standard wrapper and copies the payload using the
existing bounded snapshot operation. Failure copies any error payload and builds
a new wrapper using the enclosing function's return layout and failure tag.
Option failure is nullary; Result failure retains its exact payload type, including
Unit. Different input and return success payloads remain valid.

Failure abandons only the current frame. Pending operands and local slots in that
frame are released; the returned wrapper is delivered to the caller, which resumes
normally. A caller with its own question may then propagate separately. Counts for
active slots and operands are reduced before the caller receives the result.
Owned values are flat trees, so destruction does not recurse through payloads.
Early-return cleanup admits slot/operand scanning and value-node work before
abandoning the frame and observes cooperative cancellation at each admission.

Snapshot node/text costs, the new wrapper node and all copying/traversal work
consume the same cumulative instruction/materialization/work/control policy as
ordinary operations and calls. Cleanup never refunds materialization allowances.
Exhaustion or cancellation returns an error with original source provenance and
no partial outcome. A returned outcome retains its checked IR for inspection;
passing that IR to the ordinary VM still refuses a question-bearing source table.

Qualification covers snapshots, Result failure reconstruction, structured errors,
Unit payloads, nested questions, short circuits, repeated frame cleanup, shared
exact budget boundaries and cancellation. Its original qualification does not qualify executable source,
CLI, native outputs or other platforms. Source qualification is recorded separately.
Remaining integrations, explicit adapters and
remaining E2/G13 requirements remain open. Task Manager is unchanged.
