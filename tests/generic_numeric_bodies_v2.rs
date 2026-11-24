//! Concrete numeric typing inside symbolic generic bodies; no execution proof.
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
fn arithmetic_and_ordering_cover_every_operator_and_numeric_pair() {
    for op in ["+", "-", "*", "/", "<", "<=", ">", ">="] {
        for (a, b) in [
            ("Int", "Int"),
            ("Int", "Float"),
            ("Float", "Int"),
            ("Float", "Float"),
        ] {
            let ordered = ["<", "<=", ">", ">="].contains(&op);
            let result = if ordered {
                "Bool"
            } else if a == "Int" && b == "Int" {
                "Int"
            } else {
                "Float"
            };
            let checked = check(&format!(
                "fn numeric<T>(a:{a},b:{b})->{result}{{return a{op}b;}}"
            ));
            let body = &checked.bodies()[0];
            let e = &body.expressions()[2];
            assert_eq!(
                e.ty.primitive(),
                Some(match result {
                    "Bool" => GenericPrimitive::Bool,
                    "Int" => GenericPrimitive::Int,
                    _ => GenericPrimitive::Float,
                })
            );
            let GenericCopyExpressionKind::Numeric {
                left,
                right,
                left_conversion,
                right_conversion,
                ..
            } = e.kind
            else {
                panic!()
            };
            assert!(body.expression(left).is_some());
            assert!(body.expression(right).is_some());
            assert_eq!(
                left_conversion,
                if a == "Int" && b == "Float" {
                    GenericCopyConversion::IntToFloat
                } else {
                    GenericCopyConversion::Identity
                }
            );
            assert_eq!(
                right_conversion,
                if a == "Float" && b == "Int" {
                    GenericCopyConversion::IntToFloat
                } else {
                    GenericCopyConversion::Identity
                }
            );
        }
    }
}
#[test]
fn expected_type_does_not_change_integer_arithmetic() {
    let checked = check("fn widen<T>(a:Int,b:Int)->Float{return a+b;}");
    assert_eq!(
        checked.bodies()[0].expressions()[2].ty.primitive(),
        Some(GenericPrimitive::Int)
    );
    assert!(matches!(
        checked.bodies()[0].statements()[0],
        GenericCopyStatement::Return {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
}
#[test]
fn opaque_and_nonnumeric_operands_are_rejected() {
    for ty in ["T", "Bool", "[Int]"] {
        for op in ["+", "<"] {
            let source =
                format!("fn bad<T:std::Key>(a:{ty},b:{ty})->Bool{{let value=a{op}b;return true;}}");
            assert!(
                check_generic_copy_bodies(&table(&source), GenericCopyBodyLimits::DEFAULT).is_err()
            );
        }
    }
}
#[test]
fn nested_facts_preserve_original_source_and_precedence() {
    let source = "// λ😀\nfn calc<T>(a:Int,b:Float)->Bool{\nreturn a+b*2>=3;\n}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "a+b*2>=3");
    assert!(matches!(
        e.kind,
        GenericCopyExpressionKind::Numeric {
            operator: GenericNumericOperator::GreaterEqual,
            ..
        }
    ));
    assert_eq!(e.ty.primitive(), Some(GenericPrimitive::Bool));
}
#[test]
fn typing_does_not_evaluate_arithmetic_or_prove_runtime_safety() {
    for expr in ["1/0", "9223372036854775807+1"] {
        assert_eq!(
            check(&format!("fn later<T>()->Int{{return {expr};}}")).expression_count(),
            3
        );
    }
}
#[test]
fn numeric_nodes_use_existing_cumulative_expression_and_work_limits() {
    let table = table("fn calc<T>(a:Int,b:Float)->Float{return a+b*2;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_expressions: checked.expression_count(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limit in [
        GenericCopyBodyLimits {
            max_expressions: exact.max_expressions - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(check_generic_copy_bodies(&table, limit).is_err());
    }
}
#[test]
fn transparent_numeric_aliases_are_accepted_without_nominal_coercion() {
    let alias = parse_generic_alias("type Number<T>=Int;", GenericAliasLimits::DEFAULT).unwrap();
    let record =
        parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let declarations = [
        GenericTypeDeclaration::Alias(&alias),
        GenericTypeDeclaration::Record(&record),
    ];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    for (ty, allowed) in [("Number<T>", true), ("Box<Int>", false)] {
        let syntax = parse_generic_function(
            &format!("fn calc<T>(a:{ty},b:{ty})->Int{{return a+b;}}"),
            GenericFunctionLimits::DEFAULT,
        )
        .unwrap();
        let signatures =
            check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
        assert_eq!(
            check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).is_ok(),
            allowed
        );
    }
}
