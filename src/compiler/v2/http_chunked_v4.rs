//! Incremental bounded chunked-body decoding with explicit validated-discard trailers.
use super::http_request_v4::token;
use super::HttpRequestError;

/// Independent hard ceilings, lowerable by trusted callers.
#[derive(Debug, Clone, Copy)]
pub struct HttpChunkedLimits {
    /// Total encoded bytes, including framing and extensions.
    pub wire_bytes: usize,
    /// Total decoded bytes.
    pub body_bytes: usize,
    /// Chunk-size/extension line bytes, excluding CRLF.
    pub line_bytes: usize,
    /// Nonzero chunks admitted.
    pub chunks: usize,
    /// Aggregate logical work across all feeds, independent of fragmentation.
    pub work: usize,
}
impl HttpChunkedLimits {
    /// Finite experimental decoder defaults; not a complete server policy.
    pub const DEFAULT: Self = Self {
        wire_bytes: 2 * 1024 * 1024,
        body_bytes: 1024 * 1024,
        line_bytes: 8192,
        chunks: 4096,
        work: 8 * 1024 * 1024,
    };
}
/// Independent ceilings for validated trailer fields, which are discarded, never merged.
#[derive(Debug, Clone, Copy)]
pub struct HttpTrailerLimits {
    /// Aggregate trailer-section bytes, including every CRLF and the final CRLF.
    pub bytes: usize,
    /// Field line bytes excluding CRLF.
    pub line_bytes: usize,
    /// Maximum nonempty field lines.
    pub fields: usize,
}
impl HttpTrailerLimits {
    /// Hard ceilings; socket hosts inherit lower HTTP head/line/field settings.
    pub const DEFAULT: Self = Self {
        bytes: 16384,
        line_bytes: 8192,
        fields: 64,
    };
}
#[derive(Debug, Clone, Copy)]
enum State {
    Size,
    SizeLf,
    Data(usize),
    DataCr,
    DataLf,
    Trailer,
    TrailerLf,
    Done,
}
/// Single-use body decoder. No body is exposed until complete validation.
/// Errors are sticky; trailers require explicit discard admission, and trailing bytes refuse.
#[derive(Debug)]
pub struct HttpChunkedDecoder {
    limits: HttpChunkedLimits,
    state: State,
    line: Vec<u8>,
    body: Vec<u8>,
    wire: usize,
    chunks: usize,
    work: usize,
    failure: Option<HttpRequestError>,
    trailers: Option<HttpTrailerLimits>,
    trailer_bytes: usize,
    trailer_fields: usize,
}
fn error(code: &'static str, offset: usize, message: &'static str) -> HttpRequestError {
    HttpRequestError {
        code,
        offset,
        message,
    }
}
impl HttpChunkedDecoder {
    /// Validate ceilings before allocating or accepting peer bytes.
    pub fn new(limits: HttpChunkedLimits) -> Result<Self, HttpRequestError> {
        let cap = HttpChunkedLimits::DEFAULT;
        if limits.wire_bytes > cap.wire_bytes
            || limits.body_bytes > cap.body_bytes
            || limits.line_bytes > cap.line_bytes
            || limits.chunks > cap.chunks
            || limits.work > cap.work
        {
            return Err(error(
                "policy",
                0,
                "chunked limits may only narrow defaults",
            ));
        }
        Ok(Self {
            limits,
            state: State::Size,
            line: Vec::new(),
            body: Vec::new(),
            wire: 0,
            chunks: 0,
            work: limits.work,
            failure: None,
            trailers: None,
            trailer_bytes: 0,
            trailer_fields: 0,
        })
    }
    /// Admit bounded, syntax-checked trailer fields for discard only. No fields are exposed
    /// or merged into headers. Known framing/routing/authentication fields refuse.
    /// `new` retains its original empty-trailer-only contract.
    pub fn with_discarded_trailers(
        limits: HttpChunkedLimits,
        trailers: HttpTrailerLimits,
    ) -> Result<Self, HttpRequestError> {
        let cap = HttpTrailerLimits::DEFAULT;
        if trailers.bytes > cap.bytes
            || trailers.line_bytes > cap.line_bytes
            || trailers.fields > cap.fields
        {
            return Err(error(
                "policy",
                0,
                "trailer limits may only narrow defaults",
            ));
        }
        let mut decoder = Self::new(limits)?;
        decoder.trailers = Some(trailers);
        Ok(decoder)
    }
    /// Consume only new bytes. Returns true once the final empty trailer line completes.
    /// A later nonempty feed revokes completion; errors never permit body extraction.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<bool, HttpRequestError> {
        if let Some(e) = &self.failure {
            return Err(e.clone());
        }
        let result = self.accept(bytes);
        if let Err(e) = &result {
            self.failure = Some(e.clone());
            self.body.clear();
            self.line.clear();
        }
        result
    }
    fn charge(&mut self, n: usize, at: usize) -> Result<(), HttpRequestError> {
        self.work = self
            .work
            .checked_sub(n)
            .ok_or_else(|| error("limit", at, "chunked work exhausted"))?;
        Ok(())
    }
    fn accept(&mut self, bytes: &[u8]) -> Result<bool, HttpRequestError> {
        for &b in bytes {
            let at = self.wire;
            if at >= self.limits.wire_bytes {
                return Err(error("limit", at, "chunked wire limit exceeded"));
            }
            self.charge(1, at)?;
            self.wire += 1;
            self.state = match self.state {
                State::Size => {
                    if b == b'\r' {
                        State::SizeLf
                    } else {
                        if b == b'\n' {
                            return Err(error("syntax", at, "bare LF refuses"));
                        }
                        if self.line.len() >= self.limits.line_bytes {
                            return Err(error("limit", at, "chunk line limit exceeded"));
                        }
                        self.line
                            .try_reserve(1)
                            .map_err(|_| error("limit", at, "chunk line allocation refused"))?;
                        self.line.push(b);
                        State::Size
                    }
                }
                State::SizeLf => {
                    if b != b'\n' {
                        return Err(error("syntax", at, "CR must be followed by LF"));
                    }
                    // A fixed linear precharge covers size and extension validation, once per line.
                    self.charge(self.line.len() * 4, at)?;
                    let n = size(&self.line, self.wire - self.line.len() - 2)?;
                    self.line.clear();
                    if n == 0 {
                        State::Trailer
                    } else {
                        if self.chunks >= self.limits.chunks {
                            return Err(error("limit", at, "chunk count exceeded"));
                        }
                        self.chunks += 1;
                        if n > self.limits.body_bytes.saturating_sub(self.body.len()) {
                            return Err(error("limit", at, "decoded body limit exceeded"));
                        }
                        // Grow geometrically, capped by admitted content bytes, rather than
                        // reallocating the accumulated body for every tiny chunk.
                        let needed = self.body.len() + n;
                        if needed > self.body.capacity() {
                            let capacity = needed
                                .max(self.body.capacity().saturating_mul(2).max(8))
                                .min(self.limits.body_bytes);
                            self.body
                                .try_reserve_exact(capacity - self.body.len())
                                .map_err(|_| error("limit", at, "chunk body allocation refused"))?;
                        }
                        State::Data(n)
                    }
                }
                State::Data(n) => {
                    self.body.push(b);
                    if n == 1 {
                        State::DataCr
                    } else {
                        State::Data(n - 1)
                    }
                }
                State::DataCr => {
                    if b != b'\r' {
                        return Err(error("framing", at, "chunk data CRLF missing"));
                    }
                    State::DataLf
                }
                State::DataLf => {
                    if b != b'\n' {
                        return Err(error("framing", at, "chunk data CRLF missing"));
                    }
                    State::Size
                }
                State::Trailer => {
                    self.admit_trailer_byte(at)?;
                    if b == b'\r' {
                        State::TrailerLf
                    } else {
                        let Some(limits) = self.trailers else {
                            return Err(error(
                                "unsupported",
                                at,
                                "nonempty trailers are not admitted",
                            ));
                        };
                        if b == b'\n' {
                            return Err(error("syntax", at, "bare LF in trailer refuses"));
                        }
                        if self.line.len() >= limits.line_bytes {
                            return Err(error("limit", at, "trailer line limit exceeded"));
                        }
                        self.line
                            .try_reserve(1)
                            .map_err(|_| error("limit", at, "trailer allocation refused"))?;
                        self.line.push(b);
                        State::Trailer
                    }
                }
                State::TrailerLf => {
                    self.admit_trailer_byte(at)?;
                    if b != b'\n' {
                        return Err(error("syntax", at, "trailer CR must be followed by LF"));
                    }
                    if self.line.is_empty() {
                        State::Done
                    } else {
                        let limits = self
                            .trailers
                            .expect("nonempty trailer requires explicit admission");
                        if self.trailer_fields >= limits.fields {
                            return Err(error("limit", at, "trailer field count exceeded"));
                        }
                        self.charge(self.line.len() * 32, at)?;
                        validate_trailer(&self.line, self.wire - self.line.len() - 2)?;
                        self.trailer_fields += 1;
                        self.line.clear();
                        State::Trailer
                    }
                }
                State::Done => {
                    return Err(error("framing", at, "trailing or pipelined bytes refuse"))
                }
            };
        }
        Ok(matches!(self.state, State::Done))
    }
    fn admit_trailer_byte(&mut self, at: usize) -> Result<(), HttpRequestError> {
        if let Some(limits) = self.trailers {
            if self.trailer_bytes >= limits.bytes {
                return Err(error("limit", at, "trailer section byte limit exceeded"));
            }
        }
        self.trailer_bytes += 1;
        Ok(())
    }
    /// Consume ownership and return decoded bytes only after a complete, error-free frame.
    pub fn finish(self) -> Result<Vec<u8>, HttpRequestError> {
        if let Some(e) = self.failure {
            return Err(e);
        }
        if !matches!(self.state, State::Done) {
            return Err(error(
                "truncated",
                self.wire,
                "chunked body ended before final CRLF",
            ));
        }
        Ok(self.body)
    }
}
fn whitespace(line: &[u8], cursor: &mut usize) {
    while line.get(*cursor).is_some_and(|b| *b == b' ' || *b == b'\t') {
        *cursor += 1;
    }
}
fn size(line: &[u8], offset: usize) -> Result<usize, HttpRequestError> {
    let mut i = 0;
    let mut n = 0usize;
    while let Some(digit) = line.get(i).and_then(|b| (*b as char).to_digit(16)) {
        n = n
            .checked_mul(16)
            .and_then(|n| n.checked_add(digit as usize))
            .ok_or_else(|| error("framing", offset + i, "chunk size overflow"))?;
        i += 1;
    }
    if i == 0 {
        return Err(error(
            "syntax",
            offset,
            "chunk size requires hexadecimal digits",
        ));
    }
    // RFC 9112 section 7.1.1: BWS ';' BWS token [ BWS '=' BWS (token / quoted-string) ].
    while i < line.len() {
        whitespace(line, &mut i);
        if line.get(i) != Some(&b';') {
            return Err(error(
                "syntax",
                offset + i,
                "invalid chunk extension separator",
            ));
        }
        i += 1;
        whitespace(line, &mut i);
        let start = i;
        while line.get(i).is_some_and(|b| token(*b)) {
            i += 1;
        }
        if i == start {
            return Err(error("syntax", offset + i, "chunk extension name required"));
        }
        let name_end = i;
        whitespace(line, &mut i);
        if line.get(i) == Some(&b'=') {
            i += 1;
            whitespace(line, &mut i);
            if line.get(i) == Some(&b'"') {
                i += 1;
                loop {
                    match line.get(i).copied() {
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(b'\\') => {
                            i += 1;
                            if !line.get(i).is_some_and(|b| {
                                *b == b'\t' || *b == b' ' || (33..=126).contains(b) || *b >= 128
                            }) {
                                return Err(error(
                                    "syntax",
                                    offset + i,
                                    "invalid quoted extension escape",
                                ));
                            }
                            i += 1;
                        }
                        Some(b)
                            if b == b'\t'
                                || b == b' '
                                || b == 33
                                || (35..=91).contains(&b)
                                || (93..=126).contains(&b)
                                || b >= 128 =>
                        {
                            i += 1
                        }
                        _ => {
                            return Err(error(
                                "syntax",
                                offset + i,
                                "invalid quoted extension value",
                            ))
                        }
                    }
                }
            } else {
                let start = i;
                while line.get(i).is_some_and(|b| token(*b)) {
                    i += 1;
                }
                if start == i {
                    return Err(error("syntax", offset + i, "extension value required"));
                }
            }
        } else {
            // Leave BWS to the next semicolon production; trailing BWS is not standalone grammar.
            i = name_end;
        }
    }
    Ok(n)
}

