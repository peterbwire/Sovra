# Bounded validated-discard HTTP trailers

The experimental version-four HTTP host admits syntactically valid chunked
trailer sections under finite limits. It validates and discards trailer fields;
none are merged into request headers or exposed as application metadata. The
request body's Base64 value remains decoded content. This follows the discard
option in [RFC 9112 section 7.1.2](https://www.rfc-editor.org/rfc/rfc9112.html#section-7.1.2)
and the separation rules in [RFC 9110 section 6.5.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-6.5.1).
Digest/signature fields are discarded without verification. Applications cannot
use them as an integrity or authorization decision through this API.

## Admission and grammar

A chunked request may include one `Trailer` declaration containing comma-separated
field-name tokens with optional SP/HTAB around each name. Empty names, trailing
commas, invalid tokens, duplicate declaration fields and known prohibited names
refuse before body admission. Fixed-length requests with a Trailer declaration
remain unsupported. Declaration names need not all appear; undeclared trailer
fields may also be validated and discarded. No declaration grants authority.

Trailer lines require strict CRLF, a nonempty token name immediately followed by
colon, and values without controls other than HTAB. Opaque obs-text is allowed;
leading whitespace/folding, whitespace before colon, bare LF, invalid controls
and missing terminators refuse. Repeated permitted names are independently
validated and discarded. The final empty CRLF is mandatory; EOF before it is
truncation. Already-buffered trailing/pipelined bytes still refuse.

The conservative known prohibited-name list (case-insensitive) is:

- Framing/connection/routing: Content-Length, Transfer-Encoding, Host, Trailer,
  Connection, TE, Upgrade, Expect and Max-Forwards.
- Authentication/cookies: Authorization, Proxy-Authorization, Proxy-Authenticate,
  WWW-Authenticate, Authentication-Info, Proxy-Authentication-Info, Cookie and Set-Cookie.
- Content processing/request modifiers: Content-Type, Content-Encoding,
  Content-Range, Range, If-Match, If-None-Match, If-Range, If-Modified-Since and
  If-Unmodified-Since.

This is a conservative refusal list, not validation of every field definition's
permission for sender use in trailers. Unknown fields are discarded, never
interpreted as authority or forwarded. Full HTTP conformance remains unqualified.

## Limits and ownership

The Rust helper opts in through
`HttpChunkedDecoder::with_discarded_trailers(body_limits, HttpTrailerLimits)`.
Its original `new(body_limits)` retains empty-trailer-only admission. Consuming
`finish()` publishes only body bytes after successful complete validation.
Sticky failures revoke the entire result, including errors after completion.

Trailer limits default to 16 KiB total section bytes (including all CRLFs), 8192
bytes per line excluding CRLF and 64 field lines; trusted callers may only lower
them. The HTTP host inherits these from `http.head_bytes`, `http.line_bytes` and
`http.headers`. Head and trailer sections each have their own byte/field counters
with the same ceilings. Trailer declaration name count is also bounded by
`http.headers`. No additional policy keys or embedded grants are introduced.

Every trailer byte consumes the existing encoded-wire and shared HTTP work
budgets. Each completed field additionally charges 32 work units per line byte
before validation; declaration values similarly charge 32 units per byte.
Budgets persist across feeds and span head parsing, chunks and trailers. The
existing accept-to-response deadline, read/write operation count and callback
cancellation apply while waiting for the final trailer CRLF. Claimed refusal,
truncation, timeout or cancellation closes the socket without publishing a handler
request. Bounded automatic errors remain best-effort under remaining budget.

Only one size/field line is buffered; fields are discarded immediately after
validation. No trailer map or merged-header allocation is created. Logical
budgets are not exact allocator/CPU accounting or hard preemption.

## Evidence and remaining work

Six decoder tests cover every split/prefix, binary preservation, malformed and
prohibited fields, exact limits, sticky errors, fragmentation-independent work
and deterministic mutations. Four additional socket tests cover non-merging,
declaration refusal, inherited limits, mid-trailer cancellation and slow progress.
The independent Python HTTP client sends declared trailers to real Sovra JSON
handlers through CLI and generated native launches, alongside fixed-length and
ordinary chunked requests. Native startup removes project files and Node/PATH.

Trailer retention, integrity verification and application access remain separate
future contracts. Other expectation forms, upgrades, CONNECT, other transfer codings/versions,
Deployment authentication, TLS, concurrency/backpressure and
additional-host runtime qualification remain open under the [G14 contract](../design/G14_COMPLETION_CONTRACT.md).

Validation: 681 unique Rust tests pass (600 library and 81 HTTP/network/v3-host
integration tests), including independent CLI/native declared-trailer clients
and 18 socket exchange tests. Four Node security/language/catalog/documentation
checks, strict Clippy, exact Rust 1.74.0, Windows GNU all-target compilation and
formatting pass. Windows runtime/full HTTP/production qualification remain open.
Local evidence: `target/checked-evidence/g14-http-trailers/verification.json`.

Optional [Host/Origin admission](http-access-v4.md) now runs on the original
head before body/trailer processing; discarded trailer fields cannot alter it.
