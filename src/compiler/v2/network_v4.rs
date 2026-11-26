//! Trusted embedding foundation for bounded owned IPv4 loopback sockets.
//! No VM, source or CLI authority is installed by this module.
use super::HostError;
use std::{
    collections::BTreeSet,
    io::{self, Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
fn error(code: &'static str, message: &'static str) -> HostError {
    HostError {
        code,
        message: message.into(),
    }
}
fn io_error(value: io::Error) -> HostError {
    HostError {
        code: "io",
        message: value.to_string(),
    }
}
/// Lowerable finite limits, retained immutably by every issued socket owner.
#[derive(Debug, Clone, Copy)]
pub struct NetworkLimits {
    /// Simultaneously owned listener slots in this authority domain.
    pub listeners: usize,
    /// Simultaneously owned accepted stream slots, including pending accepts.
    pub streams: usize,
    /// Maximum bytes requested by a single read or supplied to a single write.
    pub bytes: usize,
    /// Independent deadline per accept/read/write operation.
    pub milliseconds: u64,
}
impl NetworkLimits {
    /// Initial sequential loopback profile; these are hard upper ceilings.
    pub const DEFAULT: Self = Self {
        listeners: 8,
        streams: 64,
        bytes: 65536,
        milliseconds: 30000,
    };
    fn validate(self) -> Result<(), HostError> {
        if self.listeners > Self::DEFAULT.listeners
            || self.streams > Self::DEFAULT.streams
            || self.bytes > Self::DEFAULT.bytes
            || self.milliseconds > Self::DEFAULT.milliseconds
        {
            return Err(error("policy", "network limits may only narrow defaults"));
        }
        Ok(())
    }
}
#[derive(Debug)]
struct Slot(Arc<AtomicUsize>);
impl Slot {
    fn reserve(count: &Arc<AtomicUsize>, maximum: usize) -> Result<Self, HostError> {
        let mut current = count.load(Ordering::Acquire);
        // Finite admission even if embedding threads continually contend.
        for _ in 0..128 {
            if current >= maximum {
                return Err(error("limit", "network owner limit exceeded"));
            }
            match count.compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Ok(Self(count.clone())),
                Err(next) => current = next,
            }
        }
        Err(error(
            "limit",
            "network owner admission contention limit exceeded",
        ))
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
/// Denied-by-default trusted authority. Grants apply only to future listens.
/// Port zero explicitly grants OS-selected ephemeral ports, never arbitrary chosen ports.
#[derive(Debug)]
pub struct LoopbackNetwork {
    ports: BTreeSet<u16>,
    listeners: Arc<AtomicUsize>,
    streams: Arc<AtomicUsize>,
    limits: NetworkLimits,
}
impl Default for LoopbackNetwork {
    fn default() -> Self {
        Self::new(NetworkLimits::DEFAULT).expect("finite network defaults")
    }
}
impl LoopbackNetwork {
    /// Create a fresh denied authority domain with independently lowerable limits.
    pub fn new(limits: NetworkLimits) -> Result<Self, HostError> {
        limits.validate()?;
        Ok(Self {
            ports: BTreeSet::new(),
            listeners: Arc::new(AtomicUsize::new(0)),
            streams: Arc::new(AtomicUsize::new(0)),
            limits,
        })
    }
    /// Explicit trusted IPv4-loopback listen grant; at most sixteen distinct ports.
    pub fn grant_port(&mut self, port: u16) -> Result<(), HostError> {
        if !self.ports.contains(&port) && self.ports.len() >= 16 {
            return Err(error("limit", "network grant limit exceeded"));
        }
        self.ports.insert(port);
        Ok(())
    }
    /// Current listener/accepted-stream reservations, including pending accepts.
    pub fn usage(&self) -> (usize, usize) {
        (
            self.listeners.load(Ordering::Acquire),
            self.streams.load(Ordering::Acquire),
        )
    }
    /// Bind exactly IPv4 localhost. Callback errors remain separate from host errors.
    pub fn listen<E>(
        &self,
        port: u16,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<LoopbackListener, HostError>, E> {
        if !self.ports.contains(&port) {
            return Ok(Err(error("authority", "loopback port was not granted")));
        }
        if !cfg!(target_os = "linux") {
            return Ok(Err(error(
                "unsupported",
                "loopback host is currently supported on Linux only",
            )));
        }
        checkpoint()?;
        let result = (|| {
            let slot = Slot::reserve(&self.listeners, self.limits.listeners)?;
            let socket = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
                .map_err(io_error)?;
            socket.set_nonblocking(true).map_err(io_error)?;
            Ok(LoopbackListener {
                socket,
                _slot: slot,
                streams: self.streams.clone(),
                limits: self.limits,
            })
        })();
        // Cancelled publication drops any newly bound descriptor and its reservation.
        checkpoint()?;
        Ok(result)
    }
}
/// Unique listener owner. Drop closes the socket and releases its authority slot.
#[derive(Debug)]
pub struct LoopbackListener {
    socket: TcpListener,
    _slot: Slot,
    streams: Arc<AtomicUsize>,
    limits: NetworkLimits,
}
impl LoopbackListener {
    /// Actual bound port, including the OS-selected port for an explicit zero grant.
    pub fn port(&self) -> Result<u16, HostError> {
        self.socket.local_addr().map(|a| a.port()).map_err(io_error)
    }
    /// Attempt one accept syscall without waiting or retrying. None means not ready.
    /// Capacity refusal occurs before dequeuing a peer; cancellation drops unpublished owners.
    pub fn try_accept<E>(
        &self,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<Option<LoopbackStream>, HostError>, E> {
        checkpoint()?;
        if self.limits.milliseconds == 0 {
            return Ok(Err(error("timeout", "network operation deadline exceeded")));
        }
        let result = (|| {
            let slot = Slot::reserve(&self.streams, self.limits.streams)?;
            let (socket, peer) = match self.socket.accept() {
                Ok(pair) => pair,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(io_error(e)),
            };
            if !peer.ip().is_loopback() {
                return Err(error("authority", "accepted peer is not loopback"));
            }
            socket.set_nonblocking(true).map_err(io_error)?;
            Ok(Some(LoopbackStream {
                socket,
                _slot: slot,
                limits: self.limits,
            }))
        })();
        checkpoint()?;
        Ok(result)
    }
    /// Accept one owned stream under a fresh deadline, with no background worker.
    pub fn accept<E>(
        &self,
        checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<LoopbackStream, HostError>, E> {
        let slot = match Slot::reserve(&self.streams, self.limits.streams) {
            Ok(slot) => slot,
            Err(e) => return Ok(Err(e)),
        };
        let accepted = poll(self.limits.milliseconds, checkpoint, || {
            self.socket.accept()
        })?;
        Ok(accepted.and_then(|(socket, peer)| {
            if !peer.ip().is_loopback() {
                return Err(error("authority", "accepted peer is not loopback"));
            }
            socket.set_nonblocking(true).map_err(io_error)?;
            Ok(LoopbackStream {
                socket,
                _slot: slot,
                limits: self.limits,
            })
        }))
    }
    /// Consume this listener; previously accepted streams retain their own ownership.
    pub fn close(self) {
        drop(self);
    }
}
/// A read distinguishes orderly EOF from a nonempty byte chunk.
#[derive(Debug, PartialEq, Eq)]
pub enum NetworkRead {
    /// One partial read, never larger than the caller's admitted maximum.
    Data(Vec<u8>),
    /// The peer closed its sending half; no bytes were read.
    Eof,
}
/// Unique accepted connection owner. It is not Clone, serializable or a copy VM value.
#[derive(Debug)]
pub struct LoopbackStream {
    socket: TcpStream,
    _slot: Slot,
    limits: NetworkLimits,
}
impl LoopbackStream {
    /// One nonblocking partial write. None means not ready; callers retain the suffix.
    /// Cancellation is checked before the syscall; completed bytes are never rolled back.
    pub fn try_write<E>(
        &mut self,
        bytes: &[u8],
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<Option<usize>, HostError>, E> {
        if bytes.len() > self.limits.bytes {
            return Ok(Err(error("limit", "network write byte limit exceeded")));
        }
        checkpoint()?;
        if self.limits.milliseconds == 0 {
            return Ok(Err(error("timeout", "network operation deadline exceeded")));
        }
        Ok(match self.socket.write(bytes) {
            Ok(0) if !bytes.is_empty() => Err(error("io", "network write made no progress")),
            Ok(n) => Ok(Some(n)),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(io_error(e)),
        })
    }
    /// One nonblocking read attempt. None means not ready, distinct from EOF.
    /// No sleeping/retry; cancellation is checked before any bytes are consumed.
    pub fn try_read<E>(
        &mut self,
        maximum: usize,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<Option<NetworkRead>, HostError>, E> {
        if maximum == 0 {
            return Ok(Err(error("input", "read maximum must be nonzero")));
        }
        if maximum > self.limits.bytes {
            return Ok(Err(error("limit", "network read byte limit exceeded")));
        }
        checkpoint()?;
        if self.limits.milliseconds == 0 {
            return Ok(Err(error("timeout", "network operation deadline exceeded")));
        }
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(maximum).is_err() {
            return Ok(Err(error("limit", "network read allocation refused")));
        }
        bytes.resize(maximum, 0);
        Ok(match self.socket.read(&mut bytes) {
            Ok(0) => Ok(Some(NetworkRead::Eof)),
            Ok(n) => {
                bytes.truncate(n);
                Ok(Some(NetworkRead::Data(bytes)))
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(io_error(e)),
        })
    }
    /// Read one nonempty chunk or EOF. Zero maximum is an input error, not EOF.
    pub fn read<E>(
        &mut self,
        maximum: usize,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<NetworkRead, HostError>, E> {
        if maximum == 0 {
            return Ok(Err(error("input", "read maximum must be nonzero")));
        }
        if maximum > self.limits.bytes {
            return Ok(Err(error("limit", "network read byte limit exceeded")));
        }
        checkpoint()?;
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(maximum).is_err() {
            return Ok(Err(error("limit", "network read allocation refused")));
        }
        bytes.resize(maximum, 0);
        Ok(poll(self.limits.milliseconds, checkpoint, || {
            self.socket.read(&mut bytes)
        })?
        .map(|n| {
            if n == 0 {
                NetworkRead::Eof
            } else {
                bytes.truncate(n);
                NetworkRead::Data(bytes)
            }
        }))
    }
    /// Perform one partial write; the caller owns any unwritten suffix.
    /// An over-limit input is refused before sending any prefix.
    pub fn write<E>(
        &mut self,
        bytes: &[u8],
        checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<usize, HostError>, E> {
        if bytes.len() > self.limits.bytes {
            return Ok(Err(error("limit", "network write byte limit exceeded")));
        }
        poll(self.limits.milliseconds, checkpoint, || {
            let n = self.socket.write(bytes)?;
            if n == 0 && !bytes.is_empty() {
                Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "network write made no progress",
                ))
            } else {
                Ok(n)
            }
        })
    }
    /// Consume this owner, shut down both directions and report shutdown errors.
    /// The descriptor and reservation are dropped even when shutdown fails.
    pub fn close(self) -> Result<(), HostError> {
        self.socket.shutdown(Shutdown::Both).map_err(io_error)
    }
}
fn poll<T, E>(
    milliseconds: u64,
    mut checkpoint: impl FnMut() -> Result<(), E>,
    mut attempt: impl FnMut() -> io::Result<T>,
) -> Result<Result<T, HostError>, E> {
    let started = Instant::now();
    loop {
        checkpoint()?;
        if started.elapsed() >= Duration::from_millis(milliseconds) {
            return Ok(Err(error("timeout", "network operation deadline exceeded")));
        }
        match attempt() {
            Ok(value) => return Ok(Ok(value)),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1))
            }
            Err(e) => return Ok(Err(io_error(e))),
        }
    }
}
