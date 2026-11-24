//! Caller-scoped callee signature substitution with bounded graph provenance.
use sovra::compiler::v2::*;
fn table(key: bool) -> CheckedGenericSignatures {
    let alias = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
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
fn binding(source: &str, callee: &str) -> CheckedGenericCallBinding {
    let base = table(false);
    let types = base.types();
    let a = parse_generic_function(
        "fn caller<U>(value:U)->U{return value;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let b = parse_generic_function(
        "fn nested<T>(values:[T])->[Echo<T>]{}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let c = parse_generic_function("fn empty<T>(){}", GenericFunctionLimits::DEFAULT).unwrap();
    let d = parse_generic_function("fn unit<T>()->Unit{}", GenericFunctionLimits::DEFAULT).unwrap();
    let wrap = parse_generic_function(
        "fn wrap<T>(value:Box<T>)->Box<[T]>{}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures = check_generic_signatures(
        types,
        &[&a, &b, &c, &d, &wrap],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let checked = args(&signatures, source);
    bind_generic_type_arguments(
        &signatures,
        signatures.find("caller").unwrap(),
        signatures.find(callee).unwrap(),
        &checked,
        GenericCallBindingLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn array_parameter_and_return_types_substitute_the_exact_caller_owner() {
    let binding = binding("::<U>", "nested");
    let checked =
        instantiate_generic_signature(&binding, GenericSignatureInstantiationLimits::DEFAULT)
            .unwrap();
    let caller = binding.signatures().signature(binding.caller()).unwrap();
    let expected = caller.syntax().parameters().find("U").unwrap();
    let mut types = vec![&checked.parameter_types()[0]];
    let GenericSignatureReturn::Explicit(result) = checked.return_type() else {
        panic!()
    };
    types.push(result);
    for ty in types {
        let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
            panic!()
        };
        assert!(
            matches!(ty.nodes()[element].kind,ResolvedGenericTypeKind::Parameter(id) if id==expected)
        );
    }
}
#[test]
fn nested_arguments_compose_without_changing_nominal_or_parameter_identity() {
    let checked = instantiate_generic_signature(
        &binding("::<[U]>", "nested"),
        GenericSignatureInstantiationLimits::DEFAULT,
    )
    .unwrap();
    let ty = &checked.parameter_types()[0];
    let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
        panic!()
    };
    assert!(matches!(
        ty.nodes()[element].kind,
        ResolvedGenericTypeKind::Array { .. }
    ));
    let concrete = instantiate_generic_signature(
        &binding("::<Int>", "nested"),
        GenericSignatureInstantiationLimits::DEFAULT,
    )
    .unwrap();
    let ty = &concrete.parameter_types()[0];
    let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
        panic!()
    };
    assert!(matches!(
        ty.nodes()[element].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Int)
    ));
}
#[test]
fn callee_and_argument_origins_resolve_to_different_original_sources() {
    let checked = instantiate_generic_signature(
        &binding("// λ😀\n::<U>", "nested"),
        GenericSignatureInstantiationLimits::DEFAULT,
    )
    .unwrap();
    let ty = &checked.parameter_types()[0];
    let ResolvedGenericTypeKind::Array { element } = ty.nodes()[ty.root()].kind else {
        panic!()
    };
    let root = ty.nodes()[ty.root()].origin;
    let arg = ty.nodes()[element].origin;
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[root.span.start..root.span.end],
        "[T]"
    );
    assert_eq!(
        &ty.node_source(element).unwrap()[arg.span.start..arg.span.end],
        "U"
    );
}
#[test]
fn implicit_and_explicit_unit_returns_keep_their_distinction() {
    let implicit = instantiate_generic_signature(
        &binding("::<Int>", "empty"),
        GenericSignatureInstantiationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(implicit.annotations(), 0);
    assert_eq!(implicit.nodes(), 0);
    assert!(matches!(
        implicit.return_type(),
        GenericSignatureReturn::ImplicitUnit
    ));
    let explicit = instantiate_generic_signature(
        &binding("::<Int>", "unit"),
        GenericSignatureInstantiationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(explicit.annotations(), 1);
    assert!(matches!(
        explicit.return_type(),
        GenericSignatureReturn::Explicit(_)
    ));
}
#[test]
fn exact_and_one_under_cumulative_and_per_type_caps_are_enforced() {
    let binding = binding("::<U>", "nested");
    let checked =
        instantiate_generic_signature(&binding, GenericSignatureInstantiationLimits::DEFAULT)
            .unwrap();
    let exact = GenericSignatureInstantiationLimits {
        max_annotations: checked.annotations(),
        max_nodes: checked.nodes(),
        max_edges: checked.edges(),
        max_work: checked.work(),
        ..GenericSignatureInstantiationLimits::DEFAULT
    };
    assert!(instantiate_generic_signature(&binding, exact).is_ok());
    for limits in [
        GenericSignatureInstantiationLimits {
            max_annotations: exact.max_annotations - 1,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_nodes_per_type: 1,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_edges_per_type: 0,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_work_per_type: 0,
            ..exact
        },
        GenericSignatureInstantiationLimits {
            max_depth: 1,
            ..exact
        },
    ] {
        assert!(
            instantiate_generic_signature(&binding, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn invalid_profiles_and_empty_signature_work_are_not_ignored() {
    let binding = binding("::<Int>", "empty");
    for limits in [
        GenericSignatureInstantiationLimits {
            max_work: 0,
            ..GenericSignatureInstantiationLimits::DEFAULT
        },
        GenericSignatureInstantiationLimits {
            max_depth: 0,
            ..GenericSignatureInstantiationLimits::DEFAULT
        },
        GenericSignatureInstantiationLimits {
            max_depth: 129,
            ..GenericSignatureInstantiationLimits::DEFAULT
        },
    ] {
        assert!(instantiate_generic_signature(&binding, limits).is_err());
    }
    let checked =
        instantiate_generic_signature(&binding, GenericSignatureInstantiationLimits::DEFAULT)
            .unwrap();
    drop(binding);
    assert!(checked
        .binding()
        .signatures()
        .signature(checked.binding().callee())
        .is_some());
}

#[test]
fn nominal_type_heads_remain_exact_while_nested_arguments_are_substituted() {
    let binding = binding("::<Echo<U>>", "wrap");
    let checked =
        instantiate_generic_signature(&binding, GenericSignatureInstantiationLimits::DEFAULT)
            .unwrap();
    let original = &binding
        .signatures()
        .signature(binding.callee())
        .unwrap()
        .parameter_types()[0];
    let ResolvedGenericTypeKind::Declared {
        declaration: expected,
        ..
    } = original.nodes()[original.root()].kind
    else {
        panic!()
    };
    let ty = &checked.parameter_types()[0];
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &ty.nodes()[ty.root()].kind
    else {
        panic!()
    };
    assert_eq!(*declaration, expected);
    let caller = binding
        .signatures()
        .signature(binding.caller())
        .unwrap()
        .syntax()
        .parameters()
        .find("U")
        .unwrap();
    assert!(
        matches!(ty.nodes()[arguments[0]].kind,ResolvedGenericTypeKind::Parameter(id) if id==caller)
    );
    let GenericSignatureReturn::Explicit(ty) = checked.return_type() else {
        panic!()
    };
    let ResolvedGenericTypeKind::Declared { arguments, .. } = &ty.nodes()[ty.root()].kind else {
        panic!()
    };
    assert!(matches!(
        ty.nodes()[arguments[0]].kind,
        ResolvedGenericTypeKind::Array { .. }
    ));
}
