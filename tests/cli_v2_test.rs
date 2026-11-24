//! Explicit source-only v2 test command, reports and effect boundaries.
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
            "sovra-cli-v2-test-{}-{}",
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
fn independent_fixture_is_testable_without_main_in_human_and_json_modes() {
    let t = Temp::new();
    let p = t.source("main.svr", include_str!("../examples/enum-tests/main.svr"));
    let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(o.stderr.is_empty());
    let s = String::from_utf8(o.stdout).unwrap();
    assert_eq!(s.lines().count(), 1);
    for part in [
        "\"schema_version\":1",
        "\"kind\":\"source\"",
        "\"success\":true",
        "\"passed\":2",
        "\"total\":2",
        "\"output\":[\"14\"]",
        "\"output\":[\"end\"]",
    ] {
        assert!(s.contains(part), "{s}");
    }
    let o = t.cli(&["test", "--experimental-v2", text(&p)]);
    assert!(o.status.success());
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "PASS test_value\n  14\nPASS test_end\n  end\n2 passed; 0 failed; 2 total\n"
    );
    assert!(!t.cli(&["test", text(&p)]).status.success());
}
#[test]
fn failures_continue_in_order_with_fresh_exit_status_and_no_raw_output_document() {
    let t = Temp::new();
    let p=t.source("main.svr","fn main(){print(\"never\");}fn test_status(){std::set_exit_code(7);}fn test_error(){print(\"hidden\");print(1/0);}fn test_next(){print(\"héllo\\nworld\");}");
    let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stderr.is_empty());
    let s = String::from_utf8(o.stdout).unwrap();
    assert_eq!(s.lines().count(), 1);
    assert!(s.contains("\"passed\":1,\"failed\":2,\"total\":3"));
    assert!(s.contains("\"exit_code\":7"));
    assert!(s.contains("\"exit_code\":null"));
    assert!(!s.contains("never") && !s.contains("hidden"));
    assert!(s.contains("héllo\\nworld"));
    assert!(s.find("test_status").unwrap() < s.find("test_error").unwrap());
    assert!(s.find("test_error").unwrap() < s.find("test_next").unwrap());
}
#[test]
fn invalid_signatures_and_no_tests_emit_only_a_configuration_report() {
    let t = Temp::new();
    for (source, code) in [
        ("// λ😀\r\nfn test_good(){}fn test_bad(x:Int){}", "E5001"),
        ("fn helper(){}", "E5002"),
        ("fn test_a(){print(missing);}", "E3001"),
    ] {
        let p = t.source("main.svr", source);
        let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stderr.is_empty());
        let s = String::from_utf8(o.stdout).unwrap();
        assert_eq!(s.lines().count(), 1);
        assert!(
            s.contains(code) && s.contains("\"tests\":[]") && s.contains("\"total\":0"),
            "{s}"
        );
        if code == "E5001" {
            assert!(s.contains(&format!(
                "\"start\":{}",
                source.find("fn test_bad").unwrap()
            )));
        }
    }
}
#[test]
fn unsupported_options_and_regular_source_requirements_are_usage_errors() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn test_a(){}");
    let directory = t.source("invalid.txt", "");
    for args in [
        vec!["test", "--experimental-v2"],
        vec!["test", "--experimental-v2", "--experimental-v2", text(&p)],
        vec!["test", "--experimental-v2", "--locked", text(&p)],
        vec!["test", "--experimental-v2", "--symbols", text(&p)],
        vec!["test", "--experimental-v2", "--format", "xml", text(&p)],
        vec![
            "test",
            "--experimental-v2",
            "--format",
            "json",
            "--format",
            "human",
            text(&p),
        ],
        vec!["test", "--experimental-v2", text(&p), "extra"],
        vec!["test", "--experimental-v2", text(&directory)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(o.stdout.is_empty());
    }
    let o = t.cli(&["test", "--experimental-v2", "--help"]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("--experimental-v2"));
    let dash = t.source("--format.svr", "fn test_a(){}");
    let o = t.cli(&["test", "--experimental-v2", "--", "--format.svr"]);
    assert!(o.status.success(), "{} {:?}", dash.display(), o);
}
#[test]
fn io_and_source_limits_use_the_test_schema_without_executing() {
    let t = Temp::new();
    let missing = t.0.join("missing.svr");
    let oversized = t.0.join("large.svr");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    for (path, code) in [(&missing, "E0001"), (&oversized, "E1003")] {
        let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(path)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stderr.is_empty());
        let s = String::from_utf8(o.stdout).unwrap();
        assert_eq!(s.lines().count(), 1);
        assert!(s.contains(code) && s.contains("\"tests\":[]"));
        assert!(s.contains("\"span\":null"));
    }
}
#[test]
fn unused_host_calls_are_refused_and_never_write_files() {
    let t = Temp::new();
    for op in ["std::write_text(\"marker\",\"bad\");", "std::read_line();"] {
        let p = t.source(
            "main.svr",
            &format!("fn unused(){{{op}}}fn test_a(){{print(\"hidden\");}}"),
        );
        let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        let s = String::from_utf8(o.stdout).unwrap();
        assert!(s.contains("UnsupportedStandard") && s.contains("\"failed\":1"));
        assert!(!s.contains("hidden"));
        assert!(!t.0.join("marker").exists());
    }
}
#[test]
fn default_suite_count_is_exact_and_excess_refuses_before_execution() {
    let t = Temp::new();
    for n in [32, 33] {
        let source = (0..n)
            .map(|i| format!("fn test_{i}(){{}}\n"))
            .collect::<String>();
        let p = t.source("main.svr", &source);
        let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(&p)]);
        let s = String::from_utf8(o.stdout).unwrap();
        if n == 32 {
            assert!(o.status.success());
            assert!(s.contains("\"passed\":32"));
        } else {
            assert_eq!(o.status.code(), Some(1));
            assert!(s.contains("E5003") && s.contains("\"tests\":[]"));
        }
    }
}
#[cfg(unix)]
#[test]
fn human_diagnostics_escape_control_bytes_in_source_paths() {
    let t = Temp::new();
    let p = t.source("bad\n\x1b.svr", "fn test_bad(x:Int){}");
    let o = t.cli(&["test", "--experimental-v2", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(!s.contains('\x1b'));
    assert_eq!(s.lines().count(), 2);
    assert!(s.contains("E5001"));
}
#[cfg(unix)]
#[test]
fn closed_stdout_returns_failure_without_panic() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn test_a(){}");
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader); // No reader exists before spawn, so publication cannot race its closure.
    let child = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["test", "--experimental-v2", text(&p)])
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let s = String::from_utf8_lossy(&o.stderr);
    assert!(s.contains("cannot publish test report") && !s.contains("panicked"));
}
