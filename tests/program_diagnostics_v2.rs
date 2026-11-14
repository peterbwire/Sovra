//! Root diagnostics retain source facts while quota/identity failures stay unlocated.
use sovra::compiler::{
    check_report::{self, CheckKind},
    diagnostics::{Diagnostic, Diagnostics, Severity, Span, MAX_DIAGNOSTIC_MESSAGE_BYTES},
    v2::*,
};
fn zero() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    }
}
#[test]
fn quota_failures_map_to_e3025_without_invented_locations() {
    let d = ProgramSourceError::Limit("facts").into_diagnostics();
    assert_eq!(d.items[0].code, "E3025");
    let json = check_report::render("main.svr", Some(CheckKind::Source), &d);
    assert!(json.contains("\"location\":null"));
    let mut limits = ProgramSourceLimits::DEFAULT;
    limits.max_signature_copies = 0;
    assert_eq!(
        check_program_source("s", "fn main(){}", limits)
            .unwrap_err()
            .into_diagnostics()
            .items[0]
            .code,
        "E3025"
    );
}
#[test]
fn projector_bounds_messages_and_counts_and_never_uses_fragment_labels_as_paths() {
    let raw = Diagnostics {
        items: (0..100)
            .map(|_| Diagnostic {
                code: "E3003",
                severity: Severity::Error,
                message: "x".repeat(5000),
                span: Span {
                    start: 1,
                    end: 2,
                    line: 0,
                    column: 1,
                },
                source_file: Some("enum:internal".into()),
            })
            .collect(),
    };
    let d = ProgramSourceError::Source(EntrySourceError::Syntax(raw)).into_diagnostics();
    assert_eq!(d.items.len(), PROGRAM_DIAGNOSTIC_LIMIT);
    assert_eq!(d.items.last().unwrap().code, "E3019");
    assert!(d
        .items
        .iter()
        .all(|d| d.message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES && d.source_file.is_none()));
}
#[test]
fn empty_and_warning_only_typed_errors_still_publish_failed_reports() {
    for items in [
        vec![],
        vec![Diagnostic {
            code: "W9999",
            severity: Severity::Warning,
            message: "warning".into(),
            span: zero(),
            source_file: None,
        }],
    ] {
        let d = ProgramSourceError::Source(EntrySourceError::Syntax(Diagnostics { items }))
            .into_diagnostics();
        assert!(
            check_report::render("s", Some(CheckKind::Source), &d).contains("\"success\":false")
        );
        assert!(d.items.iter().any(|d| d.code == "E3122"));
    }
}
