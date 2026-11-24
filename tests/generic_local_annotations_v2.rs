//! Explicit local type annotations with original-token parsing and cumulative admission.
use sovra::compiler::{
    ast::{ExpressionKind, Statement},
    parser::Parser,
    v2::*,
};
fn table(source: &str) -> CheckedGenericSignatures {
    let pair =
        parse_generic_record("struct Pair<A,B>{a:A,b:B}", GenericRecordLimits::DEFAULT).unwrap();
    let boxed =
        parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let keys = parse_generic_record(
        "struct Keys<K:std::Key>{key:K}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let echo = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let fixed = parse_generic_alias("type Fixed<T>=Float;", GenericAliasLimits::DEFAULT).unwrap();
    let input = [
        GenericTypeDeclaration::Record(&pair),
        GenericTypeDeclaration::Record(&boxed),
        GenericTypeDeclaration::Record(&keys),
        GenericTypeDeclaration::Alias(&echo),
        GenericTypeDeclaration::Alias(&fixed),
    ];
    let registry = register_generic_types(&input, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(
        &registry,
        &[&echo, &fixed],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap();
    let types =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    let f = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&f], GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(source: &str) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap()
}
#[test]
fn generic_local_types_keep_exact_original_fragments_and_function_parameter_identity() {
    let source="// λ😀\nfn copy<T>(value:Box<T>)->Box<T>{\n let local:Echo<Box<T>>=value;\n return local;\n}";
    let checked = check(source);
    let body = &checked.bodies()[0];
    let GenericCopyStatement::Let {
        annotation: Some(annotation),
        conversion: GenericCopyConversion::Identity,
        ..
    } = &body.statements()[0]
    else {
        panic!()
    };
    assert_eq!(
        &source[annotation.range.start..annotation.range.end],
        "Echo<Box<T>>"
    );
    assert_eq!(annotation.range.line, 2);
    assert_eq!(annotation.ty.syntax().source(), "Echo<Box<T>>");
    assert!(matches!(
        annotation.ty.nodes()[annotation.ty.root()].kind,
        ResolvedGenericTypeKind::Declared { .. }
    ));
    assert_eq!(checked.local_annotation_usage().annotations, 1);
    let bound = body.bindings()[1].ty.expanded().unwrap();
    assert!(
        compare_generic_types(bound, &annotation.ty, GenericTypeEquivalenceLimits::DEFAULT)
            .unwrap()
            .equivalent()
    );
}
#[test]
fn typed_local_widening_changes_binding_type_without_changing_initializer_type() {
    let checked = check("fn widen<T>()->Float{let value:Float=1;return value;}");
    let body = &checked.bodies()[0];
    assert!(matches!(
        body.statements()[0],
        GenericCopyStatement::Let {
            conversion: GenericCopyConversion::IntToFloat,
            ..
        }
    ));
    assert_eq!(
        body.expressions()[0].ty.primitive(),
        Some(GenericPrimitive::Int)
    );
    assert_eq!(
        body.bindings()[0].ty.primitive(),
        Some(GenericPrimitive::Float)
    );
    assert!(matches!(
        body.statements()[1],
        GenericCopyStatement::Return {
            conversion: GenericCopyConversion::Identity,
            ..
        }
    ));
}
#[test]
fn typed_shadowing_checks_initializer_before_new_binding_and_does_not_infer_opaque_types() {
    let checked = check("fn shadow<T>(value:T)->T{let value:T=value;return value;}");
    let body = &checked.bodies()[0];
    assert!(
        matches!(body.expressions()[0].kind,GenericCopyExpressionKind::Read(id)if id==body.bindings()[0].id)
    );
    assert!(
        matches!(body.expressions()[1].kind,GenericCopyExpressionKind::Read(id)if id==body.bindings()[1].id)
    );
    for source in [
        "fn bad<T>(value:T)->T{let copy:Int=value;return value;}",
        "fn bad<T>(value:T)->T{let copy:T=1;return copy;}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err());
    }
}
#[test]
fn annotation_names_arities_and_key_constraints_are_checked_with_preserved_provenance() {
    for source in [
        "fn bad<T>(value:T)->T{let copy:Missing<T>=value;return value;}",
        "fn bad<T>(value:T)->T{let copy:Int<T>=value;return value;}",
        "fn bad<T>(value:T)->T{let copy:Keys<T>=value;return value;}",
    ] {
        assert!(matches!(
            check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT),
            Err(GenericCopyBodyError::Annotation { .. })
        ));
    }
    assert!(check_generic_copy_bodies(
        &table("fn key<K:std::Key>(value:Keys<K>)->Keys<K>{let copy:Keys<K>=value;return copy;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_ok());
    let source = "// λ😀\nfn bad<T>(value:T)->T{let copy:Keys<Fixed<T>>=value;return value;}";
    let GenericCopyBodyError::Annotation {
        annotation_range,
        error,
        ..
    } = check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(
        &source[annotation_range.start..annotation_range.end],
        "Keys<Fixed<T>>"
    );
    assert!(matches!(
        *error,
        GenericLocalAnnotationError::Expansion(GenericAliasExpansionError::Bound(
            GenericExpansionOrigin { alias: Some(_), .. }
        ))
    ));
}
#[test]
fn generic_annotation_adapter_preserves_legacy_strictness_and_comparison_operators() {
    assert!(Parser::new()
        .parse_source("fn main(){let copy:Box<Int>=value;}")
        .is_err());
    let legacy = Parser::new()
        .parse_source("fn main(){let result:Bool=1>=2;}")
        .unwrap();
    let Statement::Let { value, .. } = &legacy.functions[0].body[0] else {
        panic!()
    };
    assert!(matches!(&value.kind,ExpressionKind::Binary{operator,..}if operator==">="));
    for source in [
        "fn copy<T>(x:Box<T>)->Box<T>{let y:Box<T>=x;return y;}",
        "fn copy<T>(x:Box<T>)->Box<T>{let y:Box<T> = x;return y;}",
        "fn copy<T>(x:Box<T>)->Box<T>{let y:Box< // type\n T> = x;return y;}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_ok());
    }
    for source in [
        "fn bad<T>(x:T)->T{let y:Box<>=x;return x;}",
        "fn bad<T>(x:T)->T{let y:Box<Box<T>=x;return x;}",
        "fn bad<T>(x:T)->T{let y:[Box<T>=]x;return x;}",
        "fn bad<T>(x:T)->T{let y:Pair<T,>=x;return x;}",
    ] {
        assert!(check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT).is_err());
    }
}
#[test]
fn every_local_annotation_ledger_is_exact_and_cumulative_with_child_caps() {
    let table=table("fn copy<T>(x:Pair<[T],T>)->Pair<[T],T>{let first:Pair<[T],T>=x;let second:Echo<Pair<[T],T>>=first;return second;}");
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let u = checked.local_annotation_usage();
    let exact = GenericLocalAnnotationLimits {
        max_annotations: u.annotations,
        max_source_bytes: u.source_bytes,
        max_tokens: u.tokens,
        max_nodes: u.nodes,
        max_edges: u.edges,
        max_name_bytes: u.name_bytes,
        max_expanded_nodes: u.expanded_nodes,
        max_expanded_edges: u.expanded_edges,
        ..GenericLocalAnnotationLimits::DEFAULT
    };
    let profile = GenericCopyBodyLimits {
        local_types: exact,
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, profile).is_ok());
    for local in [
        GenericLocalAnnotationLimits {
            max_annotations: exact.max_annotations - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_expanded_nodes: exact.max_expanded_nodes - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            max_expanded_edges: exact.max_expanded_edges - 1,
            ..exact
        },
        GenericLocalAnnotationLimits {
            syntax: GenericTypeLimits {
                max_nodes: 1,
                ..exact.syntax
            },
            ..exact
        },
        GenericLocalAnnotationLimits {
            expansion: GenericAliasExpansionLimits {
                max_nodes: 1,
                ..exact.expansion
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_copy_bodies(
                &table,
                GenericCopyBodyLimits {
                    local_types: local,
                    ..profile
                }
            )
            .is_err(),
            "{local:?}"
        );
    }
    let work = checked.work();
    assert!(check_generic_copy_bodies(
        &table,
        GenericCopyBodyLimits {
            max_work: work,
            ..profile
        }
    )
    .is_ok());
    assert!(check_generic_copy_bodies(
        &table,
        GenericCopyBodyLimits {
            max_work: work - 1,
            ..profile
        }
    )
    .is_err());
}
#[test]
fn local_annotation_budgets_do_not_reset_between_functions_and_mutable_annotations_check() {
    let source = "fn copy<T>(x:T)->T{let y:T=x;return y;}";
    let first = table(source);
    let types = first.types();
    let a = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    let b = parse_generic_function(
        "fn second<T>(x:T)->T{let y:T=x;return y;}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let sig = check_generic_signatures(types, &[&a, &b], GenericSignatureLimits::DEFAULT).unwrap();
    assert!(check_generic_copy_bodies(
        &sig,
        GenericCopyBodyLimits {
            local_types: GenericLocalAnnotationLimits {
                max_annotations: 1,
                ..GenericLocalAnnotationLimits::DEFAULT
            },
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    assert!(check_generic_copy_bodies(
        &table("fn copy<T>(x:T)->T{let mut y:T=x;y=x;return y;}"),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_ok());
    assert!(check_program_source("strict", source, ProgramSourceLimits::DEFAULT).is_err());
}
