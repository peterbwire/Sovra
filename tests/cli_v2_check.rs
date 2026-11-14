//! Explicit source-only experimental CLI checking; defaults and host effects stay separate.
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
            "sovra-cli-v2-{}-{}",
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
fn explicit_check_accepts_enum_control_programs_with_existing_json_schema() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        include_str!("../examples/control-matches/main.svr"),
    );
    let o = t.cli(&["check", "--experimental-v2", "--format", "json", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("\"schema_version\":1"));
    assert!(s.contains("\"kind\":\"source\""));
    assert!(s.contains("\"success\":true"));
    assert!(s.contains("\"diagnostics\":[]"));
    assert!(!t.cli(&["check", text(&p)]).status.success());
}
#[test]
fn checks_never_execute_print_file_or_exit_operations_and_need_no_main() {
    let t = Temp::new();
    let p=t.source("main.svr","fn main(){std::write_text(\"marker.txt\",\"changed\");print(\"EXECUTED\");std::set_exit_code(7);}");
    let o = t.cli(&["check", "--experimental-v2", text(&p)]);
    assert!(o.status.success());
    assert!(!String::from_utf8_lossy(&o.stdout).contains("EXECUTED"));
    assert!(!t.0.join("marker.txt").exists());
    let p = t.source("helper.svr", "fn helper()->Int{return 7;}");
    assert!(t
        .cli(&["check", "--experimental-v2", text(&p)])
        .status
        .success());
}
#[test]
fn semantic_and_enum_diagnostics_keep_original_file_coordinates() {
    let t = Temp::new();
    for source in [
        "// λ😀\r\nenum E{A}\r\nfn main(){print(missing);}",
        "// λ😀\r\nenum E{A(Missing)} fn main(){}",
    ] {
        let p = t.source("bad.svr", source);
        let o = t.cli(&["check", "--experimental-v2", "--format", "json", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        let s = String::from_utf8(o.stdout).unwrap();
        let start = source
            .find(if source.contains("missing") {
                "missing"
            } else {
                "Missing"
            })
            .unwrap();
        assert!(s.contains(&format!("\"start\":{start},")), "{s}");
        assert!(s.contains(&format!("\"file\":{:?}", text(&p))));
        assert!(!s.contains("\"file\":\"enum:"));
        assert!(s.contains("\"success\":false"));
    }
}
#[test]
fn unsupported_source_missing_function_and_duplicate_function_have_stable_codes() {
    let t = Temp::new();
    for (source, code) in [
        ("module m{fn f(){}}", "E3120"),
        ("enum E{A}", "E3121"),
        ("fn f(){}fn f(){}", "E3008"),
    ] {
        let p = t.source("bad.svr", source);
        let o = t.cli(&["check", "--experimental-v2", "--format", "json", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&o.stdout).contains(code));
    }
}
#[test]
fn unsupported_options_file_extensions_and_duplicates_are_usage_errors() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn main(){}");
    for args in [
        vec![
            "check",
            "--experimental-v2",
            "--symbols",
            "--format",
            "human",
            text(&p),
        ],
        vec!["check", "--experimental-v2", "--locked", text(&p)],
        vec!["check", "--experimental-v2", "--experimental-v2", text(&p)],
        vec![
            "check",
            "--experimental-v2",
            "--format",
            "json",
            "--format",
            "human",
            text(&p),
        ],
        vec!["check", "--experimental-v2"],
        vec![
            "check",
            "--experimental-v2",
            text(&t.source("invalid.txt", "")),
        ],
        vec!["build", "--experimental-v2", "--emit", "js", text(&p)],
    ] {
        assert_eq!(t.cli(&args).status.code(), Some(2), "{args:?}");
    }
    assert!(
        String::from_utf8_lossy(&t.cli(&["check", "--help"]).stdout).contains("--experimental-v2")
    );
}
#[test]
fn bounded_source_io_and_delimited_flag_paths_keep_checked_behavior() {
    let t = Temp::new();
    let p = t.source("--format.svr", "fn main(){}");
    assert!(t
        .cli(&[
            "check",
            "--experimental-v2",
            "--format",
            "json",
            "--",
            text(&p)
        ])
        .status
        .success());
    let p = t.0.join("missing.svr");
    let o = t.cli(&["check", "--experimental-v2", "--format", "json", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("E0001"));
    assert!(s.contains("\"location\":null"));
    let p = t.0.join("large.svr");
    let file = std::fs::File::create(&p).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    let o = t.cli(&["check", "--experimental-v2", "--format", "json", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stdout).contains("E1003"));
}
#[cfg(unix)]
#[test]
fn human_success_and_errors_escape_control_bytes_in_paths() {
    let t = Temp::new();
    let p = t.source("line\n\u{1b}.svr", "fn main(){}");
    let o = t.cli(&["check", "--experimental-v2", text(&p)]);
    assert!(o.status.success());
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(!s.contains('\u{1b}'));
    assert_eq!(s.lines().count(), 1);
}
