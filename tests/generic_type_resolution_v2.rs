//! Generic header identities, bounded annotation resolution and conservative Key proofs.
use sovra::compiler::v2::*;
fn parameters(source: &str) -> GenericParameters {
    parse_generic_parameters(source, GenericParameterLimits::DEFAULT).unwrap()
}
fn ty(source: &str) -> GenericTypeSyntax {
    parse_generic_type(source, GenericTypeLimits::DEFAULT).unwrap()
}
fn record(source: &str) -> GenericRecordSyntax {
    parse_generic_record(source, GenericRecordLimits::DEFAULT).unwrap()
}
#[test]
fn registry_ids_are_owned_and_annotations_resolve_heads_arguments_arrays_and_parameters() {
    let entry = record("struct Entry<K:std::Key,V>{key:K,value:V}");
    let option =
        parse_generic_enum("enum Option<T>{Some(T),None}", GenericEnumLimits::DEFAULT).unwrap();
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Record(&entry),
            GenericTypeDeclaration::Enum(&option),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let scope = parameters("<T>");
    let syntax = ty("Option<Entry<String,[T]>>");
    let resolved = registry
        .resolve(&syntax, &scope, GenericResolutionLimits::DEFAULT)
        .unwrap();
    assert_eq!(resolved.nodes().len(), syntax.nodes().len());
    assert!(matches!(
        resolved.nodes()[0],
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::String)
    ));
    assert!(
        matches!(resolved.nodes()[1],ResolvedGenericTypeKind::Parameter(id)if id==scope.find("T").unwrap())
    );
    assert!(
        matches!(resolved.nodes().last().unwrap(),ResolvedGenericTypeKind::Declared{declaration,..}if registry.header(*declaration).unwrap().name.as_ref()=="Option")
    );
    let other = register_generic_types(
        &[GenericTypeDeclaration::Record(&entry)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    assert!(other.header(registry.headers()[0].id).is_none());
    assert!(registry.clone().header(registry.headers()[0].id).is_some());
}
#[test]
fn unknown_names_and_wrong_primitive_parameter_and_declared_arities_refuse() {
    let item = record("struct Item<T>{value:T}");
    let registry = register_generic_types(
        &[GenericTypeDeclaration::Record(&item)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let scope = parameters("<T>");
    for source in [
        "Unknown<Int>",
        "pkg::Item<Int>",
        "Item",
        "Item<Int,String>",
        "Int<T>",
        "T<Int>",
        "[Missing]",
    ] {
        assert!(
            registry
                .resolve(&ty(source), &scope, GenericResolutionLimits::DEFAULT)
                .is_err(),
            "{source}"
        );
    }
    assert!(registry
        .resolve(&ty("Item<Int>"), &scope, GenericResolutionLimits::DEFAULT)
        .is_ok());
}
#[test]
fn key_bounds_accept_only_current_supported_keys_and_explicit_symbolic_proofs() {
    let keys = record("struct Keys<K:std::Key>{key:K}");
    let registry = register_generic_types(
        &[GenericTypeDeclaration::Record(&keys)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    for source in ["Keys<Int>", "Keys<Bool>", "Keys<String>", "Keys<K>"] {
        assert!(
            registry
                .resolve(
                    &ty(source),
                    &parameters("<K:std::Key>"),
                    GenericResolutionLimits::DEFAULT
                )
                .is_ok(),
            "{source}"
        );
    }
    for source in [
        "Keys<Float>",
        "Keys<Unit>",
        "Keys<[Int]>",
        "Keys<T>",
        "Keys<Keys<Int>>",
        "Keys<Byte>",
    ] {
        assert!(
            registry
                .resolve(
                    &ty(source),
                    &parameters("<T>"),
                    GenericResolutionLimits::DEFAULT
                )
                .is_err(),
            "{source}"
        );
    }
}
#[test]
fn aliases_are_registered_by_arity_but_unexpanded_aliases_never_prove_key_bounds() {
    let alias = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let keys = record("struct Keys<K:std::Key>{key:K}");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&alias),
            GenericTypeDeclaration::Record(&keys),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let scope = parameters("<T>");
    assert!(registry
        .resolve(&ty("Echo<Int>"), &scope, GenericResolutionLimits::DEFAULT)
        .is_ok());
    assert!(matches!(
        registry.resolve(
            &ty("Keys<Echo<Int>>"),
            &scope,
            GenericResolutionLimits::DEFAULT
        ),
        Err(GenericResolutionError::AliasBoundNeedsExpansion { .. })
    ));
}
#[test]
fn registry_rejects_duplicate_and_reserved_headers_and_primitive_parameter_shadowing() {
    let a = record("struct A<T>{value:T}");
    assert!(register_generic_types(
        &[
            GenericTypeDeclaration::Record(&a),
            GenericTypeDeclaration::Record(&a)
        ],
        GenericRegistryLimits::DEFAULT
    )
    .is_err());
    for source in ["struct Int<T>{value:T}", "struct A<Int>{value:Int}"] {
        let item = record(source);
        assert!(register_generic_types(
            &[GenericTypeDeclaration::Record(&item)],
            GenericRegistryLimits::DEFAULT
        )
        .is_err());
    }
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    assert!(matches!(
        registry.resolve(
            &ty("Int"),
            &parameters("<Int>"),
            GenericResolutionLimits::DEFAULT
        ),
        Err(GenericResolutionError::ParameterScope { .. })
    ));
}
#[test]
fn exact_registry_and_resolution_budgets_admit_and_one_under_refuses_without_partial_tables() {
    let a = record("struct A<T>{value:T}");
    let input = [GenericTypeDeclaration::Record(&a)];
    let exact = GenericRegistryLimits {
        max_declarations: 1,
        max_parameters: 1,
        max_name_bytes: 1,
        max_parameter_source_bytes: a.parameters().source().len(),
    };
    let registry = register_generic_types(&input, exact).unwrap();
    for limits in [
        GenericRegistryLimits {
            max_declarations: 0,
            ..exact
        },
        GenericRegistryLimits {
            max_parameters: 0,
            ..exact
        },
        GenericRegistryLimits {
            max_name_bytes: 0,
            ..exact
        },
        GenericRegistryLimits {
            max_parameter_source_bytes: exact.max_parameter_source_bytes - 1,
            ..exact
        },
    ] {
        assert!(register_generic_types(&input, limits).is_err());
    }
    let syntax = ty("A<[T]>");
    let scope = parameters("<T>");
    let result = registry
        .resolve(&syntax, &scope, GenericResolutionLimits::DEFAULT)
        .unwrap();
    let exact = GenericResolutionLimits {
        max_nodes: syntax.nodes().len(),
        max_edges: syntax.edges(),
        max_work: result.work(),
    };
    assert!(registry.resolve(&syntax, &scope, exact).is_ok());
    for limits in [
        GenericResolutionLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericResolutionLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericResolutionLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(registry.resolve(&syntax, &scope, limits).is_err());
    }
}
#[test]
fn resolved_annotation_keeps_its_owners_but_does_not_validate_declaration_bodies() {
    let a = record("struct A<T>{unresolved:Missing<T>}");
    let registry = register_generic_types(
        &[GenericTypeDeclaration::Record(&a)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let scope = parameters("<T>");
    let syntax = ty("A<Int>");
    let result = registry
        .resolve(&syntax, &scope, GenericResolutionLimits::DEFAULT)
        .unwrap();
    drop(scope);
    drop(syntax);
    drop(registry);
    drop(a);
    assert_eq!(result.syntax().source(), "A<Int>");
    assert_eq!(result.parameters().source(), "<T>");
    assert_eq!(result.registry().headers().len(), 1);
    assert!(check_program_source(
        "strict",
        "struct A<T>{value:T}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
