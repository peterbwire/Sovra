# G13: cooperative control in generated Node workers

Status: **Approved by the user and implemented as an opt-in Node Worker generator.**
See the [API reference](../reference/javascript-worker-control.md) for verified scope.
The [approved Rust control](G13_COOPERATIVE_INTERRUPTION.md) is implemented.
Its approval explicitly leaves JavaScript transport as a separate decision.
This approved contract adds a separate worker generator without changing existing generators,
source semantics, CLI commands, native images or runtime defaults.

## Approved transport

Generate an explicit Node Worker script whose supervisor supplies a shared
atomic cancellation flag. The worker polls synchronously at interpreter-equivalent
checkpoints. The supervisor's event loop can update the flag while the worker is
busy. Use a monotonic nanosecond clock directly; do not rely on a timer callback
executing inside the busy worker. No ambient callback/global hook, filesystem
polling, dynamic source evaluation by Sovra or source-level control API is added.

This is Node-specific trusted embedding. It establishes neither browser/mobile
support nor a process sandbox. Worker code retains the existing Node filesystem,
argument and stdio authority; worker threads share a process and its resources.
Hard termination remains the supervisor's separate responsibility and is outside
this cooperative contract.

## Implemented additive Rust generator API

In compiler::backend, expose a Debug + Clone + Copy configuration:

```rust
pub struct JavaScriptWorkerControl {
    pub enable_cancellation: bool,
    pub max_elapsed: Option<std::time::Duration>,
}
// UNLIMITED: enable_cancellation = false, max_elapsed = None.
pub fn render_javascript_worker_with_policy_and_control(
    program: &IrProgram,
    policy: RuntimePolicy,
    control: JavaScriptWorkerControl,
) -> String;
```

Existing `render_javascript`, `render_javascript_with_limits` and
`render_javascript_with_policy` retain their ordinary script format and behavior.
The new generator emits worker-only code even for UNLIMITED. It does not expose
new CLI flags or serialize controls into native artifacts. No cancellation token
is passed from Rust to Node; the trusted Node supervisor owns that transport.

Emit elapsed Duration as an exact BigInt nanosecond total from seconds and
subsecond nanoseconds. Do not convert it through Number or an absolute timestamp.
Compare `process.hrtime.bigint() - origin >= allowance`. Zero fails at the first
checkpoint; None skips clock checks. Duration::MAX remains representable as a
BigInt. Start the origin at script entry before generated setup/validation work;
Worker creation, JavaScript parsing and Rust compilation are outside that origin.

## Version 1 host envelope and lifetime

When enable_cancellation is true, require the trusted supervisor to provide:

```javascript
const flag = new SharedArrayBuffer(4);
const cancellation = new Int32Array(flag);
const worker = new Worker(generatedScript, {
  eval: true, // Node loads compiler-produced script; Sovra source has no eval API.
  workerData: {sovraControl: {version: 1, cancellation: flag}},
});
// The supervisor may call this while the worker is busy:
Atomics.store(cancellation, 0, 1);
```

Use an ordinary CommonJS worker, including a `.cjs` file alternative to eval.
Require `isMainThread === false`. Invalid main-thread use throws
`JavaScript control requires a Node worker`. An enabled cancellation envelope must
be an object with version exactly 1 and a non-growable SharedArrayBuffer of exactly
four bytes. Reject arrays, missing/wrong-version fields, ordinary ArrayBuffer,
views and larger/growable buffers with `invalid JavaScript worker control`.
Do not coerce fields or silently disable checks. Extra envelope keys are rejected;
unrelated top-level workerData keys stay host-owned and inaccessible to source.

When cancellation is disabled, no sovraControl envelope is required. If an envelope
is supplied anyway, reject it as invalid configuration rather than silently
ignoring a purported cancellation token. Elapsed/fuel policy is embedded in the
script and cannot be enlarged through workerData.

Atomics.load(flag, 0) !== 0 means cancelled. The host must initialize zero and
only transition to nonzero; it must never reset or reuse cancelled state for a new
uncancelled invocation. Source code cannot access the flag. The worker only reads
it, retains a view for its invocation and cannot detach it. A trusted supervisor
can deliberately share one flag across workers; each still has independent fuel
and elapsed origin. No growth, reset, message polling or asynchronous callbacks
are part of the token contract. Atomics here transports cancellation only, not
arbitrary host-data synchronization.

## Checkpoint and failure ordering

Preserve the approved Rust checkpoint/error contract wherever the engines share
operations. Cancellation precedes elapsed time, then existing instruction/fuel and
operation behavior. Errors remain `execution cancelled` and
`execution elapsed time allowance exceeded`.

