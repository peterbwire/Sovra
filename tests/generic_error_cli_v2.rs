//! Source and locked native package proof for structured standard errors.
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "sovra standard error {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(
            p.join("main.svr"),
            include_str!("../docs/examples/generic-error.svr"),
        )
        .unwrap();
        Self(p)
    }
    fn source(&self, command: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args([command, "--experimental-generics"])
            .arg(self.0.join("main.svr"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn documented_structured_error_checks_and_executes() {
    let f = Fixture::new();
    for command in ["check", "run"] {
        let out = f.source(command);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(f.source("run").stdout.is_empty());
}
#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn locked_error_package_build_runs_without_its_inputs_or_tools() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("sovra.toml"),
        "[project]\nname=\"error\"\nentry=\"main.svr\"\n",
    )
    .unwrap();
    let lock = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["lock", "--experimental-generics"])
        .arg(&f.0)
        .output()
        .unwrap();
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let binary = f.0.join("application");
    let out = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args([
            "build",
            "--experimental-generics",
            "--locked",
            "--emit",
            "exe",
            "--output",
        ])
        .arg(&binary)
        .arg(&f.0)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for file in ["main.svr", "sovra.toml", "sovra.lock"] {
        std::fs::remove_file(f.0.join(file)).unwrap();
    }
    let empty = f.0.join("empty");
    std::fs::create_dir(&empty).unwrap();
    let out = Command::new(binary)
        .current_dir(&empty)
        .env("PATH", empty)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
}
