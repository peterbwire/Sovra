//! Generic enum parameter ownership, payload attachment and aggregate admission.
use sovra::compiler::{lexer::Lexer, v2::*};
fn parse(source: &str) -> GenericEnumSyntax {
    parse_generic_enum(source, GenericEnumLimits::DEFAULT).unwrap()
}
#[test]
fn nullary_and_payload_variants_retain_one_parameter_owner_and_original_spans() {
    let source = "// λ😀\nexport enum Outcome<T,E>{Ok(T),Err(E),Many([T]),End,}";
    let syntax = parse(source);
    assert_eq!(syntax.name(), "Outcome");
    assert!(syntax.is_exported());
    assert_eq!(syntax.source(), source);
    assert_eq!(syntax.variants().len(), 4);
    assert!(syntax.variants()[3].payload.is_none());
    for (index, variant) in syntax.variants().iter().enumerate() {
        assert_eq!(
            &source[variant.name_span.start..variant.name_span.end],
            variant.name
        );
        if let Some(payload) = &variant.payload {
            for usage in payload.annotation.uses() {
                let parameter = syntax.parameters().parameter(usage.parameter).unwrap();
                assert!(payload
                    .annotation
                    .parameters()
                    .parameter(usage.parameter)
                    .is_some());
                let span = syntax.payload_type_node_span(index, usage.node).unwrap();
                assert_eq!(&source[span.start..span.end], parameter.name);
                assert_eq!(syntax.parameter_span(usage.parameter).unwrap().line, 1);
            }
        }
    }
    assert_eq!(
        &source[syntax.variants()[2].span.start..syntax.variants()[2].span.end],
        "Many([T])"
    );
    assert!(syntax.payload_type_node_span(3, 0).is_none());
    assert!(syntax.payload_type_node_span(usize::MAX, 0).is_none());
    let other = parse("enum Other<T>{End}");
    assert!(other
        .parameter_span(syntax.parameters().find("T").unwrap())
        .is_none());
}
#[test]
fn nested_generic_payload_commas_are_not_tuple_payloads_and_unit_stays_distinct() {
    let syntax = parse("enum E<T>{One(Map<T,Pair<[T],Int>>),UnitValue(Unit),End}");
    assert_eq!(syntax.variants().len(), 3);
    assert_eq!(syntax.parameter_uses(), 2);
    assert!(syntax.variants()[1].payload.is_some());
    assert!(syntax.variants()[2].payload.is_none());
    assert!(parse("enum Empty<T>{}").variants().is_empty()); // Syntax only; executable enums must be inhabited.
    assert!(parse_generic_enum("enum E<T>{Pair(T,Int)}", GenericEnumLimits::DEFAULT).is_err());
}
#[test]
fn exact_and_one_under_quotas_cover_nullary_names_and_all_payloads_cumulatively() {
    let source = "enum E<T>{One([T]),Two([T]),End}";
    let syntax = parse(source);
    let exact = GenericEnumLimits {
        max_source_bytes: source.len(),
        max_tokens: Lexer::new().tokenize(source).unwrap().len(),
        max_retained_source_bytes: syntax.retained_source_bytes(),
        max_name_bytes: syntax.name_bytes(),
        max_variants: 3,
        max_type_nodes: syntax.type_nodes(),
        max_type_edges: syntax.type_edges(),
        max_parameter_uses: syntax.parameter_uses(),
        ..GenericEnumLimits::DEFAULT
    };
    assert!(parse_generic_enum(source, exact).is_ok());
    for limits in [
        GenericEnumLimits {
            max_source_bytes: source.len() - 1,
            ..exact
        },
        GenericEnumLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericEnumLimits {
            max_retained_source_bytes: exact.max_retained_source_bytes - 1,
            ..exact
        },
        GenericEnumLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericEnumLimits {
            max_variants: 2,
            ..exact
        },
        GenericEnumLimits {
            max_type_nodes: exact.max_type_nodes - 1,
            ..exact
        },
        GenericEnumLimits {
            max_type_edges: exact.max_type_edges - 1,
            ..exact
        },
        GenericEnumLimits {
            max_parameter_uses: exact.max_parameter_uses - 1,
            ..exact
        },
        GenericEnumLimits {
            annotations: GenericTypeLimits {
                max_nodes: 1,
                ..exact.annotations
            },
            ..exact
        },
    ] {
        assert!(parse_generic_enum(source, limits).is_err(), "{limits:?}");
    }
    assert!(parse_generic_enum(
        "enum E<T>{End}",
        GenericEnumLimits {
            max_type_nodes: 0,
            max_type_edges: 0,
            max_parameter_uses: 0,
            ..GenericEnumLimits::DEFAULT
        }
    )
    .is_ok());
}
#[test]
fn duplicate_tuple_named_empty_and_malformed_variants_refuse_whole_declaration() {
    for source in [
        "",
        "enum",
        "enum E{}",
        "enum E<>{}",
        "enum E<T>",
        "enum E<T>{",
        "enum E<T>{A,A}",
        "enum E<T>{A(T),A(Int)}",
        "enum E<T>{A()}",
        "enum E<T>{A(T,Int)}",
        "enum E<T>{A(value:T)}",
        "enum E<T>{A(T<Int>)}",
        "enum E<T>{A([T)}",
        "enum E<T>{A(Map<T>)} trailing",
        "enum E<T>{A(T) B}",
        "enum E<T>{A(T)",
        "enum E<T>{A(T);}",
        "enum E<T>{A=1}",
        "enum E<T>{A{value:T}}",
        "enum E<T>{End}enum F<U>{End}",
        "mod m{enum E<T>{End}}",
    ] {
        assert!(
            parse_generic_enum(source, GenericEnumLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn payload_errors_map_to_original_multiline_coordinates() {
    let source = "// λ😀\nenum E<T>{\n Good(T),\n Bad(T<Int>)\n}";
    let GenericTypeError::Syntax { span, .. } =
        parse_generic_enum(source, GenericEnumLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(span.line, 3);
    assert_eq!(&source[span.start..span.end], "T<Int>");
}
#[test]
fn truncations_refuse_without_eof_panics_and_default_source_checking_stays_strict() {
    for source in [
        "enum E<T>{}",
        "export enum E<T>{One(T),Many(Pair<T,[T]>),End}",
        "// λ😀\nenum E<T>{One(T),End}",
    ] {
        for (end, _) in source.char_indices() {
            assert!(parse_generic_enum(&source[..end], GenericEnumLimits::DEFAULT).is_err());
        }
        assert!(parse_generic_enum(source, GenericEnumLimits::DEFAULT).is_ok());
    }
    assert!(check_program_source(
        "strict",
        "enum E<T>{One(T),End}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
    assert_eq!(parse("enum E<T>{One(Unknown<T>)}").variants().len(), 1);
}
