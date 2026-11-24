//! Explicit generic source/package commands with bounded host-free native images.
use super::{print_check_diagnostics, print_check_io_error, CheckFormat, CheckKind};
use crate::compiler::{
    self,
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span},
    v2::*,
};
use std::{path::Path, process::ExitCode};
fn usage(message: &str) -> ExitCode {
    cli_error!("svr: {message}");
    ExitCode::from(2)
}
#[derive(Debug)]
struct Arguments<'a> {
    path: &'a str,
    format: CheckFormat,
    output: Option<&'a str>,
    locked: bool,
    symbols: bool,
    collections: bool,
    data: bool,
    host_policy: Option<&'a str>,
    context: bool,
    overlays: bool,
}
fn arguments<'a>(args: &'a [String], command: &str) -> Result<Arguments<'a>, &'static str> {
    let checking = command == "check";
    let building = command == "build";
    let (mut emit, mut output, mut locked) = (false, None, false);
    let mut symbols = false;
    let mut collections = false;
    let mut data = false;
    let mut host_policy = None;
    let mut context = false;
    let mut overlays = false;
    let (mut selected, mut path, mut format, mut positional) = (false, None, None, false);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" if !positional => positional = true,
            "--overlays-stdin" if checking && !positional => { if overlays {return Err("--overlays-stdin may appear only once");} overlays = true; }
            "--package-context" if checking && !positional => {
                if context { return Err("--package-context may appear only once"); }
                context = true;
            }
            "--experimental-generics" if !positional => {
                if selected {return Err("--experimental-generics may appear only once");}
                selected = true;
            }
            "--host-policy" if !positional && command=="run"=>{if host_policy.is_some(){return Err("--host-policy may appear only once");}host_policy=Some(args.next().map(String::as_str).filter(|p|!p.is_empty()&&!p.starts_with('-')).ok_or("--host-policy requires a pathname")?);}
            "--experimental-data" if !positional => {if data{return Err("--experimental-data may appear only once");}data=true;}
            "--experimental-collections" if !positional => {
                if collections{return Err("--experimental-collections may appear only once");}collections=true;
            }
            "--symbols" if checking && !positional => {
                if symbols{return Err("--symbols may appear only once");}symbols=true;
            }
            "--locked" if !positional => {
                if locked {return Err("--locked may appear only once");}
                locked = true;
            }
            "--format" if (checking || command == "test") && !positional => {
                if format.is_some() {return Err("--format may appear only once");}
                format = Some(match args.next().map(String::as_str) {
                    Some("human") => CheckFormat::Human,
                    Some("json") => CheckFormat::Json,
                    _ => return Err("--format requires human or json"),
                });
            }
            "--emit" if building && !positional => {
                if emit {return Err("--emit may appear only once");}
                if args.next().map(String::as_str) != Some("exe") {return Err("generic build requires --emit exe; IR/JS are unavailable");}
                emit = true;
            }
            "--output" if building && !positional => {
                if output.is_some() {return Err("--output may appear only once");}
                output = Some(args.next().map(String::as_str).filter(|p| !p.is_empty() && !p.starts_with('-')).ok_or("--output requires a pathname")?);
            }
            value if !positional && value.starts_with('-') => return Err("generic mode supports --experimental-generics, check/test --format human|json (standalone check --symbols), --locked, build --emit exe --output PATH and one .svr file or root package"),
            value => {
                if path.replace(value).is_some() {return Err("generic mode requires exactly one input; program arguments are unavailable");}
            }
        }
    }
    if host_policy.is_some() && !data {
        return Err("--host-policy requires --experimental-data");
    }
    if !selected {
        return Err("missing --experimental-generics");
    }
    if (context || overlays)
        && (format != Some(CheckFormat::Json)
            || (context && overlays)
            || symbols
            || locked
            || collections
            || data
            || !path.is_some_and(|p| Path::new(p).is_dir()))
    {
        return Err("generic package context/overlays require JSON and a package directory without other check modes");
    }
    if symbols && (format != Some(CheckFormat::Json) || path.is_some_and(|p| Path::new(p).is_dir()))
    {
        return Err("generic --symbols requires standalone --format json");
    }
    if building && (!emit || output.is_none()) {
        return Err("generic build requires --emit exe --output PATH");
    }
    #[cfg(windows)]
    if output.is_some_and(|p| !p.ends_with(".exe")) {
        return Err("Windows executable output must end in .exe");
    }
    Ok(Arguments {
        path: path.ok_or("generic mode requires a .svr file or root package")?,
        format: format.unwrap_or(CheckFormat::Human),
        output,
        locked,
        symbols,
        collections,
        data,
        host_policy,
        context,
        overlays,
    })
}
#[derive(Debug)]
struct Input {
    text: String,
    label: String,
    kind: CheckKind,
}
// Metadata only: never lex source, verify a source lock, execute, or acquire grants.
fn package_context(path: &str) -> ExitCode {
    use crate::compiler::{check_report, project::packages};
    let selected = packages::inspect_source_inventory(path).and_then(|inventory| {
        let manifests: Vec<_> = inventory
            .graph
            .packages
            .keys()
            .map(|p| p.join("sovra.toml"))
            .collect();
        let mut bytes = 0usize;
        let paths = std::iter::once(inventory.graph.root.as_path())
            .chain(inventory.sources.iter().map(|s| s.canonical_path.as_path()))
            .chain(manifests.iter().map(|p| p.as_path()));
        let bounded = inventory.sources.len() <= 1024
            && manifests.len() <= 1024
            && paths.into_iter().all(|p| {
                p.to_str().is_some_and(|s| {
                    bytes = bytes.saturating_add(s.len());
                    s.len() <= 4096 && bytes <= 128 * 1024
                })
            });
        if !bounded {
            return Err(Diagnostics {
                items: vec![Diagnostic {
                    severity: Severity::Error,
                    code: "E4120",
                    message: "generic package task context exceeds path/count/report bounds".into(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                    source_file: None,
                }],
            });
        }
        Ok((inventory, manifests))
    });
    let errors = selected.as_ref().err().cloned().unwrap_or_default();
    let success = errors.is_empty();
    let mut report = check_report::render(path, Some(CheckKind::Project), &errors);
    report.pop();
    report.push_str(",\"scope\":\"package_graph\",\"source_check\":false,\"profile\":\"experimental-generics\",\"package_context\":");
    if let Ok((inventory, manifests)) = selected {
        report.push_str("{\"schema_version\":2,\"root\":");
        check_report::push_string(
            &mut report,
            inventory.graph.root.to_str().expect("bounded UTF-8"),
        );
        report.push_str(",\"sources\":[");
        for (i, source) in inventory.sources.iter().enumerate() {
            if i > 0 {
                report.push(',');
            }
            check_report::push_string(
                &mut report,
                source.canonical_path.to_str().expect("bounded UTF-8"),
            );
        }
        report.push_str("],\"manifests\":[");
        for (i, manifest) in manifests.iter().enumerate() {
            if i > 0 {
                report.push(',');
            }
            check_report::push_string(&mut report, manifest.to_str().expect("bounded UTF-8"));
        }
        report.push_str("]}");
    } else {
        report.push_str("null");
    }
    report.push('}');
    println!("{report}");
    ExitCode::from(u8::from(!success))
}
fn check_overlays(path: &str) -> ExitCode {
    use crate::compiler::project::packages;
    let mut symbols = None;
    let (id, errors) = match packages::read_source_overlay_request(std::io::stdin().lock()) {
        Err(errors) => (None, errors),
        Ok(request) => (
            Some(request.request_id),
            match check_generic_package_overlays_with_symbols(
                path,
                &request.sources,
                packages::SOURCE_OVERLAY_REQUEST_LIMITS,
                GenericSymbolLimits::DEFAULT,
            ) {
                Ok(value) => {
                    symbols = Some(value);
                    Diagnostics::new()
                }
                Err(errors) => errors,
            },
        ),
    };
    let (mut report, success) = super::overlay_report(path, id, &errors);
    report.pop();
    report.push_str(",\"profile\":\"experimental-generics\"");
    if let Some(symbols) = symbols.filter(|_| success) {
        if report
            .len()
            .saturating_add(symbols.len())
            .saturating_add(32)
            < 1024 * 1024
        {
            report.push_str(",\"package_symbols\":");
            report.push_str(&symbols);
        } else {
            report.push_str(",\"package_symbols\":{\"schema_version\":1,\"unavailable\":true}");
        }
    }
    report.push('}');
    println!("{report}");
    ExitCode::from(u8::from(!success))
}

fn publish_tests(result: &compiler::testing::TestReport, format: CheckFormat) -> ExitCode {
    let text = if format == CheckFormat::Json {
        compiler::testing::render_json(result) + "\n"
    } else {
        compiler::testing::render_human(result)
    };
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(text.as_bytes())
        .and_then(|_| stdout.flush())
        .is_err()
    {
        cli_error!("svr: cannot publish generic test report");
        return ExitCode::from(1);
    }
    if result.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
fn input_errors(
    path: &str,
    kind: CheckKind,
    format: CheckFormat,
    errors: Diagnostics,
    testing: bool,
) {
    if testing {
        let result = compiler::testing::TestReport {
            target: path.into(),
            package: kind == CheckKind::Project,
            diagnostics: errors,
            tests: Vec::new(),
        };
        let _ = publish_tests(&result, format);
    } else {
        print_check_diagnostics(path, kind, format, errors);
    }
}
fn input_io(path: &str, format: CheckFormat, message: String, testing: bool) {
    if testing {
        input_errors(
            path,
            CheckKind::Source,
            format,
            Diagnostics {
                items: vec![Diagnostic {
                    severity: Severity::Error,
                    code: "E0001",
                    message,
                    source_file: None,
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                }
                .bounded()],
            },
            true,
        );
    } else {
        print_check_io_error(path, Some(CheckKind::Source), format, message);
    }
}
fn input(path: &str, format: CheckFormat, locked: bool, testing: bool) -> Result<Input, ExitCode> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            let captured = compiler::project::packages::capture_root_source_with_source_limit(
                path,
                GenericSourceSyntaxLimits::DEFAULT.frontend.max_source_bytes,
            )
            .map_err(|errors| {
                input_errors(path, CheckKind::Project, format, errors, testing);
                ExitCode::from(1)
            })?;
            if locked {
                compiler::project::packages::verify_existing_root_source_lock(&captured).map_err(
                    |errors| {
                        input_errors(path, CheckKind::Project, format, errors, testing);
                        ExitCode::from(1)
                    },
                )?;
            }
            let (entry, text) = captured.into_source();
            Ok(Input {
                text,
                label: entry.to_string_lossy().into_owned(),
                kind: CheckKind::Project,
            })
        }
        Ok(metadata) if !metadata.is_file() => Err(usage(
            "generic input must be a regular .svr file or root package directory",
        )),
        metadata => {
            if locked {
                return Err(usage("--locked requires a package directory"));
            }
            if !path.ends_with(".svr") {
                return Err(usage("generic source path must have a .svr extension"));
            }
            if let Err(error) = metadata {
                input_io(
                    path,
                    format,
                    diagnostic_message!("svr: cannot inspect `{path}`: {error}"),
                    testing,
                );
                return Err(ExitCode::from(1));
            }
            let text = compiler::resources::read_source_with_limit(
                Path::new(path),
                GenericSourceSyntaxLimits::DEFAULT.frontend.max_source_bytes,
            )
            .map_err(|error| {
                match error {
                    compiler::resources::SourceReadError::Io(error) => input_io(
                        path,
                        format,
                        diagnostic_message!("svr: cannot read `{path}`: {error}"),
                        testing,
                    ),
                    compiler::resources::SourceReadError::Limit(errors) => {
                        input_errors(path, CheckKind::Source, format, errors, testing)
                    }
                }
                ExitCode::from(1)
            })?;
            Ok(Input {
                text,
                label: path.to_owned(),
                kind: CheckKind::Source,
            })
        }
    }
}
fn report(
    path: &str,
    input: &Input,
    format: CheckFormat,
    code: &'static str,
    span: Option<Span>,
    message: String,
) {
    print_check_diagnostics(
        path,
        input.kind,
        format,
        Diagnostics {
            items: vec![Diagnostic {
                severity: Severity::Error,
                code,
                message,
                source_file: span.map(|_| input.label.clone()),
                span: span.unwrap_or(Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                }),
            }
            .bounded()],
        },
    );
}
fn package_build(
    path: &str,
    output: &str,
    locked: bool,
    collections: bool,
    data: bool,
) -> ExitCode {
    let captured = match compiler::project::packages::capture_package_sources(
        path,
        compiler::project::packages::PackageCaptureLimits::DEFAULT,
    ) {
        Ok(captured) => captured,
        Err(errors) => {
            print_check_diagnostics(path, CheckKind::Project, CheckFormat::Human, errors);
            return ExitCode::from(1);
        }
    };
    if locked {
        if let Err(errors) =
            compiler::project::packages::verify_existing_captured_source_lock(&captured)
        {
            print_check_diagnostics(path, CheckKind::Project, CheckFormat::Human, errors);
            return ExitCode::from(1);
        }
    }
    let root_only = captured.inventory().graph.packages.len() == 1 && captured.sources().len() == 1;
    let result = if root_only && !collections && !data {
        let program = match compile_generic_array_source(
            captured.sources()[0].source(),
            "main",
            GenericPropagationSourceLimits::DEFAULT,
        ) {
            Ok(program) => program,
            Err(error) => {
                let input = Input {
                    text: String::new(),
                    label: captured.inventory().sources[0]
                        .canonical_path
                        .to_string_lossy()
                        .into_owned(),
                    kind: CheckKind::Project,
                };
                report(
                    path,
                    &input,
                    CheckFormat::Human,
                    "E3123",
                    error.span,
                    diagnostic_message!("{error}"),
                );
                return ExitCode::from(1);
            }
        };
        if let Err(error) = program.validate_command_entry() {
            let input = Input {
                text: String::new(),
                label: captured.inventory().sources[0]
                    .canonical_path
                    .to_string_lossy()
                    .into_owned(),
                kind: CheckKind::Project,
            };
            report(path, &input, CheckFormat::Human, "E3123", None, error);
            return ExitCode::from(1);
        }
        crate::native_build::build_generic(&program, Path::new(output))
    } else {
        let compile = if data {
            compile_generic_data_package
        } else if collections {
            compile_generic_collection_package
        } else {
            compile_generic_package
        };
        let program = match compile(captured, "main", GenericPackageCompilationLimits::DEFAULT) {
            Ok(program) => program,
            Err(errors) => {
                print_check_diagnostics(path, CheckKind::Project, CheckFormat::Human, errors);
                return ExitCode::from(1);
            }
        };
        if let Err(errors) = program.validate_command_entry() {
            print_check_diagnostics(path, CheckKind::Project, CheckFormat::Human, errors);
            return ExitCode::from(1);
        }
        let build = if data {
            crate::native_build::build_generic_data_package
        } else if collections {
            crate::native_build::build_generic_collection_package
        } else {
            crate::native_build::build_generic_package
        };
        build(&program, Path::new(output))
    };
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            cli_error!("svr: generic native build error: {error}");
            ExitCode::from(1)
        }
    }
}
fn package_command(
    command: &str,
    path: &str,
    format: CheckFormat,
    locked: bool,
    collections: bool,
    data: bool,
    host_policy: Option<&str>,
) -> ExitCode {
    let checked = (|| {
        let captured = compiler::project::packages::capture_package_sources(
            path,
            compiler::project::packages::PackageCaptureLimits::DEFAULT,
        )?;
        if locked {
            compiler::project::packages::verify_existing_captured_source_lock(&captured)?;
        }
        Ok(captured)
    })();
    let captured = match checked {
        Ok(captured) => captured,
        Err(errors) => {
            input_errors(path, CheckKind::Project, format, errors, command == "test");
            return ExitCode::from(1);
        }
    };
    if command == "test" {
        let test = if data {
            test_generic_data_package
        } else if collections {
            test_generic_collection_package
        } else {
            test_generic_package
        };
        let result = test(
            captured,
            path,
            GenericPackageCompilationLimits::DEFAULT,
            generic_command_runtime_policy(),
            GenericPackageTestLimits::DEFAULT,
        );
        return publish_tests(&result, format);
    }
    let compile = if data {
        compile_generic_data_package
    } else if collections {
        compile_generic_collection_package
    } else {
        compile_generic_package
    };
    let program = match compile(captured, "main", GenericPackageCompilationLimits::DEFAULT) {
        Ok(program) => program,
        Err(errors) => {
            print_check_diagnostics(path, CheckKind::Project, format, errors);
            return ExitCode::from(1);
        }
    };
    if command == "check" {
        if format == CheckFormat::Json {
            print_check_diagnostics(path, CheckKind::Project, format, Diagnostics::new());
        } else {
            println!(
                "{}",
                compiler::diagnostics::terminal_message(format_args!(
                    "checked experimental generic package `{path}`"
                ))
            );
        }
        return ExitCode::SUCCESS;
    }
    let execution = program.validate_command_entry().and_then(|_| {
        if let Some(path) = host_policy {
            let host = HostV3::load_policy(Path::new(path)).map_err(|e| Diagnostics {
                items: vec![Diagnostic {
                    severity: Severity::Error,
                    code: "E3125",
                    message: diagnostic_message!("host policy: {}", e.message),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                    source_file: None,
                }],
            })?;
            return execute_generic_data_package_with_host(
                &program,
                generic_command_runtime_policy(),
                &host,
            )
            .map(|_| ());
        }

        let execute = if data {
            execute_generic_data_package
        } else if collections {
            execute_generic_collection_package
        } else {
            execute_generic_package
        };
        execute(&program, generic_command_runtime_policy()).map(|_| ())
    });
    match execution {
        Ok(()) => ExitCode::SUCCESS,
        Err(errors) => {
            print_check_diagnostics(path, CheckKind::Project, format, errors);
            ExitCode::from(1)
        }
    }
}
pub(super) fn command(command: &str, args: &[String]) -> ExitCode {
    if !matches!(command, "check" | "run" | "build" | "test") {
        return usage(
            "--experimental-generics supports source/package check, run, test and native executable builds",
        );
    }
    let checking = command == "check";
    let Arguments {
        path,
        format,
        output,
        locked,
        symbols,
        collections,
        data,
        host_policy,
        context,
        overlays,
    } = match arguments(args, command) {
        Ok(v) => v,
        Err(e) => return usage(e),
    };
    if context {
        return package_context(path);
    }
    if overlays {
        return check_overlays(path);
    }
    if data && !Path::new(path).is_dir() {
        return usage("--experimental-data requires a root package directory");
    }
    if collections && !Path::new(path).is_dir() {
        return usage("--experimental-collections requires a root package directory");
    }
    if command == "build" && Path::new(path).is_dir() {
        return package_build(
            path,
            output.expect("validated native output argument"),
            locked,
            collections,
            data,
        );
    }
    if matches!(command, "check" | "run" | "test") && Path::new(path).is_dir() {
        return package_command(
            command,
            path,
            format,
            locked,
            collections,
            data,
            host_policy,
        );
    }
    let input = match input(path, format, locked, command == "test") {
        Ok(input) => input,
        Err(status) => return status,
    };
    if command == "test" {
        let mut result = test_generic_source(
            &input.text,
            &input.label,
            GenericPropagationSourceLimits::DEFAULT,
            generic_command_runtime_policy(),
            GenericTestLimits::DEFAULT,
        );
        result.target = path.into();
        result.package = input.kind == CheckKind::Project;
        return publish_tests(&result, format);
    }
    let program = match compile_generic_array_source(
        &input.text,
        "main",
        GenericPropagationSourceLimits::DEFAULT,
    ) {
        Ok(program) => program,
        Err(error) => {
            report(
                path,
                &input,
                format,
                "E3123",
                error.span,
                diagnostic_message!("{error}"),
            );
            return ExitCode::from(1);
        }
    };
    if checking {
        if format == CheckFormat::Json {
            if symbols {
                let mut report =
                    compiler::check_report::render(path, Some(input.kind), &Diagnostics::new());
                report.pop();
                report.push_str(",\"symbols\":");
                report.push_str(&generic_source_symbols(
                    &program,
                    GenericSymbolLimits::DEFAULT,
                ));
                report.push('}');
                println!("{report}");
            } else {
                print_check_diagnostics(path, input.kind, format, Diagnostics::new());
            }
        } else {
            println!(
                "{}",
                compiler::diagnostics::terminal_message(format_args!(
                    "checked experimental generic {} `{path}`",
                    if input.kind == CheckKind::Project {
                        "package"
                    } else {
                        "source"
                    }
                ))
            );
        }
        return ExitCode::SUCCESS;
    }
    if let Err(error) = program.validate_command_entry() {
        report(path, &input, format, "E3123", None, error);
        return ExitCode::from(1);
    }
    if let Some(output) = output {
        return match crate::native_build::build_generic(&program, Path::new(output)) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                cli_error!("svr: generic native build error: {error}");
                ExitCode::from(1)
            }
        };
    }
    let policy = generic_command_runtime_policy();
    match execute_generic_source(&program, policy) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            report(
                path,
                &input,
                format,
                "E3125",
                error.span,
                diagnostic_message!("{error}"),
            );
            ExitCode::from(1)
        }
    }
}
