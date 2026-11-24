//! Original source generic calls checked in active lexical scopes; no execution.
use sovra::compiler::v2::*;
fn table(sources: &[&str]) -> CheckedGenericSignatures {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types = check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
    let functions: Vec<_> = sources
        .iter()
        .map(|s| parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap())
        .collect();
    let refs: Vec<_> = functions.iter().collect();
    check_generic_signatures(&types, &refs, GenericSignatureLimits::DEFAULT).unwrap()
}
fn check(sources: &[&str]) -> CheckedGenericCopyBodies {
    check_generic_copy_bodies(&table(sources), GenericCopyBodyLimits::DEFAULT).unwrap()
}
const IDENTITY: &str = "fn identity<T>(value:T)->T{return value;}";
#[test]
fn source_call_facts_retain_original_type_value_and_callee_identity() {
    let source = "// λ😀\nfn caller<U>(x:U)->U{return identity::<U>(x);}";
    let checked = check(&[source, IDENTITY]);
    let body = &checked.bodies()[0];
    let e = body.expressions().last().unwrap();
    assert_eq!(&source[e.span.start..e.span.end], "identity::<U>(x)");
    let GenericCopyExpressionKind::Call {
        signature,
        callee_span,
        type_range,
        arguments,
    } = &e.kind
    else {
        panic!()
    };
    assert_eq!(&source[callee_span.start..callee_span.end], "identity");
    assert_eq!(&source[type_range.start..type_range.end], "::<U>");
    assert_eq!(
        signature.binding().callee(),
        checked.signatures().find("identity").unwrap()
    );
    assert_eq!(arguments[0].conversion, GenericCopyConversion::Identity);
    assert!(
        matches!(body.expression(arguments[0].expression).unwrap().kind,GenericCopyExpressionKind::Read(id) if id==body.bindings()[0].id)
    );
    let ty = e.ty.expanded().unwrap();
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id) if id==body.signature().syntax().parameters().find("U").unwrap())
    );
}
#[test]
fn nested_calls_and_contextual_arrays_use_the_original_expression_worklist() {
    let checked = check(&[
        "fn caller<U>(x:U)->[U]{return identity::<[U]>([identity::<U>(x)]);}",
        IDENTITY,
    ]);
    assert_eq!(checked.call_usage().calls, 2);
    let body = &checked.bodies()[0];
    let calls = body
        .expressions()
        .iter()
        .filter(|e| matches!(e.kind, GenericCopyExpressionKind::Call { .. }))
        .count();
    assert_eq!(calls, 2);
    assert!(
        !check(&["fn caller<U>()->[U]{return identity::<[U]>([]);}", IDENTITY]).bodies()[0]
            .implicit_unit_return()
    );
}
#[test]
fn active_local_and_parameter_shadowing_block_root_callees_and_restore_after_branches() {
    for source in [
        "fn caller<U>(identity:U,x:U)->U{return identity::<U>(x);}",
        "fn caller<U>(x:U)->U{let identity=x;return identity::<U>(x);}",
        "fn caller<U>(x:U)->U{if true{let identity=x;identity::<U>(x);}return x;}",
    ] {
        assert!(check_generic_copy_bodies(
            &table(&[source, IDENTITY]),
            GenericCopyBodyLimits::DEFAULT
        )
        .is_err());
    }
    assert!(!check(&[
        "fn caller<U>(x:U)->U{if false{let identity=x;}return identity::<U>(x);}",
        IDENTITY
    ])
    .bodies()[0]
        .implicit_unit_return());
}
#[test]
fn explicit_types_counts_bounds_and_all_unused_bodies_are_checked() {
    let keyed = "fn keyed<T:std::Key>(x:T)->T{return x;}";
    for source in [
        "fn caller<U>(x:U)->U{return keyed::<U>(x);}",
        "fn caller<U>(x:U)->U{return keyed::<Float>(1.0);}",
        "fn caller<U>(x:U)->U{return keyed::<U,U>(x);}",
        "fn caller<U>(x:U)->U{return keyed::<Int>();}",
        "fn caller<U>(x:U)->U{return keyed(x);}",
        "fn caller<U>(x:U)->U{return missing::<U>(x);}",
        "fn caller<U>(x:U)->U{return x;keyed::<Missing>(x);}",
    ] {
        assert!(
            check_generic_copy_bodies(&table(&[source, keyed]), GenericCopyBodyLimits::DEFAULT)
                .is_err(),
            "{source}"
        );
    }
    assert!(!check(&[
        "fn caller<U:std::Key>(x:U)->U{return keyed::<U>(x);}",
        keyed
    ])
    .bodies()[0]
        .implicit_unit_return());
    assert!(check_generic_copy_bodies(
        &table(&[
            "fn caller<U>(x:U)->U{return identity::<U>(x);}",
            IDENTITY,
            "fn unused<U>(){missing::<U>();}"
        ]),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn scalar_widening_is_explicit_but_existing_aggregate_values_are_not_coerced() {
    let checked = check(&[
        "fn caller<U>()->Float{return target::<U>(1);}",
        "fn target<T>(x:Float)->Float{return x;}",
    ]);
    let body = &checked.bodies()[0];
    let GenericCopyExpressionKind::Call { arguments, .. } =
        &body.expressions().last().unwrap().kind
    else {
        panic!()
    };
    assert_eq!(arguments[0].conversion, GenericCopyConversion::IntToFloat);
    assert!(check_generic_copy_bodies(
        &table(&[
            "fn caller<U>(x:[Int])->[Float]{return identity::<[Float]>(x);}",
            IDENTITY
        ]),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn recursion_is_symbolically_checked_without_executing_or_specializing() {
    let checked = check(&["fn recurse<T>(x:T)->T{return recurse::<T>(x);}"]);
    assert_eq!(checked.call_usage().calls, 1);
    assert!(!checked.bodies()[0].implicit_unit_return());
    assert_eq!(
        check(&["fn grow<T>(x:T){grow::<[T]>([x]);}"])
            .call_usage()
            .calls,
        1
    );
    assert!(check_program_source(
        "strict",
        "fn identity<T>(x:T)->T{return x;}fn main(){identity::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn cumulative_source_input_generated_and_value_call_budgets_are_exact() {
    let table = table(&[
        "fn caller<U>(x:U)->[U]{return identity::<[U]>([identity::<U>(x)]);}",
        IDENTITY,
    ]);
    let checked = check_generic_copy_bodies(&table, GenericCopyBodyLimits::DEFAULT).unwrap();
    let usage = checked.call_usage();
    let calls = GenericBodyCallLimits {
        max_calls: usage.calls,
        max_values: usage.values,
        max_nodes: usage.nodes,
        max_edges: usage.edges,
        arguments: GenericArgumentCheckLimits {
            max_arguments: usage.type_arguments,
            max_source_bytes: usage.source_bytes,
            max_tokens: usage.tokens,
            max_input_nodes: usage.input_nodes,
            max_input_edges: usage.input_edges,
            max_name_bytes: usage.name_bytes,
            ..GenericArgumentCheckLimits::DEFAULT
        },
        ..GenericBodyCallLimits::DEFAULT
    };
    let exact = GenericCopyBodyLimits {
        max_work: checked.work(),
        calls,
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&table, exact).is_ok());
    for reduced in [
        GenericBodyCallLimits {
            max_calls: calls.max_calls - 1,
            ..calls
        },
        GenericBodyCallLimits {
            max_values: calls.max_values - 1,
            ..calls
        },
        GenericBodyCallLimits {
            max_nodes: calls.max_nodes - 1,
            ..calls
        },
        GenericBodyCallLimits {
            max_edges: calls.max_edges - 1,
            ..calls
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_arguments: usage.type_arguments - 1,
                ..calls.arguments
            },
            ..calls
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_source_bytes: usage.source_bytes - 1,
                ..calls.arguments
            },
            ..calls
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_tokens: usage.tokens - 1,
                ..calls.arguments
            },
            ..calls
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_input_nodes: usage.input_nodes - 1,
                ..calls.arguments
            },
            ..calls
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_input_edges: usage.input_edges - 1,
                ..calls.arguments
            },
            ..calls
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_name_bytes: usage.name_bytes - 1,
                ..calls.arguments
            },
            ..calls
        },
    ] {
        assert!(
            check_generic_copy_bodies(
                &table,
                GenericCopyBodyLimits {
                    calls: reduced,
                    ..exact
                }
            )
            .is_err(),
            "{reduced:?}"
        );
    }
    assert!(check_generic_copy_bodies(
        &table,
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        }
    )
    .is_err());
}
#[test]
fn speculative_index_target_parsing_does_not_duplicate_call_metadata() {
    let checked = check(&[
        "fn caller<U>(xs:[U])->U{return xs[index::<U>()];}",
        "fn index<T>()->Int{return 0;}",
    ]);
    assert_eq!(checked.call_usage().calls, 1);
    let checked = check(&[
        "fn caller<U>(xs:[U]){xs[index::<U>()];}",
        "fn index<T>()->Int{return 0;}",
    ]);
    assert_eq!(checked.call_usage().calls, 1);
}

