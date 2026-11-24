# Bounded fixed-length HTTP request parsing

The Rust embedding API `parse_http_request(input, limits, eof)` now returns a
borrowed complete request, bounded incomplete state, or a fixed error with an
original zero-based wire offset. This is a G14 parser foundation. It does not
open sockets, send responses, dispatch Sovra handlers or authorize networking.
Existing CLI/native byte serving remains separate.

## Initial profile

The parser accepts one HTTP/1.1 request with strict single-SP request-line
separators and CRLF lines. Targets are ASCII origin-form paths with syntactically
valid percent escapes, or `OPTIONS *`. Methods retain their original case and
are not limited to an implemented handler set. There is no percent decoding,
path normalization, filesystem lookup or URL reconstruction.

Exactly one valid Host is required. This profile accepts ASCII DNS-style names
(including punycode and one trailing dot), IPv4-style names or bracketed IPv6,
with an optional decimal port in 0–65535. Empty hosts, userinfo, whitespace,
commas, paths, malformed brackets and invalid ports refuse. This is syntax
validation, not application Host authorization, origin checks or DNS resolution.

Field names must be ASCII tokens without whitespace before the colon. Leading
field whitespace/obsolete folding, bare LF and control values refuse. OWS is
trimmed; permitted obs-text remains opaque bytes rather than implicit UTF-8.
Source order and non-framing duplicate field lines are preserved for later
field-specific interpretation.

A single decimal Content-Length defines the exact binary body. Duplicate
Content-Length fields (even identical), comma lists, signs, overflow and
Transfer-Encoding plus Content-Length refuse. Without a declared length the
body is empty. EOF during a required frame is truncated, not a shorter success.
Already-buffered trailing/pipelined bytes refuse. The socket driver closes after
one request and does not dispatch incomplete/refused input.

Transfer-Encoding, Expect, Upgrade and Trailer fields currently refuse explicitly.
So do CONNECT, absolute-form targets and other HTTP versions. Chunked transfer
coding is required for a full RFC 9112 recipient; it is not implemented here.
This deliberately restricted profile is not a complete conformant HTTP/1.1
server or production HTTP implementation.

## Bounds and ownership

Hard ceilings are 16 KiB aggregate head, 8 KiB per line, 64 field lines, 1 MiB
fixed-length body and two million logical work units per parser invocation.
Trusted callers can only narrow them. Parsing never copies the body/target or
converts binary input to Unicode; the only result container is the bounded field
vector. Error messages do not echo untrusted request contents.

Incomplete input exposes no partial request. After the head completes,
`minimum_bytes` identifies the exact total frame size. The API is stateless:
repeated prefix parses have independent work budgets. Socket integration must
use incremental head-delimiter scanning, then wait for the admitted body size,
and impose aggregate bytes, attempts and request/connection deadlines. Per-call
limits do not establish a Slowloris defense, hard time limit or process isolation.

Tests cover every partial boundary, opaque binary bodies/field values, exact
limits, length ambiguities, whitespace/folding/controls, malformed targets and
Host fields, unsupported contexts, trailing data and deterministic byte mutations.
[Bounded response construction](http-response-framing-v4.md) is now implemented.
The next stages are socket framing with aggregate policy and actual Sovra
handlers with independent HTTP/JSON clients.

## Primary standards reviewed

Implementation review used these original standards on 2026-11-24:

- [RFC 9112 sections 2–3](https://www.rfc-editor.org/rfc/rfc9112.html#section-2): octet parsing and request lines/targets.
- [RFC 9112 section 5](https://www.rfc-editor.org/rfc/rfc9112.html#section-5): colon whitespace, OWS and obsolete folding.
- [RFC 9112 section 6.3](https://www.rfc-editor.org/rfc/rfc9112.html#section-6.3): message body length and ambiguous framing.
- [RFC 9112 section 7.1](https://www.rfc-editor.org/rfc/rfc9112.html#section-7.1): chunked-recipient requirement, retained as an explicit gap.
- [RFC 9110 sections 5.5–5.6](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.5): opaque field values and tokens.
- [RFC 9110 section 7.2](https://www.rfc-editor.org/rfc/rfc9110.html#section-7.2): Host authority.

Targeted qualification passes 635 unique Rust tests: nine HTTP parser tests,
26 socket/network-host/policy/v3 regressions and all 600 library tests. Clippy,
formatting, exact Rust 1.74.0 and Windows GNU all-target compilation pass. This
is not Windows runtime or HTTP server acceptance. Logs, the targeted report
and primary-standard hashes are retained under
`target/checked-evidence/g14-http-request-framing/`.

[Owned HTTP exchange integration](http-exchange-v4.md) now connects this helper to sockets and real Sovra CLI/native handlers under a bounded one-request policy. Earlier helper-only checkpoint evidence above remains scoped to its original run.

The fixed-length public borrowed API above retains its original refusal contract.
The HTTP host now uses the same internal checked head validation with a separate
[bounded incremental chunked decoder](http-chunked-v4.md); `http.receive` supports
restricted chunked requests through CLI/native Sovra handlers. This does not
establish full protocol conformance or admit nonempty trailers.

The public borrowed parser continues refusing Expect. The owned HTTP host uses
the checked head decision for [bounded 100-continue handling](http-expect-v4.md),
including early 417 for unsupported values before waiting for a request body.
