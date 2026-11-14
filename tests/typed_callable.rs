//! Unit-owned expression types, array conversions and bounded typed evidence.
use sovra::compiler::{
    checked::CoercionKind,
    parser::Parser,
    semantic::Type,
    v2::{
        check_enum_types, CallableLimits, CallableTarget, CheckedEnumTypes,
        EnumFunctionError as Error, EnumSource, EnumTypeLimits,
        ExpressionEvidenceLimits as Expressions, TypedCallableLimits as Limits,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "types",
        &Parser::new().parse_source("").unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{V(Float),Rows([[Float]]),Empty}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn expression_types_context_and_call_links_are_source_owned_and_nominal() {
    let env = env();
    let source = "fn f()->E{return E::V(1);}";
    let checked = env
        .check_typed_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let types = checked.types();
    let callable = checked.callable().evidence();
    assert_eq!(types.expressions().len(), 2);
    assert_eq!(types.expressions()[0].ty, Type::Int);
    assert_eq!(types.expressions()[1].ty, Type::Named("E".into()));
    assert_eq!(
        &source[types.expressions()[1].span.start..types.expressions()[1].span.end],
        "E::V(1)"
    );
    assert_eq!(types.contexts().len(), callable.contexts().len());
    for link in types.contexts() {
        let fact = types.expression(link.value).unwrap();
        assert_eq!(fact.span, callable.contexts()[link.context].value_span);
        assert_eq!(fact.ty, callable.contexts()[link.context].actual);
    }
    assert_eq!(types.calls().len(), 1);
    let link = types.calls()[0];
    assert_eq!(
        types.expression(link.result).unwrap().ty,
        Type::Named("E".into())
    );
    assert!(matches!(
        callable.calls()[link.call].target,
        CallableTarget::Constructor { .. }
    ));
}
#[test]
fn mixed_arrays_keep_int_to_float_element_contexts_and_recursive_parent_contexts() {
    let env = env();
    let source = "fn f()->E{return E::Rows([[1,2.0],[3,4]]);}";
    let checked = env
        .check_typed_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let contexts = checked.callable().evidence().contexts();
    let scalar = contexts
        .iter()
        .find(|c| {
            c.kind == CoercionKind::ArrayElement
                && c.actual == Type::Int
                && c.expected == Type::Float
        })
        .unwrap();
    assert_eq!(&source[scalar.value_span.start..scalar.value_span.end], "1");
    let recursive = contexts
        .iter()
        .find(|c| {
            c.kind == CoercionKind::ArrayElement
                && c.actual == Type::Array(Box::new(Type::Int))
                && c.expected == Type::Array(Box::new(Type::Float))
        })
        .unwrap();
    assert_eq!(
        &source[recursive.value_span.start..recursive.value_span.end],
        "[3,4]"
    );
    assert_eq!(checked.types().contexts().len(), contexts.len());
}
#[test]
fn empty_array_holes_remain_explicit_beside_the_required_context() {
    let checked = env()
        .check_typed_callable_function_source(
            "f",
            "fn f()->E{return E::Rows([]);}",
            Limits::DEFAULT,
        )
        .unwrap();
    let contexts = checked.callable().evidence().contexts();
    let context = contexts
        .iter()
        .find(|c| c.kind == CoercionKind::Argument)
        .unwrap();
    assert_eq!(context.actual, Type::Array(Box::new(Type::Unknown)));
    assert_eq!(
        context.expected,
        Type::Array(Box::new(Type::Array(Box::new(Type::Float))))
    );
    assert!(checked
        .types()
        .expressions()
        .iter()
        .any(|f| f.ty == Type::Array(Box::new(Type::Unknown))));
}
#[test]
fn identical_sources_have_distinct_expression_ids_and_clones_retain_their_ids() {
    let env = env();
    let source = "fn f()->E{return E::V(1);}";
    let a = env
        .check_typed_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let b = env
        .check_typed_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let id = a.types().expressions()[0].id;
    assert_ne!(id, b.types().expressions()[0].id);
    assert!(b.types().expression(id).is_none());
    assert!(a.clone().types().expression(id).is_some());
}
#[test]
fn expression_allowances_are_exact_zero_is_real_and_refusals_are_fresh() {
    let env = env();
    let source = "fn f()->E{return E::V(1);}";
    let exact = Limits {
        expressions: Expressions {
            max_expressions: 2,
            max_type_nodes: 2,
            max_type_text_bytes: 1,
        },
        ..Limits::DEFAULT
    };
    assert!(env
        .check_typed_callable_function_source("f", source, exact)
        .is_ok());
    for expressions in [
        Expressions {
            max_expressions: 1,
            ..exact.expressions
        },
        Expressions {
            max_type_nodes: 1,
            ..exact.expressions
        },
        Expressions {
            max_type_text_bytes: 0,
            ..exact.expressions
        },
    ] {
        assert!(matches!(
            env.check_typed_callable_function_source(
                "f",
                source,
                Limits {
                    expressions,
                    ..exact
                }
            ),
            Err(Error::Limit(_))
        ));
        assert!(env
            .check_typed_callable_function_source("f", source, exact)
            .is_ok());
    }
    assert!(env
        .check_typed_callable_function_source(
            "f",
            "fn f(){}",
            Limits {
                expressions: Expressions {
                    max_expressions: 0,
                    max_type_nodes: 0,
                    max_type_text_bytes: 0
                },
                ..Limits::DEFAULT
            }
        )
        .is_ok());
}
#[test]
fn array_contexts_share_callable_quotas_while_older_api_limits_stay_unchanged() {
    let env = env();
    let source = "fn f(){let a=[1,2.0];}";
    let callable = CallableLimits {
        max_contexts: 1,
        ..CallableLimits::DEFAULT
    };
    let old = env
        .check_callable_function_source("f", source, callable)
        .unwrap();
    assert_eq!(old.evidence().contexts().len(), 1);
    assert!(matches!(
        env.check_typed_callable_function_source(
            "f",
            source,
            Limits {
                callable,
                ..Limits::DEFAULT
            }
        ),
        Err(Error::Limit(_))
    ));
    let checked = env
        .check_typed_callable_function_source(
            "f",
            source,
            Limits {
                callable: CallableLimits {
                    max_contexts: 3,
                    max_type_nodes: 8,
                    ..callable
                },
                expressions: Expressions {
                    max_expressions: 3,
                    max_type_nodes: 4,
                    max_type_text_bytes: 0,
                },
            },
        )
        .unwrap();
    assert_eq!(checked.callable().evidence().contexts().len(), 3);
    assert!(matches!(
        env.check_typed_callable_function_source(
            "f",
            source,
            Limits {
                callable: CallableLimits {
                    max_contexts: 3,
                    max_type_nodes: 7,
                    ..callable
                },
                ..Limits::DEFAULT
            }
        ),
        Err(Error::Limit(_))
    ));
}
#[test]
fn nested_standard_self_and_constructor_calls_link_results_without_typing_heads_as_values() {
    let env = env();
    let source = "fn f(n:Int)->E{if n>0{return f(n-1);}else{print(E::V(n));return E::Empty();}}";
    let checked = env
        .check_typed_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    assert_eq!(
        checked.types().calls().len(),
        checked.callable().evidence().calls().len()
    );
    for link in checked.types().calls() {
        assert!(checked.types().expression(link.result).is_some());
        let call = &checked.callable().evidence().calls()[link.call];
        assert!(!checked
            .types()
            .expressions()
            .iter()
            .any(|e| e.span == call.span));
    }
}
#[test]
fn typed_arm_values_indexed_assignment_and_original_crlf_spans_are_preserved() {
    let source="// λ😀\r\nfn f(input:E)->Float{let mut rows=[1.0];rows[0]=2;match input{E::V(n)=>{return n;},E::Rows(r)=>{return r[0][0];},E::Empty()=>{return rows[0];}}}";
    let checked = env()
        .check_typed_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    assert!(checked
        .types()
        .expressions()
        .iter()
        .any(|e| &source[e.span.start..e.span.end] == "n" && e.ty == Type::Float));
    let indexed = checked
        .callable()
        .evidence()
        .contexts()
        .iter()
        .find(|c| c.kind == CoercionKind::IndexedAssignment)
        .unwrap();
    assert_eq!(
        &source[indexed.value_span.start..indexed.value_span.end],
        "2"
    );
    assert_eq!(indexed.value_span.line, 1);
    assert_eq!(
        checked.types().contexts().len(),
        checked.callable().evidence().contexts().len()
    );
}
#[test]
fn semantic_errors_take_precedence_over_type_retention_exhaustion() {
    let error = env()
        .check_typed_callable_function_source(
            "f",
            "fn f()->E{return E::V(true);}",
            Limits {
                expressions: Expressions {
                    max_expressions: 0,
                    ..Expressions::DEFAULT
                },
                ..Limits::DEFAULT
            },
        )
        .unwrap_err();
    let Error::Diagnostics(errors) = error else {
        panic!("expected semantic diagnostic")
    };
    assert!(errors.items.iter().any(|e| e.code == "E3007"));
}
