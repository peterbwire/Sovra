# Execution-scoped network host requests

The additive Rust execution APIs `execute_generic_data_ir_with_network_host` and
`execute_generic_data_package_with_network_host` take an owned `NetworkHostV4`.
A trusted embedder creates that scope with lowerable `NetworkLimits` and grants
exact ports explicitly. Checked data-profile `.svr` code then uses the existing
`std::host_call` builtin with these version-four requests:

| Operation | Required fields beyond `version:4` and `op` | Successful JSON result |
| --- | --- | --- |
| `net.listen` | `port` (Int, 0–65535) | Positive listener token Int |
| `net.port` | `handle` (listener token) | Actual bound port Int |
| `net.accept` | `handle` (listener token) | Positive accepted-stream token Int |
| `net.try_accept` | `handle` (listener token) | Positive accepted-stream token Int, or Null when not ready |
| `net.read` | `handle` (stream token), `maximum` (positive Int) | Strict Base64 String for a nonempty partial chunk; Null for EOF |
| `net.write` | `handle` (stream token), `data` (strict Base64 String) | Actual partial byte count Int |
| `net.close` | `handle` (listener or stream token) | Null |

Only exact required fields are accepted. Duplicate, missing, unknown or wrongly
typed fields refuse before socket effects. Unknown operations and unsupported
hosts refuse. Base64 input is capped before decoding; malformed or oversized
input sends no prefix. Reads and writes preserve binary bytes, not UTF-8 text.
Callers must handle partial progress and EOF under their own aggregate policy.

## Ownership and authority

Tokens are identifiers, not secret credentials or serialized socket owners.
They alias a single private Rust owner in this execution scope. Copying a JSON
Int does not duplicate a descriptor or extend its lifetime. Live scopes use
process-unique positive identifiers with finite admission and no token reuse;
foreign/stale tokens and wrong resource kinds refuse. Closing removes the owner
before reporting shutdown errors, invalidating every alias. A listener's close
does not close independently owned accepted streams.

Execution consumes its network scope and closes every remaining owner on success,
VM failure or control refusal. A shutdown failure during final cleanup is an
explicit fatal host-cleanup error; all other owners are still closed. Ordinary
Rust scope destruction also drops descriptors. Effects already performed are not
rolled back, including bytes consumed before a later VM materialization refusal.
Returned JSON tokens cannot carry a live socket beyond execution completion.

The network host is a separate explicit adapter from version-three file/process
hosts. Existing data execution and v3 entry points remain unchanged and install
no networking. Requests cannot add grants. Check/build/test commands and native
images gain no authority through this Rust API. [External policy dispatch](network-policy-v4.md)
now enables explicit CLI/native launch without embedding grants. [HTTP exchange operations](http-exchange-v4.md) now add one-request framing and
handler integration. Outbound connect, DNS, remote bind, async scheduling and TLS
remain outside this slice.

## Evidence and continuation

Tests run actual `.svr` source which accepts and echoes a binary byte to an
independently implemented TCP client through the checked data VM. Other source
tests bind only with explicit grants and verify owner cleanup after successful
execution, runtime failure and instruction refusal. Direct adapter tests cover
schema refusal, duplicate fields, strict Base64, wrong-kind/foreign/stale tokens,
cancelled accept, EOF and explicit cleanup. Existing data and v3 host checks
remain regression coverage.

The embedding-driven byte foundation now also has independent CLI/native
acceptance through [external policies](network-policy-v4.md). HTTP is not
implemented. Next is bounded sequential HTTP and independent API acceptance under
the [G14 contract](../design/G14_COMPLETION_CONTRACT.md).

Targeted Linux qualification passes 702 unique Rust tests: six network-host,
seven socket-owner, ten v3 host, 79 data-profile tests and all 600 library tests.
Clippy, formatting, exact Rust 1.74.0 and Windows GNU all-target compilation pass.
Windows compilation is not network runtime acceptance. Logs and the targeted
report are retained in `target/checked-evidence/g14-network-host-execution/`.

## Cooperative accept foundation (2026-11-25)

`net.try_accept` performs at most one nonblocking accept syscall, without sleeping
or retrying. WouldBlock or Interrupted returns Null: no stream owner or token is
published. A successful attempt returns a fresh owned stream token with the same
HTTP accept-time deadline origin as `net.accept`. Existing blocking operations
remain unchanged.

Stream capacity is reserved before accepting. At capacity, the operation returns
`limit` without dequeuing a peer; after an owner closes, a later attempt can
accept the queued connection. This bounds application-owned streams, not the OS
TCP backlog. A zero operation deadline returns `timeout` before accepting.
Cancellation is checked before the syscall and before publication; cancelled
publication drops the new descriptor and reservation. Consumed TCP connections
cannot be put back into the backlog. Empty attempts allocate no handle IDs.

Checked Sovra source tests distinguish idle Null from an owned connection and
verify execution-scope cleanup. Socket tests cover repeated empty attempts,
capacity preservation/recovery, wrong-kind refusal, zero deadline/capacity and
cancellation after accepting but before publication.

This is the first cooperative scheduling primitive, not concurrent HTTP serving.
Callers must bound polling with VM fuel or their own scheduler; busy loops remain
possible. Read/write and HTTP receive/respond still wait under their existing
limits. Incremental connection state, readiness/fair scheduling, bounded pending
queues, slow-client isolation, streaming and shutdown coordination remain open.
Linux remains the only qualified runtime host.
