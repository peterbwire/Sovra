//! Bounded tool probes without shell execution or pipe-reader threads.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const LIMIT: u64 = 64 * 1024;
static NEXT: AtomicUsize = AtomicUsize::new(0);

pub(super) fn resolve(name: &str) -> Result<PathBuf, String> {
    let path = std::env::var_os("PATH").ok_or("PATH is unavailable")?;
    for directory in std::env::split_paths(&path) {
        // Do not resolve through implicit working-directory entries.
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        });
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!("{name} executable not found on PATH"))
}

struct Capture(PathBuf);
impl Drop for Capture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn capture() -> Result<Capture, String> {
    for _ in 0..128 {
        let path = std::env::temp_dir().join(format!(
            "sovra-doctor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(Capture(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("cannot reserve probe capture: {e}")),
        }
    }
    Err("cannot reserve probe capture".into())
}

fn configure(command: &mut Command, directory: &Path) {
    command
        .current_dir(directory)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("RUSTUP_TRACE_DIR")
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .env_remove("NODE_V8_COVERAGE")
        .env_remove("NODE_REDIRECT_WARNINGS");
    if Path::new(command.get_program())
        .file_name()
        .is_some_and(|p| p.to_string_lossy().eq_ignore_ascii_case("powershell.exe"))
    {
        if let Some(parent) = Path::new(command.get_program()).parent() {
            let modules = parent.join("Modules");
            command.env("PSModulePath", modules);
        }
    }
}

pub(super) fn run(
    program: &Path,
    arguments: &[&OsStr],
    timeout: Duration,
) -> Result<String, String> {
    let capture = capture()?;
    let out = capture.0.join("stdout");
    let err = capture.0.join("stderr");
    let mut command = Command::new(program);
    configure(&mut command, &capture.0);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::from(File::create(&out).map_err(|e| e.to_string())?))
        .stderr(Stdio::from(File::create(&err).map_err(|e| e.to_string())?));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|e| {
        let category =
            if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(4551) {
                "probe execution blocked"
            } else {
                "cannot execute"
            };
        format!("{category} {}: {e}", program.display())
    })?;
    drop(command); // Release parent capture handles before cleanup.
    let started = Instant::now();
    let result = loop {
        let size = fs::metadata(&out)
            .and_then(|a| fs::metadata(&err).map(|b| a.len().saturating_add(b.len())));
        match size {
            Err(e) => break Err(format!("cannot inspect probe output: {e}")),
            Ok(size) if size > LIMIT => break Err("probe output exceeded 64 KiB".into()),
            _ => {}
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    break Err(format!("probe exited with {status}"));
                }
                break (|| {
                    let mut bytes = Vec::new();
                    for file in [&out, &err] {
                        let remaining = LIMIT.saturating_sub(bytes.len() as u64);
                        File::open(file)
                            .and_then(|f| f.take(remaining + 1).read_to_end(&mut bytes))
                            .map_err(|e| e.to_string())?;
                        if bytes.len() as u64 > LIMIT {
                            return Err("probe output exceeded 64 KiB".into());
                        }
                    }
                    String::from_utf8(bytes).map_err(|_| "probe output is not UTF-8".into())
                })();
            }
            Err(e) => break Err(format!("cannot inspect probe process: {e}")),
            _ => {}
        }
        if started.elapsed() >= timeout {
            break Err("probe timed out".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    // Capturing both streams in owned temporary files avoids blocked pipe readers
    // when a descendant inherits handles. Only bounded output is read into memory.
    let cleanup = fs::remove_dir_all(&capture.0);
    match (result, cleanup) {
        (_, Err(e)) => Err(format!("probe capture cleanup failed: {e}")),
        (result, _) => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(script: &str, timeout: Duration) -> Result<String, String> {
        run(
            &resolve("node").unwrap(),
            &[OsStr::new("-e"), OsStr::new(script)],
            timeout,
        )
    }
    #[test]
    fn captures_streams_and_rejects_errors_timeout_and_overflow() {
        assert_eq!(
            node(
                "process.stdout.write('one'); process.stderr.write('two')",
                Duration::from_secs(5)
            )
            .unwrap(),
            "onetwo"
        );
        assert!(node("process.exit(3)", Duration::from_secs(5))
            .unwrap_err()
            .contains("exited"));
        assert!(node("setInterval(()=>{},1000)", Duration::from_millis(150))
            .unwrap_err()
            .contains("timed out"));
        assert!(node(
            "process.stdout.write('x'.repeat(70000))",
            Duration::from_secs(5)
        )
        .unwrap_err()
        .contains("64 KiB"));
        assert!(node(
            "process.stderr.write('x'.repeat(70000))",
            Duration::from_secs(5)
        )
        .unwrap_err()
        .contains("64 KiB"));
        assert!(node(
            "process.stdout.write('x'.repeat(33000)); process.stderr.write('y'.repeat(33000))",
            Duration::from_secs(5)
        )
        .unwrap_err()
        .contains("64 KiB"));
        assert!(node(
            "process.stdout.write(Buffer.from([255]))",
            Duration::from_secs(5)
        )
        .unwrap_err()
        .contains("UTF-8"));
    }
    #[test]
    fn missing_program_is_a_probe_failure() {
        assert!(run(
            Path::new("sovra-nonexistent-probe"),
            &[],
            Duration::from_secs(1)
        )
        .unwrap_err()
        .contains("cannot execute"));
    }

    #[test]
    fn owned_capture_directory_is_removed_on_drop() {
        let capture = capture().unwrap();
        let path = capture.0.clone();
        assert!(path.is_dir());
        drop(capture);
        assert!(!path.exists());
    }

    #[test]
    fn probes_disable_auto_install_and_startup_hooks_without_changing_parent_environment() {
        let before = std::env::var_os("RUSTUP_AUTO_INSTALL");
        let mut command = Command::new("node");
        configure(&mut command, Path::new("private-capture"));
        let env = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            env.get(OsStr::new("RUSTUP_AUTO_INSTALL")),
            Some(&Some(OsStr::new("0")))
        );
        assert_eq!(
            env.get(OsStr::new("CARGO_NET_OFFLINE")),
            Some(&Some(OsStr::new("true")))
        );
        assert_eq!(env.get(OsStr::new("NODE_OPTIONS")), Some(&None));
        assert_eq!(env.get(OsStr::new("NODE_V8_COVERAGE")), Some(&None));
        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("private-capture"))
        );
        assert_eq!(std::env::var_os("RUSTUP_AUTO_INSTALL"), before);
    }
}
