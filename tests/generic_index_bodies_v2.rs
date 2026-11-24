//! Read-only symbolic array projection and exact element types.
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
fn opaque_element_reads_retain_operand_identity_and_source() {
    let source = "// λ😀\nfn first<T>(values:[T],at:Int)->T{return values[at];}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = &body.expressions()[2];
    assert_eq!(&source[e.span.start..e.span.end], "values[at]");
    let GenericCopyExpressionKind::Index { target, index } = e.kind else {
        panic!()
    };
    assert!(
        matches!(body.expression(target).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
    );
    assert!(
        matches!(body.expression(index).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
    );
    let ty = e.ty.expanded().unwrap();
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id) if id==body.signature().syntax().parameters().find("T").unwrap())
    );
}
#[test]
fn nested_projection_and_typed_locals_preserve_exact_element_types() {
    let checked = check(
        "fn nested<T>(values:[[T]])->T{let row:[T]=values[0];let value:T=row[1];return value;}",
    );
    assert!(!checked.bodies()[0].implicit_unit_return());
    let checked = check("fn number<T>(values:[Int])->Float{return values[0]+1;}");
    assert!(matches!(
        checked.bodies()[0].statements()[0],
        GenericCopyStatement::Return {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
    assert!(check_generic_copy_bodies(
        &table("fn wrong<T,U>(values:[T])->U{return values[0];}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn index_requires_int_and_target_requires_array_without_opaque_assumptions() {
    for source in [
        "fn bad<T>(x:[T],at:T)->T{return x[at];}",
        "fn bad<T:std::Key>(x:[T],at:T)->T{return x[at];}",
        "fn bad<T>(x:[T])->T{return x[0.0];}",
        "fn bad<T>(x:[T])->T{return x[true];}",
        "fn bad<T>(x:T)->T{return x[0];}",
        "fn bad<T>(x:String)->String{return x[0];}",
        "fn bad<T>(x:[T])->T{return x[missing];}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn typing_does_not_execute_or_claim_bounds_safety() {
    assert!(check("fn later<T>(x:[T])->T{return x[9223372036854775807];}").expression_count() > 0);
    assert!(check_generic_copy_bodies(
        &table("fn bad<T>(x:[T]){let mut y=x;y[0]=1;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn projected_types_compare_structurally_and_use_existing_quotas() {
    let table = table("fn same<T>(x:[[T]],y:[T])->Bool{return x[0]==y;}");
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
#[test]
fn transparent_array_aliases_project_without_copying_or_rebinding_type_owners() {
    let alias = parse_generic_alias("type Items<T>=[T];", GenericAliasLimits::DEFAULT).unwrap();
    let declarations = [GenericTypeDeclaration::Alias(&alias)];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(
        "fn item<T>(x:Items<T>)->T{return x[0];}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures =
        check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
    assert!(check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).is_ok());
}
