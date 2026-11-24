//! Explicit evidence errors convert to stable diagnostics without forged provenance.
use sovra::compiler::{
    check_report::{self, CheckKind},
    checked::{CheckError, CheckedUnit, EvidenceError, EvidenceLimits, SemanticEvidenceLimits},
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    project::packages::CheckedPackageError,
};

#[test]
fn metadata_exhaustion_has_stable_code_and_null_location() {
    let errors = [
        EvidenceError::Limit,
        EvidenceError::Check(CheckError::NodeLimit),
        EvidenceError::Check(CheckError::SourceLimit),
    ];
    for error in errors {
        for package in [false, true] {
            let diagnostics = if package {
                CheckedPackageError::Evidence {
                    entry: "provider/\u{1b}[31m.svr".into(),
                    error: error.clone(),
                }
                .into_diagnostics()
            } else {
                error.clone().into_diagnostics()
            };
            let item = &diagnostics.items[0];
            assert_eq!(item.severity, Severity::Error);
            assert_eq!(item.code, "E3025");
            assert_eq!(item.source_file, None);
            assert_eq!(
                item.span,
                Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0
                }
            );
            let json = check_report::render(
                "consumer",
                Some(if package {
                    CheckKind::Project
                } else {
                    CheckKind::Source
                }),
                &diagnostics,
            );
            assert!(json.contains("\"success\":false"));
            assert!(json.contains("\"location\":null"));
            assert!(!json.contains("provider/"));
        }
    }
}

#[test]
fn original_diagnostics_are_preserved_and_semantic_errors_still_win() {
    let original = Diagnostics {
        items: vec![Diagnostic {
            severity: Severity::Error,
            code: "E3004",
            message: "source error".into(),
            span: Span {
                start: 7,
                end: 9,
                line: 2,
                column: 3,
            },
            source_file: Some("dependency.svr".into()),
        }],
    };
    assert_eq!(
        EvidenceError::Check(CheckError::Diagnostics(original.clone())).into_diagnostics(),
        original
    );
    assert_eq!(
        CheckedPackageError::Diagnostics(original.clone()).into_diagnostics(),
        original
    );
    let limits = EvidenceLimits {
        max_nodes: 100,
        max_source_bytes: 1024,
    };
    let zero = SemanticEvidenceLimits {
        max_facts: 0,
        max_types: 0,
        max_name_bytes: 0,
    };
    let exhausted =
        CheckedUnit::from_source_with_evidence("fn main() { let x = 1; }", limits, zero)
            .unwrap_err()
            .into_diagnostics();
    assert_eq!(exhausted.items[0].code, "E3025");
    let invalid = CheckedUnit::from_source_with_evidence("fn main() { missing(); }", limits, zero)
        .unwrap_err()
        .into_diagnostics();
    assert!(invalid.items.iter().any(|d| d.code == "E3004"));
    assert!(invalid.items.iter().all(|d| d.code != "E3025"));
}
