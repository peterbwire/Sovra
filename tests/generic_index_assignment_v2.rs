//! Direct mutable array replacement with exact symbolic element types.
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
fn opaque_replacement_preserves_binding_index_value_and_source() {
    let source =
        "// λ😀\nfn replace<T>(xs:[T],item:T,at:Int)->[T]{let mut ys=xs;ys[at]=item;return ys;}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let GenericCopyStatement::IndexAssign {
        binding,
        index,
        value,
        conversion,
        target_span,
        span,
    } = body.statements()[1]
    else {
        panic!()
    };
    assert_eq!(binding, body.bindings()[3].id);
    assert_eq!(conversion, GenericCopyConversion::Identity);
    assert_eq!(&source[target_span.start..target_span.end], "ys[at]");
    assert_eq!(&source[span.start..span.end], "ys[at]=item;");
    assert!(
        matches!(body.expression(index).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[2].id)
    );
    assert!(
        matches!(body.expression(value).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
    );
}
#[test]
fn replacement_widening_is_explicit_and_array_type_remains_fixed() {
    let checked = check("fn widen<T>(xs:[Float])->[Float]{let mut ys=xs;ys[0]=1;return ys;}");
    assert!(matches!(
        checked.bodies()[0].statements()[1],
        GenericCopyStatement::IndexAssign {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
    for source in [
        "fn bad<T>(xs:[Int]){let mut ys=xs;ys[0]=1.0;}",
        "fn bad<T,U>(xs:[T],item:U){let mut ys=xs;ys[0]=item;}",
        "fn bad<T>(xs:[T]){let mut ys=xs;ys[0]=1;}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err());
    }
}
#[test]
fn writes_require_direct_mutable_array_and_concrete_int_index() {
    for source in [
        "fn bad<T>(xs:[T],x:T){xs[0]=x;}",
        "fn bad<T>(xs:[T],x:T){let ys=xs;ys[0]=x;}",
        "fn bad<T>(xs:[[T]],x:T){let mut ys=xs;ys[0][0]=x;}",
        "fn bad<T>(x:T){let mut y=x;y[0]=x;}",
        "fn bad<T>(xs:[T],x:T){let mut ys=xs;ys[false]=x;}",
        "fn bad<T>(xs:[T],x:T){let mut ys=xs;ys[x]=x;}",
        "fn bad<T>(x:T){missing[0]=x;}",
        "fn bad<T>(xs:[T],x:T){let mut ys=xs;ys[0]=missing;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn nested_array_elements_and_branch_scopes_keep_snapshot_value_types() {
    let checked=check("fn rows<T>(xs:[[T]],row:[T])->[[T]]{let mut ys=xs;if true{ys[0]=row;}while false{ys[1]=row;}return ys;}");
    assert_eq!(checked.bodies()[0].bindings().len(), 3);
    assert!(check_generic_copy_bodies(
        &table("fn bad<T>(xs:[T],x:T){let mut ys=xs;if true{let ys=xs;ys[0]=x;}}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn unreachable_writes_check_types_and_bounds_remain_runtime_work() {
    assert!(
        check("fn later<T>(xs:[T],x:T){let mut ys=xs;ys[9223372036854775807]=x;}")
            .statement_count()
            > 0
    );
    assert!(check_generic_copy_bodies(
        &table("fn bad<T>(xs:[T]){let mut ys=xs;return;ys[0]=1;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn indexed_replacement_charges_existing_cumulative_limits() {
    let table = table("fn replace<T>(xs:[T],item:T){let mut ys=xs;ys[0]=item;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_statements: checked.statement_count(),
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
        max_type_pairs: checked.type_pairs(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_statements: exact.max_statements - 1,
            ..exact
        },
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
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
