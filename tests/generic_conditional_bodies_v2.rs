//! Symbolic conditional scopes, branch facts and return coverage.
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
fn both_branch_returns_cover_nonunit_and_nested_conditions() {
    let checked=check("fn choose<T>(flag:Bool,x:T,y:T)->T{if (flag){if (flag){return x;}else{return y;}}else{return y;}}");
    assert!(!checked.bodies()[0].implicit_unit_return());
    assert_eq!(checked.statement_count(), 5);
    for source in [
        "fn bad<T>(x:T)->T{if true{return x;}}",
        "fn bad<T>(x:T)->T{if true{return x;}else{}}",
        "fn bad<T>(x:T)->T{if true{if false{return x;}}else{return x;}}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
    assert!(
        !check("fn after<T>(x:T)->T{if false{x;}return x;}").bodies()[0].implicit_unit_return()
    );
}
#[test]
fn branches_restore_parent_scope_and_keep_shadowed_identities() {
    let checked = check("fn scopes<T>(x:T){let mut y=x;if true{let y=x;y;}else{y=x;}y=x;}");
    let body = &checked.bodies()[0];
    let GenericCopyStatement::If {
        then_statements,
        else_statements,
        ..
    } = &body.statements()[1]
    else {
        panic!()
    };
    assert_eq!(then_statements.clone(), 2..4);
    assert_eq!(else_statements.clone(), Some(4..5));
    assert!(
        matches!(body.statements()[4],GenericCopyStatement::Assign{binding,..} if binding==body.bindings()[1].id)
    );
    assert!(
        matches!(body.statements()[5],GenericCopyStatement::Assign{binding,..} if binding==body.bindings()[1].id)
    );
    for source in [
        "fn bad<T>(x:T){if true{let local=x;}local;}",
        "fn bad<T>(x:T){if true{let local=x;}else{local;}}",
        "fn bad<T>(x:T){if true{}else{let local=x;}local;}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err());
    }
}
#[test]
fn conditions_require_concrete_bool_and_all_branches_are_checked() {
    for source in [
        "fn bad<T>(x:T){if (x){}}",
        "fn bad<T:std::Key>(x:T){if (x){}}",
        "fn bad<T>(){if 1{}}",
        "fn bad<T>(){if true{}else{missing;}}",
        "fn bad<T>(){if false{missing;}}",
        "fn bad<T>(){return;if true{missing;}}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn empty_else_is_distinct_and_full_source_spans_are_retained() {
    let source = "// λ😀\nfn branch<T>(){if true{}else{}if false{}}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    assert!(body.implicit_unit_return());
    let GenericCopyStatement::If {
        condition,
        then_statements,
        else_statements,
        span,
    } = &body.statements()[0]
    else {
        panic!()
    };
    assert_eq!(&source[span.start..span.end], "if true{}else{}");
    assert_eq!(then_statements.clone(), 1..1);
    assert_eq!(else_statements.clone(), Some(1..1));
    assert_eq!(
        body.expression(*condition).unwrap().ty.primitive(),
        Some(GenericPrimitive::Bool)
    );
    assert!(matches!(
        &body.statements()[1],
        GenericCopyStatement::If {
            else_statements: None,
            ..
        }
    ));
}
#[test]
fn branch_facts_and_scope_snapshots_charge_cumulative_budgets() {
    let table =
        table("fn choose<T>(flag:Bool,x:T)->T{if (flag){let y=x;return y;}else{return x;}}");
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
#[test]
fn nested_branch_traversal_and_prior_return_coverage_remain_bounded() {
    let source = format!(
        "fn nested<T>(x:T)->T{{{}x;{}return x;}}",
        "if true{".repeat(32),
        "}".repeat(32)
    );
    let checked = check(&source);
    assert_eq!(checked.statement_count(), 34);
    assert!(!checked.bodies()[0].implicit_unit_return());
    let checked = check("fn prior<T>(x:T)->T{return x;if true{}else{}}");
    assert!(!checked.bodies()[0].implicit_unit_return());
}
