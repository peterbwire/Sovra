//! Bounded explicit external verifier protocol; no credential database or cryptography is bundled.
use super::{
    parse_json, serialize_json, HostError, HostLimits, HostV3, HttpAuthenticationDecision as D,
    HttpBearerControl, HttpBearerRequest, HttpBearerTransport, HttpBearerVerifier, HttpPrincipal,
    JsonLimits, JsonValue as J,
};
use std::{
    collections::BTreeMap,
    fmt,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};
/// Captured native executable/cwd and fixed arguments, separate from source-visible v3 authority.
pub struct HttpCommandVerifier {
    host: HostV3,
    arguments: Vec<String>,
    live: Arc<AtomicUsize>,
    maximum_jobs: usize,
}
impl fmt::Debug for HttpCommandVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HttpCommandVerifier([opaque])")
    }
}
fn policy() -> HostError {
    HostError {
        code: "policy",
        message: "invalid HTTP verifier command configuration".into(),
    }
}
fn object(fields: Vec<(&str, J)>) -> J {
    J::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn field<'a>(value: &'a J, name: &str) -> Option<&'a J> {
    let J::Object(fields) = value else {
        return None;
    };
    fields.iter().find(|(key, _)| key == name).map(|(_, v)| v)
}
impl HttpCommandVerifier {
    /// Capture an exact native ELF executable and cwd, no inherited environment or shell.
    /// Fixed arguments: <=16, <=8192 total bytes. Stdio <=4096 bytes each; duration <=1000ms.
    pub fn new(
        executable: &Path,
        cwd: &Path,
        arguments: &[String],
        milliseconds: u64,
    ) -> Result<Self, HostError> {
        Self::with_job_limit(executable, cwd, arguments, milliseconds, 2)
    }
    /// Narrow simultaneous command jobs to 0..=2; zero disables launches.
    /// The same ceiling applies to synchronous and pending verification.
    pub fn with_job_limit(
        executable: &Path,
        cwd: &Path,
        arguments: &[String],
        milliseconds: u64,
        maximum_jobs: usize,
    ) -> Result<Self, HostError> {
        if maximum_jobs > 2 || milliseconds > 1000 || arguments.len() > 16 {
            return Err(policy());
        }
        let mut total = 0usize;
        for arg in arguments {
            total = total.checked_add(arg.len() + 1).ok_or_else(policy)?;
            if arg.contains('\0') || total > 8192 {
                return Err(policy());
            }
        }
        if !executable.is_absolute() || !cwd.is_absolute() {
            return Err(policy());
        }
        let mut host = HostV3::with_limits(HostLimits {
            bytes: 4096,
            arguments: 16,
            argument_bytes: 8192,
            milliseconds,
            ..HostLimits::DEFAULT
        })
        .map_err(|_| policy())?;
        host.grant_program("http-verifier", executable, cwd, BTreeMap::new())
            .map_err(|e| {
                if e.code == "unsupported" {
                    HostError {
                        code: "unsupported",
                        message:
                            "HTTP verifier command requires a supported native executable host"
                                .into(),
                    }
                } else {
                    policy()
                }
            })?;
        Ok(Self {
            host,
            arguments: arguments.to_vec(),
            live: Arc::new(AtomicUsize::new(0)),
            maximum_jobs,
        })
    }
    fn request(&self, request: HttpBearerRequest<'_>) -> Option<J> {
        let transport = match request.transport() {
            HttpBearerTransport::LoopbackPlaintext => "loopback-plaintext",
        };
        let input = object(vec![
            ("version", J::Int(1)),
            ("method", J::String(request.method().into())),
            ("target", J::String(request.target().into())),
            ("authority", J::String(request.authority().into())),
            (
                "origin",
                request
                    .origin()
                    .map(|s| J::String(s.into()))
                    .unwrap_or(J::Null),
            ),
            ("transport", J::String(transport.into())),
            (
                "credential",
                J::String(std::str::from_utf8(request.credential()).ok()?.into()),
            ),
        ]);
        let limits = JsonLimits {
            bytes: 4096,
            values: 64,
            ..JsonLimits::DEFAULT
        };
        let input = serialize_json(&input, limits).ok()?;
        let encoded = super::http_exchange_v4::encode(input.as_bytes()).ok()?;
        let call = object(vec![
            ("version", J::Int(3)),
            ("op", J::String("process.run".into())),
            ("program", J::String("http-verifier".into())),
            (
                "arguments",
                J::Array(self.arguments.iter().cloned().map(J::String).collect()),
            ),
            ("input", J::String(encoded)),
        ]);
        Some(call)
    }
    fn reply(output: &J) -> Option<D> {
        let limits = JsonLimits {
            bytes: 4096,
            values: 64,
            ..JsonLimits::DEFAULT
        };
        if field(output, "exit_code") != Some(&J::Int(0)) {
            return None;
        }
        let stdout = super::http_exchange_v4::decode(field(output, "stdout")?, 4096).ok()?;
        let result = parse_json(&stdout, limits).ok()?;
        let J::Object(fields) = &result else {
            return None;
        };
        if field(&result, "version") != Some(&J::Int(1)) {
            return None;
        }
        let J::String(decision) = field(&result, "decision")? else {
            return None;
        };
        match decision.as_str() {
            "grant" if fields.len() == 3 => {
                let J::String(name) = field(&result, "principal")? else {
                    return None;
                };
                Some(D::Granted(HttpPrincipal::new(name).ok()?))
            }
            "unauthenticated" if fields.len() == 2 => Some(D::Unauthenticated),
            "forbidden" if fields.len() == 2 => Some(D::Forbidden),
            "unavailable" if fields.len() == 2 => Some(D::Unavailable),
            _ => None,
        }
    }
    fn run(&mut self, request: HttpBearerRequest<'_>, mut poll: impl FnMut() -> bool) -> D {
        #[cfg(all(
            target_os = "linux",
            any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
        ))]
        let _slot = match JobSlot::reserve(&self.live, self.maximum_jobs) {
            Some(slot) => slot,
            None => return D::Unavailable,
        };
        let outcome = (|| {
            let call = self.request(request)?;
            let output = self
                .host
                .call(&call, || if poll() { Ok(()) } else { Err(()) })
                .ok()?
                .ok()?;
            Self::reply(&output)
        })();
        outcome.unwrap_or(D::Unavailable)
    }
}
impl HttpBearerVerifier for HttpCommandVerifier {
    fn start_verification(
        &mut self,
        request: HttpBearerRequest<'_>,
        control: &mut HttpBearerControl<'_>,
    ) -> Box<dyn super::HttpVerificationJob> {
        if !control.check() {
            return Box::new(UnavailableJob);
        }
        #[cfg(all(
            target_os = "linux",
            any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
        ))]
        {
            let job = (|| {
                let slot = JobSlot::reserve(&self.live, self.maximum_jobs)?;
                let call = self.request(request)?;
                let task = self.host.start_process(&call).ok()?;
                Some(CommandJob {
                    task: Some(task),
                    slot: Some(slot),
                })
            })();
            if let Some(job) = job {
                return Box::new(job);
            }
        }
        #[cfg(not(all(
            target_os = "linux",
            any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
        )))]
        let _ = request;
        let _ = (&self.live, self.maximum_jobs);
        Box::new(UnavailableJob)
    }
    fn verify(&mut self, request: HttpBearerRequest<'_>) -> D {
        self.run(request, || true)
    }
    fn verify_controlled(
        &mut self,
        request: HttpBearerRequest<'_>,
        control: &mut HttpBearerControl<'_>,
    ) -> D {
        self.run(request, || control.check())
    }
}

