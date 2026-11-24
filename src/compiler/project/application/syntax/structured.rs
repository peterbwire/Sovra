//! Integrated partial header coverage, preserving all opaque body/condition gaps.
use super::{
    ApplicationAuthSyntax, ApplicationDataSyntax, ApplicationRenderSyntax,
    CombinedApplicationSyntax,
};
use crate::compiler::{diagnostics::Span, lexer::Lexer};

/// Partial structured source syntax; header coverage is never semantic success.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuredApplicationSyntax {
    /// Original source byte length.
    pub source_bytes: usize,
    /// All original non-EOF tokens, including opaque bodies and conditions.
    pub token_count: usize,
    /// Entry and ordinary declaration/import syntax outside excluded containers.
    pub base: CombinedApplicationSyntax,
    /// Auth headers with every untyped condition/context field preserved.
    pub auth: ApplicationAuthSyntax,
    /// Model/enum headers with nonempty bodies explicitly unparsed.
    pub data: ApplicationDataSyntax,
    /// Page/view signatures with unresolved types/results and opaque UI bodies.
    pub render: ApplicationRenderSyntax,
    /// Tokens unparsed by every projection, never treated as validated syntax.
    pub unparsed_ranges: Vec<Span>,
}

/// Compose supported header projections without typing or executing opaque bodies.
/// Original-token boundary/overlap checks prevent exclusion from repairing invalid
/// declarations. Existing strict APIs are unchanged. Unknown declarations outside
/// recognized containers reject the request; nested unknown forms stay gaps.
pub fn parse_structured(source: &str) -> Result<StructuredApplicationSyntax, String> {
    let auth = super::parse_auth_blocks(source)?;
    let data = super::parse_data_declarations(source)?;
    let render = super::parse_render_declarations(source)?;
    let mut skipped: Vec<_> = auth
        .blocks
        .iter()
        .map(|block| block.span)
        .chain(data.declarations.iter().map(|declaration| declaration.span))
        .chain(
            render
                .declarations
                .iter()
                .map(|declaration| declaration.span),
        )
        .collect();
    skipped.sort_by_key(|span| span.start);
    let base = super::combined::parse_skipping(source, &skipped)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "structured syntax lexical errors")?;
    let gaps = super::combined::intersect(&tokens, &base.unparsed_ranges, &auth.unparsed_ranges);
    let gaps = super::combined::intersect(&tokens, &gaps, &data.unparsed_ranges);
    let unparsed_ranges = super::combined::intersect(&tokens, &gaps, &render.unparsed_ranges);
    Ok(StructuredApplicationSyntax {
        source_bytes: source.len(),
        token_count: tokens.len().saturating_sub(1),
        base,
        auth,
        data,
        render,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_headers_integrate_without_covering_bodies_guards_or_hidden_declarations() {
        let source = "model Item{field:Unknown}\nenum Role{user admin}\nauth session{\nprovider: unknown\nallow user to read on Item where false\n}\npage Home(id:Unknown){\nfn hidden(){}\nuse app.secret;\n}\napp Demo{\ntitle: \"future\"\npage \"/:id\" -> Home\n}\nfn main(){}";
        let parsed = parse_structured(source).unwrap();
        assert_eq!(parsed.data.declarations.len(), 2);
        assert_eq!(parsed.render.declarations.len(), 1);
        assert_eq!(parsed.auth.blocks.len(), 1);
        assert_eq!(parsed.base.declarations.base.base.callables.len(), 1);
        assert!(parsed.base.declarations.imports.is_empty());
        let mut opaque = parsed
            .data
            .declarations
            .iter()
            .filter_map(|declaration| declaration.unparsed_body)
            .collect::<Vec<_>>();
        opaque.push(parsed.render.declarations[0].unparsed_body.unwrap());
        let super::super::PolicyConditionSyntax::Unparsed { span } =
            parsed.auth.blocks[0].policies[0].condition
        else {
            panic!("unparsed condition")
        };
        opaque.push(span);
        opaque.push(parsed.base.entries.entries[0].unparsed_ranges[0]);
        for span in opaque {
            assert!(
                parsed
                    .unparsed_ranges
                    .iter()
                    .any(|gap| gap.start <= span.start && span.end <= gap.end),
                "{}",
                &source[span.start..span.end]
            );
        }
    }
    #[test]
    fn strict_older_apis_boundaries_and_original_unicode_positions_remain_intact() {
        let source = "model Item{field:\"🦀\"} fn f(){}";
        let parsed = parse_structured(source).unwrap();
        let span = parsed.base.declarations.base.base.callables[0]
            .function
            .span;
        assert_eq!(&source[span.start..span.end], "fn f(){}");
        assert_eq!(span.column, source[..span.start].chars().count());
        assert_ne!(span.column, span.start);
        assert!(super::super::parse_with_auth(source).is_err());
        for source in [
            "service Api\nmodel Item{}\n{fn f(){}}",
            "fn pending\nview Card(){}\n(){}",
            "type T=\nmodel Item{}\nInt;",
            "page Home()->Unknown\nmodel Item{}",
        ] {
            assert!(parse_structured(source).is_err(), "{source}");
        }
        assert!(parse_structured(
            "model Item{}\nenum Role{}\npage Home(){}\nview Card(){}\nauth session{}\napp Demo{}"
        )
        .unwrap()
        .unparsed_ranges
        .is_empty()); // Headers only, not type/body validity.
    }
}
