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
const CONTINUE_WIRE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";
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
    fn check_continue(&self) -> Result<(), HostError> {
        let response = self.limits.response;
        if response.head_bytes < CONTINUE_WIRE.len()
            || response.output_bytes < CONTINUE_WIRE.len()
            || response.line_bytes < CONTINUE_WIRE.len() - 4
            || response.work < CONTINUE_WIRE.len()
        {
            return Err(error("limit", "interim HTTP response limits exceeded"));
        }
        Ok(())
    }
    /// Explicit closure still releases the descriptor if shutdown reports an error.
    pub(super) fn close(self) -> Result<(), HostError> {
        self.stream.close()
    }
    pub(super) fn start_response<E>(
        self,
        status: u16,
        headers: &[HttpResponseHeader<'_>],
        body: &[u8],
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<PendingResponse, HostError>, E> {
        if let Err(e) =
            flatten(checked(self.started, self.milliseconds, &mut checkpoint).map(|_| Ok(())))?
        {
            return Ok(Err(e));
        }
        let wire =
            match render_http_response(status, self.context, headers, body, self.limits.response) {
                Ok(wire) => wire,
                Err(e) => return Ok(Err(error(e.code, e.message))),
            };
        if let Err(e) =
            flatten(checked(self.started, self.milliseconds, &mut checkpoint).map(|_| Ok(())))?
        {
            return Ok(Err(e));
        }
        Ok(Ok(PendingResponse {
            connection: self,
            wire,
            at: 0,
        }))
    }
    pub(super) fn respond<E>(
        self,
        status: u16,
        headers: &[HttpResponseHeader<'_>],
        body: &[u8],
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        let mut pending = match self.start_response(status, headers, body, &mut checkpoint)? {
            Ok(pending) => pending,
            Err(e) => return Ok(Err(e)),
        };
        match pending.connection.write(&pending.wire, &mut checkpoint)? {
            Err(e) => Ok(Err(e)),
            Ok(()) => Ok(pending.close().map(|_| J::Null)),
        }
    }
    fn rejection_wire(&self, code: &'static str) -> Result<Vec<u8>, HostError> {
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
        render_http_response(
            status,
            HttpResponseContext::Standard,
            headers,
            b"",
            self.limits.response,
        )
        .map_err(|e| error(e.code, e.message))
    }
    fn reject<E>(
        &mut self,
        code: &'static str,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        if let Ok(wire) = self.rejection_wire(code) {
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
/// A refusal can only publish its terminal error, never handler metadata.
pub(super) struct PendingRefusal {
    output: PendingResponse,
    failure: HostError,
}
impl std::fmt::Debug for PendingRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRefusal").finish_non_exhaustive()
    }
}
impl PendingRefusal {
    fn new(connection: HttpConnection, failure: HostError) -> Result<Self, HostError> {
        let wire = match connection.rejection_wire(failure.code) {
            Ok(wire) => wire,
            Err(_) => return Err(failure),
        };
        Ok(Self {
            output: PendingResponse {
                connection,
                wire,
                at: 0,
            },
            failure,
        })
    }
    pub(super) fn poll<E>(
        &mut self,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<bool, HostError>, E> {
        self.output.poll(checkpoint)
    }
    pub(super) fn close(self) -> Result<(), HostError> {
        self.output.close()
    }
    pub(super) fn finish(self) -> HostError {
        let _ = self.output.close();
        self.failure
    }
}
pub(super) fn queue_admission(
    stream: LoopbackStream,
    accepted: Instant,
    socket: NetworkLimits,
    limits: HttpExchangeLimits,
) -> Result<PendingRefusal, HostError> {
    PendingRefusal::new(
        HttpConnection::new(stream, accepted, socket, limits),
        error("admission", "HTTP request admission exhausted"),
    )
}
pub(super) enum Prepared {
    Body(Box<PendingBody>),
    Refusal(Box<PendingRefusal>),
    Verification(Box<PendingVerification>),
}
fn refuse<E>(
    mut connection: HttpConnection,
    failure: HostError,
    cooperative: bool,
    checkpoint: &mut impl FnMut() -> Result<(), E>,
) -> Result<Result<Prepared, HostError>, E> {
    if cooperative {
        return Ok(PendingRefusal::new(connection, failure)
            .map(|pending| Prepared::Refusal(Box::new(pending))));
    }
    connection.reject(failure.code, checkpoint)?;
    Ok(Err(failure))
}
/// Immutable rendered response and exact unsent suffix; Debug never reveals output.
pub(super) struct PendingResponse {
    connection: HttpConnection,
    wire: Vec<u8>,
    at: usize,
}
impl std::fmt::Debug for PendingResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingResponse").finish_non_exhaustive()
    }
}
impl PendingResponse {
    pub(super) fn close(self) -> Result<(), HostError> {
        self.connection.close()
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
        if self.at == self.wire.len() {
            return Ok(Ok(true));
        }
        if let Err(e) = connection.admit() {
            return Ok(Err(e));
        }
        let end = self
            .wire
            .len()
            .min(self.at.saturating_add(connection.bytes));
        let (started, milliseconds) = (connection.started, connection.milliseconds);
        match flatten(connection.stream.try_write(&self.wire[self.at..end], || {
            checked(started, milliseconds, checkpoint)
        }))? {
            Err(e) => return Ok(Err(e)),
            Ok(None) => {}
            Ok(Some(n)) => self.at += n,
        }
        Ok(Ok(self.at == self.wire.len()))
    }
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
    pub(super) fn ready(&self) -> bool {
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
pub(super) fn prepare_pending<E>(
    pending: PendingHead,
    cooperative: bool,
    access: Option<&super::HttpAccessPolicy>,
    mut verifier: Option<&mut super::http_authentication_v4::Verifier>,
    mut checkpoint: impl FnMut() -> Result<(), E>,
) -> Result<Result<Prepared, HostError>, E> {
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
            return refuse(
                connection,
                error("limit", "HTTP head did not complete within limit"),
                cooperative,
                &mut checkpoint,
            );
        }
        Err(e) => {
            return refuse(
                connection,
                error(e.code, e.message),
                cooperative,
                &mut checkpoint,
            );
        }
    };
    if let Some(access) = access {
        if let Err(e) = access.admit(&head.request, &mut head.work) {
            return refuse(connection, e, cooperative, &mut checkpoint);
        }
    }
    if cooperative {
        if let Some(verifier) = verifier.as_deref_mut() {
            let (started, milliseconds) = (connection.started, connection.milliseconds);
            if let Err(e) =
                flatten(checked(started, milliseconds, &mut checkpoint).map(|_| Ok(())))?
            {
                return Ok(Err(e));
            }
            let mut stopped = None;
            let mut poll = || {
                if stopped.is_some() {
                    return false;
                }
                match checked(started, milliseconds, &mut checkpoint) {
                    Ok(()) => true,
                    Err(stop) => {
                        stopped = Some(stop);
                        false
                    }
                }
            };
            let outcome = verifier.start(
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
            let job = match outcome {
                Ok(job) => job,
                Err(e) => return refuse(connection, e, true, &mut checkpoint),
            };
            if let Err(e) =
                flatten(checked(started, milliseconds, &mut checkpoint).map(|_| Ok(())))?
            {
                return Ok(Err(e));
            }
            connection.context = if head.request.method() == "HEAD" {
                HttpResponseContext::Head
            } else {
                HttpResponseContext::Standard
            };
            let metadata = match flatten(describe(&head.request, None, true, &mut || {
                checked(started, milliseconds, &mut checkpoint)
            }))? {
                Ok(metadata) => metadata,
                Err(e) => return Ok(Err(e)),
            };
            return Ok(Ok(Prepared::Verification(Box::new(PendingVerification {
                connection: Some(connection),
                metadata,
                framing: head.framing,
                work: head.work,
                continue_expected: head.continue_expected,
                initial: buffer[head.bytes..].to_vec(),
                job: Some(job),
            }))));
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
                return refuse(connection, e, cooperative, &mut checkpoint);
            }
        }
    } else {
        None
    };
    connection.context = if head.request.method() == "HEAD" {
        HttpResponseContext::Head
    } else {
        HttpResponseContext::Standard
    };
    let metadata = match flatten(describe(
        &head.request,
        principal.as_ref(),
        false,
        &mut || checked(connection.started, connection.milliseconds, &mut checkpoint),
    ))? {
        Ok(metadata) => metadata,
        Err(e) => return Ok(Err(e)),
    };
    let mut body = match BodyState::new(head.framing, head.work, connection.limits) {
        Ok(body) => body,
        Err(e) => {
            return refuse(connection, e, cooperative, &mut checkpoint);
        }
    };
    if let Err(e) = body.feed(&buffer[head.bytes..]) {
        return refuse(connection, e, cooperative, &mut checkpoint);
    }
    let interim = if head.continue_expected && !body.complete() {
        if let Err(e) = connection.check_continue() {
            return Ok(Err(e));
        }
        Some(0)
    } else {
        None
    };
    Ok(Ok(Prepared::Body(Box::new(PendingBody {
        connection,
        metadata,
        body,
        interim,
    }))))
}

/// Original checked metadata and one owned verification job; no credential Debug output.
pub(super) struct PendingVerification {
    connection: Option<HttpConnection>,
    metadata: J,
    framing: HttpBodyFraming,
    work: usize,
    continue_expected: bool,
    initial: Vec<u8>,
    job: Option<Box<dyn super::HttpVerificationJob>>,
}
impl std::fmt::Debug for PendingVerification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingVerification")
            .finish_non_exhaustive()
    }
}
impl Drop for PendingVerification {
    fn drop(&mut self) {
        if let Some(mut job) = self.job.take() {
            let _ = job.cancel();
        }
    }
}
impl PendingVerification {
    pub(super) fn close(mut self) -> Result<(), HostError> {
        let job = self.job.take().map_or(Ok(()), |mut job| job.cancel());
        let socket = self.connection.take().unwrap().close();
        job.and(socket)
    }
    pub(super) fn poll<E>(
        &mut self,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<Option<Prepared>, HostError>, E> {
        let connection = self.connection.as_ref().unwrap();
        let (started, milliseconds) = (connection.started, connection.milliseconds);
        if let Err(e) = flatten(checked(started, milliseconds, checkpoint).map(|_| Ok(())))? {
            return Ok(Err(e));
        }
        let mut stopped = None;
        let mut poll = || {
            if stopped.is_some() {
                return false;
            }
            match checked(started, milliseconds, checkpoint) {
                Ok(()) => true,
                Err(stop) => {
                    stopped = Some(stop);
                    false
                }
            }
        };
        let outcome = self
            .job
            .as_mut()
            .unwrap()
            .poll(&mut super::HttpBearerControl::new(&mut poll));
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
        if let Err(e) = flatten(checked(started, milliseconds, checkpoint).map(|_| Ok(())))? {
            return Ok(Err(e));
        }
        let Some(decision) = outcome else {
            return Ok(Ok(None));
        };
        self.job.take();
        let connection = self.connection.take().unwrap();
        let principal = match super::http_authentication_v4::decision_result(decision) {
            Ok(principal) => principal,
            Err(e) => {
                return refuse(connection, e, true, checkpoint).map(|result| result.map(Some))
            }
        };
        let mut metadata = std::mem::replace(&mut self.metadata, J::Null);
        if let J::Object(fields) = &mut metadata {
            fields.push(("principal".into(), J::String(principal.name().into())));
        }
        let mut body = match BodyState::new(self.framing, self.work, connection.limits) {
            Ok(body) => body,
            Err(e) => {
                return refuse(connection, e, true, checkpoint).map(|result| result.map(Some))
            }
        };
        if let Err(e) = body.feed(&self.initial) {
            return refuse(connection, e, true, checkpoint).map(|result| result.map(Some));
        }
        let interim = if self.continue_expected && !body.complete() {
            if let Err(e) = connection.check_continue() {
                return Ok(Err(e));
            }
            Some(0)
        } else {
            None
        };
        Ok(Ok(Some(Prepared::Body(Box::new(PendingBody {
            connection,
            metadata,
            body,
            interim,
        })))))
    }
}
enum BodyState {
    Fixed(super::HttpFixedBody),
    Chunked {
        decoder: HttpChunkedDecoder,
        seen: usize,
        maximum: usize,
        complete: bool,
    },
}
impl BodyState {
    fn new(
        framing: HttpBodyFraming,
        work: usize,
        limits: HttpExchangeLimits,
    ) -> Result<Self, HostError> {
        match framing {
            HttpBodyFraming::Fixed(length) => super::HttpFixedBody::new(length, limits.request)
                .map(Self::Fixed)
                .map_err(|e| error(e.code, e.message)),
            HttpBodyFraming::Chunked => {
                let chunked = HttpChunkedLimits {
                    body_bytes: limits.request.body_bytes,
                    line_bytes: limits.request.line_bytes,
                    work,
                    ..HttpChunkedLimits::DEFAULT
                };
                let trailers = HttpTrailerLimits {
                    bytes: limits.request.head_bytes,
                    line_bytes: limits.request.line_bytes,
                    fields: limits.request.headers,
                };
                let decoder = HttpChunkedDecoder::with_discarded_trailers(chunked, trailers)
                    .map_err(|e| error(e.code, e.message))?;
                Ok(Self::Chunked {
                    decoder,
                    seen: 0,
                    maximum: chunked.wire_bytes,
                    complete: false,
                })
            }
        }
    }
    fn remaining(&self) -> usize {
        match self {
            Self::Fixed(body) => body.remaining(),
            Self::Chunked { seen, maximum, .. } => maximum.saturating_sub(*seen),
        }
    }
    fn complete(&self) -> bool {
        match self {
            Self::Fixed(body) => body.remaining() == 0,
            Self::Chunked { complete, .. } => *complete,
        }
    }
    fn feed(&mut self, bytes: &[u8]) -> Result<(), HostError> {
        match self {
            Self::Fixed(body) => {
                body.feed(bytes).map_err(|e| error(e.code, e.message))?;
            }
            Self::Chunked {
                decoder,
                seen,
                maximum,
                complete,
            } => {
                if bytes.len() > maximum.saturating_sub(*seen) {
                    return Err(error("limit", "chunked wire limit exceeded"));
                }
                *complete = decoder.feed(bytes).map_err(|e| error(e.code, e.message))?;
                *seen += bytes.len();
            }
        }
        Ok(())
    }
    fn finish(self) -> Result<Vec<u8>, HostError> {
        match self {
            Self::Fixed(body) => body.finish(),
            Self::Chunked { decoder, .. } => decoder.finish(),
        }
        .map_err(|e| error(e.code, e.message))
    }
}
/// Checked metadata and verifier outcome are private until the body is complete.
pub(super) struct PendingBody {
    connection: HttpConnection,
    metadata: J,
    body: BodyState,
    interim: Option<usize>,
}
impl std::fmt::Debug for PendingBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingBody").finish_non_exhaustive()
    }
}
impl PendingBody {
    pub(super) fn close(self) -> Result<(), HostError> {
        self.connection.close()
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
        if let Some(at) = self.interim {
            if let Err(e) = connection.admit() {
                return Ok(Err(e));
            }
            let end = CONTINUE_WIRE.len().min(at.saturating_add(connection.bytes));
            let (started, milliseconds) = (connection.started, connection.milliseconds);
            match flatten(connection.stream.try_write(&CONTINUE_WIRE[at..end], || {
                checked(started, milliseconds, checkpoint)
            }))? {
                Err(e) => return Ok(Err(e)),
                Ok(None) => {}
                Ok(Some(n)) => {
                    let next = at + n;
                    self.interim = if next == CONTINUE_WIRE.len() {
                        None
                    } else {
                        Some(next)
                    };
                }
            }
            return Ok(Ok(false));
        }
        if self.body.complete() {
            return Ok(Ok(true));
        }
        if self.body.remaining() == 0 {
            return Ok(Err(error("limit", "chunked wire limit exceeded")));
        }
        if let Err(e) = connection.admit() {
            return Ok(Err(e));
        }
        let (started, milliseconds) = (connection.started, connection.milliseconds);
        let read = flatten(
            connection
                .stream
                .try_read(self.body.remaining().min(connection.bytes), || {
                    checked(started, milliseconds, checkpoint)
                }),
        )?;
        match read {
            Err(e) => return Ok(Err(e)),
            Ok(None) => {}
            Ok(Some(NetworkRead::Eof)) => {
                return Ok(Err(error("truncated", "HTTP body ended before completion")))
            }
            Ok(Some(NetworkRead::Data(bytes))) => {
                if let Err(e) = self.body.feed(&bytes) {
                    return Ok(Err(e));
                }
            }
        }
        Ok(Ok(self.body.complete()))
    }
}
pub(super) fn receive_body_pending<E>(
    mut pending: PendingBody,
    mut checkpoint: impl FnMut() -> Result<(), E>,
) -> Result<Result<(HttpConnection, J), HostError>, E> {
    if let Some(at) = pending.interim.take() {
        if let Err(e) = pending
            .connection
            .write(&CONTINUE_WIRE[at..], &mut checkpoint)?
        {
            return Ok(Err(e));
        }
    }
    while !pending.body.complete() {
        let maximum = pending.body.remaining();
        if maximum == 0 {
            pending.connection.reject("limit", &mut checkpoint)?;
            return Ok(Err(error("limit", "chunked wire limit exceeded")));
        }
        match pending.connection.read(maximum, &mut checkpoint)? {
            Err(e) => {
                pending.connection.reject(e.code, &mut checkpoint)?;
                return Ok(Err(e));
            }
            Ok(NetworkRead::Eof) => break,
            Ok(NetworkRead::Data(bytes)) => {
                if let Err(e) = pending.body.feed(&bytes) {
                    pending.connection.reject(e.code, &mut checkpoint)?;
                    return Ok(Err(e));
                }
            }
        }
    }
    let body = match pending.body.finish() {
        Ok(body) => body,
        Err(e) => {
            pending.connection.reject(e.code, &mut checkpoint)?;
            return Ok(Err(e));
        }
    };
    if let Err(e) = flatten(
        checked(
            pending.connection.started,
            pending.connection.milliseconds,
            &mut checkpoint,
        )
        .map(|_| Ok(())),
    )? {
        return Ok(Err(e));
    }
    let encoded = match encode(&body) {
        Ok(body) => body,
        Err(e) => return Ok(Err(e)),
    };
    if let Err(e) = flatten(
        checked(
            pending.connection.started,
            pending.connection.milliseconds,
            &mut checkpoint,
        )
        .map(|_| Ok(())),
    )? {
        return Ok(Err(e));
    }
    if let J::Object(fields) = &mut pending.metadata {
        fields.push(("body".into(), J::String(encoded)));
    }
    Ok(Ok((pending.connection, pending.metadata)))
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
    principal: Option<&super::HttpPrincipal>,
    redact: bool,
    checkpoint: &mut impl FnMut() -> Result<(), E>,
) -> Result<Result<J, HostError>, E> {
    checkpoint()?;
    let mut headers = Vec::new();
    for field in request.headers() {
        checkpoint()?;
        if (redact || principal.is_some()) && field.name().eq_ignore_ascii_case("authorization") {
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
    checkpoint()?;
    let mut description = vec![
        ("method".into(), J::String(request.method().into())),
        ("target".into(), J::String(request.target().into())),
        ("headers".into(), J::Array(headers)),
    ];
    if let Some(principal) = principal {
        description.push(("principal".into(), J::String(principal.name().into())));
    }
    Ok(Ok(J::Object(description)))
}
