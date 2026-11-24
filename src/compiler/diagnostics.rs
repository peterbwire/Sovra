//! Structured diagnostics shared by compiler stages.

/// Maximum retained message bytes in guarded compiler diagnostic paths.
/// This excludes source-file identity, JSON escaping and caller-owned raw records.
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4096;
const MESSAGE_TRUNCATED: &str = " [message truncated]";

mod terminal;
pub(crate) use terminal::terminal_message;

/// Internal convenience for emitters that retain an owned message string.
macro_rules! diagnostic_message {
    ($($arguments:tt)*) => {
        $crate::compiler::diagnostics::format_message(format_args!($($arguments)*))
    };
}
pub(crate) use diagnostic_message;

/// Format trusted compiler arguments directly into bounded message storage.
/// Returning fmt::Error on exhaustion stops standard Display/Debug producers;
/// eager argument construction and custom formatter internals remain separate.
pub(crate) fn format_message(arguments: std::fmt::Arguments<'_>) -> String {
    struct Message {
        text: String,
        clipped: bool,
    }
    impl std::fmt::Write for Message {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let remaining = MAX_DIAGNOSTIC_MESSAGE_BYTES - self.text.len();
            if value.len() <= remaining {
                self.text.push_str(value);
                return Ok(());
            }
            let mut end = remaining;
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            self.text.push_str(&value[..end]);
            self.clipped = true;
            Err(std::fmt::Error)
        }
    }
    let mut message = Message {
        text: String::with_capacity(MAX_DIAGNOSTIC_MESSAGE_BYTES),
        clipped: false,
    };
    let _ = std::fmt::write(&mut message, arguments);
    if message.clipped {
        let mut end =
            (MAX_DIAGNOSTIC_MESSAGE_BYTES - MESSAGE_TRUNCATED.len()).min(message.text.len());
        while !message.text.is_char_boundary(end) {
            end -= 1;
        }
        message.text.truncate(end);
        message.text.push_str(MESSAGE_TRUNCATED);
    }
    message.text.shrink_to_fit();
    message.text
}

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

    fn clipped(message: String) -> String {
        Diagnostic {
            severity: Severity::Error,
            code: "E3001",
            message,
            source_file: None,
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        }
        .bounded()
        .message
    }

    #[test]
    fn streaming_messages_match_existing_clipping_for_utf8_debug_and_boundaries() {
        for value in [
            String::new(),
            "x".repeat(4095),
            "x".repeat(4096),
            "x".repeat(4097),
            "λ".repeat(3000),
            "\n\"\\\tλ".repeat(2000),
        ] {
            for (actual, expected) in [
                (
                    format_message(format_args!("{value}")),
                    clipped(value.clone()),
                ),
                (
                    format_message(format_args!("prefix `{value}` suffix")),
                    clipped(format!("prefix `{value}` suffix")),
                ),
                (
                    format_message(format_args!("{value:?}")),
                    clipped(format!("{value:?}")),
                ),
            ] {
                assert_eq!(actual, expected);
                assert!(actual.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES);
                assert!(actual.capacity() <= MAX_DIAGNOSTIC_MESSAGE_BYTES);
            }
        }
    }

    #[test]
    fn streaming_messages_stop_formatters_after_exhaustion() {
        use std::{cell::Cell, fmt};
        struct Repeated<'a>(&'a Cell<usize>);
        impl fmt::Display for Repeated<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for _ in 0..100_000 {
                    self.0.set(self.0.get() + 1);
                    formatter.write_str("λ")?;
                }
                Ok(())
            }
        }
        let calls = Cell::new(0);
        let message = format_message(format_args!("{}", Repeated(&calls)));
        assert!(
            calls.get() < 10_000,
            "formatter kept producing discarded text"
        );
        assert_eq!(message, clipped("λ".repeat(100_000)));
        assert!(message.capacity() <= MAX_DIAGNOSTIC_MESSAGE_BYTES);
    }

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
