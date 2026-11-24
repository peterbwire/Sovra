//! Generic function header attachment with explicitly opaque bounded bodies.
use sovra::compiler::{lexer::Lexer, v2::*};
fn parse(source: &str) -> GenericFunctionSyntax {
    parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap()
}
#[test]
fn value_and_return_annotations_share_one_owner_and_preserve_original_coordinates() {
    let source =
        "// λ😀\nexport fn choose<T,K:std::Key>(key:K,values:[T])->Pair<K,[T]>{return values;}";
    let f = parse(source);
    assert_eq!(f.name(), "choose");
    assert!(f.is_exported());
    assert_eq!(f.source(), source);
    assert_eq!(f.value_parameters().len(), 2);
    let value = &f.value_parameters()[1];
    let usage = value.ty.annotation.uses()[0];
    assert_eq!(usage.parameter, f.parameters().find("T").unwrap());
    assert!(f
        .return_type()
        .unwrap()
        .annotation
        .parameters()
        .parameter(usage.parameter)
        .is_some());
    let span = f.value_type_node_span(1, usage.node).unwrap();
    assert_eq!(&source[span.start..span.end], "T");
    assert_eq!(span.line, 1);
    assert!(f.parameter_span(usage.parameter).is_some());
    assert!(f.value_type_node_span(100, 0).is_none());
    assert_eq!(f.body_source(), "{return values;}");
    assert_eq!(
        &source[f.span().start..f.span().end],
        source.trim_start_matches("// λ😀\n")
    );
    let other = parse("fn other<T>(){}");
    assert!(other.parameter_span(usage.parameter).is_none());
    let return_usage = f.return_type().unwrap().annotation.uses()[0];
    let span = f.return_type_node_span(return_usage.node).unwrap();
    assert_eq!(&source[span.start..span.end], "K");
}
#[test]
fn body_is_owned_but_not_claimed_to_be_parsed_or_semantically_checked() {
    let f = parse("fn draft<T>(){this is not valid statement syntax; missing();}");
    assert!(f.body_source().contains("missing"));
    assert!(f.return_type().is_none());
    assert!(f.return_type_node_span(0).is_none());
    let f = parse("fn text<T>(){print(\"}])\"); // }\n if true{let x=[1];}} ");
    assert!(f.body_source().contains("if true"));
    assert!(check_program_source(
        "strict",
        "fn identity<T>(value:T)->T{return value;}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn exact_and_one_under_signature_source_name_and_body_limits_are_independent() {
    let source = "fn pass<T>(x:[T],y:T)->[T]{return x;}";
    let f = parse(source);
    let exact = GenericFunctionLimits {
        max_source_bytes: source.len(),
        max_tokens: Lexer::new().tokenize(source).unwrap().len(),
        max_retained_source_bytes: f.retained_source_bytes(),
        max_name_bytes: f.name_bytes(),
        max_value_parameters: 2,
        max_type_nodes: f.type_nodes(),
        max_type_edges: f.type_edges(),
        max_parameter_uses: f.parameter_uses(),
        max_body_tokens: f.body_tokens(),
        max_body_depth: 1,
        ..GenericFunctionLimits::DEFAULT
    };
    assert!(parse_generic_function(source, exact).is_ok());
    for limits in [
        GenericFunctionLimits {
            max_source_bytes: source.len() - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_retained_source_bytes: exact.max_retained_source_bytes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_value_parameters: 1,
            ..exact
        },
        GenericFunctionLimits {
            max_type_nodes: exact.max_type_nodes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_type_edges: exact.max_type_edges - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_parameter_uses: exact.max_parameter_uses - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_body_tokens: exact.max_body_tokens - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_body_depth: 0,
            ..exact
        },
        GenericFunctionLimits {
            annotations: GenericTypeLimits {
                max_nodes: 1,
                ..exact.annotations
            },
            ..exact
        },
    ] {
        assert!(
            parse_generic_function(source, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn malformed_headers_duplicate_values_and_extra_declarations_refuse() {
    for source in [
        "",
        "fn",
        "fn f(){}",
        "fn f<>(){}",
        "fn f<T>",
        "fn f<T>(",
        "fn f<T>(x){}",
        "fn f<T>(x:){}",
        "fn f<T>(x:T,x:T){}",
        "fn f<T>(x:T<Int>){}",
        "fn f<T>(x:T)->{}",
        "fn f<T>()",
        "fn f<T>(){}fn g<U>(){}",
        "fn f<T>(x:Map<T,T){}",
        "fn f<T>(){{}",
        "fn f<T>(){[)}",
        "fn f<T>();",
        "mod m{fn f<T>(){}}",
    ] {
        assert!(
            parse_generic_function(source, GenericFunctionLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
    assert_eq!(
        parse("fn f<T>(x:Unknown<T>,){}").value_parameters().len(),
        1
    );
}
#[test]
fn return_annotation_errors_are_mapped_into_the_original_multiline_source() {
    let source = "// λ😀\nfn f<T>(x:T)\n -> T<Int> {return x;}";
    let GenericTypeError::Syntax { span, .. } =
        parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(span.line, 2);
    assert_eq!(&source[span.start..span.end], "T<Int>");
}
#[test]
fn opaque_body_depth_has_a_fixed_ceiling_and_truncations_do_not_panic() {
    let source = format!("fn f<T>(){}{}", "{".repeat(128), "}".repeat(128));
    assert!(parse_generic_function(&source, GenericFunctionLimits::DEFAULT).is_ok());
    let deeper = format!("fn f<T>(){}{}", "{".repeat(129), "}".repeat(129));
    assert!(parse_generic_function(&deeper, GenericFunctionLimits::DEFAULT).is_err());
    assert!(parse_generic_function(
        "fn f<T>(){}",
        GenericFunctionLimits {
            max_body_depth: 129,
            ..GenericFunctionLimits::DEFAULT
        }
    )
    .is_err());
    for source in [
        "fn f<T>(){}",
        "export fn f<T,K:std::Key>(x:Map<K,[T]>)->[T]{return x;}",
    ] {
        for (end, _) in source.char_indices() {
            assert!(
                parse_generic_function(&source[..end], GenericFunctionLimits::DEFAULT).is_err()
            );
        }
    }
}
