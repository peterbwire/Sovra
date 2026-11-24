//! Generic formatter syntax preservation and independent locked package consumers.
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
            "sovra generic fmt {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, source).unwrap();
        path
    }
    fn command(&self, args: &[&str], path: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .arg(path)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str], path: &Path) -> Output {
        let out = self.command(args, path);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generic_propagation_comments_literals_and_matches_preserve_syntax_and_fixed_points() {
    let f = Fixture::new();
    let source="// λ😀\r\ntype List<T> = [T];struct Box<T>{value:T}enum Choice<T>{Some(T),None}\r\nfn forward<T>(x:std::Result<T,std::Error>)->std::Result<T,std::Error>{let y=x?;// preserve ?\r\nreturn std::Result::Ok::<T,std::Error>(y);}fn use_value<T>(v:Choice<T>){match v{Choice::Some(x)=>{let text=\"? \\\" λ\";},Choice::None()=>{}}}";
    let p = f.file("source.svr", source);
    let formatted = f.ok(&["fmt", "--experimental-generics"], &p).stdout;
    assert_eq!(std::fs::read(&p).unwrap(), source.as_bytes());
    assert!(String::from_utf8_lossy(&formatted).contains("x?; // preserve ?"));
    let q = f.file("formatted.svr", std::str::from_utf8(&formatted).unwrap());
    assert_eq!(
        f.ok(&["fmt", "--experimental-generics"], &q).stdout,
        formatted
    );
    f.ok(&["fmt", "--experimental-generics", "--check"], &q);
    assert!(!f.command(&["fmt"], &p).status.success());
    assert!(!f
        .command(&["fmt", "--experimental-v2"], &p)
        .status
        .success());
}

#[test]
fn malformed_overdeep_and_conflicting_profiles_never_publish_or_modify_source() {
    let f = Fixture::new();
    for source in [
        "fn bad<T>(x:T){let y=;}",
        "fn bad<T>(x:T){let y=x??;",
        "use broken; fn main(){}",
        "fn main(){let n=1+;}",
    ] {
        let p = f.file("bad.svr", source);
        let out = f.command(&["fmt", "--experimental-generics", "--write"], &p);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), source);
    }
    for source in [
        format!(
            "fn main(){{let x={}1{};}}",
            "(".repeat(129),
            ")".repeat(129)
        ),
        format!("fn main(){{let x={};}}", vec!["1"; 4096].join("+")),
        format!("fn main(){{let x={}+;}}", vec!["1"; 4096].join("+")),
    ] {
        let p = f.file("deep.svr", &source);
        let out = f.command(&["fmt", "--experimental-generics"], &p);
        assert!(!out.status.success());
        assert!(out.status.code().is_some(), "formatter crashed");
        assert!(out.stdout.is_empty());
    }
    let p = f.file("valid.svr", "fn main(){}");
    for flags in [
        ["fmt", "--experimental-generics", "--experimental-generics"],
        ["fmt", "--experimental-generics", "--experimental-v2"],
    ] {
        assert_eq!(f.command(&flags, &p).status.code(), Some(2));
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn formatted_data_package_locks_checks_tests_and_runs_with_unchanged_behavior() {
    let f = Fixture::new();
    for name in ["main.svr", "sovra.toml"] {
        f.file(
            &format!("app/{name}"),
            &std::fs::read_to_string(format!("examples/data-toolchain/app/{name}")).unwrap(),
        );
    }
    let app = f.0.join("app");
    let before = f
        .ok(
            &["run", "--experimental-generics", "--experimental-data"],
            &app,
        )
        .stdout;
    f.ok(
        &["fmt", "--experimental-generics", "--package", "--write"],
        &app,
    );
    f.ok(
        &["fmt", "--experimental-generics", "--package", "--check"],
        &app,
    );
    f.ok(&["lock", "--experimental-generics"], &app);
    for verb in ["check", "test", "run"] {
        let out = f.ok(
            &[
                verb,
                "--experimental-generics",
                "--experimental-data",
                "--locked",
            ],
            &app,
        );
        if verb == "run" {
            assert_eq!(out.stdout, before);
        }
    }
    let lock = std::fs::read(app.join("sovra.lock")).unwrap();
    f.ok(
        &["fmt", "--experimental-generics", "--package", "--write"],
        &app,
    );
    assert_eq!(std::fs::read(app.join("sovra.lock")).unwrap(), lock);
    f.ok(
        &[
            "run",
            "--experimental-generics",
            "--experimental-data",
            "--locked",
        ],
        &app,
    );
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn dependency_module_formatting_preflights_every_source_before_writes() {
    let f = Fixture::new();
    for name in [
        "app/sovra.toml",
        "app/main.svr",
        "helpers/sovra.toml",
        "helpers/main.svr",
        "helpers/values.svr",
    ] {
        f.file(
            name,
            &std::fs::read_to_string(format!("examples/generic-packages/{name}")).unwrap(),
        );
    }
    let app = f.0.join("app");
    let before = f.ok(&["run", "--experimental-generics"], &app).stdout;
    f.ok(
        &["fmt", "--experimental-generics", "--package", "--write"],
        &app,
    );
    f.ok(
        &["fmt", "--experimental-generics", "--package", "--check"],
        &app,
    );
    f.ok(&["lock", "--experimental-generics"], &app);
    assert_eq!(
        f.ok(&["run", "--experimental-generics", "--locked"], &app)
            .stdout,
        before
    );
    let entry = f.file("app/main.svr", "use helpers::values;fn main(){print(1);}");
    f.file(
        "helpers/values.svr",
        "mod values {export fn broken<T>(x:T){let y=;}}",
    );
    let bytes = std::fs::read(&entry).unwrap();
    assert!(!f
        .command(
            &["fmt", "--experimental-generics", "--package", "--write"],
            &app
        )
        .status
        .success());
    assert_eq!(std::fs::read(entry).unwrap(), bytes);
    #[cfg(unix)]
    {
        let original = f.file("outside.svr", "fn main(){}");
        std::os::unix::fs::symlink(&original, f.0.join("link.svr")).unwrap();
        assert!(!f
            .command(
                &["fmt", "--experimental-generics", "--write"],
                &f.0.join("link.svr")
            )
            .status
            .success());
        assert_eq!(std::fs::read_to_string(original).unwrap(), "fn main(){}");
    }
}
