//! Caller-scoped explicit arguments, distinct from checked callee binding.
use sovra::compiler::v2::*;
fn types() -> CheckedGenericDeclarations {
    let alias = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let keyed = parse_generic_record(
        "struct Keyed<T:std::Key>{key:T}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let declarations = [
        GenericTypeDeclaration::Alias(&alias),
        GenericTypeDeclaration::Record(&keyed),
    ];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap()
}
fn params(source: &str) -> GenericParameters {
    parse_generic_parameters(source, GenericParameterLimits::DEFAULT).unwrap()
}
fn syntax(source: &str) -> GenericTypeArgumentsSyntax {
    parse_generic_type_arguments(source, GenericTypeArgumentLimits::DEFAULT).unwrap()
}
#[test]
fn caller_scope_aliases_and_argument_order_preserve_exact_parameter_owners() {
    let types = types();
    let parameters = params("<U>");
    let syntax = syntax("::<Echo<U>,[U],Int>");
    let checked = check_generic_type_arguments(
        &types,
        &syntax,
        &parameters,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(checked.arguments().len(), 3);
    let first = &checked.arguments()[0];
    assert!(
        matches!(first.nodes()[first.root()].kind,ResolvedGenericTypeKind::Parameter(id) if id==parameters.find("U").unwrap())
    );
    let array = &checked.arguments()[1];
    assert!(matches!(
        array.nodes()[array.root()].kind,
        ResolvedGenericTypeKind::Array { .. }
    ));
    let last = &checked.arguments()[2];
    assert!(matches!(
        last.nodes()[last.root()].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Int)
    ));
}
#[test]
fn name_head_arity_and_key_constraints_are_checked_for_every_argument() {
    let types = types();
    let parameters = params("<U>");
    for source in [
        "::<Int,Missing>",
        "::<Echo<Int,Bool>>",
        "::<Keyed<Float>>",
        "::<Keyed<U>>",
        "::<U<Int>>",
    ] {
        assert!(
            check_generic_type_arguments(
                &types,
                &syntax(source),
                &parameters,
                GenericArgumentCheckLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
    let parameters = params("<U:std::Key>");
    assert!(check_generic_type_arguments(
        &types,
        &syntax("::<Keyed<U>,Keyed<Echo<Int>>>"),
        &parameters,
        GenericArgumentCheckLimits::DEFAULT
    )
    .is_ok());
}
#[test]
fn errors_preserve_full_list_unicode_ranges_and_argument_index() {
    let types = types();
    let parameters = params("<U>");
    let source = "// λ😀\n::<Int, Missing>";
    let GenericArgumentCheckError::Argument { ordinal, span, .. } = check_generic_type_arguments(
        &types,
        &syntax(source),
        &parameters,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap_err() else {
        panic!()
    };
    assert_eq!(ordinal, 1);
    assert_eq!(&source[span.start..span.end], "Missing");
    assert_eq!(span.line, 1);
    let checked = check_generic_type_arguments(
        &types,
        &syntax("::<Int,[U]>"),
        &parameters,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap();
    let array = &checked.arguments()[1];
    let node = &array.nodes()[array.root()];
    assert_eq!(
        &array.node_source(array.root()).unwrap()[node.origin.span.start..node.origin.span.end],
        "[U]"
    );
}
#[test]
fn every_cumulative_input_copy_and_generated_dimension_is_enforced() {
    let types = types();
    let parameters = params("<U>");
    let syntax = syntax("::<Echo<U>,[U],Int>");
    let checked = check_generic_type_arguments(
        &types,
        &syntax,
        &parameters,
        GenericArgumentCheckLimits::DEFAULT,
    )
    .unwrap();
    let exact = GenericArgumentCheckLimits {
        max_arguments: syntax.roots().len(),
        max_source_bytes: syntax.source().len(),
        max_tokens: syntax.token_count(),
        max_input_nodes: syntax.nodes().len(),
        max_input_edges: syntax.edges(),
        max_name_bytes: syntax.name_bytes(),
        max_nodes: checked.nodes(),
        max_edges: checked.edges(),
        max_work: checked.work(),
        ..GenericArgumentCheckLimits::DEFAULT
    };
    assert!(check_generic_type_arguments(&types, &syntax, &parameters, exact).is_ok());
    for limits in [
        GenericArgumentCheckLimits {
            max_arguments: exact.max_arguments - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_input_nodes: exact.max_input_nodes - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_input_edges: exact.max_input_edges - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericArgumentCheckLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(
            check_generic_type_arguments(&types, &syntax, &parameters, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn independent_per_argument_expansion_caps_never_reset_cumulative_admission() {
    let types = types();
    let parameters = params("<U>");
    let syntax = syntax("::<[U],[U]>");
    assert!(check_generic_type_arguments(
        &types,
        &syntax,
        &parameters,
        GenericArgumentCheckLimits {
            max_nodes: 3,
            ..GenericArgumentCheckLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_generic_type_arguments(
        &types,
        &syntax,
        &parameters,
        GenericArgumentCheckLimits {
            expansion: GenericAliasExpansionLimits {
                max_depth: 1,
                ..GenericAliasExpansionLimits::DEFAULT
            },
            ..GenericArgumentCheckLimits::DEFAULT
        }
    )
    .is_err());
}
#[test]
fn checked_arguments_retain_sources_environment_and_original_scope_after_inputs_drop() {
    let checked = {
        let types = types();
        let parameters = params("<U>");
        check_generic_type_arguments(
            &types,
            &syntax("::<Echo<U>>"),
            &parameters,
            GenericArgumentCheckLimits::DEFAULT,
        )
        .unwrap()
    };
    let ty = &checked.arguments()[0];
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id) if checked.parameters().parameter(id).is_some())
    );
    assert_eq!(checked.syntax().source(), "::<Echo<U>>");
    assert_eq!(checked.types().definitions().len(), 2);
}
