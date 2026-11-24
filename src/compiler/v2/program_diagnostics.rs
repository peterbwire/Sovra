//! Bounded root-program error projection into the existing diagnostic/report schema.
use super::*;
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span},
    sum_syntax::EnumSyntaxError,
};
/// Maximum published root-program diagnostics; excess is replaced by E3019.
pub const PROGRAM_DIAGNOSTIC_LIMIT: usize = 64;
fn one(code: &'static str, message: String, span: Span) -> Diagnostics {
    Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code,
            message,
            span,
        }
        .bounded()],
    }
}
fn unknown() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    }
}
fn resource(kind: &'static str) -> Diagnostics {
    one(
        "E3025",
        diagnostic_message!("root-program compiler allowance exhausted: {kind}"),
        unknown(),
    )
}
fn syntax(error: EnumSyntaxError) -> Diagnostics {
    match error {
        EnumSyntaxError::Diagnostics(d) => d,
        EnumSyntaxError::Limit { .. } => resource("enum syntax"),
    }
}
fn types(error: EnumTypeError) -> Diagnostics {
    match error {
        EnumTypeError::Diagnostics(d) => Diagnostics {
            items: d
                .into_iter()
                .take(PROGRAM_DIAGNOSTIC_LIMIT + 1)
                .map(|d| d.diagnostic)
                .collect(),
        },
        EnumTypeError::Syntax { error, .. } => syntax(error),
        EnumTypeError::Limit { .. } | EnumTypeError::IdentityExhausted => {
            resource("nominal type metadata/identity")
        }
        EnumTypeError::DuplicateSourceLabel(_)
        | EnumTypeError::UnavailableModule(_)
        | EnumTypeError::UnavailableIdentity
        | EnumTypeError::UnavailableDiagnosticOrigin => one(
            "E3122",
            "root-program checked type provenance is unavailable".into(),
            unknown(),
        ),
    }
}
fn function(error: EnumFunctionError) -> Diagnostics {
    match error {
        EnumFunctionError::Syntax(d) | EnumFunctionError::Diagnostics(d) => d,
        EnumFunctionError::Coverage(e) => types(e),
        EnumFunctionError::Limit(kind) => resource(kind),
    }
}
fn source(error: EntrySourceError) -> Diagnostics {
    match error {
        EntrySourceError::Syntax(d) => d,
        EntrySourceError::EnumSyntax(e) => syntax(e),
        EntrySourceError::Types(e) => types(e),
        EntrySourceError::Function(e) => function(e),
        EntrySourceError::Limit(kind) => resource(kind),
        EntrySourceError::MissingEntry => one(
            "E3121",
            "root program requires at least one function".into(),
            unknown(),
        ),
        EntrySourceError::Unsupported {
            kind: "additional function",
            ..
        } => resource("root function count"),
        EntrySourceError::Unsupported { kind, span } => one(
            "E3120",
            diagnostic_message!("unsupported root-program syntax: {kind}"),
            span,
        ),
    }
}
impl ProgramSourceError {
    /// Project root-file coordinates/codes with bounded messages and at most 64 items.
    /// Internal fragment labels are never used as file paths. Caller supplies the root
    /// target identity to the existing reporter. Resource/invariant failures stay unlocated.
    /// A typed failure always retains an error, even for caller-made empty/warning errors.
    pub fn into_diagnostics(self) -> Diagnostics {
        let raw = match self {
            Self::Source(e) => source(e),
            Self::DuplicateFunction(span) => {
                one("E3008", "duplicate function declaration".into(), span)
            }
            Self::Function { error, .. } => function(error),
            Self::Limit(kind) => resource(kind),
            Self::InvalidLink => one(
                "E3122",
                "root-program checked call target is unavailable".into(),
                unknown(),
            ),
        };
        let mut output = Diagnostics::new();
        for mut diagnostic in raw.items {
            diagnostic.source_file = None;
            if output.items.len() == PROGRAM_DIAGNOSTIC_LIMIT {
                output.items.pop();
                output.push_bounded(Diagnostic {
                    code: "E3019",
                    message: "root-program diagnostic limit reached".into(),
                    severity: Severity::Error,
                    span: diagnostic.span,
                    source_file: None,
                });
                break;
            }
            output.push_bounded(diagnostic);
        }
        if !output.items.iter().any(|d| d.severity == Severity::Error) {
            if output.items.len() == PROGRAM_DIAGNOSTIC_LIMIT {
                output.items.pop();
            }
            output.items.extend(
                one(
                    "E3122",
                    "root-program check failed without an error diagnostic".into(),
                    unknown(),
                )
                .items,
            );
        }
        output
    }
}
