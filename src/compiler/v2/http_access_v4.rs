//! Explicit exact Host/Origin admission for the HTTP socket host, not client authentication.
use super::{HostError, HttpRequest};
fn error(code: &'static str, message: &'static str) -> HostError {
    HostError {
        code,
        message: message.into(),
    }
}
/// Immutable, trusted HTTP request-head admission. Empty lists deny their respective values.
/// This policy never authorizes raw socket operations or authenticates a client.
#[derive(Debug, Clone)]
pub struct HttpAccessPolicy {
    hosts: Vec<String>,
    origins: Vec<String>,
    allow_missing_origin: bool,
}
impl HttpAccessPolicy {
    /// Validate at most 16 exact authorities/origins of at most 256 bytes each.
    /// Hosts compare ASCII-insensitively; serialized http(s) origins compare exactly.
    pub fn new(
        hosts: &[&str],
        origins: &[&str],
        allow_missing_origin: bool,
    ) -> Result<Self, HostError> {
        if hosts.len() > 16 || origins.len() > 16 {
            return Err(error("policy", "HTTP access lists exceed 16 entries"));
        }
        for (i, host) in hosts.iter().enumerate() {
            if host.len() > 256
                || !super::http_request_v4::authority(host.as_bytes())
                || hosts[..i].iter().any(|h| h.eq_ignore_ascii_case(host))
            {
                return Err(error("policy", "invalid or duplicate HTTP Host grant"));
            }
        }
        for (i, origin) in origins.iter().enumerate() {
            if !valid_origin(origin.as_bytes()) || origins[..i].contains(origin) {
                return Err(error("policy", "invalid or duplicate HTTP Origin grant"));
            }
        }
        Ok(Self {
            hosts: hosts.iter().map(|s| (*s).into()).collect(),
            origins: origins.iter().map(|s| (*s).into()).collect(),
            allow_missing_origin,
        })
    }
    pub(super) fn admit(
        &self,
        request: &HttpRequest<'_>,
        work: &mut usize,
    ) -> Result<(), HostError> {
        // Fixed conservative admission for bounded field scans and exact-grant comparisons.
        let cost =
            request.headers().len() * 16 + (self.hosts.len() + self.origins.len()) * 256 + 512;
        *work = work
            .checked_sub(cost)
            .ok_or_else(|| error("limit", "HTTP access work exhausted"))?;
        let host = request
            .headers()
            .iter()
            .find(|h| h.name().eq_ignore_ascii_case("host"));
        if !host.is_some_and(|h| {
            self.hosts
                .iter()
                .any(|grant| grant.as_bytes().eq_ignore_ascii_case(h.value()))
        }) {
            return Err(error("authority", "HTTP Host is not granted"));
        }
        let mut origins = request
            .headers()
            .iter()
            .filter(|h| h.name().eq_ignore_ascii_case("origin"));
        let origin = origins.next();
        if origins.next().is_some() {
            return Err(error("authority", "multiple HTTP Origin fields refuse"));
        }
        match origin {
            None if self.allow_missing_origin => Ok(()),
            Some(origin)
                if valid_origin(origin.value())
                    && self
                        .origins
                        .iter()
                        .any(|grant| grant.as_bytes() == origin.value()) =>
            {
                Ok(())
            }
            _ => Err(error("authority", "HTTP Origin is not granted")),
        }
    }
}
fn valid_origin(value: &[u8]) -> bool {
    if value.len() > 256 {
        return false;
    }
    let authority = value
        .strip_prefix(b"http://")
        .or_else(|| value.strip_prefix(b"https://"));
    authority.is_some_and(super::http_request_v4::authority)
}