#[test]
fn call_budgets_do_not_reset_between_functions_and_inner_type_diagnostics_keep_sources() {
    let signatures = table(&[
        "fn first<T>(x:T)->T{return identity::<T>(x);}",
        "fn second<U>(x:U)->U{return identity::<U>(x);}",
        IDENTITY,
    ]);
    assert!(check_generic_copy_bodies(
        &signatures,
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_calls: 1,
                ..GenericBodyCallLimits::DEFAULT
            },
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    let source = "// λ😀\nfn caller<U>(x:U)->U{return identity::<U,>(x);}";
    let GenericCopyBodyError::Call {
        type_range, error, ..
    } = check_generic_copy_bodies(&table(&[source, IDENTITY]), GenericCopyBodyLimits::DEFAULT)
        .unwrap_err()
    else {
        panic!()
    };
    let fragment = &source[type_range.start..type_range.end];
    assert_eq!(fragment, "::<U,>");
    let GenericBodyCallError::Syntax(GenericTypeError::Syntax { span, .. }) = *error else {
        panic!()
    };
    assert_eq!(&fragment[span.start..span.end], ">");
}
#[test]
fn qualified_calls_remain_refused_and_deep_speculative_targets_hit_parser_limits() {
    assert!(check_generic_copy_bodies(
        &table(&[
            "fn caller<U>(x:U)->U{return module::identity::<U>(x);}",
            IDENTITY
        ]),
        GenericCopyBodyLimits::DEFAULT
    )
    .is_err());
    let source = format!(
        "fn caller<T>(xs:[T]){{xs{};}}",
        "[index::<T>()]".repeat(140)
    );
    assert!(matches!(
        check_generic_copy_bodies(
            &table(&[&source, "fn index<T>()->Int{return 0;}"]),
            GenericCopyBodyLimits::DEFAULT
        ),
        Err(GenericCopyBodyError::Parse { .. })
    ));
}
#[test]
fn source_call_pipeline_honors_each_independent_stage_profile() {
    let signatures = table(&[
        "fn caller<T>(x:T)->[T]{return identity::<[T]>([x]);}",
        IDENTITY,
    ]);
    let base = GenericBodyCallLimits::DEFAULT;
    for calls in [
        GenericBodyCallLimits {
            syntax: GenericTypeArgumentLimits {
                types: GenericTypeLimits {
                    max_depth: 1,
                    ..base.syntax.types
                },
                ..base.syntax
            },
            ..base
        },
        GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                expansion: GenericAliasExpansionLimits {
                    max_nodes: 0,
                    ..base.arguments.expansion
                },
                ..base.arguments
            },
            ..base
        },
        GenericBodyCallLimits {
            binding: GenericCallBindingLimits {
                max_bindings: 0,
                ..base.binding
            },
            ..base
        },
        GenericBodyCallLimits {
            signature: GenericSignatureInstantiationLimits {
                max_annotations: 0,
                ..base.signature
            },
            ..base
        },
        GenericBodyCallLimits {
            signature: GenericSignatureInstantiationLimits {
                max_nodes_per_type: 1,
                ..base.signature
            },
            ..base
        },
    ] {
        assert!(check_generic_copy_bodies(
            &signatures,
            GenericCopyBodyLimits {
                calls,
                ..GenericCopyBodyLimits::DEFAULT
            }
        )
        .is_err());
    }
}

