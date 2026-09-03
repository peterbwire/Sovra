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
    assert!(help.contains("M0"));
}

#[test]
fn future_command_reports_status() {
    let output = svr().arg("build").output().expect("svr should run");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not implemented in M0"));
}

#[test]
fn unknown_command_has_helpful_error() {
    let output = svr().arg("unknown").output().expect("svr should run");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("svr --help"));
}

