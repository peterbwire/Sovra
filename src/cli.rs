//! Command-line parsing for `svr`.

use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use crate::compiler;
use crate::compiler::check_report::{self, CheckKind};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};

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

    let help_requested = if command == "check" {
        args.iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg == "--help" || arg == "-h")
    } else {
        args.iter().any(|arg| arg == "--help" || arg == "-h")
    };
    if help_requested && matches!(command, "run" | "build" | "check") {
        if command == "build" {
            println!("Usage: svr build [--emit ir|js] <source.svr>");
        } else if command == "check" {
            println!("Usage: svr check [--format human|json] <source.svr|project-directory>");
        } else {
            println!("Usage: svr run <source.svr>");
        }
        return ExitCode::SUCCESS;
    }

    if help_requested {
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
    let (format, path) = match parse_check_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            print_check_io_error(
                path,
                None,
                format,
                format!("svr: cannot inspect `{path}`: {error}"),
            );
            return ExitCode::from(1);
        }
    };
    if metadata.is_dir() {
        match compiler::project::check_project(path) {
            Ok(project) => {
                if format == CheckFormat::Json {
                    println!(
                        "{}",
                        check_report::render(path, Some(CheckKind::Project), &Diagnostics::new())
                    );
                    return ExitCode::SUCCESS;
                }
                println!(
                    "checked project `{}`: {} source file(s), {} service(s), {} model(s), {} route(s), {} page(s), {} scheduled task(s), {} auth policy(ies), auth {}, entry {}",
                    project.name,
                    project.source_files.len(),
                    project.declared_services.len(),
                    project.data_models.len(),
                    project.routes.len(),
                    project.pages.len(),
                    project.scheduled_tasks.len(),
                    project.auth_policies.len(),
                    project.auth_target.as_deref().unwrap_or("none"),
                    project.entry_path.display()
                );
                ExitCode::SUCCESS
            }
            Err(diagnostics) => {
                print_check_diagnostics(path, CheckKind::Project, format, diagnostics);
                ExitCode::from(1)
            }
        }
    } else {
        check_source_file(path, format)
    }
}

fn check_source_file(path: &str, format: CheckFormat) -> ExitCode {
    if !path.ends_with(".svr") {
        eprintln!("svr: source path `{path}` must have a .svr extension");
        return ExitCode::from(2);
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            print_check_io_error(
                path,
                Some(CheckKind::Source),
                format,
                format!("svr: cannot read `{path}`: {error}"),
            );
            return ExitCode::from(1);
        }
    };
    let program = match compiler::parser::Parser::new().parse_source(&source) {
        Ok(program) => program,
        Err(diagnostics) => {
            print_check_diagnostics(path, CheckKind::Source, format, diagnostics);
            return ExitCode::from(1);
        }
    };
    if let Err(diagnostics) = compiler::semantic::SemanticAnalyzer::new().analyze(&program) {
        print_check_diagnostics(path, CheckKind::Source, format, diagnostics);
        return ExitCode::from(1);
    }
    if format == CheckFormat::Json {
        println!(
            "{}",
            check_report::render(path, Some(CheckKind::Source), &Diagnostics::new())
        );
    } else {
        println!("checked source `{path}`");
    }
    ExitCode::SUCCESS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckFormat {
    Human,
    Json,
}

fn parse_check_args(args: &[String]) -> Result<(CheckFormat, &str), String> {
    let mut format = None;
    let mut path = None;
    let mut positional_only = false;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        if !positional_only && argument == "--" {
            positional_only = true;
            continue;
        }
        let format_value = if !positional_only && argument == "--format" {
            Some(
                arguments
                    .next()
                    .ok_or("svr: --format expects `human` or `json`")?
                    .as_str(),
            )
        } else if !positional_only {
            argument.strip_prefix("--format=")
        } else {
            None
        };
        if let Some(value) = format_value {
            if format.is_some() {
                return Err("svr: --format may only be specified once".to_owned());
            }
            format = Some(match value {
                "human" => CheckFormat::Human,
                "json" => CheckFormat::Json,
                _ => {
                    return Err(format!(
                        "svr: unsupported check format `{value}`; expected `human` or `json`"
                    ))
                }
            });
        } else {
            if !positional_only && argument.starts_with('-') {
                return Err(format!("svr: unknown check option `{argument}`"));
            }
            if path.replace(argument.as_str()).is_some() {
                return Err(
                    "svr: check expects exactly one source path or project directory".to_owned(),
                );
            }
        }
    }
    let path = path.ok_or("svr: check expects exactly one source path or project directory")?;
    Ok((format.unwrap_or(CheckFormat::Human), path))
}

fn print_check_diagnostics(
    path: &str,
    kind: CheckKind,
    format: CheckFormat,
    diagnostics: Diagnostics,
) {
    if format == CheckFormat::Json {
        println!("{}", check_report::render(path, Some(kind), &diagnostics));
    } else {
        print_diagnostics(diagnostics);
    }
}

fn print_check_io_error(path: &str, kind: Option<CheckKind>, format: CheckFormat, message: String) {
    if format == CheckFormat::Human {
        eprintln!("{message}");
        return;
    }
    let diagnostics = Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code: "E0001",
            message,
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        }],
    };
    println!("{}", check_report::render(path, kind, &diagnostics));
}

fn print_diagnostics(diagnostics: compiler::diagnostics::Diagnostics) {
    for diagnostic in diagnostics.items {
        let file_prefix = diagnostic
            .source_file
            .as_deref()
            .map(|file| format!("{file}:"))
            .unwrap_or_default();
        eprintln!(
            "error[{}] at {}{}:{}: {}",
            diagnostic.code,
            file_prefix,
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
    fn check_treats_help_after_terminator_as_a_path() {
        // Exercise dispatch as well as option parsing without a subprocess.
        // Neither target exists in the repository working directory.
        for path in ["--help", "-h"] {
            assert_eq!(
                run(["check", "--format=json", "--", path]),
                ExitCode::from(1)
            );
        }
    }

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
