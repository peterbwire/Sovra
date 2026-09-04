//! Command-line parsing for `svr`.

use std::fmt::Write as _;
use std::fs;
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
  new              Create a Sovra project (planned)
  init             Initialize a Sovra project (planned)
  run              Compile and run a Sovra program
  build            Compile a Sovra source file
  test             Run Sovra tests (planned)
  check            Check a Sovra source file or project
  fmt              Format Sovra source files (planned)
  repl             Start the Sovra REPL (planned)
  install          Install a package (planned)
  update           Update project dependencies (planned)
  doc              Build Sovra documentation (planned)

M12 provides `check` for source files and project manifests. M11 provides
`run` over lexing, parsing, semantic analysis, IR lowering, and interpreter
execution, plus `build` for IR inspection and portable JavaScript output.
Other commands remain planned.
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

    if args.iter().any(|arg| arg == "--help" || arg == "-h")
        && matches!(command, "run" | "build" | "check")
    {
        if command == "build" {
            println!("Usage: svr build [--emit ir|js] <source.svr>");
        } else if command == "check" {
            println!("Usage: svr check <source.svr|project-directory>");
        } else {
            println!("Usage: svr run <source.svr>");
        }
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{command} is planned for a future Sovra milestone.");
        return ExitCode::SUCCESS;
    }

    if command == "run" || command == "build" {
        return compile_command(command, args);
    }
    if command == "check" {
        return check_command(args);
    }

    let mut message = String::new();
    let _ = write!(
        message,
        "svr: command '{command}' is not implemented in M11"
    );
    if !args.is_empty() {
        let _ = write!(message, " (arguments were not processed)");
    }

    fn compile_command(command: &str, args: &[String]) -> ExitCode {
        let (emit, source_args) = match parse_emit(command, args) {
            Ok(parsed) => parsed,
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            }
        };
        if source_args.len() != 1 {
            eprintln!("svr: {command} expects exactly one .svr source path");
            return ExitCode::from(2);
        }
        let path = match source_args.first() {
            Some(path) if path.ends_with(".svr") => path,
            Some(path) => {
                eprintln!("svr: source path `{path}` must have a .svr extension");
                return ExitCode::from(2);
            }
            None => unreachable!("argument count is validated above"),
        };
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("svr: cannot read `{path}`: {error}");
                return ExitCode::from(1);
            }
        };
        let program = match compiler::parser::Parser::new().parse_source(&source) {
            Ok(program) => program,
            Err(diagnostics) => {
                for diagnostic in diagnostics.items {
                    eprintln!("error[{}]: {}", diagnostic.code, diagnostic.message);
                }
                return ExitCode::from(1);
            }
        };
        let typed = match compiler::semantic::SemanticAnalyzer::new().analyze(&program) {
            Ok(typed) => typed,
            Err(diagnostics) => {
                for diagnostic in diagnostics.items {
                    eprintln!("error[{}]: {}", diagnostic.code, diagnostic.message);
                }
                return ExitCode::from(1);
            }
        };
        let ir = compiler::ir::lower(&typed);
        if command == "build" {
            match emit {
                Emit::Ir => print!("{}", compiler::backend::render(&ir)),
                Emit::Js => print!("{}", compiler::backend::render_javascript(&ir)),
            }
            return ExitCode::SUCCESS;
        }
        match compiler::interpreter::run(&ir) {
            Ok(output) => {
                for line in output {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("svr: runtime error: {error}");
                ExitCode::from(1)
            }
        }
    }
    eprintln!("{message}.");
    eprintln!("See docs/roadmap.md for planned functionality.");
    ExitCode::from(1)
}

fn check_command(args: &[String]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("svr: check expects exactly one source path or project directory");
        return ExitCode::from(2);
    }
    let path = &args[0];
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("svr: cannot inspect `{path}`: {error}");
            return ExitCode::from(1);
        }
    };
    if metadata.is_dir() {
        match compiler::project::check_project(path) {
            Ok(project) => {
                println!(
                    "checked project `{}`: {} source file(s), entry {}",
                    project.name,
                    project.source_files.len(),
                    project.entry_path.display()
                );
                ExitCode::SUCCESS
            }
            Err(diagnostics) => {
                print_diagnostics(diagnostics);
                ExitCode::from(1)
            }
        }
    } else {
        check_source_file(path)
    }
}

fn check_source_file(path: &str) -> ExitCode {
    if !path.ends_with(".svr") {
        eprintln!("svr: source path `{path}` must have a .svr extension");
        return ExitCode::from(2);
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("svr: cannot read `{path}`: {error}");
            return ExitCode::from(1);
        }
    };
    let program = match compiler::parser::Parser::new().parse_source(&source) {
        Ok(program) => program,
        Err(diagnostics) => {
            print_diagnostics(diagnostics);
            return ExitCode::from(1);
        }
    };
    if let Err(diagnostics) = compiler::semantic::SemanticAnalyzer::new().analyze(&program) {
        print_diagnostics(diagnostics);
        return ExitCode::from(1);
    }
    println!("checked source `{path}`");
    ExitCode::SUCCESS
}

fn print_diagnostics(diagnostics: compiler::diagnostics::Diagnostics) {
    for diagnostic in diagnostics.items {
        eprintln!(
            "error[{}] at {}:{}: {}",
            diagnostic.code,
            diagnostic.span.line + 1,
            diagnostic.span.column + 1,
            diagnostic.message
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Emit {
    Ir,
    Js,
}

fn parse_emit(command: &str, args: &[String]) -> Result<(Emit, Vec<String>), String> {
    if command != "build" {
        return Ok((Emit::Ir, args.to_vec()));
    }
    let mut emit = Emit::Ir;
    let mut source_args = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--emit" {
            let Some(value) = args.get(index + 1) else {
                return Err("svr: --emit expects `ir` or `js`".to_owned());
            };
            emit = parse_emit_value(value)?;
            index += 2;
        } else if let Some(value) = arg.strip_prefix("--emit=") {
            emit = parse_emit_value(value)?;
            index += 1;
        } else {
            source_args.push(arg.clone());
            index += 1;
        }
    }
    Ok((emit, source_args))
}

fn parse_emit_value(value: &str) -> Result<Emit, String> {
    match value {
        "ir" => Ok(Emit::Ir),
        "js" => Ok(Emit::Js),
        _ => Err(format!("svr: unsupported emit target `{value}`")),
    }
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
        assert_eq!(run(["build"]), ExitCode::from(2));
    }

    #[test]
    fn unknown_command_is_rejected() {
        assert_eq!(run(["wat"]), ExitCode::from(2));
    }
}
