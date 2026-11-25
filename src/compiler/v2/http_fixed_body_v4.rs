//! Bounded resumable fixed-length body ownership; no socket or authentication effects.
use super::{HttpRequestError, HttpRequestLimits};
/// Retains a fixed-length body across fragments without publishing partial payloads.
pub struct HttpFixedBody {
    bytes: Vec<u8>,
    length: usize,
    failure: Option<HttpRequestError>,
}
impl std::fmt::Debug for HttpFixedBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpFixedBody")
            .field("received", &self.bytes.len())
            .field("length", &self.length)
            .field("failed", &self.failure.is_some())
            .finish()
    }
}
fn error(code: &'static str, offset: usize, message: &'static str) -> HttpRequestError {
    HttpRequestError {
        code,
        offset,
        message,
    }
}
impl HttpFixedBody {
    /// Admit the declared length and allocate only within lowerable parser limits.
    pub fn new(length: usize, limits: HttpRequestLimits) -> Result<Self, HttpRequestError> {
        limits.validate()?;
        if length > limits.body_bytes {
            return Err(error("limit", 0, "HTTP body length exceeds limit"));
        }
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(length).is_err() {
            return Err(error("limit", 0, "HTTP body allocation refused"));
        }
        Ok(Self {
            bytes,
            length,
            failure: None,
        })
    }
    /// Remaining body bytes; callers must bound any read by this amount.
    pub fn remaining(&self) -> usize {
        self.length - self.bytes.len()
    }
    /// Feed a fragment. Excess bytes refuse atomically with sticky framing failure.
    pub fn feed(&mut self, fragment: &[u8]) -> Result<bool, HttpRequestError> {
        if let Some(e) = &self.failure {
            return Err(e.clone());
        }
        if fragment.len() > self.remaining() {
            let e = error(
                "framing",
                self.bytes.len(),
                "trailing or pipelined request bytes refuse",
            );
            self.failure = Some(e.clone());
            return Err(e);
        }
        self.bytes.extend_from_slice(fragment);
        Ok(self.remaining() == 0)
    }
    /// Publish only a complete body, preserving any earlier refusal.
    pub fn finish(self) -> Result<Vec<u8>, HttpRequestError> {
        if let Some(e) = self.failure {
            return Err(e);
        }
        if self.remaining() != 0 {
            return Err(error(
                "truncated",
                self.bytes.len(),
                "HTTP body ended before completion",
            ));
        }
        Ok(self.bytes)
    }
}
