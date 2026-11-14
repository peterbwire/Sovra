//! Explicit v2 source commands; default command behavior is unchanged.
use super::{print_check_diagnostics, print_check_io_error, CheckFormat, CheckKind};
use crate::compiler::{
    self,
    diagnostics::{diagnostic_message, Diagnostics},
};
use std::{path::Path, process::ExitCode};
fn usage(message: &str) -> ExitCode {
    eprintln!(
        "{}",
        compiler::diagnostics::terminal_message(format_args!("svr: {message}"))
    );
    ExitCode::from(2)
}
fn arguments(
    args: &[String],
    allow_symbols: bool,
) -> Result<(&str, CheckFormat, bool, bool), &'static str> {
    let (mut path, mut format, mut selected, mut i, mut positional, mut locked) =
        (None, None, false, 0, false, false);
    let mut symbols = false;
    while i < args.len() {
        let arg = args[i].as_str();
        if !positional && arg == "--" {
            positional = true;
            i += 1;
            continue;
        }
        if !positional && arg == "--experimental-v2" {
            if selected {
                return Err("--experimental-v2 may appear only once");
            }
            selected = true;
        } else if !positional && allow_symbols && arg == "--symbols" {
            if symbols {
                return Err("--symbols may appear only once");
            }
            symbols = true;
        } else if !positional && arg == "--locked" {
            if locked {
                return Err("--locked may appear only once");
            }
            locked = true;
        } else if !positional && arg == "--format" {
            if format.is_some() {
                return Err("--format may appear only once");
            }
            i += 1;
            format = Some(match args.get(i).map(String::as_str) {
                Some("human") => CheckFormat::Human,
                Some("json") => CheckFormat::Json,
                _ => return Err("--format requires human or json"),
            });
        } else if !positional && arg.starts_with('-') {
            return Err("--experimental-v2 supports only --locked, --format and one input");
        } else if path.replace(arg).is_some() {
            return Err("--experimental-v2 requires exactly one input");
        }
        i += 1;
    }
    if !selected {
        return Err("missing --experimental-v2");
    }
    Ok((
        path.ok_or("--experimental-v2 requires a source file or package directory")?,
        format.unwrap_or(CheckFormat::Human),
        locked,
        symbols,
    ))
}
pub(super) fn check(args: &[String]) -> ExitCode {
    let (path, format, locked, symbols) = match arguments(args, true) {
        Ok(v) => v,
        Err(e) => return usage(e),
    };
    if symbols && (format != CheckFormat::Json || Path::new(path).is_dir()) {
        return usage("v2 --symbols requires standalone --format json");
    }
    let source = match read_source(path, format, locked) {
        Ok(source) => source,
        Err(status) => return status,
    };
    match compiler::v2::check_program_source(
        &source.label,
        &source.text,
        compiler::v2::ProgramSourceLimits::DEFAULT,
    ) {
        Ok(checked) => {
            if format == CheckFormat::Json {
                let mut report =
                    compiler::check_report::render(path, Some(source.kind), &Diagnostics::new());
                if symbols {
                    report.pop();
                    report.push_str(",\"symbols\":");
                    report.push_str(
                        &checked.render_symbols(compiler::v2::ProgramSymbolLimits::DEFAULT),
                    );
                    report.push('}');
                }
                println!("{report}");
            } else {
                println!(
                    "{}",
                    compiler::diagnostics::terminal_message(format_args!(
                        "checked experimental v2 {} `{path}`",
                        if source.kind == CheckKind::Project {
                            "package"
                        } else {
                            "source"
                        }
                    ))
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            print_check_diagnostics(
                path,
                source.kind,
                format,
                source.locate(e.into_diagnostics()),
            );
            ExitCode::from(1)
        }
    }
}

struct SourceInput {
    text: String,
    label: String,
    kind: CheckKind,
}
impl SourceInput {
    fn locate(&self, mut diagnostics: Diagnostics) -> Diagnostics {
        for diagnostic in &mut diagnostics.items {
            if diagnostic.source_file.is_none() && diagnostic.span.end > 0 {
                diagnostic.source_file = Some(self.label.clone());
            }
        }
        diagnostics
    }
}
enum SourceError {
    Usage(&'static str),
    Io(String),
    Diagnostics(Diagnostics, CheckKind),
}
fn source(path: &str, locked: bool) -> Result<SourceInput, SourceError> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            let captured = compiler::project::packages::capture_root_source(path)
                .map_err(|errors| SourceError::Diagnostics(errors, CheckKind::Project))?;
            if locked {
                compiler::project::packages::verify_existing_root_source_lock(&captured)
                    .map_err(|errors| SourceError::Diagnostics(errors, CheckKind::Project))?;
            }
            let (entry, text) = captured.into_source();
            return Ok(SourceInput {
                text,
                label: entry.to_string_lossy().into_owned(),
                kind: CheckKind::Project,
            });
        }
        Ok(metadata) if metadata.is_file() => {
            if locked {
                return Err(SourceError::Usage("--locked requires a package directory"));
            }
        }
        Ok(_) => {
            return Err(SourceError::Usage(
                "experimental input must be a regular .svr file or single-entry package directory",
            ))
        }
        Err(error) => {
            if locked {
                return Err(SourceError::Usage("--locked requires a package directory"));
            }
            if !path.ends_with(".svr") {
                return Err(SourceError::Usage(
                    "experimental input must be a .svr file or existing package directory",
                ));
            }
            return Err(SourceError::Io(diagnostic_message!(
                "svr: cannot inspect `{path}`: {error}"
            )));
        }
    }
    if !path.ends_with(".svr") {
        return Err(SourceError::Usage(
            "experimental source path must have a .svr extension",
        ));
    }
    let text = compiler::resources::read_source(Path::new(path)).map_err(|error| match error {
        compiler::resources::SourceReadError::Limit(d) => {
            SourceError::Diagnostics(d, CheckKind::Source)
        }
        compiler::resources::SourceReadError::Io(e) => {
            SourceError::Io(diagnostic_message!("svr: cannot read `{path}`: {e}"))
        }
    })?;
    Ok(SourceInput {
        text,
        label: path.into(),
        kind: CheckKind::Source,
    })
}
fn read_source(path: &str, format: CheckFormat, locked: bool) -> Result<SourceInput, ExitCode> {
    match source(path, locked) {
        Ok(source) => Ok(source),
        Err(SourceError::Usage(error)) => Err(usage(error)),
        Err(SourceError::Io(error)) => {
            print_check_io_error(path, Some(CheckKind::Source), format, error);
            Err(ExitCode::from(1))
        }
        Err(SourceError::Diagnostics(errors, kind)) => {
            print_check_diagnostics(path, kind, format, errors);
            Err(ExitCode::from(1))
        }
    }
}

