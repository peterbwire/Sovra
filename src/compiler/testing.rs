//! Checked language test discovery and sequential execution under ADR 0028.

use super::{
    ast::Program,
    check_report::push_string,
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    interpreter::{run_function_with_host_status, RuntimeHost},
    ir,
    parser::Parser,
    project::packages,
    semantic::{self, SemanticAnalyzer, Type},
    stdlib, text_files,
};
use std::{fs, path::Path};

/// Outcome of one discovered test function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOutcome {
    /// Original root function name.
    pub name: String,
    /// Whether execution completed with status zero.
    pub passed: bool,
    /// Lines captured even when execution fails.
    pub output: Vec<String>,
    /// Final status, absent when a runtime error interrupts execution.
    pub exit_code: Option<u8>,
    /// Runtime failure or null for an ordinary return.
    pub error: Option<String>,
}

/// Complete result of checking, discovering and running one source/package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestReport {
    /// Input as supplied by the caller.
    pub target: String,
    /// Whether the input was classified as a package directory.
    pub package: bool,
    /// Compilation/configuration diagnostics; no tests run when errors exist.
    pub diagnostics: Diagnostics,
    /// Sequential outcomes in source order.
    pub tests: Vec<TestOutcome>,
}

impl TestReport {
    /// True only for a nonempty passing suite without error diagnostics.
    pub fn success(&self) -> bool {
        !self
            .diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
            && !self.tests.is_empty()
            && self.tests.iter().all(|test| test.passed)
    }
    /// Number of successful tests.
    pub fn passed(&self) -> usize {
        self.tests.iter().filter(|test| test.passed).count()
    }
}

/// Check the whole input, discover valid root tests and run them sequentially.
///
/// Tests run relative to the caller's working directory. File effects are not
/// rolled back or sandboxed. Arguments are empty; stdin is unavailable. Every
/// test has fresh frames, host output and application exit status.
pub fn run(path: impl AsRef<Path>) -> TestReport {
    let path = path.as_ref();
    let mut report = TestReport {
        target: path.to_string_lossy().into_owned(),
        package: path.is_dir(),
        diagnostics: Diagnostics::new(),
        tests: Vec::new(),
    };
    let compiled = if report.package {
        packages::compile_with_root(path)
    } else {
        compile_source(path)
    };
    let (program, root, entry) = match compiled {
        Ok(compiled) => compiled,
        Err(diagnostics) => {
            report.diagnostics = diagnostics;
            return report;
        }
    };
    let selected = discover(&root, &entry, &mut report.diagnostics);
    if !report.diagnostics.is_empty() {
        return report;
    }
    for name in selected {
        let runtime_name = if report.package {
            packages::root_symbol(&name)
        } else {
            name.clone()
        };
        let mut host = TestHost::default();
        let result = run_function_with_host_status(&program, &runtime_name, &mut host);
        let (passed, exit_code, error) = match result {
            Ok(outcome) => (outcome.exit_code == 0, Some(outcome.exit_code), None),
            Err(error) => (false, None, Some(error)),
        };
        report.tests.push(TestOutcome {
            name,
            passed,
            output: host.output,
            exit_code,
            error,
        });
    }
    report
}

fn compile_source(
    path: &Path,
) -> Result<(ir::IrProgram, Program, std::path::PathBuf), Diagnostics> {
    let text = fs::read_to_string(path)
        .map_err(|error| configuration("E0001", format!("cannot read source: {error}"), None))?;
    let parsed = Parser::new()
        .parse_source(&text)
        .map_err(|errors| locate(errors, path))?;
    let typed = SemanticAnalyzer::new()
        .analyze(&parsed)
        .map_err(|errors| locate(errors, path))?;
    Ok((ir::lower(&typed), typed.program, path.to_owned()))
}

