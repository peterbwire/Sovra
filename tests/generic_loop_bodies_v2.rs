//! Symbolic while scopes and conservative return coverage.
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
fn loop_facts_retain_condition_body_ranges_and_source() {
    let source =
        "// λ😀\nfn repeat<T>(x:T){let mut running=true;while (running){x;running=false;}}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    assert!(body.implicit_unit_return());
    let GenericCopyStatement::While {
        condition,
        body_statements,
        span,
    } = &body.statements()[1]
    else {
        panic!()
    };
    assert_eq!(body_statements.clone(), 2..4);
    assert_eq!(
        &source[span.start..span.end],
        "while (running){x;running=false;}"
    );
    assert!(
        matches!(body.expression(*condition).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
    );
    assert!(
        matches!(body.statements()[3],GenericCopyStatement::Assign{binding,..} if binding==body.bindings()[1].id)
    );
}
#[test]
fn loops_never_supply_nonunit_return_coverage_by_themselves() {
    for source in [
        "fn missing<T>(x:T)->T{while false{return x;}}",
        "fn missing<T>(x:T)->T{while true{return x;}}",
        "fn missing<T>(x:T)->T{if true{while true{return x;}}else{return x;}}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
    for source in [
        "fn after<T>(x:T)->T{while false{return x;}return x;}",
        "fn prior<T>(x:T)->T{return x;while true{}}",
        "fn nested<T>(x:T)->T{if true{return x;while false{}}else{return x;}}",
    ] {
        assert!(!check(source).bodies()[0].implicit_unit_return());
    }
}
#[test]
fn loop_scope_restores_outer_bindings_and_rejects_leaks() {
    let checked = check("fn scope<T>(x:T){let mut y=x;while false{let y=x;y;}y=x;}");
    let body = &checked.bodies()[0];
    assert!(
        matches!(body.statements()[4],GenericCopyStatement::Assign{binding,..} if binding==body.bindings()[1].id)
    );
    for source in [
        "fn bad<T>(x:T){while false{let y=x;}y;}",
        "fn bad<T>(x:T){while false{x=x;}}",
        "fn bad<T>(x:T){while false{let y=x;y=x;}}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err());
    }
}
#[test]
fn nonbool_conditions_and_dead_body_errors_are_rejected() {
    for source in [
        "fn bad<T>(x:T){while (x){}}",
        "fn bad<T:std::Key>(x:T){while (x){}}",
        "fn bad<T>(){while 1{}}",
        "fn bad<T>(){while false{missing;}}",
        "fn bad<T>(){return;while false{missing;}}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn nested_loop_traversal_is_typing_only_and_preserves_flat_ranges() {
    let source = format!(
        "fn loops<T>(x:T){{{}x;{}}}",
        "while true{".repeat(32),
        "}".repeat(32)
    );
    let checked = check(&source);
    assert_eq!(checked.statement_count(), 33);
    assert!(checked.bodies()[0].implicit_unit_return());
    let GenericCopyStatement::While {
        body_statements, ..
    } = &checked.bodies()[0].statements()[0]
    else {
        panic!()
    };
    assert_eq!(body_statements.clone(), 1..33);
}
#[test]
fn loop_scope_snapshots_and_facts_charge_cumulative_limits() {
    let table = table("fn loops<T>(x:T){let mut flag=true;while (flag){let y=x;y;flag=false;}}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_statements: checked.statement_count(),
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
        max_bindings: checked.binding_count(),
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
            max_bindings: exact.max_bindings - 1,
            ..exact
        },
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
