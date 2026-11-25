# Aggregate fixed-window HTTP attempt admission

Version-four policies may opt into a per-execution-domain HTTP attempt budget:

```json
{
  "version": 4,
  "listen_ports": [47123],
  "http_admission": {"requests": 100, "milliseconds": 1000}
}
```

Both fields are required, with no unknown/duplicate keys. Requests is a nonnegative
Int at most 1024; zero denies all HTTP attempts. Window duration is an Int in
1..60000 milliseconds; zero duration refuses instead of resetting on every call.
Rust embedding callers use `HttpAdmissionPolicy` and
`NetworkHostV4::set_http_admission` before creating socket owners.
Omitting this object preserves existing behavior.

## Counting and window rules

The counter belongs to one NetworkHostV4 execution domain and spans all its
listeners/streams. Each `http.receive` claim of a valid raw accepted stream
attempts admission before reading/parsing a head, checking access/authentication,
spawning a verifier, waiting for a body or sending Continue. One admitted claim
consumes one unit even if framing, credentials, permissions, I/O or later control
checks fail. Failure does not refund admission. Stale/foreign/wrong-kind handles
and cancellation at the initial preclaim checkpoint do not charge the counter.

A monotonic fixed window starts at configuration. At or after its expiration,
the next attempt starts a new window with a fresh count. Idle time does not
accumulate credits, denied requests do not overflow the count, and no peer date
or forwarding header can reset it. Requests are counted by admission start, not
completion. Fixed-window boundaries may allow bursts approaching twice the
configured count across adjacent windows; this is not a rolling/sliding quota.

Closing/recreating listeners or streams does not reset the domain counter.
Trusted Rust code may reconfigure only with no live owners; source-visible JSON
requests cannot change policy. A new host/execution domain has an independent
budget. This is not a global machine, per-IP, per-credential or distributed limit.
An operator can intentionally create additional domains.

## Refusal and custody

Exhausted admission returns the stable `admission` host error, attempts an empty
429 Too Many Requests with `Cache-Control: no-store`, then drops the raw stream.
This follows the status/cache semantics in
[RFC 6585 section 4](https://www.rfc-editor.org/rfc/rfc6585.html#section-4).
There is no principal/handler publication or verifier invocation. Empty output
is safe before HEAD method proof; no Retry-After promise is made, including for
zero-quota denial.

Error packets are best-effort under the same response byte/field/work, I/O and
accept-to-response deadline bounds. Exhausted output/control limits may close
without a packet. Peer resets or unread TCP data can also prevent delivery;
there is no unbounded drain or wait for a head/body to enable rejection. Callback
cancellation while rejecting still propagates separately and releases the owner.

This budget governs HTTP host calls, not TCP accepts/backlog memory or explicitly
granted raw socket protocols. It does not provide authentication, fair scheduling,
worker limits, OS process isolation or protection against every denial of service.
Those production boundaries remain separate.

## Evidence and remaining work

Three deterministic clock tests cover exact boundary reset, no idle credits,
zero-denial, clock-regression refusal and configuration caps. Four socket/policy
cases cover exact optional JSON, failed/successful attempts across listeners,
listener replacement, preclaim/claimed cancellation, pre-verifier refusal,
output-limit closure and non-cacheable 429.

Authenticated CLI/native consumers now make five connections: valid identity,
wrong-token 401, permission 403, subsequent success and final quota 429. The
backend invocation file must contain exactly four entries, proving the rejected
connection does not spawn verification. Native startup still deletes project
files, clears Node/PATH and checks that credentials are not embedded.

Production per-identity quotas/backpressure, credential lifecycle, TLS, full HTTP
conformance and additional-platform runtime evidence remain open under the
[G14 contract](../design/G14_COMPLETION_CONTRACT.md).
