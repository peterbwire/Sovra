//! Direct field-parameter projection preserves caller owners and arenas.
use sovra::compiler::v2::*;
fn table(source: &str) -> CheckedGenericSignatures {
    let record = parse_generic_record(
        "struct Box<T>{count:Int,values:[String],value:T,nested:Box<Int>,dependent:[T],erased:Ignore<T>}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let alias = parse_generic_alias("type Ignore<T>=Int;", GenericAliasLimits::DEFAULT).unwrap();
    let pair =
        parse_generic_record("struct Pair<A,B>{second:B}", GenericRecordLimits::DEFAULT).unwrap();
    let declarations = [
        GenericTypeDeclaration::Record(&record),
        GenericTypeDeclaration::Alias(&alias),
        GenericTypeDeclaration::Record(&pair),
    ];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(source: &str) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn direct_parameter_field_preserves_the_callers_parameter_identity() {
    let checked = check("fn value<U>(box:Box<U>)->U{return box.value;}");
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert!(matches!(
        e.kind,
        GenericCopyExpressionKind::Field {
            parameter_argument: Some(0),
            ..
        }
    ));
    let ty = e.ty.expanded().unwrap();
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id) if id==body.signature().syntax().parameters().find("U").unwrap())
    );
    let receiver = body.expressions()[0].ty.expanded().unwrap();
    assert!(std::ptr::eq(ty.nodes().as_ptr(), receiver.nodes().as_ptr()));
    assert_eq!(ty.syntax().source(), receiver.syntax().source());
}
#[test]
fn concrete_array_and_nominal_arguments_project_without_substitution_arenas() {
    assert_eq!(
        check("fn number<T>(box:Box<Int>)->Int{return box.value+1;}").bodies()[0]
            .expressions()
            .last()
            .unwrap()
            .ty
            .primitive(),
        Some(GenericPrimitive::Int)
    );
    assert!(
        !check("fn array<T>(box:Box<[T]>)->T{return box.value[0];}").bodies()[0]
            .implicit_unit_return()
    );
    assert!(
        !check("fn nested<T>(box:Box<Box<T>>)->T{return box.value.value;}").bodies()[0]
            .implicit_unit_return()
    );
}
#[test]
fn projected_parameter_does_not_gain_numeric_or_foreign_parameter_capabilities() {
    for source in [
        "fn bad<T,U>(box:Box<T>)->U{return box.value;}",
        "fn bad<T:std::Key>(box:Box<T>)->T{return box.value+box.value;}",
        "fn bad<T>(box:Box<T>){let mut b=box;b.value=1;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn original_unicode_field_span_and_argument_provenance_are_retained() {
    let source = "// λ😀\nfn value<T>(box:Box<T>)->T{return box.value;}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "box.value");
    let ty = e.ty.expanded().unwrap();
    let origin = ty.nodes()[ty.root()].origin;
    assert!(origin.alias.is_none());
    assert_eq!(
        &ty.syntax().source()[origin.span.start..origin.span.end],
        "T"
    );
}
#[test]
fn projection_uses_existing_cumulative_expression_work_and_comparison_limits() {
    let table = table("fn value<T>(box:Box<Box<T>>)->T{return box.value.value;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
        max_type_pairs: checked.type_pairs(),
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
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}

#[test]
fn nonfirst_parameter_selects_the_exact_declared_argument() {
    let checked = check("fn second<T,U>(pair:Pair<T,U>)->U{return pair.second;}");
    assert!(matches!(
        checked.bodies()[0].expressions().last().unwrap().kind,
        GenericCopyExpressionKind::Field {
            parameter_argument: Some(1),
            ..
        }
    ));
    assert!(check_generic_copy_bodies(
        &table("fn bad<T,U>(pair:Pair<T,U>)->T{return pair.second;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
