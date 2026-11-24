# Optional runtime instruction and output limits

Status: **Implemented additive Rust and JavaScript APIs; G13 remains partial.**
The [resource contract](../design/G13_RUNTIME_BUDGETS.md) defines independent
per-invocation counters. Existing CLI commands, native-image format, native runner
and earlier execution APIs retain their existing policies; they do not enable
these optional counters automatically.

```rust
use sovra::compiler::{backend, execution::ExecutionLimits, interpreter};

let limits = ExecutionLimits {
    max_instructions: Some(100_000),
    max_output_bytes: Some(64 * 1024),
};
// `ir` is the checked/lowered program supplied by your trusted Rust host.
let lines = interpreter::run_with_limits(&ir, limits)?;
let javascript = backend::render_javascript_with_limits(&ir, limits);
```

Each field is `Option<u64>`. `None` disables that counter; `Some(0)` is a real zero
allowance. `ExecutionLimits::UNLIMITED` explicitly disables both. There is no new
bounded default profile. JavaScript uses BigInt to preserve exact u64 values.

Rust host variants are `run_with_host_status_and_limits` and
`run_function_with_host_status_and_limits`. The latter selects an existing lowered
zero-argument function for trusted tooling; it does not grant source-level access
or validate source permissions. Earlier public result shapes stay unchanged.
The JavaScript backend embeds the selected limits in the generated Node script.
A fresh invocation/script starts fresh counters shared by every callee; functions
cannot reset the counters when they call another function.

## Instruction admission

One instruction is charged immediately before each executed IR instruction. Calls,
jumps, explicit Return/Pop and emitted parameter-conversion instructions count.
Unreachable instructions and implicit fallthrough returns do not. A zero allowance
refuses the next instruction before its effects. An empty function can therefore
finish with zero allowance. Existing declaration validation still occurs first.

An exhausted run returns `execution instruction allowance exceeded`, or throws
that error in generated JavaScript. An IR loop cannot continue dispatching after
its finite allowance is exhausted. Host calls, copying, equality, formatting,
cleanup and declaration validation are not individually metered by this count.
It is not a total-work or wall-clock limit and cannot interrupt blocked host I/O.

## Printed output admission

Count logical UTF-8 bytes of each print/println line plus one LF. Embedded line
breaks count as ordinary payload bytes. Empty lines still consume one byte.
For example, printing `λ😀` requires seven bytes: six payload bytes and one LF.
The shared counter includes all earlier accepted lines, including callee output.

Reserve the LF and bound iterative formatting before constructing an oversized
line or calling the host. A refused line publishes no prefix. Exhaustion returns
or throws `execution output byte allowance exceeded`. Earlier output/host effects
remain and are not rolled back. Host errors retain their normal behavior. As with
ordinary execution errors, Rust does not return a successful partial RunOutcome;
a host can retain its own record of already accepted lines.

This counts printed-line payloads, not allocator capacity, captured Vec headers,
files, stderr, input or intermediate values such as `std::to_string` results.
An arbitrary Rust host can perform additional effects outside the counter.

## Scope and verification

Tests compare Rust and actual Node success/error/stdout for exact, exceeded, zero
and unlimited allowances; Unicode/LF and empty lines; cross-frame counters and
prior effects; large u64 limits; typed conversion prologues; malformed declarations;
and an independent registered-package consumer. Isolated 16,000-level array/record
cases verify iterative cleanup after instruction/output exhaustion. Infinite-loop
regressions have test-harness deadlines, not runtime preemption guarantees.

A default locked check/test/run/JS/native smoke preserves earlier behavior on the
actual Linux host. It does not qualify bounded native CLI artifacts or another
platform. Physical heap accounting, interruption, capabilities,
CLI policy, native serialized policy and release defaults remain separate work.
These APIs are component controls for a trusted embedding host, not a process
sandbox or permission to execute untrusted code with unrestricted host authority.


[Read-only logical value measurements](value-footprint.md) support the separate
[implemented materialization policy](materialization-limits.md). `RuntimePolicy`
composes both sets of optional counters without changing `ExecutionLimits`.


[Rust cooperative control](execution-control.md) can additionally check a shared
cancellation token and elapsed allowance at explicit interpreter boundaries. It
does not change fuel costs or enable hard interruption. JavaScript also has a
separate [opt-in Node Worker control API](javascript-worker-control.md).
