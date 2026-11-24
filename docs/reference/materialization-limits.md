# Optional logical materialization fuel

Status: **Implemented opt-in Rust and generated JavaScript; G13 remains partial.**
The user-approved [model v1 cost table](../design/G13_MATERIALIZATION_FUEL.md)
defines every event. `MATERIALIZATION_MODEL_VERSION` is 1; emitted JavaScript
retains the same version marker.

```rust
use sovra::compiler::{backend, execution::{ExecutionLimits, MaterializationLimits, RuntimePolicy}, interpreter};
let policy = RuntimePolicy {
    execution: ExecutionLimits::UNLIMITED,
    materialization: MaterializationLimits {
        max_value_nodes: Some(100_000),
        max_text_bytes: Some(1_000_000),
    },
};
let lines = interpreter::run_with_policy(&ir, policy)?;
let javascript = backend::render_javascript_with_policy(&ir, policy);
```

Host entry points are `run_with_host_status_and_policy` and
`run_function_with_host_status_and_policy`. All frames share the fresh invocation's
counters. Each field is `Option<u64>`: None disables it; Some(0) refuses positive
charges. Both limit structs and RuntimePolicy expose UNLIMITED. Existing APIs,
CLI commands, native image format and default profiles retain earlier behavior.

Each new scalar, Unit, array or record costs one node. Owned String UTF-8 bytes
and record type/field labels consume text fuel. Complete snapshot copies charge
the full footprint even when an engine physically shares strings. Containers
charge their node/labels only; transferred children are already paid. Dropping,
overwriting, moving or returning an existing value gives no refund and no new
charge. Empty returns, including root fallthrough, create one Unit. Numeric
widening costs zero; representation scratch is outside this logical model.

String concatenation and display-based producers premeasure text before building
it. Print charges one Unit and display text; its LF belongs only to the independent
output allowance. For `let original=["λ","😀"]; let copy=original; print(copy);`
inside main, successful completion requires exactly 11 nodes and 28 text bytes;
printed output independently needs 11 bytes. Ten nodes fails at the final root
Unit after output. Twenty-seven text bytes refuses print before output.

Validation precedes materialization admission. Instruction admission precedes
instruction effects. Both materialization dimensions are checked before either
is decremented; node exhaustion wins if both fail. Errors are
`execution value node allowance exceeded`,
`execution text materialization allowance exceeded`, or
`logical materialization size overflow`. JavaScript uses exact BigInt arithmetic.
Print checks output admissibility first, then materialization, committing both
before formatting/publication. Exhaustion has no unlimited fallback or rollback.

Host reads/writes preflight the known minimum result shape before calling the host.
The complete returned shape/payload is admitted afterward before VM publication.
Unknown payload exhaustion can therefore follow a completed host effect. Host
errors preserve their normal behavior; trusted host work/memory is outside fuel.

Exact/one-under/zero/unlimited Rust/Node tests cover table rows, Unicode, records,
snapshots, callee sharing, overwrite without refunds, atomic failure, validation
priority, host effects and a registered package consumer. Rust child processes
exercise iterative cleanup at 16,000 levels, including partially built frames.
Generated JavaScript also admits/rejects 16,000-level arrays and alternating
records under a 256 KiB stack, preserving both counters on rejected snapshots.
EOF, empty input and host-error tests verify minimum-shape admission ordering.
Default Linux native compatibility is separate from bounded execution evidence.

Logical fuel is not live heap, allocator capacity, RSS, total work, GC, host memory,
wall-clock interruption or process isolation. Scratch/metadata are excluded.
Physical memory limits, blocked-I/O cancellation, capability policy, bounded native
serialization, default profiles and other platform qualification remain open.
