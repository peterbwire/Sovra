//! Original-source match parsing, admission and compatibility regressions.
use sovra::compiler::{
    ast::Statement,
    parser::Parser,
    v2::{
        check_enum_types, parse_match_statement as parse, EnumMatchArm, EnumSource, EnumTypeLimits,
        MatchSyntaxLimits as Limits,
    },
};
#[test]
fn original_spans_qualified_patterns_and_core_bodies_are_retained() {
    let source = "// λ😀\r\nmatch result {\r\nlib::Outcome::Text(text) => { let n = std::array_len([1,2]); if n > 0 { print(text); } },\r\nlib::Outcome::End() => { return; },\r\n}";
    let parsed = parse(source, Limits::DEFAULT).unwrap();
    assert_eq!(parsed.scrutinee, "result");
    assert_eq!(
        &source[parsed.scrutinee_span.start..parsed.scrutinee_span.end],
        "result"
    );
    assert_eq!(parsed.arms.len(), 2);
    let arm = &parsed.arms[0];
    assert_eq!(arm.enum_name, "lib::Outcome");
    assert_eq!(arm.variant_name, "Text");
    assert_eq!(arm.binding.as_deref(), Some("text"));
    let binding = arm.binding_span.unwrap();
    assert_eq!(&source[binding.start..binding.end], "text");
    assert_eq!(binding.line, 2);
    let Statement::Let { span, .. } = &arm.body[0] else {
        panic!("expected let")
    };
    assert!(source[span.start..span.end].starts_with("let n"));
    assert!(matches!(&arm.body[1], Statement::If { .. }));
    assert!(matches!(
        &parsed.arms[1].body[0],
        Statement::Return { value: None, .. }
    ));
    assert_eq!(
        &source[parsed.span.start..parsed.span.end],
        source.trim_start_matches("// λ😀\r\n")
    );
}
#[test]
fn syntax_shapes_feed_the_nominal_coverage_checker_without_body_validation() {
    let context = Parser::new().parse_source("").unwrap();
    let facts = check_enum_types(
        "owner",
        "context",
        &context,
        &[EnumSource {
            label: "enum",
            source: "enum Outcome{Text(String),End}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap();
    let syntax = parse(
        "match result {Outcome::Text(text)=>{missing(text);},Outcome::End()=>{return;}}",
        Limits::DEFAULT,
    )
    .unwrap();
    let arms: Vec<_> = syntax
        .arms
        .iter()
        .map(|arm| EnumMatchArm {
            enum_name: &arm.enum_name,
            variant_name: &arm.variant_name,
            has_payload_binding: arm.binding.is_some(),
            span: arm.span,
        })
        .collect();
    facts
        .check_match(facts.find("Outcome").unwrap(), "match", syntax.span, &arms)
        .unwrap();
    // Undefined body names are retained syntax, never passed off as semantic success.
    assert!(matches!(syntax.arms[0].body[0], Statement::Expression(_)));
}
#[test]
fn malformed_patterns_and_bodies_never_return_partial_success() {
    for source in [
        "",
        "match(x)",
        "match obtain() {}",
        "match value.field {}",
        "match x {E::A(a,b)=>{}}",
        "match x {E::A(mut x)=>{}}",
        "match x {E::A(_)=>{}}",
        "match x {E::A(x) if true=>{}}",
        "match x {E::A(x)|E::B(x)=>{}}",
        "match x {E::A(E::B(x))=>{}}",
        "match x {E::A()=>return;}",
        "match x {E::A()=>{let x=;}}",
        "match x {E::A()=>{}} trailing",
        "match x {E::A()=>{} E::B()=>{}}",
        "match x {E::A()=>{}",
        "match x {E::A()=>{",
        "match x {",
        "match x {E::A()=>{match x {E::A()=>{}}}}}",
    ] {
        assert!(parse(source, Limits::DEFAULT).is_err(), "{source}");
    }
}
#[test]
fn limits_are_exact_zero_is_real_and_errors_do_not_poison_later_parses() {
    let source = "match x{E::V(v)=>{}}";
    let limits = Limits {
        max_arms: 1,
        max_header_text_bytes: 4,
        ..Limits::DEFAULT
    };
    assert!(parse(source, limits).is_ok());
    for limits in [
        Limits {
            max_arms: 0,
            ..limits
        },
        Limits {
            max_header_text_bytes: 3,
            ..limits
        },
        Limits {
            frontend: sovra::compiler::resources::FrontendLimits {
                max_source_bytes: source.len() - 1,
                ..limits.frontend
            },
            ..limits
        },
        Limits {
            frontend: sovra::compiler::resources::FrontendLimits {
                max_tokens: 0,
                ..limits.frontend
            },
            ..limits
        },
    ] {
        assert_eq!(parse(source, limits).unwrap_err().items[0].code, "E1003");
        assert!(parse(source, Limits::DEFAULT).is_ok());
    }
    assert!(parse(
        "match x{}",
        Limits {
            max_arms: 0,
            max_header_text_bytes: 1,
            ..Limits::DEFAULT
        }
    )
    .is_ok());
    assert!(parse(
        "match x{}",
        Limits {
            max_header_text_bytes: 0,
            ..Limits::DEFAULT
        }
    )
    .is_err());
}
#[test]
fn deep_sources_refuse_before_recursive_body_parsing() {
    let source = format!(
        "match x{{E::V()=>{{{}true{}}}}}",
        "if true {".repeat(129),
        "}".repeat(129)
    );
    assert!(parse(&source, Limits::DEFAULT).is_err());
    // Delimiters inside strings are not structural depth.
    let source = format!("match x{{E::V()=>{{print(\"{}\");}}}}", "{".repeat(500));
    assert!(parse(&source, Limits::DEFAULT).is_ok());
}
#[test]
fn contextual_identifiers_remain_ordinary_in_v1_and_arm_bodies() {
    assert!(Parser::new()
        .parse_source("fn match(enum:Int)->Int{return enum;} fn main(){let enum=match(1);}")
        .is_ok());
    assert!(parse(
        "match match{enum::match(enum)=>{match(enum);}}",
        Limits::DEFAULT
    )
    .is_ok());
    assert!(Parser::new()
        .parse_source("fn main(){match x{E::V()=>{}}}")
        .is_err());
}
#[test]
fn duplicate_and_empty_arm_lists_remain_syntax_only() {
    assert!(parse("match x{}", Limits::DEFAULT).unwrap().arms.is_empty());
    assert_eq!(
        parse("match x{E::V()=>{},E::V()=>{}}", Limits::DEFAULT)
            .unwrap()
            .arms
            .len(),
        2
    );
}

#[test]
fn token_source_and_diagnostic_boundaries_are_honored_by_body_parser() {
    use sovra::compiler::lexer::Lexer;
    let source = "match x{E::V()=>{print(1);}}";
    let count = Lexer::new().tokenize(source).unwrap().len() - 1;
    let limits = Limits {
        frontend: sovra::compiler::resources::FrontendLimits {
            max_source_bytes: source.len(),
            max_tokens: count,
            ..Limits::DEFAULT.frontend
        },
        ..Limits::DEFAULT
    };
    assert!(parse(source, limits).is_ok());
    assert_eq!(
        parse(
            source,
            Limits {
                frontend: sovra::compiler::resources::FrontendLimits {
                    max_tokens: count - 1,
                    ..limits.frontend
                },
                ..limits
            }
        )
        .unwrap_err()
        .items[0]
            .code,
        "E1003"
    );
    for max_diagnostics in [0, 1, 2] {
        let error = parse(
            "match x{E::V()=>{let a=;let b=;let c=;}}",
            Limits {
                frontend: sovra::compiler::resources::FrontendLimits {
                    max_diagnostics,
                    ..limits.frontend
                },
                ..limits
            },
        )
        .unwrap_err();
        assert!(error.items.len() <= max_diagnostics.max(1));
    }
}
