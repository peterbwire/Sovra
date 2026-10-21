//! Command-line parsing for `svr`.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::compiler;
use crate::compiler::check_report::{self, CheckKind};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::project_new;

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
  new              Create a console Sovra project
  init             Initialize a Sovra project (planned)
  run              Compile and run a Sovra program
  build            Compile a Sovra source file
  test             Run checked Sovra test functions
  check            Check a Sovra source file or project
  doctor           Diagnose toolchain prerequisites without repairs
  fmt              Format a standalone Sovra source file
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

    if command == "doctor" {
        return crate::doctor::command(args);
    }
    if command == "fmt" {
        return crate::formatter::command(args);
    }

    let help_requested = if command == "check" || command == "run" {
        args.iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg == "--help" || arg == "-h")
    } else {
        args.iter().any(|arg| arg == "--help" || arg == "-h")
    };
    if help_requested && matches!(command, "new" | "run" | "build" | "check" | "test") {
        if command == "new" {
            println!("Usage: svr new <path>");
            println!("  Create a console project; the destination must not exist.");
        } else if command == "test" {
            println!("Usage: svr test [--format human|json] <source.svr|package-directory>");
            println!("  Run root-level parameterless Unit test_ functions; stdin is unavailable.");
        } else if command == "build" {
            println!("Usage: svr build [--emit ir|js] <source.svr|package-directory>");
            println!("       svr build --emit exe --output PATH <source.svr|package-directory>");
            println!("  exe: host-native output; Rust/Cargo and a host linker are build-time requirements.");
        } else if command == "check" {
            println!("Usage: svr check [--format human|json] [--service-calls] <source.svr|project-directory>");
            println!("  --service-calls  Experimental project-only contract checks; incomplete coverage fails.");
        } else {
            println!("Usage: svr run <source.svr|package-directory> [-- <program-arg>...]");
        }
        return ExitCode::SUCCESS;
    }

    if help_requested {
        println!("{command} is planned for a future Sovra milestone.");
        return ExitCode::SUCCESS;
    }

    if command == "new" {
        return new_command(args);
    }

    if command == "run" || command == "build" {
        return compile_command(command, args);
    }
    if command == "check" {
        return check_command(args);
    }
    if command == "test" {
        return test_command(args);
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
        let (compiler_args, program_args) = if command == "run" {
            match args.iter().position(|argument| argument == "--") {
                Some(index) => (&args[..index], &args[index + 1..]),
                None => (args, &[][..]),
            }
        } else {
            (args, &[][..])
        };
        let (emit, source_args) = match parse_emit(command, compiler_args) {
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
            Some(path) if std::path::Path::new(path).is_dir() => {
                return match compiler::project::packages::compile(path) {
                    Ok(ir) => execute_ir(command, emit, &ir, program_args),
                    Err(diagnostics) => {
                        print_diagnostics(diagnostics);
                        ExitCode::from(1)
                    }
                };
            }
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
        execute_ir(command, emit, &ir, program_args)
    }
    eprintln!("{message}.");
    eprintln!("See docs/roadmap.md for planned functionality.");
    ExitCode::from(1)
}

