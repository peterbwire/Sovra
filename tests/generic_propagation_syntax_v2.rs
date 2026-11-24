//! Original-source postfix metadata, strict legacy boundaries and cumulative syntax admission.
use sovra::compiler::{lexer::Lexer, parser::Parser, v2::*};
fn parse(source: &str) -> GenericPropagationSyntax {
    parse_generic_propagation_block(source, GenericPropagationSyntaxLimits::DEFAULT).unwrap()
}
fn slice(syntax: &GenericPropagationSyntax, span: sovra::compiler::diagnostics::Span) -> &str {
    &syntax.source()[span.start..span.end]
}
#[test]
fn source_calls_nested_arguments_and_repeated_postfix_retain_exact_ranges() {
    let p = parse("{let value=take::<Int>(first?,other()?)??;}");
    assert_eq!(p.sites().len(), 4);
    let operands: Vec<_> = p.sites().iter().map(|s| slice(&p, s.operand)).collect();
    assert_eq!(
        operands,
        [
            "first",
            "other()",
            "take::<Int>(first?,other()?)",
            "take::<Int>(first?,other()?)?"
        ]
    );
    for site in p.sites() {
        assert_eq!(slice(&p, site.question), "?");
        assert_eq!(site.operand.start, site.expression.start);
        assert_eq!(site.question.end, site.expression.end);
    }
}
#[test]
fn field_index_call_and_group_suffixes_keep_application_order() {
    let p = parse("{let value=(object?.items[index?])?.field?;}");
    let operands: Vec<_> = p.sites().iter().map(|s| slice(&p, s.operand)).collect();
    assert_eq!(
        operands,
        [
            "object",
            "index",
            "(object?.items[index?])",
            "(object?.items[index?])?.field"
        ]
    );
}
#[test]
fn speculative_assignment_target_rollback_never_duplicates_sites() {
    for source in [
        "{array[index?];}",
        "{array[choose(index?)?];}",
        "{array[index?]=replacement?;}",
    ] {
        let p = parse(source);
        assert_eq!(
            p.sites().len(),
            source.chars().filter(|c| *c == '?').count()
        );
        let mut offsets: Vec<_> = p.sites().iter().map(|s| s.question.start).collect();
        offsets.sort_unstable();
        offsets.dedup();
        assert_eq!(offsets.len(), p.sites().len());
    }
}
#[test]
fn loops_branches_matches_and_records_share_the_same_capture() {
    let p=parse("{while condition?{if flag?{let error=std::Error{code:code?,message:\"?\",offset:offset?};}else{match item{std::Option::Some(x)=>{let value=x?;},std::Option::None()=>{return fallback?;}}}}}");
    assert_eq!(p.sites().len(), 6);
}
#[test]
fn comments_strings_unicode_and_crlf_use_original_byte_and_character_coordinates() {
    let source = "{\r\n// λ ?\r\nlet text=\"😀?\"; let value = item // ?\r\n ?;\r\n}";
    let p = parse(source);
    assert_eq!(p.source(), source);
    assert_eq!(p.sites().len(), 1);
    let s = p.sites()[0];
    assert_eq!(slice(&p, s.operand), "item");
    assert_eq!(s.question.line, 3);
    assert_eq!(s.question.column, 1);
    assert!(slice(&p, s.expression).contains("// ?"));
}
#[test]
fn question_marks_in_types_or_nonpostfix_positions_refuse_without_partial_metadata() {
    for source in [
        "{let x=?value;}",
        "{let x=value?field:other;}",
        "{let x=call::<Int?>(1);}",
        "{let x:std::Option<Int?>=value;}",
        "{let x=value?.;}",
        "{value?=1;}",
        "{let x=value?;",
        "{let x=value?;} trailing",
        "fn main(){let x=value?;}",
    ] {
        assert!(
            parse_generic_propagation_block(source, GenericPropagationSyntaxLimits::DEFAULT)
                .is_err(),
            "{source}"
        );
    }
}
#[test]
fn unmapped_type_question_is_reported_at_its_original_token() {
    let source = "{let x=call::<Int?>(value?);}";
    let error = parse_generic_propagation_block(source, GenericPropagationSyntaxLimits::DEFAULT)
        .unwrap_err();
    let GenericPropagationSyntaxError::Source(span) = error else {
        panic!("expected unmapped marker refusal")
    };
    assert_eq!(span.start, source.find('?').unwrap());
    assert_eq!(&source[span.start..span.end], "?");
}

