//! Command-line parsing for `svr`.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::compiler;
use crate::compiler::check_report::{self, CheckKind};
use crate::compiler::diagnostics::diagnostic_message;
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::project_new;

// Keep CLI-generated error lines printable without changing program output or
// machine-readable reports. Formatting arguments stream through the escape layer.
macro_rules! cli_error {
    ($($arguments:tt)*) => {
        eprintln!("{}", $crate::compiler::diagnostics::terminal_message(format_args!($($arguments)*)))
    };
}

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
  lock             Generate or verify a local source lock
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
        cli_error!("svr: unexpected arguments after {command}");
        return ExitCode::from(2);
    }

    if matches!(command, "--version" | "-V") {
        cli_error!("svr: unexpected arguments after {command}");
        return ExitCode::from(2);
    }

    if !compiler::is_known_command(command) {
        cli_error!("svr: unknown command '{command}'");
        cli_error!("Try 'svr --help' for available commands.");
        return ExitCode::from(2);
    }

    if command == "lock" {
        return lock_command(args);
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
            println!(
                "Usage: svr test [--locked] [--format human|json] <source.svr|package-directory>"
            );
            println!("  Run root-level parameterless Unit test_ functions; stdin is unavailable.");
        } else if command == "build" {
            println!("Usage: svr build [--locked] [--emit ir|js] <source.svr|package-directory>");
            println!("       svr build --emit exe --output PATH <source.svr|package-directory>");
            println!("  exe: host-native output; Rust/Cargo and a host linker are build-time requirements.");
        } else if command == "check" {
            println!("Usage: svr check [--format human|json] [--service-calls] [--symbols] <source.svr|project-directory>");
            println!("       svr check --format json --overlays-stdin <package-directory>");
            println!("  --service-calls  Experimental project-only contract checks; incomplete coverage fails.");
            println!("  --symbols        Opt-in checked symbol/type evidence for successful standalone JSON checks.");
            println!("  --overlays-stdin Framed unsaved package buffers; requires JSON, never executes or writes source.");
            println!("  --package-context Resolve canonical entry paths only; JSON, no source validation.");
            println!("  --locked         Verify sovra.lock; requires --executable-package.");
            println!("  --executable-package Compile all package sources and linked IR; never execute them.");
        } else {
            println!(
                "Usage: svr run [--locked] <source.svr|package-directory> [-- <program-arg>...]"
            );
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
        let locked = compiler_args
            .iter()
            .filter(|arg| arg.as_str() == "--locked")
            .count();
        if locked > 1 {
            cli_error!("svr: --locked may appear only once");
            return ExitCode::from(2);
        }
        let compiler_args: Vec<_> = compiler_args
            .iter()
            .filter(|arg| arg.as_str() != "--locked")
            .cloned()
            .collect();
        let (emit, source_args) = match parse_emit(command, &compiler_args) {
            Ok(parsed) => parsed,
            Err(message) => {
                cli_error!("{message}");
                return ExitCode::from(2);
            }
        };
        if source_args.len() != 1 {
            cli_error!("svr: {command} expects exactly one .svr source path");
            return ExitCode::from(2);
        }
        let path = match source_args.first() {
            Some(path) if std::path::Path::new(path).is_dir() => {
                let compiled = if locked == 1 {
                    compiler::project::packages::compile_locked(Path::new(path))
                        .map(|package| package.into_ir())
                        .map_err(|error| error.into_diagnostics())
                } else {
                    compiler::project::packages::compile_cli(Path::new(path))
                };
                return match compiled {
                    Ok(ir) => execute_ir(command, emit, &ir, program_args),
                    Err(diagnostics) => {
                        print_diagnostics(diagnostics);
                        ExitCode::from(1)
                    }
                };
            }
            Some(path) if path.ends_with(".svr") && locked == 0 => path,
            Some(path) => {
                cli_error!("svr: source path `{path}` must have a .svr extension");
                return ExitCode::from(2);
            }
            None => unreachable!("argument count is validated above"),
        };
        let source = match compiler::resources::read_source(std::path::Path::new(path)) {
            Ok(source) => source,
            Err(compiler::resources::SourceReadError::Limit(diagnostics)) => {
                print_diagnostics(diagnostics);
                return ExitCode::from(1);
            }
            Err(compiler::resources::SourceReadError::Io(error)) => {
                cli_error!("svr: cannot read `{path}`: {error}");
                return ExitCode::from(1);
            }
        };
        let program = match compiler::parser::Parser::new().parse_source(&source) {
            Ok(program) => program,
            Err(diagnostics) => {
                for diagnostic in diagnostics.items {
                    cli_error!("error[{}]: {}", diagnostic.code, diagnostic.message);
                }
                return ExitCode::from(1);
            }
        };
        let (ir, _) = match compiler::compile_program(&program) {
            Ok(compiled) => compiled,
            Err(diagnostics) => {
                print_diagnostics(diagnostics);
                return ExitCode::from(1);
            }
        };
        execute_ir(command, emit, &ir, program_args)
    }
    cli_error!("{message}.");
    cli_error!("See docs/roadmap.md for planned functionality.");
    ExitCode::from(1)
}