#[test]
fn local_name_uses_preserve_shadowing_and_grouped_token_ranges() {
    let source = "fn f<T>(x:Int)->Int{let y=x;if true{let x=y;let z=(x);}let y=y+1;return y;}";
    let checked = check(&[source]);
    let body = &checked.bodies()[0];
    let ids: Vec<_> = body
        .name_uses()
        .iter()
        .map(|usage| {
            let binding = body.binding(usage.binding).unwrap();
            assert_eq!(
                &source[usage.span.start..usage.span.end],
                binding.name.as_ref()
            );
            usage.binding.ordinal()
        })
        .collect();
    assert_eq!(ids, [0, 1, 2, 1, 4]);
}

#[test]
fn writes_and_indexed_writes_retain_only_receiver_name_tokens() {
    let source = "fn f<T>(){let mut x=0;x=x+1;let mut xs:[Int]=[x];xs[x]=x;}";
    let checked = check(&[source]);
    let body = &checked.bodies()[0];
    let mut names: Vec<_> = body
        .name_uses()
        .iter()
        .map(|u| {
            assert!(body.binding(u.binding).is_some());
            &source[u.span.start..u.span.end]
        })
        .collect();
    names.sort();
    assert_eq!(names, ["x", "x", "x", "x", "x", "xs"]);
}
