//! Concrete operation/context rechecking before executable generic lowering.
use sovra::compiler::v2::*;
fn graph(root: &str, functions: &[&str]) -> CheckedGenericSpecializations {
    let rows = parse_generic_alias("type Rows<T>=[T];", GenericAliasLimits::DEFAULT).unwrap();
    let ignored = parse_generic_alias("type Ignore<T>=Int;", GenericAliasLimits::DEFAULT).unwrap();
    let fixed = parse_generic_record(
        "struct Fixed<T>{value:Ignore<T>}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let ignored_list =
        parse_generic_alias("type IgnoreList<T>=[Int];", GenericAliasLimits::DEFAULT).unwrap();
    let ignored_record = parse_generic_alias(
        "type IgnoreRecord<T>=Box<Int>;",
        GenericAliasLimits::DEFAULT,
    )
    .unwrap();
    let fixed_list = parse_generic_record(
        "struct FixedList<T>{value:IgnoreList<T>}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let fixed_record = parse_generic_record(
        "struct FixedRecord<T>{value:IgnoreRecord<T>}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let boxed =
        parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let entry = parse_generic_record(
        "struct Entry<K:std::Key,V>{key:K,value:V}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let choice = parse_generic_enum(
        "enum Choice<T>{End,One(T),Many(Rows<T>),Unit(Unit)}",
        GenericEnumLimits::DEFAULT,
    )
    .unwrap();
    let definitions = [
        GenericTypeDeclaration::Alias(&rows),
        GenericTypeDeclaration::Alias(&ignored),
        GenericTypeDeclaration::Record(&fixed),
        GenericTypeDeclaration::Alias(&ignored_list),
        GenericTypeDeclaration::Alias(&ignored_record),
        GenericTypeDeclaration::Record(&fixed_list),
        GenericTypeDeclaration::Record(&fixed_record),
        GenericTypeDeclaration::Record(&boxed),
        GenericTypeDeclaration::Record(&entry),
        GenericTypeDeclaration::Enum(&choice),
    ];
    let registry = register_generic_types(&definitions, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(
        &registry,
        &[&rows, &ignored, &ignored_list, &ignored_record],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap();
    let types =
        check_generic_declarations(&env, &definitions, GenericDeclarationLimits::DEFAULT).unwrap();
    let mut syntax = vec![parse_specialization_root(root, GenericFunctionLimits::DEFAULT).unwrap()];
    syntax.extend(
        functions
            .iter()
            .map(|s| parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap()),
    );
    let signatures = check_generic_signatures(
        &types,
        &syntax.iter().collect::<Vec<_>>(),
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let bodies = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    discover_generic_specializations(
        &bodies,
        signatures.find("root").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap()
}
fn recheck(graph: &CheckedGenericSpecializations) -> CheckedConcreteGenericBodies {
    recheck_generic_concrete_bodies(graph, GenericConcreteBodyLimits::DEFAULT).unwrap()
}
const ID: &str = "fn identity<T>(x:T)->T{return x;}";
#[test]
fn signatures_bindings_and_expressions_receive_instance_specific_concrete_types() {
    let graph = graph(
        "fn root(){identity::<Int>(1);identity::<String>(\"hi\");}",
        &[ID],
    );
    let checked = recheck(&graph);
    assert_eq!(checked.bodies().len(), 3);
    for (index, primitive) in [(1, GenericPrimitive::Int), (2, GenericPrimitive::String)] {
        let concrete = &checked.bodies()[index];
        assert_eq!(concrete.return_type().primitive(), Some(primitive));
        assert_eq!(concrete.parameter_types()[0].primitive(), Some(primitive));
        for binding in concrete.body().bindings() {
            assert_eq!(
                concrete.binding_type(binding.id).unwrap().primitive(),
                Some(primitive)
            );
        }
        for expression in concrete.body().expressions() {
            assert_eq!(
                concrete.expression_type(expression.id).unwrap().primitive(),
                Some(primitive)
            );
        }
    }
    let id = checked.bodies()[1].body().bindings()[0].id;
    assert_eq!(
        checked.bodies()[2].binding_type(id).unwrap().primitive(),
        Some(GenericPrimitive::String)
    );
    assert!(checked.bodies()[0].binding_type(id).is_none());
    let foreign = graph_clone_helper();
    assert!(checked.bodies()[1]
        .binding_type(foreign.bodies().bodies()[1].bindings()[0].id)
        .is_none());
    let clone = checked.clone();
    drop(checked);
    drop(graph);
    assert_eq!(
        clone.bodies()[1].return_type().primitive(),
        Some(GenericPrimitive::Int)
    );
}
fn graph_clone_helper() -> CheckedGenericSpecializations {
    graph("fn root(){identity::<Int>(1);}", &[ID])
}
#[test]
fn arithmetic_strings_logic_equality_arrays_mutability_and_control_contexts_recheck() {
    let function="fn ops<T>(x:T)->T {let add=1+2.0;let sub=3-1;let mul=2*4;let div=4/2;let lt=1<2;let le=1<=2.0;let gt=2.0>1;let ge=2>=1;let words=\"a\"+\"b\";let ordered=words<\"z\";let truth=true&&false||true;let same=x==x;let mixed=1==2.0;let reverse=2.0!=1;let empty:[T]=[];let nested:[[T]]=[[],[x]];let mut rows:[T]=[x];rows[0]=x;let first=rows[0];let mut value:T=first;value=x;while false {value=first;}if true {return value;}else{return first;}}";
    let checked = recheck(&graph(
        "fn root(){ops::<String>(\"text\");ops::<Int>(7);}",
        &[function],
    ));
    for concrete in &checked.bodies()[1..] {
        assert!(concrete.body().expressions().len() > 40);
        for source in concrete.body().expressions() {
            if let Some(ty) = concrete.expression_type(source.id).unwrap().expanded() {
                assert!(ty
                    .nodes()
                    .iter()
                    .all(|node| !matches!(node.kind, ResolvedGenericTypeKind::Parameter(_))));
            }
        }
    }
}
#[test]
fn nominal_constructors_field_reads_nested_matches_and_payloads_recheck() {
    let choose="fn choose<T>(value:Choice<T>,fallback:T)->T {match value {Choice::End()=>{return fallback;},Choice::One(x)=>{let boxed=Box::<T>{value:x};return boxed.value;},Choice::Many(xs)=>{let next=Choice::<T>::One(xs[0]);match next {Choice::End()=>{return fallback;},Choice::One(x)=>{return x;},Choice::Many(xs)=>{return xs[0];},Choice::Unit(u)=>{return fallback;}}},Choice::Unit(u)=>{return fallback;}}}";
    let make="fn make<T>(x:T)->Choice<T>{let rows=Box::<[T]>{value:[x]};return Choice::<T>::Many(rows.value);}";
    let checked=recheck(&graph("fn root(){choose::<Int>(make::<Int>(1),0);choose::<String>(Choice::<String>::End(),\"fallback\");choose::<Int>(Choice::<Int>::Unit(unit::<Int>()),0);}",&[choose,make,"fn unit<T>(){}"]));
    assert!(checked.bodies().len() >= 4);
    for body in checked.bodies() {
        for binding in body.body().bindings() {
            if matches!(binding.kind, GenericCopyBindingKind::MatchPayload { .. }) {
                assert!(body.binding_type(binding.id).unwrap().expanded().is_some());
            }
        }
    }
}
#[test]
fn scalar_widening_remains_explicit_in_calls_fields_arrays_and_returns() {
    let function="fn widen<T>(x:T,value:Float)->Float{let typed:Float=1;let mut scalar:Float=1;scalar=2;let mut values:[Float]=[1,2.0];values[0]=3;let record=Box::<Float>{value:4};let wrapped=Choice::<Float>::One(5);return 6;}";
    let checked = recheck(&graph("fn root(){widen::<Bool>(true,1);}", &[function]));
    assert_eq!(
        checked.bodies()[1].return_type().primitive(),
        Some(GenericPrimitive::Float)
    );
}
#[test]
fn concrete_key_constraints_and_nominal_argument_owners_are_preserved() {
    let function="fn wrap<K:std::Key,V>(key:K,value:V)->Entry<K,V>{return Entry::<K,V>{key:key,value:value};}";
    let checked = recheck(&graph(
        "fn root(){wrap::<Int,String>(1,\"a\");wrap::<String,Bool>(\"k\",true);}",
        &[function],
    ));
    for body in &checked.bodies()[1..] {
        let ty = body.return_type().expanded().unwrap();
        assert!(matches!(
            ty.nodes()[ty.root()].kind,
            ResolvedGenericTypeKind::Declared { .. }
        ));
        assert!(ty
            .nodes()
            .iter()
            .all(|n| !matches!(n.kind, ResolvedGenericTypeKind::Parameter(_))));
    }
}
#[test]
fn direct_and_mutual_recursion_use_concrete_target_signatures_without_recursive_rechecking() {
    let checked = recheck(&graph(
        "fn root(){first::<Int>(1);}",
        &[
            "fn first<T>(x:T)->T{return second::<T>(x);}",
            "fn second<U>(x:U)->U{return first::<U>(x);}",
        ],
    ));
    assert_eq!(checked.bodies().len(), 3);
    assert_eq!(checked.graph().instances()[2].calls()[0].target, 1);
    assert_eq!(
        checked.bodies()[2].return_type().primitive(),
        Some(GenericPrimitive::Int)
    );
}
#[test]
fn all_rechecking_dimensions_are_exact_and_one_under_refuses_without_partial_bodies() {
    let graph = graph(
        "fn root(){identity::<[Int]>([1]);identity::<[String]>([\"s\"]);}",
        &[ID],
    );
    let usage = recheck(&graph).usage();
    let exact = GenericConcreteBodyLimits {
        max_instances: usage.instances,
        max_types: usage.types,
        max_nodes: usage.nodes,
        max_edges: usage.edges,
        max_pairs: usage.pairs,
        max_comparison_edges: usage.comparison_edges,
        max_work: usage.work,
        ..GenericConcreteBodyLimits::DEFAULT
    };
    assert_eq!(
        recheck_generic_concrete_bodies(&graph, exact)
            .unwrap()
            .usage(),
        usage
    );
    for limits in [
        GenericConcreteBodyLimits {
            max_instances: exact.max_instances - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            max_types: exact.max_types - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            max_pairs: exact.max_pairs - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            max_comparison_edges: exact.max_comparison_edges - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericConcreteBodyLimits {
            expansion: GenericAliasExpansionLimits {
                max_work: 0,
                ..exact.expansion
            },
            ..exact
        },
        GenericConcreteBodyLimits {
            comparison: GenericTypeEquivalenceLimits {
                max_pairs: 0,
                ..exact.comparison
            },
            ..exact
        },
    ] {
        assert!(
            recheck_generic_concrete_bodies(&graph, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn temporary_member_contexts_share_cumulative_graph_budgets_and_depth_limits() {
    let graph = graph(
        "fn root(){boxer::<Int>(1);boxer::<String>(\"s\");}",
        &["fn boxer<T>(x:T)->T{let b=Box::<[T]>{value:[x]};return b.value[0];}"],
    );
    let usage = recheck(&graph).usage();
    let retained: usize = graph
        .instances()
        .iter()
        .map(|i| {
            let b = graph.bodies().body(i.function()).unwrap();
            b.signature().parameter_types().len() + 1 + b.bindings().len() + b.expressions().len()
        })
        .sum();
    assert!(usage.types > retained);
    for depth in [0, 1, 129] {
        assert!(recheck_generic_concrete_bodies(
            &graph,
            GenericConcreteBodyLimits {
                max_depth: depth,
                ..GenericConcreteBodyLimits::DEFAULT
            }
        )
        .is_err());
    }
    assert!(recheck_generic_concrete_bodies(
        &graph,
        GenericConcreteBodyLimits {
            max_types: retained,
            ..GenericConcreteBodyLimits::DEFAULT
        }
    )
    .is_err());
}
#[test]
fn unreachable_operations_are_rechecked_and_original_source_ranges_remain_owned() {
    let graph = graph(
        "// λ😀\nfn root(){identity::<Int>(1);}",
        &["fn identity<T>(x:T)->T{return x;let ignored=x==x;}"],
    );
    let checked = recheck(&graph);
    let concrete = &checked.bodies()[1];
    assert_eq!(concrete.body().statements().len(), 2);
    for expression in concrete.body().expressions() {
        let source = concrete.body().signature().syntax().source();
        assert!(!source[expression.span.start..expression.span.end].is_empty());
        if matches!(expression.kind, GenericCopyExpressionKind::Equality { .. }) {
            assert_eq!(
                concrete.expression_type(expression.id).unwrap().primitive(),
                Some(GenericPrimitive::Bool)
            );
        }
    }
    assert!(check_program_source(
        "strict",
        "fn identity<T>(x:T)->T{return x;}fn main(){identity::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn empty_unit_root_requires_no_generated_nodes_and_keeps_explicit_zero_quotas() {
    let graph = graph("fn root(){}", &[]);
    let checked = recheck_generic_concrete_bodies(
        &graph,
        GenericConcreteBodyLimits {
            max_instances: 1,
            max_types: 1,
            max_nodes: 0,
            max_edges: 0,
            max_comparison_edges: 0,
            ..GenericConcreteBodyLimits::DEFAULT
        },
    )
    .unwrap();
    assert_eq!(checked.usage().types, 1);
    assert_eq!(checked.usage().nodes, 0);
    assert_eq!(
        checked.bodies()[0].return_type().primitive(),
        Some(GenericPrimitive::Unit)
    );
    assert!(checked.bodies()[0].body().implicit_unit_return());
}

#[test]
fn aliases_that_discard_record_parameters_do_not_leak_unused_foreign_owners() {
    let graph = graph(
        "fn root(){read::<String>(Fixed::<String>{value:1});}",
        &["fn read<U>(x:Fixed<U>)->Int{let value=x.value;return value;}"],
    );
    let checked = recheck(&graph);
    let body = &checked.bodies()[1];
    assert_eq!(body.return_type().primitive(), Some(GenericPrimitive::Int));
    assert_eq!(
        body.binding_type(body.body().bindings()[1].id)
            .unwrap()
            .primitive(),
        Some(GenericPrimitive::Int)
    );
}

#[test]
fn discarded_field_parameters_in_arrays_and_nominal_aliases_keep_reachable_provenance() {
    let graph=graph("fn root(){list::<String>(FixedList::<String>{value:[1]});record::<String>(FixedRecord::<String>{value:Box::<Int>{value:1}});}", &[
        "fn list<U>(x:FixedList<U>)->Int{let rows=x.value;return rows[0];}",
        "fn record<U>(x:FixedRecord<U>)->Int{let boxed=x.value;return boxed.value;}",
    ]);
    let checked = recheck(&graph);
    for body in &checked.bodies()[1..] {
        assert_eq!(body.return_type().primitive(), Some(GenericPrimitive::Int));
        for source in body.body().expressions() {
            let concrete = body.expression_type(source.id).unwrap();
            if let Some(ty) = concrete.expanded() {
                assert!(ty
                    .nodes()
                    .iter()
                    .all(|n| !matches!(n.kind, ResolvedGenericTypeKind::Parameter(_))));
                let root = &ty.nodes()[ty.root()];
                let text = ty.node_source(ty.root()).unwrap();
                assert!(!text[root.origin.span.start..root.origin.span.end].is_empty());
            }
        }
    }
    let usage = checked.usage();
    let exact = GenericConcreteBodyLimits {
        max_types: usage.types,
        max_nodes: usage.nodes,
        max_edges: usage.edges,
        max_work: usage.work,
        ..GenericConcreteBodyLimits::DEFAULT
    };
    assert!(recheck_generic_concrete_bodies(&graph, exact).is_ok());
    assert!(recheck_generic_concrete_bodies(
        &graph,
        GenericConcreteBodyLimits {
            max_nodes: usage.nodes - 1,
            ..exact
        }
    )
    .is_err());
    assert!(recheck_generic_concrete_bodies(
        &graph,
        GenericConcreteBodyLimits {
            max_work: usage.work - 1,
            ..exact
        }
    )
    .is_err());
}
