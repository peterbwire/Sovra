//! JSON reports for source and project checks.

use std::fmt::Write as _;

use crate::compiler::diagnostics::{Diagnostics, Severity, Span};

/// The kind of target processed by a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    /// A source file checked by the executable compiler pipeline.
    Source,
    /// A project checked by the manifest and application-wiring scanner.
    Project,
}

/// Render a version-one JSON check report without a trailing newline.
///
/// A missing kind means the target could not be classified. Source locations
/// contain the target as supplied and the compiler's zero-based byte offsets,
/// line, and character column. All-zero spans without a file have no known
/// location. Project locations require explicit file identity from the scanner.
/// A report succeeds when its diagnostics contain no errors; warnings alone do
/// not make a check fail.
pub fn render(target: &str, kind: Option<CheckKind>, diagnostics: &Diagnostics) -> String {
    let mut output = String::from("{\"schema_version\":1,\"target\":");
    push_string(&mut output, target);
    output.push_str(",\"kind\":");
    output.push_str(match kind {
        Some(CheckKind::Source) => "\"source\"",
        Some(CheckKind::Project) => "\"project\"",
        None => "null",
    });
    output.push_str(",\"success\":");
    let success = !diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    output.push_str(if success { "true" } else { "false" });
    output.push_str(",\"diagnostics\":[");
    for (index, diagnostic) in diagnostics.items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"severity\":");
        push_string(
            &mut output,
            match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
        );
        output.push_str(",\"code\":");
        push_string(&mut output, diagnostic.code);
        output.push_str(",\"message\":");
        push_string(&mut output, &diagnostic.message);
        output.push_str(",\"location\":");
        let file = diagnostic.source_file.as_deref().or_else(|| {
            if kind == Some(CheckKind::Source) && has_location(diagnostic.span) {
                Some(target)
            } else {
                None
            }
        });
        if let Some(file) = file {
            output.push_str("{\"file\":");
            push_string(&mut output, file);
            let span = diagnostic.span;
            let _ = write!(
                output,
                ",\"start\":{},\"end\":{},\"line\":{},\"column\":{}}}",
                span.start, span.end, span.line, span.column
            );
        } else {
            output.push_str("null");
        }
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn has_location(span: Span) -> bool {
    span.start != 0 || span.end != 0 || span.line != 0 || span.column != 0
}

fn push_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\u{0000}'..='\u{001f}' => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            _ => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    use super::*;
    use crate::compiler::diagnostics::Diagnostic;

    fn assert_report(report: &str, assertions: &str) {
        let script = format!(
            "const fs = require('node:fs');\n\
             const assert = require('node:assert/strict');\n\
             const report = JSON.parse(fs.readFileSync(0, 'utf8'));\n\
             {assertions}"
        );
        let mut child = Command::new("node")
            .args(["-e", &script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node.js is required to validate check-report JSON");
        child
            .stdin
            .take()
            .expect("child stdin should be piped")
            .write_all(report.as_bytes())
            .expect("check report should be written to Node.js");
        let output = child.wait_with_output().expect("Node.js should finish");
        assert!(
            output.status.success(),
            "JSON assertion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn diagnostic(severity: Severity, span: Span) -> Diagnostic {
        Diagnostic {
            source_file: None,
            severity,
            code: "E3014",
            message: "parameter requires an explicit type annotation".into(),
            span,
        }
    }

    fn zero_span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn project_validation_locations_survive_json_serialization() {
        let root = format!(
            "{}/tests/fixtures/project-locations",
            env!("CARGO_MANIFEST_DIR")
        );
        let errors = crate::compiler::project::check_project(&root).expect_err("invalid project");
        let report = render(&root, Some(CheckKind::Project), &errors);
        assert_report(
            &report,
            r#"
            for (const [code, file, spelling, line] of [
                ['E4003', 'sovra.toml', 'target = "unknown"', 5],
                ['E4033', 'main.svr', '  route GET "/missing" -> absent.handler', 2],
                ['E4081', 'auth.svr', '  allow user to read on Missing', 1]
            ]) {
                const d = report.diagnostics.find(d => d.code === code);
                const path = require('node:path');
                assert.equal(path.resolve(d.location.file), path.resolve(report.target, file));
                const bytes = fs.readFileSync(d.location.file);
                const start = bytes.indexOf(spelling);
                assert.deepEqual(d.location, { file: d.location.file, start,
                    end: start + Buffer.byteLength(spelling), line, column: 0 });
            }
        "#,
        );
    }

    #[test]
    fn empty_diagnostics_produce_a_successful_report() {
        let report = render("main.svr", Some(CheckKind::Source), &Diagnostics::new());
        assert_report(
            &report,
            r#"assert.deepEqual(report, {
                schema_version: 1,
                target: 'main.svr',
                kind: 'source',
                success: true,
                diagnostics: []
            });"#,
        );
    }

    #[test]
    fn strings_round_trip_through_json_parse() {
        let controls: String = (0u8..=31).map(char::from).collect();
        let text = format!(
            "quotes: \" | backslash: \\ | Unicode: café 😀 \u{2028}\u{2029} | controls: {controls}"
        );
        let mut error = diagnostic(Severity::Error, zero_span());
        error.message = text.clone();
        let diagnostics = Diagnostics { items: vec![error] };
        let report = render(&text, Some(CheckKind::Source), &diagnostics);
        assert_report(
            &report,
            r#"const expected = 'quotes: " | backslash: \\ | Unicode: café 😀 \u2028\u2029 | controls: '
                + String.fromCharCode(...Array.from({ length: 32 }, (_, index) => index));
            assert.equal(report.target, expected);
            assert.equal(report.diagnostics[0].message, expected);
            assert.equal(report.diagnostics[0].code, 'E3014');
            assert.equal(report.diagnostics[0].severity, 'error');
            assert.equal(report.success, false);"#,
        );
    }

    #[test]
    fn source_locations_retain_offsets_and_warnings_do_not_fail_checks() {
        let diagnostics = Diagnostics {
            items: vec![diagnostic(
                Severity::Warning,
                Span {
                    start: 12,
                    end: 17,
                    line: 1,
                    column: 3,
                },
            )],
        };
        let report = render("folder\\main.svr", Some(CheckKind::Source), &diagnostics);
        assert_report(
            &report,
            r#"assert.equal(report.success, true);
            assert.equal(report.diagnostics[0].severity, 'warning');
            assert.deepEqual(report.diagnostics[0].location, {
                file: 'folder\\main.svr', start: 12, end: 17, line: 1, column: 3
            });"#,
        );
    }

    #[test]
    fn unavailable_locations_remain_null() {
        for (kind, span, expected_kind) in [
            (Some(CheckKind::Source), zero_span(), "'source'"),
            (
                Some(CheckKind::Project),
                Span {
                    start: 12,
                    end: 17,
                    line: 1,
                    column: 3,
                },
                "'project'",
            ),
            (
                None,
                Span {
                    start: 12,
                    end: 17,
                    line: 1,
                    column: 3,
                },
                "null",
            ),
        ] {
            let diagnostics = Diagnostics {
                items: vec![diagnostic(Severity::Error, span)],
            };
            let report = render("target", kind, &diagnostics);
            assert_report(
                &report,
                &format!(
                    "assert.equal(report.kind, {expected_kind});\n\
                     assert.equal(report.success, false);\n\
                     assert.equal(report.diagnostics[0].location, null);"
                ),
            );
        }
    }
}
