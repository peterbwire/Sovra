//! Finite reachable concrete call closure, distinct from executable specialization.
use sovra::compiler::v2::*;
fn bodies(sources: &[&str]) -> CheckedGenericCopyBodies {
    let alias = parse_generic_alias("type Same<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let left =
        parse_generic_record("struct Left<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let right =
        parse_generic_record("struct Right<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let declarations = [
        GenericTypeDeclaration::Alias(&alias),
        GenericTypeDeclaration::Record(&left),
        GenericTypeDeclaration::Record(&right),
    ];
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let functions: Vec<_> = sources
        .iter()
        .map(|s| {
            if s.starts_with("fn root(") {
                parse_specialization_root(s, GenericFunctionLimits::DEFAULT).unwrap()
            } else {
                parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap()
            }
        })
        .collect();
    let signatures = check_generic_signatures(
        &types,
        &functions.iter().collect::<Vec<_>>(),
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap()
}
fn discover(bodies: &CheckedGenericCopyBodies) -> CheckedGenericSpecializations {
    discover_generic_specializations(
        bodies,
        bodies.signatures().find("root").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap()
}
const ID: &str = "fn id<T>(x:T)->T{return x;}";
#[test]
fn concrete_transitive_calls_deduplicate_aliases_and_preserve_exact_source_edges() {
    let bodies = bodies(&[
        "fn root(){id::<Int>(1); id::<Same<Int>>(2); outer::<String>(\"hi\");}",
        ID,
        "fn outer<U>(x:U)->U{return id::<U>(x);}",
    ]);
    let graph = discover(&bodies);
    let instances = graph.instances();
    assert_eq!(instances.len(), 4);
    assert_eq!(
        instances[0]
            .calls()
            .iter()
            .map(|c| c.target)
            .collect::<Vec<_>>(),
        [1, 1, 2]
    );
    assert_eq!(instances[2].calls()[0].target, 3);
    assert_eq!(instances[1].function(), instances[3].function());
    assert!(matches!(
        instances[1].arguments()[0].nodes()[instances[1].arguments()[0].root()].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Int)
    ));
    assert!(matches!(
        instances[3].arguments()[0].nodes()[instances[3].arguments()[0].root()].kind,
        ResolvedGenericTypeKind::Primitive(GenericPrimitive::String)
    ));
    for instance in instances {
        let body = graph.bodies().body(instance.function()).unwrap();
        for call in instance.calls() {
            assert!(matches!(
                body.expression(call.expression).unwrap().kind,
                GenericCopyExpressionKind::Call { .. }
            ));
            assert!(call.target < instances.len());
        }
        assert!(instance
            .arguments()
            .iter()
            .flat_map(|ty| ty.nodes())
            .all(|n| !matches!(n.kind, ResolvedGenericTypeKind::Parameter(_))));
    }
    let cloned = graph.clone();
    drop(graph);
    drop(bodies);
    assert_eq!(cloned.instances().len(), 4);
}
#[test]
fn identical_direct_and_mutual_recursion_reuse_instances_before_traversal() {
    let direct = bodies(&[
        "fn root(){recur::<Int>(1);}",
        "fn recur<T>(x:T)->T{return recur::<T>(x);}",
    ]);
    let graph = discover(&direct);
    assert_eq!(graph.instances().len(), 2);
    assert_eq!(graph.instances()[1].calls()[0].target, 1);
    let mutual = bodies(&[
        "fn root(){first::<Int>(1);}",
        "fn first<T>(x:T)->T{return second::<T>(x);}",
        "fn second<U>(x:U)->U{return first::<U>(x);}",
    ]);
    let graph = discover(&mutual);
    assert_eq!(graph.instances().len(), 3);
    assert_eq!(graph.instances()[2].calls()[0].target, 1);
}
#[test]
fn expanding_recursion_refuses_on_instances_and_depth_without_fallback() {
    let bodies = bodies(&["fn root(){grow::<Int>();}", "fn grow<T>(){grow::<[T]>();}"]);
    let root = bodies.signatures().find("root").unwrap();
    assert!(matches!(
        discover_generic_specializations(
            &bodies,
            root,
            GenericSpecializationLimits {
                max_instances: 4,
                ..GenericSpecializationLimits::DEFAULT
            }
        ),
        Err(GenericSpecializationError::Limit(
            "specialization instances"
        ))
    ));
    assert!(discover_generic_specializations(
        &bodies,
        root,
        GenericSpecializationLimits {
            max_depth: 3,
            ..GenericSpecializationLimits::DEFAULT
        }
    )
    .is_err());
}
#[test]
fn nominal_heads_and_argument_order_are_significant_in_cache_identity() {
    let bodies=bodies(&["fn root(){sink::<Left<Int>,Bool>();sink::<Right<Int>,Bool>();sink::<Bool,Left<Int>>();sink::<Left<Int>,Bool>();}","fn sink<A,B>(){}"]);
    let graph = discover(&bodies);
    assert_eq!(graph.instances().len(), 4);
    assert_eq!(
        graph.instances()[0]
            .calls()
            .iter()
            .map(|c| c.target)
            .collect::<Vec<_>>(),
        [1, 2, 3, 1]
    );
}
#[test]
fn roots_require_exact_table_ownership_and_no_opaque_type_parameters() {
    let bodies = bodies(&["fn root(){}", ID]);
    let foreign = super_table_root();
    assert!(matches!(
        discover_generic_specializations(&bodies, foreign, GenericSpecializationLimits::DEFAULT),
        Err(GenericSpecializationError::Root)
    ));
    assert!(matches!(
        discover_generic_specializations(
            &bodies,
            bodies.signatures().find("id").unwrap(),
            GenericSpecializationLimits::DEFAULT
        ),
        Err(GenericSpecializationError::Root)
    ));
    for depth in [0, 129] {
        assert!(discover_generic_specializations(
            &bodies,
            bodies.signatures().find("root").unwrap(),
            GenericSpecializationLimits {
                max_depth: depth,
                ..GenericSpecializationLimits::DEFAULT
            }
        )
        .is_err());
    }
    assert_eq!(GenericSpecializationLimits::DEFAULT.max_instances, 1024);
    assert_eq!(GenericSpecializationLimits::PACKAGE.max_instances, 4096);
}
fn super_table_root() -> GenericSignatureId {
    bodies(&["fn root(){}"]).signatures().find("root").unwrap()
}
#[test]
fn unreachable_source_calls_are_included_but_uncalled_generic_bodies_are_not_instantiated() {
    let bodies = bodies(&[
        "fn root(){if false {id::<Int>(1);}return;id::<String>(\"unreachable\");}",
        ID,
        "fn unused<T>(){unused::<[T]>();}",
    ]);
    let graph = discover(&bodies);
    assert_eq!(graph.instances().len(), 3);
    assert_eq!(graph.instances()[0].calls().len(), 2);
    assert!(graph
        .instances()
        .iter()
        .all(|i| i.function() != bodies.signatures().find("unused").unwrap()));
}
#[test]
fn cumulative_and_per_argument_budgets_are_exact_and_charge_duplicate_candidates() {
    let bodies = bodies(&["fn root(){id::<[Int]>([1]);id::<Same<[Int]>>([2]);}", ID]);
    let graph = discover(&bodies);
    let usage = graph.usage();
    assert_eq!(usage.instances, 2);
    assert_eq!(usage.arguments, 2);
    assert_eq!(usage.calls, 2);
    let exact = GenericSpecializationLimits {
        max_instances: usage.instances,
        max_calls: usage.calls,
        max_arguments: usage.arguments,
        max_nodes: usage.nodes,
        max_edges: usage.edges,
        max_key_nodes: usage.key_nodes,
        max_key_edges: usage.key_edges,
        max_work: usage.work,
        ..GenericSpecializationLimits::DEFAULT
    };
    let root = bodies.signatures().find("root").unwrap();
    assert_eq!(
        discover_generic_specializations(&bodies, root, exact)
            .unwrap()
            .usage(),
        usage
    );
    for limits in [
        GenericSpecializationLimits {
            max_instances: exact.max_instances - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_calls: exact.max_calls - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_arguments: exact.max_arguments - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_key_nodes: exact.max_key_nodes - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_key_edges: exact.max_key_edges - 1,
            ..exact
        },
        GenericSpecializationLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericSpecializationLimits {
            expansion: GenericAliasExpansionLimits {
                max_nodes: 0,
                ..exact.expansion
            },
            ..exact
        },
        GenericSpecializationLimits {
            keys: GenericTypeKeyLimits {
                max_work: 0,
                ..exact.keys
            },
            ..exact
        },
    ] {
        assert!(
            discover_generic_specializations(&bodies, root, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn empty_root_still_consumes_instance_and_work_budgets_and_is_deterministic() {
    let bodies = bodies(&["fn root(){}"]);
    let a = discover(&bodies);
    let b = discover(&bodies);
    assert_eq!(a.usage(), b.usage());
    assert_eq!(a.instances().len(), 1);
    let root = bodies.signatures().find("root").unwrap();
    for limits in [
        GenericSpecializationLimits {
            max_instances: 0,
            ..GenericSpecializationLimits::DEFAULT
        },
        GenericSpecializationLimits {
            max_work: 0,
            ..GenericSpecializationLimits::DEFAULT
        },
    ] {
        assert!(discover_generic_specializations(&bodies, root, limits).is_err());
    }
}

#[test]
fn nongeneric_root_adapter_preserves_ranges_and_exact_header_admission() {
    let source = "// λ\nexport fn host(value:[Int])->[Int]{return value;}";
    let root = parse_specialization_root(source, GenericFunctionLimits::DEFAULT).unwrap();
    assert!(root.parameters().parameters().is_empty());
    assert!(root.parameter_range().start == root.parameter_range().end);
    assert_eq!(
        &source[root.parameter_range().start..root.parameter_range().start + 1],
        "("
    );
    assert_eq!(
        &source[root.value_parameters()[0].ty.type_range.start
            ..root.value_parameters()[0].ty.type_range.end],
        "[Int]"
    );
    assert!(root.is_exported());
    let exact = GenericFunctionLimits {
        max_source_bytes: source.len(),
        max_tokens: sovra::compiler::lexer::Lexer::new()
            .tokenize(source)
            .unwrap()
            .len(),
        max_retained_source_bytes: root.retained_source_bytes(),
        max_name_bytes: root.name_bytes(),
        max_value_parameters: 1,
        max_type_nodes: root.type_nodes(),
        max_type_edges: root.type_edges(),
        max_parameter_uses: 0,
        max_body_tokens: root.body_tokens(),
        max_body_depth: 1,
        ..GenericFunctionLimits::DEFAULT
    };
    assert!(parse_specialization_root(source, exact).is_ok());
    for limits in [
        GenericFunctionLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_retained_source_bytes: exact.max_retained_source_bytes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_value_parameters: 0,
            ..exact
        },
        GenericFunctionLimits {
            max_type_nodes: exact.max_type_nodes - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_type_edges: exact.max_type_edges - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_body_tokens: exact.max_body_tokens - 1,
            ..exact
        },
        GenericFunctionLimits {
            max_body_depth: 0,
            ..exact
        },
    ] {
        assert!(
            parse_specialization_root(source, limits).is_err(),
            "{limits:?}"
        );
    }
    assert!(parse_generic_function(source, GenericFunctionLimits::DEFAULT).is_err());
    assert!(parse_specialization_root("fn host<T>(){}", GenericFunctionLimits::DEFAULT).is_err());
    for bad in [
        "fn host(value){}",
        "fn host()->{}",
        "fn host(){} fn other(){}",
        "fn host(",
    ] {
        assert!(parse_specialization_root(bad, GenericFunctionLimits::DEFAULT).is_err());
    }
    let complete = "fn host(value:Int)->Int{return value;}";
    for end in 0..complete.len() {
        assert!(std::panic::catch_unwind(|| parse_specialization_root(
            &complete[..end],
            GenericFunctionLimits::DEFAULT
        ))
        .unwrap()
        .is_err());
    }
}
#[test]
fn plain_main_can_seed_graph_but_generic_main_and_public_generic_programs_still_refuse() {
    let base = bodies(&[ID]);
    let main =
        parse_specialization_root("fn main(){id::<Int>(7);}", GenericFunctionLimits::DEFAULT)
            .unwrap();
    let table = check_generic_signatures(
        base.signatures().types(),
        &[base.signatures().signatures()[0].syntax(), &main],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let graph = discover_generic_specializations(
        &checked,
        table.find("main").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(graph.instances().len(), 2);
    let generic_main =
        parse_generic_function("fn main<T>(){}", GenericFunctionLimits::DEFAULT).unwrap();
    assert!(check_generic_signatures(
        table.types(),
        &[&generic_main],
        GenericSignatureLimits::DEFAULT
    )
    .is_err());
    assert!(check_program_source(
        "strict",
        "fn id<T>(x:T)->T{return x;}fn main(){id::<Int>(7);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
