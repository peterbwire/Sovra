//! Subprocess qualification for explicit standalone generic command routing.
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sovra-generic-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.svr"), source).unwrap();
        Self(dir)
    }
    fn run(&self, command: &str, flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg(command)
            .args(flags)
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
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}
const VALID: &str = include_str!("../docs/examples/generic-source.svr");
#[test]
fn standalone_check_and_run_accept_complete_generic_program() {
    let f = Fixture::new(VALID);
    let checked = f.run("check", &["--experimental-generics"]);
    assert!(checked.status.success(), "{}", text(&checked.stderr));
    assert!(text(&checked.stdout).contains("checked experimental generic source"));
    let ran = f.run("run", &["--experimental-generics"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(ran.stdout.is_empty());
    assert!(ran.stderr.is_empty());
}
#[test]
fn json_check_reports_success_without_execution() {
    let f = Fixture::new("fn main(){let fail=1/0;}");
    let out = f.run("check", &["--experimental-generics", "--format", "json"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert!(text(&out.stdout).contains("\"success\":true,\"diagnostics\":[]"));
    let ran = f.run("run", &["--experimental-generics"]);
    assert_eq!(ran.status.code(), Some(1));
    assert!(text(&ran.stderr).contains("E3125"));
}
#[test]
fn json_compile_error_retains_original_file_coordinates() {
    let f = Fixture::new("// λ\n\nfn main(){}\nfn unused(){let bad:Bool=1;}");
    let out = f.run("check", &["--experimental-generics", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());
    let report = text(&out.stdout);
    assert!(report.contains("\"schema_version\":1"));
    assert!(report.contains("\"success\":false"));
    assert!(report.contains("\"code\":\"E3123\""));
    assert!(report.contains("\"line\":3"), "{report}");
    assert!(report.contains("main.svr"));
}
#[test]
fn global_compile_errors_have_no_invented_location() {
    let f = Fixture::new("fn other(){}");
    let out = f.run("check", &["--experimental-generics", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("\"location\":null"));
}
#[test]
fn runtime_error_points_at_original_helper_line() {
    let f = Fixture::new(
        "// header\nfn main(){let x=helper();}\n// helper\nfn helper()->Int{return 1/0;}",
    );
    let out = f.run("run", &["--experimental-generics"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let error = text(&out.stderr);
    assert!(error.contains("E3125"));
    assert!(error.contains("main.svr:4:"), "{error}");
}
#[test]
fn unit_entry_is_required_only_for_run() {
    let f = Fixture::new("fn main()->Int{return 42;}");
    assert!(f
        .run("check", &["--experimental-generics"])
        .status
        .success());
    let out = f.run("run", &["--experimental-generics"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("Unit main"));
}
#[test]
fn finite_instruction_or_work_policy_stops_infinite_loop() {
    let f = Fixture::new("fn main(){while true{}}");
    let out = f.run("run", &["--experimental-generics"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E3125"));
}
#[test]
fn host_calls_remain_unavailable_and_publish_no_output() {
    for source in [
        "fn main(){print(42);}",
        "fn main(){let value=std::readLine();}",
    ] {
        let f = Fixture::new(source);
        let out = f.run("run", &["--experimental-generics"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn mixed_modes_permissions_duplicates_and_extra_flags_are_usage_errors() {
    let f = Fixture::new(VALID);
    for flags in [
        vec!["--experimental-generics", "--experimental-v2"],
        vec!["--experimental-generics", "--experimental-generics"],
        vec!["--experimental-generics", "--locked"],
        vec!["--experimental-generics", "--symbols"],
        vec!["--experimental-generics", "--allow-stdin"],
        vec!["--experimental-generics", "--allow-files"],
        vec!["--experimental-generics", "--format", "json"],
    ] {
        let out = f.run("run", &flags);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{flags:?}: {}",
            text(&out.stderr)
        );
    }
    for flags in [
        vec!["--experimental-generics", "--format", "xml"],
        vec![
            "--experimental-generics",
            "--format",
            "json",
            "--format",
            "json",
        ],
    ] {
        assert_eq!(f.run("check", &flags).status.code(), Some(2));
    }
}
#[test]
fn build_requires_explicit_backend_and_generic_test_has_no_legacy_fallback() {
    let f = Fixture::new(VALID);
    let out = f.run("build", &["--experimental-generics"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let out = f.run("test", &["--experimental-generics", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("E5002"));
    assert!(out.stderr.is_empty());
}

#[test]
fn delimiter_keeps_program_arguments_and_flag_names_as_data() {
    let f = Fixture::new(VALID);
    let out = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["run", "--experimental-generics"])
        .arg(f.0.join("main.svr"))
        .args(["--", "--help"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let out = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["run", "--", "--experimental-generics"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!text(&out.stderr).contains("missing --experimental-generics"));
}
#[test]
fn directories_and_non_source_inputs_are_refused() {
    let f = Fixture::new(VALID);
    std::fs::create_dir(f.0.join("directory.svr")).unwrap();
    for path in [f.0.clone(), f.0.join("directory.svr"), f.0.join("main.txt")] {
        let out = Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(["check", "--experimental-generics"])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(if path.is_dir() { 1 } else { 2 }));
    }
}
#[test]
fn bounded_source_reads_and_invalid_utf8_produce_json_errors() {
    let f = Fixture::new("");
    for bytes in [vec![b' '; 4 * 1024 * 1024 + 1], vec![0xff]] {
        std::fs::write(f.0.join("main.svr"), bytes).unwrap();
        let out = f.run("check", &["--experimental-generics", "--format", "json"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stderr.is_empty());
        assert!(text(&out.stdout).contains("\"success\":false"));
    }
}
#[test]
fn missing_input_and_missing_file_have_distinct_exit_codes() {
    let missing = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["check", "--experimental-generics"])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    let f = Fixture::new(VALID);
    std::fs::remove_file(f.0.join("main.svr")).unwrap();
    let missing = f.run("check", &["--experimental-generics", "--format", "json"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(text(&missing.stdout).contains("E0001"));
}
#[test]
fn help_documents_the_explicit_mode() {
    for command in ["check", "run"] {
        let out = Command::new(env!("CARGO_BIN_EXE_svr"))
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(text(&out.stdout).contains("--experimental-generics"));
    }
}

#[test]
fn default_and_existing_v2_modes_do_not_infer_generic_selection() {
    let f = Fixture::new(VALID);
    for flags in [vec![], vec!["--experimental-v2"]] {
        let out = f.run("check", &flags);
        assert_eq!(out.status.code(), Some(1));
        assert!(!text(&out.stderr).contains("E3123"));
    }
}
