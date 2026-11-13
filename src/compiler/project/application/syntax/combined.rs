//! Composition of entry and declaration coverage against unmodified source tokens.
use super::{ApplicationEntrySyntax, ApplicationSyntax};
use crate::compiler::{
    diagnostics::Span,
    lexer::{Lexer, TokenKind},
};

/// Combined supported syntax; gaps and semantic validity remain separate.
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedApplicationSyntax {
    /// Original source UTF-8 byte length.
    pub source_bytes: usize,
    /// Original non-EOF token count, including all entry contents.
    pub token_count: usize,
    /// Type, callable, service and import nodes outside entry containers.
    pub declarations: ApplicationSyntax,
    /// Entry containers and recognized binding members.
    pub entries: ApplicationEntrySyntax,
    /// Tokens left unparsed by both projections, including unknown entry fields.
    pub unparsed_ranges: Vec<Span>,
}

/// Combine supported entry/declaration syntax without rewriting the source.
///
/// Only recognized entry envelopes are excluded from ordinary declaration
/// admission. Their unsupported members remain gaps in the combined result.
/// Unknown or malformed declarations outside entries still reject the request.
/// Empty gaps never authorize bindings, validate types or establish M12 success.
pub fn parse_combined(source: &str) -> Result<CombinedApplicationSyntax, String> {
    parse_skipping(source, &[])
}

pub(super) fn parse_skipping(
    source: &str,
    extra: &[Span],
) -> Result<CombinedApplicationSyntax, String> {
    let entries = super::parse_entry_blocks(source)?;
    let mut skipped: Vec<_> = entries.entries.iter().map(|entry| entry.span).collect();
    skipped.extend_from_slice(extra);
    skipped.sort_by_key(|span| span.start);
    if skipped.windows(2).any(|pair| pair[0].end > pair[1].start) {
        return Err("overlapping structured container ranges".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "combined syntax lexical errors")?;
    for span in &skipped {
        let start = tokens.partition_point(|token| token.span.start < span.start);
        let end = tokens.partition_point(|token| token.span.start < span.end);
        if let Some(previous) = start.checked_sub(1).and_then(|index| tokens.get(index)) {
            let terminated = matches!(previous.kind, TokenKind::Punctuation('}' | ';'));
            let line = tokens.partition_point(|token| token.span.line < previous.span.line);
            let import = tokens[line].kind == TokenKind::Keyword("use")
                && super::super::super::imports::parse(
                    &source[tokens[line].span.start..previous.span.end],
                )
                .is_ok_and(|parsed| parsed.is_some());
            if !terminated && !import {
                return Err("structured container interrupts a declaration header".into());
            }
        }
        let next = &tokens[end].kind;
        let declaration = matches!(
            next,
            TokenKind::Eof
                | TokenKind::Keyword("fn" | "export" | "type" | "struct" | "task" | "use")
        ) || matches!(next, TokenKind::Identifier(name) if matches!(name.as_str(), "service" | "task" | "app" | "auth"));
        let next_container = skipped
            .binary_search_by_key(&tokens[end].span.start, |range| range.start)
            .is_ok();
        if !declaration && !next_container {
            return Err("structured container interrupts declaration syntax".into());
        }
    }
    let declarations = super::imports::parse_skipping(source, &skipped)?;

    let unparsed_ranges = intersect(
        &tokens,
        &declarations.unparsed_ranges,
        &entries.unparsed_ranges,
    );
    Ok(CombinedApplicationSyntax {
        source_bytes: source.len(),
        token_count: tokens.len().saturating_sub(1),
        declarations,
        entries,
        unparsed_ranges,
    })
}

pub(super) fn intersect(
    tokens: &[crate::compiler::lexer::Token],
    left: &[Span],
    right: &[Span],
) -> Vec<Span> {
    let mut unparsed_ranges = Vec::new();
    let mut run: Option<Span> = None;
    for token in tokens.iter().filter(|token| token.kind != TokenKind::Eof) {
        let in_gap = |ranges: &[Span]| {
            let index = ranges.partition_point(|range| range.end <= token.span.start);
            ranges
                .get(index)
                .is_some_and(|range| range.start <= token.span.start && token.span.end <= range.end)
        };
        if in_gap(left) && in_gap(right) {
            match &mut run {
                Some(span) => span.end = token.span.end,
                None => run = Some(token.span),
            }
        } else if let Some(span) = run.take() {
            unparsed_ranges.push(span);
        }
    }
    if let Some(span) = run {
        unparsed_ranges.push(span);
    }
    unparsed_ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combines_known_coverage_and_preserves_unknown_entry_ranges() {
        let source = "use app.jobs;\napp Demo{\ntitle: \"λ 🦀\"\nroute GET \"/\" -> jobs.run\n}\nfn main(){print(1);}";
        let parsed = parse_combined(source).unwrap();
        assert_eq!(parsed.declarations.imports.len(), 1);
        assert_eq!(parsed.declarations.base.base.callables.len(), 1);
        assert_eq!(parsed.entries.entries[0].members.len(), 1);
        assert_eq!(parsed.unparsed_ranges.len(), 1);
        let gap = parsed.unparsed_ranges[0];
        assert_eq!(&source[gap.start..gap.end], "title: \"λ 🦀\"");
    }
    #[test]
    fn original_same_line_unicode_positions_and_body_nodes_survive_filtering() {
        let source =
            "// λ\r\napp Demo {\r\nunknown: \"🦀\"\r\n} fn f(value:Int)->Int{return value;}\r\n";
        let parsed = parse_combined(source).unwrap();
        let function = &parsed.declarations.base.base.callables[0].function;
        assert_eq!(
            &source[function.span.start..function.span.end],
            "fn f(value:Int)->Int{return value;}"
        );
        assert_eq!(function.span.line, 3);
        assert_eq!(function.span.column, 2);
        assert_eq!(function.parameters[0].span.line, 3);
        let inline = "app Demo{ unknown: \"🦀\" } fn f(){}";
        let combined = parse_combined(inline).unwrap();
        let span = combined.declarations.base.base.callables[0].function.span;
        let prefix = &inline[..span.start];
        assert_eq!(span.column, prefix.chars().count());
        assert_ne!(span.column, prefix.len());
        assert!(parse_combined("app Demo{} ")
            .unwrap()
            .unparsed_ranges
            .is_empty()); // EOF retained.
    }
    #[test]
    fn unknown_nested_entry_content_stays_unknown_and_other_apis_remain_strict() {
        let source = "app Demo{\nunknown{\nuse not.a.real.import;\nroute GET \"/\" -> missing\n}\n}\nfn main(){}";
        let parsed = parse_combined(source).unwrap();
        assert!(parsed.declarations.imports.is_empty());
        assert!(parsed.entries.entries[0].members.is_empty());
        assert!(!parsed.unparsed_ranges.is_empty());
        assert!(super::super::parse_file(source).is_err());
        assert!(parse_combined("app Demo{}\nmodel Unsupported{}").is_err());
        assert!(parse_combined("app Demo{\nroute GET \"/\"\n}\nfn f(){}").is_err());
    }
}
