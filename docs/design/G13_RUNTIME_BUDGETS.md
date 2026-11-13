# G13: opt-in execution and output allowances

Status: **Additive instruction/output and approved logical materialization APIs implemented; physical-memory policy remains planned.** This first resource slice adds
explicit Rust APIs and a JavaScript generator option. Existing execution entry
points, CLI commands, native-image format and native runner defaults keep their
current behavior. No source syntax, ownership model or default limit changes.

## Contract

`compiler::execution::ExecutionLimits` has independently optional `u64` fields
`max_instructions` and `max_output_bytes`. `None` means unlimited for that dimension;
`ExecutionLimits::UNLIMITED` is explicit. Zero is a real zero allowance. Each fresh
invocation/script starts fresh counters, shared by all user-function frames.

- Charge one instruction immediately before dispatching each executed IR instruction,
  including calls, jumps, prologue conversion instructions, Pop and Return. Reject
  the next instruction when allowance is zero, before its effects. Fallthrough
  return and function-environment setup are not IR instructions. Unreachable
  instructions consume no allowance. This is an instruction count, not total work.
- Output counts UTF-8 bytes passed to `RuntimeHost::write_line`/generated stdout,
  including one LF per print/println. Reserve the LF and bound compound formatting
  before constructing an oversized line or invoking the host. Count empty lines,
  Unicode, standard record/array formatting and repeated calls cumulatively.
- Exhaustion returns a runtime error or throws in generated JS, without fallback,
  counter reset or publishing the refused line. Earlier host effects remain; there
  is no rollback. Host errors retain their existing error behavior. Invalid IR
  declarations still fail validation before execution.
- Counters use checked integer arithmetic; JS uses BigInt so u64 limits do not
  round through Number. Optional dimensions are independent and deterministic.

Rust adds `run_with_limits`, `run_with_host_status_and_limits`, and
`run_function_with_host_status_and_limits`. The backend adds
`render_javascript_with_limits`. Existing public result shapes and unlimited
entry points stay compatible. No CLI flags or bounded-native artifact claims are
part of this slice. A future CLI/default/serialized-policy change needs review.

## Boundaries

Instruction allowance does not meter work inside one instruction, value copying,
formatting, equality, host I/O, cleanup or declaration validation. It cannot cancel
a blocked host or provide a wall-clock deadline. Output allowance covers print/
println and retained printed lines, not files, stderr, input, intermediate strings
such as std::to_string, or arbitrary Rust host effects. It is a logical UTF-8 output
bound, not allocator/RSS accounting. Work-list/capacity/host allocations remain.
Cleanup of runtime-owned values remains iterative after exhaustion.

## Approved cumulative materialization model

[Model v1](G13_MATERIALIZATION_FUEL.md) now implements the approved cumulative
logical node/text accounting in both engines. Separate MaterializationLimits and
RuntimePolicy APIs preserve existing ExecutionLimits and default behavior.
Successful admission consumes fuel permanently, including snapshot copies;
dropping values gives no credits. See the [API](../reference/materialization-limits.md).
Physical heap/RSS, helper work, hard interruption, capability restrictions, native
serialization, default profiles and general collections remain separate work.

## Verification

Test exact/exceeded/zero/unlimited dimensions, Unicode/LF, empty lines, nested
records, shared callee counters, loop termination, values beyond Number precision,
prior effects, host errors and iterative cleanup after exhaustion. Compare Rust
and generated JS success, failure and stdout, while retaining old unlimited
behavior. Qualify actual hosts rather than inferring Windows/macOS/MSRV support.


See the [API reference](../reference/execution-limits.md) for call sites, exact
accounting, failure behavior and verification scope. No new default profile or
source-level compatibility decision was needed for these opt-in component APIs.


## Accounting boundary

The approved complete cost/effect table is implemented. Read-only footprint
measurement alone enables no quota. Logical fuel does not measure resident memory
or interrupt blocked host calls.


## Cooperative Rust control

The approved [cooperative interruption contract](G13_COOPERATIVE_INTERRUPTION.md)
now has an opt-in [Rust embedding API](../reference/execution-control.md),
monotonic elapsed checks, permanent shared cancellation and explicit checkpoint/
error order. Node transport has the separate approved implementation below;
browser transport and hard process deadlines remain separate.


The approved [Node Worker transport](G13_JAVASCRIPT_WORKER_CONTROL.md) now has a
separate opt-in [generator API](../reference/javascript-worker-control.md),
versioned shared atomic flag, exact monotonic elapsed checks and explicit inherited
process-stdio boundaries. Ordinary scripts/defaults remain unchanged. Browser
transport and hard termination remain separate.
