//! Integration tests for the `svr` command-line contract.

use std::process::Command;

fn svr() -> Command {
    Command::new(env!("CARGO_BIN_EXE_svr"))
}

#[test]
fn version_command_prints_canonical_version() {
    let output = svr().arg("--version").output().expect("svr should run");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "svr 0.1.0");
}

#[test]
fn help_command_describes_m0() {
    let output = svr().arg("--help").output().expect("svr should run");
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("Usage:"));
    assert!(help.contains("build"));
    assert!(help.contains("M4"));
}

#[test]
fn all_reserved_commands_are_recognized() {
    for command in [
        "new", "init", "run", "build", "test", "check", "fmt", "repl", "install", "update", "doc",
    ] {
        let output = svr().arg(command).output().expect("svr should run");
        if command == "run" || command == "build" {
            assert_eq!(output.status.code(), Some(2));
        } else {
            assert_ne!(output.status.code(), Some(2));
        }
    }
}

#[test]
fn unknown_command_has_helpful_error() {
    let output = svr().arg("unknown").output().expect("svr should run");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("svr --help"));
}
