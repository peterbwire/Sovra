//! Exhaustive typed generic enum pattern headers, not arm typing or source-site scope.
use sovra::compiler::{lexer::Lexer, v2::*};
fn table(source: &str) -> CheckedGenericSignatures {
    let enums: Vec<_> = [
        "enum Choice<T>{End,One(T),Many(Rows<T>),Unit(Unit)}",
        "enum Other<T>{One(T)}",
        "enum Empty<T>{End}",
    ]
    .iter()
    .map(|s| parse_generic_enum(s, GenericEnumLimits::DEFAULT).unwrap())
    .collect();
    let alias = parse_generic_alias("type Rows<T>=[T];", GenericAliasLimits::DEFAULT).unwrap();
    let mut declarations: Vec<_> = enums.iter().map(GenericTypeDeclaration::Enum).collect();
    declarations.push(GenericTypeDeclaration::Alias(&alias));
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let function = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&function], GenericSignatureLimits::DEFAULT).unwrap()
}
fn bodies() -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(
        &table("fn caller<U>(value:Choice<U>){}"),
        GenericCopyBodyLimits::DEFAULT,
    )
    .unwrap()
}
const MATCH:&str="match value {Choice::End()=>{},Choice::One(x)=>{},Choice::Many(xs)=>{},Choice::Unit(unit)=>{}}";
#[test]
fn exhaustive_headers_keep_nominal_identity_order_and_exact_caller_payload_types() {
    let bodies = bodies();
    let body = &bodies.bodies()[0];
    let checked = check_generic_enum_patterns(
        MATCH,
        body,
        body.bindings()[0].id,
        GenericEnumPatternLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        checked
            .arms()
            .iter()
            .map(|a| a.variant_index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert!(checked.arms()[0].payload.is_none());
    let payload = &checked.arms()[1].payload.as_ref().unwrap().ty;
    assert!(
        matches!(payload.nodes()[payload.root()].kind,ResolvedGenericTypeKind::Parameter(id) if id==body.signature().syntax().parameters().find("U").unwrap())
    );
    assert_eq!(checked.scrutinee().id, body.bindings()[0].id);
    let GenericTypeKind::Named { .. } = payload.syntax().root().kind else {
        panic!()
    };
}
#[test]
fn missing_duplicate_foreign_unknown_and_wrong_binding_shapes_never_publish_coverage() {
    let bodies = bodies();
    let body = &bodies.bodies()[0];
    let id = body.bindings()[0].id;
    assert!(matches!(
        check_generic_enum_patterns(
            "match value {Choice::End()=>{}}",
            body,
            id,
            GenericEnumPatternLimits::DEFAULT
        ),
        Err(GenericEnumPatternError::MissingVariants { count: 3, .. })
    ));
    assert!(matches!(
        check_generic_enum_patterns(
            "match value {Choice::End()=>{},Choice::End()=>{}}",
            body,
            id,
            GenericEnumPatternLimits::DEFAULT
        ),
        Err(GenericEnumPatternError::DuplicateVariant { .. })
    ));
    assert!(matches!(
        check_generic_enum_patterns(
            "match value {Other::One(x)=>{}}",
            body,
            id,
            GenericEnumPatternLimits::DEFAULT
        ),
        Err(GenericEnumPatternError::ForeignEnum { .. })
    ));
    assert!(matches!(
        check_generic_enum_patterns(
            "match value {Choice::Missing()=>{}}",
            body,
            id,
            GenericEnumPatternLimits::DEFAULT
        ),
        Err(GenericEnumPatternError::UnknownVariant { .. })
    ));
    for source in [
        "match value {Choice::End(x)=>{}}",
        "match value {Choice::One()=>{}}",
        "match value {Choice::Unit()=>{}}",
    ] {
        assert!(matches!(
            check_generic_enum_patterns(source, body, id, GenericEnumPatternLimits::DEFAULT),
            Err(GenericEnumPatternError::PayloadArity { .. })
        ));
    }
}
#[test]
fn nullary_and_unit_contexts_shadow_names_without_allocating_fake_binding_ids() {
    let bodies = bodies();
    let body = &bodies.bodies()[0];
    let source = MATCH.replace("One(x)", "One(value)");
    let checked = check_generic_enum_patterns(
        &source,
        body,
        body.bindings()[0].id,
        GenericEnumPatternLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        checked.arms()[1].payload.as_ref().unwrap().name.as_ref(),
        "value"
    );
    assert_eq!(checked.scrutinee().name.as_ref(), "value");
    let unit = &checked.arms()[3].payload.as_ref().unwrap().ty;
    assert!(matches!(
        unit.nodes()[unit.root()].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Unit)
    ));
    assert!(checked.arms()[0].payload.is_none());
    let empty = check_generic_copy_bodies(
        &table("fn caller<U>(value:Empty<U>){}"),
        GenericCopyBodyLimits::DEFAULT,
    )
    .unwrap();
    let body = &empty.bodies()[0];
    assert!(check_generic_enum_patterns(
        "match value {Empty::End()=>{}}",
        body,
        body.bindings()[0].id,
        GenericEnumPatternLimits {
            max_nodes: 0,
            max_edges: 0,
            ..GenericEnumPatternLimits::DEFAULT
        }
    )
    .is_ok());
}
#[test]
fn selection_checks_exact_arena_name_type_and_original_unicode_spans() {
    let signatures = table("fn caller<U>(value:Choice<U>){}");
    let a = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let b = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let id = a.bodies()[0].bindings()[0].id;
    assert_eq!(
        check_generic_enum_patterns(MATCH, &b.bodies()[0], id, GenericEnumPatternLimits::DEFAULT)
            .unwrap_err(),
        GenericEnumPatternError::ScrutineeBinding
    );
    let source = "// λ😀\nmatch other {Choice::End()=>{}}";
    let GenericEnumPatternError::ScrutineeName { span } = check_generic_enum_patterns(
        source,
        &a.bodies()[0],
        id,
        GenericEnumPatternLimits::DEFAULT,
    )
    .unwrap_err() else {
        panic!()
    };
    assert_eq!(&source[span.start..span.end], "other");
    assert_eq!(span.line, 1);
    let scalar = check_generic_copy_bodies(
        &table("fn caller<U>(value:Int){}"),
        GenericCopyBodyLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        check_generic_enum_patterns(
            MATCH,
            &scalar.bodies()[0],
            scalar.bodies()[0].bindings()[0].id,
            GenericEnumPatternLimits::DEFAULT
        )
        .unwrap_err(),
        GenericEnumPatternError::ScrutineeType
    );
}
#[test]
fn alias_payload_sources_survive_and_arm_bodies_are_deliberately_untyped() {
    let bodies = bodies();
    let body = &bodies.bodies()[0];
    let source = MATCH.replace("One(x)=>{}", "One(x)=>{missing;}");
    let checked = check_generic_enum_patterns(
        &source,
        body,
        body.bindings()[0].id,
        GenericEnumPatternLimits::DEFAULT,
    )
    .unwrap();
    let ty = &checked.arms()[2].payload.as_ref().unwrap().ty;
    let node = &ty.nodes()[ty.root()];
    assert!(node.origin.alias.is_some());
    assert_eq!(
        &ty.node_source(ty.root()).unwrap()[node.origin.span.start..node.origin.span.end],
        "[T]"
    );
    let clone = checked.clone();
    drop(checked);
    drop(bodies);
    assert!(clone.source().contains("missing;"));
    assert_eq!(clone.scrutinee().name.as_ref(), "value");
}
#[test]
fn exact_and_one_under_syntax_context_and_work_budgets_are_enforced() {
    let bodies = bodies();
    let body = &bodies.bodies()[0];
    let id = body.bindings()[0].id;
    let checked =
        check_generic_enum_patterns(MATCH, body, id, GenericEnumPatternLimits::DEFAULT).unwrap();
    let raw = parse_match_statement(MATCH, MatchSyntaxLimits::DEFAULT).unwrap();
    let header_bytes = raw.scrutinee.len()
        + raw
            .arms
            .iter()
            .map(|a| {
                a.enum_name.len() + a.variant_name.len() + a.binding.as_ref().map_or(0, String::len)
            })
            .sum::<usize>();
    let exact = GenericEnumPatternLimits {
        max_nodes: checked.nodes(),
        max_edges: checked.edges(),
        max_work: checked.work(),
        syntax: MatchSyntaxLimits {
            frontend: sovra::compiler::resources::FrontendLimits {
                max_source_bytes: MATCH.len(),
                max_tokens: Lexer::new().tokenize(MATCH).unwrap().len() - 1,
                max_diagnostics: 64,
            },
            max_arms: 4,
            max_header_text_bytes: header_bytes,
        },
        ..GenericEnumPatternLimits::DEFAULT
    };
    assert!(check_generic_enum_patterns(MATCH, body, id, exact).is_ok());
    for limits in [
        GenericEnumPatternLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericEnumPatternLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericEnumPatternLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericEnumPatternLimits {
            syntax: MatchSyntaxLimits {
                max_arms: 3,
                ..exact.syntax
            },
            ..exact
        },
        GenericEnumPatternLimits {
            syntax: MatchSyntaxLimits {
                max_header_text_bytes: header_bytes - 1,
                ..exact.syntax
            },
            ..exact
        },
        GenericEnumPatternLimits {
            syntax: MatchSyntaxLimits {
                frontend: sovra::compiler::resources::FrontendLimits {
                    max_source_bytes: MATCH.len() - 1,
                    ..exact.syntax.frontend
                },
                ..exact.syntax
            },
            ..exact
        },
        GenericEnumPatternLimits {
            syntax: MatchSyntaxLimits {
                frontend: sovra::compiler::resources::FrontendLimits {
                    max_tokens: exact.syntax.frontend.max_tokens - 1,
                    ..exact.syntax.frontend
                },
                ..exact.syntax
            },
            ..exact
        },
        GenericEnumPatternLimits {
            payload: GenericFieldSubstitutionLimits {
                max_work: 0,
                ..exact.payload
            },
            ..exact
        },
        GenericEnumPatternLimits {
            payload: GenericFieldSubstitutionLimits {
                max_depth: 1,
                ..exact.payload
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_enum_patterns(MATCH, body, id, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn wildcard_guard_tuple_qualified_and_expression_scrutinee_profiles_are_refused() {
    let bodies = bodies();
    let body = &bodies.bodies()[0];
    let id = body.bindings()[0].id;
    for source in [
        "match value {_=>{}}",
        "match value {Choice::One(_)=>{}}",
        "match value {Choice::One(x,y)=>{}}",
        "match value {Choice::One(x) if true=>{}}",
        "match obtain() {Choice::End()=>{}}",
        "match value {module::Choice::End()=>{}}",
    ] {
        assert!(
            check_generic_enum_patterns(source, body, id, GenericEnumPatternLimits::DEFAULT)
                .is_err()
        );
    }
}