/// Fixed finite CLI profile. Library defaults deliberately remain unchanged.
fn run_policy() -> compiler::v2::RuntimeVmPolicy {
    compiler::v2::command_runtime_policy()
}
struct RunArguments<'a> {
    path: &'a str,
    program_arguments: &'a [String],
    allow_stdin: bool,
    allow_files: bool,
    locked: bool,
}
fn run_arguments(args: &[String]) -> Result<RunArguments<'_>, &'static str> {
    let delimiter = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let (mut path, mut selected, mut allow_stdin, mut allow_files, mut locked) =
        (None, false, false, false, false);
    for arg in &args[..delimiter] {
        match arg.as_str() {
            "--experimental-v2" => {
                if selected { return Err("--experimental-v2 may appear only once"); }
                selected = true;
            },
            "--locked" => {
                if locked { return Err("--locked may appear only once"); }
                locked=true;
            },
            "--allow-stdin" => {
                if allow_stdin { return Err("--allow-stdin may appear only once"); }
                allow_stdin = true;
            },
            "--allow-files" => {
                if allow_files { return Err("--allow-files may appear only once"); }
                allow_files = true;
            },
            _ if arg.starts_with('-') => return Err("experimental run supports only --experimental-v2, --locked, --allow-stdin, --allow-files and one input"),
            _ if path.replace(arg.as_str()).is_some() => return Err("experimental run requires exactly one input"),
            _ => {},
        }
    }
    if !selected {
        return Err("missing --experimental-v2");
    }
    Ok(RunArguments {
        path: path.ok_or("experimental run requires an input before --")?,
        program_arguments: args.get(delimiter + 1..).unwrap_or(&[]),
        allow_stdin,
        allow_files,
        locked,
    })
}
pub(super) fn run(args: &[String]) -> ExitCode {
    use compiler::v2::*;
    let RunArguments {
        path,
        program_arguments,
        allow_stdin,
        allow_files,
        locked,
    } = match run_arguments(args) {
        Ok(arguments) => arguments,
        Err(error) => return usage(error),
    };
    let source = match read_source(path, CheckFormat::Human, locked) {
        Ok(source) => source,
        Err(status) => return status,
    };
    let checked =
        match check_program_source(&source.label, &source.text, ProgramSourceLimits::DEFAULT) {
            Ok(checked) => checked,
            Err(error) => {
                print_check_diagnostics(
                    path,
                    source.kind,
                    CheckFormat::Human,
                    source.locate(error.into_diagnostics()),
                );
                return ExitCode::from(1);
            }
        };
    let entry = match checked.function_id("main") {
        Some(entry) => entry,
        None => {
            cli_error!("svr: experimental execution requires a parameterless Unit main function");
            return ExitCode::from(1);
        }
    };
    let mut input = compiler::interpreter::ProcessHost::new(Vec::new());
    let mut files = compiler::interpreter::ProcessHost::new(Vec::new());
    let services = RuntimeVmServices {
        input: allow_stdin.then_some(&mut input as &mut dyn RuntimeVmInput),
        files: allow_files.then_some(&mut files as &mut dyn RuntimeVmFiles),
    };
    let outcome = match checked.execute_with_services(
        entry,
        &[],
        program_arguments,
        services,
        ProgramLoweringLimits::DEFAULT,
        run_policy(),
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            cli_error!("svr: {error}");
            return ExitCode::from(1);
        }
    };
    // Output is published only after complete execution succeeds. Program text is
    // intentionally not terminal-escaped; write errors are reported without panic.
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    for line in outcome.execution.output {
        if let Err(error) = writeln!(stdout, "{line}") {
            cli_error!("svr: cannot publish program output: {error}");
            return ExitCode::from(1);
        }
    }
    if let Err(error) = stdout.flush() {
        cli_error!("svr: cannot flush program output: {error}");
        return ExitCode::from(1);
    }
    ExitCode::from(outcome.exit_code)
}

