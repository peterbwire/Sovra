//! Source constructor/self-call checking with bounded resolved call/context facts.
use sovra::compiler::{
    checked::CoercionKind,
    parser::Parser,
    semantic::Type,
    v2::{
        check_enum_types, CallableLimits as Limits, CallableTarget as Target, CheckedEnumTypes,
        EnumFunctionError as Error, EnumSource, EnumTypeLimits,
    },
};
fn env(declaration: &str) -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "types",
        &Parser::new().parse_source("").unwrap(),
        &[EnumSource {
            label: "enum",
            source: declaration,
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
fn codes(error: Error) -> Vec<&'static str> {
    let Error::Diagnostics(errors) = error else {
        panic!("expected core diagnostics: {error:?}")
    };
    errors.items.into_iter().map(|e| e.code).collect()
}
#[test]
fn constructors_can_initialize_locals_feed_matches_and_return_nominal_values() {
    let env = env("enum E{V(Float),Empty}");
    let source="fn f()->Float{let value=E::V(1);match value{E::V(n)=>{return n;},E::Empty()=>{return 0;}}}";
    let checked = env
        .check_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    assert!(checked.function().guarantees_return());
    assert_eq!(checked.evidence().calls().len(), 1);
    let call = checked.evidence().calls()[0];
    assert_eq!(&source[call.span.start..call.span.end], "E::V");
    assert_eq!(
        call.target,
        Target::Constructor {
            enum_id: env.find("E").unwrap(),
            variant: 0
        }
    );
    assert!(checked
        .evidence()
        .contexts()
        .iter()
        .any(|c| c.kind == CoercionKind::Argument
            && c.actual == Type::Int
            && c.expected == Type::Float));
    assert!(env
        .check_callable_function_source("f", "fn f()->E{return E::Empty();}", Limits::DEFAULT)
        .is_ok());
}
#[test]
fn recursive_calls_use_current_source_signature_and_record_self_identity() {
    let env = env("enum E{V(Int)}");
    let source = "fn count(n:Int)->E{if n>0{return count(n-1);}else{return E::V(n);}}";
    let checked = env
        .check_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    assert!(checked
        .evidence()
        .calls()
        .iter()
        .any(|c| c.target == Target::SelfFunction));
    assert!(checked
        .evidence()
        .calls()
        .iter()
        .any(|c| matches!(c.target, Target::Constructor { .. })));
    assert!(codes(
        env.check_callable_function_source(
            "f",
            "fn f(n:Int)->Int{return f(true);}",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3007"));
    assert!(env
        .check_callable_function_source("f", "fn f()->Int{return other();}", Limits::DEFAULT)
        .is_err());
}
#[test]
fn nullary_unit_arity_and_payload_type_errors_are_preserved() {
    let env = env("enum E{Empty,Unit(Unit),V(Int)}");
    assert!(env
        .check_callable_function_source(
            "f",
            "fn f()->E{return E::Unit(print(1));}",
            Limits::DEFAULT
        )
        .is_ok());
    for (source, code) in [
        ("fn f()->E{return E::Empty(1);}", "E3006"),
        ("fn f()->E{return E::Unit();}", "E3006"),
        ("fn f()->E{return E::V();}", "E3006"),
        ("fn f()->E{return E::V(1,2);}", "E3006"),
        ("fn f()->E{return E::V(true);}", "E3007"),
        ("fn f()->E{return E::Missing();}", "E3004"),
    ] {
        assert!(
            codes(
                env.check_callable_function_source("f", source, Limits::DEFAULT)
                    .unwrap_err()
            )
            .contains(&code),
            "{source}"
        );
    }
}
#[test]
fn array_payload_widening_retains_recursive_type_context() {
    let env = env("enum E{Rows([[Float]])}");
    let checked = env
        .check_callable_function_source("f", "fn f()->E{return E::Rows([[1,2]]);}", Limits::DEFAULT)
        .unwrap();
    let context = checked
        .evidence()
        .contexts()
        .iter()
        .find(|c| c.kind == CoercionKind::Argument)
        .unwrap();
    assert_eq!(
        context.actual,
        Type::Array(Box::new(Type::Array(Box::new(Type::Int))))
    );
    assert_eq!(
        context.expected,
        Type::Array(Box::new(Type::Array(Box::new(Type::Float))))
    );
    assert!(env
        .check_callable_function_source("f", "fn f()->E{return E::Rows([]);}", Limits::DEFAULT)
        .is_ok());
    assert!(env
        .check_callable_function_source(
            "f",
            "fn f()->E{return E::Rows([[true]]);}",
            Limits::DEFAULT
        )
        .is_err());
}
#[test]
fn nominal_enums_and_record_payloads_keep_distinct_identities() {
    let env = check_enum_types(
        "owner",
        "types",
        &Parser::new().parse_source("struct Item{x:Int}").unwrap(),
        &[
            EnumSource {
                label: "e",
                source: "enum E{V(Item)}",
            },
            EnumSource {
                label: "f",
                source: "enum F{V(Item)}",
            },
        ],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap();
    assert!(env
        .check_callable_function_source("f", "fn f()->E{return E::V(Item{x:1});}", Limits::DEFAULT)
        .is_ok());
    assert!(env
        .check_callable_function_source("f", "fn f()->E{return F::V(Item{x:1});}", Limits::DEFAULT)
        .is_err());
    assert!(env
        .check_callable_function_source("f", "fn f()->E{return E{V:Item{x:1}};}", Limits::DEFAULT)
        .is_err());
}
#[test]
fn standard_aliases_and_constructor_calls_in_arm_bodies_are_resolved() {
    let env = env("enum E{V(Int),Empty}");
    let checked=env.check_callable_function_source("f","fn f(input:E)->E{match input{E::V(n)=>{print(n);return E::V(n);},E::Empty()=>{return E::Empty();}}}",Limits::DEFAULT).unwrap();
    assert!(checked
        .evidence()
        .calls()
        .iter()
        .any(|c| c.target == Target::Standard("std::print")));
    assert_eq!(
        checked
            .evidence()
            .calls()
            .iter()
            .filter(|c| matches!(c.target, Target::Constructor { .. }))
            .count(),
        2
    );
}
#[test]
fn old_source_api_remains_standard_only_and_no_local_value_captures_call_targets() {
    let env = env("enum E{V(Int)}");
    let source = "fn f()->E{let f=3;return E::V(1);}";
    assert!(env
        .check_function_source("f", source, Limits::DEFAULT.function)
        .is_err());
    assert!(env
        .check_callable_function_source("f", source, Limits::DEFAULT)
        .is_ok());
    assert!(env
        .check_callable_function_source(
            "f",
            "fn f(n:Int)->E{let f=3;if n>0{return f(n-1);}else{return E::V(n);}}",
            Limits::DEFAULT
        )
        .is_ok());
}
#[test]
fn signature_and_evidence_limits_are_exact_and_fresh_after_failure() {
    let env = env("enum E{V(Float)}");
    let source = "fn f()->E{return E::V(1);}";
    let exact = Limits {
        max_signatures: 2,
        max_signature_text_bytes: 29,
        max_calls: 1,
        max_contexts: 2,
        max_type_nodes: 4,
        max_type_text_bytes: 2,
        ..Limits::DEFAULT
    };
    assert!(env
        .check_callable_function_source("f", source, exact)
        .is_ok());
    for limits in [
        Limits {
            max_signatures: 1,
            ..exact
        },
        Limits {
            max_signature_text_bytes: 28,
            ..exact
        },
        Limits {
            max_calls: 0,
            ..exact
        },
        Limits {
            max_contexts: 1,
            ..exact
        },
        Limits {
            max_type_nodes: 3,
            ..exact
        },
        Limits {
            max_type_text_bytes: 1,
            ..exact
        },
    ] {
        assert!(matches!(
            env.check_callable_function_source("f", source, limits),
            Err(Error::Limit(_))
        ));
        assert!(env
            .check_callable_function_source("f", source, exact)
            .is_ok());
    }
    let source = "fn f(){}";
    assert!(env
        .check_callable_function_source(
            "f",
            source,
            Limits {
                max_calls: 0,
                max_contexts: 0,
                max_type_nodes: 0,
                max_type_text_bytes: 0,
                ..Limits::DEFAULT
            }
        )
        .is_ok());
}
#[test]
fn semantic_errors_win_over_exhausted_evidence_without_success_fallback() {
    let env = env("enum E{V(Float)}");
    assert!(codes(
        env.check_callable_function_source(
            "f",
            "fn f()->E{return E::V(true);}",
            Limits {
                max_calls: 0,
                ..Limits::DEFAULT
            }
        )
        .unwrap_err()
    )
    .contains(&"E3007"));
    assert!(matches!(
        env.check_callable_function_source(
            "f",
            "fn f()->E{return E::V(1);}",
            Limits {
                max_calls: 0,
                ..Limits::DEFAULT
            }
        ),
        Err(Error::Limit(_))
    ));
}
#[test]
fn standard_constructor_collisions_refuse_with_original_enum_provenance() {
    let env = env("enum std{print(Int)}");
    let Error::Coverage(sovra::compiler::v2::EnumTypeError::Diagnostics(errors)) = env
        .check_callable_function_source("function", "fn f(){}", Limits::DEFAULT)
        .unwrap_err()
    else {
        panic!("expected nominal collision diagnostic")
    };
    assert_eq!(errors[0].source_label.as_ref(), "enum");
    assert_eq!(errors[0].diagnostic.code, "E3110");
    assert_eq!(
        &"enum std{print(Int)}"[errors[0].diagnostic.span.start..errors[0].diagnostic.span.end],
        "print"
    );
}

#[test]
fn scoped_constructor_and_argument_facts_keep_original_unicode_crlf_spans() {
    use sovra::compiler::v2::{check_scoped_enum_types, ScopedEnumSource};
    let context = Parser::new()
        .parse_source("mod lib{} type Alias=lib::E;")
        .unwrap();
    let env = check_scoped_enum_types(
        "owner",
        "types",
        &context,
        &[ScopedEnumSource {
            module: Some("lib"),
            declaration: EnumSource {
                label: "enum",
                source: "export enum E{V(Float)}",
            },
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap();
    let source = "// λ😀\r\nfn f()->Alias{\r\nreturn lib::E::V(7);\r\n}";
    let checked = env
        .check_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let call = checked.evidence().calls()[0];
    assert_eq!(&source[call.span.start..call.span.end], "lib::E::V");
    assert_eq!(call.span.line, 2);
    let argument = checked
        .evidence()
        .contexts()
        .iter()
        .find(|c| c.kind == CoercionKind::Argument)
        .unwrap();
    assert_eq!(
        &source[argument.value_span.start..argument.value_span.end],
        "7"
    );
    let bad = source.replace("(7)", "(true)");
    let Error::Diagnostics(errors) = env
        .check_callable_function_source("f", &bad, Limits::DEFAULT)
        .unwrap_err()
    else {
        panic!("expected argument diagnostic")
    };
    let error = errors.items.iter().find(|e| e.code == "E3007").unwrap();
    assert_eq!(&bad[error.span.start..error.span.end], "true");
    assert_eq!(error.span.line, 2);
    assert!(env
        .check_callable_function_source("f", "fn f()->Alias{return Alias::V(1);}", Limits::DEFAULT)
        .is_ok());
}
