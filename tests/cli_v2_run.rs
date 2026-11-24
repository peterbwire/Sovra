//! Explicit linked VM command execution and capability/resource boundaries.
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
            "sovra-cli-v2-run-{}-{}",
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
fn independent_enum_fixtures_run_only_with_explicit_selection() {
    let t = Temp::new();
    for (source, expected) in [
        (include_str!("../examples/enum-entry/main.svr"), "7\n"),
        (include_str!("../examples/enum-functions/main.svr"), "14\n"),
        (
            include_str!("../examples/nested-matches/main.svr"),
            "7\n0\n0\n",
        ),
        (include_str!("../examples/control-matches/main.svr"), "9\n"),
    ] {
        let p = t.source("main.svr", source);
        let o = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(String::from_utf8_lossy(&o.stdout), expected);
        assert!(o.stderr.is_empty());
        assert!(!t.cli(&["run", text(&p)]).status.success());
    }
}
#[test]
fn program_arguments_and_exit_status_flow_through_linked_calls() {
    let t = Temp::new();
    let p=t.source("main.svr", "fn main(){helper();} fn helper(){print(std::arg_count());print(std::arg(0));print(std::arg(1));print(std::arg(2));std::set_exit_code(7);}");
    let o = t.cli(&[
        "run",
        "--experimental-v2",
        text(&p),
        "--",
        "--help",
        "--experimental-v2",
        "héllo",
    ]);
    assert_eq!(
        o.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "3\n--help\n--experimental-v2\nhéllo\n"
    );
    assert!(o.stderr.is_empty());
}
#[test]
fn missing_or_invalid_main_and_invalid_source_never_publish_output() {
    let t = Temp::new();
    for (source, message) in [
        ("fn helper(){}", "requires a parameterless Unit main"),
        ("fn main(x:Int){print(x);}", "E3009"),
        ("fn main()->Int{return 1;}", "E3010"),
        ("fn main(){print(unknown);}", "E3001"),
    ] {
        let p = t.source("main.svr", source);
        let o = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&o.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}
#[test]
fn file_and_input_capabilities_are_denied_even_in_unused_functions() {
    let t = Temp::new();
    for (body, name) in [
        ("std::write_text(\"marker\",\"bad\");", "std::write_text"),
        ("std::read_text(\"marker\");", "std::read_text"),
        (
            "std::write_text_if(\"marker\",false,\"\",\"bad\");",
            "std::write_text_if",
        ),
        ("std::read_line();", "std::read_line"),
    ] {
        let p = t.source(
            "main.svr",
            &format!("fn main(){{print(\"hidden\");}} fn unused(){{{body}}}"),
        );
        let o = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&o.stderr).contains(name),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(!t.0.join("marker").exists());
    }
}
#[test]
fn runtime_errors_discard_captured_output_and_requested_status() {
    let t = Temp::new();
    for source in [
        "fn main(){print(\"hidden\");std::set_exit_code(7);print(1/0);}",
        "fn main(){print(\"hidden\");std::set_exit_code(126);}",
    ] {
        let p = t.source("main.svr", source);
        let o = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(String::from_utf8_lossy(&o.stderr).contains("execution refused"));
    }
}
#[test]
fn finite_instruction_and_call_depth_limits_stop_unbounded_programs() {
    let t = Temp::new();
    for (source, message) in [
        ("fn main(){while true {}}", "instruction allowance"),
        ("fn main(){again();} fn again(){again();}", "CallDepth"),
    ] {
        let p = t.source("main.svr", source);
        let o = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&o.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}
#[test]
fn finite_output_and_materialization_limits_refuse_growth_without_publication() {
    let t = Temp::new();
    for (source, message) in [
        (
            format!("fn main(){{print(\"{}\");}}", "x".repeat(1_048_576)),
            "output byte allowance",
        ),
        (
            "fn main(){let mut xs=[0];let mut n=0;while n<24{xs=std::array_concat(xs,xs);n=n+1;}}"
                .into(),
            "NodeLimit",
        ),
        (
            "fn main(){let mut s=\"x\";let mut n=0;while n<25{s=s+s;n=n+1;}}".into(),
            "TextLimit",
        ),
    ] {
        let p = t.source("main.svr", &source);
        let o = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&o.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}
#[test]
fn unsupported_options_and_packages_are_usage_errors_and_help_is_explicit() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn main(){}");
    for args in [
        vec!["run", "--experimental-v2"],
        vec!["run", "--experimental-v2", "--experimental-v2", text(&p)],
        vec!["run", "--experimental-v2", "--locked", text(&p)],
        vec!["run", "--experimental-v2", "--format", "json", text(&p)],
        vec!["run", "--experimental-v2", text(&p), "extra"],
        vec![
            "run",
            "--experimental-v2",
            text(&t.source("invalid.txt", "")),
        ],
        vec!["run", "--experimental-v2", "--", text(&p)],
        vec!["build", "--experimental-v2", "--emit", "js", text(&p)],
    ] {
        assert_eq!(t.cli(&args).status.code(), Some(2), "{args:?}");
    }
    let help = t.cli(&["run", "--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--experimental-v2"));
    let missing = t.cli(&["run", "--experimental-v2", text(&t.0.join("missing.svr"))]);
    assert_eq!(missing.status.code(), Some(1));
}
#[test]
fn successful_program_output_keeps_raw_text_and_status_zero() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn main(){print(\"héllo\\nworld\");}");
    let o = t.cli(&["run", "--experimental-v2", text(&p)]);
    assert!(o.status.success());
    assert_eq!(String::from_utf8_lossy(&o.stdout), "héllo\nworld\n");
    assert!(o.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn closed_stdout_is_reported_without_a_panic() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn main(){print(\"hello\");}");
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader); // No reader exists before spawn, so publication cannot race its closure.
    let child = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["run", "--experimental-v2", text(&p)])
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("program output"), "{error}");
    assert!(!error.contains("panicked"));
}

