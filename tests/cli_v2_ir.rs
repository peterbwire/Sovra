//! Explicit v2 IR building: no effects, bounded publication and strict backend selection.
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "sovra-cli-v2-ir-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn source(&self, name: &str, source: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, source).unwrap();
        p
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
#[test]
fn independent_enum_program_builds_deterministic_ir_only_when_selected() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        include_str!("../examples/enum-functions/main.svr"),
    );
    let first = t.cli(&["build", "--experimental-v2", "--emit", "ir", text(&p)]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stderr.is_empty());
    let inspection = String::from_utf8(first.stdout.clone()).unwrap();
    assert!(inspection.starts_with("sovra-ir v2 inspection-only\n"));
    assert!(inspection.contains("switch m0 e0") && inspection.contains("call function f1"));
    let second = t.cli(&["build", "--experimental-v2", text(&p)]);
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(!t.cli(&["build", text(&p)]).status.success());
}
#[test]
fn inspection_has_no_input_file_print_exit_or_loop_execution() {
    let t = Temp::new();
    let p=t.source("main.svr","fn main(){std::write_text(\"marker\",\"bad\");let line=std::read_line();print(line.text);std::set_exit_code(7);print(1/0);while true{}}");
    let o = t.cli(&["build", "--experimental-v2", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.starts_with("sovra-ir v2 inspection-only\n"));
    assert!(!t.0.join("marker").exists());
    for part in [
        "std::write_text",
        "std::read_line",
        "std::set_exit_code",
        "binary \"/\"",
        "jump pc",
    ] {
        assert!(s.contains(part), "{part}");
    }
}
#[test]
fn helper_only_source_builds_and_source_or_read_errors_publish_no_ir() {
    let t = Temp::new();
    let p = t.source("helpers.svr", "fn helper(n:Int)->Int{return n+1;}");
    let o = t.cli(&["build", "--experimental-v2", text(&p)]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("function f0 \"helper\""));
    for (name, code) in [("bad.svr", "E3001"), ("missing.svr", "cannot inspect")] {
        let p = if name == "bad.svr" {
            t.source(name, "fn helper(){print(unknown);}")
        } else {
            t.0.join(name)
        };
        let o = t.cli(&["build", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(String::from_utf8_lossy(&o.stderr).contains(code));
    }
}
#[test]
fn unavailable_backends_grants_output_and_unsupported_options_are_usage_errors() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn main(){}");
    let directory = t.source("invalid.txt", "");
    for args in [
        vec!["build", "--experimental-v2"],
        vec!["build", "--experimental-v2", "--emit", "js", text(&p)],
        vec!["build", "--experimental-v2", "--emit", "exe", text(&p)],
        vec![
            "build",
            "--experimental-v2",
            "--emit",
            "ir",
            "--emit",
            "ir",
            text(&p),
        ],
        vec!["build", "--experimental-v2", "--experimental-v2", text(&p)],
        vec!["build", "--experimental-v2", "--allow-files", text(&p)],
        vec!["build", "--experimental-v2", "--allow-stdin", text(&p)],
        vec!["build", "--experimental-v2", "--locked", text(&p)],
        vec!["build", "--experimental-v2", "--format", "json", text(&p)],
        vec!["build", "--experimental-v2", "--output", "app", text(&p)],
        vec!["build", "--experimental-v2", text(&p), "extra"],
        vec!["build", "--experimental-v2", text(&directory)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(o.stdout.is_empty());
    }
    assert!(!t.0.join("app").exists());
    let o = t.cli(&["build", "--help"]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("source-backed native executable"));
    t.source("--emit.svr", "fn helper(){}");
    let o = t.cli(&["build", "--experimental-v2", "--", "--emit.svr"]);
    assert!(o.status.success());
}
#[test]
fn full_ir_output_limit_refuses_without_publishing_a_prefix() {
    let t = Temp::new();
    let source = format!("fn helper(){{print(\"{}\");}}", "x".repeat(4 * 1024 * 1024));
    let p = t.source("large.svr", &source);
    let o = t.cli(&["build", "--experimental-v2", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("IR inspection refused: Limit"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
}
#[test]
fn string_payload_controls_are_escaped_and_unicode_is_preserved() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn helper(){print(\"λ\\n\\t\\\"\");}");
    let o = t.cli(&["build", "--experimental-v2", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("λ\\n\\t"));
    assert!(!s.contains('\t'));
    assert!(!s.contains("λ\n"));
}
#[cfg(unix)]
#[test]
fn closed_stdout_is_an_ir_publication_failure_without_panic() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn helper(){}");
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let o = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["build", "--experimental-v2", text(&p)])
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let error = String::from_utf8_lossy(&o.stderr);
    assert!(error.contains("cannot publish IR inspection") && !error.contains("panicked"));
}