1. Invalid IR declarations preserve the existing generated declaration error
   before worker/control setup. For a valid declaration set, validate worker mode
   and transport before executing source. Existing missing-entry and entry-arity
   errors precede the first interruption check.
2. Check before user frame setup and each parameter snapshot, each IR dispatch
   before instruction fuel, empty Return/fallthrough before Unit creation, and
   final success before setting the worker's exit status.
3. For host operations, validate arguments and preflight the known minimum result
   shape first. Check immediately before entering the host and after a successful
   return before VM result publication/full payload admission. For print, preserve
   output/materialization admission and formatting order, then check before the
   first write and after the completed logical line. Do not insert cancellation
   checks between partial low-level writes of an admitted line.
4. A thrown host error retains its normal failure; do not replace it with a later
   interruption error. TextRead/TextWrite records with ok:false are ordinary
   successful callbacks and still need post-call checks. A post-call refusal may
   follow an already completed effect; admitted charges remain consumed.
5. A refused checkpoint spends no additional fuel. Never retry unlimited, reset
   counters, refund committed charges or catch-and-resume interruption errors.

No public worker-message protocol is introduced. Successful completion uses the
existing source exit status as the worker's exit code; errors use ordinary Worker
error/exit events. The supervisor must handle those events. Cooperatively aborted
runs have no successful RunOutcome or automatic rollback. No changes to the
supervisor's process.exitCode are made by the generated program.

## Existing host boundary that must remain explicit

The current backend uses fs.readSync(0) and fs.writeSync(1), not worker stream
proxies. Those are inherited process descriptors shared with the supervisor.
`stdout: true` or `stdin: true` on Worker does not redirect these direct operations.
Worker fd-1 output can interleave with the supervisor/other workers; input can be
consumed by competing readers. Keep those behaviors explicit rather than claiming
per-worker capture/isolation. Qualify output through a containing subprocess with
controlled descriptors. A future per-worker host adapter is a separate contract.

Workers also share filesystem authority and other process resources. Cooperative
checks do not preempt fs calls, helper traversal, allocation, cleanup or garbage
collection. Cancellation can race with the next operation after a check; no fixed
latency, physical memory bound, transaction or authority restriction is promised.

## Implementation and qualification gates

- Add an explicit internal generator mode and lexical checkpoint helper. Do not
  rewrite finished scripts with textual replacements or modify ordinary scripts
  to depend on globals. Keep public stage types and existing generator signatures.
- Test exact zero/disabled/pre-cancelled controls, invalid envelope shapes and
  worker-only enforcement. Validate declaration/entry/arity error priority.
- Execute real generated busy-callee programs in actual Node workers. A separate
  supervisor signals the flag and retains its event loop; use deterministic
  readiness handshakes and an independent test-process kill deadline. A timer in
  the worker must not be the cancellation mechanism.
- Use private injected clocks for exact elapsed and large-duration tests, and
  real hrtime checks for eventual busy-loop exhaustion. Do not assert fixed
  cancellation latency or cross-engine timing equality.
- Compare Rust/worker error/output/effect ordering on deterministic fixtures,
  including host failure versus post-success cancellation and pending ingress
  exhaustion. Retain caller-shared fuel and model-v1 exact-cost tests.
- Verify 16,000-level values under a reduced Node stack, including parameter-copy
  refusal, and no unmetered fallback. Keep normal Node scripts, full Rust checks
  and default Linux native compatibility passing. Collect actual Node 22 CI and
  other-platform evidence separately; a local probe is not qualification.

## Transport evidence before implementation

A standalone experiment on Linux with Node v24.22.1 verified six categories:
busy-worker atomic cancellation; an already-set flag winning over zero elapsed;
monotonic elapsed exhaustion inside a busy loop; a Duration::MAX-sized exact
BigInt allowance; invalid buffer/version refusal; and inherited fd-1 output with
worker-local exit status. It did not execute generated Sovra code or implement
this API. Evidence is retained in
`target/checked-evidence/g13-worker-transport-review.json`.

## Approval boundary

[AGENTS.md](../../AGENTS.md) requires a concrete reviewed contract before broader
interruption policies, and the approved Rust contract explicitly defers JavaScript
transport. The user explicitly approved this worker-only API, version-1 shared
flag, checkpoint/error order and inherited host-boundary limitations before
implementation. Do not request the same approval again. That approval does not
approve browser transport, new defaults/native policy, per-worker I/O adapters,
full-work accounting, hard termination or physical-memory isolation.