fn discover(program: &Program, entry: &Path, diagnostics: &mut Diagnostics) -> Vec<String> {
    let known = semantic::collect_named_types(program, diagnostics);
    let mut selected = Vec::new();
    for function in &program.functions {
        if !function.name.starts_with("test_") {
            continue;
        }
        let result = function
            .return_type
            .as_deref()
            .map(|name| Type::from_name_with_known(name, Some(&known)))
            .unwrap_or(Type::Unit)
            .canonicalize(&known);
        if !function.parameters.is_empty() || result != Type::Unit {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: "E5001",
                message: format!(
                    "test function `{}` must have no parameters and return Unit",
                    function.name
                ),
                span: function.span,
                source_file: Some(entry.to_string_lossy().into_owned()),
            });
        }
        selected.push(function.name.clone());
    }
    if selected.is_empty() {
        diagnostics.items.extend(
            configuration(
                "E5002",
                "no root-level test_ functions were found".into(),
                None,
            )
            .items,
        );
    }
    selected
}

fn configuration(code: &'static str, message: String, source_file: Option<String>) -> Diagnostics {
    Diagnostics {
        items: vec![Diagnostic {
            code,
            message,
            source_file,
            severity: Severity::Error,
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        }],
    }
}
fn locate(mut errors: Diagnostics, path: &Path) -> Diagnostics {
    for error in &mut errors.items {
        error.source_file = Some(path.to_string_lossy().into_owned());
    }
    errors
}

#[derive(Debug, Default)]
struct TestHost {
    output: Vec<String>,
}
impl RuntimeHost for TestHost {
    fn arguments(&self) -> &[String] {
        &[]
    }
    fn read_line(&mut self) -> Result<Option<String>, String> {
        Err("standard input is unavailable to test functions".into())
    }
    fn write_line(&mut self, line: &str) -> Result<(), String> {
        self.output.push(line.to_owned());
        Ok(())
    }
    fn read_text(&mut self, path: &str) -> text_files::TextRead {
        text_files::read_text(path)
    }
    fn write_text(&mut self, path: &str, text: &str) -> text_files::TextWrite {
        text_files::write_text(path, text)
    }
}

