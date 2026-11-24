//! Explicit standalone v2 JSON symbol selection and refusals.
use std::{fs, process::Command};
#[test]
fn json_only_successful_standalone_checks_publish_symbols() {
    let dir = std::env::temp_dir().join(format!("sovra-v2-symbols-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("main.svr");
    fs::write(
        &source,
        "enum E{End}fn helper()->E{return E::End();}fn main(){let e=helper();}",
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .output()
            .unwrap()
    };
    let p = source.to_str().unwrap();
    let out = run(&[
        "check",
        "--experimental-v2",
        "--format",
        "json",
        "--symbols",
        p,
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = String::from_utf8(out.stdout).unwrap();
    assert!(json.contains("\"symbols\":{\"declarations\":["));
    assert!(json.contains("\"name\":\"helper\""));
    assert!(!json.contains("unavailable"));
    let out = run(&["check", "--experimental-v2", "--format", "json", p]);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("\"symbols\":"));
    for args in [
        vec!["check", "--experimental-v2", "--symbols", p],
        vec![
            "check",
            "--experimental-v2",
            "--format",
            "json",
            "--symbols",
            "--symbols",
            p,
        ],
        vec![
            "check",
            "--experimental-v2",
            "--format",
            "--symbols",
            "json",
            p,
        ],
        vec![
            "test",
            "--experimental-v2",
            "--format",
            "json",
            "--symbols",
            p,
        ],
        vec![
            "check",
            "--experimental-v2",
            "--format",
            "json",
            "--symbols",
            dir.to_str().unwrap(),
        ],
    ] {
        assert_eq!(run(&args).status.code(), Some(2), "{args:?}");
    }
    fs::write(&source, "fn main(){missing();}").unwrap();
    let out = run(&[
        "check",
        "--experimental-v2",
        "--format",
        "json",
        "--symbols",
        p,
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("\"symbols\":"));
    fs::remove_dir_all(dir).unwrap();
}
