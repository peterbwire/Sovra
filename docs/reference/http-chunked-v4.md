# Incremental chunked-body decoder

Implemented in the experimental Rust library and version-four HTTP socket host:
`HttpChunkedDecoder::new(HttpChunkedLimits)`, `feed(new_bytes)` and consuming
`finish()`. `http.receive` now admits a single case-insensitive
`Transfer-Encoding: chunked` field and returns the decoded body through the
existing Base64 request description. The public borrowed `parse_http_request`
API retains its fixed-length-only contract. G14 stays partial.

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

## HTTP host integration

The receiver incrementally finds the head terminator and validates the head once.
An internal checked decision admits either a fixed length or exactly `chunked`.
Transfer-Encoding with Content-Length, duplicate coding fields, repeated/non-final
chunked, coding lists/parameters and unsupported codings continue to refuse.
`Trailer` declarations and nonempty trailers still refuse. No partially decoded
body reaches Sovra, and all claimed failures drop the stream.

Head validation and chunk decoding share the existing HTTP `work` budget: only
remaining work passes to the decoder. Decoded bytes and size/extension lines use
HTTP `body_bytes` and `line_bytes`; encoded body bytes and nonzero chunks also
retain the decoder's 2 MiB/4096 hard caps. The existing accept-to-response deadline,
read/write operation count, cancellation and VM result-materialization limits
remain active. New feeds do not reset any counter. The decoder is bounded
cooperative work; no hard preemption or streaming-body API is claimed.

Wire headers remain original metadata, including Transfer-Encoding; the request
`body` is decoded content. These metadata fields are not a reconstructed set of
headers for forwarding the decoded body. Responses retain renderer-owned fixed
Content-Length and connection-close framing.

Independent Python `http.client` sends Unicode JSON through an iterable using
chunked coding, splitting a UTF-8 sequence across chunks. Both CLI and generated
native Sovra handlers parse the decoded JSON and return checked responses; native
startup is tested after deleting project files with an empty environment and
Node/PATH unavailable. Socket tests cover one-byte reads, ignored extensions,
malformed framing/codings, truncation, shared work/line/body limits, slow body
input, mid-body cancellation and I/O exhaustion. Production controls, trailers
and full HTTP protocol qualification remain open.

## Integrated serving validation

Latest chunked serving validation: 671 Rust tests pass (600 library and 71
HTTP/network/v3-host integration tests), including independent CLI/native
fixed-length and chunked Unicode JSON clients. Four Node security/language/
example/documentation checks, strict Clippy, exact Rust 1.74.0, Windows GNU
all-target compilation and formatting pass. Windows runtime and full HTTP
conformance remain unqualified. Local evidence:
`target/checked-evidence/g14-http-chunked-serving/verification.json`.

## Earlier isolated decoder checkpoint

Validation: 638 Rust tests pass (600 library and 38 chunk/request/response/socket
exchange tests), strict Clippy, exact Rust 1.74.0, Windows GNU all-target
compilation, formatting and documentation links. This slice does not rerun
CLI/native HTTP acceptance or establish Windows runtime support. Local evidence:
`target/checked-evidence/g14-http-chunked-decoder/verification.json`.
