//! Owned generic parameter declarations and bounded annotation-use binding.
use sovra::compiler::{lexer::Lexer, v2::*};
fn parse(source: &str) -> GenericParameters {
    parse_generic_parameters(source, GenericParameterLimits::DEFAULT).unwrap()
}
#[test]
fn parameter_order_bound_and_original_unicode_coordinates_are_retained() {
    let source = "// λ😀\n <T, K : std :: Key> ";
    let list = parse(source);
    assert_eq!(list.source(), source);
    assert_eq!(list.name_bytes(), 2);
    let p = list.parameters();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].bound, None);
    assert_eq!(p[1].bound, Some(GenericParameterBound::Key));
    assert_eq!(p[1].id.ordinal(), 1);
    assert_eq!(p[1].name_span.line, 1);
    let bound = p[1].bound_span.unwrap();
    assert_eq!(&source[bound.start..bound.end], "std :: Key");
    assert_eq!(&source[p[1].span.start..p[1].span.end], "K : std :: Key");
}
#[test]
fn unrelated_declarations_never_share_parameter_identity_and_clones_keep_ownership() {
    let a = parse("<T>");
    let b = parse("<T>");
    let id = a.find("T").unwrap();
    assert!(a.parameter(id).is_some());
    assert!(a.clone().parameter(id).is_some());
    assert!(b.parameter(id).is_none());
    assert_ne!(id, b.find("T").unwrap());
}
#[test]
fn duplicates_unknown_bounds_and_incomplete_or_trailing_forms_refuse() {
    for source in [
        "",
        "<>",
        "<T,T>",
        "<T:std::Key,T>",
        "<T,>",
        "<,T>",
        "<T",
        "T>",
        "<T>>",
        "<T U>",
        "<T:Int>",
        "<T:Key>",
        "<T:other::Key>",
        "<T:std::Other>",
        "<T:std::Key+Other>",
        "<T=Int>",
        "<T:>",
        "<T:std::>",
        "<pkg::T>",
        "<T<U>>",
    ] {
        assert!(
            parse_generic_parameters(source, GenericParameterLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn exact_and_one_under_parameter_source_token_name_quotas_are_independent() {
    let source = "<Item, K:std::Key>";
    let list = parse(source);
    let exact = GenericParameterLimits {
        max_source_bytes: source.len(),
        max_tokens: Lexer::new().tokenize(source).unwrap().len(),
        max_parameters: 2,
        max_name_bytes: list.name_bytes(),
    };
    assert!(parse_generic_parameters(source, exact).is_ok());
    for limits in [
        GenericParameterLimits {
            max_source_bytes: source.len() - 1,
            ..exact
        },
        GenericParameterLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericParameterLimits {
            max_parameters: 1,
            ..exact
        },
        GenericParameterLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
    ] {
        assert!(parse_generic_parameters(source, limits).is_err());
    }
    assert!(parse_generic_parameters(
        "<T>",
        GenericParameterLimits {
            max_parameters: 0,
            ..exact
        }
    )
    .is_err());
}
#[test]
fn binding_links_exact_unqualified_parameter_nodes_and_keeps_both_owners() {
    let list = parse("<T,K:std::Key>");
    let ty = parse_generic_type("Map<K,[T]>", GenericTypeLimits::DEFAULT).unwrap();
    let uses = list.bind_type(&ty, 2).unwrap();
    assert_eq!(uses.uses().len(), 2);
    assert_eq!(uses.uses()[0].node, 0);
    assert_eq!(uses.uses()[0].parameter, list.find("K").unwrap());
    assert_eq!(uses.uses()[1].parameter, list.find("T").unwrap());
    assert_eq!(uses.syntax().source(), ty.source());
    assert_eq!(uses.parameters().source(), list.source());
    assert!(list.bind_type(&ty, 1).is_err());
    let no_uses = parse_generic_type("pkg::T<Int>", GenericTypeLimits::DEFAULT).unwrap();
    assert!(list.bind_type(&no_uses, 0).unwrap().uses().is_empty());
    let applied = parse_generic_type("T<Int>", GenericTypeLimits::DEFAULT).unwrap();
    assert!(list.bind_type(&applied, 10).is_err());
}
#[test]
fn reused_parameter_in_multiple_arguments_keeps_one_identity_and_separate_uses() {
    let list = parse("<T>");
    let ty = parse_generic_type("Pair<T,[T]>", GenericTypeLimits::DEFAULT).unwrap();
    let uses = list.bind_type(&ty, 2).unwrap();
    assert_eq!(uses.uses().len(), 2);
    assert_eq!(uses.uses()[0].parameter, uses.uses()[1].parameter);
    assert_ne!(uses.uses()[0].node, uses.uses()[1].node);
    drop(list);
    drop(ty);
    assert!(uses
        .parameters()
        .parameter(uses.uses()[0].parameter)
        .is_some());
}
