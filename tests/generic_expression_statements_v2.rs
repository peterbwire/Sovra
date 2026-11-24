//! Checked, discarded symbolic expression results.
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
fn discarded_results_retain_source_types_and_expression_identity() {
    let source = "// λ😀\nfn discard<T>(x:T,a:Int,b:Float){x;a+b;true&&false;\"a\"+\"λ😀\";}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    assert!(body.implicit_unit_return());
    assert_eq!(body.statements().len(), 4);
    for (statement, expected) in body.statements().iter().zip([
        None,
        Some(GenericPrimitive::Float),
        Some(GenericPrimitive::Bool),
        Some(GenericPrimitive::String),
    ]) {
        let GenericCopyStatement::Expression { value } = statement else {
            panic!()
        };
        let expression = body.expression(*value).unwrap();
        assert_eq!(expression.ty.primitive(), expected);
        assert!(!source[expression.span.start..expression.span.end].is_empty());
    }
    let GenericCopyStatement::Expression { value } = body.statements()[3] else {
        panic!()
    };
    let e = body.expression(value).unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "\"a\"+\"λ😀\"");
}
#[test]
fn discarded_expression_does_not_supply_a_nonunit_return() {
    assert!(check_generic_copy_bodies(
        &table("fn missing<T>(x:T)->T{x;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
    let checked = check("fn explicit<T>(x:T)->T{x;return x;}");
    assert!(!checked.bodies()[0].implicit_unit_return());
}
#[test]
fn discarded_and_unreachable_expressions_are_fully_checked() {
    for source in [
        "fn bad<T>(){missing;}",
        "fn bad<T>(){1+true;}",
        "fn bad<T>(){false&&missing;}",
        "fn bad<T>(x:T){missing(x);}",
        "fn bad<T>(x:T){[x];}",
        "fn bad<T>(){return;missing;}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
    assert!(check("fn unreachable<T>(){return;1+2;}").expression_count() > 0);
}
#[test]
fn discarded_reads_follow_mutation_and_shadowing_binding_identity() {
    let checked = check("fn uses<T>(x:T){let mut y=x;y;y=x;let y=y;y;}");
    let body = &checked.bodies()[0];
    for (index, binding) in [(1, 1), (4, 2)] {
        let GenericCopyStatement::Expression { value } = body.statements()[index] else {
            panic!()
        };
        assert!(
            matches!(body.expression(value).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[binding].id)
        );
    }
}
#[test]
fn discarded_expressions_use_existing_cumulative_statement_expression_and_work_limits() {
    let table = table("fn uses<T>(x:T){x;1+2;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_statements: checked.statement_count(),
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
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
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
