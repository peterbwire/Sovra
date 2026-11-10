//! Structured diagnostics shared by compiler stages.

/// Maximum retained message bytes in guarded compiler diagnostic paths.
/// This excludes source-file identity, JSON escaping and caller-owned raw records.
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4096;
const MESSAGE_TRUNCATED: &str = " [message truncated]";

/// A source location represented by zero-based byte, line, and column offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The byte offset where the span starts.
    pub start: usize,
    /// The byte offset immediately after the span.
    pub end: usize,
    /// The zero-based source line.
    pub line: usize,
    /// The zero-based source column.
    pub column: usize,
}

/// The severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// An error prevents the current compiler stage from succeeding.
    Error,
    /// A warning does not prevent compilation.
    Warning,
}

/// A human-readable compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Diagnostic severity.
    pub severity: Severity,
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Short explanation of the problem.
    pub message: String,
    /// Source location associated with the problem.
    pub span: Span,
    /// File containing the span, when explicitly retained by a multi-file stage.
    /// None leaves source-file identity to the caller; project reports must not
    /// infer a file for diagnostics without provenance.
    pub source_file: Option<String>,
}

impl Diagnostic {
    /// Bound owned message storage without altering severity, code or locations.
    pub(crate) fn bounded(mut self) -> Self {
        if self.message.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES {
            let mut end = MAX_DIAGNOSTIC_MESSAGE_BYTES - MESSAGE_TRUNCATED.len();
            while !self.message.is_char_boundary(end) {
                end -= 1;
            }
            let mut bounded = String::with_capacity(MAX_DIAGNOSTIC_MESSAGE_BYTES);
            bounded.push_str(&self.message[..end]);
            bounded.push_str(MESSAGE_TRUNCATED);
            self.message = bounded;
        } else if self.message.capacity() > MAX_DIAGNOSTIC_MESSAGE_BYTES {
            // Truncating length alone would retain the original large allocation.
            self.message = self.message.as_str().to_owned();
        }
        self
    }
}

/// A collection of diagnostics produced by a compiler stage.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    /// Collected diagnostics in source order.
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Create an empty collection.
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a diagnostic to the collection.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub(crate) fn push_bounded(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic.bounded());
    }

    /// Bound a compiler-owned error collection without changing the public raw API.
    /// The final slot is an error notice at the diagnostic's original location.
    pub(crate) fn push_capped_error(
        &mut self,
        mut diagnostic: Diagnostic,
        capacity: usize,
        exhausted_code: &'static str,
        stage: &str,
    ) {
        let capacity = capacity.max(1);
        if self.items.len() >= capacity {
            return;
        }
        if self.items.len() == capacity - 1 {
            diagnostic.severity = Severity::Error;
            diagnostic.code = exhausted_code;
            diagnostic.message = format!("{stage} diagnostic limit of {capacity} reached");
        }
        self.push_bounded(diagnostic);
    }

    pub(crate) fn limit_reached(&self, exhausted_code: &str) -> bool {
        self.items
            .last()
            .is_some_and(|item| item.code == exhausted_code)
    }

    /// Return whether no diagnostics were collected.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_byte_boundary_is_utf8_safe_and_raw_api_is_unchanged() {
        for message in ["x".repeat(4096), "x".repeat(4097), "λ".repeat(3000)] {
            let raw = Diagnostic {
                severity: Severity::Error,
                code: "E3001",
                message: message.clone(),
                source_file: Some("source.svr".into()),
                span: Span {
                    start: 0,
                    end: message.len(),
                    line: 0,
                    column: 0,
                },
            };
            let bounded = raw.clone().bounded();
            assert!(bounded.message.len() <= 4096 && bounded.message.capacity() <= 4096);
            assert_eq!(bounded.code, raw.code);
            assert_eq!(bounded.span, raw.span);
            assert_eq!(bounded.source_file, raw.source_file);
            if message.len() == 4096 {
                assert_eq!(bounded.message, message);
            } else {
                assert!(bounded.message.ends_with(MESSAGE_TRUNCATED));
            }
            assert_eq!(bounded.clone().bounded(), bounded);
            let mut external = Diagnostics::new();
            external.push(raw);
            assert_eq!(external.items[0].message, message);
        }
    }

    #[test]
    fn guarded_messages_release_excess_capacity_without_changing_metadata() {
        let mut message = String::with_capacity(1024 * 1024);
        message.push_str("short");
        let mut errors = Diagnostics::new();
        let span = Span {
            start: 1,
            end: 5,
            line: 0,
            column: 1,
        };
        errors.push_capped_error(
            Diagnostic {
                severity: Severity::Error,
                code: "E3001",
                message,
                span,
                source_file: Some("source.svr".into()),
            },
            256,
            "E3019",
            "semantic",
        );
        let error = &errors.items[0];
        assert_eq!(error.message, "short");
        assert!(error.message.capacity() <= 4096);
        assert_eq!(error.code, "E3001");
        assert_eq!(error.span, span);
        assert_eq!(error.source_file.as_deref(), Some("source.svr"));
    }
}
