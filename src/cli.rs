//! Command-line parsing for `svr`.

use std::fmt::Write as _;
use std::process::ExitCode;

use crate::compiler;

/// The version of the Sovra toolchain exposed by the CLI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
Sovra toolchain

Usage:
  svr [OPTIONS]
  svr <COMMAND>

Options:
  -h, --help       Print this help message
  -V, --version    Print version information

Commands:
  build            Compile a Sovra source file (planned)
  check            Check a Sovra source file (planned)
  fmt              Format Sovra source files (planned)
  init             Create a Sovra project (planned)
  run              Compile and run a Sovra program (planned)

M0 provides the command-line foundation only. Compiler stages are placeholders.
Use `svr <COMMAND> --help` for command-specific status.";

/// Run the CLI using an iterator of argument strings.
pub fn run<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    match args.as_slice() {
        [] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [flag] if flag == "--version" || flag == "-V" => {
            println!("svr {VERSION}");
            ExitCode::SUCCESS
        }
        [command, rest @ ..] => command_status(command, rest),
    }
}

fn command_status(command: &str, args: &[String]) -> ExitCode {
    if matches!(command, "--help" | "-h") {
        if args.len() == 1 {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        eprintln!("svr: unexpected arguments after {command}");
        return ExitCode::from(2);
    }

    if matches!(command, "--version" | "-V") {
        eprintln!("svr: unexpected arguments after {command}");
        return ExitCode::from(2);
    }

    if !compiler::is_known_command(command) {
        eprintln!("svr: unknown command '{command}'");
        eprintln!("Try 'svr --help' for available commands.");
        return ExitCode::from(2);
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{command} is planned for a future Sovra milestone.");
        println!("M0 does not implement compiler or runtime functionality.");
        return ExitCode::SUCCESS;
    }

    let mut message = String::new();
    let _ = write!(
        message,
        "svr: command '{command}' is not implemented in M0"
    );
    if !args.is_empty() {
        let _ = write!(message, " (arguments were not processed)");
    }
    eprintln!("{message}.");
    eprintln!("See docs/roadmap.md for planned functionality.");
    ExitCode::from(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported() {
        assert_eq!(run(["--version"]), ExitCode::SUCCESS);
    }

    #[test]
    fn known_future_command_is_not_implemented() {
        assert_eq!(run(["build"]), ExitCode::from(1));
    }

    #[test]
    fn unknown_command_is_rejected() {
        assert_eq!(run(["wat"]), ExitCode::from(2));
    }
}