#[test]
fn every_cumulative_input_boundary_is_exact() {
    let source = "{let x=take(first?,second?)?;}";
    let p = parse(source);
    macro_rules! boundary {
        ($field:ident,$value:expr) => {{
            let mut l = GenericPropagationSyntaxLimits::DEFAULT;
            l.$field = $value;
            parse_generic_propagation_block(source, l).unwrap();
            l.$field -= 1;
            assert!(
                parse_generic_propagation_block(source, l).is_err(),
                stringify!($field)
            );
        }};
    }
    boundary!(max_sites, p.sites().len());
    boundary!(max_work, p.work());
    let mut l = GenericPropagationSyntaxLimits::DEFAULT;
    l.frontend.max_source_bytes = source.len();
    parse_generic_propagation_block(source, l).unwrap();
    l.frontend.max_source_bytes -= 1;
    assert!(parse_generic_propagation_block(source, l).is_err());
    let mut l = GenericPropagationSyntaxLimits::DEFAULT;
    l.frontend.max_tokens = p.tokens() - 1;
    parse_generic_propagation_block(source, l).unwrap();
    l.frontend.max_tokens -= 1;
    assert!(parse_generic_propagation_block(source, l).is_err());
}
#[test]
fn exact_chain_limits_and_existing_nesting_guards_refuse_excess() {
    let mut l = GenericPropagationSyntaxLimits::DEFAULT;
    l.max_chain = 2;
    parse_generic_propagation_block("{value??;}", l).unwrap();
    assert!(parse_generic_propagation_block("{value???;}", l).is_err());
    l.max_chain = 0;
    parse_generic_propagation_block("{}", l).unwrap();
    assert!(parse_generic_propagation_block("{value?;}", l).is_err());
    l.max_chain = 129;
    assert!(parse_generic_propagation_block("{}", l).is_err());
    let nested = format!("{{let x={}value?{};}}", "(".repeat(129), ")".repeat(129));
    assert!(
        parse_generic_propagation_block(&nested, GenericPropagationSyntaxLimits::DEFAULT).is_err()
    );
}
#[test]
fn independent_scopes_share_global_site_allowance_and_syntax_does_not_claim_typing() {
    let source = "{let x=1?;if true{let y=missing?;}else{let z=false?;}}";
    let p = parse(source);
    assert_eq!(p.sites().len(), 3);
    let mut l = GenericPropagationSyntaxLimits::DEFAULT;
    l.max_sites = 2;
    assert!(parse_generic_propagation_block(source, l).is_err());
}
#[test]
fn legacy_lexing_parsing_compilation_and_native_profiles_remain_strict() {
    let source = "fn main(){let x=value?;}";
    let diagnostics = Lexer::new().tokenize(source).unwrap_err();
    assert!(diagnostics.items.iter().any(|d| d.code == "E1000"));
    assert!(Parser::new().parse_source(source).is_err());
    let mut tokens = Lexer::new().tokenize("fn main(){let x=value;}").unwrap();
    let at = tokens
        .iter()
        .position(|t| t.kind == sovra::compiler::lexer::TokenKind::Punctuation(';'))
        .unwrap();
    let span = tokens[at].span;
    tokens.insert(
        at,
        sovra::compiler::lexer::Token {
            kind: sovra::compiler::lexer::TokenKind::Punctuation('?'),
            span,
        },
    );
    assert!(Parser::new().parse_tokens(&tokens).is_err());
    assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
    for features in [1u32, 3, 7] {
        let mut bytes = b"SVRG2\0".to_vec();
        bytes.extend_from_slice(&[1, 2]);
        bytes.extend_from_slice(&features.to_le_bytes());
        bytes.extend_from_slice(&(source.len() as u32).to_le_bytes());
        bytes.extend_from_slice(source.as_bytes());
        assert!(decode_generic_native_image(&bytes).is_err());
    }
}
#[test]
fn clones_retain_original_source_and_every_utf8_prefix_is_bounded() {
    let source = "{ // λ\nlet x=take(value?)?; }";
    let p = parse(source);
    let clone = p.clone();
    drop(p);
    assert_eq!(clone.source(), source);
    for end in 0..=source.len() {
        if source.is_char_boundary(end) {
            let _ = parse_generic_propagation_block(
                &source[..end],
                GenericPropagationSyntaxLimits::DEFAULT,
            );
        }
    }
}