fn test_command(args: &[String]) -> ExitCode {
    let mut format = "human";
    let mut seen_format = false;
    let mut path = None;
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--format" || argument.starts_with("--format=") {
            if seen_format {
                eprintln!("svr: --format may appear only once");
                return ExitCode::from(2);
            }
            seen_format = true;
            let value = if argument == "--format" {
                index += 1;
                args.get(index).map(String::as_str)
            } else {
                argument.strip_prefix("--format=")
            };
            match value {
                Some(value @ ("human" | "json")) => format = value,
                _ => {
                    eprintln!("svr: --format expects human or json");
                    return ExitCode::from(2);
                }
            }
        } else if argument.starts_with('-') {
            eprintln!("svr: unknown test option `{argument}`");
            return ExitCode::from(2);
        } else if path.replace(argument.as_str()).is_some() {
            eprintln!("svr: test expects exactly one source or package");
            return ExitCode::from(2);
        }
        index += 1;
    }
    let Some(path) = path else {
        eprintln!("svr: test expects exactly one source or package");
        return ExitCode::from(2);
    };
    if !Path::new(path).is_dir() && !path.ends_with(".svr") {
        eprintln!("svr: test source path must end in .svr");
        return ExitCode::from(2);
    }
    let report = compiler::testing::run(path);
    if format == "json" {
        println!("{}", compiler::testing::render_json(&report));
    } else if report.diagnostics.is_empty() {
        print!("{}", compiler::testing::render_human(&report));
    } else {
        eprint!("{}", compiler::testing::render_human(&report));
    }
    if report.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn new_command(args: &[String]) -> ExitCode {
    let [argument] = args else {
        eprintln!("svr: new expects exactly one project path");
        return ExitCode::from(2);
    };
    if argument.starts_with('-') {
        eprintln!("svr: new does not accept options; use `svr new <path>`");
        return ExitCode::from(2);
    }
    let path = Path::new(argument);
    let name = match project_new::project_name(path) {
        Ok(name) => name,
        Err(message) => {
            eprintln!("svr: invalid project path `{argument}`: {message}");
            return ExitCode::from(2);
        }
    };
    match project_new::create(path, name) {
        Ok(()) => {
            println!("created project `{}`", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("svr: cannot create project `{}`: {error}", path.display());
            ExitCode::from(1)
        }
    }
}

fn execute_ir(
    command: &str,
    emit: Emit,
    ir: &compiler::ir::IrProgram,
    args: &[String],
) -> ExitCode {
    if command == "build" {
        match emit {
            Emit::Ir => print!("{}", compiler::backend::render(ir)),
            Emit::Js => print!("{}", compiler::backend::render_javascript(ir)),
            Emit::Exe(output) => match crate::native_build::build(ir, Path::new(&output)) {
                Ok(path) => println!("{}", path.display()),
                Err(error) => {
                    eprintln!("svr: native build error: {error}");
                    return ExitCode::from(1);
                }
            },
        }
        return ExitCode::SUCCESS;
    }
    let mut host = compiler::interpreter::ProcessHost::new(args.to_vec());
    match compiler::interpreter::run_with_host_status(ir, &mut host) {
        Ok(outcome) => ExitCode::from(outcome.exit_code),
        Err(error) => {
            eprintln!("svr: runtime error: {error}");
            ExitCode::from(1)
        }
    }
}

fn check_command(args: &[String]) -> ExitCode {
    let (format, path, service_calls) = match parse_check_args(args) {
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
        match compiler::project::packages::declares_dependencies(std::path::Path::new(path)) {
            Ok(true) => {
                if service_calls {
                    eprintln!("svr: --service-calls applies to application projects, not executable dependency packages");
                    return ExitCode::from(2);
                }
                let diagnostics = match compiler::project::packages::compile(path) {
                    Ok(_) => Diagnostics::new(),
                    Err(diagnostics) => diagnostics,
                };
                let success = diagnostics.is_empty();
                if format == CheckFormat::Json {
                    println!(
                        "{}",
                        check_report::render(path, Some(CheckKind::Project), &diagnostics)
                    );
                } else if success {
                    println!("checked executable package `{path}`");
                } else {
                    print_diagnostics(diagnostics);
                }
                return if success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                };
            }
            Err(diagnostics) => {
                print_check_diagnostics(path, CheckKind::Project, format, diagnostics);
                return ExitCode::from(1);
            }
            Ok(false) => {}
        }
        match compiler::project::check_project(path) {
            Ok(project) => {
                if service_calls {
                    let report = compiler::project::application::check_service_calls(&project);
                    let success = report.diagnostics.is_empty();
                    if format == CheckFormat::Json {
                        println!("{}", check_report::render_service_check(path, &report));
                    } else {
                        let inspected = report
                            .files
                            .iter()
                            .filter(|file| file.functions.is_ok())
                            .count();
                        println!(
                            "service-call inspection: {inspected}/{} file(s) covered",
                            report.files.len()
                        );
                        print_diagnostics(report.diagnostics);
                    }
                    return if success {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    };
                }
                if format == CheckFormat::Json {
                    println!("{}", check_report::render_project(path, &project));
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
        if service_calls {
            eprintln!("svr: --service-calls requires a project directory");
            return ExitCode::from(2);
        }
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

fn parse_check_args(args: &[String]) -> Result<(CheckFormat, &str, bool), String> {
    let mut service_calls = false;
    let mut format = None;
    let mut path = None;
    let mut positional_only = false;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        if !positional_only && argument == "--" {
            positional_only = true;
            continue;
        }
        if !positional_only && argument == "--service-calls" {
            if service_calls {
                return Err("svr: --service-calls may only be specified once".into());
            }
            service_calls = true;
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
    Ok((format.unwrap_or(CheckFormat::Human), path, service_calls))
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum Emit {
    Ir,
    Js,
    Exe(String),
}

fn parse_emit(command: &str, args: &[String]) -> Result<(Emit, Vec<String>), String> {
    if command != "build" {
        return Ok((Emit::Ir, args.to_vec()));
    }
    let mut emit = Emit::Ir;
    let mut source_args = Vec::new();
    let mut output = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--emit" {
            let Some(value) = args.get(index + 1) else {
                return Err("svr: --emit expects `ir`, `js` or `exe`".to_owned());
            };
            emit = parse_emit_value(value)?;
            index += 2;
        } else if let Some(value) = arg.strip_prefix("--emit=") {
            emit = parse_emit_value(value)?;
            index += 1;
        } else if arg == "--output" || arg.starts_with("--output=") {
            let value = if arg == "--output" {
                index += 1;
                args.get(index)
                    .ok_or("svr: --output expects a file path")?
                    .clone()
            } else {
                arg.strip_prefix("--output=").unwrap().to_owned()
            };
            if value.is_empty() || output.replace(value).is_some() {
                return Err("svr: --output requires one nonempty file path".into());
            }
            index += 1;
        } else {
            source_args.push(arg.clone());
            index += 1;
        }
    }
    if matches!(emit, Emit::Exe(_)) {
        let path = output.ok_or("svr: --emit exe requires --output PATH")?;
        if cfg!(windows)
            && !Path::new(&path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return Err("svr: Windows executable output must end in .exe".into());
        }
        emit = Emit::Exe(path);
    } else if output.is_some() {
        return Err("svr: --output is supported only with --emit exe".into());
    }
    Ok((emit, source_args))
}

fn parse_emit_value(value: &str) -> Result<Emit, String> {
    match value {
        "ir" => Ok(Emit::Ir),
        "js" => Ok(Emit::Js),
        "exe" => Ok(Emit::Exe(String::new())),
        _ => Err(format!("svr: unsupported emit target `{value}`")),
    }
}
