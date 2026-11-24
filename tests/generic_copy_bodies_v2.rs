//! Symbolic generic copy/return/equality body checking, source ownership and budgets.
use sovra::compiler::v2::*;
fn signatures(sources: &[&str]) -> CheckedGenericSignatures {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax: Vec<_> = sources
        .iter()
        .map(|s| parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap())
        .collect();
    let refs: Vec<_> = syntax.iter().collect();
    check_generic_signatures(&types, &refs, GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(sources: &[&str]) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&signatures(sources), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn identity_and_inferred_local_copies_are_checked_symbolically_with_original_spans() {
    let source = "// λ😀\nfn identity<T>(value:T)->T{\n let copy=value;\n return copy;\n}";
    let checked = check(&[source]);
    let body = &checked.bodies()[0];
    assert_eq!(body.bindings().len(), 2);
    assert_eq!(body.expressions().len(), 2);
    assert!(!body.implicit_unit_return());
    assert!(matches!(
        body.bindings()[0].kind,
        GenericCopyBindingKind::Parameter(0)
    ));
    assert_eq!(body.bindings()[1].kind, GenericCopyBindingKind::Local);
    assert_eq!(
        &source[body.bindings()[1].name_span.start..body.bindings()[1].name_span.end],
        "copy"
    );
    assert_eq!(body.bindings()[1].name_span.line, 2);
    assert_eq!(body.expressions()[1].span.line, 3);
    assert_eq!(
        &source[body.expressions()[1].span.start..body.expressions()[1].span.end],
        "copy"
    );
    let ty = body.expressions()[1].ty.expanded().unwrap();
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id)if id==body.signature().syntax().parameters().find("T").unwrap())
    );
}
#[test]
fn inferred_shadowing_reads_the_previous_binding_before_declaring_its_replacement() {
    let checked =
        check(&["fn shadow<T>(value:T)->T{let value=value;let value=value;return value;}"]);
    let body = &checked.bodies()[0];
    assert_eq!(body.bindings().len(), 3);
    for (index, expression) in body.expressions().iter().enumerate() {
        assert!(
            matches!(expression.kind,GenericCopyExpressionKind::Read(id)if id==body.bindings()[index].id)
        );
    }
}
#[test]
fn structural_generic_equality_supports_arrays_and_opaque_values_but_distinguishes_parameters() {
    let checked = check(&[
        "fn same<T>(left:[T],right:[T])->Bool{return left==right;}",
        "fn different<T>(a:T,b:T)->Bool{return a!=b;}",
    ]);
    let equality = &checked.bodies()[0].expressions()[2];
    assert_eq!(equality.ty.primitive(), Some(GenericPrimitive::Bool));
    assert!(matches!(
        equality.kind,
        GenericCopyExpressionKind::Equality { negated: false, .. }
    ));
    assert!(matches!(
        checked.bodies()[1].expressions()[2].kind,
        GenericCopyExpressionKind::Equality { negated: true, .. }
    ));
    let bad = signatures(&["fn bad<T,U>(a:T,b:U)->Bool{return a==b;}"]);
    assert!(matches!(
        check_generic_copy_bodies(&bad, GenericCopyBodyLimits::DEFAULT),
        Err(GenericCopyBodyError::Semantic { .. })
    ));
}
#[test]
fn existing_concrete_numeric_widening_is_explicit_and_never_assumed_for_opaque_parameters() {
    let checked = check(&[
        "fn widen<T>()->Float{return 1;}",
        "fn compare<T>(n:Int,m:Float)->Bool{return n==m;}",
    ]);
    assert!(matches!(
        checked.bodies()[0].statements()[0],
        GenericCopyStatement::Return {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
    assert!(matches!(
        checked.bodies()[1].expressions()[2].kind,
        GenericCopyExpressionKind::Equality {
            left_conversion: GenericCopyConversion::IntToFloat,
            right_conversion: GenericCopyConversion::Identity,
            ..
        }
    ));
    for source in [
        "fn bad<T>()->T{return 1;}",
        "fn bad<T:std::Key>(x:T)->T{return x+x;}",
    ] {
        assert!(
            check_generic_copy_bodies(&signatures(&[source]), GenericCopyBodyLimits::DEFAULT)
                .is_err()
        );
    }
}
#[test]
fn unit_fallthrough_void_returns_and_missing_nonunit_returns_are_distinct() {
    let checked = check(&[
        "fn empty<T>(){}",
        "fn explicit<T>()->Unit{return;}",
        "fn fallthrough<T>()->Unit{}",
    ]);
    assert!(checked.bodies()[0].implicit_unit_return());
    assert!(!checked.bodies()[1].implicit_unit_return());
    assert!(checked.bodies()[2].implicit_unit_return());
    assert_eq!(checked.expression_count(), 0);
    let source = "fn missing<T>(value:T)->T{let copy=value;}";
    let GenericCopyBodyError::Semantic { span, reason, .. } =
        check_generic_copy_bodies(&signatures(&[source]), GenericCopyBodyLimits::DEFAULT)
            .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(&source[span.start..span.end], "}");
    assert!(reason.contains("does not return"));
    assert!(check_generic_copy_bodies(
        &signatures(&["fn wrong<T>(x:T)->T{return;}"]),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn unsupported_constructs_and_unreachable_errors_cannot_escape_as_checked_bodies() {
    for source in [
        "fn bad<T>(x:T)->T{if true{return missing;}return x;}",
        "fn bad<T>(x:T)->T{while false{missing;}return x;}",
        "fn bad<T>(x:T)->T{let mut y=x;y=1;return y;}",
        "fn bad<T>(x:T)->T{return missing(x);}",
        "fn bad<T>(x:T)->T{return [x];}",
        "fn bad<T>(x:T)->T{return x;let bad=missing;}",
        "fn bad<T>(x:T)->T{x=x;return x;}",
    ] {
        assert!(
            check_generic_copy_bodies(&signatures(&[source]), GenericCopyBodyLimits::DEFAULT)
                .is_err(),
            "{source}"
        );
    }
    let table = signatures(&[
        "fn good<T>(x:T)->T{return x;}",
        "fn unused<T>()->Int{return missing;}",
    ]);
    assert!(check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).is_err());
    let table = signatures(&["fn draft<T>(){let value=;}"]);
    assert!(matches!(
        check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT),
        Err(GenericCopyBodyError::Parse { .. })
    ));
}
#[test]
fn every_cumulative_admission_dimension_and_independent_comparison_limit_is_enforced() {
    let table = signatures(&[
        "fn same<T>(left:[T],right:[T])->Bool{return left==right;}",
        "fn identity<U>(x:U)->U{let copy=x;return copy;}",
    ]);
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_functions: 2,
        max_source_bytes: checked.source_bytes(),
        max_tokens: checked.tokens(),
        max_ast_nodes: checked.ast_nodes(),
        max_statements: checked.statement_count(),
        max_expressions: checked.expression_count(),
        max_bindings: checked.binding_count(),
        max_binding_text_bytes: checked.binding_text_bytes(),
        max_type_pairs: checked.type_pairs(),
        max_type_edges: checked.type_edges(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for limits in [
        GenericCopyBodyLimits {
            max_functions: 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_ast_nodes: exact.max_ast_nodes - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_statements: exact.max_statements - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_expressions: exact.max_expressions - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_bindings: exact.max_bindings - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_binding_text_bytes: exact.max_binding_text_bytes - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_type_pairs: exact.max_type_pairs - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_type_edges: exact.max_type_edges - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            comparison: GenericTypeEquivalenceLimits {
                max_pairs: 1,
                ..exact.comparison
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_copy_bodies(&table, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn body_binding_and_expression_ids_reject_foreign_functions_and_signature_tables() {
    let table = signatures(&[
        "fn first<T>(x:T)->T{return x;}",
        "fn second<T>(x:T)->T{return x;}",
    ]);
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let a = &checked.bodies()[0];
    let b = &checked.bodies()[1];
    assert!(b.binding(a.bindings()[0].id).is_none());
    assert!(b.expression(a.expressions()[0].id).is_none());
    assert!(a.binding(a.bindings()[0].id).is_some());
    let foreign = signatures(&["fn first<T>(x:T)->T{return x;}"]);
    assert!(checked.body(foreign.find("first").unwrap()).is_none());
    let cloned = a.clone();
    drop(table);
    drop(checked);
    assert_eq!(cloned.signature().syntax().name(), "first");
}
#[test]
fn literal_type_ranges_finiteness_and_original_unicode_source_are_checked() {
    let source = "// λ😀\nfn text<T>()->String{return \"hello λ😀\";}";
    let checked = check(&[source]);
    let e = &checked.bodies()[0].expressions()[0];
    assert_eq!(e.ty.primitive(), Some(GenericPrimitive::String));
    assert_eq!(e.span.line, 1);
    assert_eq!(&source[e.span.start..e.span.end], "\"hello λ😀\"");
    let huge = format!("fn large<T>()->Float{{return {}.0;}}", "9".repeat(400));
    for source in ["fn too_big<T>()->Int{return 9223372036854775808;}", &huge] {
        assert!(
            check_generic_copy_bodies(&signatures(&[source]), GenericCopyBodyLimits::DEFAULT)
                .is_err()
        );
    }
    assert!(check_program_source(
        "strict",
        "fn identity<T>(x:T)->T{return x;}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn nominal_and_alias_parameter_annotations_use_semantic_equivalence_in_body_equality() {
    let boxed =
        parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let echo = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let input = [
        GenericTypeDeclaration::Record(&boxed),
        GenericTypeDeclaration::Alias(&echo),
    ];
    let registry = register_generic_types(&input, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&echo], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    let syntax = parse_generic_function(
        "fn equal<T>(a:Box<T>,b:Echo<Box<T>>)->Bool{return a==b;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let table =
        check_generic_signatures(&types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
    assert!(check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).is_ok());
}

#[test]
fn repeated_checks_of_one_signature_cannot_mix_distinct_arena_ids() {
    let table = signatures(&["fn identity<T>(x:T)->T{let y=x;return y;}"]);
    let first = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let second = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let a = &first.bodies()[0];
    let b = &second.bodies()[0];
    assert_eq!(a.signature().id(), b.signature().id());
    assert_eq!(a.bindings()[0].id.ordinal(), b.bindings()[0].id.ordinal());
    assert_eq!(
        a.expressions()[0].id.ordinal(),
        b.expressions()[0].id.ordinal()
    );
    assert_ne!(a.bindings()[0].id, b.bindings()[0].id);
    assert_ne!(a.expressions()[0].id, b.expressions()[0].id);
    assert!(b.binding(a.bindings()[0].id).is_none());
    assert!(a.binding(b.bindings()[0].id).is_none());
    assert!(b.expression(a.expressions()[0].id).is_none());
    assert!(a.expression(b.expressions()[0].id).is_none());
    let clone = a.clone();
    drop(first);
    assert!(clone.expression(clone.expressions()[0].id).is_some());
    assert!(clone.binding(clone.bindings()[0].id).is_some());
}
