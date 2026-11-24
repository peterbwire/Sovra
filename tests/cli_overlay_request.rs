//! Real stdin/stdout transport checks sources without running or writing them.
use sovra::compiler::project::packages::SOURCE_OVERLAY_MAGIC;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr-overlay-wire-{}-{} with spaces",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname = \"wire\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("main.svr"),
            "fn main() { print(\"MUST_NOT_EXECUTE\"); }",
        )
        .unwrap();
        Self(root)
    }
    fn request(&self, source: &[u8]) -> Vec<u8> {
        let path = self.0.join("main.svr");
        let path = path.to_str().unwrap();
        let mut bytes = SOURCE_OVERLAY_MAGIC.to_vec();
        bytes.extend(73u32.to_be_bytes());
        bytes.extend(1u32.to_be_bytes());
        bytes.extend((path.len() as u32).to_be_bytes());
        bytes.extend((source.len() as u32).to_be_bytes());
        bytes.extend(path.as_bytes());
        bytes.extend(source);
        bytes
    }
    fn run(&self, flags: &[&str], input: &[u8]) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg("check")
            .args(flags)
            .arg(&self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if !input.is_empty() {
            child.stdin.take().unwrap().write_all(input).unwrap();
        } else {
            drop(child.stdin.take());
        }
        child.wait_with_output().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn valid_request_echoes_id_and_checks_unsaved_text_without_execution_or_disk_writes() {
    let f = Fixture::new();
    let disk = std::fs::read(f.0.join("main.svr")).unwrap();
    let output = f.run(
        &["--format", "json", "--overlays-stdin"],
        &f.request(b"fn main() {}"),
    );
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"success\":true"));
    assert!(text.contains("\"request_id\":73"));
    assert!(!text.contains("MUST_NOT_EXECUTE"));
    assert_eq!(std::fs::read(f.0.join("main.svr")).unwrap(), disk);
    let failed = f.run(
        &["--format=json", "--overlays-stdin"],
        &f.request("// λ😀\r\nfn main() { missing(); }".as_bytes()),
    );
    assert_eq!(failed.status.code(), Some(1));
    assert!(failed.stderr.is_empty());
    let text = String::from_utf8(failed.stdout).unwrap();
    assert!(text.contains("E3004"));
    assert!(text.contains("\"request_id\":73"));
    assert!(text.contains("\"line\":1"));
}
#[test]
fn malformed_request_is_json_failure_with_null_id_and_incompatible_flags_are_usage_errors() {
    let f = Fixture::new();
    let output = f.run(&["--format=json", "--overlays-stdin"], b"bad");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("E4119"));
    assert!(text.contains("\"request_id\":null"));
    assert!(text.contains("\"location\":null"));
    for flags in [
        &["--overlays-stdin"][..],
        &["--format=json", "--symbols", "--overlays-stdin"][..],
        &["--format=json", "--service-calls", "--overlays-stdin"][..],
        &["--format=json", "--overlays-stdin", "--overlays-stdin"][..],
    ] {
        let output = f.run(flags, b"");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn package_context_is_canonical_graph_discovery_not_source_validation() {
    let f = Fixture::new();
    std::fs::write(f.0.join("main.svr"), "not valid Sovra!").unwrap();
    let output = f.run(&["--format=json", "--package-context"], b"");
    assert!(output.status.success(), "{:?}", output);
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("\"scope\":\"package_graph\""));
    assert!(report.contains("\"source_check\":false"));
    assert!(report.contains("\"package_context\":{\"schema_version\":1"));
    assert!(!report.contains("E2001"));
    for flags in [
        &["--package-context"][..],
        &["--format=json", "--package-context", "--overlays-stdin"][..],
        &["--format=json", "--package-context", "--package-context"][..],
    ] {
        assert_eq!(f.run(flags, b"").status.code(), Some(2));
    }
}

#[test]
fn explicit_executable_package_check_never_uses_partial_application_scanning() {
    let f = Fixture::new();
    let valid = f.run(&["--executable-package", "--format=json"], b"");
    assert!(valid.status.success());
    assert!(!String::from_utf8(valid.stdout)
        .unwrap()
        .contains("MUST_NOT_EXECUTE"));
    std::fs::write(f.0.join("main.svr"), "fn main() { missing(); }").unwrap();
    let invalid = f.run(&["--executable-package", "--format=json"], b"");
    assert_eq!(invalid.status.code(), Some(1));
    assert!(String::from_utf8(invalid.stdout).unwrap().contains("E3004"));
    let incompatible = f.run(
        &["--executable-package", "--package-context", "--format=json"],
        b"",
    );
    assert_eq!(incompatible.status.code(), Some(2));
}
