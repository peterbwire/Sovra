# G13: cooperative interruption for trusted Rust embedding

Status: **Approved by the user and implemented for opt-in Rust embedding.**
See the [API reference](../reference/execution-control.md) for verified behavior.
Existing [instruction/output](../reference/execution-limits.md) and
[logical materialization](../reference/materialization-limits.md) controls remain
implemented independently. This proposal does not change their accounting model.

## Approved first slice

Add explicit cooperative cancellation and elapsed-time checks to the Rust
interpreter embedding API. A trusted desktop, server or tooling host can request
that a run stop at the next interpreter checkpoint. Preserve all existing entry
points, source syntax, RuntimePolicy fields, CLI defaults and native-image format.
This is a Rust-host feature first. Generated JavaScript remains on its existing
policy; no cross-engine cancellation or browser/mobile runtime support is claimed.

This scope is useful without pretending a synchronous interpreter can forcibly
interrupt arbitrary host code. A host that needs a hard deadline must still use
its own process isolation and termination mechanism.

## Implemented API

- `CancellationToken`: Clone + Debug; shared Arc<AtomicBool> state, initially false.
  `new()`, `cancel()` and `is_cancelled()`. Cancellation is permanent; no reset.
  Clones can be sent to a supervising thread. A token can deliberately cancel
  several invocations; it does not share their fuel or elapsed-time origins.
- `ExecutionControl`: Clone + Debug, with optional CancellationToken and optional
  Duration `max_elapsed`. Both absent means no control. Expose an UNLIMITED value.
  This stays separate from the Copy limit structs and serialized policies.
- New `run_with_policy_and_control`, `run_with_host_status_and_policy_and_control`
  and `run_function_with_host_status_and_policy_and_control` entry points.
  Existing APIs retain behavior and result/error shapes.

Start a monotonic Instant at invocation entry. Parsing/compilation is outside the
run; initial IR declaration validation keeps its existing diagnostic priority.
Elapsed validation time counts toward the first checkpoint, but validation itself
is not interrupted. Each invocation uses a fresh origin, shared across callees.
Avoid constructing an absolute Instant by addition: compare elapsed Duration,
so enormous durations cannot overflow deadline construction. Zero duration fails
at the first checkpoint, including for an otherwise empty main.

## Checkpoint and failure contract

Check cancellation first, then elapsed time. Use errors `execution cancelled` and
`execution elapsed time allowance exceeded`. The first checkpoint occurs after
initial declaration/entry validation and before constructing argument frames.

| Boundary | Required ordering |
| --- | --- |
| Each IR dispatch | Control checkpoint, existing instruction admission, existing operation validation/materialization/effects. |
| User-call frame creation | Check before frame setup and each parameter snapshot; share the invocation control. |
| Empty return/fallthrough | Check before creating the implicit Unit, including root completion. |
| Host call | Check immediately before entering the host, after ordinary argument validation and minimum-shape admission. |
| Successful host return | Check before publishing the returned VM value or successful run result. Unknown payload admission follows the checkpoint. |
| Failed host return | Preserve the host error; cancellation does not replace an already returned host failure. |
| Run completion | Final checkpoint before reporting success; earlier effects remain visible. |

A refused checkpoint consumes no new instruction/node/text/output fuel. Fuel
already admitted earlier is not refunded. Cancellation during an admitted print
or other host operation can leave that operation's effects and charges in place.
No prefix is deliberately published by a refused print. Cancellation is observed
only when checked: a concurrent request just after a checkpoint can precede one
more operation. Do not advertise instantaneous cancellation or a fixed latency.

Checks do not throw from Drop, interrupt cleanup or poll recursively. Runtime-owned
cleanup remains iterative. Validation, one-instruction helper traversals, large
copies/formatting/equality/conversion, allocation and blocked host I/O can run
until their next boundary. Total work and peak physical memory remain unbounded
by this contract. Checking a deadline before/after I/O cannot cancel the I/O.

## Implemented qualification

1. Pre-cancelled and zero-duration empty runs refuse before creating values or
   publishing output. Invalid declarations preserve their existing errors.
2. Cancellation wins over a simultaneous elapsed/instruction/node/text refusal.
   Disabled control preserves all existing policy tests and APIs.
3. A host cancels during successful read/write/print: effects remain, returned
   values/success are refused at the documented checkpoint. Failed host calls
   preserve their own errors. No run silently retries without control.
4. Callee loops share control; clone cancellation is permanent across threads.
   Use deterministic handshakes and test-harness kill deadlines for live loops.
5. A private injectable clock verifies exact elapsed boundaries and very large
   durations without timing-sensitive sleeps or a new public clock abstraction.
6. Deep array/record errors and partially copied frames clean up iteratively.
   Retain ordinary interpreter/Node policy parity and default Linux native smoke.

JavaScript needs its own reviewed embedding/checkpoint transport: a synchronous
Node event loop cannot process an ordinary timer or message callback during a
busy run. A future worker/shared-atomic flag or trusted synchronous callback must
have explicit ownership, availability and failure behavior. Do not add an
ambient global hook, filesystem polling or hidden process authority to simulate
portable cancellation. CLI/native serialization/default profiles, host capability
restrictions, full-work metering and physical memory isolation remain separate.

## Review boundary

[AGENTS.md](../../AGENTS.md) requires a concrete reviewed contract before broader
full-work/interruption policies, and says: "Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions."
The user explicitly approved this Rust-only control API, checkpoint/error order
and limitations before implementation. Do not request the same approval again.
The separate [Node Worker contract](G13_JAVASCRIPT_WORKER_CONTROL.md) has since
been explicitly approved and implemented. Hard process termination, browser
control transport and new default/native policies remain separate decisions.
