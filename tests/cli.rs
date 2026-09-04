//! Integration tests for the `svr` command-line contract.

use std::io;
use std::process::{Command, Output};

fn svr() -> Command {
    Command::new(env!("CARGO_BIN_EXE_svr"))
}

fn output_or_skip(command: &mut Command) -> Option<Output> {
    match command.output() {
        Ok(output) => Some(output),
        Err(error) if is_application_control_block(&error) => {
            eprintln!(
                "skipping CLI integration assertion: Windows Application Control blocked svr.exe"
            );
            None
        }
        Err(error) => panic!("svr should run: {error}"),
    }
}

fn is_application_control_block(error: &io::Error) -> bool {
    error.raw_os_error() == Some(4551)
}

#[test]
fn version_command_prints_canonical_version() {
    let Some(output) = output_or_skip(svr().arg("--version")) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "svr 0.1.0");
}

#[test]
fn help_command_describes_m11() {
    let Some(output) = output_or_skip(svr().arg("--help")) else {
        return;
    };
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("Usage:"));
    assert!(help.contains("build"));
    assert!(help.contains("M12"));
}

#[test]
fn run_command_executes_source_file() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["run", source.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Hello, Sovra!"
    );
}

#[test]
fn build_command_emits_inspectable_ir() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["build", source.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    let ir = String::from_utf8_lossy(&output.stdout);
    assert!(ir.contains("function main:"));
    assert!(ir.contains("call print 1"));
}

#[test]
fn build_command_emits_javascript_backend() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["build", "--emit", "js", source.as_str()]))
    else {
        return;
    };
    assert!(output.status.success());
    let javascript = String::from_utf8_lossy(&output.stdout);
    assert!(javascript.contains("\"use strict\";"));
    assert!(javascript.contains("svrFunctions[\"main\"] = svr_fn_0;"));
    assert!(javascript.contains("console.log"));
}

#[test]
fn build_help_describes_current_usage() {
    let Some(output) = output_or_skip(svr().args(["build", "--help"])) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Usage: svr build [--emit ir|js] <source.svr>"
    );
}

#[test]
fn check_help_describes_current_usage() {
    let Some(output) = output_or_skip(svr().args(["check", "--help"])) else {
        return;
    };
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Usage: svr check <source.svr|project-directory>"
    );
}

#[test]
fn check_command_validates_source_file() {
    let source = format!(
        "{}/examples/hello-world/main.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", source.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("checked source"));
}

#[test]
fn check_command_validates_project_directory() {
    let project = format!("{}/examples/fielddesk", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", project.as_str()])) else {
        return;
    };
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("checked project `fielddesk`"));
    assert!(stdout.contains("2 service(s), 4 model(s), 3 route(s), 2 page(s)"));
    assert!(stdout.contains("1 scheduled task(s), 3 auth policy(ies), auth auth.session"));
    assert!(stdout.contains("entry"));
}

#[test]
fn run_command_rejects_non_sovra_paths() {
    let Some(output) = output_or_skip(svr().args(["run", "README.md"])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains(".svr extension"));
}

#[test]
fn all_reserved_commands_are_recognized() {
    for command in [
        "new", "init", "run", "build", "test", "check", "fmt", "repl", "install", "update", "doc",
    ] {
        let Some(output) = output_or_skip(svr().arg(command)) else {
            return;
        };
        if matches!(command, "run" | "build" | "check") {
            assert_eq!(output.status.code(), Some(2));
        } else {
            assert_ne!(output.status.code(), Some(2));
        }
    }
}

#[test]
fn unknown_command_has_helpful_error() {
    let Some(output) = output_or_skip(svr().arg("unknown")) else {
        return;
    };
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("svr --help"));
}
