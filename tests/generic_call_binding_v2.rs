//! Callee type-parameter matching with exact signature/environment/scope owners.
use sovra::compiler::v2::*;
fn table(key: bool) -> CheckedGenericSignatures {
    let alias = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let declarations = [GenericTypeDeclaration::Alias(&alias)];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let caller = format!(
        "fn caller<U{}>(value:U)->U{{return value;}}",
        if key { ":std::Key" } else { "" }
    );
    let a = parse_generic_function(&caller, GenericFunctionLimits::DEFAULT).unwrap();
    let b = parse_generic_function(
        "fn callee<T:std::Key>(value:T)->T{return value;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let c = parse_generic_function(
        "fn pair<A,B>(first:A,second:B)->A{return first;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    check_generic_signatures(&types, &[&a, &b, &c], GenericSignatureLimits::DEFAULT).unwrap()
}
fn args(table: &CheckedGenericSignatures, source: &str) -> CheckedGenericTypeArguments {
    let caller = table.signature(table.find("caller").unwrap()).unwrap();
    check_generic_type_arguments(
        table.types(),
        &parse_generic_type_arguments(source, GenericTypeArgumentLimits::DEFAULT).unwrap(),
        caller.syntax().parameters(),
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap()
}
fn bind(
    table: &CheckedGenericSignatures,
    arguments: &CheckedGenericTypeArguments,
) -> Result<CheckedGenericCallBinding, GenericCallBindingError> {
    bind_generic_type_arguments(
        table,
        table.find("caller").unwrap(),
        table.find("callee").unwrap(),
        arguments,
        GenericCallBindingLimits::DEFAULT,
    )
}
#[test]
fn concrete_and_alias_arguments_bind_exact_callee_parameter_identity() {
    let table = table(false);
    for source in ["::<Int>", "::<Bool>", "::<String>", "::<Echo<Int>>"] {
        let checked = bind(&table, &args(&table, source)).unwrap();
        let callee = table.signature(table.find("callee").unwrap()).unwrap();
        assert_eq!(
            checked.bindings()[0].parameter,
            callee.syntax().parameters().find("T").unwrap()
        );
        assert_eq!(checked.bindings()[0].argument, 0);
        assert_eq!(checked.caller(), table.find("caller").unwrap());
        assert_eq!(checked.callee(), callee.id());
    }
}
#[test]
fn opaque_arguments_need_the_callers_own_key_proof() {
    let plain = table(false);
    assert!(matches!(
        bind(&plain, &args(&plain, "::<U>")),
        Err(GenericCallBindingError::Bound { .. })
    ));
    let keyed = table(true);
    assert!(bind(&keyed, &args(&keyed, "::<U>")).is_ok());
    for source in ["::<Float>", "::<Unit>", "::<[Int]>"] {
        assert!(matches!(
            bind(&keyed, &args(&keyed, source)),
            Err(GenericCallBindingError::Bound { .. })
        ));
    }
}
#[test]
fn foreign_signature_environment_and_scope_owners_are_rejected() {
    let a = table(true);
    let b = table(true);
    let arguments = args(&a, "::<Int>");
    let caller = a.find("caller").unwrap();
    let callee = a.find("callee").unwrap();
    assert_eq!(
        bind_generic_type_arguments(
            &a,
            b.find("caller").unwrap(),
            callee,
            &arguments,
            GenericCallBindingLimits::DEFAULT
        )
        .unwrap_err(),
        GenericCallBindingError::Caller
    );
    assert_eq!(
        bind_generic_type_arguments(
            &a,
            caller,
            b.find("callee").unwrap(),
            &arguments,
            GenericCallBindingLimits::DEFAULT
        )
        .unwrap_err(),
        GenericCallBindingError::Callee
    );
    assert_eq!(
        bind(&a, &args(&b, "::<Int>")).unwrap_err(),
        GenericCallBindingError::Environment
    );
    let foreign =
        parse_generic_parameters("<U:std::Key>", GenericParameterLimits::DEFAULT).unwrap();
    let checked = check_generic_type_arguments(
        a.types(),
        &parse_generic_type_arguments("::<Int>", GenericTypeArgumentLimits::DEFAULT).unwrap(),
        &foreign,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        bind(&a, &checked).unwrap_err(),
        GenericCallBindingError::CallerScope
    );
}
#[test]
fn arity_and_source_order_mapping_are_exact() {
    let table = table(false);
    let arguments = args(&table, "::<Int,Bool>");
    assert_eq!(
        bind(&table, &arguments).unwrap_err(),
        GenericCallBindingError::Arity {
            expected: 1,
            actual: 2
        }
    );
    let pair = table.find("pair").unwrap();
    let checked = bind_generic_type_arguments(
        &table,
        table.find("caller").unwrap(),
        pair,
        &arguments,
        GenericCallBindingLimits::DEFAULT,
    )
    .unwrap();
    let parameters = table.signature(pair).unwrap().syntax().parameters();
    assert_eq!(
        checked.bindings()[0].parameter,
        parameters.find("A").unwrap()
    );
    assert_eq!(
        checked.bindings()[1].parameter,
        parameters.find("B").unwrap()
    );
    assert_eq!(checked.bindings()[1].argument, 1);
}
#[test]
fn bounds_retain_original_unicode_argument_ranges() {
    let table = table(false);
    let source = "// λ😀\n::<Float>";
    let GenericCallBindingError::Bound { argument, span, .. } =
        bind(&table, &args(&table, source)).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(argument, 0);
    assert_eq!(&source[span.start..span.end], "Float");
    assert_eq!(span.line, 1);
}
#[test]
fn exact_binding_and_work_caps_and_owned_result_retention() {
    let table = table(false);
    let arguments = args(&table, "::<Int>");
    let checked = bind(&table, &arguments).unwrap();
    let exact = GenericCallBindingLimits {
        max_bindings: 1,
        max_work: checked.work(),
    };
    let caller = checked.caller();
    let callee = checked.callee();
    assert!(bind_generic_type_arguments(&table, caller, callee, &arguments, exact).is_ok());
    for limits in [
        GenericCallBindingLimits {
            max_bindings: 0,
            ..exact
        },
        GenericCallBindingLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCallBindingLimits {
            max_work: 0,
            ..exact
        },
    ] {
        assert!(bind_generic_type_arguments(&table, caller, callee, &arguments, limits).is_err());
    }
    drop(table);
    drop(arguments);
    assert!(checked.signatures().signature(checked.callee()).is_some());
    assert_eq!(checked.arguments().arguments().len(), 1);
}
