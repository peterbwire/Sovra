//! Source-owned generic alias attachment and aggregate admission.
use sovra::compiler::{lexer::Lexer, v2::*};
fn parse(source: &str) -> GenericAliasSyntax {
    parse_generic_alias(source, GenericAliasLimits::DEFAULT).unwrap()
}
#[test]
fn alias_attaches_its_own_parameters_and_binds_nested_target_uses() {
    let source = "// λ😀\n type Rows<T,K:std::Key> = Map<K,[T]>; // tail";
    let alias = parse(source);
    assert_eq!(alias.source(), source);
    assert_eq!(alias.name(), "Rows");
    assert_eq!(
        &source[alias.span().start..alias.span().end],
        "type Rows<T,K:std::Key> = Map<K,[T]>;"
    );
    assert_eq!(
        &source[alias.name_span().start..alias.name_span().end],
        "Rows"
    );
    assert_eq!(alias.parameters().parameters().len(), 2);
    assert_eq!(alias.target().uses().len(), 2);
    for item in alias.target().uses() {
        let parameter = alias.parameters().parameter(item.parameter).unwrap();
        let full = alias.target_node_span(item.node).unwrap();
        assert_eq!(&source[full.start..full.end], parameter.name);
        let declaration = alias.parameter_span(item.parameter).unwrap();
        assert_eq!(declaration.line, 1);
        assert!(source[declaration.start..declaration.end].starts_with(&parameter.name));
    }
    assert!(alias.target_node_span(usize::MAX).is_none());
}
#[test]
fn adjacent_parameter_end_assignment_is_accepted_without_changing_comparisons() {
    for source in [
        "type Rows<T>=[T];",
        "type Rows<T> = [T];",
        "type Rows<T>//x\n=//y\n[T];",
    ] {
        let alias = parse(source);
        assert_eq!(alias.target().uses().len(), 1);
    }
    assert!(parse_generic_type("A<T>=T", GenericTypeLimits::DEFAULT).is_err());
}
#[test]
fn equal_spelling_aliases_retain_distinct_parameter_owners_and_foreign_spans_refuse() {
    let a = parse("type A<T>=T;");
    let b = parse("type B<T>=T;");
    let id = a.parameters().find("T").unwrap();
    assert!(a.parameter_span(id).is_some());
    assert!(b.parameter_span(id).is_none());
    assert!(a.clone().parameter_span(id).is_some());
}
#[test]
fn fragment_errors_are_reported_in_full_source_coordinates() {
    for source in [
        "// λ😀\ntype A<T,T>=T;",
        "// λ😀\ntype A<T>=\n T<Int>;",
        "// λ😀\ntype A<T:Other>=T;",
    ] {
        let error = parse_generic_alias(source, GenericAliasLimits::DEFAULT).unwrap_err();
        let GenericTypeError::Syntax { span, .. } = error else {
            panic!("{error:?}");
        };
        assert!(span.start >= source.find("type").unwrap());
        assert!(span.line >= 1);
        assert!(span.end <= source.len());
    }
    let source = "// λ😀\ntype A<T>=\n T<Int>;";
    let GenericTypeError::Syntax { span, .. } =
        parse_generic_alias(source, GenericAliasLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(span.line, 2);
    assert_eq!(&source[span.start..span.end], "T<Int>");
}
#[test]
fn declaration_and_fragment_limits_are_exact_and_aggregate() {
    let source = "type Rows<T,K:std::Key>=Map<K,[T]>;";
    let alias = parse(source);
    let exact = GenericAliasLimits {
        max_source_bytes: source.len(),
        max_tokens: Lexer::new().tokenize(source).unwrap().len(),
        max_retained_source_bytes: alias.retained_source_bytes(),
        max_name_bytes: alias.name_bytes(),
        max_parameter_uses: 2,
        ..GenericAliasLimits::DEFAULT
    };
    assert!(parse_generic_alias(source, exact).is_ok());
    for limits in [
        GenericAliasLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericAliasLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericAliasLimits {
            max_retained_source_bytes: exact.max_retained_source_bytes - 1,
            ..exact
        },
        GenericAliasLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericAliasLimits {
            max_parameter_uses: 1,
            ..exact
        },
        GenericAliasLimits {
            parameters: GenericParameterLimits {
                max_parameters: 1,
                ..exact.parameters
            },
            ..exact
        },
        GenericAliasLimits {
            target: GenericTypeLimits {
                max_nodes: 3,
                ..exact.target
            },
            ..exact
        },
    ] {
        assert!(parse_generic_alias(source, limits).is_err(), "{limits:?}");
    }
}
#[test]
fn malformed_extra_declarations_and_applied_parameters_never_publish_partial_success() {
    for source in [
        "",
        "type",
        "type A",
        "type A=Int;",
        "type A<>=Int;",
        "type A<T>;",
        "type A<T>=;",
        "type A<T>=T",
        "type A<T>=T;;",
        "type A<T>=T;type B<U>=U;",
        "type A<T>=T<Int>;",
        "export type A<T>=T;",
        "type A<T<U>>=T;",
        "type A<T>==T;",
    ] {
        assert!(
            parse_generic_alias(source, GenericAliasLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
    let unresolved = parse("type A<T>=Unknown<T>;");
    assert_eq!(unresolved.target().uses().len(), 1);
    assert!(check_program_source(
        "strict",
        "type A<T>=[T];fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn every_utf8_boundary_truncation_refuses_without_indexing_past_eof() {
    for source in [
        "type A<T>=T;",
        "// λ😀\ntype Rows<T,K:std::Key> = Map<K,[T]>;",
        "type A<T> // parameters\n = // target\n [T];",
    ] {
        for (end, _) in source.char_indices() {
            assert!(
                parse_generic_alias(&source[..end], GenericAliasLimits::DEFAULT).is_err(),
                "{source:?} at {end}"
            );
        }
        assert!(parse_generic_alias(source, GenericAliasLimits::DEFAULT).is_ok());
    }
}
