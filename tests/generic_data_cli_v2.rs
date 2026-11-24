//! Data package commands and independent word-2047 native launch.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use std::{path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "sovra data cli {} {}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        for file in ["main.svr", "sovra.toml"] {
            std::fs::copy(
                PathBuf::from("examples/data-toolchain/app").join(file),
                root.join("app").join(file),
            )
            .unwrap();
        }
        Self(root)
    }
    fn cli(&self, verb: &str, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg(verb)
            .arg("--experimental-generics")
            .args(args)
            .arg(self.0.join("app"))
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
fn locked_data_package_checks_tests_runs_and_builds_independent_native() {
    let f = Fixture::new();
    for mode in ["--write", "--check"] {
        let formatted = f.cli("fmt", &["--package", mode]);
        assert!(
            formatted.status.success(),
            "{}",
            String::from_utf8_lossy(&formatted.stderr)
        );
    }
    let locked = f.cli("lock", &[]);
    assert!(locked.status.success(), "{:?}", locked);
    for verb in ["check", "run", "test"] {
        let out = f.cli(verb, &["--experimental-data", "--locked"]);
        assert!(
            out.status.success(),
            "{verb}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(!f
        .cli("check", &["--experimental-data", "--experimental-data"])
        .status
        .success());
    let exe = f.0.join("program");
    let built = f.cli(
        "build",
        &[
            "--experimental-data",
            "--locked",
            "--emit",
            "exe",
            "--output",
            exe.to_str().unwrap(),
        ],
    );
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    std::fs::remove_dir_all(f.0.join("app")).unwrap();
    let out = Command::new(exe)
        .env_clear()
        .env("PATH", "/nonexistent")
        .current_dir(&f.0)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn explicit_host_policy_runs_source_and_native_without_embedding_authority() {
    use sovra::compiler::v2::{serialize_json, JsonLimits, JsonValue as J};
    fn q(s: &str) -> String {
        serialize_json(&J::String(s.into()), JsonLimits::DEFAULT).unwrap()
    }
    let f = Fixture::new();
    let data = f.0.join("data");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(data.join("input.bin"), b"abc").unwrap();
    let read = r#"{"version":3,"op":"fs.read","root":"files","path":"input.bin"}"#;
    let process =
        r#"{"version":3,"op":"process.run","program":"cat","arguments":[],"input":"AP8="}"#;
    let date = r#"{"version":3,"op":"date.parse","text":"1970-01-01T00:00:00Z"}"#;
    let mut source=String::from("fn probe(request:String,expected:String)->std::Result<Bool,std::Error>{let request=std::json_parse(request)?;let value=std::host_call(request)?;let text=std::json_stringify(value)?;return std::Result::<Bool,std::Error>::Ok(text==expected);}fn must(value:std::Result<Bool,std::Error>){match value{std::Result::Ok(ok)=>{if ok==false{let failure=1/0;}},std::Result::Err(error)=>{let failure=1/0;}}}fn main(){");
    for (req, expected) in [
        (read, "\"YWJj\""),
        (
            process,
            "{\"exit_code\":0,\"stdout\":\"AP8=\",\"stderr\":\"\"}",
        ),
        (date, "0"),
    ] {
        source.push_str(&format!("must(probe({},{}));", q(req), q(expected)));
    }
    source.push('}');
    source.push_str(&format!("fn test_denied(){{let request=std::json_parse({}) ;match request{{std::Result::Ok(request)=>{{let result=std::host_call(request);match result{{std::Result::Ok(value)=>{{let failure=1/0;}},std::Result::Err(error)=>{{if error.code!=\"authority\"{{let failure=1/0;}}}}}}}},std::Result::Err(error)=>{{let failure=1/0;}}}}}}",q(read)));
    std::fs::write(f.0.join("app/main.svr"), source).unwrap();
    let roots = J::Object(vec![(
        "files".into(),
        J::String(data.to_str().unwrap().into()),
    )]);
    let policy = J::Object(vec![
        ("version".into(), J::Int(3)),
        ("read_roots".into(), roots),
        (
            "processes".into(),
            J::Object(vec![(
                "cat".into(),
                J::Object(vec![
                    ("executable".into(), J::String("/bin/cat".into())),
                    ("cwd".into(), J::String(data.to_str().unwrap().into())),
                    ("environment".into(), J::Object(vec![])),
                ]),
            )]),
        ),
    ]);
    let policy_file = f.0.join("policy.json");
    std::fs::write(
        &policy_file,
        serialize_json(&policy, JsonLimits::DEFAULT).unwrap(),
    )
    .unwrap();
    assert!(f.cli("lock", &[]).status.success());
    for verb in ["check", "test"] {
        let out = f.cli(verb, &["--experimental-data", "--locked"]);
        assert!(
            out.status.success(),
            "{verb}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(!f
        .cli("run", &["--experimental-data", "--locked"])
        .status
        .success());
    let out = f.cli(
        "run",
        &[
            "--experimental-data",
            "--locked",
            "--host-policy",
            policy_file.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = f.0.join("host-program");
    let built = f.cli(
        "build",
        &[
            "--experimental-data",
            "--locked",
            "--emit",
            "exe",
            "--output",
            exe.to_str().unwrap(),
        ],
    );
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    std::fs::remove_dir_all(f.0.join("app")).unwrap();
    assert!(!Command::new(&exe)
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap()
        .status
        .success());
    let out = Command::new(exe)
        .arg("--host-policy")
        .arg(policy_file)
        .env_clear()
        .env("PATH", "/nonexistent")
        .current_dir(&f.0)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