fn test_command(args: &[String]) -> ExitCode {
    let mut format = "human";
    let mut seen_format = false;
    let mut locked = false;
    let mut path = None;
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--format" || argument.starts_with("--format=") {
            if seen_format {
                cli_error!("svr: --format may appear only once");
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
                    cli_error!("svr: --format expects human or json");
                    return ExitCode::from(2);
                }
            }
        } else if argument == "--locked" && !locked {
            locked = true;
        } else if argument.starts_with('-') {
            cli_error!("svr: unknown test option `{argument}`");
            return ExitCode::from(2);
        } else if path.replace(argument.as_str()).is_some() {
            cli_error!("svr: test expects exactly one source or package");
            return ExitCode::from(2);
        }
        index += 1;
    }
    let Some(path) = path else {
        cli_error!("svr: test expects exactly one source or package");
        return ExitCode::from(2);
    };
    if !Path::new(path).is_dir() && !path.ends_with(".svr") {
        cli_error!("svr: test source path must end in .svr");
        return ExitCode::from(2);
    }
    if locked && !Path::new(path).is_dir() {
        cli_error!("svr: --locked requires a package directory");
        return ExitCode::from(2);
    }
    let report = if locked {
        compiler::testing::run_locked(path)
    } else {
        compiler::testing::run(path)
    };
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
        cli_error!("svr: new expects exactly one project path");
        return ExitCode::from(2);
    };
    if argument.starts_with('-') {
        cli_error!("svr: new does not accept options; use `svr new <path>`");
        return ExitCode::from(2);
    }
    let path = Path::new(argument);
    let name = match project_new::project_name(path) {
        Ok(name) => name,
        Err(message) => {
            cli_error!("svr: invalid project path `{argument}`: {message}");
            return ExitCode::from(2);
        }
    };
    match project_new::create(path, name) {
        Ok(()) => {
            println!("created project `{}`", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            cli_error!("svr: cannot create project `{}`: {error}", path.display());
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
                    cli_error!("svr: native build error: {error}");
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
            cli_error!("svr: runtime error: {error}");
            ExitCode::from(1)
        }
    }
}

