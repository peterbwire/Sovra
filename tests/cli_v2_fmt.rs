//! Explicit v2 formatter gates and guarded write behavior.
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
fn compiler() -> PathBuf {
    std::env::var_os("SOVRA_FORMATTER_COMPILER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_svr")))
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "sovra-cli-v2-fmt-{}-{}",
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
        Command::new(compiler())
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

const RAW: &str =
    "enum E{A(Int),End}fn main(){let e=E::A(1);match e{E::A(n)=>{print(n);},E::End()=>{}}}";
#[test]
fn enum_match_formatting_is_explicit_and_stdout_never_changes_the_input() {
    let t = Temp::new();
    let p = t.source("main.svr", RAW);
    let o = t.cli(&["fmt", "--experimental-v2", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let formatted = String::from_utf8(o.stdout).unwrap();
    assert!(formatted.contains("enum E {\n    A(Int),\n    End\n}"));
    assert!(formatted.contains("E::A(n) => {\n            print(n);"));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), RAW);
    assert!(!t.cli(&["fmt", text(&p)]).status.success());
    let o = t.cli(&["fmt", "--experimental-v2", "--check", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    let q = t.source("formatted.svr", &formatted);
    let again = t.cli(&["fmt", "--experimental-v2", text(&q)]);
    assert!(again.status.success());
    assert_eq!(again.stdout, formatted.as_bytes());
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--check", text(&q)])
        .status
        .success());
}
#[test]
fn real_recursive_control_and_enum_fixtures_remain_checked_and_execute_identically() {
    let t = Temp::new();
    for source in [
        include_str!("../examples/enum-entry/main.svr"),
        include_str!("../examples/enum-functions/main.svr"),
        include_str!("../examples/nested-matches/main.svr"),
        include_str!("../examples/control-matches/main.svr"),
    ] {
        let p = t.source("before.svr", source);
        let before = t.cli(&["run", "--experimental-v2", text(&p)]);
        assert!(before.status.success());
        let formatted = t.cli(&["fmt", "--experimental-v2", text(&p)]);
        assert!(
            formatted.status.success(),
            "{}",
            String::from_utf8_lossy(&formatted.stderr)
        );
        let q = t.source("after.svr", std::str::from_utf8(&formatted.stdout).unwrap());
        let after = t.cli(&["run", "--experimental-v2", text(&q)]);
        assert!(
            after.status.success(),
            "{}",
            String::from_utf8_lossy(&after.stderr)
        );
        assert_eq!(before.stdout, after.stdout);
    }
}
#[test]
fn type_errors_and_declarations_without_main_can_be_formatted_without_execution() {
    let t = Temp::new();
    for source in ["enum E{A(Missing),A}type Alias=Missing;fn main(x:Unknown){std::write_text(\"marker\",\"bad\");print(unknown);}","enum Empty{}type Alias=Missing;struct S{x:Missing}",""] {
  let p=t.source("main.svr",source);let o=t.cli(&["fmt","--experimental-v2",text(&p)]);assert!(o.status.success(),"{}",String::from_utf8_lossy(&o.stderr));assert!(!t.0.join("marker").exists());assert_eq!(std::fs::read_to_string(&p).unwrap(),source);
 }
}
#[test]
fn comments_crlf_unicode_and_literal_spelling_survive_normalized_layout() {
    let t = Temp::new();
    let source="// λ😀 keep\r\nenum E{A(Int), // variant\r\nEnd}\r\nfn main(){let e=E::A(1); // bind\r\nmatch e{E::A(n)=>{print(\"keep // λ\\n\");}, // arm\r\nE::End()=>{}}} // last\r\n";
    let p = t.source("main.svr", source);
    let o = t.cli(&["fmt", "--experimental-v2", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let output = String::from_utf8(o.stdout).unwrap();
    for part in [
        "// λ😀 keep",
        "// variant",
        "// bind",
        "// arm",
        "// last",
        "\"keep // λ\\n\"",
    ] {
        assert!(output.contains(part), "{part}");
    }
    assert!(!output.contains('\r'));
    assert!(!output.contains('\t'));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), source);
}
#[test]
fn malformed_and_oversized_sources_refuse_without_writes_or_partial_stdout() {
    let t = Temp::new();
    for source in ["enum E{A(]}", "fn main(){match e{E::A()=>{}}", "module m{}"] {
        let p = t.source("main.svr", source);
        let o = t.cli(&["fmt", "--experimental-v2", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), source);
    }
    let p = t.0.join("large.svr");
    std::fs::File::create(&p)
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    let o = t.cli(&["fmt", "--experimental-v2", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    let many = (0..129)
        .map(|i| format!("fn helper_{i}(){{}}\n"))
        .collect::<String>();
    let p = t.source("many.svr", &many);
    let o = t.cli(&["fmt", "--experimental-v2", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    let error = String::from_utf8_lossy(&o.stderr);
    assert!(error.contains("E3025"));
    assert!(!error.contains("many.svr:1:1"));
}
#[test]
fn unknown_conflicting_and_duplicate_modes_remain_usage_errors() {
    let t = Temp::new();
    let p = t.source("main.svr", RAW);
    for args in [
        vec!["fmt", "--experimental-v2"],
        vec!["fmt", "--experimental-v2", "--experimental-v2", text(&p)],
        vec!["fmt", "--experimental-v2", "--check", "--write", text(&p)],
        vec!["fmt", "--experimental-v2", "--locked", text(&p)],
        vec!["fmt", "--experimental-v2", "--package", text(&t.0)],
        vec!["fmt", "--experimental-v2", text(&t.0)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(o.stdout.is_empty());
    }
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--help"])
        .status
        .success());
    let p = t.source("--write.svr", RAW);
    let o = t.cli(&["fmt", "--experimental-v2", "--", "--write.svr"]);
    assert!(o.status.success());
    assert_eq!(std::fs::read_to_string(p).unwrap(), RAW);
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[test]
fn guarded_write_reaches_a_fixed_point_and_unchanged_writes_keep_metadata() {
    let t = Temp::new();
    let p = t.source("main.svr", RAW);
    let o = t.cli(&["fmt", "--experimental-v2", "--write", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(o.stdout.is_empty());
    let formatted = std::fs::read_to_string(&p).unwrap();
    let before = std::fs::metadata(&p).unwrap().modified().unwrap();
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--check", text(&p)])
        .status
        .success());
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--write", text(&p)])
        .status
        .success());
    assert_eq!(std::fs::read_to_string(&p).unwrap(), formatted);
    assert_eq!(std::fs::metadata(&p).unwrap().modified().unwrap(), before);
    let p = t.source("bad.svr", "enum E{A(]}");
    let o = t.cli(&["fmt", "--experimental-v2", "--write", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "enum E{A(]}");
}
#[cfg(unix)]
#[test]
fn symlink_inputs_are_refused_and_readonly_targets_are_not_changed() {
    use std::os::unix::fs::PermissionsExt;
    let t = Temp::new();
    let p = t.source("main.svr", RAW);
    let alias = t.0.join("alias.svr");
    std::os::unix::fs::symlink(&p, &alias).unwrap();
    assert_eq!(
        t.cli(&["fmt", "--experimental-v2", text(&alias)])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(std::fs::read_to_string(&p).unwrap(), RAW);
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o400)).unwrap();
    let o = t.cli(&["fmt", "--experimental-v2", "--write", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), RAW);
}
#[cfg(unix)]
#[test]
fn closed_stdout_returns_failure_without_a_panic_or_input_change() {
    let t = Temp::new();
    let p = t.source("main.svr", RAW);
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let o = Command::new(compiler())
        .args(["fmt", "--experimental-v2", text(&p)])
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let error = String::from_utf8_lossy(&o.stderr);
    assert!(error.contains("cannot publish formatted output") && !error.contains("panicked"));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), RAW);
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[test]
fn relocated_formatter_executable_needs_no_checkout_or_runtime_payload() {
    let t = Temp::new();
    let bin = t.0.join("installed/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let executable = bin.join(if cfg!(windows) { "svr.exe" } else { "svr" });
    std::fs::copy(compiler(), &executable).unwrap();
    let p = t.source("main.svr", RAW);
    let output = Command::new(&executable)
        .args(["fmt", "--experimental-v2", "--write", text(&p)])
        .current_dir(&t.0)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let output = Command::new(&executable)
        .args(["fmt", "--experimental-v2", "--check", text(&p)])
        .current_dir(&t.0)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!t.0.join("installed/runtime-source").exists());
}

fn package(t: &Temp, name: &str, source: &str, extra: &str) -> PathBuf {
    let root = t.0.join(name);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        format!("[project]\nname=\"fixture\"\nentry=\"src/main.svr\"\n{extra}"),
    )
    .unwrap();
    std::fs::write(root.join("src/main.svr"), source).unwrap();
    root
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[test]
fn package_graph_formats_each_canonical_entry_once_without_touching_other_inputs() {
    let t = Temp::new();
    let provider = package(&t, "provider", "enum E{A(Missing),End}", "");
    let root = package(
        &t,
        "app",
        RAW,
        "[dependencies.one]\npath=\"../provider\"\n[dependencies.two]\npath=\"../provider\"\n",
    );
    let ignored = root.join("unrelated.svr");
    std::fs::write(&ignored, "invalid untouched draft").unwrap();
    let lock = root.join("sovra.lock");
    std::fs::write(&lock, "untouched lock bytes").unwrap();
    let manifest = std::fs::read(root.join("sovra.toml")).unwrap();
    let check = t.cli(&[
        "fmt",
        "--experimental-v2",
        "--package",
        "--check",
        text(&root),
    ]);
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(
        std::fs::read_to_string(root.join("src/main.svr")).unwrap(),
        RAW
    );
    assert_eq!(
        String::from_utf8_lossy(&check.stderr)
            .matches("Formatting needed:")
            .count(),
        2
    );
    let write = t.cli(&[
        "fmt",
        "--experimental-v2",
        "--package",
        "--write",
        text(&root),
    ]);
    assert!(
        write.status.success(),
        "{}",
        String::from_utf8_lossy(&write.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&write.stderr)
            .matches("Formatted:")
            .count(),
        2
    );
    assert!(std::fs::read_to_string(provider.join("src/main.svr"))
        .unwrap()
        .contains("Missing"));
    assert!(t
        .cli(&[
            "fmt",
            "--experimental-v2",
            "--package",
            "--check",
            text(&root)
        ])
        .status
        .success());
    assert_eq!(std::fs::read(root.join("sovra.toml")).unwrap(), manifest);
    assert_eq!(
        std::fs::read_to_string(lock).unwrap(),
        "untouched lock bytes"
    );
    assert_eq!(
        std::fs::read_to_string(ignored).unwrap(),
        "invalid untouched draft"
    );
}

#[test]
fn package_preflight_rejects_a_later_malformed_source_without_writes_or_notices() {
    let t = Temp::new();
    let root = package(&t, "a", RAW, "[dependencies.b]\npath=\"../b\"\n");
    let dependency = package(&t, "b", "enum Broken{A(]}", "");
    for mode in ["--check", "--write"] {
        let o = t.cli(&["fmt", "--experimental-v2", "--package", mode, text(&root)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&o.stderr);
        assert!(!stderr.contains("Formatting needed:") && !stderr.contains("Formatted:"));
        if mode == "--write" && !cfg!(any(windows, target_os = "linux")) {
            assert!(stderr.contains("qualified only on Windows and Linux"));
        } else {
            let source = std::fs::canonicalize(dependency.join("src/main.svr")).unwrap();
            assert!(stderr.contains(text(&source)), "{stderr}");
        }
        assert_eq!(
            std::fs::read_to_string(root.join("src/main.svr")).unwrap(),
            RAW
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn registered_package_selection_formats_v2_files_without_linking_or_executing() {
    let t = Temp::new();
    let root = package(
        &t,
        "app",
        "enum E{End}fn main(){std::write_text(\"marker\",\"bad\");}",
        "[modules]\nvalues=\"values.svr\"\n",
    );
    std::fs::write(root.join("values.svr"), "enum E{A(Missing)}").unwrap();
    let o = t.cli(&[
        "fmt",
        "--experimental-v2",
        "--package",
        "--write",
        text(&root),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stderr)
            .matches("Formatted:")
            .count(),
        2
    );
    assert!(!t.0.join("marker").exists());
    let check = t.cli(&["check", "--experimental-v2", text(&root)]);
    assert_eq!(check.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&check.stderr).contains("E3124"));
}

#[test]
fn package_manifest_and_source_limit_failures_preserve_entry_bytes() {
    let t = Temp::new();
    let root = package(&t, "a", RAW, "[dependencies.b]\npath=\"../b\"\n");
    let b = package(&t, "b", "fn main(){}", "");
    std::fs::File::create(b.join("src/main.svr"))
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    for mode in ["--check", "--write"] {
        let o = t.cli(&["fmt", "--experimental-v2", "--package", mode, text(&root)]);
        assert_eq!(o.status.code(), Some(1));
        assert_eq!(
            std::fs::read_to_string(root.join("src/main.svr")).unwrap(),
            RAW
        );
        assert!(!String::from_utf8_lossy(&o.stderr).contains("Formatted:"));
    }
    std::fs::write(root.join("sovra.toml"), "broken manifest").unwrap();
    assert_eq!(
        t.cli(&[
            "fmt",
            "--experimental-v2",
            "--package",
            "--check",
            text(&root)
        ])
        .status
        .code(),
        Some(1)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/main.svr")).unwrap(),
        RAW
    );
}

#[test]
fn module_function_formatting_preserves_syntax_without_enabling_execution() {
    let t = Temp::new();
    let source="// λ😀\r\nmod a{enum E{End}export fn work(x:E)->E{match x{E::End()=>{return x;}}}}mod b{fn work(){missing();}}fn main(){std::write_text(\"marker\",\"bad\");}";
    let p = t.source("main.svr", source);
    let formatted = t.cli(&["fmt", "--experimental-v2", text(&p)]);
    assert!(
        formatted.status.success(),
        "{}",
        String::from_utf8_lossy(&formatted.stderr)
    );
    assert!(!t.0.join("marker").exists());
    assert_eq!(std::fs::read_to_string(&p).unwrap(), source);
    let after = t.source("after.svr", std::str::from_utf8(&formatted.stdout).unwrap());
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--check", text(&after)])
        .status
        .success());
    for args in [
        vec!["check", "--experimental-v2", text(&after)],
        vec!["run", "--experimental-v2", "--allow-files", text(&after)],
        vec!["test", "--experimental-v2", text(&after)],
        vec!["build", "--experimental-v2", text(&after)],
    ] {
        let out = t.cli(&args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(!t.0.join("marker").exists());
    }
    let written = t.cli(&["fmt", "--experimental-v2", "--write", text(&p)]);
    if cfg!(any(windows, target_os = "linux")) {
        assert!(written.status.success());
        assert_eq!(std::fs::read(&p).unwrap(), formatted.stdout);
    } else {
        assert_eq!(written.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&written.stderr)
            .contains("qualified only on Windows and Linux"));
        assert_eq!(std::fs::read(&p).unwrap(), source.as_bytes());
    }
    assert!(!t.0.join("marker").exists());
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[test]
fn package_formatter_prepares_module_function_sources_without_executing_them() {
    let t = Temp::new();
    let root = package(
        &t,
        "module-package",
        "mod m{enum E{End}export fn pick(x:E)->E{match x{E::End()=>{return x;}}}}",
        "",
    );
    let p = text(&root);
    let dirty = t.cli(&["fmt", "--experimental-v2", "--package", "--check", p]);
    assert_eq!(dirty.status.code(), Some(1));
    let write = t.cli(&["fmt", "--experimental-v2", "--package", "--write", p]);
    assert!(
        write.status.success(),
        "{}",
        String::from_utf8_lossy(&write.stderr)
    );
    assert!(t
        .cli(&["fmt", "--experimental-v2", "--package", "--check", p])
        .status
        .success());
    let checked = t.cli(&["check", "--experimental-v2", p]);
    if cfg!(target_os = "linux") {
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
    } else {
        assert_eq!(checked.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&checked.stderr)
            .contains("guarded package source capture unavailable on this host"));
    }
}
