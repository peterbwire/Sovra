# Bounded HTTP expectation handling

The experimental version-four socket host supports one case-insensitive
`Expect: 100-continue` field on admitted HTTP/1.1 requests. This follows the
handshake requirements in [RFC 9110 section 10.1.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-10.1.1).
The public borrowed fixed-length parser retains its original Expect refusal;
this behavior belongs to the owned HTTP host.

## Order and framing

The host validates the complete head, Host syntax, body framing, size admission
and trailer declarations before any interim response. It then:

- Sends exactly `HTTP/1.1 100 Continue\r\n\r\n` before waiting for an incomplete
  fixed-length or chunked body.
- Omits the interim response when the entire body is already buffered or a
  fixed-length frame indicates no body.
- Returns an empty final `417 Expectation Failed` and closes for other Expect
  values, including lists/parameters outside this restricted profile. It does
  not wait for the declared body.
- Refuses duplicate Expect fields as ambiguous framing, with a bounded final
  400 where budget permits. Invalid/oversized framing similarly refuses before
  an interim response; malformed requests never reach handlers.

Headers and body bytes already buffered can establish an earlier refusal. An
interim response does not authorize an application action or establish Host/
origin authentication. Application handlers receive only a completely admitted
body. Unknown expectations never invoke a handler or consume a response token.

## Limits and cleanup

The constant interim packet is validated against response output/head/line/work
ceilings before writing. It has 25 wire bytes, a 21-byte status line and no fields
or body; preflight requires at least 25 logical response work units. Final response
rendering retains its own work/output ceilings. The same cumulative read/write
operation count and accept-to-response deadline span head reads, interim partial
writes, body/chunk/trailer reads, handler time and final output. No budget or
clock is restarted by the handshake. Callback cancellation still propagates
separately from host errors.

If remaining limits prevent an interim or final error packet, the socket closes.
Partial writes cannot be undone. After a 100 response, successful request
processing produces the ordinary final response; malformed input, disconnect,
resource refusal or cancellation can instead close the connection prematurely.
There is no background body wait or second request on the same connection.

## Application evidence

The Sovra example handles thirteen sequential connections and recovers ordinary
receive errors per connection, preserving host cleanup and subsequent service.
An independent Python HTTP client sends only headers, verifies the 100 packet,
then sends JSON and validates the final response. A separate unsupported
expectation sends no body and receives 417; subsequent route/handler cases still
pass. CLI and generated native execution use the same host; native startup
removes project files and Node/PATH.

Six additional socket tests cover fixed/chunked handshakes across one-byte I/O,
buffered/empty body omission, early framing/expectation refusal, interim output
limits, shared I/O/cancellation and aggregate timeout. Existing chunk/trailer/
response/network/v3-host regressions remain required.

Full HTTP conformance, other interim responses, authentication, TLS, concurrency/backpressure and additional-host runtime
qualification remain open under the [G14 contract](../design/G14_COMPLETION_CONTRACT.md).

Validation: 687 unique Rust tests pass (600 library and 87 HTTP/network/v3-host
integration tests), including 24 socket exchange tests and independent CLI/native
100-continue/early-417 clients. Four Node security/language/catalog/documentation
checks, strict Clippy, exact Rust 1.74.0, Windows GNU all-target compilation and
formatting pass. This is targeted Linux G14 evidence; Windows runtime, full HTTP
conformance and production qualification remain open. Local evidence:
`target/checked-evidence/g14-http-expect/verification.json`.

Configured [Host/Origin admission](http-access-v4.md) precedes any interim packet.
Unlisted requests receive a final bounded 403 before the host awaits a body.

Optional exact method/target access lists are checked before Continue, alongside
the configured Host/Origin rules. Unlisted requests receive bounded final 403.

## Cooperative interim output (2026-11-25)

After a checked/authenticated head needs Continue, pending-body state now retains
an offset into the fixed interim packet. `http.poll_body` attempts at most one
nonblocking write of its remaining suffix per turn, using the original deadline,
byte quantum and shared I/O admission. Not-ready writes preserve the suffix;
partial writes advance exactly by the socket count. Even the final interim write
returns false without reading a body fragment in the same turn. The following
poll can resume body input. Co-read complete bodies still suppress Continue.

Interim framing/work/output limits are validated before queuing. Callback refusal,
timeout or I/O failure drops custody; already transmitted prefixes are not rolled
back and no second packet is appended to a partial Continue. Blocking receive can
finish the same suffix before waiting for body input, without re-verifying or
resending any prefix. Tests verify each byte with a one-byte socket quantum and
switch from polling to blocking receive after the first transmitted byte.

This removes waiting interim writes from cooperative body turns. Initial trusted
verification and denial/error output can still wait. Fairness remains bounded to
the documented example and does not imply preemption, production scheduler
acceptance or secure deployment.
