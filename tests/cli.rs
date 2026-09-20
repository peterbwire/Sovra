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

fn assert_json_report(output: &Output, assertions: &str) {
    use std::io::Write as _;
    use std::process::Stdio;
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let script = format!(
        "const assert = require('node:assert/strict'); const report = JSON.parse(require('node:fs').readFileSync(0, 'utf8')); assert.equal(report.schema_version, 1); {assertions}"
    );
    let mut child = Command::new("node")
        .args(["-e", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node.js required for JSON contract tests");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&output.stdout)
        .expect("write JSON");
    let result = child.wait_with_output().expect("Node.js result");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn check_json_reports_source_success() {
    let source = format!("{}/examples/functions/main.svr", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(0));
    assert_json_report(&output, "assert.equal(report.kind, 'source'); assert.equal(report.success, true); assert.deepEqual(report.diagnostics, []); assert.ok(report.target.endsWith('main.svr'));");
}

#[test]
fn check_project_diagnostics_identify_scanned_files() {
    for (fixture, expected) in [
        ("project-locations", "[['E4034', 'main.svr', '  route broken', 1], ['E4080', 'auth.svr', '  allow broken', 0], ['E4033', 'main.svr', '  route GET \"/missing\" -> absent.handler', 2], ['E4081', 'auth.svr', '  allow user to read on Missing', 1], ['E4003', 'sovra.toml', 'target = \"unknown\"', 5]]"),
        ("manifest-locations", "[['E4010', 'sovra.toml', '[unknown]', 0]]"),
    ] {
        let project = format!("{}/tests/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
        let Some(output) = output_or_skip(svr().args(["check", "--format=json", &project])) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert_json_report(&output, &format!(r#"
            assert.equal(report.kind, 'project');
            assert.equal(report.success, false);
            if (report.target.endsWith('project-locations')) {{
                for (const code of ['E4024', 'E4062']) {{
                    const d = report.diagnostics.find(d => d.code === code);
                    assert.ok(d.location.file.endsWith('main.svr'));
                    assert.equal(d.location.line, code === 'E4024' ? 3 : 4);
                }}
            }}
            for (const [code, file, spelling, line] of {expected}) {{
                const d = report.diagnostics.find(d => d.code === code);
                const path = require('node:path').join(report.target, file);
                const bytes = require('node:fs').readFileSync(path);
                const start = bytes.indexOf(spelling);
                assert.equal(require('node:path').resolve(d.location.file),
                    require('node:path').resolve(path));
                assert.deepEqual(d.location, {{ file: d.location.file, start,
                    end: start + Buffer.byteLength(spelling), line, column: 0 }});
            }}
        "#));
        let Some(output) = output_or_skip(svr().args(["check", &project])) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(if fixture == "project-locations" { "main.svr:2:1" } else { "sovra.toml:1:1" }));
    }
}

#[test]
fn check_json_reports_expression_locations() {
    let source = format!(
        "{}/tests/fixtures/expression-locations.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format=json", &source])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(
        &output,
        r#"
        assert.equal(report.success, false);
        const bytes = require('node:fs').readFileSync(report.target);
        for (const [code, spelling, line, column] of [
            ['E3007', '42', 1, 25], ['E3001', 'missing', 2, 10]
        ]) {
            const diagnostic = report.diagnostics.find(d => d.code === code);
            const start = bytes.indexOf(spelling);
            assert.deepEqual(diagnostic.location, {
                file: report.target, start, end: start + spelling.length, line, column
            });
        }
    "#,
    );
}

#[test]
fn check_json_reports_parameter_diagnostic_location() {
    let source = format!(
        "{}/tests/fixtures/untyped-parameter.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", &source, "--format", "json"])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(&output, "assert.equal(report.kind, 'source'); assert.equal(report.success, false); const d = report.diagnostics.find(d => d.code === 'E3014'); assert.equal(d.severity, 'error'); assert.ok(d.message.includes('value: Type')); assert.equal(d.location.file, report.target); assert.equal(d.location.start, 10); assert.equal(d.location.end, 15); assert.equal(d.location.line, 0); assert.equal(d.location.column, 10);");
}

#[test]
fn check_json_distinguishes_project_validation() {
    let project = format!("{}/examples/fielddesk", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", &project])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(0));
    assert_json_report(&output, "assert.equal(report.kind, 'project'); assert.equal(report.success, true); assert.deepEqual(report.diagnostics, []);");

    let no_manifest = format!("{}/examples/hello-world", env!("CARGO_MANIFEST_DIR"));
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", &no_manifest]))
    else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(&output, "assert.equal(report.kind, 'project'); assert.equal(report.success, false); assert.equal(report.diagnostics[0].code, 'E4000'); assert.equal(report.diagnostics[0].location, null);");
}

#[test]
fn check_json_reports_io_errors() {
    let missing = format!(
        "{}/tests/fixtures/no-such-source.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    let Some(output) = output_or_skip(svr().args(["check", "--format", "json", &missing])) else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_json_report(&output, "assert.equal(report.kind, null); assert.equal(report.success, false); assert.equal(report.diagnostics[0].code, 'E0001'); assert.equal(report.diagnostics[0].location, null);");
}

#[test]
fn check_json_respects_option_terminator() {
    for path in ["--help", "-h"] {
        let Some(output) = output_or_skip(
            svr()
                .current_dir(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")))
                .args(["check", "--format=json", "--", path]),
        ) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert_json_report(&output, "assert.equal(report.success, false); assert.equal(report.kind, null); assert.equal(report.diagnostics[0].code, 'E0001');");
    }
}

#[test]
fn check_rejects_invalid_format_arguments() {
    for arguments in [
        vec!["check", "--format", "xml", "README.md"],
        vec!["check", "--format"],
        vec!["check", "--format", "json"],
        vec!["check", "--format=json", "--format=human", "README.md"],
        vec!["check", "--format=json", "README.md"],
    ] {
        let Some(output) = output_or_skip(svr().args(&arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}");
    }
}

#[test]
fn source_commands_reject_mistyped_concatenation() {
    let source = format!(
        "{}/tests/fixtures/invalid-concatenation.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for args in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&args)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E3002"));
    }
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
        "Usage: svr check [--format human|json] <source.svr|project-directory>"
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
fn source_commands_reject_invalid_module_bodies() {
    let source = format!(
        "{}/tests/fixtures/invalid-module.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for arguments in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E3001"), "{arguments:?}: {stderr}");
        assert!(stderr.contains("undefined variable `missing`"));
    }
}

#[test]
fn source_commands_require_parameter_annotations() {
    let source = format!(
        "{}/tests/fixtures/untyped-parameter.svr",
        env!("CARGO_MANIFEST_DIR")
    );
    for arguments in [
        vec!["check", source.as_str()],
        vec!["run", source.as_str()],
        vec!["build", source.as_str()],
        vec!["build", "--emit", "js", source.as_str()],
    ] {
        let Some(output) = output_or_skip(svr().args(&arguments)) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E3014"), "{arguments:?}: {stderr}");
        assert!(stderr.contains("write `value: Type`"));
    }
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