fn check_overlays(path: &str) -> ExitCode {
    use compiler::project::packages;
    let mut navigation = None;
    let (request_id, diagnostics) =
        match packages::read_source_overlay_request(std::io::stdin().lock()) {
            Err(errors) => (None, errors),
            Ok(request) => {
                let result = packages::compile_with_source_navigation(
                    path,
                    &request.sources,
                    packages::SOURCE_OVERLAY_REQUEST_LIMITS,
                );
                (
                    Some(request.request_id),
                    result
                        .map(|result| {
                            navigation = Some(result.navigation);
                            Diagnostics::new()
                        })
                        .unwrap_or_else(|error| error.into_diagnostics()),
                )
            }
        };
    let (mut report, mut success) = overlay_report(path, request_id, &diagnostics);
    if success {
        if let Some(navigation) = navigation {
            report.pop();
            report.push_str(",\"package_symbols\":");
            report.push_str(navigation.as_json());
            report.push('}');
        }
    }
    if report.len() >= 1024 * 1024 {
        let errors = overlay_report_limit();
        (report, success) = overlay_report(path, request_id, &errors);
    }
    println!("{report}");
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn overlay_report_limit() -> Diagnostics {
    Diagnostics {
        items: vec![Diagnostic {
            severity: Severity::Error,
            code: "E4120",
            message: "source-overlay report byte allowance exceeded".to_owned(),
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
            source_file: None,
        }],
    }
}

fn overlay_report(
    path: &str,
    request_id: Option<u32>,
    diagnostics: &Diagnostics,
) -> (String, bool) {
    let mut success = diagnostics.is_empty();
    let mut report = check_report::render(path, Some(CheckKind::Project), diagnostics);
    // Limit published report bytes independently of input/compiler record quotas.
    if report.len() > 1024 * 1024 - 128 {
        success = false;
        report = check_report::render(path, Some(CheckKind::Project), &overlay_report_limit());
    }
    report.pop();
    let _ = write!(
        report,
        ",\"overlay_request\":{{\"schema_version\":1,\"request_id\":{}}}}}",
        request_id.map_or_else(|| "null".into(), |id| id.to_string())
    );
    (report, success)
}

fn package_context(path: &str) -> ExitCode {
    let graph = compiler::project::packages::resolve(path);
    let mut diagnostics = graph.as_ref().err().cloned().unwrap_or_default();
    let mut context = String::from("null");
    if let Ok(graph) = graph {
        let manifests: Vec<_> = graph
            .packages
            .keys()
            .map(|root| root.join("sovra.toml"))
            .collect();
        let paths = std::iter::once(graph.root.as_path())
            .chain(graph.packages.values().map(|p| p.entry.as_path()))
            .chain(manifests.iter().map(|p| p.as_path()));
        if paths
            .clone()
            .any(|p| p.to_str().map_or(true, |s| s.len() > 4096))
        {
            diagnostics = Diagnostics {
                items: vec![Diagnostic {
                    severity: Severity::Error,
                    code: "E4117",
                    message: "package context paths require bounded UTF-8".to_owned(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                    source_file: None,
                }],
            };
        } else {
            context = String::from("{\"schema_version\":1,\"root\":");
            check_report::push_string(&mut context, graph.root.to_str().unwrap());
            context.push_str(",\"entries\":[");
            for (index, package) in graph.packages.values().enumerate() {
                if index > 0 {
                    context.push(',');
                }
                check_report::push_string(&mut context, package.entry.to_str().unwrap());
            }
            context.push_str("],\"manifests\":[");
            for (index, manifest) in manifests.iter().enumerate() {
                if index > 0 {
                    context.push(',');
                }
                check_report::push_string(&mut context, manifest.to_str().unwrap());
            }
            context.push_str("]}");
        }
    }
    let mut success = diagnostics.is_empty();
    let mut report = check_report::render(path, Some(CheckKind::Project), &diagnostics);
    report.pop();
    report.push_str(",\"scope\":\"package_graph\",\"source_check\":false,\"package_context\":");
    report.push_str(&context);
    report.push('}');
    if report.len() >= 1024 * 1024 {
        success = false;
        let errors = Diagnostics {
            items: vec![Diagnostic {
                severity: Severity::Error,
                code: "E4120",
                message: "package context report byte allowance exceeded".to_owned(),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
                source_file: None,
            }],
        };
        report = check_report::render(path, Some(CheckKind::Project), &errors);
        report.pop();
        report.push_str(
            ",\"scope\":\"package_graph\",\"source_check\":false,\"package_context\":null}",
        );
    }
    println!("{report}");
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn check_command(args: &[String]) -> ExitCode {
    let CheckArguments {
        format,
        path,
        service_calls,
        symbols,
        overlays,
        context,
        executable,
        locked,
    } = match parse_check_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            cli_error!("{message}");
            return ExitCode::from(2);
        }
    };
    if locked && !executable {
        cli_error!("svr: --locked requires --executable-package");
        return ExitCode::from(2);
    }
    if executable {
        if context || overlays || symbols || service_calls || Path::new(path).is_file() {
            cli_error!(
                "svr: --executable-package requires a package directory without other check modes"
            );
            return ExitCode::from(2);
        }
        let compiled = if locked {
            compiler::project::packages::compile_locked(Path::new(path))
                .map(|_| ())
                .map_err(|error| error.into_diagnostics())
        } else {
            compiler::project::packages::compile_cli(Path::new(path)).map(|_| ())
        };
        let diagnostics = compiled
            .map(|_| Diagnostics::new())
            .unwrap_or_else(|errors| errors);
        let success = diagnostics.is_empty();
        if format == CheckFormat::Json {
            println!(
                "{}",
                check_report::render(path, Some(CheckKind::Project), &diagnostics)
            );
        } else if success {
            println!(
                "{}",
                compiler::diagnostics::terminal_message(format_args!(
                    "checked executable package `{path}`"
                ))
            );
        } else {
            print_diagnostics(diagnostics);
        }
        return if success {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    if context {
        if format != CheckFormat::Json
            || service_calls
            || symbols
            || overlays
            || path.len() > compiler::project::packages::MAX_OVERLAY_PATH_BYTES
            || Path::new(path).is_file()
        {
            cli_error!("svr: --package-context requires JSON and a package directory without other check modes");
            return ExitCode::from(2);
        }
        return package_context(path);
    }
    if overlays {
        if format != CheckFormat::Json
            || service_calls
            || symbols
            || path.len() > compiler::project::packages::MAX_OVERLAY_PATH_BYTES
            || Path::new(path).is_file()
        {
            cli_error!("svr: --overlays-stdin requires --format json and a package directory; it cannot combine with --symbols or --service-calls");
            return ExitCode::from(2);
        }
        return check_overlays(path);
    }
    if symbols && (format != CheckFormat::Json || service_calls) {
        cli_error!("svr: --symbols requires --format json and a standalone source file");
        return ExitCode::from(2);
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            print_check_io_error(
                path,
                None,
                format,
                diagnostic_message!("svr: cannot inspect `{path}`: {error}"),
            );
            return ExitCode::from(1);
        }
    };
    if symbols && metadata.is_dir() {
        cli_error!("svr: --symbols requires --format json and a standalone source file");
        return ExitCode::from(2);
    }
    if metadata.is_dir() {
        match compiler::project::packages::classify_cli(std::path::Path::new(path)) {
            Ok(
                mode @ (compiler::project::packages::CliPackageMode::EntryGraph
                | compiler::project::packages::CliPackageMode::Registered),
            ) => {
                if service_calls {
                    cli_error!("svr: --service-calls applies to application projects, not executable packages");
                    return ExitCode::from(2);
                }
                let diagnostics =
                    match compiler::project::packages::compile_cli_mode(Path::new(path), mode) {
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
            Ok(compiler::project::packages::CliPackageMode::Application) => {}
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
            cli_error!("svr: --service-calls requires a project directory");
            return ExitCode::from(2);
        }
        check_source_file(path, format, symbols)
    }
}

fn check_source_file(path: &str, format: CheckFormat, symbols: bool) -> ExitCode {
    if !path.ends_with(".svr") {
        cli_error!("svr: source path `{path}` must have a .svr extension");
        return ExitCode::from(2);
    }
    let source = match compiler::resources::read_source(std::path::Path::new(path)) {
        Ok(source) => source,
        Err(compiler::resources::SourceReadError::Limit(diagnostics)) => {
            print_check_diagnostics(path, CheckKind::Source, format, diagnostics);
            return ExitCode::from(1);
        }
        Err(compiler::resources::SourceReadError::Io(error)) => {
            print_check_io_error(
                path,
                Some(CheckKind::Source),
                format,
                diagnostic_message!("svr: cannot read `{path}`: {error}"),
            );
            return ExitCode::from(1);
        }
    };
    if symbols {
        return match compiler::ide_symbols::check(path, &source) {
            Ok(report) => {
                println!("{report}");
                ExitCode::SUCCESS
            }
            Err(diagnostics) => {
                print_check_diagnostics(path, CheckKind::Source, format, diagnostics);
                ExitCode::from(1)
            }
        };
    }
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

struct CheckArguments<'a> {
    format: CheckFormat,
    path: &'a str,
    service_calls: bool,
    symbols: bool,
    overlays: bool,
    context: bool,
    executable: bool,
    locked: bool,
}
fn parse_check_args(args: &[String]) -> Result<CheckArguments<'_>, String> {
    let mut service_calls = false;
    let mut overlays = false;
    let mut context = false;
    let mut executable = false;
    let mut locked = false;
    let mut symbols = false;
    let mut format = None;
    let mut path = None;
    let mut positional_only = false;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        if !positional_only && argument == "--" {
            positional_only = true;
            continue;
        }
        if !positional_only && argument == "--locked" {
            if locked {
                return Err("svr: --locked may appear only once".into());
            }
            locked = true;
            continue;
        }
        if !positional_only && argument == "--executable-package" {
            if executable {
                return Err("svr: --executable-package may only be specified once".into());
            }
            executable = true;
            continue;
        }
        if !positional_only && argument == "--package-context" {
            if context {
                return Err("svr: --package-context may only be specified once".into());
            }
            context = true;
            continue;
        }
        if !positional_only && argument == "--overlays-stdin" {
            if overlays {
                return Err("svr: --overlays-stdin may only be specified once".into());
            }
            overlays = true;
            continue;
        }
        if !positional_only && argument == "--symbols" {
            if symbols {
                return Err("svr: --symbols may only be specified once".into());
            }
            symbols = true;
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
    Ok(CheckArguments {
        format: format.unwrap_or(CheckFormat::Human),
        path,
        service_calls,
        symbols,
        overlays,
        context,
        executable,
        locked,
    })
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
    let diagnostic = Diagnostic {
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
    }
    .bounded();
    if format == CheckFormat::Human {
        cli_error!("{}", diagnostic.message);
        return;
    }
    let diagnostics = Diagnostics {
        items: vec![diagnostic],
    };
    println!("{}", check_report::render(path, kind, &diagnostics));
}

fn print_diagnostics(diagnostics: compiler::diagnostics::Diagnostics) {
    for diagnostic in diagnostics.items {
        if let Some(file) = diagnostic.source_file.as_deref() {
            cli_error!(
                "error[{}] at {}:{}:{}: {}",
                diagnostic.code,
                file,
                diagnostic.span.line + 1,
                diagnostic.span.column + 1,
                diagnostic.message
            );
        } else {
            cli_error!(
                "error[{}] at {}:{}: {}",
                diagnostic.code,
                diagnostic.span.line + 1,
                diagnostic.span.column + 1,
                diagnostic.message
            );
        }
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

fn lock_command(args: &[String]) -> ExitCode {
    use compiler::project::packages;
    if matches!(args, [arg] if arg == "--help" || arg == "-h") {
        println!("Usage: svr lock [--check] <package-directory>");
        return ExitCode::SUCCESS;
    }
    let mut check = false;
    let mut path = None;
    for arg in args {
        if arg == "--check" && !check {
            check = true;
        } else if arg.starts_with('-') || path.replace(arg).is_some() {
            cli_error!("svr: lock expects [--check] and one package directory");
            return ExitCode::from(2);
        }
    }
    let Some(path) = path.filter(|path| Path::new(path).is_dir()) else {
        cli_error!("svr: lock requires a package directory");
        return ExitCode::from(2);
    };
    let result = if check {
        packages::check_source_lock(Path::new(path))
    } else {
        packages::write_source_lock(Path::new(path))
    };
    match result {
        Ok(()) => {
            println!(
                "{}",
                if check {
                    "source lock verified"
                } else {
                    "source lock generated"
                }
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            print_diagnostics(errors);
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod overlay_report_tests {
    use super::*;
    #[test]
    fn oversized_report_becomes_one_failure_with_correlation_and_null_location() {
        let diagnostic = Diagnostic {
            severity: Severity::Error,
            code: "E3004",
            message: "x".repeat(4096),
            span: Span {
                start: 1,
                end: 2,
                line: 0,
                column: 1,
            },
            source_file: Some("p".repeat(4096)),
        };
        let errors = Diagnostics {
            items: vec![diagnostic; 256],
        };
        let (report, success) = overlay_report("package", Some(u32::MAX), &errors);
        assert!(!success);
        assert!(report.len() < 1024 * 1024);
        assert!(report.contains("E4120"));
        assert!(!report.contains("E3004"));
        assert!(report.contains("\"location\":null"));
        assert!(report.contains("\"request_id\":4294967295"));
    }
}
