//! Contextual symbolic array construction without inference or execution.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(source: &str) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn typed_locals_and_returns_check_opaque_elements_and_empty_arrays() {
    let checked = check("fn build<T>(x:T)->[T]{let items:[T]=[x];return [x];}");
    let body = &checked.bodies()[0];
    assert_eq!(
        body.expressions()
            .iter()
            .filter(|e| matches!(e.kind, GenericCopyExpressionKind::Array(_)))
            .count(),
        2
    );
    let empty = check("fn empty<T>()->[T]{return [];}");
    assert!(
        matches!(&empty.bodies()[0].expressions()[0].kind,GenericCopyExpressionKind::Array(items) if items.is_empty())
    );
}
#[test]
fn nested_arrays_and_assignment_contexts_retain_declared_types() {
    let checked = check(
        "fn rows<T>(x:T)->[[T]]{let mut rows:[[T]]=[[],[x]];rows=[[x]];rows[0]=[x];return rows;}",
    );
    assert!(matches!(
        checked.bodies()[0].statements()[1],
        GenericCopyStatement::Assign { .. }
    ));
    assert!(matches!(
        checked.bodies()[0].statements()[2],
        GenericCopyStatement::IndexAssign { .. }
    ));
}
#[test]
fn scalar_item_widening_is_explicit_and_does_not_change_arithmetic() {
    let checked = check("fn floats<T>()->[Float]{return [1+2,3.0];}");
    let body = &checked.bodies()[0];
    let GenericCopyExpressionKind::Array(items) = &body.expressions().last().unwrap().kind else {
        panic!()
    };
    assert_eq!(items[0].conversion, GenericCopyConversion::IntToFloat);
    assert_eq!(items[1].conversion, GenericCopyConversion::Identity);
    assert_eq!(
        body.expression(items[0].value).unwrap().ty.primitive(),
        Some(GenericPrimitive::Int)
    );
}
#[test]
fn arrays_require_context_and_every_item_must_match_exact_symbolic_type() {
    for source in [
        "fn bad<T>(x:T){let items=[x];}",
        "fn bad<T>(){[];}",
        "fn bad<T>()->T{return [];}",
        "fn bad<T,U>(x:T,y:U)->[T]{return [x,y];}",
        "fn bad<T>()->[T]{return [1];}",
        "fn bad<T>()->[Int]{return [1.0];}",
        "fn bad<T>(x:[Int])->[Float]{return x;}",
        "fn bad<T>()->[Int]{return [missing];}",
        "fn bad<T>()->[Int]{return [];return [true];}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn array_item_ids_and_original_unicode_source_order_are_retained() {
    let source = "// λ😀\nfn texts<T>()->[String]{return [\"λ😀\",\"z\"]; }";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "[\"λ😀\",\"z\"]");
    let GenericCopyExpressionKind::Array(items) = &e.kind else {
        panic!()
    };
    for (item, text) in items.iter().zip(["\"λ😀\"", "\"z\""]) {
        let e = body.expression(item.value).unwrap();
        assert_eq!(&source[e.span.start..e.span.end], text);
    }
}
#[test]
fn arrays_charge_items_comparisons_expression_and_work_budgets() {
    let table = table("fn build<T>(x:T)->[[T]]{return [[x],[]];}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
        max_type_pairs: checked.type_pairs(),
        max_type_edges: checked.type_edges(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_expressions: exact.max_expressions - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_type_pairs: exact.max_type_pairs - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_type_edges: exact.max_type_edges - 1,
            ..exact
        },
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
