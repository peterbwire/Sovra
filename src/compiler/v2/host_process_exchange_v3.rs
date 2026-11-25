//! Private owned process polling foundation. No source-visible process operation is added.
use super::{encode, err, io, obj, HostError, HostLimits, OwnedChild, J};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    process::{Child, ExitStatus},
    time::{Duration, Instant},
};
/// Owns one child and all pipe progress; no secret input/output is exposed through Debug.
pub(crate) struct OwnedProcessExchange {
    child: OwnedChild,
    input_pipe: UnixStream,
    stdout: UnixStream,
    stderr: UnixStream,
    input: Vec<u8>,
    output: Vec<u8>,
    errors: Vec<u8>,
    sent: usize,
    closed: bool,
    out_eof: bool,
    err_eof: bool,
    status: Option<ExitStatus>,
    start: Instant,
    limits: HostLimits,
    complete: bool,
}
impl std::fmt::Debug for OwnedProcessExchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedProcessExchange")
            .finish_non_exhaustive()
    }
}
impl OwnedProcessExchange {
    pub(super) fn new(
        child: Child,
        input_pipe: UnixStream,
        stdout: UnixStream,
        stderr: UnixStream,
        input: Vec<u8>,
        limits: HostLimits,
    ) -> Self {
        Self {
            child: OwnedChild(child),
            input_pipe,
            stdout,
            stderr,
            input,
            output: Vec::new(),
            errors: Vec::new(),
            sent: 0,
            closed: false,
            out_eof: false,
            err_eof: false,
            status: None,
            start: Instant::now(),
            limits,
            complete: false,
        }
    }
    /// At most one write, one read per output stream and one try-wait; never sleeps.
    pub(crate) fn poll<E>(
        &mut self,
        tick: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<Option<J>, HostError>, E> {
        tick()?;
        if self.complete {
            return Ok(Err(err("input", "owned process task already complete")));
        }
        if self.start.elapsed() > Duration::from_millis(self.limits.milliseconds) {
            return Ok(Err(err("timeout", "owned child deadline")));
        }
        Ok(self.step())
    }
    fn step(&mut self) -> Result<Option<J>, HostError> {
        if !self.closed {
            if self.sent < self.input.len() {
                match self.input_pipe.write(&self.input[self.sent..]) {
                    Ok(0) => self.closed = true,
                    Ok(n) => self.sent += n,
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => self.closed = true,
                    Err(e) => return Err(io(e)),
                }
            }
            if self.sent == self.input.len() || self.closed {
                let _ = self.input_pipe.shutdown(std::net::Shutdown::Write);
                self.closed = true;
            }
        }
        let mut quiet = true;
        let exited_before = self.status.is_some();
        for (pipe, data, eof) in [
            (&mut self.stdout, &mut self.output, &mut self.out_eof),
            (&mut self.stderr, &mut self.errors, &mut self.err_eof),
        ] {
            if *eof {
                continue;
            }
            let mut chunk = [0u8; 8192];
            match pipe.read(&mut chunk) {
                Ok(0) => *eof = true,
                Ok(n) => {
                    quiet = false;
                    if n > self.limits.bytes.saturating_sub(data.len()) {
                        return Err(err("limit", "child output ceiling"));
                    }
                    data.extend_from_slice(&chunk[..n]);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(io(e)),
            }
        }
        if self.status.is_none() {
            self.status = self.child.try_wait().map_err(io)?;
        }
        if let Some(exit) = self.status {
            if (self.out_eof && self.err_eof) || (exited_before && quiet) {
                self.complete = true;
                return Ok(Some(obj(vec![
                    ("exit_code", J::Int(exit.code().map_or(-1, i64::from))),
                    ("stdout", encode(&self.output)?),
                    ("stderr", encode(&self.errors)?),
                ])));
            }
        }
        Ok(None)
    }
    /// Direct-child cleanup remains explicit; Drop is a kill/reap fallback.
    pub(crate) fn cleanup(&mut self) -> Result<(), HostError> {
        let result = if self.status.is_some() {
            self.child.wait()
        } else {
            let killed = self.child.kill();
            let waited = self.child.wait();
            match (killed, waited) {
                (_, Ok(status)) => Ok(status),
                (Err(e), Err(_)) => Err(e),
                (_, Err(e)) => Err(e),
            }
        };
        result
            .map(|_| ())
            .map_err(|e| err("cleanup", format!("owned child cleanup failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::fd::OwnedFd,
        process::{Command, Stdio},
    };
    fn task(input: Vec<u8>, bytes: usize) -> OwnedProcessExchange {
        let (a, child_in) = UnixStream::pair().unwrap();
        let (b, child_out) = UnixStream::pair().unwrap();
        let (c, child_err) = UnixStream::pair().unwrap();
        for socket in [&a, &b, &c] {
            socket.set_nonblocking(true).unwrap();
        }
        let child = Command::new("/bin/cat")
            .env_clear()
            .stdin(Stdio::from(OwnedFd::from(child_in)))
            .stdout(Stdio::from(OwnedFd::from(child_out)))
            .stderr(Stdio::from(OwnedFd::from(child_err)))
            .spawn()
            .unwrap();
        OwnedProcessExchange::new(
            child,
            a,
            b,
            c,
            input,
            HostLimits {
                bytes,
                milliseconds: 1000,
                ..HostLimits::DEFAULT
            },
        )
    }
    #[test]
    fn process_poll_retains_partial_streams_and_publishes_one_complete_result() {
        let input = b"private-process-fixture\0\xff".repeat(1500);
        let mut task = task(input.clone(), 65536);
        assert!(!format!("{task:?}").contains("private-process-fixture"));
        let mut result = None;
        for _ in 0..1000 {
            let before = task.output.len();
            result = task.poll(&mut || Ok::<_, ()>(())).unwrap().unwrap();
            assert!(task.output.len() - before <= 8192);
            if result.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let result = result.expect("complete child exchange");
        assert_eq!(
            super::super::field(&result, "exit_code").unwrap(),
            &J::Int(0)
        );
        let stdout =
            super::super::decode(super::super::field(&result, "stdout").unwrap(), 65536).unwrap();
        assert_eq!(stdout, input);
        assert_eq!(
            task.poll(&mut || Ok::<_, ()>(()))
                .unwrap()
                .unwrap_err()
                .code,
            "input"
        );
        task.cleanup().unwrap();
    }
    #[test]
    fn process_poll_control_refusal_allows_explicit_kill_and_reap() {
        let mut task = task(Vec::new(), 4096);
        let pid = task.child.id();
        assert_eq!(
            task.poll(&mut || Err("cancel task")).unwrap_err(),
            "cancel task"
        );
        task.cleanup().unwrap();
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
