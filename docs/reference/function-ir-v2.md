# Versioned single-function control-flow IR

Status: **Partial E1 function-emission foundation.**
`ExpressionIr::emit_function_ir` emits and validates function instructions around
[checked expression graphs](expression-ir-v2.md). A separate [opt-in Rust VM](runtime-vm-v2.md) now executes it with a restricted
standard adapter. JavaScript/native backends, serialization and CLI integration
remain open.

Instructions load parameters, enter/exit scopes, evaluate expression roots, apply
deferred initialization/assignment/return contexts, bind/store snapshots and
perform indexed stores. Expression roots are verified as closed contiguous
segments owned by their mapped source tree. Indexed stores evaluate the base,
index and replacement in source order. Explicit root releases clean statement
temporaries and associated boundary conversions.

Condition branches test and release Bool roots. Loops recompute their conditions
and use real backward edges; branches have patched function-relative targets.
A direct match snapshots its bound scrutinee once, dispatches its nominal tag
without materializing a Bool, enters an isolated arm scope, and binds the selected
payload snapshot. Arm completion exits the scope and releases the match snapshot.
Returns require runtime unwinding of all active scopes, roots, conversions and
match snapshots. Implicit Unit completion follows root-scope exit; reachable
non-Unit fallthrough is refused.

The function retains its checked expression/source owner. Registers remain in
that expression graph; function operands reference intrinsic roots or checked
boundary-context results. These indexes, scope labels and jump targets are local
to this sealed function, not external executable handles. Declaration and enum
identities retain their checked unit/environment ownership. Canonical return types
come from checked return contexts, with Unit for source functions that have no
value-return context. No new expression type inference or lexical name lookup is
used during emission.

Validation first checks expression proof and static function targets, parameter/
local/payload coverage, boundary contexts, assignment permissions/types, return
types and exact exhaustive switch variants. A bounded worklist then checks every
reachable path for active bindings, evaluated roots/conversions, balanced scope
joins, available snapshots and variant-tag guards before payload extraction.
Availability/tag facts intersect at joins, while possibly live resources union;
this prevents a release missing on only one branch from disappearing at a join.
Loop states converge conservatively. Unreachable source
still has static shape/type/coverage checks, while dynamic availability checks
apply to reachable instructions. Returns are terminal and permit active state
because the future runtime must unwind it. Completed Unit paths require clean
scopes, roots, conversions and snapshots.

`FunctionIrLimits` bounds retained instructions, explicit jump/branch/switch edges,
active scope frames, validation steps and cumulative copied flow-state entries.
Validation steps count static/reachable instructions, switch arms, scanned
expression instructions and propagated edges. State entries include a base entry,
scope frames/bindings, available and possibly live roots/conversions/snapshots,
and tag facts, charged before
state copies. `FunctionValidation` reports actual charged steps/items. Expression
validation has its own inherited allowance. Initial indexes/state slots and merge
bookkeeping remain bounded by admitted source/instructions but are not all charged
as work steps. These are logical bounds, not physical heap or complete-work limits.
Zero caps are real; refusal returns no partial function or legacy fallback.

A separate [model-v2 value layer](runtime-values-v2.md) now provides flat owned
values, nominal enum operations and atomic logical fuel. Function execution,
checked arithmetic/bounds and shared component policies are now covered by the
initial Rust VM, including shared materialization, output and cooperative control
limits. Full standard/host adapter qualification, Rust/JavaScript/native parity
and integration remain required. Nested matches,
other user functions/complete programs, packages and tooling remain open under the
[approved completion contract](../design/G13_COMPLETION_CONTRACT.md). Existing v1
IR/Value/native formats and CLI behavior remain unchanged.
