# Incremental chunked-body decoder

Implemented as an experimental Rust library foundation:
`HttpChunkedDecoder::new(HttpChunkedLimits)`, `feed(new_bytes)` and consuming
`finish()`. It is not yet wired into `parse_http_request` or `http.receive`;
CLI/native HTTP serving continues to refuse Transfer-Encoding. G14 stays partial.

The decoder follows the chunk-size/data/CRLF/last-chunk grammar in
[RFC 9112 section 7.1](https://www.rfc-editor.org/rfc/rfc9112.html#section-7.1).
Each feed accepts only newly received bytes. No growing-input rescanning occurs.
Size lines are validated once, hexadecimal arithmetic is checked, chunk-data is
opaque binary, and decoded body storage grows geometrically. Complete framing
must include the zero-sized chunk and final empty trailer line. No body is
exposed before consuming `finish()` succeeds. Errors are sticky, including a
nonempty feed after completion; a failed decoder cannot yield a partial body.

Unknown syntactically valid extensions are ignored, as required by section 7.1.1.
The implementation validates token and quoted-string values, quoted-pair escapes,
BWS separators and original wire offsets. Bare LF, invalid controls, size
overflow, missing delimiters and trailing/pipelined bytes refuse. Nonempty
trailers explicitly refuse; they are never merged into request headers. This
restricted trailer profile does not establish full RFC conformance.

Trusted callers may only lower the following defaults:

| Limit | Default |
| --- | --- |
| Encoded wire bytes, including framing/extensions | 2 MiB |
| Decoded body bytes | 1 MiB |
| Size/extension line bytes, excluding CRLF | 8192 |
| Nonzero chunks | 4096 |
| Aggregate logical work | 8 Mi units |

Work persists across feeds: one unit per wire byte and four units per size-line
byte before size/extension validation. Fragmentation cannot reset it. Zero is a
valid denial limit; with zero body/chunks, an empty body can still be admitted
when wire/line/work limits allow. Allocator overhead and physical instructions
are not represented by logical byte/work limits. There are no sockets, timers,
callback controls, header framing decisions or decompression in this helper.

Nine tests exercise every split and truncated prefix, one-byte feeding, binary
preservation, extensions, malformed grammar, sticky failures, exact/lowered
limits, fragmentation-independent work, thousands of tiny chunks and deterministic
byte mutations. Existing request, response and socket exchange tests remain strict.

Next: integrate the decoder with a checked request-head framing decision and the
existing aggregate socket deadline/work limits, without repeated head parsing or
body copying. Test independent chunked clients through both CLI and native output
before advertising chunked request support. Transfer-Encoding/Content-Length
ambiguity, duplicate coding fields, non-final/repeated chunked and unsupported
codings must continue to refuse; full protocol and production gates remain open.

Validation: 638 Rust tests pass (600 library and 38 chunk/request/response/socket
exchange tests), strict Clippy, exact Rust 1.74.0, Windows GNU all-target
compilation, formatting and documentation links. This slice does not rerun
CLI/native HTTP acceptance or establish Windows runtime support. Local evidence:
`target/checked-evidence/g14-http-chunked-decoder/verification.json`.