pub(super) fn test(args: &[String]) -> ExitCode {
    use compiler::{
        testing::{self, TestReport},
        v2::*,
    };
    let (path, format, locked, _) = match arguments(args, false) {
        Ok(arguments) => arguments,
        Err(error) => return usage(error),
    };
    let mut report = TestReport {
        target: path.into(),
        package: false,
        diagnostics: Diagnostics::new(),
        tests: Vec::new(),
    };
    match source(path, locked) {
        Err(SourceError::Usage(error)) => return usage(error),
        Err(SourceError::Io(message)) => {
            report
                .diagnostics
                .items
                .push(compiler::diagnostics::Diagnostic {
                    code: "E0001",
                    message,
                    severity: compiler::diagnostics::Severity::Error,
                    source_file: None,
                    span: compiler::diagnostics::Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                })
        }
        Err(SourceError::Diagnostics(errors, kind)) => {
            report.package = kind == CheckKind::Project;
            report.diagnostics = errors;
        }
        Ok(source) => {
            match check_program_source(&source.label, &source.text, ProgramSourceLimits::DEFAULT) {
                Ok(checked) => {
                    report = checked.test(
                        ProgramLoweringLimits::DEFAULT,
                        run_policy(),
                        ProgramTestLimits::DEFAULT,
                    )
                }
                Err(error) => report.diagnostics = error.into_diagnostics(),
            }
            report.target = path.into();
            report.package = source.kind == CheckKind::Project;
            report.diagnostics = source.locate(report.diagnostics);
        }
    }
    let rendered = if format == CheckFormat::Json {
        testing::render_json(&report) + "\n"
    } else {
        testing::render_human(&report)
    };
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = stdout
        .write_all(rendered.as_bytes())
        .and_then(|_| stdout.flush())
    {
        cli_error!("svr: cannot publish test report: {error}");
        return ExitCode::from(1);
    }
    if report.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn build_arguments(args: &[String]) -> Result<(&str, bool, Option<&str>), &'static str> {
    let (mut path, mut selected, mut emit, mut positional, mut index, mut locked) =
        (None, false, false, false, 0, false);
    let mut native = false;
    let mut output = None;
    while index < args.len() {
        let arg = args[index].as_str();
        if !positional && arg == "--" {
            positional = true;
        } else if !positional && arg == "--experimental-v2" {
            if selected {
                return Err("--experimental-v2 may appear only once");
            }
            selected = true;
        } else if !positional && arg == "--locked" {
            if locked {
                return Err("--locked may appear only once");
            }
            locked = true;
        } else if !positional && arg == "--emit" {
            if emit {
                return Err("--emit may appear only once");
            }
            emit = true;
            index += 1;
            match args.get(index).map(String::as_str) {
                Some("ir") => {}
                Some("exe") => native = true,
                _ => return Err(
                    "experimental build supports only --emit ir|exe; v2 JS emission is unavailable",
                ),
            }
        } else if !positional && arg == "--output" {
            if output.is_some() {
                return Err("--output may appear only once");
            }
            index += 1;
            output = Some(
                args.get(index)
                    .map(String::as_str)
                    .filter(|v| !v.starts_with('-'))
                    .ok_or("--output requires a path")?,
            );
        } else if !positional && arg.starts_with('-') {
            return Err(
                "experimental build supports only --experimental-v2, --locked, --emit ir|exe, --output and one input",
            );
        } else if path.replace(arg).is_some() {
            return Err("experimental build requires exactly one input");
        }
        index += 1;
    }
    if !selected {
        return Err("missing --experimental-v2");
    }
    if native != output.is_some() {
        return Err("--emit exe requires --output PATH; --output is unavailable for IR");
    }
    Ok((
        path.ok_or("experimental build requires a source file or package directory")?,
        locked,
        output,
    ))
}
pub(super) fn build(args: &[String]) -> ExitCode {
    use compiler::v2::*;
    let (path, locked, output) = match build_arguments(args) {
        Ok(arguments) => arguments,
        Err(error) => return usage(error),
    };
    let source = match read_source(path, CheckFormat::Human, locked) {
        Ok(source) => source,
        Err(status) => return status,
    };
    let checked =
        match check_program_source(&source.label, &source.text, ProgramSourceLimits::DEFAULT) {
            Ok(checked) => checked,
            Err(error) => {
                print_check_diagnostics(
                    path,
                    source.kind,
                    CheckFormat::Human,
                    source.locate(error.into_diagnostics()),
                );
                return ExitCode::from(1);
            }
        };
    if let Some(output) = output {
        return match crate::native_build::build_v2(&checked, Path::new(output)) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                cli_error!("svr: native v2 build error: {error}");
                ExitCode::from(1)
            }
        };
    }
    let text = match checked.render_ir(ProgramLoweringLimits::DEFAULT, ProgramIrLimits::DEFAULT) {
        Ok(text) => text,
        Err(error) => {
            cli_error!("svr: {error}");
            return ExitCode::from(1);
        }
    };
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = stdout
        .write_all(text.as_bytes())
        .and_then(|_| stdout.flush())
    {
        cli_error!("svr: cannot publish IR inspection: {error}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

pub(super) fn lock(args: &[String]) -> ExitCode {
    if args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| arg == "--help" || arg == "-h")
    {
        println!("Usage: svr lock --experimental-v2 [--check] <package-directory>");
        return ExitCode::SUCCESS;
    }
    let (mut selected, mut check, mut path, mut positional) = (false, false, None, false);
    for arg in args {
        if !positional && arg == "--" {
            positional = true;
        } else if !positional && arg == "--experimental-v2" {
            if selected {
                return usage("--experimental-v2 may appear only once");
            }
            selected = true;
        } else if !positional && arg == "--check" {
            if check {
                return usage("--check may appear only once");
            }
            check = true;
        } else if !positional && arg.starts_with('-') {
            return usage("experimental lock supports only --check and one package directory");
        } else if path.replace(arg.as_str()).is_some() {
            return usage("experimental lock requires one package directory");
        }
    }
    let Some(path) = path.filter(|path| selected && Path::new(path).is_dir()) else {
        return usage("experimental lock requires a package directory");
    };
    let result = if check {
        compiler::project::packages::check_root_source_lock(path)
    } else {
        compiler::project::packages::write_root_source_lock(path)
    };
    match result {
        Ok(()) => {
            use std::io::Write;
            let mut stdout = std::io::stdout().lock();
            if let Err(error) = writeln!(
                stdout,
                "source lock {}",
                if check { "verified" } else { "generated" }
            )
            .and_then(|_| stdout.flush())
            {
                cli_error!("svr: cannot publish lock status: {error}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(errors) => {
            print_check_diagnostics(path, CheckKind::Project, CheckFormat::Human, errors);
            ExitCode::from(1)
        }
    }
}
