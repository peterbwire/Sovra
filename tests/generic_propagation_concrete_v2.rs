//! Closed specialization and propagation rechecking, with an unconditional IR gate.
use sovra::compiler::v2::*;
fn bodies(sources: &[&str]) -> CheckedGenericCopyBodies {
    let base = compile_generic_source(
        "type Failure<T>=std::Result<T,std::Error>;fn main(){}",
        "main",
        GenericSourceLimits::DEFAULT,
    )
    .unwrap();
    let types = base
        .ir()
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let syntax: Vec<_> = sources
        .iter()
        .map(|s| parse_generic_propagation_function(s, GenericFunctionLimits::DEFAULT).unwrap())
        .collect();
    let signatures = check_generic_signatures(
        types,
        &syntax.iter().collect::<Vec<_>>(),
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    check_generic_propagation_bodies(
        &signatures,
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
}
fn graph(sources: &[&str]) -> CheckedGenericSpecializations {
    let b = bodies(sources);
    discover_generic_propagation_specializations(
        &b,
        b.signatures().find("root").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap()
}
fn concrete(sources: &[&str]) -> CheckedConcreteGenericBodies {
    recheck_generic_propagation_concrete_bodies(
        &graph(sources),
        GenericConcreteBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
}
const HELPER:&str="fn extract<T,U,E>(x:std::Result<T,E>,fallback:U)->std::Result<U,E>{let value=x?;return std::Result::<U,E>::Ok(fallback);}";
const ROOT:&str="fn root()->Bool{let a=extract::<Int,Bool,String>(std::Result::<Int,String>::Ok(1),true);let b=extract::<String,Int,std::Option<[String]>>(std::Result::<String,std::Option<[String]>>::Ok(\"ok\"),2);let c=extract::<Bool,Int,std::Error>(std::Result::<Bool,std::Error>::Ok(true),3);return true;}";
fn primitive(ty: &ExpandedGenericType) -> Option<GenericPrimitive> {
    if let ResolvedGenericTypeKind::Primitive(p) = ty.nodes()[ty.root()].kind {
        Some(p)
    } else {
        None
    }
}
#[test]
fn each_specialization_recomputes_distinct_closed_payloads_and_targets() {
    let c = concrete(&[ROOT, HELPER]);
    assert_eq!(c.bodies().len(), 4);
    assert_eq!(c.propagation_usage().unwrap().sites, 3);
    let success: Vec<_> = c
        .bodies()
        .iter()
        .flat_map(|body| body.propagations())
        .map(|p| primitive(p.types().success_type()).unwrap())
        .collect();
    assert_eq!(
        success,
        [
            GenericPrimitive::Int,
            GenericPrimitive::String,
            GenericPrimitive::Bool
        ]
    );
    for body in c.bodies() {
        for p in body.propagations() {
            assert_eq!(p.types().kind(), GenericStandardEnum::Result);
            assert!(p.types().failure_type().is_some());
            assert_eq!(p.types().success_variant(), 0);
            assert_eq!(p.types().failure_variant(), 1);
            assert!(p.operand().ordinal() < p.expression().ordinal());
            assert!(body.expression_type(p.expression()).is_some());
            assert_eq!(
                &body.body().signature().syntax().source()[p.question().start..p.question().end],
                "?"
            );
        }
    }
}
#[test]
fn all_retained_concrete_contexts_have_no_parameter_nodes() {
    let c = concrete(&[ROOT, HELPER]);
    for body in c.bodies() {
        for p in body.propagations() {
            let mut contexts = vec![p.types().success_type(), p.types().return_type()];
            contexts.extend(p.types().failure_type());
            for ty in contexts {
                assert!(ty
                    .nodes()
                    .iter()
                    .all(|n| !matches!(n.kind, ResolvedGenericTypeKind::Parameter(_))));
            }
        }
    }
}
#[test]
fn recursive_instances_are_reused_without_rechecking_unbounded_calls() {
    let sources=["fn root()->std::Option<Int>{return recur::<Int>(2,std::Option::<Int>::Some(1));}","fn recur<T>(n:Int,x:std::Option<T>)->std::Option<T>{if n>0{let value=recur::<T>(n-1,x)?;}return x;}"];
    let g = graph(&sources);
    assert_eq!(g.instances().len(), 2);
    assert_eq!(g.instances()[1].calls()[0].target, 1);
    let c = recheck_generic_propagation_concrete_bodies(
        &g,
        GenericConcreteBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(c.propagation_usage().unwrap().sites, 1);
    let p = &c.bodies()[1].propagations()[0];
    assert!(p.types().failure_type().is_none());
    assert_eq!(
        primitive(p.types().success_type()),
        Some(GenericPrimitive::Int)
    );
}
#[test]
fn aliases_structured_errors_grouped_chains_and_fields_recheck() {
    let sources=["fn root()->Failure<Int>{let inner=std::Result::<Int,std::Error>::Ok(7);let outer=std::Result::<std::Result<Int,std::Error>,std::Error>::Ok(inner);return test::<Int>(outer);}","fn test<T>(x:std::Result<std::Result<T,std::Error>,std::Error>)->Failure<T>{let value=(x?)?;return std::Result::<T,std::Error>::Ok(value);}"];
    let c = concrete(&sources);
    assert_eq!(c.propagation_usage().unwrap().sites, 2);
    assert!(c.bodies()[1]
        .propagations()
        .iter()
        .all(|p| p.types().failure_type().is_some()));
    let c=concrete(&["fn root()->std::Option<Int>{let error=std::Error{code:\"e\",message:\"m\",offset:std::Option::<Int>::Some(7)};return test(error.offset);}","fn test(x:std::Option<Int>)->std::Option<Int>{let value=(x?)+1;return std::Option::<Int>::Some(value);}"]);
    assert_eq!(c.propagation_usage().unwrap().sites, 1);
}

#[test]
fn ordinary_specialization_and_concrete_apis_keep_their_explicit_refusal() {
    let b = bodies(&[ROOT, HELPER]);
    let root = b.signatures().find("root").unwrap();
    assert!(matches!(
        discover_generic_specializations(&b, root, GenericSpecializationLimits::DEFAULT),
        Err(GenericSpecializationError::PropagationUnavailable)
    ));
    let g = discover_generic_propagation_specializations(
        &b,
        root,
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        recheck_generic_concrete_bodies(&g, GenericConcreteBodyLimits::DEFAULT),
        Err(GenericConcreteBodyError::PropagationUnavailable)
    ));
}
#[test]
fn literal_layout_and_nominal_plans_cannot_promote_postfix_to_ir() {
    let c = concrete(&[ROOT, HELPER]);
    let literals = decode_generic_literals(&c, GenericLiteralLimits::DEFAULT).unwrap();
    let layouts = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    let plans = plan_generic_construction(&layouts, GenericConstructionLimits::DEFAULT).unwrap();
    assert!(matches!(
        lower_generic_ir(&plans, GenericIrLimits::DEFAULT),
        Err(GenericIrError::PropagationUnavailable)
    ));
}
#[test]
fn unreachable_source_questions_still_block_lowering() {
    let c = concrete(&[
        "fn root(){}",
        "fn unused(x:std::Option<Int>)->std::Option<Int>{let value=x?;return x;}",
    ]);
    assert_eq!(c.propagation_usage().unwrap().sites, 0);
    let l = decode_generic_literals(&c, GenericLiteralLimits::DEFAULT).unwrap();
    let l = plan_generic_layouts(&l, GenericLayoutLimits::DEFAULT).unwrap();
    let p = plan_generic_construction(&l, GenericConstructionLimits::DEFAULT).unwrap();
    assert!(matches!(
        lower_generic_ir(&p, GenericIrLimits::DEFAULT),
        Err(GenericIrError::PropagationUnavailable)
    ));
}
#[test]
fn empty_feature_tables_preserve_the_ordinary_pipeline() {
    let c = concrete(&["fn root()->Int{return 7;}"]);
    assert_eq!(c.propagation_usage().unwrap().sites, 0);
    let l = decode_generic_literals(&c, GenericLiteralLimits::DEFAULT).unwrap();
    let l = plan_generic_layouts(&l, GenericLayoutLimits::DEFAULT).unwrap();
    let p = plan_generic_construction(&l, GenericConstructionLimits::DEFAULT).unwrap();
    let ir = lower_generic_ir(&p, GenericIrLimits::DEFAULT).unwrap();
    assert_eq!(
        execute_generic_ir(&ir, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_int(),
        Some(7)
    );
}
#[test]
fn concrete_propagation_quotas_are_shared_across_instances() {
    let g = graph(&[ROOT, HELPER]);
    let usage = recheck_generic_propagation_concrete_bodies(
        &g,
        GenericConcreteBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
    .propagation_usage()
    .unwrap();
    macro_rules! boundary {
        ($limit:ident,$cost:ident) => {{
            let mut l = GenericPropagationLimits::DEFAULT;
            l.$limit = usage.$cost;
            recheck_generic_propagation_concrete_bodies(&g, GenericConcreteBodyLimits::DEFAULT, l)
                .unwrap();
            l.$limit -= 1;
            assert!(
                recheck_generic_propagation_concrete_bodies(
                    &g,
                    GenericConcreteBodyLimits::DEFAULT,
                    l
                )
                .is_err(),
                stringify!($limit)
            );
        }};
    }
    boundary!(max_sites, sites);
    boundary!(max_contexts, contexts);
    boundary!(max_pairs, pairs);
    boundary!(max_edges, edges);
    boundary!(max_work, work);
}
#[test]
fn concrete_stage_quotas_include_descriptors_contexts_and_comparisons() {
    let g = graph(&[ROOT, HELPER]);
    let usage = recheck_generic_propagation_concrete_bodies(
        &g,
        GenericConcreteBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
    .usage();
    macro_rules! boundary {
        ($limit:ident,$cost:ident) => {{
            let mut l = GenericConcreteBodyLimits::DEFAULT;
            l.$limit = usage.$cost;
            recheck_generic_propagation_concrete_bodies(&g, l, GenericPropagationLimits::DEFAULT)
                .unwrap();
            l.$limit -= 1;
            assert!(
                recheck_generic_propagation_concrete_bodies(
                    &g,
                    l,
                    GenericPropagationLimits::DEFAULT
                )
                .is_err(),
                stringify!($limit)
            );
        }};
    }
    boundary!(max_instances, instances);
    boundary!(max_types, types);
    boundary!(max_nodes, nodes);
    boundary!(max_edges, edges);
    boundary!(max_pairs, pairs);
    boundary!(max_comparison_edges, comparison_edges);
    boundary!(max_work, work);
}
#[test]
fn independent_comparison_work_is_not_an_unlimited_fallback() {
    let g = graph(&[ROOT, HELPER]);
    let mut l = GenericConcreteBodyLimits::DEFAULT;
    l.comparison.max_work = 0;
    assert!(
        recheck_generic_propagation_concrete_bodies(&g, l, GenericPropagationLimits::DEFAULT)
            .is_err()
    );
}
#[test]
fn source_and_existing_native_profiles_still_refuse_postfix() {
    let source="fn root()->std::Option<Int>{let value=std::Option::<Int>::Some(1)?;return std::Option::<Int>::Some(value);}";
    assert!(compile_generic_source(source, "root", GenericSourceLimits::DEFAULT).is_err());
    for features in [1u32, 3, 7] {
        let mut bytes = b"SVRG2\0".to_vec();
        bytes.extend_from_slice(&[1, 2]);
        bytes.extend_from_slice(&features.to_le_bytes());
        bytes.extend_from_slice(&(source.len() as u32).to_le_bytes());
        bytes.extend_from_slice(source.as_bytes());
        assert!(decode_generic_native_image(&bytes).is_err());
    }
}

#[test]
fn independent_comparison_work_covers_comparisons_not_site_overhead() {
    let g=graph(&["fn root()->Bool{extract::<Int,Bool,String>(std::Result::<Int,String>::Ok(1),true);return true;}",HELPER]);
    let mut limits = GenericConcreteBodyLimits::DEFAULT;
    limits.comparison.max_work = 5;
    recheck_generic_propagation_concrete_bodies(&g, limits, GenericPropagationLimits::DEFAULT)
        .unwrap();
}

#[test]
fn literals_in_question_bodies_keep_original_ranges_through_layout_planning() {
    let c=concrete(&["fn root()->std::Option<Int>{return test(std::Option::<Int>::Some(7));}","fn test(x:std::Option<Int>)->std::Option<Int>{let value=(x?)+1;return std::Option::<Int>::Some(value);}"]);
    let literals = decode_generic_literals(&c, GenericLiteralLimits::DEFAULT).unwrap();
    let body = &literals.bodies()[1];
    assert_eq!(body.literals().len(), 1);
    let fact = &body.literals()[0];
    let source = body.body().body().signature().syntax().source();
    assert_eq!(&source[fact.span.start..fact.span.end], "1");
    let layouts = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    let p = &layouts.bodies()[1].body().propagations()[0];
    assert!(layouts.bodies()[1].expression_layout(p.operand()).is_some());
    assert!(layouts.bodies()[1]
        .expression_layout(p.expression())
        .is_some());
    let plans = plan_generic_construction(&layouts, GenericConstructionLimits::DEFAULT).unwrap();
    assert!(matches!(
        lower_generic_ir(&plans, GenericIrLimits::DEFAULT),
        Err(GenericIrError::PropagationUnavailable)
    ));
}

fn propagation_layouts(sources: &[&str]) -> CheckedGenericLayouts {
    let c = concrete(sources);
    let literals = decode_generic_literals(&c, GenericLiteralLimits::DEFAULT).unwrap();
    plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap()
}
#[test]
fn propagation_operation_layouts_preserve_per_instance_result_contracts() {
    let layouts = propagation_layouts(&[ROOT, HELPER]);
    let planned =
        plan_generic_propagation_layouts(&layouts, GenericPropagationLayoutLimits::DEFAULT)
            .unwrap();
    assert_eq!(planned.plans().len(), 3);
    for p in planned.plans() {
        let b = &planned.layouts().bodies()[p.instance];
        assert_eq!(b.expression_layout(p.operand), Some(p.input));
        assert_eq!(b.expression_layout(p.expression), Some(p.success));
        assert_eq!(b.return_layout(), p.result);
        assert!(p.failure.is_some());
        assert_eq!((p.success_tag, p.failure_tag), (0, 1));
    }
    let clone = planned.clone();
    drop(planned);
    assert_eq!(clone.plans().len(), 3);
    let nominal =
        plan_generic_construction(clone.layouts(), GenericConstructionLimits::DEFAULT).unwrap();
    assert!(matches!(
        lower_generic_ir(&nominal, GenericIrLimits::DEFAULT),
        Err(GenericIrError::PropagationUnavailable)
    ));
}
#[test]
fn option_extraction_allows_different_caller_success_type() {
    let l = propagation_layouts(&["fn root(x:std::Option<Int>)->std::Option<Bool>{let n=x?;return std::Option::<Bool>::Some(true);}"]);
    let p = plan_generic_propagation_layouts(&l, GenericPropagationLayoutLimits::DEFAULT).unwrap();
    assert_eq!(p.plans().len(), 1);
    let site = &p.plans()[0];
    assert_eq!(site.failure, None);
    assert_ne!(site.input, site.result);
    assert_eq!(
        p.layouts().layout(site.success).unwrap().ty().primitive(),
        Some(GenericPrimitive::Int)
    );
}
#[test]
fn chained_propagation_operations_keep_source_order() {
    let l = propagation_layouts(&["fn root(x:std::Option<std::Option<Int>>)->std::Option<Int>{let n=(x?)?;return std::Option::<Int>::Some(n);}"]);
    let p = plan_generic_propagation_layouts(&l, GenericPropagationLayoutLimits::DEFAULT).unwrap();
    assert_eq!(p.plans().len(), 2);
    assert_eq!(p.plans()[0].success, p.plans()[1].input);
    assert!(p.plans()[0].question.start < p.plans()[1].question.start);
}
#[test]
fn operation_layout_limits_are_shared_and_return_no_partial_table() {
    let l = propagation_layouts(&[ROOT, HELPER]);
    let default = GenericPropagationLayoutLimits::DEFAULT;
    for limits in [
        GenericPropagationLayoutLimits {
            max_instances: 3,
            ..default
        },
        GenericPropagationLayoutLimits {
            max_plans: 2,
            ..default
        },
        GenericPropagationLayoutLimits {
            max_work: 0,
            ..default
        },
        GenericPropagationLayoutLimits {
            comparison: GenericTypeEquivalenceLimits {
                max_pairs: 0,
                ..default.comparison
            },
            ..default
        },
        GenericPropagationLayoutLimits {
            comparison: GenericTypeEquivalenceLimits {
                max_edges: 0,
                ..default.comparison
            },
            ..default
        },
        GenericPropagationLayoutLimits {
            comparison: GenericTypeEquivalenceLimits {
                max_work: 0,
                ..default.comparison
            },
            ..default
        },
    ] {
        assert!(plan_generic_propagation_layouts(&l, limits).is_err());
    }
    assert!(plan_generic_propagation_layouts(
        &l,
        GenericPropagationLayoutLimits {
            max_instances: 4,
            max_plans: 3,
            ..default
        }
    )
    .is_ok());
}
#[test]
fn no_site_layout_tables_require_no_site_or_comparison_allowance() {
    let l = propagation_layouts(&[
        "fn root(){}",
        "fn unused(x:std::Option<Int>)->std::Option<Int>{let n=x?;return x;}",
    ]);
    let p = plan_generic_propagation_layouts(
        &l,
        GenericPropagationLayoutLimits {
            max_instances: 1,
            max_plans: 0,
            max_work: 1,
            comparison: GenericTypeEquivalenceLimits {
                max_pairs: 0,
                max_edges: 0,
                max_work: 0,
            },
        },
    )
    .unwrap();
    assert!(p.plans().is_empty());
    let nominal =
        plan_generic_construction(p.layouts(), GenericConstructionLimits::DEFAULT).unwrap();
    assert!(matches!(
        lower_generic_ir(&nominal, GenericIrLimits::DEFAULT),
        Err(GenericIrError::PropagationUnavailable)
    ));
}

fn propagation_ir(sources: &[&str]) -> CheckedGenericPropagationIr {
    let layouts = propagation_layouts(sources);
    let operations =
        plan_generic_propagation_layouts(&layouts, GenericPropagationLayoutLimits::DEFAULT)
            .unwrap();
    lower_generic_propagation_ir(
        &operations,
        GenericConstructionLimits::DEFAULT,
        GenericIrLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn early_return_ir_links_each_specialized_failure_to_its_caller_result() {
    let ir = propagation_ir(&[ROOT, HELPER]);
    let mut count = 0;
    for f in ir.functions() {
        for instruction in f.instructions() {
            if let GenericIrOperation::Propagate {
                input,
                success,
                result,
                success_tag,
                failure_tag,
                failure,
            } = instruction.operation
            {
                count += 1;
                assert_eq!(f.return_layout(), result);
                assert_ne!(input, success);
                assert_eq!((success_tag, failure_tag), (0, 1));
                assert!(failure.is_some());
            }
        }
    }
    assert_eq!(count, 3);
}
#[test]
fn early_return_ir_preserves_pending_operands_and_nested_source_order() {
    let source="fn root(x:std::Option<std::Option<Int>>)->std::Option<Int>{return std::Option::<Int>::Some(7+(x?)?);}";
    let ir = propagation_ir(&[source]);
    let function = &ir.functions()[0];
    assert!(function.max_stack() >= 2);
    let questions: Vec<_> = function
        .instructions()
        .iter()
        .filter(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
        .map(|i| i.span)
        .collect();
    assert_eq!(questions.len(), 2);
    assert!(questions[0].start < questions[1].start);
    for q in questions {
        assert_eq!(&source[q.start..q.end], "?");
    }
    assert_eq!(
        function
            .instructions()
            .iter()
            .filter(|i| matches!(i.operation, GenericIrOperation::Load { .. }))
            .count(),
        1
    );
}
#[test]
fn early_return_ir_validates_short_circuit_loop_and_match_joins() {
    let source="fn root(xs:[std::Option<std::Error>],flag:std::Option<Bool>)->std::Option<String>{if true&&flag?{let e=xs[0]?;return std::Option::<String>::Some(e.message);}while false{let e=xs[0]?;}let item=xs[0];match item{std::Option::Some(error)=>{let offset=error.offset;let value=offset?;return std::Option::<String>::Some(error.code);},std::Option::None()=>{return std::Option::<String>::None();}}}";
    let ir = propagation_ir(&[source]);
    assert_eq!(
        ir.functions()[0]
            .instructions()
            .iter()
            .filter(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
            .count(),
        4
    );
}
#[test]
fn early_return_lowering_budgets_have_exact_usage_boundaries() {
    let l = propagation_layouts(&[
        "fn root(x:std::Option<Int>)->std::Option<Int>{return std::Option::<Int>::Some(1+x?);}",
    ]);
    let p = plan_generic_propagation_layouts(&l, GenericPropagationLayoutLimits::DEFAULT).unwrap();
    let make =
        |limits| lower_generic_propagation_ir(&p, GenericConstructionLimits::DEFAULT, limits);
    let usage = make(GenericIrLimits::DEFAULT).unwrap().usage();
    let exact = GenericIrLimits {
        max_functions: usage.functions,
        max_instructions: usage.instructions,
        max_slots: usage.slots,
        max_operands: usage.operands,
        max_labels: usage.labels,
        max_targets: usage.targets,
        max_pending: usage.peak_pending,
        max_stack_nodes: usage.stack_nodes,
        max_work: usage.work,
        ..GenericIrLimits::DEFAULT
    };
    assert!(make(exact).is_ok());
    for bad in [
        GenericIrLimits {
            max_instructions: usage.instructions - 1,
            ..exact
        },
        GenericIrLimits {
            max_operands: usage.operands - 1,
            ..exact
        },
        GenericIrLimits {
            max_targets: usage.targets - 1,
            ..exact
        },
        GenericIrLimits {
            max_stack_nodes: usage.stack_nodes - 1,
            ..exact
        },
        GenericIrLimits {
            max_work: usage.work - 1,
            ..exact
        },
        GenericIrLimits {
            max_pending: usage.peak_pending - 1,
            ..exact
        },
        GenericIrLimits {
            max_stack_depth: 1,
            ..exact
        },
    ] {
        assert!(make(bad).is_err());
    }
    assert!(matches!(
        lower_generic_propagation_ir(
            &p,
            GenericConstructionLimits {
                max_work: 0,
                ..GenericConstructionLimits::DEFAULT
            },
            exact
        ),
        Err(GenericIrError::Construction(_))
    ));
}
#[test]
fn early_return_ir_does_not_enable_the_ordinary_lowering_entry() {
    let ir = propagation_ir(&[
        "fn root(){}",
        "fn unused(x:std::Option<Int>)->std::Option<Int>{let n=x?;return x;}",
    ]);
    assert_eq!(ir.functions().len(), 1);
    assert!(!ir.functions()[0]
        .instructions()
        .iter()
        .any(|i| matches!(i.operation, GenericIrOperation::Propagate { .. })));
    let nominal =
        plan_generic_construction(ir.layouts(), GenericConstructionLimits::DEFAULT).unwrap();
    assert!(matches!(
        lower_generic_ir(&nominal, GenericIrLimits::DEFAULT),
        Err(GenericIrError::PropagationUnavailable)
    ));
}

#[test]
fn unreachable_post_return_questions_are_still_emitted_and_structurally_checked() {
    let ir = propagation_ir(&["fn root(x:std::Option<Int>)->std::Option<Int>{return x;let n=x?;}"]);
    let code = ir.functions()[0].instructions();
    let ret = code
        .iter()
        .position(|i| matches!(i.operation, GenericIrOperation::Return))
        .unwrap();
    let question = code
        .iter()
        .position(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
        .unwrap();
    assert!(question > ret);
}

fn run_propagation(sources: &[&str]) -> GenericVmOutcome {
    execute_generic_propagation_ir(&propagation_ir(sources), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn propagation_vm_extracts_owned_payloads_and_preserves_snapshots() {
    let outcome=run_propagation(&["fn root()->std::Option<[Int]>{let mut a:[Int]=[1,2];let x=std::Option::<[Int]>::Some(a);let copy=x?;a[0]=99;return std::Option::<[Int]>::Some(copy);}"]);
    let value = outcome.value();
    assert_eq!(value.enum_tag(), Some(0));
    assert_eq!(
        value.payload().unwrap().array_item(0).unwrap().as_int(),
        Some(1)
    );
    assert!(matches!(
        execute_generic_ir(outcome.ir(), GenericVmPolicy::DEFAULT),
        Err(GenericVmError {
            kind: GenericVmErrorKind::PropagationUnavailable,
            ..
        })
    ));
}
#[test]
fn propagation_vm_rebuilds_result_with_distinct_success_type_and_exact_error() {
    let outcome=run_propagation(&["fn root()->std::Result<Bool,String>{let x=std::Result::<Int,String>::Err(\"failure\");let n=10+x?;return std::Result::<Bool,String>::Ok(1/0==0);}"]);
    assert_eq!(outcome.value().enum_tag(), Some(1));
    assert_eq!(outcome.value().payload().unwrap().as_str(), Some("failure"));
    assert_eq!(
        outcome.value().layout(),
        outcome.ir().functions()[0].return_layout()
    );
}
#[test]
fn propagation_vm_preserves_structured_error_payloads() {
    let outcome=run_propagation(&["fn root()->std::Result<Bool,std::Error>{let e=std::Error{code:\"bad\",message:\"invalid\",offset:std::Option::<Int>::Some(7)};let x=std::Result::<Int,std::Error>::Err(e);let n=x?;return std::Result::<Bool,std::Error>::Ok(true);}"]);
    let error = outcome.value().payload().unwrap();
    assert_eq!(error.field(0).unwrap().as_str(), Some("bad"));
    assert_eq!(error.field(1).unwrap().as_str(), Some("invalid"));
    assert_eq!(error.field(2).unwrap().payload().unwrap().as_int(), Some(7));
}
#[test]
fn propagation_vm_handles_chains_and_short_circuits_without_evaluating_failure_rhs() {
    let outcome=run_propagation(&["fn root()->std::Option<Int>{let inner=std::Option::<Int>::Some(5);let outer=std::Option::<std::Option<Int>>::Some(inner);let n=(outer?)?;let flag=std::Option::<Bool>::None();let skip=false&&flag?;return std::Option::<Int>::Some(10+n);}"]);
    assert_eq!(outcome.value().payload().unwrap().as_int(), Some(15));
    let none=run_propagation(&["fn root()->std::Option<Bool>{let x=std::Option::<Int>::None();let n=1+x?;return std::Option::<Bool>::Some(1/0==0);}"]);
    assert_eq!(none.value().enum_tag(), Some(1));
    assert!(none.value().payload().is_none());
}
const CLEANUP_ROOT:&str="fn root()->Int{let mut n=0;while n<20{let result=fail::<String>(std::Option::<Int>::None());match result{std::Option::Some(v)=>{return 0;},std::Option::None()=>{n=n+1;}}}return n;}";
const CLEANUP_HELPER:&str="fn fail<T>(x:std::Option<Int>)->std::Option<Int>{let pending:[Int]=[1,2,3];return std::Option::<Int>::Some(100+x?);}";
#[test]
fn repeated_early_returns_release_frame_slots_and_pending_operands() {
    let ir = propagation_ir(&[CLEANUP_ROOT, CLEANUP_HELPER]);
    let first = execute_generic_propagation_ir(&ir, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(first.value().as_int(), Some(20));
    assert_eq!(first.peak_frames(), 2);
    let exact = GenericVmPolicy {
        max_call_depth: 2,
        max_active_slots: first.peak_slots(),
        max_active_operands: first.peak_operands(),
        ..GenericVmPolicy::DEFAULT
    };
    assert_eq!(
        execute_generic_propagation_ir(&ir, exact)
            .unwrap()
            .value()
            .as_int(),
        Some(20)
    );
}
#[test]
fn propagation_vm_uses_shared_exact_work_instruction_and_materialization_limits() {
    use sovra::compiler::execution::{ExecutionLimits, MaterializationLimits, RuntimePolicy};
    let ir=propagation_ir(&["fn root()->std::Result<Bool,String>{let x=std::Result::<Int,String>::Err(\"error\");let n=1+x?;return std::Result::<Bool,String>::Ok(true);}"]);
    let out = execute_generic_propagation_ir(&ir, GenericVmPolicy::DEFAULT).unwrap();
    let default = GenericVmPolicy::DEFAULT;
    let nodes = default.runtime.materialization.max_value_nodes.unwrap()
        - out.remaining().materialization.max_value_nodes.unwrap();
    let text = default.runtime.materialization.max_text_bytes.unwrap()
        - out.remaining().materialization.max_text_bytes.unwrap();
    let exact = GenericVmPolicy {
        max_work: out.work(),
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(out.instructions()),
                max_output_bytes: Some(0),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(nodes),
                max_text_bytes: Some(text),
            },
        },
        ..default
    };
    assert!(execute_generic_propagation_ir(&ir, exact.clone()).is_ok());
    let mut bad = exact.clone();
    bad.max_work -= 1;
    let err = execute_generic_propagation_ir(&ir, bad).unwrap_err();
    assert!(matches!(err.kind, GenericVmErrorKind::Limit(_)));
    assert_eq!(
        &ir.layouts()
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .signatures()[0]
            .syntax()
            .source()[err.span.start..err.span.end],
        "?"
    );
    let mut bad = exact.clone();
    bad.runtime.execution.max_instructions = Some(out.instructions() - 1);
    assert!(execute_generic_propagation_ir(&ir, bad).is_err());
    let mut bad = exact.clone();
    bad.runtime.materialization.max_value_nodes = Some(nodes - 1);
    assert!(matches!(
        execute_generic_propagation_ir(&ir, bad),
        Err(GenericVmError {
            kind: GenericVmErrorKind::Materialization(_),
            ..
        })
    ));
    let mut bad = exact;
    bad.runtime.materialization.max_text_bytes = Some(text - 1);
    assert!(matches!(
        execute_generic_propagation_ir(&ir, bad),
        Err(GenericVmError {
            kind: GenericVmErrorKind::Materialization(_),
            ..
        })
    ));
}
#[test]
fn propagation_vm_observes_cancellation_before_execution() {
    use sovra::compiler::execution::{CancellationToken, ExecutionControl};
    let ir=propagation_ir(&["fn root()->std::Option<Int>{let x=std::Option::<Int>::Some(1);return std::Option::<Int>::Some(x?);}"]);
    let token = CancellationToken::new();
    token.cancel();
    let policy = GenericVmPolicy {
        control: ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        },
        ..GenericVmPolicy::DEFAULT
    };
    assert!(matches!(
        execute_generic_propagation_ir(&ir, policy),
        Err(GenericVmError {
            kind: GenericVmErrorKind::Execution(_),
            ..
        })
    ));
}

#[test]
fn propagation_vm_keeps_unit_failure_payload_distinct_from_nullary_none() {
    let out=run_propagation(&["fn root()->std::Result<Bool,Unit>{let x=std::Result::<Int,Unit>::Err(unit());let n=x?;return std::Result::<Bool,Unit>::Ok(true);}","fn unit(){}"]);
    assert_eq!(out.value().enum_tag(), Some(1));
    assert!(out.value().payload().unwrap().is_unit());
}

#[test]
fn recursive_propagation_exits_each_frame_separately_and_caller_can_handle_error() {
    let out=run_propagation(&["fn root()->Int{let x=chain::<Bool>(12);match x{std::Result::Ok(n)=>{return 0;},std::Result::Err(e)=>{return 42;}}}","fn chain<T>(n:Int)->std::Result<Int,String>{if n==0{return std::Result::<Int,String>::Err(\"stop\");}return std::Result::<Int,String>::Ok(10+chain::<T>(n-1)?);}"]);
    assert_eq!(out.value().as_int(), Some(42));
    assert_eq!(out.peak_frames(), 14);
}

fn compile_postfix(source: &str) -> CompiledGenericSource {
    compile_generic_propagation_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
        .unwrap()
}
#[test]
fn whole_source_postfix_pipeline_executes_generics_aliases_and_original_unicode() {
    let source="// π and literal ? stay original\ntype Maybe<T>=std::Option<T>;\nfn take<T>(x:Maybe<T>)->Maybe<T>{return std::Option::<T>::Some(x?);}\nfn main()->Maybe<Int>{return take::<Int>(std::Option::<Int>::Some(9));}";
    let compiled = compile_postfix(source);
    assert_eq!(compiled.source(), source);
    assert!(compiled.propagation_ir().is_some());
    let out = execute_generic_source(&compiled, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.execution().value().payload().unwrap().as_int(), Some(9));
    for (index, f) in compiled.ir().functions().iter().enumerate() {
        for i in f.instructions() {
            if matches!(i.operation, GenericIrOperation::Propagate { .. }) {
                let span = compiled.file_span(index, i.span).unwrap();
                assert_eq!(&source[span.start..span.end], "?");
                assert_eq!(span.line, 2);
            }
        }
    }
    assert!(compile_generic_source(source, "main", GenericSourceLimits::DEFAULT).is_err());
}
#[test]
fn whole_source_postfix_maps_unused_body_error_to_exact_original_question() {
    let source = "// λ\nfn main(){}\nfn unused(x:std::Option<Int>)->Int{let n=x?;return 1;}";
    let error =
        compile_generic_propagation_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
            .unwrap_err();
    assert!(matches!(
        *error.kind,
        GenericSourceErrorKind::Bodies(GenericCopyBodyError::Propagation { .. })
    ));
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "?");
    assert_eq!(span.line, 2);
}
#[test]
fn whole_source_postfix_maps_runtime_error_through_specialized_function_origin() {
    let source="// Ω\nfn main()->std::Result<Bool,String>{return fail::<Int>();}\nfn fail<T>()->std::Result<Bool,String>{let x=std::Result::<Int,String>::Err(\"wide error\");let n=x?;return std::Result::<Bool,String>::Ok(true);}";
    let compiled = compile_postfix(source);
    let error = execute_generic_source(
        &compiled,
        GenericVmPolicy {
            runtime: sovra::compiler::execution::RuntimePolicy {
                materialization: sovra::compiler::execution::MaterializationLimits {
                    max_text_bytes: Some(
                        2 * ("wide error".len() + "std::Result".len() + "Err".len()) as u64,
                    ),
                    ..GenericVmPolicy::DEFAULT.runtime.materialization
                },
                ..GenericVmPolicy::DEFAULT.runtime
            },
            ..GenericVmPolicy::DEFAULT
        },
    )
    .unwrap_err();
    assert_eq!(
        error.span,
        compiled.file_span(error.error.function, error.error.span)
    );
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "?");
    assert_eq!(span.line, 2);
    assert_eq!(error.error.function, 1);
    let f = compiled
        .ir()
        .functions()
        .iter()
        .enumerate()
        .find(|(_, f)| {
            f.instructions()
                .iter()
                .any(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
        })
        .unwrap();
    let q =
        f.1.instructions()
            .iter()
            .find(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
            .unwrap();
    assert_eq!(
        &source[compiled.file_span(f.0, q.span).unwrap().start
            ..compiled.file_span(f.0, q.span).unwrap().end],
        "?"
    );
}
#[test]
fn whole_source_postfix_enforces_shared_site_operation_and_ir_admission() {
    let source="fn main()->std::Option<Int>{let x=std::Option::<std::Option<Int>>::Some(std::Option::<Int>::Some(3));return std::Option::<Int>::Some((x?)?);}";
    let d = GenericPropagationSourceLimits::DEFAULT;
    for limits in [
        GenericPropagationSourceLimits {
            propagation: GenericPropagationLimits {
                max_sites: 1,
                ..d.propagation
            },
            ..d
        },
        GenericPropagationSourceLimits {
            operations: GenericPropagationLayoutLimits {
                max_plans: 1,
                ..d.operations
            },
            ..d
        },
        GenericPropagationSourceLimits {
            source: GenericSourceLimits {
                ir: GenericIrLimits {
                    max_instructions: 0,
                    ..d.source.ir
                },
                ..d.source
            },
            ..d
        },
        GenericPropagationSourceLimits {
            source: GenericSourceLimits {
                syntax: GenericSourceSyntaxLimits {
                    max_tokens: 0,
                    ..d.source.syntax
                },
                ..d.source
            },
            ..d
        },
    ] {
        assert!(compile_generic_propagation_source(source, "main", limits).is_err());
    }
    assert!(compile_generic_propagation_source(
        source,
        "main",
        GenericPropagationSourceLimits {
            propagation: GenericPropagationLimits {
                max_sites: 2,
                ..d.propagation
            },
            operations: GenericPropagationLayoutLimits {
                max_plans: 2,
                ..d.operations
            },
            ..d
        }
    )
    .is_ok());
}
#[test]
fn whole_source_postfix_keeps_headers_and_nonfunction_declarations_strict() {
    for source in [
        "type Bad=std::Option<Int>?;fn main(){}",
        "struct Bad{value:Int?}fn main(){}",
        "enum Bad{Value(Int?)}fn main(){}",
        "fn bad(x:Int?){ }fn main(){}",
        "fn main()->Int?{return 1;}",
        "fn main(){let x=1??;}",
    ] {
        assert!(
            compile_generic_propagation_source(
                source,
                "main",
                GenericPropagationSourceLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
}
#[test]
fn whole_source_postfix_retains_unused_questions_and_uses_new_native_features() {
    let compiled = compile_postfix(
        "fn main(){}fn unused(x:std::Option<Int>)->std::Option<Int>{let n=x?;return x;}",
    );
    assert!(execute_generic_source(&compiled, GenericVmPolicy::DEFAULT)
        .unwrap()
        .execution()
        .value()
        .is_unit());
    assert!(matches!(
        execute_generic_ir(compiled.ir(), GenericVmPolicy::DEFAULT),
        Err(GenericVmError {
            kind: GenericVmErrorKind::PropagationUnavailable,
            ..
        })
    ));
    assert_eq!(
        &compiled.encode_generic_native_image().unwrap()[8..12],
        &15u32.to_le_bytes()
    );
    let plain = compile_postfix("fn main(){}");
    assert!(execute_generic_source(&plain, GenericVmPolicy::DEFAULT)
        .unwrap()
        .execution()
        .value()
        .is_unit());
    assert_eq!(
        &plain.encode_generic_native_image().unwrap()[8..12],
        &15u32.to_le_bytes()
    );
    let ordinary =
        compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
    assert!(ordinary.propagation_ir().is_none());
    assert!(ordinary.encode_generic_native_image().is_ok());
}

#[test]
fn propagated_failure_has_the_same_label_footprint_as_an_explicit_constructor() {
    let propagated=compile_postfix("fn main()->std::Result<Bool,String>{let x=std::Result::<Int,String>::Err(\"bad\");let n=x?;return std::Result::<Bool,String>::Ok(true);}");
    let direct = compile_generic_source(
        "fn main()->std::Result<Bool,String>{return std::Result::<Bool,String>::Err(\"bad\");}",
        "main",
        GenericSourceLimits::DEFAULT,
    )
    .unwrap();
    let a = execute_generic_source(&propagated, GenericVmPolicy::DEFAULT).unwrap();
    let b = execute_generic_source(&direct, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(
        a.execution().value().footprint(),
        b.execution().value().footprint()
    );
}

#[test]
fn native_postfix_feature_roundtrip_rechecks_source_and_rejects_downgrades() {
    let source="fn main(){let x=fail::<Int>();match x{std::Result::Ok(n)=>{let bad=1/0;},std::Result::Err(e)=>{}}}fn fail<T>()->std::Result<Bool,String>{let x=std::Result::<Int,String>::Err(\"failure\");let n=10+x?;return std::Result::<Bool,String>::Ok(1/0==0);}";
    let image = compile_postfix(source)
        .encode_generic_native_image()
        .unwrap();
    let decoded = decode_generic_native_image(&image).unwrap();
    assert_eq!(decoded.source(), source);
    assert!(decoded.propagation_ir().is_some());
    assert!(execute_generic_native_image(&image)
        .unwrap()
        .execution()
        .value()
        .is_unit());
    for features in [1u32, 3, 7, 8, 9, 11, 13, 14, 16, 23, u32::MAX] {
        let mut bad = image.clone();
        bad[8..12].copy_from_slice(&features.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err(), "{features}");
    }
    let invalid = source.replace("let n=10+x?;", "let n=10+true?;");
    let mut bad = image[..16].to_vec();
    bad[12..16].copy_from_slice(&(invalid.len() as u32).to_le_bytes());
    bad.extend_from_slice(invalid.as_bytes());
    assert!(decode_generic_native_image(&bad).is_err());
    for features in [1u32, 3, 7, 15] {
        let plain =
            compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
        let mut old = plain.encode_generic_native_image().unwrap();
        old[8..12].copy_from_slice(&features.to_le_bytes());
        assert!(execute_generic_native_image(&old).is_ok());
    }
}