#[test]
fn alias_qualified_constructors_and_patterns_work_through_cli_and_formatter() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        include_str!("../examples/enum-aliases/main.svr"),
    );
    let path = text(&p);
    for args in [
        vec!["check", "--experimental-v2", path],
        vec!["test", "--experimental-v2", path],
        vec!["build", "--experimental-v2", "--emit", "ir", path],
    ] {
        let result = t.cli(&args);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let before = t.cli(&["run", "--experimental-v2", path]);
    assert!(
        before.status.success(),
        "{}",
        String::from_utf8_lossy(&before.stderr)
    );
    assert_eq!(before.stdout, b"7\n");
    let formatted = t.cli(&["fmt", "--experimental-v2", path]);
    assert!(
        formatted.status.success(),
        "{}",
        String::from_utf8_lossy(&formatted.stderr)
    );
    std::fs::write(&p, formatted.stdout).unwrap();
    let after = t.cli(&["run", "--experimental-v2", path]);
    assert!(after.status.success());
    assert_eq!(after.stdout, before.stdout);
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--check", path])
        .status
        .success());
}

#[test]
fn type_only_modules_flow_through_cli_check_test_ir_run_and_format() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        include_str!("../examples/type-modules/main.svr"),
    );
    let path = text(&p);
    for args in [
        vec!["check", "--experimental-v2", path],
        vec![
            "check",
            "--experimental-v2",
            "--format",
            "json",
            "--symbols",
            path,
        ],
        vec!["test", "--experimental-v2", path],
        vec!["build", "--experimental-v2", "--emit", "ir", path],
    ] {
        let out = t.cli(&args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!String::from_utf8_lossy(&out.stdout).contains("\"unavailable\":true"));
    }
    let before = t.cli(&["run", "--experimental-v2", path]);
    assert!(before.status.success());
    assert_eq!(before.stdout, b"7\n");
    let formatted = t.cli(&["fmt", "--experimental-v2", path]);
    assert!(
        formatted.status.success(),
        "{}",
        String::from_utf8_lossy(&formatted.stderr)
    );
    std::fs::write(&p, formatted.stdout).unwrap();
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--check", path])
        .status
        .success());
    let after = t.cli(&["run", "--experimental-v2", path]);
    assert!(after.status.success());
    assert_eq!(after.stdout, before.stdout);
}

#[test]
fn qualified_module_functions_use_the_checked_cli_path_and_root_tests() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        include_str!("../examples/module-functions/main.svr"),
    );
    let path = text(&p);
    for args in [
        vec![
            "check",
            "--experimental-v2",
            "--format",
            "json",
            "--symbols",
            path,
        ],
        vec!["test", "--experimental-v2", path],
        vec!["build", "--experimental-v2", path],
    ] {
        let out = t.cli(&args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!String::from_utf8_lossy(&out.stdout).contains("\"unavailable\":true"));
    }
    let out = t.cli(&["run", "--experimental-v2", path]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"7\n");
}

#[test]
fn module_main_and_test_names_never_replace_root_entry_or_test_discovery() {
    let t = Temp::new();
    let p=t.source("main.svr","mod m{export fn main(){print(7);}fn test_private(n:Int){std::set_exit_code(1);}}fn test_root(){}");
    let path = text(&p);
    let run = t.cli(&["run", "--experimental-v2", path]);
    assert_eq!(run.status.code(), Some(1));
    assert!(run.stdout.is_empty());
    let tests = t.cli(&["test", "--experimental-v2", "--format", "json", path]);
    assert!(
        tests.status.success(),
        "{}",
        String::from_utf8_lossy(&tests.stderr)
    );
    let report = String::from_utf8_lossy(&tests.stdout);
    assert!(report.contains("test_root"));
    assert!(!report.contains("test_private"));
}
