//! Pure bounded final-response rendering for the initial nonpersistent HTTP profile.
use super::http_request_v4::token;
/// Request context controls whether representation bytes may appear on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpResponseContext {
    /// Ordinary supported origin-form request; no tunnel or protocol switching.
    Standard,
    /// HEAD metadata: declare the supplied representation length but send no body.
    Head,
}
/// Untrusted response field input; validation occurs before output allocation.
#[derive(Debug, Clone, Copy)]
pub struct HttpResponseHeader<'a> {
    /// ASCII token field name.
    pub name: &'a str,
    /// Opaque field value without leading/trailing OWS or control bytes.
    pub value: &'a [u8],
}
/// Independently lowerable response allocation and logical-work ceilings.
#[derive(Debug, Clone, Copy)]
pub struct HttpResponseLimits {
    /// Aggregate status/field section bytes including final CRLF.
    pub head_bytes: usize,
    /// Maximum status/field line bytes excluding CRLF.
    pub line_bytes: usize,
    /// Total serialized field count, including generated framing fields.
    pub headers: usize,
    /// Supplied representation bytes, even for a HEAD response.
    pub body_bytes: usize,
    /// Actual complete output allocation size.
    pub output_bytes: usize,
    /// Logical validation/comparison/copy work, not physical CPU instructions.
    pub work: usize,
}
impl HttpResponseLimits {
    /// Initial finite response profile, independent of socket/request policy.
    pub const DEFAULT: Self = Self {
        head_bytes: 16384,
        line_bytes: 8192,
        headers: 64,
        body_bytes: 1024 * 1024,
        output_bytes: 16384 + 1024 * 1024,
        work: 2 * 1024 * 1024,
    };
    pub(super) fn validate(self) -> Result<(), HttpResponseError> {
        if self.head_bytes > Self::DEFAULT.head_bytes
            || self.line_bytes > Self::DEFAULT.line_bytes
            || self.headers > Self::DEFAULT.headers
            || self.body_bytes > Self::DEFAULT.body_bytes
            || self.output_bytes > Self::DEFAULT.output_bytes
            || self.work > Self::DEFAULT.work
        {
            Err(fail(
                "policy",
                None,
                "HTTP response limits may only narrow defaults",
            ))
        } else {
            Ok(())
        }
    }
}
/// Fixed refusal; no untrusted names, values or body bytes appear in messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponseError {
    /// policy, limit, status, unsupported, framing, syntax, body or internal.
    pub code: &'static str,
    /// Custom field ordinal when the refusal concerns one field.
    pub header: Option<usize>,
    /// Fixed explanation.
    pub message: &'static str,
}
fn fail(code: &'static str, header: Option<usize>, message: &'static str) -> HttpResponseError {
    HttpResponseError {
        code,
        header,
        message,
    }
}
fn charge(work: &mut usize, n: usize, header: Option<usize>) -> Result<(), HttpResponseError> {
    *work = work
        .checked_sub(n)
        .ok_or_else(|| fail("limit", header, "HTTP response work exhausted"))?;
    Ok(())
}
fn add(total: usize, n: usize) -> Result<usize, HttpResponseError> {
    total
        .checked_add(n)
        .ok_or_else(|| fail("limit", None, "HTTP response size overflow"))
}
fn reason(status: u16) -> &'static [u8] {
    match status {
        200 => b"OK",
        201 => b"Created",
        202 => b"Accepted",
        203 => b"Non-Authoritative Information",
        204 => b"No Content",
        205 => b"Reset Content",
        206 => b"Partial Content",
        301 => b"Moved Permanently",
        302 => b"Found",
        303 => b"See Other",
        304 => b"Not Modified",
        307 => b"Temporary Redirect",
        308 => b"Permanent Redirect",
        400 => b"Bad Request",
        401 => b"Unauthorized",
        403 => b"Forbidden",
        404 => b"Not Found",
        405 => b"Method Not Allowed",
        408 => b"Request Timeout",
        409 => b"Conflict",
        410 => b"Gone",
        411 => b"Length Required",
        413 => b"Content Too Large",
        414 => b"URI Too Long",
        415 => b"Unsupported Media Type",
        417 => b"Expectation Failed",
        422 => b"Unprocessable Content",
        429 => b"Too Many Requests",
        431 => b"Request Header Fields Too Large",
        500 => b"Internal Server Error",
        501 => b"Not Implemented",
        502 => b"Bad Gateway",
        503 => b"Service Unavailable",
        504 => b"Gateway Timeout",
        505 => b"HTTP Version Not Supported",
        _ => b"Status",
    }
}
fn decimal(mut value: usize) -> ([u8; 20], usize) {
    let mut buffer = [0; 20];
    let mut at = 20;
    loop {
        at -= 1;
        buffer[at] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    (buffer, at)
}
/// Validate and render one bounded final HTTP/1.1 response with Connection: close.
/// Socket writes, method selection and required application metadata remain caller work.
pub fn render_http_response(
    status: u16,
    context: HttpResponseContext,
    headers: &[HttpResponseHeader<'_>],
    body: &[u8],
    limits: HttpResponseLimits,
) -> Result<Vec<u8>, HttpResponseError> {
    limits.validate()?;
    if (100..200).contains(&status) {
        return Err(fail(
            "unsupported",
            None,
            "interim responses and protocol switching are not supported",
        ));
    }
    if !(200..=599).contains(&status) {
        return Err(fail("status", None, "final status must be in 200..599"));
    }
    if body.len() > limits.body_bytes {
        return Err(fail(
            "limit",
            None,
            "HTTP representation body limit exceeded",
        ));
    }
    let forbidden_body = matches!(status, 204 | 205 | 304);
    if forbidden_body && !body.is_empty() {
        return Err(fail(
            "body",
            None,
            "this status forbids representation content",
        ));
    }
    let length_allowed = !matches!(status, 204 | 304);
    let generated = 1 + usize::from(length_allowed);
    if headers.len() > limits.headers || generated > limits.headers - headers.len() {
        return Err(fail("limit", None, "HTTP response field count exceeded"));
    }
    let reason = reason(status);
    let status_line = 13 + reason.len();
    let (digits, start) = decimal(body.len());
    let digits = &digits[start..];
    if status_line > limits.line_bytes
        || 17 > limits.line_bytes
        || (length_allowed && 16 + digits.len() > limits.line_bytes)
    {
        return Err(fail(
            "limit",
            None,
            "generated HTTP response line exceeds limit",
        ));
    }
    let mut head = add(status_line + 2, 19)?; // status CRLF and Connection: close CRLF
    if length_allowed {
        head = add(head, 18 + digits.len())?;
    }
    head = add(head, 2)?; // end of field section
    let mut work = limits.work;
    for (i, field) in headers.iter().enumerate() {
        let line = add(add(field.name.len(), 2)?, field.value.len())?;
        if line > limits.line_bytes {
            return Err(fail(
                "limit",
                Some(i),
                "HTTP response field line exceeds limit",
            ));
        }
        head = add(head, add(line, 2)?)?;
        if head > limits.head_bytes {
            return Err(fail("limit", Some(i), "HTTP response head exceeds limit"));
        }
        charge(&mut work, line * 2, Some(i))?;
        if field.name.is_empty() || !field.name.bytes().all(token) {
            return Err(fail("syntax", Some(i), "invalid response field name"));
        }
        charge(&mut work, field.name.len() * 7, Some(i))?;
        for reserved in [
            "content-length",
            "transfer-encoding",
            "connection",
            "trailer",
            "upgrade",
            "keep-alive",
            "proxy-connection",
        ] {
            if field.name.eq_ignore_ascii_case(reserved) {
                return Err(fail(
                    "framing",
                    Some(i),
                    "response framing fields are renderer-owned",
                ));
            }
        }
        if field
            .value
            .iter()
            .any(|b| (*b < 32 && *b != b'\t') || *b == 127)
        {
            return Err(fail(
                "syntax",
                Some(i),
                "control byte in response field value",
            ));
        }
        if field
            .value
            .first()
            .is_some_and(|b| *b == b' ' || *b == b'\t')
            || field
                .value
                .last()
                .is_some_and(|b| *b == b' ' || *b == b'\t')
        {
            return Err(fail(
                "syntax",
                Some(i),
                "response field value must exclude surrounding OWS",
            ));
        }
        if !field.name.eq_ignore_ascii_case("set-cookie") {
            for previous in &headers[..i] {
                charge(&mut work, field.name.len() + previous.name.len(), Some(i))?;
                if field.name.eq_ignore_ascii_case(previous.name) {
                    return Err(fail("framing", Some(i), "duplicate response field refuses"));
                }
            }
        }
    }
    if head > limits.head_bytes {
        return Err(fail("limit", None, "HTTP response head exceeds limit"));
    }
    let send_body = context == HttpResponseContext::Standard && !forbidden_body;
    let total = add(head, if send_body { body.len() } else { 0 })?;
    if total > limits.output_bytes {
        return Err(fail("limit", None, "HTTP response output limit exceeded"));
    }
    charge(&mut work, total, None)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| fail("limit", None, "HTTP response allocation refused"))?;
    output.extend_from_slice(b"HTTP/1.1 ");
    output.extend_from_slice(&[
        b'0' + (status / 100) as u8,
        b'0' + ((status / 10) % 10) as u8,
        b'0' + (status % 10) as u8,
    ]);
    output.push(b' ');
    output.extend_from_slice(reason);
    output.extend_from_slice(b"\r\n");
    for field in headers {
        output.extend_from_slice(field.name.as_bytes());
        output.extend_from_slice(b": ");
        output.extend_from_slice(field.value);
        output.extend_from_slice(b"\r\n");
    }
    if length_allowed {
        output.extend_from_slice(b"Content-Length: ");
        output.extend_from_slice(digits);
        output.extend_from_slice(b"\r\n");
    }
    output.extend_from_slice(b"Connection: close\r\n\r\n");
    if send_body {
        output.extend_from_slice(body);
    }
    if output.len() != total {
        return Err(fail(
            "internal",
            None,
            "HTTP response sizing invariant failed",
        ));
    }
    Ok(output)
}
