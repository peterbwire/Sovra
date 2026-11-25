//! Bounded borrowed parsing for the initial fixed-length HTTP/1.1 request profile.
//! No socket operations, dispatch, decompression or URI normalization occur here.
/// Independently lowerable parser ceilings; work is charged per invocation.
#[derive(Debug, Clone, Copy)]
pub struct HttpRequestLimits {
    /// Request line, fields and terminating CRLF, in aggregate wire bytes.
    pub head_bytes: usize,
    /// Bytes per request/field line, excluding CRLF.
    pub line_bytes: usize,
    /// Number of field lines.
    pub headers: usize,
    /// Fixed-length wire body bytes.
    pub body_bytes: usize,
    /// Logical scanning/validation admission; not physical CPU instruction counts.
    pub work: usize,
}
impl HttpRequestLimits {
    /// Initial finite parser profile, not a complete connection/server policy.
    pub const DEFAULT: Self = Self {
        head_bytes: 16384,
        line_bytes: 8192,
        headers: 64,
        body_bytes: 1024 * 1024,
        work: 2 * 1024 * 1024,
    };
    pub(super) fn validate(self) -> Result<(), HttpRequestError> {
        if self.head_bytes > Self::DEFAULT.head_bytes
            || self.line_bytes > Self::DEFAULT.line_bytes
            || self.headers > Self::DEFAULT.headers
            || self.body_bytes > Self::DEFAULT.body_bytes
            || self.work > Self::DEFAULT.work
        {
            Err(fail(
                "policy",
                0,
                "HTTP parser limits may only narrow defaults",
            ))
        } else {
            Ok(())
        }
    }
}
/// Stable refusal category and original zero-based wire offset; no peer text is echoed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestError {
    /// input, policy, limit, syntax, framing, expectation, unsupported or truncated.
    pub code: &'static str,
    /// Original input byte offset.
    pub offset: usize,
    /// Fixed explanation, independent of untrusted field/target contents.
    pub message: &'static str,
}
/// Bounded resumable delimiter accumulation; completion does not validate HTTP syntax.
/// Bytes co-read after the delimiter are retained for subsequent body framing.
#[derive(Debug)]
pub struct HttpHeadBuffer {
    bytes: Vec<u8>,
    maximum: usize,
    scan: usize,
    complete: bool,
    failure: Option<HttpRequestError>,
}
impl HttpHeadBuffer {
    /// Create a head accumulator under the existing lowerable request limits.
    pub fn new(limits: HttpRequestLimits) -> Result<Self, HttpRequestError> {
        limits.validate()?;
        Ok(Self {
            bytes: Vec::new(),
            maximum: limits.head_bytes,
            scan: 0,
            complete: false,
            failure: None,
        })
    }
    /// Remaining admitted input bytes, including any co-read body prefix.
    pub fn remaining(&self) -> usize {
        self.maximum - self.bytes.len()
    }
    /// True only when CRLF CRLF has been found; semantic parsing is still required.
    pub fn complete(&self) -> bool {
        self.complete
    }
    /// Feed one fragment. Errors are sticky and no over-limit prefix is copied.
    pub fn feed(&mut self, fragment: &[u8]) -> Result<bool, HttpRequestError> {
        if let Some(e) = &self.failure {
            return Err(e.clone());
        }
        let refusal = if self.complete {
            Some(fail(
                "input",
                self.bytes.len(),
                "HTTP head accumulation already complete",
            ))
        } else if fragment.len() > self.remaining() {
            Some(fail(
                "limit",
                self.bytes.len(),
                "HTTP head accumulation limit exceeded",
            ))
        } else if self.bytes.try_reserve(fragment.len()).is_err() {
            Some(fail(
                "limit",
                self.bytes.len(),
                "HTTP head allocation refused",
            ))
        } else {
            None
        };
        if let Some(e) = refusal {
            self.failure = Some(e.clone());
            return Err(e);
        }
        self.bytes.extend_from_slice(fragment);
        while self.scan + 4 <= self.bytes.len() {
            if &self.bytes[self.scan..self.scan + 4] == b"\r\n\r\n" {
                self.complete = true;
                break;
            }
            self.scan += 1;
        }
        Ok(self.complete)
    }
    /// Consume the buffer without copying; callers must still parse and validate it.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
fn fail(code: &'static str, offset: usize, message: &'static str) -> HttpRequestError {
    HttpRequestError {
        code,
        offset,
        message,
    }
}
fn charge(work: &mut usize, n: usize, at: usize) -> Result<(), HttpRequestError> {
    *work = work
        .checked_sub(n)
        .ok_or_else(|| fail("limit", at, "HTTP parser work exhausted"))?;
    Ok(())
}
/// Original field name and OWS-trimmed byte value; obs-text is not decoded as UTF-8.
#[derive(Debug, Clone, Copy)]
pub struct HttpRequestHeader<'a> {
    name: &'a str,
    value: &'a [u8],
}
impl<'a> HttpRequestHeader<'a> {
    /// Original ASCII field spelling.
    pub fn name(&self) -> &'a str {
        self.name
    }
    /// Original value bytes excluding leading/trailing SP and HTAB.
    pub fn value(&self) -> &'a [u8] {
        self.value
    }
}
/// Complete single-request proof borrowing the exact input; fields cannot be forged.
#[derive(Debug)]
pub struct HttpRequest<'a> {
    method: &'a str,
    target: &'a str,
    headers: Vec<HttpRequestHeader<'a>>,
    body: &'a [u8],
}
impl<'a> HttpRequest<'a> {
    /// Case-sensitive method token, without interpreting handler support.
    pub fn method(&self) -> &'a str {
        self.method
    }
    /// Original origin-form target (or OPTIONS *); no decoding/normalization.
    pub fn target(&self) -> &'a str {
        self.target
    }
    /// Source-order field lines, including permitted non-framing repetitions.
    pub fn headers(&self) -> &[HttpRequestHeader<'a>] {
        &self.headers
    }
    /// Exact body bytes, which may contain NUL or invalid UTF-8.
    pub fn body(&self) -> &'a [u8] {
        self.body
    }
}
/// No partially parsed request is exposed before all bytes are admitted.
#[derive(Debug)]
pub enum HttpRequestParse<'a> {
    /// More bytes are needed. Once the head is complete the exact total is known.
    Incomplete {
        /// Required total input bytes, or None while the head is incomplete.
        minimum_bytes: Option<usize>,
    },
    /// One complete request with no trailing/pipelined bytes.
    Complete(HttpRequest<'a>),
}
fn incomplete<'a>(
    eof: bool,
    offset: usize,
    minimum_bytes: Option<usize>,
) -> Result<HttpRequestParse<'a>, HttpRequestError> {
    if eof {
        Err(fail(
            "truncated",
            offset,
            "HTTP request ended before its frame completed",
        ))
    } else {
        Ok(HttpRequestParse::Incomplete { minimum_bytes })
    }
}
pub(super) fn token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}
fn line<'a>(
    input: &'a [u8],
    start: usize,
    limits: HttpRequestLimits,
    work: &mut usize,
) -> Result<Option<(&'a [u8], usize)>, HttpRequestError> {
    let mut i = start;
    while i < input.len() {
        if i >= limits.head_bytes {
            return Err(fail("limit", i, "HTTP head byte limit exceeded"));
        }
        charge(work, 1, i)?;
        let b = input[i];
        if b == b'\r' {
            if i - start > limits.line_bytes || i + 2 > limits.head_bytes {
                return Err(fail("limit", i, "HTTP line/head limit exceeded"));
            }
            let Some(next) = input.get(i + 1) else {
                return Ok(None);
            };
            charge(work, 1, i + 1)?;
            if *next != b'\n' {
                return Err(fail("syntax", i + 1, "CR must be followed by LF"));
            }
            return Ok(Some((&input[start..i], i + 2)));
        }
        if i - start >= limits.line_bytes {
            return Err(fail("limit", i, "HTTP line limit exceeded"));
        }
        if b == b'\n' {
            return Err(fail("syntax", i, "bare LF refuses"));
        }
        i += 1;
    }
    if i >= limits.head_bytes {
        return Err(fail("limit", i, "HTTP head byte limit exceeded"));
    }
    Ok(None)
}
pub(super) fn target_valid(target: &[u8], method: &str) -> bool {
    if target == b"*" {
        return method == "OPTIONS";
    }
    if !target.starts_with(b"/") {
        return false;
    }
    let mut i = 0;
    while i < target.len() {
        let b = target[i];
        if b == b'%' {
            if i + 2 >= target.len()
                || !target[i + 1].is_ascii_hexdigit()
                || !target[i + 2].is_ascii_hexdigit()
            {
                return false;
            }
            i += 3;
            continue;
        }
        if !b.is_ascii_alphanumeric() && !b"-._~!$&'()*+,;=:@/?".contains(&b) {
            return false;
        }
        i += 1;
    }
    true
}
pub(super) fn authority(value: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(value) else {
        return false;
    };
    let port = if let Some(rest) = text.strip_prefix('[') {
        let Some(end) = rest.find(']') else {
            return false;
        };
        if rest[..end].parse::<std::net::Ipv6Addr>().is_err() {
            return false;
        }
        let tail = &rest[end + 1..];
        if tail.is_empty() {
            return true;
        }
        let Some(port) = tail.strip_prefix(':') else {
            return false;
        };
        Some(port)
    } else {
        let (host, port) = match text.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (text, None),
        };
        if host.is_empty() || host.len() > 253 {
            return false;
        }
        let host = host.strip_suffix('.').unwrap_or(host);
        if host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label.as_bytes()[0].is_ascii_alphanumeric()
                || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }) {
            return false;
        }
        port
    };
    match port {
        None => true,
        Some(port) => {
            !port.is_empty()
                && port.bytes().all(|b| b.is_ascii_digit())
                && port.parse::<u16>().is_ok()
        }
    }
}
/// Parse exactly one fixed-length HTTP/1.1 request, or report bounded incomplete input.
/// `eof` turns incomplete input into a truncated refusal. No authority is installed.
pub fn parse_http_request(
    input: &[u8],
    limits: HttpRequestLimits,
    eof: bool,
) -> Result<HttpRequestParse<'_>, HttpRequestError> {
    limits.validate()?;
    if input.len() > limits.head_bytes.saturating_add(limits.body_bytes) {
        return Err(fail("limit", 0, "HTTP aggregate input limit exceeded"));
    }
    let Some(head) = parse_http_head(input, limits, eof, false)? else {
        return incomplete(eof, input.len(), None);
    };
    let HttpBodyFraming::Fixed(length) = head.framing else {
        unreachable!("fixed-only head parsing cannot admit chunked");
    };
    let expected = head.bytes + length;
    if input.len() < expected {
        return incomplete(eof, input.len(), Some(expected));
    }
    if input.len() > expected {
        return Err(fail(
            "framing",
            expected,
            "trailing or pipelined request bytes refuse",
        ));
    }
    let mut request = head.request;
    request.body = &input[head.bytes..expected];
    Ok(HttpRequestParse::Complete(request))
}
#[derive(Debug, Clone, Copy)]
pub(super) enum HttpBodyFraming {
    Fixed(usize),
    Chunked,
}
#[derive(Debug)]
pub(super) struct HttpRequestHead<'a> {
    pub request: HttpRequest<'a>,
    pub framing: HttpBodyFraming,
    pub bytes: usize,
    pub work: usize,
    pub continue_expected: bool,
}
/// Internal checked framing decision; never publishes a complete request to source.
pub(super) fn parse_http_head(
    input: &[u8],
    limits: HttpRequestLimits,
    eof: bool,
    allow_chunked: bool,
) -> Result<Option<HttpRequestHead<'_>>, HttpRequestError> {
    limits.validate()?;
    let mut work = limits.work;
    let Some((start, mut cursor)) = line(input, 0, limits, &mut work)? else {
        return if eof {
            Err(fail(
                "truncated",
                input.len(),
                "HTTP head ended before completion",
            ))
        } else {
            Ok(None)
        };
    };
    charge(&mut work, start.len() * 2, 0)?;
    let mut parts = start.split(|b| *b == b' ');
    let method = parts.next().unwrap_or(&[]);
    let target = parts.next().unwrap_or(&[]);
    let version = parts.next().unwrap_or(&[]);
    if method.is_empty()
        || !method.iter().copied().all(token)
        || target.is_empty()
        || parts.next().is_some()
    {
        return Err(fail(
            "syntax",
            0,
            "request line requires method SP target SP version",
        ));
    }
    if version.len() != 8
        || !version.starts_with(b"HTTP/")
        || !version[5].is_ascii_digit()
        || version[6] != b'.'
        || !version[7].is_ascii_digit()
    {
        return Err(fail("syntax", 0, "invalid HTTP version syntax"));
    }
    if version != b"HTTP/1.1" {
        return Err(fail(
            "unsupported",
            0,
            "only the initial HTTP/1.1 profile is supported",
        ));
    }
    let method =
        std::str::from_utf8(method).map_err(|_| fail("syntax", 0, "invalid method token"))?;
    if method == "CONNECT" || target.starts_with(b"http://") || target.starts_with(b"https://") {
        return Err(fail(
            "unsupported",
            method.len() + 1,
            "request target form is not supported",
        ));
    }
    if !target_valid(target, method) {
        return Err(fail(
            "syntax",
            method.len() + 1,
            "invalid origin-form request target",
        ));
    }
    let target = std::str::from_utf8(target).map_err(|_| {
        fail(
            "syntax",
            method.len() + 1,
            "request target must be ASCII URI bytes",
        )
    })?;
    let mut headers = Vec::new();
    let mut host = false;
    let mut length = None;
    let mut transfer = false;
    let mut chunked = false;
    let mut unsupported = false;
    let mut trailer = false;
    let mut expect = None;
    loop {
        let at = cursor;
        let Some((field, next)) = line(input, cursor, limits, &mut work)? else {
            return if eof {
                Err(fail(
                    "truncated",
                    input.len(),
                    "HTTP head ended before completion",
                ))
            } else {
                Ok(None)
            };
        };
        cursor = next;
        if field.is_empty() {
            break;
        }
        if headers.len() >= limits.headers {
            return Err(fail("limit", at, "HTTP field count exceeded"));
        }
        charge(&mut work, field.len() * 4, at)?;
        let colon = field
            .iter()
            .position(|b| *b == b':')
            .ok_or_else(|| fail("syntax", at, "field colon is missing"))?;
        if colon == 0 || !field[..colon].iter().copied().all(token) {
            return Err(fail(
                "syntax",
                at,
                "invalid field name or whitespace before colon",
            ));
        }
        let name = std::str::from_utf8(&field[..colon])
            .map_err(|_| fail("syntax", at, "field name must be ASCII"))?;
        let raw = &field[colon + 1..];
        if raw.iter().any(|b| (*b < 32 && *b != b'\t') || *b == 127) {
            return Err(fail(
                "syntax",
                at + colon + 1,
                "control byte in field value",
            ));
        }
        let begin = raw
            .iter()
            .position(|b| *b != b' ' && *b != b'\t')
            .unwrap_or(raw.len());
        let value = &raw[begin..];
        let end = value
            .iter()
            .rposition(|b| *b != b' ' && *b != b'\t')
            .map(|n| n + 1)
            .unwrap_or(0);
        let value = &value[..end];
        let value_at = at + colon + 1 + begin;
        charge(&mut work, name.len() * 7, at)?;
        if name.eq_ignore_ascii_case("host") {
            if host {
                return Err(fail("framing", at, "multiple Host fields refuse"));
            }
            if !authority(value) {
                return Err(fail(
                    "syntax",
                    value_at,
                    "invalid Host authority for this profile",
                ));
            }
            host = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err(fail("framing", at, "multiple Content-Length fields refuse"));
            }
            if value.is_empty() {
                return Err(fail("framing", value_at, "empty Content-Length refuses"));
            }
            let mut n = 0usize;
            for (i, b) in value.iter().enumerate() {
                if !b.is_ascii_digit() {
                    return Err(fail(
                        "framing",
                        value_at + i,
                        "Content-Length must contain only decimal digits",
                    ));
                }
                n = n
                    .checked_mul(10)
                    .and_then(|n| n.checked_add((b - b'0') as usize))
                    .ok_or_else(|| fail("framing", value_at + i, "Content-Length overflow"))?;
            }
            if n > limits.body_bytes {
                return Err(fail("limit", value_at, "HTTP body limit exceeded"));
            }
            length = Some(n);
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            if transfer {
                return Err(fail(
                    "framing",
                    at,
                    "multiple Transfer-Encoding fields refuse",
                ));
            }
            transfer = true;
            chunked = value.eq_ignore_ascii_case(b"chunked");
        } else if name.eq_ignore_ascii_case("trailer") {
            if trailer {
                return Err(fail("framing", at, "multiple Trailer declarations refuse"));
            }
            trailer = true;
            charge(&mut work, value.len() * 32, value_at)?;
            let mut count = 0;
            for name in value.split(|b| *b == b',') {
                let begin = name
                    .iter()
                    .position(|b| *b != b' ' && *b != b'\t')
                    .unwrap_or(name.len());
                let name = &name[begin..];
                let end = name
                    .iter()
                    .rposition(|b| *b != b' ' && *b != b'\t')
                    .map(|n| n + 1)
                    .unwrap_or(0);
                let name = &name[..end];
                count += 1;
                if count > limits.headers {
                    return Err(fail(
                        "limit",
                        value_at,
                        "trailer declaration count exceeded",
                    ));
                }
                if name.is_empty() || !name.iter().all(|b| token(*b)) {
                    return Err(fail("syntax", value_at, "invalid Trailer declaration"));
                }
                if !super::http_chunked_v4::trailer_name_allowed(name) {
                    return Err(fail(
                        "unsupported",
                        value_at,
                        "prohibited Trailer declaration",
                    ));
                }
            }
        } else if name.eq_ignore_ascii_case("expect") {
            if expect.is_some() {
                return Err(fail("framing", at, "multiple Expect fields refuse"));
            }
            expect = Some(value.eq_ignore_ascii_case(b"100-continue"));
        } else if name.eq_ignore_ascii_case("upgrade") {
            unsupported = true;
        }
        headers.push(HttpRequestHeader { name, value });
    }
    if !host {
        return Err(fail("framing", cursor, "HTTP/1.1 Host field is required"));
    }
    if transfer && length.is_some() {
        return Err(fail(
            "framing",
            cursor,
            "Transfer-Encoding with Content-Length refuses",
        ));
    }
    if allow_chunked && expect == Some(false) {
        return Err(fail(
            "expectation",
            cursor,
            "requested expectation is not supported",
        ));
    }
    if (transfer && (!allow_chunked || !chunked))
        || (trailer && (!allow_chunked || !chunked))
        || (!allow_chunked && expect.is_some())
        || unsupported
    {
        return Err(fail(
            "unsupported",
            cursor,
            "transfer coding, expectations, trailers and upgrades are not implemented",
        ));
    }
    Ok(Some(HttpRequestHead {
        request: HttpRequest {
            method,
            target,
            headers,
            body: &[],
        },
        framing: if transfer {
            HttpBodyFraming::Chunked
        } else {
            HttpBodyFraming::Fixed(length.unwrap_or(0))
        },
        bytes: cursor,
        work,
        continue_expected: expect == Some(true),
    }))
}
