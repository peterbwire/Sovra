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
413 for limits, 403 for configured Host/Origin denial, 417 for unsupported expectations, 501 for other unsupported contexts), then closes. Peer resets,
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

[The Sovra example](../../examples/http-loopback/README.md) runs thirteen sequential
connections with GET/HEAD health, fixed-length/chunked/trailer Unicode POST JSON echo, a 404 route and a handler Result
error recovered as 500. Python's independent `http.client` validates status,
Content-Type, exact Content-Length/body, HEAD and connection-close behavior.
Both CLI and native launch pass; native execution is tested after source deletion,
environment clearing and unavailable Node/PATH. Catalog checks do not run the
server or open ports.

This remains a restricted loopback HTTP profile, not full RFC 9112 conformance
or production deployment. Transfer codings beyond a single chunked coding,
upgrades, CONNECT,
absolute targets and other HTTP versions still refuse. General method/routing,
Authentication, concurrency/backpressure, TLS and additional-platform
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

The host now supports [bounded 100-continue](http-expect-v4.md), with early 417
for unsupported expectations. Interim writes share the accept-to-response
deadline and I/O count; the public source response operation remains final-only.

An optional trusted [Host/Origin access policy](http-access-v4.md) now runs after
head/framing validation and before any Continue/body processing, sharing request
work. The example enables it; absent policy preserves existing HTTP behavior.

