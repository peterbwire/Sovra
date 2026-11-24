//! Postfix propagation through opt-in commands, locked packages and an independent executable.
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "sovra propagation {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("main.svr"),
            format!(
                "{}\n{}",
                sovra::compiler::v2::STANDARD_RESULT_ADAPTER_SOURCE,
                include_str!("../docs/examples/generic-result-adapters.svr")
            ),
        )
        .unwrap();
        Self(path)
    }
    fn cli(&self, args: &[&str], package: bool) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .arg(if package {
                self.0.clone()
            } else {
                self.0.join("main.svr")
            })
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
#[test]
fn postfix_source_checks_and_runs_only_with_the_explicit_generic_flag() {
    let f = Fixture::new();
    let report = success(f.cli(
        &["check", "--experimental-generics", "--format", "json"],
        false,
    ));
    assert!(String::from_utf8_lossy(&report.stdout).contains("\"diagnostics\":[]"));
    assert!(success(f.cli(&["run", "--experimental-generics"], false))
        .stdout
        .is_empty());
    assert!(!f.cli(&["check"], false).status.success());
}
#[test]
fn postfix_cli_reports_original_question_ranges_and_refuses_failed_build_publication() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("main.svr"),
        "// λ\nfn main(){}\nfn unused(x:std::Option<Int>)->Int{let n=x?;return 1;}",
    )
    .unwrap();
    let report = f.cli(
        &["check", "--experimental-generics", "--format", "json"],
        false,
    );
    assert!(!report.status.success());
    let text = String::from_utf8_lossy(&report.stdout);
    assert!(text.contains("E3123"), "{text}");
    assert!(text.contains("\"line\":2"), "{text}");
    let output = f.0.join("should-not-exist");
    let build = f.cli(
        &[
            "build",
            "--experimental-generics",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
        false,
    );
    assert!(!build.status.success());
    assert!(!output.exists());
}
#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn locked_postfix_package_links_and_runs_without_source_manifest_or_tools() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("sovra.toml"),
        "[project]\nname=\"propagation\"\nentry=\"main.svr\"\n",
    )
    .unwrap();
    success(f.cli(&["lock", "--experimental-generics"], true));
    success(f.cli(
        &[
            "check",
            "--experimental-generics",
            "--locked",
            "--format",
            "json",
        ],
        true,
    ));
    success(f.cli(&["run", "--experimental-generics", "--locked"], true));
    let output = f.0.join("application");
    success(f.cli(
        &[
            "build",
            "--experimental-generics",
            "--locked",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
        true,
    ));
    for file in ["main.svr", "sovra.toml", "sovra.lock"] {
        std::fs::remove_file(f.0.join(file)).unwrap();
    }
    let empty = f.0.join("empty-path");
    std::fs::create_dir(&empty).unwrap();
    let ran = success(
        Command::new(&output)
            .env("PATH", &empty)
            .current_dir(&empty)
            .output()
            .unwrap(),
    );
    assert!(ran.stdout.is_empty());
    assert!(ran.stderr.is_empty());
    assert!(!Command::new(&output)
        .arg("unsupported")
        .env("PATH", &empty)
        .output()
        .unwrap()
        .status
        .success());
    assert!(!std::fs::read_dir(&f.0).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".sovra-exe-stage-")));
}
