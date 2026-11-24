//! Locked multi-file native consumers run after project deletion with no Node/PATH.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra native graph λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn example(&self) -> PathBuf {
        for relative in [
            "app/sovra.toml",
            "app/main.svr",
            "helpers/sovra.toml",
            "helpers/main.svr",
            "helpers/values.svr",
        ] {
            let target = self.0.join("project").join(relative);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(
                Path::new("examples/generic-packages").join(relative),
                target,
            )
            .unwrap();
        }
        self.0.join("project/app")
    }
    fn cli(&self, root: &Path, command: &str, flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg(command)
            .arg("--experimental-generics")
            .args(flags)
            .arg(root)
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
#[test]
fn locked_native_graph_runs_without_inputs_node_or_filesystem_source_fallback() {
    let f = Fixture::new();
    let root = f.example();
    assert!(f.cli(&root, "lock", &[]).status.success());
    let lock = std::fs::read(root.join("sovra.lock")).unwrap();
    let output = f.0.join("application");
    let built = f.cli(
        &root,
        "build",
        &[
            "--locked",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(
        built.status.success(),
        "{} {}",
        text(&built.stdout),
        text(&built.stderr)
    );
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), lock);
    std::fs::remove_dir_all(f.0.join("project")).unwrap();
    let cwd = f.0.join("empty");
    std::fs::create_dir_all(cwd.join("@sovra-image/p1")).unwrap();
    std::fs::write(cwd.join("@sovra-image/p1/values.svr"), "not actual source").unwrap();
    let out = Command::new(&output)
        .current_dir(&cwd)
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
    let out = Command::new(&output)
        .arg("unexpected")
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("do not accept arguments"));
    let count = std::fs::read_dir(&f.0)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".sovra-exe-stage-")
        })
        .count();
    assert_eq!(count, 0);
}
#[test]
fn stale_graph_locks_and_invalid_unused_sources_refuse_before_output_creation() {
    let f = Fixture::new();
    let root = f.example();
    assert!(f.cli(&root, "lock", &[]).status.success());
    let lock = std::fs::read(root.join("sovra.lock")).unwrap();
    let module = f.0.join("project/helpers/values.svr");
    std::fs::write(&module, "mod values {export fn broken(){missing();}}").unwrap();
    let output = f.0.join("absent/application");
    let out = f.cli(
        &root,
        "build",
        &[
            "--locked",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E4165"));
    assert!(!output.parent().unwrap().exists());
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), lock);
    let out = f.cli(
        &root,
        "build",
        &["--emit", "exe", "--output", output.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E3123"));
    assert!(!output.parent().unwrap().exists());
}

#[test]
fn explicit_collection_package_cli_and_native_launch_preserve_order_owners_and_locks() {
    let f = Fixture::new();
    let root = f.example();
    let source = std::fs::read_to_string("examples/intrinsic-collections/app/main.svr").unwrap();
    std::fs::write(root.join("main.svr"), source).unwrap();
    assert!(f.cli(&root, "lock", &[]).status.success());
    let lock = std::fs::read(root.join("sovra.lock")).unwrap();
    assert!(!f.cli(&root, "check", &["--locked"]).status.success());
    for command in ["check", "run", "test"] {
        let result = f.cli(&root, command, &["--experimental-collections", "--locked"]);
        assert!(
            result.status.success(),
            "{command}: {} {}",
            text(&result.stdout),
            text(&result.stderr)
        );
    }
    let json = f.cli(
        &root,
        "test",
        &["--experimental-collections", "--locked", "--format", "json"],
    );
    assert!(json.status.success(), "{}", text(&json.stderr));
    assert!(text(&json.stdout).contains("test_collections"));
    let output = f.0.join("collections");
    let built = f.cli(
        &root,
        "build",
        &[
            "--experimental-collections",
            "--locked",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(
        built.status.success(),
        "{} {}",
        text(&built.stdout),
        text(&built.stderr)
    );
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), lock);
    std::fs::remove_dir_all(f.0.join("project")).unwrap();
    let cwd = f.0.join("empty");
    std::fs::create_dir(&cwd).unwrap();
    let result = Command::new(&output)
        .current_dir(&cwd)
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", text(&result.stderr));
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    let result = Command::new(output)
        .arg("unexpected")
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(text(&result.stderr).contains("do not accept arguments"));
}
#[test]
fn collection_cli_rejects_duplicates_standalone_inputs_and_stale_locks_before_build() {
    let f = Fixture::new();
    let root = f.example();
    assert!(f.cli(&root, "lock", &[]).status.success());
    let duplicate = f.cli(
        &root,
        "check",
        &["--experimental-collections", "--experimental-collections"],
    );
    assert_eq!(duplicate.status.code(), Some(2));
    let file = root.join("main.svr");
    let standalone = f.cli(&file, "check", &["--experimental-collections"]);
    assert_eq!(standalone.status.code(), Some(2));
    std::fs::write(&file, "fn main(){let m:std::Map<Int,Int>=std::map_new();}").unwrap();
    let output = f.0.join("should-not-exist");
    let built = f.cli(
        &root,
        "build",
        &[
            "--experimental-collections",
            "--locked",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(!built.status.success());
    assert!(!output.exists());
}

#[test]
fn single_entry_collection_packages_use_the_collection_native_profile() {
    let f = Fixture::new();
    let root = f.0.join("single");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname=\"single\"\nentry=\"main.svr\"\n",
    )
    .unwrap();
    std::fs::write(root.join("main.svr"),"fn main(){let m:std::Map<Int,Int>=std::map_new();let full=std::map_insert(m,7,9);if std::to_string(full)!=\"std::Map { 7: 9 }\"{let bad=1/0;}}fn test_map(){main();}").unwrap();
    for command in ["check", "run", "test"] {
        let result = f.cli(&root, command, &["--experimental-collections"]);
        assert!(
            result.status.success(),
            "{command}: {}",
            text(&result.stderr)
        );
    }
    let output = f.0.join("single-native");
    let result = f.cli(
        &root,
        "build",
        &[
            "--experimental-collections",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(result.status.success(), "{}", text(&result.stderr));
    std::fs::remove_dir_all(root).unwrap();
    assert!(Command::new(output)
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap()
        .status
        .success());
}

#[test]
fn collection_cli_has_no_js_ir_symbols_or_output_overwrite_fallback() {
    let f = Fixture::new();
    let root = f.example();
    std::fs::write(
        root.join("main.svr"),
        "fn main(){let m:std::Map<Int,Int>=std::map_new();}",
    )
    .unwrap();
    let output = f.0.join("protected");
    for emit in ["js", "ir"] {
        let result = f.cli(
            &root,
            "build",
            &[
                "--experimental-collections",
                "--emit",
                emit,
                "--output",
                output.to_str().unwrap(),
            ],
        );
        assert_eq!(result.status.code(), Some(2));
        assert!(!output.exists());
    }
    let symbols = f.cli(
        &root,
        "check",
        &[
            "--experimental-collections",
            "--symbols",
            "--format",
            "json",
        ],
    );
    assert_eq!(symbols.status.code(), Some(2));
    std::fs::write(&output, b"keep original").unwrap();
    let result = f.cli(
        &root,
        "build",
        &[
            "--experimental-collections",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(!result.status.success());
    assert_eq!(std::fs::read(&output).unwrap(), b"keep original");
}

#[test]
fn published_collection_package_checks_runs_and_tests_without_generated_outputs() {
    let f = Fixture::new();
    let root = Path::new("examples/intrinsic-collections/app");
    for command in ["check", "run", "test"] {
        let result = f.cli(root, command, &["--experimental-collections"]);
        assert!(
            result.status.success(),
            "{command}: {} {}",
            text(&result.stdout),
            text(&result.stderr)
        );
    }
    assert!(!root.join("sovra.lock").exists());
}
