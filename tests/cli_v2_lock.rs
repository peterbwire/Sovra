//! Locked v2 compilation verifies captured inputs before all checking/effects.
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
            "sovra-cli-v2-lock-{}-{}",
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

impl Temp {
    fn lock(&self, root: &Path) {
        let o = self.cli(&["lock", "--experimental-v2", text(root)]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
}
#[test]
fn generated_enum_lock_verifies_all_four_commands_without_changing_it() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    t.lock(&root);
    let before = std::fs::read(root.join("sovra.lock")).unwrap();
    let o = t.cli(&["lock", "--experimental-v2", "--check", text(&root)]);
    assert!(o.status.success());
    assert_eq!(o.stdout, b"source lock verified\n");
    let o = t.cli(&[
        "check",
        "--experimental-v2",
        "--locked",
        "--format",
        "json",
        text(&root),
    ]);
    assert!(o.status.success());
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("\"kind\":\"project\"") && s.contains("\"success\":true"));
    let o = t.cli(&["run", "--experimental-v2", "--locked", text(&root)]);
    assert!(o.status.success());
    assert_eq!(o.stdout, b"14\n");
    let o = t.cli(&[
        "test",
        "--experimental-v2",
        "--locked",
        "--format",
        "json",
        text(&root),
    ]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("\"passed\":1"));
    let o = t.cli(&["build", "--experimental-v2", "--locked", text(&root)]);
    assert!(o.status.success());
    assert!(o.stdout.starts_with(b"sovra-ir v2 inspection-only\n"));
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), before);
    assert!(!t.cli(&["run", "--locked", text(&root)]).status.success());
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let status = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["lock", "--experimental-v2", text(&root)])
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert_eq!(status.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&status.stderr).contains("cannot publish lock status"));
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), before);
}
#[test]
fn stale_source_refuses_before_compilation_tests_ir_or_granted_file_effects() {
    let t = Temp::new();
    let source = "fn main(){std::write_text(\"marker\",\"before\");}fn test_a(){}";
    let root = t.package("app", source);
    t.lock(&root);
    let before = std::fs::read(root.join("sovra.lock")).unwrap();
    std::fs::write(
        root.join("src/main.svr"),
        source.replace("before", "after "),
    )
    .unwrap();
    for command in ["check", "test", "run", "build"] {
        let o = if command == "run" {
            t.cli(&[
                command,
                "--experimental-v2",
                "--locked",
                "--allow-files",
                text(&root),
            ])
        } else {
            t.cli(&[command, "--experimental-v2", "--locked", text(&root)])
        };
        assert_eq!(o.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("E4165")
                || String::from_utf8_lossy(&o.stdout).contains("E4165")
        );
        assert!(!t.0.join("marker").exists());
        if command != "test" {
            assert!(o.stdout.is_empty());
        }
    }
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), before);
    t.lock(&root);
    let o = t.cli(&[
        "run",
        "--experimental-v2",
        "--locked",
        "--allow-files",
        text(&root),
    ]);
    assert!(o.status.success());
    assert_eq!(
        std::fs::read_to_string(t.0.join("marker")).unwrap(),
        "after "
    );
}
#[test]
fn missing_malformed_wrong_version_and_oversized_locks_fail_one_json_report() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    let lock = root.join("sovra.lock");
    for data in [
        None,
        Some("bad"),
        Some("{\"format\":\"sovra-lock\",\"version\":2}"),
    ] {
        if let Some(data) = data {
            std::fs::write(&lock, data).unwrap();
        }
        for command in ["check", "test"] {
            let o = t.cli(&[
                command,
                "--experimental-v2",
                "--locked",
                "--format",
                "json",
                text(&root),
            ]);
            assert_eq!(o.status.code(), Some(1));
            assert!(o.stderr.is_empty());
            let s = String::from_utf8(o.stdout).unwrap();
            assert_eq!(s.lines().count(), 1);
            assert!(s.contains("E4165") && s.contains("\"success\":false"));
            if command == "test" {
                assert!(s.contains("\"tests\":[]"));
            }
        }
        if let Some(data) = data {
            assert_eq!(std::fs::read_to_string(&lock).unwrap(), data);
        } else {
            assert!(!lock.exists());
        }
    }
    std::fs::File::create(&lock)
        .unwrap()
        .set_len(sovra::compiler::project::packages::MAX_LOCK_BYTES as u64 + 1)
        .unwrap();
    let o = t.cli(&["run", "--experimental-v2", "--locked", text(&root)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).contains("E4165"));
}
#[test]
fn locked_and_lock_options_reject_duplicates_standalone_sources_and_unrelated_flags() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    let file = t.source("main.svr", "fn main(){}fn test_a(){}");
    for command in ["check", "run", "test", "build"] {
        for args in [
            vec![command, "--experimental-v2", "--locked", text(&file)],
            vec![
                command,
                "--experimental-v2",
                "--locked",
                "--locked",
                text(&root),
            ],
        ] {
            let o = t.cli(&args);
            assert_eq!(o.status.code(), Some(2));
            assert!(o.stdout.is_empty());
        }
    }
    for args in [
        vec!["lock", "--experimental-v2"],
        vec!["lock", "--experimental-v2", text(&file)],
        vec![
            "lock",
            "--experimental-v2",
            "--check",
            "--check",
            text(&root),
        ],
        vec![
            "lock",
            "--experimental-v2",
            "--experimental-v2",
            text(&root),
        ],
        vec!["lock", "--experimental-v2", "--locked", text(&root)],
        vec!["lock", "--experimental-v2", "--format", "json", text(&root)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(2));
        assert!(o.stdout.is_empty());
    }
    let o = t.cli(&["lock", "--experimental-v2", "--help"]);
    assert!(o.status.success());
    assert!(!root.join("sovra.lock").exists());
    let root = t.package("arguments", "fn main(){print(std::arg(0));}");
    let o = t.cli(&["run", "--experimental-v2", text(&root), "--", "--locked"]);
    assert!(o.status.success());
    assert_eq!(o.stdout, b"--locked\n");
}
#[test]
fn verification_never_grants_runtime_hosts_or_changes_cwd_arguments_and_status() {
    let t = Temp::new();
    let root = t.package(
        "app",
        "fn main(){std::write_text(\"relative.txt\",std::arg(0));std::set_exit_code(7);}",
    );
    t.lock(&root);
    let o = t.cli(&[
        "run",
        "--experimental-v2",
        "--locked",
        text(&root),
        "--",
        "payload",
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!t.0.join("relative.txt").exists());
    let o = t.cli(&[
        "run",
        "--experimental-v2",
        "--locked",
        "--allow-files",
        text(&root),
        "--",
        "payload",
    ]);
    assert_eq!(o.status.code(), Some(7));
    assert_eq!(
        std::fs::read_to_string(t.0.join("relative.txt")).unwrap(),
        "payload"
    );
    assert!(!root.join("relative.txt").exists());
}
#[test]
fn symlink_lock_reads_and_publication_refuse_without_touching_other_data() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    t.lock(&root);
    let path = root.join("sovra.lock");
    let before = std::fs::read(&path).unwrap();
    let outside = t.0.join("outside.lock");
    std::fs::rename(&path, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    for args in [
        vec!["lock", "--experimental-v2", text(&root)],
        vec!["lock", "--experimental-v2", "--check", text(&root)],
        vec!["run", "--experimental-v2", "--locked", text(&root)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
    }
    assert_eq!(std::fs::read(&outside).unwrap(), before);
    assert!(std::fs::symlink_metadata(&path)
        .unwrap()
        .file_type()
        .is_symlink());
}
#[test]
fn unsupported_package_shape_refuses_before_lock_generation_or_verification() {
    let t = Temp::new();
    let root = t.package("app", PROGRAM);
    let manifest = std::fs::read_to_string(root.join("sovra.toml")).unwrap();
    std::fs::write(root.join("helper.svr"), "module helper{}").unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        manifest + "[modules]\nhelper=\"helper.svr\"\n",
    )
    .unwrap();
    for args in [
        vec!["lock", "--experimental-v2", text(&root)],
        vec!["run", "--experimental-v2", "--locked", text(&root)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&o.stderr).contains("E3124"));
    }
    assert!(!root.join("sovra.lock").exists());
}
#[test]
fn verified_integrity_does_not_bypass_source_semantics() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){print(unknown);}fn test_a(){}");
    t.lock(&root);
    let before = std::fs::read(root.join("sovra.lock")).unwrap();
    assert!(t
        .cli(&["lock", "--experimental-v2", "--check", text(&root)])
        .status
        .success());
    for command in ["check", "test", "run", "build"] {
        let o = t.cli(&[command, "--experimental-v2", "--locked", text(&root)]);
        assert_eq!(o.status.code(), Some(1));
        let s = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(s.contains("E3001"), "{s}");
    }
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), before);
}
