//! One-request owned socket framing. No worker threads, callbacks into source or persistence.
use super::http_request_v4::{parse_http_head, HttpBodyFraming};
use super::{
    render_http_response, HostError, HttpChunkedDecoder, HttpChunkedLimits, HttpRequest,
    HttpRequestLimits, HttpResponseContext, HttpResponseHeader, HttpResponseLimits,
    HttpTrailerLimits, JsonValue as J, LoopbackStream, NetworkLimits, NetworkRead,
    RuntimeBytesLimits, RuntimeBytesV2, RuntimeDataWork, RuntimeValueFuel,
};
use crate::compiler::execution::MaterializationLimits;
use std::time::{Duration, Instant};
fn error(code: &'static str, message: &'static str) -> HostError {
    HostError {
        code,
        message: message.into(),
    }
}
/// Aggregate HTTP limits retained by each claimed connection; only narrowing is allowed.
#[derive(Debug, Clone, Copy)]
pub struct HttpExchangeLimits {
    /// Request parsing/allocation ceilings.
    pub request: HttpRequestLimits,
    /// Response rendering/allocation ceilings.
    pub response: HttpResponseLimits,
    /// Total receive/handler/send lifetime, measured from TCP accept.
    pub milliseconds: u64,
    /// Total successful or attempted socket read/write operations across the exchange.
    pub io_operations: usize,
}
impl HttpExchangeLimits {
    /// Initial fixed-length/chunked one-request profile, not production HTTP conformance.
    pub const DEFAULT: Self = Self {
        request: HttpRequestLimits::DEFAULT,
        response: HttpResponseLimits::DEFAULT,
        milliseconds: 30000,
        io_operations: 4096,
    };
    pub(super) fn validate(self) -> Result<(), HostError> {
        self.request
            .validate()
            .map_err(|_| error("policy", "invalid HTTP request limits"))?;
        self.response
            .validate()
            .map_err(|_| error("policy", "invalid HTTP response limits"))?;
        if self.milliseconds > 30000 || self.io_operations > 4096 {
            return Err(error(
                "policy",
                "HTTP exchange limits may only narrow defaults",
            ));
        }
        Ok(())
    }
}
#[derive(Debug)]
pub(super) struct HttpConnection {
    stream: LoopbackStream,
    context: HttpResponseContext,
    started: Instant,
    milliseconds: u64,
    operations: usize,
    bytes: usize,
    limits: HttpExchangeLimits,
}
enum Stop<E> {
    External(E),
    Timeout,
}
fn checked<E>(
    started: Instant,
    milliseconds: u64,
    checkpoint: &mut impl FnMut() -> Result<(), E>,
) -> Result<(), Stop<E>> {
    checkpoint().map_err(Stop::External)?;
    if started.elapsed() >= Duration::from_millis(milliseconds) {
        Err(Stop::Timeout)
    } else {
        Ok(())
    }
}
fn flatten<T, E>(result: Result<Result<T, HostError>, Stop<E>>) -> Result<Result<T, HostError>, E> {
    match result {
        Err(Stop::External(e)) => Err(e),
        Err(Stop::Timeout) => Ok(Err(error(
            "timeout",
            "aggregate HTTP exchange deadline exceeded",
        ))),
        Ok(value) => Ok(value),
    }
}
impl HttpConnection {
    fn new(
        stream: LoopbackStream,
        accepted: Instant,
        socket: NetworkLimits,
        limits: HttpExchangeLimits,
    ) -> Self {
        Self {
            stream,
            context: HttpResponseContext::Standard,
            started: accepted,
            milliseconds: socket.milliseconds.min(limits.milliseconds),
            operations: 0,
            bytes: socket.bytes,
            limits,
        }
    }
    fn admit(&mut self) -> Result<(), HostError> {
        if self.operations >= self.limits.io_operations || self.bytes == 0 {
            return Err(error("limit", "HTTP I/O admission exhausted"));
        }
        self.operations += 1;
        Ok(())
    }
    fn read<E>(
        &mut self,
        maximum: usize,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<NetworkRead, HostError>, E> {
        if let Err(e) = self.admit() {
            return Ok(Err(e));
        }
        let (started, milliseconds) = (self.started, self.milliseconds);
        flatten(self.stream.read(maximum.min(self.bytes), || {
            checked(started, milliseconds, checkpoint)
        }))
    }
    fn write<E>(
        &mut self,
        wire: &[u8],
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<(), HostError>, E> {
        let mut at = 0;
        while at < wire.len() {
            if let Err(e) = self.admit() {
                return Ok(Err(e));
            }
            let (started, milliseconds) = (self.started, self.milliseconds);
            let end = wire.len().min(at.saturating_add(self.bytes));
            match flatten(self.stream.write(&wire[at..end], || {
                checked(started, milliseconds, checkpoint)
            }))? {
                Ok(n) => at += n,
                Err(e) => return Ok(Err(e)),
            }
        }
        Ok(Ok(()))
    }
    fn send_continue<E>(
        &mut self,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<(), HostError>, E> {
        const WIRE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";
        let response = self.limits.response;
        if response.head_bytes < WIRE.len()
            || response.output_bytes < WIRE.len()
            || response.line_bytes < WIRE.len() - 4
            || response.work < WIRE.len()
        {
            return Ok(Err(error("limit", "interim HTTP response limits exceeded")));
        }
        self.write(WIRE, checkpoint)
    }
    /// Explicit closure still releases the descriptor if shutdown reports an error.
    pub(super) fn close(self) -> Result<(), HostError> {
        self.stream.close()
    }
    pub(super) fn respond<E>(
        mut self,
        status: u16,
        headers: &[HttpResponseHeader<'_>],
        body: &[u8],
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        let wire =
            match render_http_response(status, self.context, headers, body, self.limits.response) {
                Ok(wire) => wire,
                Err(e) => {
                    return Ok(Err(HostError {
                        code: e.code,
                        message: e.message.into(),
                    }))
                }
            };
        match self.write(&wire, &mut checkpoint)? {
            Err(e) => Ok(Err(e)),
            Ok(()) => Ok(self.close().map(|_| J::Null)),
        }
    }
    fn reject<E>(
        &mut self,
        code: &'static str,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        let status = match code {
            "limit" => 413,
            "unsupported" => 501,
            "expectation" => 417,
            "authority" => 403,
            "admission" => 429,
            "authentication" => 401,
            "authentication_unavailable" => 503,
            _ => 400,
        };
        // Empty automatic error bodies are safe for HEAD even before method proof.
        let challenge = [HttpResponseHeader {
            name: "WWW-Authenticate",
            value: b"Bearer realm=\"Sovra\"",
        }];
        let no_store = [HttpResponseHeader {
            name: "Cache-Control",
            value: b"no-store",
        }];
        let headers: &[HttpResponseHeader<'_>] = match status {
            401 => &challenge,
            429 => &no_store,
            _ => &[],
        };
        if let Ok(wire) = render_http_response(
            status,
            HttpResponseContext::Standard,
            headers,
            b"",
            self.limits.response,
        ) {
            let _ = self.write(&wire, checkpoint)?;
        }
        Ok(())
    }
}
/// Refuse a claimed raw owner without reading a head or invoking any verifier.
pub(super) fn reject_admission<E>(
    stream: LoopbackStream,
    accepted: Instant,
    socket: NetworkLimits,
    limits: HttpExchangeLimits,
    mut checkpoint: impl FnMut() -> Result<(), E>,
) -> Result<Result<J, HostError>, E> {
    let mut connection = HttpConnection::new(stream, accepted, socket, limits);
    connection.reject("admission", &mut checkpoint)?;
    Ok(Err(error("admission", "HTTP request admission exhausted")))
}
/// Owned head preparation only: no parsed request or handler authority is published.
#[derive(Debug)]
pub(super) struct PendingHead {
    connection: HttpConnection,
    buffer: super::HttpHeadBuffer,
    eof: bool,
}
impl PendingHead {
    pub(super) fn new(
        stream: LoopbackStream,
        accepted: Instant,
        socket: NetworkLimits,
        limits: HttpExchangeLimits,
    ) -> Result<Self, HostError> {
        Ok(Self {
            connection: HttpConnection::new(stream, accepted, socket, limits),
            buffer: super::HttpHeadBuffer::new(limits.request)
                .map_err(|e| error(e.code, e.message))?,
            eof: false,
        })
    }
    fn ready(&self) -> bool {
        self.eof || self.buffer.complete() || self.buffer.remaining() == 0
    }
    pub(super) fn poll<E>(
        &mut self,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<bool, HostError>, E> {
        let connection = &mut self.connection;
        if let Err(e) = flatten(
            checked(connection.started, connection.milliseconds, checkpoint).map(|_| Ok(())),
        )? {
            return Ok(Err(e));
        }
        if self.eof || self.buffer.complete() || self.buffer.remaining() == 0 {
            return Ok(Ok(true));
        }
        if let Err(e) = connection.admit() {
            return Ok(Err(e));
        }
        let (started, milliseconds) = (connection.started, connection.milliseconds);
        match flatten(
            connection
                .stream
                .try_read(self.buffer.remaining().min(connection.bytes), || {
                    checked(started, milliseconds, checkpoint)
                }),
        )? {
            Err(e) => return Ok(Err(e)),
            Ok(None) => {}
            Ok(Some(NetworkRead::Eof)) => self.eof = true,
            Ok(Some(NetworkRead::Data(bytes))) => {
                if let Err(e) = self.buffer.feed(&bytes) {
                    return Ok(Err(error(e.code, e.message)));
                }
            }
        }
        Ok(Ok(self.ready()))
    }
    pub(super) fn close(self) -> Result<(), HostError> {
        self.connection.close()
    }
}
/// Consume pending head custody; strict validation precedes handler publication.
pub(super) fn receive_pending<E>(
    pending: PendingHead,
    access: Option<&super::HttpAccessPolicy>,
    verifier: Option<&mut super::http_authentication_v4::Verifier>,
    mut checkpoint: impl FnMut() -> Result<(), E>,
) -> Result<Result<(HttpConnection, J), HostError>, E> {
    let PendingHead {
        mut connection,
        buffer: mut pending,
        mut eof,
    } = pending;
    let limits = connection.limits;
    if let Err(e) = flatten(
        checked(connection.started, connection.milliseconds, &mut checkpoint).map(|_| Ok(())),
    )? {
        return Ok(Err(e));
    }
    while !pending.complete() && !eof && pending.remaining() > 0 {
        match connection.read(pending.remaining(), &mut checkpoint)? {
            Err(e) => return Ok(Err(e)),
            Ok(NetworkRead::Eof) => eof = true,
            Ok(NetworkRead::Data(bytes)) => {
                if let Err(e) = pending.feed(&bytes) {
                    return Ok(Err(error(e.code, e.message)));
                }
            }
        }
    }
    let buffer = pending.into_bytes();
    let mut head = match parse_http_head(&buffer, limits.request, eof, true) {
        Ok(Some(head)) => head,
        Ok(None) => {
            connection.reject("limit", &mut checkpoint)?;
            return Ok(Err(error(
                "limit",
                "HTTP head did not complete within limit",
            )));
        }
        Err(e) => {
            connection.reject(e.code, &mut checkpoint)?;
            return Ok(Err(error(e.code, e.message)));
        }
    };
    if let Some(access) = access {
        if let Err(e) = access.admit(&head.request, &mut head.work) {
            connection.reject(e.code, &mut checkpoint)?;
            return Ok(Err(e));
        }
    }
    let principal = if let Some(verifier) = verifier {
        if let Err(e) = flatten(
            checked(connection.started, connection.milliseconds, &mut checkpoint).map(|_| Ok(())),
        )? {
            return Ok(Err(e));
        }
        let mut stopped = None;
        let mut poll = || {
            if stopped.is_some() {
                return false;
            }
            match checked(connection.started, connection.milliseconds, &mut checkpoint) {
                Ok(()) => true,
                Err(stop) => {
                    stopped = Some(stop);
                    false
                }
            }
        };
        let outcome = verifier.authenticate(
            &head.request,
            &mut head.work,
            &mut super::HttpBearerControl::new(&mut poll),
        );
        match stopped {
            Some(Stop::External(e)) => return Err(e),
            Some(Stop::Timeout) => {
                return Ok(Err(error(
                    "timeout",
                    "aggregate HTTP exchange deadline exceeded",
                )))
            }
            None => {}
        }
        if let Err(e) = flatten(
            checked(connection.started, connection.milliseconds, &mut checkpoint).map(|_| Ok(())),
        )? {
            return Ok(Err(e));
        }
        match outcome {
            Ok(principal) => Some(principal),
            Err(e) => {
                connection.reject(e.code, &mut checkpoint)?;
                return Ok(Err(e));
            }
        }
    } else {
        None
    };
    let body = match receive_body(
        &mut connection,
        &buffer[head.bytes..],
        head.framing,
        head.work,
        head.continue_expected,
        &mut checkpoint,
    )? {
        Ok(body) => body,
        Err(e) => {
            connection.reject(e.code, &mut checkpoint)?;
            return Ok(Err(e));
        }
    };
    let value = match describe(&head.request, &body, principal.as_ref(), &mut || {
        checked(connection.started, connection.milliseconds, &mut checkpoint)
    }) {
        Ok(Ok(value)) => value,
        Ok(Err(e)) => return Ok(Err(e)),
        Err(Stop::External(e)) => return Err(e),
        Err(Stop::Timeout) => {
            return Ok(Err(error(
                "timeout",
                "aggregate HTTP exchange deadline exceeded",
            )))
        }
    };
    connection.context = if head.request.method() == "HEAD" {
        HttpResponseContext::Head
    } else {
        HttpResponseContext::Standard
    };
    Ok(Ok((connection, value)))
}
fn receive_body<E>(
    connection: &mut HttpConnection,
    initial: &[u8],
    framing: HttpBodyFraming,
    work: usize,
    continue_expected: bool,
    checkpoint: &mut impl FnMut() -> Result<(), E>,
) -> Result<Result<Vec<u8>, HostError>, E> {
    match framing {
        HttpBodyFraming::Fixed(length) => {
            let mut body = match super::HttpFixedBody::new(length, connection.limits.request) {
                Ok(body) => body,
                Err(e) => return Ok(Err(error(e.code, e.message))),
            };
            if let Err(e) = body.feed(initial) {
                return Ok(Err(error(e.code, e.message)));
            }
            if continue_expected && body.remaining() > 0 {
                if let Err(e) = connection.send_continue(checkpoint)? {
                    return Ok(Err(e));
                }
            }
            while body.remaining() > 0 {
                match connection.read(body.remaining(), checkpoint)? {
                    Err(e) => return Ok(Err(e)),
                    Ok(NetworkRead::Eof) => {
                        return Ok(Err(error("truncated", "HTTP body ended before completion")))
                    }
                    Ok(NetworkRead::Data(bytes)) => {
                        if let Err(e) = body.feed(&bytes) {
                            return Ok(Err(error(e.code, e.message)));
                        }
                    }
                }
            }
            Ok(body.finish().map_err(|e| error(e.code, e.message)))
        }
        HttpBodyFraming::Chunked => {
            let limits = HttpChunkedLimits {
                body_bytes: connection.limits.request.body_bytes,
                line_bytes: connection.limits.request.line_bytes,
                work,
                ..HttpChunkedLimits::DEFAULT
            };
            let trailers = HttpTrailerLimits {
                bytes: connection.limits.request.head_bytes,
                line_bytes: connection.limits.request.line_bytes,
                fields: connection.limits.request.headers,
            };
            let mut decoder = match HttpChunkedDecoder::with_discarded_trailers(limits, trailers) {
                Ok(d) => d,
                Err(e) => return Ok(Err(error(e.code, e.message))),
            };
            let mut seen = initial.len();
            let mut complete = match decoder.feed(initial) {
                Ok(done) => done,
                Err(e) => return Ok(Err(error(e.code, e.message))),
            };
            if continue_expected && !complete {
                if let Err(e) = connection.send_continue(checkpoint)? {
                    return Ok(Err(e));
                }
            }
            while !complete {
                let maximum = limits.wire_bytes.saturating_sub(seen);
                if maximum == 0 {
                    return Ok(Err(error("limit", "chunked wire limit exceeded")));
                }
                match connection.read(maximum, checkpoint)? {
                    Err(e) => return Ok(Err(e)),
                    Ok(NetworkRead::Eof) => break,
                    Ok(NetworkRead::Data(bytes)) => {
                        seen += bytes.len();
                        complete = match decoder.feed(&bytes) {
                            Ok(done) => done,
                            Err(e) => return Ok(Err(error(e.code, e.message))),
                        };
                    }
                }
            }
            Ok(decoder.finish().map_err(|e| error(e.code, e.message)))
        }
    }
}
fn codec_fuel() -> RuntimeValueFuel {
    RuntimeValueFuel::new(MaterializationLimits {
        max_value_nodes: Some(2 * 1024 * 1024 + 16),
        max_text_bytes: Some(4 * 1024 * 1024),
    })
}
pub(super) fn encode(bytes: &[u8]) -> Result<String, HostError> {
    let mut fuel = codec_fuel();
    let mut work = RuntimeDataWork::new(64 * 1024 * 1024);
    let limits = RuntimeBytesLimits {
        max_bytes: 1024 * 1024,
        ..RuntimeBytesLimits::DEFAULT
    };
    let bytes = RuntimeBytesV2::from_slice(bytes, limits, &mut fuel, &mut work)
        .map_err(|_| error("limit", "HTTP byte output refused"))?;
    bytes
        .to_base64(limits, &mut fuel, &mut work)
        .map_err(|_| error("limit", "HTTP Base64 output refused"))
}
pub(super) fn decode(value: &J, maximum: usize) -> Result<Vec<u8>, HostError> {
    let text = match value {
        J::String(text) => text,
        _ => return Err(error("input", "HTTP body must be Base64 String")),
    };
    if text.len() > maximum.div_ceil(3) * 4 {
        return Err(error("limit", "HTTP Base64 body limit exceeded"));
    }
    let mut fuel = codec_fuel();
    let mut work = RuntimeDataWork::new(64 * 1024 * 1024);
    let limits = RuntimeBytesLimits {
        max_bytes: maximum,
        ..RuntimeBytesLimits::DEFAULT
    };
    RuntimeBytesV2::from_base64(text, limits, &mut fuel, &mut work)
        .map(|v| v.as_slice().to_vec())
        .map_err(|_| error("input", "strict bounded HTTP Base64 required"))
}
fn describe<E>(
    request: &HttpRequest<'_>,
    body: &[u8],
    principal: Option<&super::HttpPrincipal>,
    checkpoint: &mut impl FnMut() -> Result<(), E>,
) -> Result<Result<J, HostError>, E> {
    checkpoint()?;
    let mut headers = Vec::new();
    for field in request.headers() {
        checkpoint()?;
        if principal.is_some() && field.name().eq_ignore_ascii_case("authorization") {
            continue;
        }
        let value = match encode(field.value()) {
            Ok(v) => v,
            Err(e) => return Ok(Err(e)),
        };
        headers.push(J::Object(vec![
            ("name".into(), J::String(field.name().into())),
            ("value".into(), J::String(value)),
        ]));
    }
    let body = match encode(body) {
        Ok(v) => v,
        Err(e) => return Ok(Err(e)),
    };
    checkpoint()?;
    let mut description = vec![
        ("method".into(), J::String(request.method().into())),
        ("target".into(), J::String(request.target().into())),
        ("headers".into(), J::Array(headers)),
        ("body".into(), J::String(body)),
    ];
    if let Some(principal) = principal {
        description.push(("principal".into(), J::String(principal.name().into())));
    }
    Ok(Ok(J::Object(description)))
}
