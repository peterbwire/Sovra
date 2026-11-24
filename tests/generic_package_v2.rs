//! Guarded generic root packages, source locks and source-owned command diagnostics.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::project::packages;
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "sovra generic package λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(
            p.join("sovra.toml"),
            "[project]\nname=\"generic\"\nentry=\"src/main.svr\"\n",
        )
        .unwrap();
        std::fs::write(p.join("src/main.svr"), source).unwrap();
        Self(p)
    }
    fn run(&self, command: &str, flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg(command)
            .arg("--experimental-generics")
            .args(flags)
            .arg(&self.0)
            .output()
            .unwrap()
    }
    fn source(&self, source: &str) {
        std::fs::write(self.0.join("src/main.svr"), source).unwrap();
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
fn package_checks_and_executes_only_its_captured_entry() {
    let f = Fixture::new(VALID);
    std::fs::write(f.0.join("unused.svr"), "invalid").unwrap();
    let out = f.run("check", &["--format", "json"]);
    assert!(
        out.status.success(),
        "{} {}",
        text(&out.stdout),
        text(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    assert!(text(&out.stdout).contains("\"kind\":\"project\",\"success\":true"));
    let out = f.run("run", &[]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
    assert!(!f.0.join("sovra.lock").exists());
}
#[test]
fn source_locks_generate_verify_and_guard_check_and_run_without_rewriting() {
    let f = Fixture::new(VALID);
    let out = f.run("lock", &[]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let original = std::fs::read(f.0.join("sovra.lock")).unwrap();
    assert!(f.run("lock", &["--check"]).status.success());
    assert!(f
        .run("check", &["--locked", "--format", "json"])
        .status
        .success());
    assert!(f.run("run", &["--locked"]).status.success());
    assert_eq!(std::fs::read(f.0.join("sovra.lock")).unwrap(), original);
    f.source(&format!("{VALID}\n// changed"));
    for command in ["check", "run"] {
        let out = f.run(command, &["--locked"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(text(&out.stderr).contains("E4165"));
    }
    assert_eq!(std::fs::read(f.0.join("sovra.lock")).unwrap(), original);
    assert!(f.run("run", &[]).status.success());
}
#[test]
fn missing_and_malformed_locks_fail_before_execution_without_updating() {
    let f = Fixture::new("fn main(){let bad=1/0;}");
    for contents in [None, Some("not json"), Some("{}")] {
        if let Some(contents) = contents {
            std::fs::write(f.0.join("sovra.lock"), contents).unwrap();
        }
        let out = f.run("check", &["--locked", "--format", "json"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stderr.is_empty());
        let report = text(&out.stdout);
        assert!(report.contains("E4165"));
        assert!(!report.contains("E3123"));
        assert!(!report.contains("E3125"));
        if let Some(contents) = contents {
            assert_eq!(
                std::fs::read_to_string(f.0.join("sovra.lock")).unwrap(),
                contents
            );
        } else {
            assert!(!f.0.join("sovra.lock").exists());
        }
    }
}
#[test]
fn canonical_entry_owns_compile_and_runtime_error_locations() {
    let f = Fixture::new("// λ\nfn main(){}\nfn unused(){let bad:Bool=1;}");
    let entry = std::fs::canonicalize(f.0.join("src/main.svr")).unwrap();
    let out = f.run("check", &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let report = text(&out.stdout);
    assert!(report.contains("\"kind\":\"project\""));
    assert!(report.contains(entry.to_str().unwrap()));
    assert!(report.contains("\"line\":2"));
    assert!(report.contains("E3123"));
    f.source("// header\nfn main(){let bad=1/0;}");
    let out = f.run("run", &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains(&format!("{}:2:", entry.display())));
    assert!(text(&out.stderr).contains("E3125"));
}
#[test]
fn missing_entry_has_null_location_in_project_json() {
    let f = Fixture::new("fn other(){}");
    let out = f.run("check", &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("\"location\":null"));
}
#[test]
fn fingerprints_do_not_authorize_invalid_unused_bodies() {
    let f = Fixture::new("fn main(){} fn unused(){missing();}");
    assert!(f.run("lock", &[]).status.success());
    let out = f.run("check", &["--locked", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("E3123"));
    assert!(!text(&out.stdout).contains("E4165"));
}
#[test]
fn manifest_changes_stale_the_lock_even_when_entry_is_unchanged() {
    let f = Fixture::new(VALID);
    assert!(f.run("lock", &[]).status.success());
    let mut manifest = std::fs::read_to_string(f.0.join("sovra.toml")).unwrap();
    manifest.push_str("# changed\n");
    std::fs::write(f.0.join("sovra.toml"), manifest).unwrap();
    assert_eq!(f.run("run", &["--locked"]).status.code(), Some(1));
}
#[test]
fn check_run_and_lock_capture_registered_graphs_without_entry_only_fallback() {
    for registration in [
        "[modules]\nextra=\"extra.svr\"\n",
        "[dependencies.lib]\npath=\"lib\"\n",
    ] {
        let f = Fixture::new(VALID);
        std::fs::create_dir(f.0.join("lib")).unwrap();
        std::fs::write(
            f.0.join("lib/sovra.toml"),
            "[project]\nname=\"lib\"\nentry=\"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(f.0.join("lib/main.svr"), "fn helper(){}").unwrap();
        std::fs::write(f.0.join("extra.svr"), "mod extra {}").unwrap();
        let manifest = std::fs::read_to_string(f.0.join("sovra.toml")).unwrap();
        std::fs::write(f.0.join("sovra.toml"), format!("{manifest}{registration}")).unwrap();
        for command in ["check", "run", "lock"] {
            let out = f.run(command, &[]);
            assert!(out.status.success(), "{}", text(&out.stderr));
        }
        let lock = std::fs::read(f.0.join("sovra.lock")).unwrap();
        let (path, bad) = if registration.starts_with("[modules]") {
            ("extra.svr", "mod extra {fn unused(){missing();}}")
        } else {
            ("lib/main.svr", "fn unused(){missing();}")
        };
        std::fs::write(f.0.join(path), bad).unwrap();
        for command in ["check", "run"] {
            let out = f.run(command, &[]);
            assert_eq!(out.status.code(), Some(1));
            assert!(text(&out.stderr).contains("E3123"));
            assert!(text(&out.stderr).contains(path));
        }
        let out = f.run("lock", &["--check"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(text(&out.stderr).contains("E4165"));
        assert_eq!(std::fs::read(f.0.join("sovra.lock")).unwrap(), lock);
        let out = f.run("test", &[]);
        assert_eq!(out.status.code(), Some(1));
        assert!(text(&out.stdout).contains("E3123"));
    }
}

#[test]
fn imports_and_modules_in_the_entry_remain_explicitly_unavailable() {
    for source in [
        "import lib;fn main(){}",
        "mod local{fn helper(){}}fn main(){}",
    ] {
        let f = Fixture::new(source);
        assert_eq!(f.run("check", &["--format", "json"]).status.code(), Some(1));
    }
}
#[test]
fn symlink_entry_and_lock_cannot_select_unowned_inputs() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new(VALID);
    let outside = Fixture::new(VALID);
    std::fs::remove_file(f.0.join("src/main.svr")).unwrap();
    symlink(outside.0.join("src/main.svr"), f.0.join("src/main.svr")).unwrap();
    assert_eq!(f.run("check", &[]).status.code(), Some(1));
    let f = Fixture::new(VALID);
    std::fs::write(outside.0.join("lock"), "sentinel").unwrap();
    symlink(outside.0.join("lock"), f.0.join("sovra.lock")).unwrap();
    assert_eq!(f.run("check", &["--locked"]).status.code(), Some(1));
    assert_eq!(
        std::fs::read_to_string(outside.0.join("lock")).unwrap(),
        "sentinel"
    );
}
#[test]
fn reduced_capture_limit_is_exact_and_zero_never_falls_back() {
    let source = "fn main(){}";
    let f = Fixture::new(source);
    assert_eq!(
        packages::capture_root_source_with_source_limit(&f.0, source.len())
            .unwrap()
            .source(),
        source
    );
    for cap in [0, source.len() - 1] {
        assert!(packages::capture_root_source_with_source_limit(&f.0, cap).is_err());
    }
    assert_eq!(
        packages::capture_root_source(&f.0).unwrap().source(),
        source
    );
}
#[test]
fn generic_package_byte_budget_is_applied_before_entry_copy() {
    let f = Fixture::new(&" ".repeat(4 * 1024 * 1024 + 1));
    let out = f.run("check", &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());
    assert!(text(&out.stdout).contains("E1003"));
}
#[test]
fn strict_lock_and_locked_flag_grammar_preserves_existing_modes() {
    let f = Fixture::new(VALID);
    for flags in [
        vec!["--experimental-generics"],
        vec!["--experimental-v2"],
        vec!["--check", "--check"],
        vec!["--locked"],
        vec!["--format", "json"],
    ] {
        assert_eq!(f.run("lock", &flags).status.code(), Some(2));
    }
    assert_eq!(
        f.run("check", &["--locked", "--locked"]).status.code(),
        Some(2)
    );
    let help = f.run("lock", &["--help"]);
    assert!(help.status.success());
    assert!(text(&help.stdout).contains("lock --experimental-generics"));
}
#[test]
fn native_build_verifies_lock_before_creating_output_parent() {
    let f = Fixture::new(VALID);
    let output = f.0.join("new/app");
    let out = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args([
            "build",
            "--experimental-generics",
            "--locked",
            "--emit",
            "exe",
            "--output",
        ])
        .arg(&output)
        .arg(&f.0)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!output.parent().unwrap().exists());
}

#[test]
fn locked_generic_package_build_runs_after_its_inputs_are_removed() {
    let f = Fixture::new(VALID);
    assert!(f.run("lock", &[]).status.success());
    let output = f.0.join("application");
    let built = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args([
            "build",
            "--experimental-generics",
            "--locked",
            "--emit",
            "exe",
            "--output",
        ])
        .arg(&output)
        .arg(&f.0)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert!(built.stdout.is_empty());
    std::fs::remove_dir_all(f.0.join("src")).unwrap();
    std::fs::remove_file(f.0.join("sovra.toml")).unwrap();
    std::fs::remove_file(f.0.join("sovra.lock")).unwrap();
    let empty = f.0.join("empty-path");
    std::fs::create_dir(&empty).unwrap();
    let ran = Command::new(&output)
        .current_dir(&empty)
        .env("PATH", &empty)
        .output()
        .unwrap();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(ran.stdout.is_empty());
    assert!(ran.stderr.is_empty());
}

#[test]
fn generic_suite_locked_package_uses_test_roots_and_preserves_lock() {
    let f=Fixture::new("fn id<T>(x:T)->T{return x;}fn main(){let bad=1/0;}fn test_a(){let x=id::<Int>(7);}fn test_b(){let x=id::<String>(\"ok\");}");
    assert!(f.run("lock", &[]).status.success());
    let lock = std::fs::read(f.0.join("sovra.lock")).unwrap();
    let out = f.run("test", &["--locked", "--format", "json"]);
    assert!(
        out.status.success(),
        "{} {}",
        text(&out.stdout),
        text(&out.stderr)
    );
    let json = text(&out.stdout);
    assert!(json.contains("\"kind\":\"package\""));
    assert!(json.contains("\"passed\":2,\"failed\":0,\"total\":2"));
    assert!(out.stderr.is_empty());
    assert_eq!(std::fs::read(f.0.join("sovra.lock")).unwrap(), lock);
    f.source("fn test_a(){let bad=1/0;}");
    let out = f.run("test", &["--locked", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let json = text(&out.stdout);
    assert!(json.contains("E4165"));
    assert!(json.contains("\"tests\":[]"));
    assert!(json.contains("\"schema_version\":1"));
    assert!(out.stderr.is_empty());
    assert_eq!(std::fs::read(f.0.join("sovra.lock")).unwrap(), lock);
}
#[test]
fn generic_suite_reports_failures_continues_and_refuses_incompatible_flags() {
    let f = Fixture::new("fn test_bad(){let bad=1/0;}fn test_good(){}");
    let out = f.run("test", &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let json = text(&out.stdout);
    assert!(json.contains("\"passed\":1,\"failed\":1,\"total\":2"));
    assert!(out.stderr.is_empty());
    for flags in [
        &["--symbols"][..],
        &["--emit", "exe"],
        &["--experimental-v2"],
        &["--format", "json", "--format", "human"],
    ] {
        assert_eq!(f.run("test", flags).status.code(), Some(2));
    }
    std::fs::remove_file(f.0.join("src/main.svr")).unwrap();
    let out = f.run("test", &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("\"tests\":[]"));
    assert!(out.stderr.is_empty());
}
