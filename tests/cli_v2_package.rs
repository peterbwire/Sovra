//! End-to-end root-only package compilation over the guarded source reader.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
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
            "sovra-cli-v2-package-{}-{}",
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

impl Temp {
    fn package(&self, name: &str, source: &str) -> PathBuf {
        let root = self.0.join(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname=\"app\"\nentry=\"src/main.svr\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src/main.svr"), source).unwrap();
        root
    }
}
const PROGRAM: &str = include_str!("../examples/enum-package/src/main.svr");
#[test]
fn all_four_commands_compile_the_entry_instead_of_scanning_application_metadata() {
    let t = Temp::new();
    let root = t.package("app λ", PROGRAM);
    let check = t.cli(&[
        "check",
        "--experimental-v2",
        "--format",
        "json",
        text(&root),
    ]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let json = String::from_utf8(check.stdout).unwrap();
    assert_eq!(json.lines().count(), 1);
    assert!(
        json.contains("\"schema_version\":1")
            && json.contains("\"kind\":\"project\"")
            && json.contains("\"success\":true")
    );
    let run = t.cli(&["run", "--experimental-v2", text(&root)]);
    assert!(run.status.success());
    assert_eq!(run.stdout, b"14\n");
    let tests = t.cli(&["test", "--experimental-v2", "--format", "json", text(&root)]);
    assert!(tests.status.success());
    let json = String::from_utf8(tests.stdout).unwrap();
    assert!(
        json.contains("\"kind\":\"package\"")
            && json.contains("\"passed\":1")
            && json.contains("\"output\":[\"ok\"]")
    );
    let package_ir = t.cli(&["build", "--experimental-v2", "--emit", "ir", text(&root)]);
    assert!(package_ir.status.success());
    let source_ir = t.cli(&[
        "build",
        "--experimental-v2",
        text(&root.join("src/main.svr")),
    ]);
    assert!(source_ir.status.success());
    assert_eq!(package_ir.stdout, source_ir.stdout);
    assert!(!t.cli(&["run", text(&root)]).status.success());
}
#[test]
fn directories_named_svr_are_packages_and_helper_only_entries_need_main_only_for_run() {
    let t = Temp::new();
    let root = t.package("project.svr", "fn test_only(){}");
    for command in ["check", "test", "build"] {
        let o = t.cli(&[command, "--experimental-v2", text(&root)]);
        assert!(
            o.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let o = t.cli(&["run", "--experimental-v2", text(&root)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).contains("requires a parameterless Unit main"));
}
#[test]
fn original_unicode_crlf_spans_and_entry_identity_survive_check_and_test_reports() {
    let t = Temp::new();
    let source = "// λ😀\r\nfn main(){}\r\nfn test_bad(){print(missing);}";
    let root = t.package("app", source);
    let entry = std::fs::canonicalize(root.join("src/main.svr")).unwrap();
    for command in ["check", "test"] {
        let o = t.cli(&[
            command,
            "--experimental-v2",
            "--format",
            "json",
            text(&root),
        ]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stderr.is_empty());
        let json = String::from_utf8(o.stdout).unwrap();
        assert_eq!(json.lines().count(), 1);
        assert!(json.contains("E3001"));
        assert!(json.contains(&format!("\"start\":{}", source.find("missing").unwrap())));
        assert!(json.contains(&format!("{:?}", text(&entry))), "{json}");
        assert!(!json.contains("enum:"));
        if command == "test" {
            assert!(json.contains("\"kind\":\"package\"") && json.contains("\"tests\":[]"));
        }
    }
}
#[test]
fn dependencies_and_registered_modules_refuse_without_running_the_entry() {
    let t = Temp::new();
    let root = t.package(
        "app",
        "fn main(){std::write_text(\"marker\",\"bad\");}fn test_a(){}",
    );
    t.package("dep", "fn main(){}");
    std::fs::write(root.join("helper.svr"), "module helper{}").unwrap();
    for suffix in [
        "[dependencies.dep]\npath=\"../dep\"\n",
        "[modules]\nhelper=\"helper.svr\"\n",
    ] {
        std::fs::write(
            root.join("sovra.toml"),
            format!("[project]\nname=\"app\"\nentry=\"src/main.svr\"\n{suffix}"),
        )
        .unwrap();
        for command in ["check", "test", "build", "run"] {
            let o = if command == "run" {
                t.cli(&[command, "--experimental-v2", "--allow-files", text(&root)])
            } else {
                t.cli(&[command, "--experimental-v2", text(&root)])
            };
            assert_eq!(o.status.code(), Some(1), "{command}: {o:?}");
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            assert!(combined.contains("E3124"), "{combined}");
            assert!(!t.0.join("marker").exists());
        }
    }
}
#[test]
fn malformed_lock_refuses_locked_mode_and_is_never_modified() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    std::fs::write(root.join("sovra.lock"), "unchanged").unwrap();
    for command in ["check", "run", "test", "build"] {
        let o = t.cli(&[command, "--experimental-v2", "--locked", text(&root)]);
        assert_eq!(o.status.code(), Some(1));
        if command == "test" {
            assert!(String::from_utf8_lossy(&o.stdout).contains("E4165"));
            assert!(String::from_utf8_lossy(&o.stdout).contains("0 total"));
        } else {
            assert!(o.stdout.is_empty());
        }
    }
    let o = t.cli(&["run", "--experimental-v2", text(&root)]);
    assert!(o.status.success());
    assert_eq!(
        std::fs::read_to_string(root.join("sovra.lock")).unwrap(),
        "unchanged"
    );
}
#[test]
fn package_selection_does_not_change_cwd_or_silently_grant_file_authority() {
    let t = Temp::new();
    let root=t.package("app","fn main(){let result=std::write_text(\"relative.txt\",std::arg(0));print(result.ok);std::set_exit_code(7);}");
    let o = t.cli(&["run", "--experimental-v2", text(&root), "--", "payload"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!t.0.join("relative.txt").exists());
    let o = t.cli(&[
        "run",
        "--experimental-v2",
        "--allow-files",
        text(&root),
        "--",
        "payload",
    ]);
    assert_eq!(o.status.code(), Some(7));
    assert_eq!(o.stdout, b"true\n");
    assert_eq!(
        std::fs::read_to_string(t.0.join("relative.txt")).unwrap(),
        "payload"
    );
    assert!(!root.join("relative.txt").exists());
}
#[test]
fn missing_manifest_and_invalid_utf8_are_package_configuration_reports() {
    let t = Temp::new();
    let empty = t.0.join("empty");
    std::fs::create_dir(&empty).unwrap();
    let root = t.package("app", "fn main(){}");
    std::fs::write(root.join("src/main.svr"), [0xff]).unwrap();
    for path in [&empty, &root] {
        for command in ["check", "test"] {
            let o = t.cli(&[command, "--experimental-v2", "--format", "json", text(path)]);
            assert_eq!(o.status.code(), Some(1));
            assert!(o.stderr.is_empty());
            let json = String::from_utf8(o.stdout).unwrap();
            assert_eq!(json.lines().count(), 1);
            assert!(json.contains("\"success\":false"));
            if command == "check" {
                assert!(json.contains("\"kind\":\"project\""));
            } else {
                assert!(json.contains("\"kind\":\"package\"") && json.contains("\"tests\":[]"));
            }
        }
    }
}
#[test]
fn capture_errors_keep_manifest_provenance_and_unsafe_entries_never_execute() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    t.source("outside.svr", "fn main(){}");
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname=\"app\"\nentry=\"../outside.svr\"\n",
    )
    .unwrap();
    let o = t.cli(&[
        "check",
        "--experimental-v2",
        "--format",
        "json",
        text(&root),
    ]);
    assert_eq!(o.status.code(), Some(1));
    let json = String::from_utf8(o.stdout).unwrap();
    assert!(json.contains("sovra.toml"));
    assert!(!json.contains("\"success\":true"));
    let o = t.cli(&["run", "--experimental-v2", text(&root)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
}
#[test]
fn unregistered_files_are_not_executed_and_ir_does_not_run_package_main() {
    let t = Temp::new();
    let root = t.package(
        "app",
        "fn main(){std::write_text(\"marker\",\"bad\");}fn test_only(){}",
    );
    std::fs::write(root.join("ignored.svr"), [0xff]).unwrap();
    for command in ["check", "build"] {
        let o = t.cli(&[command, "--experimental-v2", text(&root)]);
        assert!(o.status.success());
        assert!(!t.0.join("marker").exists());
    }
    // Main is not called, but whole-program missing-host preflight still rejects tests.
    let o = t.cli(&["test", "--experimental-v2", "--format", "json", text(&root)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!t.0.join("marker").exists());
}
