//! Boolean symbolic typing preserves both checked operands and original source.
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
fn logical_operators_retain_bool_results_and_operand_identity() {
    for (op, expected) in [
        ("&&", GenericLogicalOperator::And),
        ("||", GenericLogicalOperator::Or),
    ] {
        let checked = check(&format!(
            "fn logic<T>(a:Bool,b:Bool)->Bool{{return a{op}b;}}"
        ));
        let body = &checked.bodies()[0];
        let e = &body.expressions()[2];
        assert_eq!(e.ty.primitive(), Some(GenericPrimitive::Bool));
        let GenericCopyExpressionKind::Logical {
            operator,
            left,
            right,
        } = e.kind
        else {
            panic!()
        };
        assert_eq!(operator, expected);
        assert!(
            matches!(body.expression(left).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
        );
        assert!(
            matches!(body.expression(right).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[1].id)
        );
    }
}
#[test]
fn both_sides_require_bool_without_truthiness_or_key_assumptions() {
    for ty in ["T", "Int", "Float", "String", "[Bool]"] {
        for expr in ["a&&true", "false||a", "true&&a", "a||false"] {
            let source = format!("fn bad<T:std::Key>(a:{ty})->Bool{{return {expr};}}");
            assert!(
                check_generic_copy_bodies(&table(&source), GenericCopyBodyLimits::DEFAULT).is_err()
            );
        }
    }
}
#[test]
fn runtime_short_circuit_does_not_hide_invalid_right_operand() {
    for expr in ["false&&missing", "true||missing", "false&&(1+true==2)"] {
        let source = format!("fn bad<T>()->Bool{{return {expr};}}");
        assert!(
            check_generic_copy_bodies(&table(&source), GenericCopyBodyLimits::DEFAULT).is_err()
        );
    }
}
#[test]
fn precedence_nested_comparisons_and_unicode_spans_are_preserved() {
    let source = "// λ😀\nfn logic<T>(a:Int,b:Float,c:Bool)->Bool{\nreturn a<b&&c||false;\n}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "a<b&&c||false");
    let GenericCopyExpressionKind::Logical {
        operator: GenericLogicalOperator::Or,
        left,
        ..
    } = e.kind
    else {
        panic!()
    };
    assert!(matches!(
        body.expression(left).unwrap().kind,
        GenericCopyExpressionKind::Logical {
            operator: GenericLogicalOperator::And,
            ..
        }
    ));
}
#[test]
fn logical_nodes_charge_cumulative_expression_and_work_limits() {
    let table = table("fn logic<T>(a:Bool,b:Bool)->Bool{return a&&b||false;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
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
    ] {
        assert!(check_generic_copy_bodies(&table, limits).is_err());
    }
}
#[test]
fn transparent_bool_aliases_are_accepted() {
    let alias = parse_generic_alias("type Flag<T>=Bool;", GenericAliasLimits::DEFAULT).unwrap();
    let declarations = [GenericTypeDeclaration::Alias(&alias)];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(
        "fn flags<T>(a:Flag<T>,b:Bool)->Bool{return a&&b;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures =
        check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
    assert!(check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).is_ok());
}