/// Conservative refusal of known fields affecting framing, routing, authentication or content
/// processing. All other fields are discarded, never interpreted as authority or verified metadata.
pub(super) fn trailer_name_allowed(name: &[u8]) -> bool {
    ![
        "content-length",
        "transfer-encoding",
        "host",
        "trailer",
        "connection",
        "te",
        "upgrade",
        "expect",
        "authorization",
        "proxy-authorization",
        "proxy-authenticate",
        "www-authenticate",
        "authentication-info",
        "proxy-authentication-info",
        "cookie",
        "set-cookie",
        "content-type",
        "content-encoding",
        "content-range",
        "range",
        "if-match",
        "if-none-match",
        "if-range",
        "if-modified-since",
        "if-unmodified-since",
        "max-forwards",
    ]
    .iter()
    .any(|field| name.eq_ignore_ascii_case(field.as_bytes()))
}
fn validate_trailer(line: &[u8], offset: usize) -> Result<(), HttpRequestError> {
    let colon = line
        .iter()
        .position(|b| *b == b':')
        .ok_or_else(|| error("syntax", offset, "trailer colon missing"))?;
    if colon == 0 || !line[..colon].iter().all(|b| token(*b)) {
        return Err(error("syntax", offset, "invalid trailer field name"));
    }
    if !trailer_name_allowed(&line[..colon]) {
        return Err(error("unsupported", offset, "prohibited trailer field"));
    }
    if line[colon + 1..]
        .iter()
        .any(|b| (*b < 32 && *b != b'\t') || *b == 127)
    {
        return Err(error(
            "syntax",
            offset + colon + 1,
            "control in trailer value",
        ));
    }
    Ok(())
}
