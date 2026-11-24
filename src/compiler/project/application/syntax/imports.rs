//! Application import nodes and aggregate supported-source coverage.
use super::ApplicationServiceSyntax;
use crate::compiler::{
    diagnostics::Span,
    lexer::{Lexer, TokenKind},
};

/// Maximum application import declarations retained per source.
pub const MAX_IMPORTS: usize = 4096;

/// Project-relative dotted import syntax, without a resolved filesystem target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSyntax {
    /// Ordered source path segments; duplicates are retained.
    pub segments: Vec<String>,
    /// Original declaration range, including an optional semicolon.
    pub span: Span,
}

/// Aggregate syntax for the currently supported application subset.
/// Even empty gaps never establish semantic validity or M12 completion.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationSyntax {
    /// Original source UTF-8 byte length.
    pub source_bytes: usize,
    /// Original non-EOF token count for cumulative metadata admission.
    pub token_count: usize,
    /// Type, callable/body and service nodes from the same source.
    pub base: ApplicationServiceSyntax,
    /// Project-relative imports in declaration order.
    pub imports: Vec<ImportSyntax>,
    /// Remaining unsupported token envelopes, never treated as checked syntax.
    pub unparsed_ranges: Vec<Span>,
}

/// Retain supported application syntax, including imports, without resolving targets.
/// The existing single-line dotted import grammar is reused unchanged.
pub fn parse_file(source: &str) -> Result<ApplicationSyntax, String> {
    parse_skipping(source, &[])
}

pub(super) fn parse_skipping(source: &str, skipped: &[Span]) -> Result<ApplicationSyntax, String> {
    let base = super::services::parse_skipping(source, skipped)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "application syntax lexical errors")?;
    let mut imports = Vec::new();
    let mut position = 0;
    while position < tokens.len() && tokens[position].kind != TokenKind::Eof {
        let skip = skipped.partition_point(|span| span.end <= tokens[position].span.start);
        if let Some(span) = skipped
            .get(skip)
            .filter(|span| span.start <= tokens[position].span.start)
        {
            position = tokens.partition_point(|token| token.span.start < span.end);
            continue;
        }

        let gap = base
            .unparsed_ranges
            .partition_point(|span| span.end <= tokens[position].span.start);
        let in_gap = base
            .unparsed_ranges
            .get(gap)
            .is_some_and(|span| span.start <= tokens[position].span.start);
        if !in_gap || tokens[position].kind != TokenKind::Keyword("use") {
            position += 1;
            continue;
        }
        if imports.len() == MAX_IMPORTS {
            return Err("application syntax import limit exceeded".into());
        }
        let start = tokens[position].span;
        position += 1;
        while tokens[position].kind != TokenKind::Eof && tokens[position].span.line == start.line {
            if tokens[position].kind == TokenKind::Punctuation(';') {
                position += 1;
                break;
            }
            position += 1;
        }
        let span = Span {
            end: tokens[position - 1].span.end,
            ..start
        };
        let segments = super::super::super::imports::parse(&source[span.start..span.end])
            .map_err(str::to_owned)?
            .ok_or("missing application import")?;
        imports.push(ImportSyntax { segments, span });
    }
    let spans: Vec<_> = imports.iter().map(|import| import.span).collect();
    let unparsed_ranges = super::callables::unparsed(&tokens, &base.unparsed_ranges, &spans);
    Ok(ApplicationSyntax {
        source_bytes: source.len(),
        token_count: tokens.len().saturating_sub(1),
        base,
        imports,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_import_order_duplicates_optional_terminators_and_original_spans() {
        let source = "// λ\r\nuse app.library; // retained\r\nuse other\r\nuse app.library; fn f(){print(\"use not.an.import\");}";
        let parsed = parse_file(source).unwrap();
        assert_eq!(parsed.source_bytes, source.len());
        assert_eq!(parsed.imports.len(), 3);
        assert_eq!(parsed.imports[0].segments, ["app", "library"]);
        assert_eq!(parsed.imports[0].segments, parsed.imports[2].segments);
        let span = parsed.imports[1].span;
        assert_eq!(&source[span.start..span.end], "use other");
        assert!(parsed.unparsed_ranges.is_empty());
    }
    #[test]
    fn preserves_import_grammar_and_never_treats_unknown_forms_as_covered() {
        for source in [
            "use lib::api;",
            "use app..library",
            "use ../outside;",
            "use app . library;",
            "use app.\nlibrary;",
            "use app; view Unknown{}",
        ] {
            assert!(parse_file(source).is_err(), "{source}");
        }
        assert!(parse_file("use missing.module; fn f(){missing();}").is_ok());
        let exact = "use app;\n".repeat(MAX_IMPORTS);
        assert_eq!(parse_file(&exact).unwrap().imports.len(), MAX_IMPORTS);
        assert!(parse_file(&(exact + "use extra;\n"))
            .unwrap_err()
            .contains("import limit"));
    }
}
