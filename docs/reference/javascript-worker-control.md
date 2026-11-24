# Cooperative control in generated Node workers

Status: **Implemented opt-in Node Worker generator; G13 remains partial.**
The user-approved [transport contract](../design/G13_JAVASCRIPT_WORKER_CONTROL.md)
adds worker-only control without changing ordinary JavaScript generators, CLI
commands, native images, source syntax or execution defaults.

```rust
use std::time::Duration;
use sovra::compiler::{backend, execution::RuntimePolicy};
let javascript = backend::render_javascript_worker_with_policy_and_control(
    &ir,
    RuntimePolicy::UNLIMITED,
    backend::JavaScriptWorkerControl {
        enable_cancellation: true,
        max_elapsed: Some(Duration::from_secs(2)),
    },
);
```

The trusted Node supervisor loads the generated CommonJS script in a Worker:

```javascript
const {Worker} = require('node:worker_threads');
const flag = new SharedArrayBuffer(4);
const worker = new Worker(javascript, {
  eval: true, // Alternatively, write the generated script to a .cjs file.
  workerData: {sovraControl: {version: 1, cancellation: flag}},
});
worker.on('error', error => { /* Record the execution error. */ });
worker.on('exit', code => { /* Handle source exit status or worker failure. */ });
// Call from the supervisor when cancellation is requested:
Atomics.store(new Int32Array(flag), 0, 1);
```

Cancellation is a permanent host-owned request: initialize zero, transition only
to nonzero and never reset it. Several workers may deliberately share a flag;
their fuel and elapsed origins stay independent. The generated program reads
but never resets the flag. Sovra source cannot access it. The supervisor remains
responsive while a worker executes synchronous source; a timer callback inside
that busy worker would not provide cancellation.

JavaScriptWorkerControl is Debug + Clone + Copy. UNLIMITED disables cancellation
and elapsed checks but still requires a Worker. When cancellation is enabled,
require exactly the envelope keys version and cancellation, version 1, and a
non-growable four-byte SharedArrayBuffer. Missing/extra/wrong fields, arrays,
ordinary buffers, views and other sizes fail with
`invalid JavaScript worker control`. Disabled cancellation requires the
sovraControl property to be absent, including no explicit undefined property.
Main-thread use fails with `JavaScript control requires a Node worker`.
Unrelated top-level workerData remains host-owned; fuel/deadlines are embedded in
the script and cannot be enlarged by that data.

Elapsed time uses process.hrtime.bigint and an exact BigInt nanosecond Duration,
including Duration::MAX. Its origin is script entry, excluding Worker creation,
JavaScript parsing and Rust compilation. Zero fails at the first checkpoint;
None performs no elapsed clock reads. Cancellation takes priority over elapsed,
with errors `execution cancelled` and
`execution elapsed time allowance exceeded`.

Invalid IR declaration errors precede worker setup. For valid declarations,
validate transport before source execution. Missing-entry/entry-arity errors
precede the first interruption check. Checkpoints precede frame setup and every
parameter copy, each dispatch before instruction fuel, empty returns and
fallthrough, host entry after argument/minimum-shape validation, successful host
return before full payload admission, and final success before source exit status.
Argv is an existing private process snapshot rather than a host callback.

Print preserves existing fuel/formatting order, then checks before its first
write and after the completed line. It does not check between partial writes.
A thrown host failure keeps its error; an ok:false text-result record is an
ordinary successful callback subject to post-call checks. Effects and admitted
charges can remain after failure. Refused checkpoints charge no additional fuel;
there are no refunds, retries, catch-and-resume paths or unlimited fallback.

The program uses normal Worker error/exit events, with no public message protocol.
Its successful source exit code belongs to the worker, not the supervisor's
process.exitCode. The supervisor must handle worker errors.

Current readSync(0)/writeSync(1) operations use inherited process descriptors.
Worker stdin/stdout proxy options do not redirect those direct operations.
Output can interleave; competing readers can consume input. Use a containing
subprocess with controlled descriptors for output qualification. Filesystem and
other process authority also remain shared. This is trusted Node embedding,
not per-worker I/O isolation, browser/mobile support or a process sandbox.

Checks cannot preempt host I/O, helper traversals, allocation, cleanup or GC.
A concurrent request just after a check can precede another operation. No fixed
stop latency, hard deadline or physical-memory cap is promised.

Actual Linux/Node v24.22.1 worker tests cover transport/error ordering, busy callee
cancellation, real elapsed exhaustion, private exact-clock boundaries, large
Duration, host effects/failures, complete partial-write publication, shared fuel,
registered package execution and 16,000-level parameter copies/refusal under a
0.5 MiB worker stack. Test supervisors have an independent kill deadline.
Node 22 CI, other operating systems, browser/mobile hosts, MSRV/release and bounded
native policy remain separate qualification gates.
