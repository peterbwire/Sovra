//! Execution-scoped JSON requests for trusted loopback socket owners.
use super::{
    HostError, JsonValue as J, LoopbackListener, LoopbackNetwork, LoopbackStream, NetworkLimits,
    NetworkRead, RuntimeBytesLimits, RuntimeBytesV2, RuntimeDataWork, RuntimeValueFuel,
};
use crate::compiler::execution::MaterializationLimits;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};
fn error(code: &'static str, message: &'static str) -> HostError {
    HostError {
        code,
        message: message.into(),
    }
}
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
fn token() -> Result<i64, HostError> {
    let mut current = NEXT_HANDLE.load(Ordering::Acquire);
    for _ in 0..128 {
        if current >= i64::MAX as u64 {
            return Err(error("limit", "network handle space exhausted"));
        }
        match NEXT_HANDLE.compare_exchange(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return Ok(current as i64),
            Err(next) => current = next,
        }
    }
    Err(error(
        "limit",
        "network handle admission contention exceeded",
    ))
}
#[derive(Debug)]
enum Owner {
    Listener(LoopbackListener),
    Stream {
        stream: LoopbackStream,
        accepted: std::time::Instant,
    },
    Http(super::http_exchange_v4::HttpConnection),
    PendingHead(Box<super::http_exchange_v4::PendingHead>),
}
/// Mutable trusted execution scope. Tokens alias one owner and never duplicate sockets.
/// Closing invalidates all token aliases; tokens are never reused in this process.
#[derive(Debug)]
pub struct NetworkHostV4 {
    network: LoopbackNetwork,
    owners: BTreeMap<i64, Owner>,
    limits: NetworkLimits,
    http: super::HttpExchangeLimits,
    http_access: Option<Box<super::HttpAccessPolicy>>,
    http_verifier: Option<super::http_authentication_v4::Verifier>,
    http_admission: Option<Box<super::http_admission_v4::HttpAdmission>>,
}
impl NetworkHostV4 {
    /// Start a denied host with only lowerable socket limits.
    pub fn new(limits: NetworkLimits) -> Result<Self, HostError> {
        Ok(Self {
            network: LoopbackNetwork::new(limits)?,
            owners: BTreeMap::new(),
            limits,
            http: super::HttpExchangeLimits::DEFAULT,
            http_access: None,
            http_verifier: None,
            http_admission: None,
        })
    }
    /// Narrow aggregate HTTP policy before any socket owners are issued.
    pub fn set_http_limits(&mut self, limits: super::HttpExchangeLimits) -> Result<(), HostError> {
        limits.validate()?;
        if !self.owners.is_empty() {
            return Err(error(
                "policy",
                "HTTP limits must be configured before socket creation",
            ));
        }
        self.http = limits;
        Ok(())
    }
    /// Configure trusted HTTP head admission before creating socket owners.
    pub fn set_http_access(&mut self, policy: super::HttpAccessPolicy) -> Result<(), HostError> {
        if !self.owners.is_empty() {
            return Err(error(
                "policy",
                "HTTP access must be configured before socket creation",
            ));
        }
        self.http_access = Some(Box::new(policy));
        Ok(())
    }
    /// Install a trusted Bearer verifier before creating socket owners, after HTTP access policy.
    /// No external JSON policy or Sovra source request can register a verifier.
    pub fn set_http_bearer_verifier(
        &mut self,
        verifier: Box<dyn super::HttpBearerVerifier>,
    ) -> Result<(), HostError> {
        if !self.owners.is_empty() || self.http_access.is_none() {
            return Err(error(
                "policy",
                "HTTP verifier requires access policy and no live socket owners",
            ));
        }
        self.http_verifier = Some(super::http_authentication_v4::Verifier(verifier));
        Ok(())
    }
    /// Set optional aggregate HTTP attempt admission before any socket owners are issued.
    pub fn set_http_admission(
        &mut self,
        policy: super::HttpAdmissionPolicy,
    ) -> Result<(), HostError> {
        if !self.owners.is_empty() {
            return Err(error(
                "policy",
                "HTTP admission must be configured before socket creation",
            ));
        }
        self.http_admission = Some(Box::new(super::http_admission_v4::HttpAdmission::new(
            policy,
        )?));
        Ok(())
    }
    /// Explicit trusted exact-port authority; request data cannot create grants.
    pub fn grant_port(&mut self, port: u16) -> Result<(), HostError> {
        self.network.grant_port(port)
    }
    /// Current listener and stream reservations in this execution scope.
    pub fn usage(&self) -> (usize, usize) {
        self.network.usage()
    }
    /// Validate and execute a version-four network request, preserving callback errors.
    pub fn call<E>(
        &mut self,
        request: &J,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        checkpoint()?;
        let fields = match request {
            J::Object(fields) if fields.len() <= 6 => fields,
            _ => return Ok(Err(error("input", "bounded request object required"))),
        };
        let get = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, v)| v)
                .ok_or_else(|| error("input", "missing request field"))
        };
        let setup = (|| {
            if !matches!(get("version")?, J::Int(4)) {
                return Err(error("input", "network request version must be 4"));
            }
            let op = match get("op")? {
                J::String(op) if op.len() <= 32 => op.as_str(),
                _ => return Err(error("input", "network operation required")),
            };
            let allowed: &[&str] = match op {
                "net.listen" => &["version", "op", "port"],
                "net.port" | "net.accept" | "net.try_accept" | "net.close" | "http.receive"
                | "http.poll_head" => &["version", "op", "handle"],
                "net.read" | "net.try_read" => &["version", "op", "handle", "maximum"],
                "net.write" | "net.try_write" => &["version", "op", "handle", "data"],
                "http.respond" => &["version", "op", "handle", "status", "headers", "body"],
                _ => {
                    return Err(error(
                        "unsupported",
                        "unknown version-four network operation",
                    ))
                }
            };
            if fields.len() != allowed.len() {
                return Err(error("input", "request field count mismatch"));
            }
            for (index, (key, _)) in fields.iter().enumerate() {
                if !allowed.contains(&key.as_str())
                    || fields[..index].iter().any(|(other, _)| other == key)
                {
                    return Err(error("input", "unknown or duplicate request field"));
                }
            }
            Ok(op)
        })();
        let op = match setup {
            Ok(op) => op,
            Err(e) => return Ok(Err(e)),
        };
        if op == "net.listen" {
            let port = match get("port").and_then(|v| match v {
                J::Int(n) => u16::try_from(*n).map_err(|_| error("input", "invalid port")),
                _ => Err(error("input", "port must be Int")),
            }) {
                Ok(port) => port,
                Err(e) => return Ok(Err(e)),
            };
            let handle = match token() {
                Ok(handle) => handle,
                Err(e) => return Ok(Err(e)),
            };
            return Ok(self.network.listen(port, &mut checkpoint)?.map(|owner| {
                self.owners.insert(handle, Owner::Listener(owner));
                J::Int(handle)
            }));
        }
        let handle = match get("handle") {
            Ok(J::Int(handle)) if *handle > 0 => *handle,
            _ => return Ok(Err(error("input", "positive handle Int required"))),
        };
        if op == "net.close" {
            return Ok(match self.owners.remove(&handle) {
                Some(Owner::Listener(owner)) => {
                    owner.close();
                    Ok(J::Null)
                }
                Some(Owner::Stream { stream, .. }) => stream.close().map(|_| J::Null),
                Some(Owner::Http(owner)) => owner.close().map(|_| J::Null),
                Some(Owner::PendingHead(owner)) => owner.close().map(|_| J::Null),
                None => Err(error("input", "stale or foreign network handle")),
            });
        }
        if op == "http.receive" || op == "http.poll_head" {
            if !matches!(
                self.owners.get(&handle),
                Some(Owner::Stream { .. }) | Some(Owner::PendingHead(_))
            ) {
                return Ok(Err(error(
                    "input",
                    "HTTP receive requires a raw or pending-head owner",
                )));
            }
            let mut pending = match self.owners.remove(&handle) {
                Some(Owner::PendingHead(pending)) => *pending,
                Some(Owner::Stream { stream, accepted }) => {
                    if !self
                        .http_admission
                        .as_mut()
                        .map_or(true, |limiter| limiter.admit())
                    {
                        return super::http_exchange_v4::reject_admission(
                            stream,
                            accepted,
                            self.limits,
                            self.http,
                            checkpoint,
                        );
                    }
                    match super::http_exchange_v4::PendingHead::new(
                        stream,
                        accepted,
                        self.limits,
                        self.http,
                    ) {
                        Ok(pending) => pending,
                        Err(e) => return Ok(Err(e)),
                    }
                }
                _ => unreachable!(),
            };
            if op == "http.poll_head" {
                return Ok(pending.poll(&mut checkpoint)?.map(|ready| {
                    self.owners
                        .insert(handle, Owner::PendingHead(Box::new(pending)));
                    J::Bool(ready)
                }));
            }
            return Ok(super::http_exchange_v4::receive_pending(
                pending,
                self.http_access.as_deref(),
                self.http_verifier.as_mut(),
                checkpoint,
            )?
            .map(|(owner, request)| {
                self.owners.insert(handle, Owner::Http(owner));
                request
            }));
        }
        if op == "http.respond" {
            if !matches!(self.owners.get(&handle), Some(Owner::Http(_))) {
                return Ok(Err(error(
                    "input",
                    "HTTP response requires one parsed request",
                )));
            }
            let setup = (|| {
                let status = match get("status")? {
                    J::Int(n) => {
                        u16::try_from(*n).map_err(|_| error("input", "HTTP status must fit u16"))?
                    }
                    _ => return Err(error("input", "HTTP status must be Int")),
                };
                let items = match get("headers")? {
                    J::Array(items) if items.len() <= self.http.response.headers => items,
                    _ => return Err(error("input", "bounded HTTP header array required")),
                };
                let mut headers = Vec::new();
                for item in items {
                    let pair = match item {
                        J::Array(pair) if pair.len() == 2 => pair,
                        _ => {
                            return Err(error(
                                "input",
                                "HTTP headers must be name/value String pairs",
                            ))
                        }
                    };
                    let (J::String(name), J::String(value)) = (&pair[0], &pair[1]) else {
                        return Err(error("input", "HTTP header Strings required"));
                    };
                    headers.push(super::HttpResponseHeader {
                        name,
                        value: value.as_bytes(),
                    });
                }
                let body =
                    super::http_exchange_v4::decode(get("body")?, self.http.response.body_bytes)?;
                Ok((status, headers, body))
            })();
            let Some(Owner::Http(owner)) = self.owners.remove(&handle) else {
                unreachable!()
            };
            return match setup {
                Ok((status, headers, body)) => owner.respond(status, &headers, &body, checkpoint),
                Err(e) => Ok(Err(e)),
            };
        }
        let owner = match self.owners.get_mut(&handle) {
            Some(owner) => owner,
            None => return Ok(Err(error("input", "stale or foreign network handle"))),
        };
        match (op, owner) {
            ("net.port", Owner::Listener(owner)) => {
                Ok(owner.port().map(|port| J::Int(port.into())))
            }
            ("net.try_accept", Owner::Listener(owner)) => {
                let accepted = owner.try_accept(&mut checkpoint)?;
                Ok(accepted.and_then(|stream| {
                    let Some(stream) = stream else {
                        return Ok(J::Null);
                    };
                    let handle = token()?;
                    self.owners.insert(
                        handle,
                        Owner::Stream {
                            stream,
                            accepted: std::time::Instant::now(),
                        },
                    );
                    Ok(J::Int(handle))
                }))
            }
            ("net.accept", Owner::Listener(owner)) => {
                let handle = match token() {
                    Ok(handle) => handle,
                    Err(e) => return Ok(Err(e)),
                };
                let accepted = owner.accept(&mut checkpoint)?;
                Ok(accepted.map(|owner| {
                    self.owners.insert(
                        handle,
                        Owner::Stream {
                            stream: owner,
                            accepted: std::time::Instant::now(),
                        },
                    );
                    J::Int(handle)
                }))
            }
            ("net.read" | "net.try_read", Owner::Stream { stream: owner, .. }) => {
                let maximum = match get("maximum") {
                    Ok(J::Int(n)) => usize::try_from(*n).ok(),
                    _ => None,
                };
                let Some(maximum) = maximum else {
                    return Ok(Err(error("input", "nonnegative read maximum required")));
                };
                if op == "net.try_read" {
                    return Ok(owner.try_read(maximum, checkpoint)?.and_then(|value| {
                        let ready = value.is_some();
                        let data = match value {
                            Some(NetworkRead::Data(bytes)) => encode(&bytes)?,
                            Some(NetworkRead::Eof) | None => J::Null,
                        };
                        Ok(J::Object(vec![
                            ("ready".into(), J::Bool(ready)),
                            ("data".into(), data),
                        ]))
                    }));
                }
                Ok(owner
                    .read(maximum, checkpoint)?
                    .and_then(|value| match value {
                        NetworkRead::Eof => Ok(J::Null),
                        NetworkRead::Data(bytes) => encode(&bytes),
                    }))
            }
            ("net.write" | "net.try_write", Owner::Stream { stream: owner, .. }) => {
                let bytes = match get("data").and_then(|value| decode(value, self.limits.bytes)) {
                    Ok(bytes) => bytes,
                    Err(e) => return Ok(Err(e)),
                };
                if op == "net.try_write" {
                    return Ok(owner.try_write(&bytes, checkpoint)?.map(|count| {
                        J::Object(vec![
                            ("ready".into(), J::Bool(count.is_some())),
                            (
                                "written".into(),
                                count.map_or(J::Null, |n| J::Int(n as i64)),
                            ),
                        ])
                    }));
                }
                Ok(owner.write(&bytes, checkpoint)?.map(|n| J::Int(n as i64)))
            }
            _ => Ok(Err(error(
                "input",
                "network handle kind does not match operation",
            ))),
        }
    }
    /// Close every owner, even after an error. First shutdown failure is explicit.
    pub fn close_all(&mut self) -> Result<(), HostError> {
        let mut first = None;
        for (_, owner) in std::mem::take(&mut self.owners) {
            let closed = match owner {
                Owner::Stream { stream, .. } => stream.close(),
                Owner::Http(owner) => owner.close(),
                Owner::PendingHead(owner) => owner.close(),
                Owner::Listener(_) => Ok(()),
            };
            if let Err(mut e) = closed {
                e.code = "cleanup";
                if first.is_none() {
                    first = Some(e);
                }
            }
        }
        if let Some(error) = first {
            Err(error)
        } else {
            Ok(())
        }
    }
}
fn fuel() -> RuntimeValueFuel {
    RuntimeValueFuel::new(MaterializationLimits {
        max_value_nodes: Some(131076),
        max_text_bytes: Some(262144),
    })
}
fn encode(bytes: &[u8]) -> Result<J, HostError> {
    let mut fuel = fuel();
    let mut work = RuntimeDataWork::new(4_000_000);
    let bytes =
        RuntimeBytesV2::from_slice(bytes, RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
            .map_err(|_| error("limit", "network binary output refused"))?;
    bytes
        .to_base64(RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
        .map(J::String)
        .map_err(|_| error("limit", "network Base64 output refused"))
}
fn decode(value: &J, maximum: usize) -> Result<Vec<u8>, HostError> {
    let text = match value {
        J::String(text) => text,
        _ => return Err(error("input", "Base64 String required")),
    };
    if text.len() > maximum.saturating_add(2) / 3 * 4 {
        return Err(error("limit", "network Base64 input limit exceeded"));
    }
    let mut fuel = fuel();
    let mut work = RuntimeDataWork::new(4_000_000);
    let limits = RuntimeBytesLimits {
        max_bytes: maximum,
        ..RuntimeBytesLimits::DEFAULT
    };
    RuntimeBytesV2::from_base64(text, limits, &mut fuel, &mut work)
        .map(|v| v.as_slice().to_vec())
        .map_err(|_| error("input", "strict bounded Base64 required"))
}
