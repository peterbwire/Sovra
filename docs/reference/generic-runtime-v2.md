# Host-free execution of generic IR

`execute_generic_ir` runs [checked generic IR](generic-ir-v2.md) in Rust. It accepts
a sealed program whose root has no value parameters and returns a
`GenericVmOutcome`. Internal generic calls support the full current checked body
profile. This library API grants no stdin, filesystem, output, network or process
authority. [Standalone generic source execution](generic-source-v2.md) now composes
the stages for admitted files, with [explicit CLI check/run](generic-cli-v2.md).
[Generic native packaging](generic-native-v2.md) also uses this VM.
Program/package and JavaScript integration remain separate; ordinary command/runtime behavior is unchanged.

The VM executes constants/Unit, primitive arithmetic and structural equality,
String operations, arrays, records, enums, mutable locals/indexed replacement,
field/index snapshots, concrete calls, branches, loops and exhaustive matches.
It follows the emitted short-circuit branches and evaluates match scrutinees once.
Payload bindings retain immutable snapshots when a parent local is replaced.
Integer overflow and division by zero refuse; negative/unavailable indexes refuse.
Numeric widening and runtime IEEE Float behavior preserve the existing numeric
contract, including overflow to Infinity. Literal constants remain finite.

Calls use an explicit frame vector, including direct/mutual recursion, without
recursive Rust invocation. Parameter values transfer into incoming slots; returns
transfer owned values out. All frames share the same instruction, materialization,
work and cancellation/elapsed policy. The root counts toward the active frame cap.
Local slot storage is admitted before frame allocation, including uninitialized
lexical slots. Operand caps apply to actual active operand values across frames.
The compile-time maximum stack also guards each frame's pushes.

Values use private flat postorder arenas with cached subtree node/text footprints.
Arrays/records/enums reference child indices rather than recursive Rust values.
Snapshots copy admitted contiguous subtrees iteratively; field/variant extraction
and compound equality are iterative. Declaration-order record fields can have a
different physical arena order without changing equality or field access. Indexed
replacement transfers owned nodes, compacts the arena and maintains subtree
intervals/footprints. Other array values remain independent snapshots. Ordinary
result, replacement, frame-return and error cleanup do not recursively destroy
compound language values.

Materialization uses the existing `RuntimeValueFuel` and logical cost conventions:
new scalars/Unit/array/record/enum wrappers cost one node; existing children move
without a second charge. String UTF-8 bytes and original nominal type/field/variant
label bytes are charged, even when schema labels are physically shared. Snapshots
charge their full subtree footprint before copying. Widening, moving, overwriting
and returning existing values create no new materialization charge or refund.
Array constructors retain the existing standard array-length ceiling. Temporary
representation/frame buffers are outside logical materialization, with separate
work/active-slot/operand limits; these units are not a physical heap/RSS measure.

`GenericVmPolicy::DEFAULT` is a new finite library profile: 1,000,000 instructions,
1,000,000 materialized nodes, 16 MiB logical text, zero output bytes, 256 active
frames, 65,536 active local slots, 65,536 active operand values and 16,000,000 work
units. Existing `RuntimePolicy` components can be selected explicitly; frame depth
must remain in 1..=256. Work includes dispatch, frame setup, traversals, copies,
comparisons, compaction and String processing. Long node traversals check existing
cooperative control. String helpers, allocation and cleanup are not hard-interrupted;
this is not a process sandbox or hard CPU/heap deadline. Failure publishes no
partial successful result and has no legacy/unlimited/backend fallback.

`GenericVmOutcome` retains the checked IR and owned result and exposes remaining
shared policy, consumed work/instructions and peak frames/slots/operands.
`value()` returns an allocation-free borrowed `GenericValueRef` with primitive,
array, field, tag, payload and cached footprint inspection. A nullary payload is
None; a Unit payload remains Some. Layout indices belong to the outcome's retained
IR. No unmetered owned-value Clone or external value-ingress API is introduced.
Inspection is trusted host code outside VM instruction/work accounting.

Errors identify the concrete function, instruction, original source span and
execution/materialization/limit/arithmetic/bounds/entry/consistency category.
Cancellation and zero elapsed allowance refuse before entry allocation. Root
parameters return `RootArguments` rather than accepting unchecked host values.

Tests: `tests/generic_runtime_v2.rs` executes actual VM programs for factorial,
loops, skipped failing RHS calls, strings/widening, reordered record equality,
enum payloads, parent/callee snapshot isolation, repeated nested array replacement,
2,000-level recursive values, exact/one-under shared limits and cooperative/frame
refusals. This exceeds the earlier scalar-only IR test probe but does not qualify
other backends or hosts.

Remaining work includes source/program/package/CLI and native/JavaScript integration,
validated external argument ingress, standard Option/Result and structured errors,
remaining G13 libraries and consolidated exact-host qualification. G13 E2 remains
incomplete.
