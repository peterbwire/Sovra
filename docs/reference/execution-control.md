# Cooperative execution control for Rust hosts

Status: **Implemented opt-in Rust embedding API; G13 remains partial.**
The user approved the [checkpoint contract](../design/G13_COOPERATIVE_INTERRUPTION.md).
Existing APIs, RuntimePolicy fields, CLI/native defaults and generated JavaScript
retain their behavior. JavaScript has a separate opt-in [Node Worker control API](javascript-worker-control.md);
ordinary scripts still have no cancellation transport.

```rust
use std::time::Duration;
use sovra::compiler::{
    execution::{CancellationToken, ExecutionControl, RuntimePolicy},
    interpreter,
};
let token = CancellationToken::new();
// Give token.clone() to a trusted supervisor; token.cancel() requests a stop.
let control = ExecutionControl {
    cancellation: Some(token.clone()),
    max_elapsed: Some(Duration::from_secs(2)),
};
let lines = interpreter::run_with_policy_and_control(
    &ir, RuntimePolicy::UNLIMITED, control,
)?;
```

The host variants are `run_with_host_status_and_policy_and_control` and
`run_function_with_host_status_and_policy_and_control`. The latter selects an
existing lowered zero-argument function for trusted tooling, with no new source
visibility or permission rules. Return/error shapes remain unchanged.

CancellationToken is Clone + Debug, initially uncancelled, with `new`, `cancel`
and `is_cancelled`. Cancelling any clone permanently requests cancellation for
all invocations sharing it; no reset exists. It can be sent between threads.
Each invocation gets fresh logical fuel and a fresh monotonic elapsed origin;
callee frames share both. ExecutionControl is Clone + Debug with optional token
and optional Duration. UNLIMITED disables both controls. It is separate from the
Copy limit structs and is not serialized into native artifacts.

The elapsed origin starts at run entry. Initial IR declaration, entry lookup and
entry-arity validation retain diagnostic priority. Declaration validation time
counts toward the first check but validation itself is not interrupted. Elapsed
Duration is compared directly against the allowance, avoiding absolute-deadline
addition overflow. Zero refuses the first checkpoint, including an empty main.

At each checkpoint cancellation wins, then elapsed exhaustion. Errors are
`execution cancelled` and `execution elapsed time allowance exceeded`.
Checkpoints occur before initial/callee parameter setup and each parameter
snapshot, before every IR dispatch (before instruction fuel), before implicit
Unit returns/fallthrough, immediately before entering a host after argument and
minimum-shape checks, after successful host callbacks before publication/ingress,
and before reporting run success. Argument-list host callbacks are covered too.

A host `Err` keeps its own error even if the host also requests cancellation.
TextRead/TextWrite `ok:false` records are ordinary successful callback returns;
they still require the post-call checkpoint before VM publication. Cancellation
or elapsed failure after a callback can preserve completed host effects. An
admitted print can consume fuel and publish its line before cancellation is
observed afterward. There is no rollback, refund, retry or unlimited fallback.
A refused checkpoint consumes no additional logical fuel.

These are cooperative checks, not hard deadlines. A request just after a check
can precede another operation. Checks cannot interrupt validation, one-instruction
helpers/copying/formatting/equality/conversion, allocation, cleanup or blocked I/O.
Cleanup remains iterative and does not poll or fail midway. No fixed response
latency, total-work bound, physical-memory cap or process sandbox is promised.
Use host process isolation/termination when hard limits are required.

Private clock tests verify exact elapsed boundaries, large durations, dispatch
and return priority, no charge at refusal, host-result ordering and committed
print fuel. A supervised callee regression uses a test-process kill deadline;
that deadline is a harness control, not an interpreter guarantee. Isolated
16,000-level cases cover cancelled host output and partially copied frames.
Actual Linux default native compatibility is verified separately; other host,
MSRV, release and bounded-native qualification remain independent gates.
