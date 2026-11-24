//! Complete opt-in symbolic postfix body checking; downstream execution remains gated.
use sovra::compiler::v2::*;
fn signatures(sources: &[&str]) -> CheckedGenericSignatures {
    let base = compile_generic_source(
        "type Maybe<T>=std::Option<T>;type Failure<T>=std::Result<T,std::Error>;fn main(){}",
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
        .types()
        .clone();
    let syntax: Vec<_> = sources
        .iter()
        .map(|source| {
            parse_generic_propagation_function(source, GenericFunctionLimits::DEFAULT).unwrap()
        })
        .collect();
    check_generic_signatures(
        &types,
        &syntax.iter().collect::<Vec<_>>(),
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap()
}
fn checked(sources: &[&str]) -> CheckedGenericCopyBodies {
    check_generic_propagation_bodies(
        &signatures(sources),
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn parameter_local_and_chained_payloads_produce_owned_postorder_facts() {
    let source="fn test(x:std::Option<std::Option<Int>>)->std::Option<Int>{let value=(x?)?;return std::Option::<Int>::Some(value);}";
    let bodies = checked(&[source]);
    assert_eq!(bodies.propagation_usage().unwrap().sites, 2);
    let body = &bodies.bodies()[0];
    let mut n = 0;
    for e in body.expressions() {
        if let GenericCopyExpressionKind::Propagate {
            operand,
            question,
            types,
        } = &e.kind
        {
            n += 1;
            assert!(operand.ordinal() < e.id.ordinal());
            assert_eq!(&source[question.start..question.end], "?");
            assert_eq!(types.kind(), GenericStandardEnum::Option);
            assert!(body.expression(*operand).is_some());
        }
    }
    assert_eq!(n, 2);
}
#[test]
fn calls_arithmetic_return_contexts_and_generic_error_identity_are_checked() {
    let sources=["fn get<T,E>(x:T)->std::Result<T,E>{return std::Result::<T,E>::Ok(x);}","fn test<E>(x:Int)->std::Result<Float,E>{let value=get::<Int,E>(x)?;return std::Result::<Float,E>::Ok(value+0.5);}"];
    let b = checked(&sources);
    assert_eq!(b.propagation_usage().unwrap().sites, 1);
    let f = &b.bodies()[1];
    assert!(f.expressions().iter().any(|e| matches!(
        e.kind,
        GenericCopyExpressionKind::Propagate { .. }
    ) && e.ty.primitive() == Some(GenericPrimitive::Int)));
}
#[test]
fn indexes_fields_branches_loops_and_match_payloads_use_active_scopes() {
    let source="fn test(xs:[std::Option<std::Error>],flag:std::Option<Bool>)->std::Option<String>{if flag?{let e=xs[0]?;return std::Option::<String>::Some(e.message);}while false{let e=xs[0]?;}let item=xs[0];match item{std::Option::Some(error)=>{let offset=error.offset;let value=offset?;return std::Option::<String>::Some(error.code);},std::Option::None()=>{return std::Option::<String>::None();}}}";
    assert_eq!(checked(&[source]).propagation_usage().unwrap().sites, 4);
}
#[test]
fn aliases_and_grouped_call_results_preserve_operand_spans() {
    let sources=["fn get(x:Maybe<Int>)->Maybe<Int>{return x;}","fn test(x:Maybe<Int>)->Maybe<Int>{let value=(get(x)?) + 1;return std::Option::<Int>::Some(value);}"];
    let b = checked(&sources);
    let body = &b.bodies()[1];
    let e = body
        .expressions()
        .iter()
        .find(|e| matches!(e.kind, GenericCopyExpressionKind::Propagate { .. }))
        .unwrap();
    assert_eq!(&sources[1][e.span.start..e.span.end], "(get(x)?)");
}
#[test]
fn wrong_error_types_unit_returns_primitive_operands_and_shadowing_refuse() {
    for source in ["fn test(x:std::Result<Int,Int>)->std::Result<Int,Float>{let value=x?;return std::Result::<Int,Float>::Ok(value);}","fn test(x:std::Option<Int>){let value=x?;}","fn test()->std::Option<Int>{let value=1?;return std::Option::<Int>::Some(value);}","fn test(x:std::Option<Int>)->std::Option<Int>{let x=1;let value=x?;return std::Option::<Int>::Some(value);}","fn test(x:std::Option<Int>)->std::Option<Int>{let value=x?;}" ] {
        assert!(check_generic_propagation_bodies(&signatures(&[source]),GenericCopyBodyLimits::DEFAULT,GenericPropagationLimits::DEFAULT).is_err(),"{source}");
    }
}
#[test]
fn invalid_unused_and_unreachable_body_code_is_not_skipped() {
    for source in [
        "fn test(x:std::Option<Int>)->std::Option<Int>{return x;let bad=missing?;}",
        "fn test(x:std::Option<Int>)->std::Option<Int>{let value=x?;let bad:Bool=1;return x;}",
    ] {
        assert!(check_generic_propagation_bodies(
            &signatures(&[source]),
            GenericCopyBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT
        )
        .is_err());
    }
}
#[test]
fn naked_callable_heads_and_type_position_questions_never_become_partial_bodies() {
    for source in [
        "fn test(x:std::Option<Int>)->std::Option<Int>{let value=helper::<Int?>(x);return x;}",
        "fn test(x:std::Option<Int>)->std::Option<Int>{let value=(x?)();return x;}",
    ] {
        assert!(check_generic_propagation_bodies(
            &signatures(&[source]),
            GenericCopyBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT
        )
        .is_err());
    }
}
#[test]
fn specialization_refuses_all_propagation_tables_but_accepts_an_empty_feature_set() {
    let b = checked(&[
        "fn root(){}",
        "fn test(x:std::Option<Int>)->std::Option<Int>{let value=x?;return x;}",
    ]);
    let root = b.signatures().find("root").unwrap();
    assert!(matches!(
        discover_generic_specializations(&b, root, GenericSpecializationLimits::DEFAULT),
        Err(GenericSpecializationError::PropagationUnavailable)
    ));
    let b = checked(&["fn root()->Int{return 1;}"]);
    let root = b.signatures().find("root").unwrap();
    let graph =
        discover_generic_specializations(&b, root, GenericSpecializationLimits::DEFAULT).unwrap();
    recheck_generic_concrete_bodies(&graph, GenericConcreteBodyLimits::DEFAULT).unwrap();
}
#[test]
fn strict_function_body_source_and_native_entry_points_stay_disabled() {
    let source = "fn test(x:std::Option<Int>)->std::Option<Int>{let value=x?;return x;}";
    assert!(parse_specialization_root(source, GenericFunctionLimits::DEFAULT).is_err());
    let signatures = signatures(&[source]);
    assert!(check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).is_err());
    assert!(compile_generic_source(
        &format!("{source}fn main(){{}}"),
        "main",
        GenericSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn propagation_allowances_are_cumulative_across_functions() {
    let sources = [
        "fn a(x:std::Result<Int,[String]>)->std::Result<Int,[String]>{let value=x?;return x;}",
        "fn b(x:std::Result<Int,[String]>)->std::Result<Int,[String]>{let value=x?;return x;}",
    ];
    let sig = signatures(&sources);
    let usage = check_generic_propagation_bodies(
        &sig,
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap()
    .propagation_usage()
    .unwrap();
    macro_rules! boundary {
        ($field:ident,$cost:ident) => {{
            let mut p = GenericPropagationLimits::DEFAULT;
            p.$field = usage.$cost;
            check_generic_propagation_bodies(&sig, GenericCopyBodyLimits::DEFAULT, p).unwrap();
            p.$field -= 1;
            assert!(
                check_generic_propagation_bodies(&sig, GenericCopyBodyLimits::DEFAULT, p).is_err(),
                stringify!($field)
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
fn indexed_assignment_operands_bind_once_in_the_active_scope() {
    let source="fn test(index:std::Option<Int>,value:std::Option<Int>)->std::Option<Unit>{let mut items:[Int]=[0];items[index?]=value?;return std::Option::<Unit>::None();}";
    let b = checked(&[source]);
    assert_eq!(b.propagation_usage().unwrap().sites, 2);
    assert!(b.bodies()[0]
        .statements()
        .iter()
        .any(|s| matches!(s, GenericCopyStatement::IndexAssign { .. })));
}
#[test]
fn virtual_question_nodes_participate_in_the_expression_depth_ceiling() {
    for (depth, accepted) in [(125, true), (126, false)] {
        let argument = format!("{}Int{}", "std::Option<".repeat(depth), ">".repeat(depth));
        let source = format!(
            "fn test(x:{argument})->std::Option<Int>{{return std::Option::<Int>::Some(x{}+1);}}",
            "?".repeat(depth)
        );
        let result = check_generic_propagation_bodies(
            &signatures(&[&source]),
            GenericCopyBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        );
        assert_eq!(result.is_ok(), accepted, "{depth}: {result:?}");
    }
}
#[test]
fn body_admission_includes_virtual_nodes_and_failure_type_comparisons() {
    let sig = signatures(&[
        "fn test(x:std::Result<Int,[String]>)->std::Result<Int,[String]>{let value=x?;return x;}",
    ]);
    let b = check_generic_propagation_bodies(
        &sig,
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap();
    macro_rules! boundary {
        ($field:ident,$value:expr) => {{
            let mut l = GenericCopyBodyLimits::DEFAULT;
            l.$field = $value;
            check_generic_propagation_bodies(&sig, l, GenericPropagationLimits::DEFAULT).unwrap();
            l.$field -= 1;
            assert!(
                check_generic_propagation_bodies(&sig, l, GenericPropagationLimits::DEFAULT)
                    .is_err(),
                stringify!($field)
            );
        }};
    }
    boundary!(max_ast_nodes, b.ast_nodes());
    boundary!(max_expressions, b.expression_count());
    boundary!(max_type_pairs, b.type_pairs());
    boundary!(max_type_edges, b.type_edges());
    boundary!(max_work, b.work());
}

#[test]
fn semantic_refusals_retain_question_and_operand_source_provenance() {
    let source="fn test(x:std::Option<Int>)->std::Option<Int>{\n// λ\nlet x=1;\nlet value=x?;return std::Option::<Int>::Some(value);}";
    let sig = signatures(&[source]);
    let error = check_generic_propagation_bodies(
        &sig,
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap_err();
    let GenericCopyBodyError::Propagation {
        function,
        span,
        error,
    } = error
    else {
        panic!("expected propagation refusal")
    };
    assert_eq!(function, sig.find("test").unwrap());
    assert_eq!(span.start, source.find('?').unwrap());
    assert_eq!(span.line, 3);
    let GenericPropagationError::Operand {
        function: owner,
        span: operand,
    } = *error
    else {
        panic!("expected primitive operand refusal")
    };
    assert_eq!(owner, function);
    assert_eq!(&source[operand.start..operand.end], "x");
}

#[test]
fn postfix_header_adapter_keeps_headers_strict_and_body_syntax_opaque() {
    for source in [
        "fn test<T?>(){}",
        "fn test(x:Int?){}",
        "fn test()->std::Option<Int?>{}",
        "fn test(){}?",
    ] {
        assert!(
            parse_generic_propagation_function(source, GenericFunctionLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
    let source = "fn test()->std::Option<Int>{let value=missing?;}";
    parse_generic_propagation_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    assert!(check_generic_propagation_bodies(
        &signatures(&[source]),
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT
    )
    .is_err());
}
