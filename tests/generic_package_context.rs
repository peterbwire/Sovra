//! Generic task inventories are versioned metadata, not source validation.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::v2::{parse_json, JsonLimits, JsonValue as J};
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra generic context λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .arg(self.0.join("app"))
            .output()
            .unwrap()
    }
    fn graph(&self) {
        self.file("app/sovra.toml","[project]\nname=\"app\"\nentry=\"main.svr\"\n[modules]\nlocal=\"local.svr\"\n[dependencies.helper]\npath=\"../helper\"\n");
        self.file(
            "helper/sovra.toml",
            "[project]\nname=\"helper\"\nentry=\"main.svr\"\n[modules]\nvalues=\"values.svr\"\n",
        );
        for file in [
            "app/main.svr",
            "app/local.svr",
            "helper/main.svr",
            "helper/values.svr",
        ] {
            self.file(file, "deliberately invalid source ?");
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn field<'a>(j: &'a J, key: &str) -> &'a J {
    let J::Object(v) = j else {
        panic!("object required")
    };
    &v.iter().find(|(k, _)| k == key).unwrap().1
}
fn report(out: &Output) -> J {
    parse_json(&out.stdout, JsonLimits::DEFAULT).unwrap()
}
const FLAGS: &[&str] = &[
    "check",
    "--experimental-generics",
    "--format",
    "json",
    "--package-context",
];
#[test]
fn metadata_selects_registered_dependency_sources_without_checking_or_writing_them() {
    let f = Fixture::new();
    f.graph();
    let out = f.cli(FLAGS);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r = report(&out);
    assert_eq!(field(&r, "source_check"), &J::Bool(false));
    assert_eq!(
        field(&r, "profile"),
        &J::String("experimental-generics".into())
    );
    let c = field(&r, "package_context");
    assert_eq!(field(c, "schema_version"), &J::Int(2));
    let J::Array(paths) = field(c, "sources") else {
        panic!()
    };
    assert_eq!(paths.len(), 4);
    for name in [
        "app/main.svr",
        "app/local.svr",
        "helper/main.svr",
        "helper/values.svr",
    ] {
        assert!(paths.contains(&J::String(
            std::fs::canonicalize(f.0.join(name))
                .unwrap()
                .to_str()
                .unwrap()
                .into()
        )));
        assert_eq!(
            std::fs::read_to_string(f.0.join(name)).unwrap(),
            "deliberately invalid source ?"
        );
    }
    let J::Array(manifests) = field(c, "manifests") else {
        panic!()
    };
    assert_eq!(manifests.len(), 2);
    assert!(!f.0.join("app/sovra.lock").exists());
    assert!(!f
        .cli(&["check", "--experimental-generics", "--format", "json"])
        .status
        .success());
}
#[test]
fn stale_or_invalid_manifests_return_no_partial_context() {
    let f = Fixture::new();
    f.graph();
    std::fs::remove_file(f.0.join("helper/values.svr")).unwrap();
    let out = f.cli(FLAGS);
    assert_eq!(out.status.code(), Some(1));
    let r = report(&out);
    assert_eq!(field(&r, "success"), &J::Bool(false));
    assert_eq!(field(&r, "package_context"), &J::Null);
}
#[test]
fn context_refuses_ambiguous_profiles_and_nonmetadata_operations() {
    let f = Fixture::new();
    f.graph();
    for extra in [
        "--package-context",
        "--symbols",
        "--locked",
        "--experimental-data",
        "--experimental-collections",
    ] {
        let mut args = FLAGS.to_vec();
        args.push(extra);
        let out = f.cli(&args);
        assert_eq!(out.status.code(), Some(2), "{extra}");
        assert!(out.stdout.is_empty());
    }
    for args in [
        &["run", "--experimental-generics", "--package-context"][..],
        &["check", "--experimental-generics", "--package-context"][..],
    ] {
        assert_eq!(f.cli(args).status.code(), Some(2));
    }
}
#[test]
fn task_source_ceiling_accepts_1024_and_refuses_1025_without_partial_inventory() {
    let f = Fixture::new();
    f.file("app/main.svr", "fn main(){}");
    let mut manifest = String::from("[project]\nname=\"app\"\nentry=\"main.svr\"\n[modules]\n");
    for n in 0..1023 {
        manifest.push_str(&format!("m{n}=\"m{n}.svr\"\n"));
        f.file(&format!("app/m{n}.svr"), "not parsed");
    }
    f.file("app/sovra.toml", &manifest);
    let out = f.cli(FLAGS);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let r = report(&out);
    let J::Array(sources) = field(field(&r, "package_context"), "sources") else {
        panic!()
    };
    assert_eq!(sources.len(), 1024);
    manifest.push_str("last=\"last.svr\"\n");
    f.file("app/last.svr", "not parsed");
    f.file("app/sovra.toml", &manifest);
    let out = f.cli(FLAGS);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(field(&report(&out), "package_context"), &J::Null);
}