Configured [method/target grants](http-access-v4.md#optional-exact-method-and-target-grants)
now narrow HTTP access before Continue/body handling, with exact raw-target
matching and shared work. They do not implement application authentication.

Trusted Rust embedding callers may now install the [Bearer-verifier boundary](http-authentication-v4.md).
It runs after access policy and before body/Continue, publishes a validated public
principal and removes Authorization from the returned header array on success.
CLI/native launches retain their existing no-provider behavior.

Optional [fixed-window HTTP admission](http-admission-v4.md) now runs before
head/parser/verifier work. All listeners share one domain count; failures consume
admitted attempts and quota refusal closes with best-effort non-cacheable 429.

## Resumable head-buffer foundation (2026-11-25)

`HttpHeadBuffer` now owns bounded accumulated bytes, the delimiter scan cursor,
completion and sticky failure state. Each feed resumes scanning from the prior
cursor, including delimiters split across fragments. It does not reparse earlier
fragments. Co-read body bytes remain intact; consuming the buffer moves its bytes
without copying. An over-limit fragment is refused before any prefix is appended.
A completed buffer refuses further feeds, including empty ones. Zero head capacity
admits no bytes. Construction validates the existing lowerable request limits.

The existing HTTP receive path now uses this accumulator while keeping its
accept-to-response deadline, socket I/O counts and strict parser/access/verifier
ordering. Delimiter completion is not syntax validation or authorization: the
complete head still passes the existing parser exactly once, and no handler sees
an incomplete/unchecked request. Parser work remains separately bounded; the
accumulator only bounds byte storage and incremental delimiter scanning.

Tests cover every two-fragment split and one-byte fragmentation, body-prefix
preservation, exact/zero limits and sticky refusal without partial append.
This extracts resumable head state; the socket receive loop still waits. Body,
authentication, Continue and response state machines and fair scheduling remain
open before concurrent HTTP service can be claimed.

## Yielding head preparation (2026-11-25)

Strict v4 `{"version":4,"op":"http.poll_head","handle":N}` claims a raw stream
into unique pending-head custody. Subsequent polls resume the same head buffer.
A successful poll attempts at most one nonblocking read and returns Bool false
while more input is needed, or true when a delimiter, EOF or head-byte ceiling
requires final parsing. True is not validation, authentication or a handler
request. No raw read/write, respond or second independent claim can bypass the
pending state. `net.close` and execution cleanup release it.

`http.receive` accepts raw or pending-head custody, preserving accumulated bytes,
accept time and I/O counts. It checks the original aggregate deadline again,
then completes existing strict parsing, access/authentication, Continue and body
handling before publishing a handler request. It can still wait for a body or
verifier; this is not a fully cooperative HTTP receive API.

Admission is charged once on the initial claim, not on subsequent polls or the
transition to receive. Every actual polling read attempt, including idle attempts,
consumes the shared I/O budget. Already-ready polls perform no new read, but still
check control/deadline. Claimed cancellation, timeout, I/O exhaustion or errors
drop pending custody. Initial host-call cancellation leaves the unclaimed owner
intact. Quota refusal retains the existing bounded best-effort 429 write path,
which may wait under its deadline; normal head preparation never sleeps/retries.

Tests cover fragmented input, idle yields while another connection reaches its
handler, readiness without validity, EOF refusal, raw-I/O exclusion, cancellation,
shared I/O exhaustion, admission-once transitions, prepared-head deadline expiry
and actual checked Sovra polling/cleanup. Retained head buffers expose only lengths
and state in debug output, never raw headers or credentials. Fair scheduling, resumable
body/verifier/output phases and production concurrency remain open.

## Resumable fixed-length body state (2026-11-25)

`HttpFixedBody` owns the declared-length buffer independently of socket progress.
Construction validates existing request limits and admits length before allocation.
Each feed retains partial binary data, returns completion and exposes remaining
bytes. Excess input refuses before any prefix append with a sticky framing error.
Finishing publishes only a complete body; incomplete finish returns truncated,
and any prior framing failure remains authoritative. Error offsets are relative
to the accumulated body. Debug output exposes counts/state, not payload bytes.
Zero-length bodies complete without input; empty fragments are permitted.

The actual fixed-length receive path now uses this state after existing strict
head parsing/access/authentication. Continue, bounded reads, aggregate deadlines,
I/O counts and handler publication ordering remain in place. Reads are bounded by
remaining body length, and completion moves the payload without another copy.
Every two-part split and bytewise input, arbitrary binary bytes, exact/zero caps,
truncation, sticky excess refusal and opaque debug have focused regression tests.

This retains body state for future yielding receive phases; it does not yet expose
body polling or make the current body loop nonblocking. Chunked/trailer decoding
already retains incremental decoder state but still waits in the socket loop.
Persisting checked request metadata and authentication outcome, Continue/output
state and fair scheduling remain open.

## Checked metadata and body polling (2026-11-25)

`http.poll_body` requires a ready pending head (created with `http.poll_head`) or
an existing pending body. Wrong-kind/unprepared owners refuse before consuming
custody. On the first call it runs existing strict parsing, exact access checks
and optional verification, owns the checked method/target/header/principal
metadata, initializes fixed-length or chunked/trailer state and handles bounded
Continue if needed. This preparation can wait for the trusted verifier or interim
output under existing controls; it is not an asynchronous verifier/output API.

After preparation each poll performs at most one nonblocking body read and returns
only Bool readiness. Partial bodies, metadata and principal remain private.
Ready bodies perform no further read. Idle attempts count against the same I/O
budget; all phases retain the original accepted timestamp and aggregate deadline.
Head admission is never recharged by body polling. Raw I/O, head polling and
respond cannot bypass pending-body custody. Claimed cancellation, timeout,
truncation or decoding/I/O failure drops the pending owner; body-poll failures may
close without an HTTP error packet. Explicit close and scope cleanup release it.

`http.receive` accepts pending bodies and finishes the existing handler request
without parsing or verifying again. It can still wait for incomplete bodies.
Body encoding and final publication recheck the aggregate deadline. Only complete
bodies publish the retained metadata and principal. Buffer and pending-body debug
output omits credentials, payload and identity metadata. Authentication decisions
are scoped to this bounded exchange, not a continuous revocation service.

Tests cover fragmented fixed/chunked bodies, wrong-kind/raw-I/O exclusion,
truncation, pending-body cancellation/deadline, actual checked Sovra polling and
scope cleanup, and repeated idle polls with exactly one verifier invocation.
Independent CLI/native authenticated and default server regressions remain part
of validation. Fair scheduling, asynchronous verification/Continue/final output,
production concurrency and secure deployment remain open.

## Cooperative final response output (2026-11-25)

`http.start_response` accepts the same status/headers/Base64 body fields as
`http.respond`, validates/renders through the same bounded framing code and
returns Null without a socket write. It consumes the parsed request owner into
private pending-response custody with immutable wire bytes and an unsent offset.
It checks the shared accepted-time deadline before and after rendering.

`http.poll_response` takes only version/op/handle, attempts at most one nonblocking
write and returns Bool false while output remains. Partial writes advance exactly
by the reported count; not-ready attempts retain the same suffix. Every write
attempt, including not-ready, consumes the existing shared I/O budget and byte
quantum. Polling never resets the accepted-time deadline. True means all bytes
were accepted by the socket and close succeeded, not proof of peer receipt. It
consumes custody and makes every token alias stale. A close error still drops the
owner and is reported rather than publishing True.

Raw I/O, receive and another response cannot bypass pending-output custody.
Claimed cancellation, deadline, I/O/error refusal or explicit close drops queued
output and the stream. Already transmitted prefixes are not rolled back, and no
second error response is appended to a partial response. Initial host-call
cancellation leaves unclaimed custody intact. Pending debug omits wire content.
Each queued response remains under the existing output cap and stream slot;
there is no unbounded background output queue. This is not an OS memory bound
or a fairness/global-machine quota.

Blocking `http.respond` remains supported through the same preparation/renderer.
Tests verify byte-exact output through one-byte writes, retained HEAD context,
stale/wrong-kind refusal, invalid framing without output, cancellation, shared
I/O exhaustion, deadlines and opaque debug. The real Sovra HTTP example now
starts and polls response output; independent default/authenticated CLI/native
clients verify its behavior. Its demonstration loop remains sequential and is
bounded by VM/control/I/O limits; this does not add a fair scheduler. Initial
verification/Continue and denial output may still wait; production concurrency,
streaming and deployment remain open.
