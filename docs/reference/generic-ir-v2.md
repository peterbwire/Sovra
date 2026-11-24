# Typed generic instruction lowering

`lower_generic_ir` consumes [checked nominal plans](generic-construction-v2.md)
and emits owned stack instructions for every concrete function instance. It
resolves function-local labels and validates instruction structures and reachable
typed control-flow joins before returning `CheckedGenericIr`. This is an
executable-form compiler IR, not a VM, native image codec or public Program/CLI
integration. No generic source is executed by this API.

Instructions cover decoded constants, Unit materialization, binding snapshots,
local initialization/assignment, indexed mutable replacement, explicit numeric
conversion, primitive/structural binary operations, arrays, nominal constructors,
field/index reads, concrete calls, branches, loops, enum dispatch and returns.
Record operands retain source evaluation order and their declaration-order slots.
Enum payload presence retains the nullary/Unit distinction. Numeric conversions
remain explicit instructions; Float constants retain finite binary64 bits and
String constants share the original immutable decoded payloads.

Lowering walks expression trees and original nested statement ranges iteratively.
It does not execute the global expression table as a linear stream. `&&` and `||`
branch around their right-hand expressions; loop test addresses re-evaluate the
condition on every iteration. Match dispatch evaluates one source scrutinee
snapshot, retains it for tag selection, then enters a tag-matching `MatchBind`
that consumes the snapshot and initializes the original payload slot. Arm ranges
become explicit instruction addresses without fallthrough to other arms.

`GenericIrFunction` exposes incoming argument slots, local layouts/origins, the
result layout, resolved instructions and maximum reachable operand-stack depth.
Incoming arguments initialize parameter slots. Local initialization can re-enter
its original lexical slot in a loop; assignment requires a mutable-local origin.
Calls pop source-order arguments into the target instance's parameter slots and
produce one result value, including Unit. Returns consume exactly one result and
leave no operand values. Implicit Unit fallthrough emits Unit/Return. A terminal
Unreachable sentinel makes missing exits a validation error instead of falling
out of the instruction stream.

Every function/slot table is checked before any control-flow pass indexes a callee
signature, including recursive callees. All instruction addresses, record slots,
enum tags, payload destinations, constants and call arities are structurally
checked. Nonfinite constant bits refuse. A Switch target must start with the
matching enum layout/tag's MatchBind. Reachable dataflow uses canonical persistent
typed stacks: joins must agree on the entire layout stack, not merely its height.
Underflow, invalid operand/result types, invalid returns and reachable terminal
sentinels refuse. Loop joins and direct/mutual calls do not recursively expand the
compiler's call stack. Unreachable instructions remain structurally checked and
retain the earlier sealed source/type proof; fresh stack dataflow starts at each
function entry. This API does not ingest arbitrary caller-supplied bytecode.

`CheckedGenericIr` retains the complete source/type/layout/nominal proof chain and
functions in concrete graph order; root index is zero. Instruction spans refer to
the original function source. Slot and instruction addresses are function-local;
layout indices and call targets belong to the enclosing program. These indices
are not a serialized ABI. Clones preserve the original source owners and decoded
payload sharing.

`GenericIrLimits` bounds functions, instructions, local slots, stack/local operand
references, temporary labels, explicit branch targets, simultaneous pending
emission tasks, canonical validation-stack nodes, per-function stack depth and
logical work. Fan-out tasks are admitted before allocation; call mapping storage
is bounded by the remaining instruction allowance. Typed stack states are shared
instead of copying a whole type vector at every instruction. `usage()` includes
peak pending tasks and exact cumulative counts. Exhaustion returns no partial IR
and has no legacy/execution fallback. These are compiler metadata/work allowances,
not runtime instruction, allocation, stack-frame or hard interruption limits.

Tests: `tests/generic_ir_v2.rs` uses an independent scalar-only instruction probe
to verify loops, branches, short-circuit skipping and recursive factorial results;
it is not a production VM. Other tests cover arrays/nominals/conversions/matches,
source ownership, exact/one-under limits and nested control flow. Private negative
regressions reject equal-height/different-type joins, invalid targets, underflow,
empty code and nonfinite constant payloads.

The separate [host-free Rust VM](generic-runtime-v2.md) now executes this IR with
flat owned values, iterative frames and shared resource/control policy. Remaining
work includes native/JavaScript backend and source/program/package/CLI integration, standard Option/Result and structured errors. G13 E2
remains incomplete.
