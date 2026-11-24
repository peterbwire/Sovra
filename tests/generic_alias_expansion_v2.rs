//! Transparent generic alias substitution, cycles, symbolic bounds and cumulative limits.
use sovra::compiler::v2::*;
fn alias(s: &str) -> GenericAliasSyntax {
    parse_generic_alias(s, GenericAliasLimits::DEFAULT).unwrap()
}
fn record(s: &str) -> GenericRecordSyntax {
    parse_generic_record(s, GenericRecordLimits::DEFAULT).unwrap()
}
fn scope() -> GenericParameters {
    parse_generic_parameters("<T>", GenericParameterLimits::DEFAULT).unwrap()
}
fn ty(s: &str) -> GenericTypeSyntax {
    parse_generic_type(s, GenericTypeLimits::DEFAULT).unwrap()
}
#[test]
fn aliases_expand_by_parameter_identity_and_keep_nominal_heads_and_shared_argument_roots() {
    let echo = alias("type Echo<K>=K;");
    let twice = alias("type Twice<T>=Pair<T,T>;");
    let pair = record("struct Pair<A,B>{a:A,b:B}");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&echo),
            GenericTypeDeclaration::Alias(&twice),
            GenericTypeDeclaration::Record(&pair),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let env = prepare_generic_aliases(
        &registry,
        &[&echo, &twice],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap();
    let scope = scope();
    let result = env
        .expand(&ty("Echo<T>"), &scope, GenericAliasExpansionLimits::DEFAULT)
        .unwrap();
    assert!(
        matches!(result.nodes()[result.root()].kind,ResolvedGenericTypeKind::Parameter(id)if id==scope.find("T").unwrap())
    );
    let result = env
        .expand(
            &ty("Twice<[Int]>"),
            &scope,
            GenericAliasExpansionLimits::DEFAULT,
        )
        .unwrap();
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &result.nodes()[result.root()].kind
    else {
        panic!()
    };
    assert_eq!(registry.header(*declaration).unwrap().name.as_ref(), "Pair");
    assert_eq!(arguments[0], arguments[1]);
    for node in result.nodes() {
        if let ResolvedGenericTypeKind::Declared { declaration, .. } = node.kind {
            assert_ne!(
                registry.header(declaration).unwrap().kind,
                GenericTypeDeclarationKind::Alias
            );
        }
    }
}
#[test]
fn explicit_root_and_provenance_preserve_unused_eager_arguments_and_alias_constants() {
    let first = alias("type First<A,B>=A;");
    let text = alias("type Text<T>=String;");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&first),
            GenericTypeDeclaration::Alias(&text),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let env = prepare_generic_aliases(
        &registry,
        &[&first, &text],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap();
    let result = env
        .expand(
            &ty("First<Int,String>"),
            &scope(),
            GenericAliasExpansionLimits::DEFAULT,
        )
        .unwrap();
    assert_eq!(result.root(), 0);
    assert_eq!(result.nodes().len(), 2);
    assert!(matches!(
        result.nodes()[0].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Int)
    ));
    let result = env
        .expand(
            &ty("Text<Int>"),
            &scope(),
            GenericAliasExpansionLimits::DEFAULT,
        )
        .unwrap();
    let origin = result.nodes()[result.root()].origin;
    let source = env.alias_source(origin.alias.unwrap()).unwrap();
    assert_eq!(&source[origin.span.start..origin.span.end], "String");
}
#[test]
fn expanded_key_proofs_accept_transparent_keys_and_refuse_false_or_unproven_keys() {
    let echo = alias("type Echo<T>=T;");
    let keys = record("struct Keys<K:std::Key>{key:K}");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&echo),
            GenericTypeDeclaration::Record(&keys),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let env = prepare_generic_aliases(&registry, &[&echo], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    assert!(matches!(
        registry.resolve(
            &ty("Keys<Echo<Int>>"),
            &scope(),
            GenericResolutionLimits::DEFAULT
        ),
        Err(GenericResolutionError::AliasBoundNeedsExpansion { .. })
    ));
    assert!(env
        .expand(
            &ty("Keys<Echo<Int>>"),
            &scope(),
            GenericAliasExpansionLimits::DEFAULT
        )
        .is_ok());
    for s in ["Keys<Echo<Float>>", "Keys<Echo<[Int]>>", "Keys<Echo<T>>"] {
        assert!(matches!(
            env.expand(&ty(s), &scope(), GenericAliasExpansionLimits::DEFAULT),
            Err(GenericAliasExpansionError::Bound(_))
        ));
    }
}
#[test]
fn bad_symbolic_alias_bounds_refuse_even_if_a_concrete_use_could_have_satisfied_them() {
    let bad = alias("type Bad<T>=Keys<T>;");
    let good = alias("type Good<K:std::Key>=Keys<K>;");
    let keys = record("struct Keys<K:std::Key>{key:K}");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&bad),
            GenericTypeDeclaration::Record(&keys),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        prepare_generic_aliases(&registry, &[&bad], GenericAliasEnvironmentLimits::DEFAULT),
        Err(GenericAliasExpansionError::Bound(_))
    ));
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&good),
            GenericTypeDeclaration::Record(&keys),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let env = prepare_generic_aliases(&registry, &[&good], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    assert!(env
        .expand(
            &ty("Good<Int>"),
            &scope(),
            GenericAliasExpansionLimits::DEFAULT
        )
        .is_ok());
    assert!(env
        .expand(
            &ty("Good<Float>"),
            &scope(),
            GenericAliasExpansionLimits::DEFAULT
        )
        .is_err());
}
#[test]
fn direct_indirect_nested_and_growing_cycles_refuse_without_recursive_expansion() {
    for sources in [
        vec!["type A<T>=A<T>;"],
        vec!["type A<T>=B<T>;", "type B<T>=A<T>;"],
        vec!["type A<T>=A<[T]>;"],
        vec!["type A<T>=Pair<T,A<T>>;"],
    ] {
        let aliases: Vec<_> = sources.iter().map(|s| alias(s)).collect();
        let pair = record("struct Pair<A,B>{a:A,b:B}");
        let mut declarations: Vec<_> = aliases.iter().map(GenericTypeDeclaration::Alias).collect();
        declarations.push(GenericTypeDeclaration::Record(&pair));
        let registry =
            register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
        let refs: Vec<_> = aliases.iter().collect();
        assert!(matches!(
            prepare_generic_aliases(&registry, &refs, GenericAliasEnvironmentLimits::DEFAULT),
            Err(GenericAliasExpansionError::Cycle(_))
        ));
    }
}
#[test]
fn missing_duplicate_foreign_and_invalid_targets_refuse_environment_publication() {
    let a = alias("type A<T>=T;");
    let registry = register_generic_types(
        &[GenericTypeDeclaration::Alias(&a)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    assert!(
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).is_err()
    );
    assert!(
        prepare_generic_aliases(&registry, &[&a, &a], GenericAliasEnvironmentLimits::DEFAULT)
            .is_err()
    );
    let foreign = alias("type A<T>=T;");
    assert!(prepare_generic_aliases(
        &registry,
        &[&foreign],
        GenericAliasEnvironmentLimits::DEFAULT
    )
    .is_err());
    let invalid = alias("type Invalid<T>=Missing<T>;");
    let registry = register_generic_types(
        &[GenericTypeDeclaration::Alias(&invalid)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        prepare_generic_aliases(
            &registry,
            &[&invalid],
            GenericAliasEnvironmentLimits::DEFAULT
        ),
        Err(GenericAliasExpansionError::Resolution { alias: Some(_), .. })
    ));
}
#[test]
fn exact_environment_and_output_limits_admit_and_one_under_refuses() {
    let a = alias("type A<T>=[T];");
    let b = alias("type B<T>=A<T>;");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&a),
            GenericTypeDeclaration::Alias(&b),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let refs = [&a, &b];
    let env =
        prepare_generic_aliases(&registry, &refs, GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let exact = GenericAliasEnvironmentLimits {
        max_aliases: 2,
        max_nodes: refs
            .iter()
            .map(|a| 2 * a.target().syntax().nodes().len())
            .sum(),
        max_edges: refs.iter().map(|a| 2 * a.target().syntax().edges()).sum(),
        max_source_bytes: refs
            .iter()
            .map(|a| a.target().syntax().source().len())
            .sum(),
        max_name_bytes: refs.iter().map(|a| a.target().syntax().name_bytes()).sum(),
        max_validation_nodes: env.validation_nodes(),
        max_validation_edges: env.validation_edges(),
        max_work: env.validation_work(),
        ..GenericAliasEnvironmentLimits::DEFAULT
    };
    assert!(prepare_generic_aliases(&registry, &refs, exact).is_ok());
    for limits in [
        GenericAliasEnvironmentLimits {
            max_aliases: 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_validation_nodes: exact.max_validation_nodes - 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_validation_edges: exact.max_validation_edges - 1,
            ..exact
        },
        GenericAliasEnvironmentLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(
            prepare_generic_aliases(&registry, &refs, limits).is_err(),
            "{limits:?}"
        );
    }
    let syntax = ty("B<Int>");
    let scope = scope();
    let result = env
        .expand(&syntax, &scope, GenericAliasExpansionLimits::DEFAULT)
        .unwrap();
    let exact = GenericAliasExpansionLimits {
        max_nodes: result.nodes().len(),
        max_edges: result.edges(),
        max_work: result.work(),
        ..GenericAliasExpansionLimits::DEFAULT
    };
    assert!(env.expand(&syntax, &scope, exact).is_ok());
    for limits in [
        GenericAliasExpansionLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericAliasExpansionLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericAliasExpansionLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericAliasExpansionLimits {
            max_depth: 0,
            ..exact
        },
        GenericAliasExpansionLimits {
            max_depth: 129,
            ..exact
        },
    ] {
        assert!(env.expand(&syntax, &scope, limits).is_err());
    }
}
#[test]
fn substituted_structural_depth_cannot_hide_behind_a_shallow_alias_body() {
    let a = alias("type Add<T>=[[T]];");
    let registry = register_generic_types(
        &[GenericTypeDeclaration::Alias(&a)],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let env =
        prepare_generic_aliases(&registry, &[&a], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let source = format!("Add<{}Int{}>", "[".repeat(126), "]".repeat(126));
    let syntax = ty(&source);
    assert!(matches!(
        env.expand(&syntax, &scope(), GenericAliasExpansionLimits::DEFAULT),
        Err(GenericAliasExpansionError::Limit(
            "expanded structural depth"
        ))
    ));
    assert!(check_program_source(
        "strict",
        "type Add<T>=[[T]];fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn acyclic_alias_dependency_walk_still_obeys_its_finite_depth_ceiling() {
    let a = alias("type A<T>=B<T>;");
    let b = alias("type B<T>=C<T>;");
    let c = alias("type C<T>=D<T>;");
    let d = alias("type D<T>=T;");
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Alias(&a),
            GenericTypeDeclaration::Alias(&b),
            GenericTypeDeclaration::Alias(&c),
            GenericTypeDeclaration::Alias(&d),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    let refs = [&a, &b, &c, &d];
    assert!(
        prepare_generic_aliases(&registry, &refs, GenericAliasEnvironmentLimits::DEFAULT).is_ok()
    );
    assert!(matches!(
        prepare_generic_aliases(
            &registry,
            &refs,
            GenericAliasEnvironmentLimits {
                max_depth: 3,
                ..GenericAliasEnvironmentLimits::DEFAULT
            }
        ),
        Err(GenericAliasExpansionError::Limit("alias dependency depth"))
    ));
    assert!(prepare_generic_aliases(
        &registry,
        &refs,
        GenericAliasEnvironmentLimits {
            max_depth: 129,
            ..GenericAliasEnvironmentLimits::DEFAULT
        }
    )
    .is_err());
}
