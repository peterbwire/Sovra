//! Additive syntax-only payload enum metadata, not executable error semantics.
use sovra::compiler::{
    lexer::{Lexer, TokenKind},
    parser::Parser,
    project::application::syntax::parse_data_declarations,
    resources::FrontendLimits,
    sum_syntax::{
        parse_enum_declaration as parse, EnumSyntaxError, EnumSyntaxLimit as Dim,
        EnumSyntaxLimits as Limits,
    },
};
fn refused(source: &str, limits: Limits, dimension: Dim, cap: usize) {
    assert_eq!(
        parse(source, limits),
        Err(EnumSyntaxError::Limit {
            dimension,
            limit: cap
        })
    );
}
#[test]
fn declaration_payloads_duplicates_and_original_unicode_crlf_spans_are_retained() {
    let source="// λ😀\r\nexport enum Outcome {\r\nEnd, Text(String), Rows([ [ lib :: Alias ] ]), Text(Unknown),\r\n} // tail";
    let result = parse(source, Limits::DEFAULT).unwrap();
    assert!(result.is_exported);
    assert_eq!(result.name, "Outcome");
    assert_eq!(
        &source[result.name_span.start..result.name_span.end],
        "Outcome"
    );
    assert_eq!(result.name_span.line, 1);
    assert_eq!(result.name_span.column, 12);
    assert!(source[result.span.start..result.span.end].starts_with("export enum"));
    assert!(source[result.span.start..result.span.end].ends_with('}'));
    assert_eq!(result.variants.len(), 4);
    assert_eq!(
        result
            .variants
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        ["End", "Text", "Rows", "Text"]
    );
    assert_eq!(result.variants[0].payload_type, None);
    assert_eq!(result.variants[0].payload_type_span, None);
    assert_eq!(
        result.variants[2].payload_type.as_deref(),
        Some("[[lib::Alias]]")
    );
    let span = result.variants[2].payload_type_span.unwrap();
    assert_eq!(&source[span.start..span.end], "[ [ lib :: Alias ] ]");
    assert_eq!(
        &source[result.variants[2].span.start..result.variants[2].span.end],
        "Rows([ [ lib :: Alias ] ])"
    );
    let tokens = Lexer::new().tokenize(source).unwrap();
    for span in [
        result.span,
        result.name_span,
        result.variants[0].name_span,
        span,
    ] {
        assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
        let token = tokens.iter().find(|t| t.span.start == span.start).unwrap();
        assert_eq!(
            (span.line, span.column),
            (token.span.line, token.span.column)
        );
    }
    assert!(parse("enum Empty {}", Limits::DEFAULT)
        .unwrap()
        .variants
        .is_empty());
    assert_eq!(
        parse("enum enum { match, enum(Int) }", Limits::DEFAULT)
            .unwrap()
            .variants[0]
            .name,
        "match"
    );
}
#[test]
fn unsupported_and_malformed_forms_never_publish_partial_declarations() {
    for source in [
        "",
        "enum",
        "enum E",
        "enum E {",
        "enum E { A",
        "enum E { A B }",
        "enum E {,}",
        "enum E {A,,}",
        "enum E {A()}",
        "enum E {A(Int,String)}",
        "enum E {A(value:Int)}",
        "enum E {A{value:Int}}",
        "enum E {A=1}",
        "enum E<T>{A}",
        "enum E{A(List<Int>)}",
        "enum E{A(fn()->Int)}",
        "enum E{A(Int::)}",
        "enum E{A([Int)}",
        "enum E{A([Int,String])}",
        "enum E{A(Int)};",
        "enum E{A} enum F{}",
        "enum E{} fn main(){}",
        "model E{A}",
        "#[x] enum E{}",
        "enum E{\"unterminated}",
        "enum E{A} @",
    ] {
        match parse(source, Limits::DEFAULT).unwrap_err() {
            EnumSyntaxError::Diagnostics(errors) => {
                assert!(!errors.items.is_empty(), "{source}");
                for error in errors.items {
                    assert!(error.code.starts_with("E1") || error.code.starts_with("E2"));
                    assert!(
                        error.span.start <= error.span.end && error.span.end <= source.len(),
                        "{source}: {error:?}"
                    );
                    assert!(
                        source.is_char_boundary(error.span.start)
                            && source.is_char_boundary(error.span.end)
                    );
                }
            }
            other => panic!("unexpected resource error for {source}: {other:?}"),
        }
    }
}
#[test]
fn exact_accounting_and_one_under_each_dimension_are_explicit() {
    let source = "enum E { A, B([Int]) }";
    assert_eq!(Lexer::new().tokenize(source).unwrap().len() - 1, 12);
    let limits = Limits {
        max_source_bytes: source.len(),
        max_tokens: 12,
        max_variants: 2,
        max_payload_type_nodes: 2,
        max_retained_text_bytes: 8,
    };
    assert_eq!(parse(source, limits).unwrap().variants.len(), 2);
    refused(
        source,
        Limits {
            max_source_bytes: source.len() - 1,
            ..limits
        },
        Dim::SourceBytes,
        source.len() - 1,
    );
    refused(
        source,
        Limits {
            max_tokens: 11,
            ..limits
        },
        Dim::Tokens,
        11,
    );
    refused(
        source,
        Limits {
            max_variants: 1,
            ..limits
        },
        Dim::Variants,
        1,
    );
    refused(
        source,
        Limits {
            max_payload_type_nodes: 1,
            ..limits
        },
        Dim::PayloadTypeNodes,
        1,
    );
    refused(
        source,
        Limits {
            max_retained_text_bytes: 7,
            ..limits
        },
        Dim::RetainedTextBytes,
        7,
    );
    assert_eq!(
        parse(source, limits).unwrap().variants[1]
            .payload_type
            .as_deref(),
        Some("[Int]")
    ); // fresh counters
}
#[test]
fn zero_empty_and_cumulative_qualified_payload_limits_do_not_mean_unlimited() {
    let zero = Limits {
        max_source_bytes: 0,
        max_tokens: 0,
        max_variants: 0,
        max_payload_type_nodes: 0,
        max_retained_text_bytes: 0,
    };
    refused("@", zero, Dim::SourceBytes, 0);
    refused(
        "enum E{}",
        Limits {
            max_source_bytes: 100,
            ..zero
        },
        Dim::Tokens,
        0,
    );
    refused(
        "enum E{}",
        Limits {
            max_source_bytes: 100,
            max_tokens: 4,
            ..zero
        },
        Dim::RetainedTextBytes,
        0,
    );
    let empty = Limits {
        max_source_bytes: 100,
        max_tokens: 4,
        max_retained_text_bytes: 1,
        ..zero
    };
    assert!(parse("enum E{}", empty).unwrap().variants.is_empty());
    refused(
        "enum E{A}",
        Limits {
            max_tokens: 5,
            ..empty
        },
        Dim::Variants,
        0,
    );
    let source = "enum E{A([[p::T]]),B(Int),A}";
    // Qualified components are one leaf node; labels and canonical spelling count once.
    let limits = Limits {
        max_payload_type_nodes: 4,
        max_retained_text_bytes: 15,
        ..Limits::DEFAULT
    };
    assert!(parse(source, limits).is_ok());
    refused(
        source,
        Limits {
            max_payload_type_nodes: 3,
            ..limits
        },
        Dim::PayloadTypeNodes,
        3,
    );
    refused(
        source,
        Limits {
            max_retained_text_bytes: 14,
            ..limits
        },
        Dim::RetainedTextBytes,
        14,
    );
}
#[test]
fn payload_depth_and_type_spelling_match_the_existing_annotation_grammar() {
    for (layers, accepted) in [
        (0, true),
        (1, true),
        (127, true),
        (128, false),
        (10000, false),
    ] {
        let annotation = format!("{}Int{}", "[".repeat(layers), "]".repeat(layers));
        let source = format!("enum E{{V({annotation})}}");
        let result = parse(&source, Limits::DEFAULT);
        assert_eq!(result.is_ok(), accepted, "{layers}");
        assert_eq!(
            Parser::new()
                .parse_source(&format!("fn f(value:{annotation}){{}}"))
                .is_ok(),
            accepted,
            "{layers}"
        );
        if !accepted {
            assert_eq!(
                result.unwrap_err(),
                EnumSyntaxError::Limit {
                    dimension: Dim::TypeDepth,
                    limit: 128
                }
            );
        }
    }
    for annotation in [
        "Int",
        "Unit",
        "Float",
        "Bool",
        "String",
        "Alias",
        "lib::Type",
        "[[lib::Type]]",
    ] {
        let result = parse(&format!("enum E{{V({annotation})}}"), Limits::DEFAULT).unwrap();
        assert_eq!(result.variants[0].payload_type.as_deref(), Some(annotation));
        assert!(Parser::new()
            .parse_source(&format!("fn f(v:{annotation}){{}}"))
            .is_ok());
    }
}
#[test]
fn lexical_errors_and_resource_dimensions_preserve_public_lexer_contracts() {
    assert!(
        matches!(parse("enum E{} @",Limits::DEFAULT),Err(EnumSyntaxError::Diagnostics(ref errors))if errors.items[0].code=="E1000")
    );
    refused(
        &"@".repeat(300),
        Limits::DEFAULT,
        Dim::LexicalDiagnostics,
        FrontendLimits::DEFAULT.max_diagnostics,
    );
    let limits = FrontendLimits {
        max_source_bytes: 100,
        max_tokens: 1,
        ..FrontendLimits::DEFAULT
    };
    let errors = Lexer::new()
        .tokenize_with_limits("enum E{}", limits)
        .unwrap_err();
    assert_eq!(errors.items[0].code, "E1003");
    assert!(errors.items[0].message.contains("token limit of 1"));
    let tokens = Lexer::new().tokenize("enum match =>").unwrap();
    assert!(matches!(&tokens[0].kind,TokenKind::Identifier(name)if name=="enum"));
    assert!(matches!(&tokens[1].kind,TokenKind::Identifier(name)if name=="match"));
    assert_eq!(tokens[2].kind, TokenKind::Operator("=>"));
}
#[test]
fn executable_parser_and_old_g11_coverage_stay_strict_and_opaque() {
    let source = "enum E{A(Int),B}";
    assert!(parse(source, Limits::DEFAULT).is_ok());
    assert!(Parser::new().parse_source(source).is_err());
    let prior = parse_data_declarations(source).unwrap();
    assert_eq!(prior.declarations.len(), 1);
    assert!(prior.declarations[0].unparsed_body.is_some());
    assert!(!prior.unparsed_ranges.is_empty());
    let ast = Parser::new()
        .parse_source("fn enum(match:Int)->Int{return match;}fn main(){print(enum(7));}")
        .unwrap();
    let ir = sovra::compiler::compile_program(&ast).unwrap().0;
    assert_eq!(sovra::compiler::interpreter::run(&ir).unwrap(), ["7"]);
}
#[test]
fn default_source_and_variant_saturation_are_real_admission_boundaries() {
    let prefix = "enum E{} //";
    let source = prefix.to_owned() + &"x".repeat(Limits::DEFAULT.max_source_bytes - prefix.len());
    assert_eq!(source.len(), 1024 * 1024);
    assert!(parse(&source, Limits::DEFAULT).is_ok());
    refused(
        &(source + "x"),
        Limits::DEFAULT,
        Dim::SourceBytes,
        1024 * 1024,
    );
    let exact = format!("enum E{{{}}}", "A,".repeat(4096));
    assert_eq!(parse(&exact, Limits::DEFAULT).unwrap().variants.len(), 4096);
    let excess = format!("enum E{{{}}}", "A,".repeat(4097));
    refused(&excess, Limits::DEFAULT, Dim::Variants, 4096);
    assert_eq!(
        parse(
            &excess,
            Limits {
                max_variants: 4097,
                ..Limits::DEFAULT
            }
        )
        .unwrap()
        .variants
        .len(),
        4097
    );
}
#[test]
fn default_token_type_and_text_saturation_remain_independent() {
    let path = "T::".repeat(49996) + "T";
    let exact = format!("enum E{{V({path})}}");
    assert_eq!(Lexer::new().tokenize(&exact).unwrap().len() - 1, 100000);
    assert!(parse(&exact, Limits::DEFAULT).is_ok());
    refused(
        &format!("enum E{{V({path}::T)}}"),
        Limits::DEFAULT,
        Dim::Tokens,
        100000,
    );
    let deep = "[".repeat(127) + "Int" + &"]".repeat(127);
    let exact = format!("enum E{{{}}}", format!("V({deep}),").repeat(128));
    assert!(parse(&exact, Limits::DEFAULT).is_ok());
    let excess = exact.trim_end_matches('}').to_owned() + "V(Int)}";
    refused(&excess, Limits::DEFAULT, Dim::PayloadTypeNodes, 16384);
    let name = "N".repeat(256 * 1024 - 1);
    assert!(parse(&format!("enum E{{{name}}}"), Limits::DEFAULT).is_ok());
    refused(
        &format!("enum E{{{name}N}}"),
        Limits::DEFAULT,
        Dim::RetainedTextBytes,
        256 * 1024,
    );
}
