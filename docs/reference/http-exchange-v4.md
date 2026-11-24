# One-request HTTP exchanges through the network host

Version-four hosts now connect the bounded HTTP parser and renderer to real
owned sockets and checked Sovra data-profile code. Use the existing explicit
network policy and `std::host_call`:

| Operation | Required fields beyond `version:4` and `op` | Result |
| --- | --- | --- |
| `http.receive` | `handle` of a raw accepted stream | JSON request object |
| `http.respond` | `handle`, `status` Int, `headers` String-pair array, `body` Base64 String | Null after complete write and close |

A received request has `method` and raw `target` Strings, source-order `headers`
objects with `name` and Base64 `value`, and a Base64 `body`. Opaque wire field/body
bytes are not implicitly interpreted as Unicode. Fixed-length and restricted
chunked requests are admitted; chunked bodies are decoded before Base64 output,
while headers preserve original wire metadata. Application code decides how
to decode and validate payloads; the JSON echo example uses strict UTF-8/JSON.

## Ownership state and refusal

An accepted stream is initially raw. `http.receive` consumes that raw ownership;
it publishes one HTTP response owner only after a complete admitted request.
Once claimed for HTTP, raw read/write and repeated receive refuse. A response
uses the original token but only the HTTP state accepts it; the request's checked
method determines HEAD behavior. Source response data cannot override that context.
A response consumes the owner, attempts all writes and closes the connection.
There is no second response, persistence, implicit pipelining or worker thread.

Malformed/truncated input is not published to a handler. Within remaining budget,
the host attempts an empty fixed error response (400 for malformed framing,
413 for limits, 501 for unsupported profile contexts), then closes. Peer resets,
expired budgets or limits too small for that error packet can prevent delivery;
no drain or unbounded error-reply loop runs. Invalid response data likewise
revokes and closes its owner without sending partial renderer output. Partial
socket writes already completed cannot be rolled back after a later I/O failure.

Cancellation before request processing leaves an existing owner untouched.
After receive claims a stream, refusal/cancellation drops it. Execution cleanup
also closes unanswered HTTP owners. Ordinary Result-based handler failures can
be mapped by Sovra to a valid 500 response; VM faults/cancellation still terminate
execution and close owned resources rather than invoking a handler fallback.

## Aggregate policy

The trusted policy may add an optional `http` object to a version-four document:

```json
{
  "version": 4,
  "listen_ports": [47123],
  "limits": {"listeners":1,"streams":1,"bytes":4096,"milliseconds":5000},
  "http": {"milliseconds":5000,"io_operations":512,"head_bytes":4096,"body_bytes":65536,"response_bytes":69632}
}
```

The eight supported HTTP keys are `milliseconds`, `io_operations`, `head_bytes`,
`line_bytes`, `headers`, `body_bytes`, `response_bytes` and `work`. All are
nonnegative Ints and may only narrow defaults. Head/line/count/body/work settings
apply to both request parsing and response rendering. `response_bytes` bounds
serialized response output; response header counts include generated fields.
Defaults are the existing parser/renderer ceilings, 30 seconds and 4096 socket
read/write API operations. Nonblocking syscall retries remain within each API
operation and check cancellation and the same aggregate deadline.

The deadline starts at successful accept publication and spans receive, Sovra
handler time and send. It is capped by both network and HTTP milliseconds;
handlers cannot restart it. One operation counter spans receive and response.
HTTP policy can only be set before socket creation; issued owners retain limits.
Raw socket operations keep their existing policy semantics: HTTP limits are not
a protocol sandbox for a program explicitly granted raw network authority.

The receiver scans newly arrived head bytes incrementally, parses the head once,
then receives the admitted fixed-length body or feeds the bounded incremental
chunked decoder. It does not reparse the head or growing body prefixes. Chunked
decoding consumes the head parser's remaining HTTP work budget; see the
[chunked contract](http-chunked-v4.md). Buffers, request/result descriptions, codecs and response
serialization have finite ceilings; VM node/text/work admission still applies
before a result reaches Sovra. Materialization can refuse below transport limits.
No counter is a claim of exact allocator/OS backlog accounting, hard preemption
or process isolation.

## Actual application evidence

[The Sovra example](../../examples/http-loopback/README.md) runs seven sequential
connections with GET/HEAD health, fixed-length/chunked/trailer Unicode POST JSON echo, a 404 route and a handler Result
error recovered as 500. Python's independent `http.client` validates status,
Content-Type, exact Content-Length/body, HEAD and connection-close behavior.
Both CLI and native launch pass; native execution is tested after source deletion,
environment clearing and unavailable Node/PATH. Catalog checks do not run the
server or open ports.

This remains a restricted loopback HTTP profile, not full RFC 9112 conformance
or production deployment. Transfer codings beyond a single chunked coding,
Expect, upgrades, CONNECT,
absolute targets and other HTTP versions still refuse. General method/routing,
Host/origin authorization, concurrency/backpressure, TLS and additional-platform
qualification remain open under the [G14 contract](../design/G14_COMPLETION_CONTRACT.md).

## Earlier fixed-length verification checkpoint

The Linux checkpoint passes 772 unique Rust tests, including nine exchange tests
and two independent HTTP client tests covering CLI and native execution, plus
four Node security/language/catalog/documentation checks. Clippy with warnings
denied, exact Rust 1.74.0, formatting and Windows GNU all-target compilation pass.
Windows compilation is not runtime acceptance. Retained local evidence is under
`target/checked-evidence/g14-http-exchange/verification.json`; this is targeted
G14 coverage, not a rerun of the full G13 qualification or full HTTP conformance.

Latest chunked serving validation: 671 Rust tests pass (600 library and 71
HTTP/network/v3-host integration tests), including independent CLI/native
fixed-length and chunked Unicode JSON clients. Four Node security/language/
example/documentation checks, strict Clippy, exact Rust 1.74.0, Windows GNU
all-target compilation and formatting pass. Windows runtime and full HTTP
conformance remain unqualified. Local evidence:
`target/checked-evidence/g14-http-chunked-serving/verification.json`.

Chunked trailer sections now follow the [bounded validated-discard policy](http-trailers-v4.md).
Trailer fields never enter the handler header map, and integrity values are not
verified or exposed. Shared work/deadline/I/O limits include trailer processing.