/// Render safe human output with per-test status and totals.
pub fn render_human(report: &TestReport) -> String {
    let mut output = String::new();
    for diagnostic in &report.diagnostics.items {
        let location = if diagnostic.span.end > 0 {
            diagnostic
                .source_file
                .as_deref()
                .map(|file| {
                    format!(
                        " at {}:{}:{}",
                        stdlib::escape_terminal(file),
                        diagnostic.span.line + 1,
                        diagnostic.span.column + 1
                    )
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        output.push_str(&format!(
            "{}[{}]{}: {}\n",
            if diagnostic.severity == Severity::Error {
                "error"
            } else {
                "warning"
            },
            diagnostic.code,
            location,
            stdlib::escape_terminal(&diagnostic.message)
        ));
    }
    for test in &report.tests {
        output.push_str(&format!(
            "{} {}\n",
            if test.passed { "PASS" } else { "FAIL" },
            stdlib::escape_terminal(&test.name)
        ));
        for line in &test.output {
            output.push_str(&format!("  {}\n", stdlib::escape_terminal(line)));
        }
        if let Some(error) = &test.error {
            output.push_str(&format!("  {}\n", stdlib::escape_terminal(error)));
        } else if !test.passed {
            output.push_str(&format!("  exit status {}\n", test.exit_code.unwrap_or(0)));
        }
    }
    output.push_str(&format!(
        "{} passed; {} failed; {} total\n",
        report.passed(),
        report.tests.len() - report.passed(),
        report.tests.len()
    ));
    output
}

/// Render one schema-one JSON test report without a trailing newline.
pub fn render_json(report: &TestReport) -> String {
    let mut output = String::from("{\"schema_version\":1,\"target\":");
    push_string(&mut output, &report.target);
    output.push_str(if report.package {
        ",\"kind\":\"package\""
    } else {
        ",\"kind\":\"source\""
    });
    output.push_str(&format!(
        ",\"success\":{},\"passed\":{},\"failed\":{},\"total\":{},\"diagnostics\":[",
        report.success(),
        report.passed(),
        report.tests.len() - report.passed(),
        report.tests.len()
    ));
    for (index, diagnostic) in report.diagnostics.items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(if diagnostic.severity == Severity::Error {
            "{\"severity\":\"error\",\"code\":"
        } else {
            "{\"severity\":\"warning\",\"code\":"
        });
        push_string(&mut output, diagnostic.code);
        output.push_str(",\"message\":");
        push_string(&mut output, &diagnostic.message);
        output.push_str(",\"source_file\":");
        if let Some(file) = &diagnostic.source_file {
            push_string(&mut output, file);
        } else {
            output.push_str("null");
        }
        output.push_str(",\"span\":");
        if diagnostic.span.start == 0 && diagnostic.span.end == 0 {
            output.push_str("null");
        } else {
            let span = diagnostic.span;
            output.push_str(&format!(
                "{{\"start\":{},\"end\":{},\"line\":{},\"column\":{}}}",
                span.start, span.end, span.line, span.column
            ));
        }
        output.push('}');
    }
    output.push_str("],\"tests\":[");
    for (index, test) in report.tests.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"name\":");
        push_string(&mut output, &test.name);
        output.push_str(&format!(",\"passed\":{},\"exit_code\":", test.passed));
        if let Some(code) = test.exit_code {
            output.push_str(&code.to_string());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"error\":");
        if let Some(error) = &test.error {
            push_string(&mut output, error);
        } else {
            output.push_str("null");
        }
        output.push_str(",\"output\":[");
        for (index, line) in test.output.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            push_string(&mut output, line);
        }
        output.push_str("]}");
    }
    output.push_str("]}");
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    #[derive(Debug)]
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "svr-language-tests-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn source(&self, text: &str) -> std::path::PathBuf {
            let path = self.0.join("tests.svr");
            fs::write(&path, text).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn discovers_source_order_unit_aliases_and_explicit_main_calls() {
        let fixture = Fixture::new();
        let path = fixture.source("type Done = Unit; mod types { type Finished = Unit; } fn main() { print(\"main\"); } fn test_z() -> Done { main(); } fn test_a() -> types::Finished { print(\"alias\"); } mod ignored { export fn test_not_discovered() { std::set_exit_code(1); } }");
        let report = run(path);
        assert!(report.success(), "{:?}", report.diagnostics);
        assert_eq!(
            report
                .tests
                .iter()
                .map(|test| test.name.as_str())
                .collect::<Vec<_>>(),
            ["test_z", "test_a"]
        );
        assert_eq!(report.tests[0].output, ["main"]);
        assert_eq!(report.tests[1].output, ["alias"]);
    }

    #[test]
    fn continues_after_errors_and_nonzero_status_with_fresh_host_state() {
        let fixture = Fixture::new();
        let report = run(fixture.source("fn test_exit() { std::set_exit_code(4); } fn test_error() { print(\"before\"); let zero = 0; print(1 / zero); } fn test_after() { print(std::arg_count()); } fn test_stdin() { std::read_line(); }"));
        assert_eq!(report.tests.len(), 4);
        assert_eq!(report.passed(), 1);
        assert_eq!(report.tests[0].exit_code, Some(4));
        assert_eq!(report.tests[1].output, ["before"]);
        assert!(report.tests[1]
            .error
            .as_ref()
            .unwrap()
            .contains("division by zero"));
        assert!(report.tests[2].passed);
        assert_eq!(report.tests[2].exit_code, Some(0));
        assert_eq!(report.tests[2].output, ["0"]);
        assert!(report.tests[3]
            .error
            .as_ref()
            .unwrap()
            .contains("unavailable"));
    }

    #[test]
    fn invalid_signatures_and_unused_body_errors_prevent_all_effects() {
        let fixture = Fixture::new();
        let marker = fixture.0.join("marker.txt");
        let marker_literal = format!("{:?}", marker.to_string_lossy().replace('\\', "/"));
        for invalid in [
            "fn test_bad(value: Int) {}",
            "fn test_bad() -> Int { return 1; }",
            "fn unused() { missing(); }",
        ] {
            let source = format!(
                "fn test_effect() {{ std::write_text({marker_literal}, \"changed\"); }} {invalid}"
            );
            let report = run(fixture.source(&source));
            assert!(!report.diagnostics.is_empty());
            assert!(report.tests.is_empty());
            assert!(!marker.exists());
        }
    }

    #[test]
    fn test_host_keeps_existing_file_contracts_without_rollback() {
        let fixture = Fixture::new();
        let marker = fixture.0.join("written.txt");
        let literal = format!("{:?}", marker.to_string_lossy().replace('\\', "/"));
        let source = format!("fn test_write() {{ let saved = std::write_text({literal}, \"hello\"); print(saved.ok); }} fn test_read() {{ let loaded = std::read_text({literal}); print(loaded.text); }}");
        let report = run(fixture.source(&source));
        assert!(report.success());
        assert_eq!(report.tests[0].output, ["true"]);
        assert_eq!(report.tests[1].output, ["hello"]);
        assert_eq!(fs::read_to_string(marker).unwrap(), "hello");
    }

    #[test]
    fn rejects_empty_discovery_and_preserves_parser_locations() {
        let fixture = Fixture::new();
        let no_tests = run(fixture.source("fn main() {}"));
        assert_eq!(no_tests.diagnostics.items[0].code, "E5002");
        let invalid = run(fixture.source("fn test_bad() { let broken = }"));
        assert!(invalid.diagnostics.items[0].code.starts_with("E200"));
        assert!(invalid.diagnostics.items[0].source_file.is_some());
        assert!(invalid.diagnostics.items[0].span.end > 0);
    }

    #[test]
    fn report_preserves_warning_severity_without_failing_passing_tests() {
        let mut warning = configuration("W0001", "warning".into(), None);
        warning.items[0].severity = Severity::Warning;
        let report = TestReport {
            target: "tests.svr".into(),
            package: false,
            diagnostics: warning,
            tests: vec![TestOutcome {
                name: "test_ok".into(),
                passed: true,
                output: vec![],
                exit_code: Some(0),
                error: None,
            }],
        };
        assert!(report.success());
        assert!(render_json(&report).contains("\"severity\":\"warning\""));
        assert!(render_human(&report).contains("warning[W0001]"));
    }

    #[test]
    fn package_discovery_uses_linked_root_symbols_and_excludes_dependencies() {
        let fixture = Fixture::new();
        let app = fixture.0.join("app");
        let library = fixture.0.join("library");
        fs::create_dir(&app).unwrap();
        fs::create_dir(&library).unwrap();
        fs::write(app.join("sovra.toml"), "[project]\nname = \"test-app\"\nentry = \"main.svr\"\n[dependencies.util]\npath = \"../library\"\n").unwrap();
        fs::write(
            library.join("sovra.toml"),
            "[project]\nname = \"test-library\"\nentry = \"library.svr\"\n",
        )
        .unwrap();
        fs::write(library.join("library.svr"), "fn test_dependency() { std::set_exit_code(1); } mod math { export fn square(value: Int) -> Int { return value * value; } }").unwrap();
        fs::write(
            app.join("main.svr"),
            "use util::math; fn test_package() { print(util::math::square(7)); }",
        )
        .unwrap();
        let report = run(&app);
        assert!(report.success(), "{:?}", report.diagnostics);
        assert_eq!(report.tests.len(), 1);
        assert_eq!(report.tests[0].output, ["49"]);
    }
}
