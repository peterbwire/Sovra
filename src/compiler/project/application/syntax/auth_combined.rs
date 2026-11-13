//! Auth-aware coverage composition; conditions always remain explicitly unparsed.
use super::{ApplicationAuthSyntax, CombinedApplicationSyntax};
use crate::compiler::{diagnostics::Span, lexer::Lexer};

/// Supported source syntax including auth headers, without policy validation.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthCombinedSyntax {
    /// Original source byte length.
    pub source_bytes: usize,
    /// Original non-EOF token count, including all auth conditions/context fields.
    pub token_count: usize,
    /// Existing entry/declaration projection with auth envelopes excluded internally.
    pub base: CombinedApplicationSyntax,
    /// Auth headers with explicit unparsed conditions and context fields.
    pub auth: ApplicationAuthSyntax,
    /// Tokens not parsed by either projection; every auth condition stays here.
    pub unparsed_ranges: Vec<Span>,
}

/// Combine supported auth/entry/declaration syntax against original source tokens.
/// This is additive: existing combined parsing and semantic checking stay strict.
/// Header coverage never authorizes policies; unknown declarations outside known
/// containers still reject the request, and unknown nested forms stay unparsed.
pub fn parse_with_auth(source: &str) -> Result<AuthCombinedSyntax, String> {
    let auth = super::parse_auth_blocks(source)?;
    let skipped: Vec<_> = auth.blocks.iter().map(|block| block.span).collect();
    let base = super::combined::parse_skipping(source, &skipped)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "auth-aware syntax lexical errors")?;
    let unparsed_ranges =
        super::combined::intersect(&tokens, &base.unparsed_ranges, &auth.unparsed_ranges);
    Ok(AuthCombinedSyntax {
        source_bytes: source.len(),
        token_count: tokens.len().saturating_sub(1),
        base,
        auth,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn composition_keeps_all_guard_and_context_gaps_without_promoting_nested_nodes() {
        let source = "// λ\r\nauth session{\r\nprovider: unknown\r\nallow user to read on Item where false\r\nunknown{\nfn hidden(){}\nuse app.secret;\n}\r\n}\r\napp Demo{}\r\nfn main(){}";
        let parsed = parse_with_auth(source).unwrap();
        assert_eq!(parsed.auth.blocks[0].policies.len(), 1);
        assert_eq!(parsed.base.declarations.base.base.callables.len(), 1);
        assert!(parsed.base.declarations.imports.is_empty());
        let super::super::PolicyConditionSyntax::Unparsed { span } =
            parsed.auth.blocks[0].policies[0].condition
        else {
            panic!("unparsed guard")
        };
        assert_eq!(&source[span.start..span.end], "where false");
        assert!(parsed
            .unparsed_ranges
            .iter()
            .any(|range| range.start <= span.start && span.end <= range.end));
        assert!(parsed
            .unparsed_ranges
            .iter()
            .any(|range| source[range.start..range.end].contains("fn hidden")));
    }
    #[test]
    fn old_apis_stay_strict_and_original_unicode_positions_survive_new_filtering() {
        let source = "auth session{\nunknown: \"🦀\"\n} fn f(){}";
        let parsed = parse_with_auth(source).unwrap();
        let span = parsed.base.declarations.base.base.callables[0]
            .function
            .span;
        assert_eq!(&source[span.start..span.end], "fn f(){}");
        assert_eq!((span.line, span.column), (2, 2));
        assert!(super::super::parse_combined(source).is_err());
        assert!(super::super::super::check_local_functions(source).is_err());
        assert!(parse_with_auth("auth session{}")
            .unwrap()
            .unparsed_ranges
            .is_empty()); // Syntax only.
        assert!(parse_with_auth("auth session{}\nmodel Unsupported{}").is_err());
        for interrupted in [
            "service Api\nauth session{}\n{fn f(){}}",
            "fn pending\nauth session{}\n(){}",
            "type T =\nauth session{}\nInt;",
            "service Api\napp Demo{}\n{fn f(){}}",
        ] {
            assert!(parse_with_auth(interrupted).is_err(), "{interrupted}");
        }
    }
}