struct UnavailableJob;
impl super::HttpVerificationJob for UnavailableJob {
    fn poll(&mut self, _: &mut HttpBearerControl<'_>) -> Option<D> {
        Some(D::Unavailable)
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
struct JobSlot(Arc<AtomicUsize>);
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl JobSlot {
    fn reserve(live: &Arc<AtomicUsize>, maximum: usize) -> Option<Self> {
        let mut current = live.load(std::sync::atomic::Ordering::Acquire);
        for _ in 0..128 {
            if current >= maximum {
                return None;
            }
            match live.compare_exchange(
                current,
                current + 1,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self(live.clone())),
                Err(next) => current = next,
            }
        }
        None
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl Drop for JobSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
struct CommandJob {
    task: Option<super::host_v3::OwnedProcessExchange>,
    slot: Option<JobSlot>,
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl super::HttpVerificationJob for CommandJob {
    fn cancel(&mut self) -> Result<(), HostError> {
        let result = self.task.take().map_or(Ok(()), |mut task| task.cleanup());
        self.slot.take();
        result
    }
    fn poll(&mut self, control: &mut HttpBearerControl<'_>) -> Option<D> {
        let Some(task) = self.task.as_mut() else {
            return Some(D::Unavailable);
        };
        let outcome = task.poll(&mut || if control.check() { Ok(()) } else { Err(()) });
        if matches!(outcome, Ok(Ok(None))) {
            return None;
        }
        let result = match outcome {
            Ok(Ok(Some(value))) => HttpCommandVerifier::reply(&value).unwrap_or(D::Unavailable),
            _ => D::Unavailable,
        };
        let cleanup = self.task.take().unwrap().cleanup();
        self.slot.take();
        Some(if cleanup.is_ok() {
            result
        } else {
            D::Unavailable
        })
    }
}
