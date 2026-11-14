//! Explicit process host grants; defaults, effect ordering and existing bounds.
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
            "sovra-cli-v2-hosts-{}-{}",
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
    fn input(&self, args: &[&str], bytes: &[u8]) -> Output {
        use std::io::Write;
        let mut child = Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .current_dir(&self.0)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let result = child.stdin.take().unwrap().write_all(bytes);
        if let Err(error) = result {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        child.wait_with_output().unwrap()
    }
}
#[test]
fn independent_enum_input_file_pipeline_requires_both_grants() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        include_str!("../examples/enum-text-copy/main.svr"),
    );
    for flags in [vec![], vec!["--allow-stdin"], vec!["--allow-files"]] {
        let mut args = vec!["run", "--experimental-v2"];
        args.extend(flags);
        args.extend([text(&p), "--", "out.txt"]);
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(!t.0.join("out.txt").exists());
    }
    let o = t.input(
        &[
            "run",
            "--experimental-v2",
            "--allow-stdin",
            "--allow-files",
            text(&p),
            "--",
            "out.txt",
        ],
        "λ😀\r\n".as_bytes(),
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(o.stdout, b"6\n");
    assert_eq!(std::fs::read_to_string(t.0.join("out.txt")).unwrap(), "λ😀");
}
#[test]
fn line_reader_shares_stdin_across_calls_and_preserves_eof_and_terminators() {
    let t = Temp::new();
    let p=t.source("main.svr","fn main(){read();read();read();}fn read(){let line=std::read_line();print(line.text);print(line.eof);}");
    let o = t.input(
        &["run", "--experimental-v2", "--allow-stdin", text(&p)],
        "héllo\r\nlast\r".as_bytes(),
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "héllo\nfalse\nlast\r\nfalse\n\ntrue\n"
    );
}
#[test]
fn line_reader_enforces_exact_byte_limit_and_strict_utf8() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        "fn main(){let line=std::read_line();print(std::len(line.text));}",
    );
    let limit = sovra::compiler::stdlib::MAX_INPUT_LINE_BYTES;
    let mut exact = vec![b'x'; limit];
    exact.extend_from_slice(b"\r\n");
    let o = t.input(
        &["run", "--experimental-v2", "--allow-stdin", text(&p)],
        &exact,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout), format!("{limit}\n"));
    for bytes in [vec![b'x'; limit + 1], vec![0xff, b'\n']] {
        let o = t.input(
            &["run", "--experimental-v2", "--allow-stdin", text(&p)],
            &bytes,
        );
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert!(String::from_utf8_lossy(&o.stderr).contains("standard input"));
    }
}
#[test]
fn file_grant_preserves_atomic_conditional_results_and_linked_services() {
    let t = Temp::new();
    let p=t.source("main.svr","fn main(){files();}fn files(){let a=std::write_text(\"p\",\"λ\");print(a.ok);let b=std::write_text_if(\"p\",true,\"old\",\"lost\");print(b.error);let c=std::write_text_if(\"p\",true,\"λ\",\"😀\");print(c.ok);let d=std::read_text(\"p\");print(d.text);}");
    let o = t.cli(&["run", "--experimental-v2", "--allow-files", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "true\nconflict\ntrue\n😀\n"
    );
    assert_eq!(std::fs::read_to_string(t.0.join("p")).unwrap(), "😀");
}
#[test]
fn file_errors_remain_structured_for_missing_invalid_and_oversized_text() {
    let t = Temp::new();
    std::fs::write(t.0.join("bad"), [0xff]).unwrap();
    std::fs::File::create(t.0.join("large"))
        .unwrap()
        .set_len(sovra::compiler::stdlib::MAX_TEXT_FILE_BYTES as u64 + 1)
        .unwrap();
    let p=t.source("main.svr","fn main(){let a=std::read_text(\"missing\");print(a.error);let b=std::read_text(\"bad\");print(b.error);let c=std::read_text(\"large\");print(c.error);let d=std::write_text(\"\",\"x\");print(d.error);}");
    let o = t.cli(&["run", "--experimental-v2", "--allow-files", text(&p)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        o.stdout,
        b"not_found\ninvalid_utf8\ntoo_large\ninvalid_path\n"
    );
}
#[test]
fn successful_effects_survive_later_runtime_and_fuel_refusal_without_output() {
    let t = Temp::new();
    for failure in ["print(1/0);", "while true{}"] {
        let p=t.source("main.svr",&format!("fn main(){{std::write_text(\"committed\",\"kept\");print(\"hidden\");std::set_exit_code(7);{failure}}}"));
        let o = t.cli(&["run", "--experimental-v2", "--allow-files", text(&p)]);
        assert_eq!(o.status.code(), Some(1));
        assert!(o.stdout.is_empty());
        assert_eq!(
            std::fs::read_to_string(t.0.join("committed")).unwrap(),
            "kept"
        );
    }
}
#[test]
fn missing_capability_preflight_precedes_every_effect_even_in_unused_functions() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        "fn main(){std::write_text(\"marker\",\"bad\");}fn unused(){std::read_line();}",
    );
    let o = t.cli(&["run", "--experimental-v2", "--allow-files", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("std::read_line"));
    assert!(!t.0.join("marker").exists());
    let p = t.source(
        "main.svr",
        "fn main(){std::write_text(\"marker\",\"bad\");print(unknown);}",
    );
    let o = t.cli(&["run", "--experimental-v2", "--allow-files", text(&p)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!t.0.join("marker").exists());
}
#[test]
fn grants_apply_only_to_experimental_run_before_the_argument_delimiter() {
    let t = Temp::new();
    let p = t.source("main.svr", "fn main(){}fn test_a(){}");
    for args in [
        vec![
            "run",
            "--experimental-v2",
            "--allow-files",
            "--allow-files",
            text(&p),
        ],
        vec![
            "run",
            "--experimental-v2",
            "--allow-stdin",
            "--allow-stdin",
            text(&p),
        ],
        vec!["run", "--allow-files", text(&p)],
        vec!["run", "--allow-stdin", text(&p)],
        vec!["check", "--experimental-v2", "--allow-files", text(&p)],
        vec!["test", "--experimental-v2", "--allow-stdin", text(&p)],
        vec!["build", "--experimental-v2", "--allow-files", text(&p)],
    ] {
        let o = t.cli(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(o.stdout.is_empty());
    }
    let p = t.source(
        "main.svr",
        "fn main(){print(std::arg(0));print(std::arg(1));}",
    );
    let o = t.cli(&[
        "run",
        "--experimental-v2",
        text(&p),
        "--",
        "--allow-files",
        "--allow-stdin",
    ]);
    assert!(o.status.success());
    assert_eq!(o.stdout, b"--allow-files\n--allow-stdin\n");
    let p = t.source(
        "main.svr",
        "fn main(){std::write_text(\"marker\",\"bad\");}",
    );
    let o = t.cli(&["run", "--experimental-v2", text(&p), "--", "--allow-files"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!t.0.join("marker").exists());
}
#[test]
fn file_grant_uses_the_callers_cwd_and_process_permissions_without_path_confinement() {
    let t = Temp::new();
    let cwd = t.0.join("cwd");
    std::fs::create_dir(&cwd).unwrap();
    let p = t.source(
        "main.svr",
        "fn main(){let result=std::write_text(\"../outside.txt\",\"explicit\");print(result.ok);}",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["run", "--experimental-v2", "--allow-files", text(&p)])
        .current_dir(&cwd)
        .output()
        .unwrap();
    assert!(o.status.success());
    assert_eq!(o.stdout, b"true\n");
    assert_eq!(
        std::fs::read_to_string(t.0.join("outside.txt")).unwrap(),
        "explicit"
    );
}

#[cfg(unix)]
#[test]
fn completed_file_effect_survives_output_publication_failure() {
    let t = Temp::new();
    let p = t.source(
        "main.svr",
        "fn main(){std::write_text(\"committed\",\"kept\");print(\"result\");}",
    );
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let o = Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["run", "--experimental-v2", "--allow-files", text(&p)])
        .current_dir(&t.0)
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("program output"));
    assert_eq!(
        std::fs::read_to_string(t.0.join("committed")).unwrap(),
        "kept"
    );
}
