//! Trusted Bearer-verifier embedding boundary; no built-in credential backend or cryptography.
use super::{HostError, HttpRequest};
use std::fmt;
fn error(code: &'static str, message: &'static str) -> HostError {
    HostError {
        code,
        message: message.into(),
    }
}
/// Bounded provider-issued identity. Wire headers cannot construct this request result field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpPrincipal(String);
impl HttpPrincipal {
    /// Admit 1..64 ASCII letters, digits, dot, underscore or hyphen; no secret data belongs here.
    pub fn new(name: &str) -> Result<Self, HostError> {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(error("policy", "invalid HTTP principal name"));
        }
        Ok(Self(name.into()))
    }
    /// Provider-issued public identity label.
    pub fn name(&self) -> &str {
        &self.0
    }
}
/// Actual accepted socket transport, independent of peer-supplied forwarding headers.
/// Additional secure transports require separately implemented and qualified host support.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpBearerTransport {
    /// IPv4 loopback TCP without TLS; this is not a confidential/authenticated channel.
    LoopbackPlaintext,
}
/// Borrowed checked head context. Debug output deliberately omits credentials and URI data.
pub struct HttpBearerRequest<'a> {
    method: &'a str,
    target: &'a str,
    credential: &'a [u8],
    authority: &'a str,
    origin: Option<&'a str>,
}
impl fmt::Debug for HttpBearerRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HttpBearerRequest([redacted])")
    }
}
impl HttpBearerRequest<'_> {
    /// Original checked method token; case-sensitive.
    pub fn method(&self) -> &str {
        self.method
    }
    /// Original checked target; no decoding/normalization.
    pub fn target(&self) -> &str {
        self.target
    }
    /// Original validated Host authority after configured access admission; no normalization.
    pub fn authority(&self) -> &str {
        self.authority
    }
    /// Original admitted Origin, or None for explicitly permitted absence. Never inferred.
    pub fn origin(&self) -> Option<&str> {
        self.origin
    }
    /// Actual host transport. Forwarded/X-Forwarded-Proto headers cannot promote its security.
    pub fn transport(&self) -> HttpBearerTransport {
        HttpBearerTransport::LoopbackPlaintext
    }
    /// Borrowed Bearer credential bytes, at most 256; providers must not log or publish them.
    pub fn credential(&self) -> &[u8] {
        self.credential
    }
}
/// Trusted provider outcome; error responses never include provider or peer text.
#[derive(Debug)]
pub enum HttpAuthenticationDecision {
    /// Verified credential and authorized method/target, with a bounded public principal.
    Granted(HttpPrincipal),
    /// Credential is absent/invalid/unrecognized: 401 with a static Bearer challenge.
    Unauthenticated,
    /// Verified caller lacks request permission: 403.
    Forbidden,
    /// Provider cannot establish authority (e.g. backend unavailable): 503.
    Unavailable,
}
/// Trusted synchronous embedding interface, not an untrusted plugin or production backend.
/// Implementations verify credentials and request permissions using their reviewed backend.
/// They must return promptly; host controls check cancellation/deadline before and after calls.
pub trait HttpBearerVerifier: Send + Sync {
    /// Authenticate the checked credential and authorize this exact request head.
    fn verify(&mut self, request: HttpBearerRequest<'_>) -> HttpAuthenticationDecision;
}
pub(super) struct Verifier(pub Box<dyn HttpBearerVerifier>);
impl fmt::Debug for Verifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HttpBearerVerifier([opaque])")
    }
}
impl Verifier {
    pub(super) fn authenticate(
        &mut self,
        request: &HttpRequest<'_>,
        work: &mut usize,
    ) -> Result<HttpPrincipal, HostError> {
        let mut fields = request
            .headers()
            .iter()
            .filter(|h| h.name().eq_ignore_ascii_case("authorization"));
        let field = fields.next();
        let authority = request
            .headers()
            .iter()
            .find(|h| h.name().eq_ignore_ascii_case("host"))
            .map(|h| h.value())
            .ok_or_else(|| {
                error(
                    "authentication_unavailable",
                    "HTTP authority context unavailable",
                )
            })?;
        let origin = request
            .headers()
            .iter()
            .find(|h| h.name().eq_ignore_ascii_case("origin"))
            .map(|h| h.value());
        let cost = request.headers().len() * 48
            + authority.len()
            + origin.map_or(0, |value| value.len())
            + field.map_or(0, |h| h.value().len() * 2)
            + request.method().len()
            + request.target().len()
            + 512;
        *work = work
            .checked_sub(cost)
            .ok_or_else(|| error("limit", "HTTP authentication work exhausted"))?;
        if fields.next().is_some() {
            return Err(error("authentication", "ambiguous HTTP credentials"));
        }
        let value = field
            .map(|h| h.value())
            .ok_or_else(|| error("authentication", "HTTP Bearer credential required"))?;
        if value.len() < 8 || !value[..6].eq_ignore_ascii_case(b"Bearer") || value[6] != b' ' {
            return Err(error("authentication", "HTTP Bearer credential required"));
        }
        let start = value[7..]
            .iter()
            .position(|b| *b != b' ')
            .map(|n| n + 7)
            .unwrap_or(value.len());
        let credential = &value[start..];
        let base = credential
            .iter()
            .position(|b| *b == b'=')
            .unwrap_or(credential.len());
        if credential.is_empty()
            || credential.len() > 256
            || base == 0
            || !credential[..base]
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/".contains(b))
            || !credential[base..].iter().all(|b| *b == b'=')
        {
            return Err(error(
                "authentication",
                "invalid HTTP Bearer credential syntax",
            ));
        }
        // Access admission already checked Host and any present Origin. Preserve exact wire forms.
        let authority = std::str::from_utf8(authority).map_err(|_| {
            error(
                "authentication_unavailable",
                "invalid HTTP authority context",
            )
        })?;
        let origin = origin
            .map(std::str::from_utf8)
            .transpose()
            .map_err(|_| error("authentication_unavailable", "invalid HTTP origin context"))?;
        match self.0.verify(HttpBearerRequest {
            method: request.method(),
            target: request.target(),
            credential,
            authority,
            origin,
        }) {
            HttpAuthenticationDecision::Granted(principal) => Ok(principal),
            HttpAuthenticationDecision::Unauthenticated => {
                Err(error("authentication", "HTTP credential not verified"))
            }
            HttpAuthenticationDecision::Forbidden => {
                Err(error("authority", "HTTP request not authorized"))
            }
            HttpAuthenticationDecision::Unavailable => Err(error(
                "authentication_unavailable",
                "HTTP verifier unavailable",
            )),
        }
    }
}
