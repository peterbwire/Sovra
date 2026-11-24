//! Native generic linking, independent runtime execution and guarded publication.
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
            "sovra generic native {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.svr"), source).unwrap();
        Self(dir)
    }
    fn output(&self) -> PathBuf {
        self.0.join(if cfg!(windows) { "app.exe" } else { "app" })
    }
    fn cli(&self, options: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(["build", "--experimental-generics"])
            .args(options)
            .arg(self.0.join("main.svr"))
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn build(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args([
                "build",
                "--experimental-generics",
                "--emit",
                "exe",
                "--output",
            ])
            .arg(self.output())
            .arg(self.0.join("main.svr"))
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn linked_executable_runs_with_source_removed_and_no_tool_on_path() {
    let f = Fixture::new(include_str!("../docs/examples/generic-source.svr"));
    let built = f.build();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(built.stdout.is_empty());
    assert!(f.output().is_file());
    std::fs::remove_file(f.0.join("main.svr")).unwrap();
    let empty = f.0.join("empty-path");
    std::fs::create_dir(&empty).unwrap();
    let ran = Command::new(f.output())
        .current_dir(&empty)
        .env("PATH", &empty)
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(ran.stdout.is_empty());
    assert!(ran.stderr.is_empty());
    let rejected = Command::new(f.output())
        .arg("unavailable")
        .env("PATH", &empty)
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("do not accept arguments"));
    assert!(!std::fs::read_dir(&f.0).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".sovra-exe-stage-")));
}
#[test]
fn unsupported_emit_missing_options_and_incompatible_modes_do_not_publish() {
    let f = Fixture::new("fn main(){}");
    for options in [
        vec![],
        vec!["--emit", "exe"],
        vec!["--output", "app.exe"],
        vec!["--emit", "ir", "--output", "app.exe"],
        vec!["--emit", "js", "--output", "app.exe"],
        vec!["--emit", "exe", "--emit", "exe", "--output", "app.exe"],
        vec![
            "--emit",
            "exe",
            "--output",
            "app.exe",
            "--output",
            "again.exe",
        ],
        vec!["--emit", "exe", "--output", "app.exe", "--allow-files"],
        vec!["--emit", "exe", "--output", "app.exe", "--experimental-v2"],
        vec!["--emit", "exe", "--output", "app.exe", "--format", "json"],
    ] {
        let out = f.cli(&options);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{options:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty());
    }
    assert!(!f.0.join("app.exe").exists());
}
#[test]
fn existing_output_and_source_as_output_are_never_overwritten() {
    let f = Fixture::new("fn main(){}");
    std::fs::write(f.output(), b"sentinel").unwrap();
    let out = f.build();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(std::fs::read(f.output()).unwrap(), b"sentinel");
    if !cfg!(windows) {
        let out = Command::new(env!("CARGO_BIN_EXE_svr"))
            .args([
                "build",
                "--experimental-generics",
                "--emit",
                "exe",
                "--output",
            ])
            .arg(f.0.join("main.svr"))
            .arg(f.0.join("main.svr"))
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(
            std::fs::read_to_string(f.0.join("main.svr")).unwrap(),
            "fn main(){}"
        );
    }
}
#[test]
fn invalid_source_and_nonunit_entry_never_create_outputs() {
    for source in ["fn main(){missing();}", "fn main()->Int{return 4;}"] {
        let f = Fixture::new(source);
        let out = f.build();
        assert_eq!(out.status.code(), Some(1));
        assert!(!f.output().exists());
    }
}
#[test]
fn oversized_native_source_refuses_before_creating_staging() {
    let mut source = "fn main(){}//".to_owned();
    source.push_str(&"x".repeat(1024 * 1024));
    let f = Fixture::new(&source);
    let out = f.build();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("exceeds 1 MiB"));
    assert!(!f.output().exists());
    assert_eq!(std::fs::read_dir(&f.0).unwrap().count(), 1);
}

#[test]
fn build_help_respects_the_option_delimiter() {
    let f = Fixture::new("fn main(){}");
    let out = f.cli(&["--emit", "exe", "--output", "app.exe", "--", "--help"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!f.output().exists());
}
