//! Complete generic type definition coverage, symbolic annotations and cumulative admission.
use sovra::compiler::v2::*;
fn record(s: &str) -> GenericRecordSyntax {
    parse_generic_record(s, GenericRecordLimits::DEFAULT).unwrap()
}
fn en(s: &str) -> GenericEnumSyntax {
    parse_generic_enum(s, GenericEnumLimits::DEFAULT).unwrap()
}
fn alias(s: &str) -> GenericAliasSyntax {
    parse_generic_alias(s, GenericAliasLimits::DEFAULT).unwrap()
}
fn environment(
    declarations: &[GenericTypeDeclaration<'_>],
    aliases: &[&GenericAliasSyntax],
) -> GenericAliasEnvironment {
    let registry = register_generic_types(declarations, GenericRegistryLimits::DEFAULT).unwrap();
    prepare_generic_aliases(&registry, aliases, GenericAliasEnvironmentLimits::DEFAULT).unwrap()
}
#[test]
fn complete_definitions_check_fields_payloads_and_aliases_in_registry_order() {
    let a = alias("type Echo<T>=T;");
    let r = record("struct Box<T>{value:Echo<T>}");
    let e = en("enum Outcome<T>{Value(Box<T>),UnitValue(Unit),End}");
    let input = [
        GenericTypeDeclaration::Alias(&a),
        GenericTypeDeclaration::Record(&r),
        GenericTypeDeclaration::Enum(&e),
    ];
    let env = environment(&input, &[&a]);
    let checked = check_generic_declarations(
        &env,
        &[input[2], input[0], input[1]],
        GenericDeclarationLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        checked.definitions()[0],
        CheckedGenericDefinition::Alias(_)
    ));
    let CheckedGenericDefinition::Record(boxed) = &checked.definitions()[1] else {
        panic!()
    };
    assert_eq!(boxed.field_types().len(), 1);
    let ty = &boxed.field_types()[0];
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id)if id==r.parameters().find("T").unwrap())
    );
    let CheckedGenericDefinition::Enum(outcome) = &checked.definitions()[2] else {
        panic!()
    };
    assert_eq!(outcome.payload_types().len(), 3);
    assert!(outcome.payload_types()[1].is_some());
    assert!(outcome.payload_types()[2].is_none());
    assert!(checked.definition(boxed.id()).is_some());
    let other = environment(&[GenericTypeDeclaration::Record(&r)], &[]);
    assert!(checked
        .definition(other.registry().headers()[0].id)
        .is_none());
}
#[test]
fn invalid_unused_record_or_enum_annotations_refuse_the_entire_collection() {
    for is_enum in [false, true] {
        let good = record("struct Good<T>{value:T}");
        let bad = record("struct Bad<T>{unused:Missing<T>}");
        let bad_enum = en("enum Bad<T>{Unused(Missing<T>),End}");
        let input = if is_enum {
            vec![
                GenericTypeDeclaration::Record(&good),
                GenericTypeDeclaration::Enum(&bad_enum),
            ]
        } else {
            vec![
                GenericTypeDeclaration::Record(&good),
                GenericTypeDeclaration::Record(&bad),
            ]
        };
        let env = environment(&input, &[]);
        assert!(matches!(
            check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT),
            Err(GenericDeclarationError::Annotation { member: 0, .. })
        ));
    }
}
#[test]
fn empty_raw_enum_refuses_semantic_check_but_recursive_nominal_annotations_do_not_unfold() {
    let empty = en("enum Empty<T>{}");
    let input = [GenericTypeDeclaration::Enum(&empty)];
    let env = environment(&input, &[]);
    assert!(matches!(
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT),
        Err(GenericDeclarationError::EmptyEnum { .. })
    ));
    let recursive = en("enum List<T>{Next(List<T>),Value(T),End}");
    let input = [GenericTypeDeclaration::Enum(&recursive)];
    let env = environment(&input, &[]);
    let checked =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    assert_eq!(checked.expanded_nodes(), 3);
}
#[test]
fn symbolic_key_constraints_are_checked_for_fields_and_expanded_alias_arguments() {
    let keys = record("struct Keys<K:std::Key>{key:K}");
    let echo = alias("type Echo<T>=T;");
    for (source, pass) in [
        ("struct Holder<T>{keys:Keys<T>}", false),
        ("struct Holder<K:std::Key>{keys:Keys<Echo<K>>}", true),
        ("struct Holder<T>{keys:Keys<Echo<Float>>}", false),
    ] {
        let holder = record(source);
        let input = [
            GenericTypeDeclaration::Record(&keys),
            GenericTypeDeclaration::Alias(&echo),
            GenericTypeDeclaration::Record(&holder),
        ];
        let env = environment(&input, &[&echo]);
        assert_eq!(
            check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).is_ok(),
            pass
        );
    }
}
#[test]
fn missing_duplicate_foreign_or_wrong_kind_definitions_cannot_masquerade_as_complete() {
    let r = record("struct Item<T>{value:T}");
    let input = [GenericTypeDeclaration::Record(&r)];
    let env = environment(&input, &[]);
    assert!(matches!(
        check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT),
        Err(GenericDeclarationError::Missing(_))
    ));
    assert!(check_generic_declarations(
        &env,
        &[input[0], input[0]],
        GenericDeclarationLimits::DEFAULT
    )
    .is_err());
    let foreign = record("struct Item<T>{value:T}");
    assert!(matches!(
        check_generic_declarations(
            &env,
            &[GenericTypeDeclaration::Record(&foreign)],
            GenericDeclarationLimits::DEFAULT
        ),
        Err(GenericDeclarationError::Ownership { .. })
    ));
    let wrong = en("enum Item<T>{Value(T)}");
    assert!(check_generic_declarations(
        &env,
        &[GenericTypeDeclaration::Enum(&wrong)],
        GenericDeclarationLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn exact_aggregate_budgets_admit_and_one_under_refuses_without_resetting_per_member() {
    let a = alias("type Echo<T>=T;");
    let r = record("struct Pair<T>{first:[T],second:T}");
    let e = en("enum E<T>{Value(Pair<T>),End}");
    let input = [
        GenericTypeDeclaration::Alias(&a),
        GenericTypeDeclaration::Record(&r),
        GenericTypeDeclaration::Enum(&e),
    ];
    let env = environment(&input, &[&a]);
    let checked =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    let metadata = 3 + 3 + 4 + a.target().syntax().nodes().len() + r.type_nodes() + e.type_nodes(); // declaration + parameters + members + type nodes
    let exact = GenericDeclarationLimits {
        max_declarations: 3,
        max_members: 4,
        max_metadata_nodes: metadata,
        max_source_bytes: a.retained_source_bytes()
            + r.retained_source_bytes()
            + e.retained_source_bytes(),
        max_name_bytes: a.name_bytes()
            + a.name().len()
            + r.name_bytes()
            + r.name().len()
            + e.name_bytes()
            + e.name().len(),
        max_expanded_nodes: checked.expanded_nodes(),
        max_expanded_edges: checked.expanded_edges(),
        max_work: checked.work(),
        ..GenericDeclarationLimits::DEFAULT
    };
    assert!(check_generic_declarations(&env, &input, exact).is_ok());
    for limits in [
        GenericDeclarationLimits {
            max_declarations: 2,
            ..exact
        },
        GenericDeclarationLimits {
            max_members: 3,
            ..exact
        },
        GenericDeclarationLimits {
            max_metadata_nodes: exact.max_metadata_nodes - 1,
            ..exact
        },
        GenericDeclarationLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericDeclarationLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericDeclarationLimits {
            max_expanded_nodes: exact.max_expanded_nodes - 1,
            ..exact
        },
        GenericDeclarationLimits {
            max_expanded_edges: exact.max_expanded_edges - 1,
            ..exact
        },
        GenericDeclarationLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericDeclarationLimits {
            expansion: GenericAliasExpansionLimits {
                max_nodes: 1,
                ..exact.expansion
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_declarations(&env, &input, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn member_error_keeps_original_annotation_range_and_inner_alias_provenance() {
    let a = alias("type Fixed<T>=Float;");
    let keys = record("struct Keys<K:std::Key>{key:K}");
    let bad = record("// λ😀\nstruct Bad<T>{\n value:Keys<Fixed<T>>\n}");
    let input = [
        GenericTypeDeclaration::Alias(&a),
        GenericTypeDeclaration::Record(&keys),
        GenericTypeDeclaration::Record(&bad),
    ];
    let env = environment(&input, &[&a]);
    let GenericDeclarationError::Annotation {
        declaration,
        member,
        annotation_range,
        error,
    } = check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    let GenericAliasExpansionError::Bound(origin) = *error else {
        panic!()
    };
    assert_eq!(
        env.registry().header(declaration).unwrap().name.as_ref(),
        "Bad"
    );
    assert_eq!(member, 0);
    assert_eq!(
        &bad.source()[annotation_range.start..annotation_range.end],
        "Keys<Fixed<T>>"
    );
    assert_eq!(annotation_range.line, 2);
    assert!(origin.alias.is_some());
    assert_eq!(env.alias_source(origin.alias.unwrap()).unwrap(), "Float");
}
#[test]
fn checked_definitions_retain_owners_and_do_not_enable_generic_program_execution() {
    let r = record("struct R<T>{value:T}");
    let input = [GenericTypeDeclaration::Record(&r)];
    let env = environment(&input, &[]);
    let checked =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    drop(env);
    drop(r);
    let CheckedGenericDefinition::Record(proof) = &checked.definitions()[0] else {
        panic!()
    };
    assert_eq!(proof.syntax().name(), "R");
    assert_eq!(proof.field_types().len(), 1);
    assert!(proof.environment().registry().header(proof.id()).is_some());
    assert!(check_program_source(
        "strict",
        "struct R<T>{value:T}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn caller_type_parameter_spelling_cannot_capture_a_nominal_head_in_an_alias_target() {
    let nominal = record("struct R<T>{value:T}");
    let make = alias("type Make<X>=R<X>;");
    let holder = record("struct Holder<R>{value:Make<R>}");
    let input = [
        GenericTypeDeclaration::Record(&nominal),
        GenericTypeDeclaration::Alias(&make),
        GenericTypeDeclaration::Record(&holder),
    ];
    let env = environment(&input, &[&make]);
    let checked =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    let CheckedGenericDefinition::Record(proof) = &checked.definitions()[2] else {
        panic!()
    };
    let ty = &proof.field_types()[0];
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &ty.nodes()[ty.root()].kind
    else {
        panic!()
    };
    assert_eq!(*declaration, env.registry().headers()[0].id);
    assert!(
        matches!(ty.nodes()[arguments[0]].kind,ResolvedGenericTypeKind::Parameter(id)if id==holder.parameters().find("R").unwrap())
    );
}
