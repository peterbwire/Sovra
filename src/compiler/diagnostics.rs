//! Structured diagnostics shared by compiler stages.

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

    /// Return whether no diagnostics were collected.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
