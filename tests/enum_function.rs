//! Source-derived function scopes and exhaustive match return coverage.
use sovra::compiler::{
    parser::Parser,
    v2::{
        check_enum_types, CheckedEnumTypes, EnumFunctionError as Error,
        EnumFunctionLimits as Limits, EnumFunctionStatement, EnumSource, EnumTypeLimits,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "types",
        &Parser::new()
            .parse_source("type Alias=E;struct Item{x:Int}")
            .unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{Number(Int),Empty,Item(Item)}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
fn code(error: Error) -> &'static str {
    let Error::Diagnostics(errors) = error else {
        panic!("expected semantic diagnostics: {error:?}")
    };
    errors.items[0].code
}
const COMPLETE:&str="fn value(input:E)->Int{let local=input;match local{E::Number(n)=>{return n;},E::Empty()=>{return 0;},E::Item(item)=>{return item.x;}}}";
#[test]
fn source_locals_payloads_record_fields_and_complete_returns_are_checked() {
    let checked = env()
        .check_function_source("fn", COMPLETE, Limits::DEFAULT)
        .unwrap();
    assert!(checked.guarantees_return());
    assert_eq!(checked.matches().len(), 1);
    assert_eq!(
        checked.syntax().parameters[0].type_name.as_deref(),
        Some("E")
    );
    assert!(matches!(
        checked.syntax().body[0],
        EnumFunctionStatement::Core(_)
    ));
    assert!(matches!(
        checked.syntax().body[1],
        EnumFunctionStatement::Match(_)
    ));
}
#[test]
fn partial_matches_require_continuation_and_loops_do_not_establish_return() {
    let env = env();
    let partial="match input{E::Number(n)=>{if true{return n;}},E::Empty()=>{},E::Item(item)=>{while true{return item.x;}}}";
    assert_eq!(
        code(
            env.check_function_source(
                "f",
                &format!("fn value(input:E)->Int{{{partial}}}"),
                Limits::DEFAULT
            )
            .unwrap_err()
        ),
        "E3013"
    );
    assert!(env
        .check_function_source(
            "f",
            &format!("fn value(input:E)->Int{{{partial}return 4;}}"),
            Limits::DEFAULT
        )
        .unwrap()
        .guarantees_return());
    assert_eq!(
        code(
            env.check_function_source(
                "f",
                "fn value()->Int{while true{return 1;}}",
                Limits::DEFAULT
            )
            .unwrap_err()
        ),
        "E3013"
    );
}
#[test]
fn match_arm_scopes_do_not_escape_into_later_source() {
    let env = env();
    for suffix in ["print(n);", "print(item);", "print(hidden);"] {
        let source=format!("fn value(input:E){{match input{{E::Number(n)=>{{let hidden=n;}},E::Empty()=>{{}},E::Item(item)=>{{}}}}{suffix}}}");
        assert_eq!(
            code(
                env.check_function_source("f", &source, Limits::DEFAULT)
                    .unwrap_err()
            ),
            "E3001"
        );
    }
    assert_eq!(code(env.check_function_source("f","fn value(input:E){if true{let hidden=input;} match hidden{E::Number(n)=>{},E::Empty()=>{},E::Item(i)=>{}}}",Limits::DEFAULT).unwrap_err()),"E3108");
}
#[test]
fn payload_shadowing_is_immutable_and_outer_locals_keep_their_mutability() {
    let env = env();
    let source="fn value(input:E){let mut n=0;match input{E::Number(n)=>{n=1;},E::Empty()=>{n=2;},E::Item(i)=>{}}}";
    assert_eq!(
        code(
            env.check_function_source("f", source, Limits::DEFAULT)
                .unwrap_err()
        ),
        "E3011"
    );
    assert!(env.check_function_source("f","fn value(input:E){let mut n=0;match input{E::Number(v)=>{n=v;},E::Empty()=>{n=2;},E::Item(i)=>{}}n=3;}",Limits::DEFAULT).is_ok());
}
#[test]
fn parameters_annotations_entry_rules_and_builtin_collisions_match_core() {
    let env = env();
    for (source, expected) in [
        ("fn value(input){}", "E3014"),
        ("fn value(a:Int,a:Int){}", "E3011"),
        ("fn value(a:Missing){}", "E3017"),
        ("fn value()->Missing{}", "E3017"),
        ("fn main(a:Int){}", "E3009"),
        ("fn main()->Int{return 1;}", "E3010"),
        ("fn print(){}", "E3016"),
        ("fn value()->Int{return true;}", "E3002"),
    ] {
        assert_eq!(
            code(
                env.check_function_source("f", source, Limits::DEFAULT)
                    .unwrap_err()
            ),
            expected,
            "{source}"
        );
    }
    assert!(env
        .check_function_source("f", "fn main(){print(1);}", Limits::DEFAULT)
        .is_ok());
}
#[test]
fn coverage_scrutinee_types_and_unavailable_source_forms_are_refused() {
    let env = env();
    assert!(matches!(
        env.check_function_source(
            "f",
            "fn value(input:E){match input{E::Empty()=>{}}}",
            Limits::DEFAULT
        ),
        Err(Error::Coverage(_))
    ));
    assert_eq!(
        code(
            env.check_function_source(
                "f",
                "fn value(input:Int){match input{E::Empty()=>{}}}",
                Limits::DEFAULT
            )
            .unwrap_err()
        ),
        "E3108"
    );
    for source in ["fn a(){}fn b(){}","fn value(input:E){if true{match input{E::Empty()=>{}}}}",
        "fn value(input:E){match input{E::Number(n)=>{match input{E::Empty()=>{}}},E::Empty()=>{},E::Item(i)=>{}}}",
        "fn value(input:E){", "fn value(input:E){} garbage"] {
        assert!(matches!(env.check_function_source("f",source,Limits::DEFAULT),Err(Error::Syntax(_))),"{source}");
    }
}
#[test]
fn contextual_function_names_alias_parameters_and_ordinary_returns_work() {
    let env = env();
    let checked = env
        .check_function_source(
            "f",
            "fn match(enum:Alias)->E{let copy=enum;return copy;}",
            Limits::DEFAULT,
        )
        .unwrap();
    assert!(checked.guarantees_return());
    assert!(checked.matches().is_empty());
    assert!(env
        .check_function_source(
            "f",
            "export fn value(input:Alias)->Float{if true{return 1;}else{return 2;}}",
            Limits::DEFAULT
        )
        .unwrap()
        .guarantees_return());
    assert!(env
        .check_function_source("f", "fn value(input:E){let v=E::Empty();}", Limits::DEFAULT)
        .is_err());
    assert!(env
        .check_function_source("f", "fn value()->Int{return value();}", Limits::DEFAULT)
        .is_err());
}
#[test]
fn original_unicode_crlf_spans_survive_source_driven_body_errors() {
    let env = env();
    let source="// λ😀\r\nfn value(input:E){\r\nmatch input{E::Number(n)=>{n=0;},E::Empty()=>{},E::Item(item)=>{}}\r\n}";
    let Error::Diagnostics(errors) = env
        .check_function_source("source", source, Limits::DEFAULT)
        .unwrap_err()
    else {
        panic!("expected assignment error")
    };
    let error = errors.items.iter().find(|e| e.code == "E3011").unwrap();
    assert_eq!(&source[error.span.start..error.span.end], "n");
    assert_eq!(error.span.line, 2);
}
#[test]
fn function_limits_are_cumulative_exact_and_fresh_after_failure() {
    let env = env();
    let simple = "fn value(input:E){match input{E::Number(n)=>{},E::Empty()=>{},E::Item(i)=>{}}}";
    // Headers retain input (5), E/variant names (7+6+5), and payload n/i (2).
    let limits = Limits {
        max_parameters: 1,
        max_statements: 1,
        max_match_arms: 3,
        max_match_header_text_bytes: 25,
        max_scope_bindings: 5,
        max_source_label_bytes: 1,
        ..Limits::DEFAULT
    };
    assert!(env.check_function_source("f", simple, limits).is_ok());
    for limits in [
        Limits {
            max_parameters: 0,
            ..limits
        },
        Limits {
            max_statements: 0,
            ..limits
        },
        Limits {
            max_match_arms: 2,
            ..limits
        },
        Limits {
            max_match_header_text_bytes: 24,
            ..limits
        },
        Limits {
            max_scope_bindings: 4,
            ..limits
        },
        Limits {
            max_source_label_bytes: 0,
            ..limits
        },
    ] {
        assert!(env.check_function_source("f", simple, limits).is_err());
        assert!(env
            .check_function_source("f", simple, Limits::DEFAULT)
            .is_ok());
    }
    let twice = format!(
        "fn value(input:E){{{}{}}}",
        "match input{E::Number(n)=>{},E::Empty()=>{},E::Item(i)=>{}}",
        "match input{E::Number(n)=>{},E::Empty()=>{},E::Item(i)=>{}}"
    );
    assert!(env
        .check_function_source(
            "f",
            &twice,
            Limits {
                max_match_arms: 5,
                ..Limits::DEFAULT
            }
        )
        .is_err());
    assert!(env
        .check_function_source(
            "f",
            &twice,
            Limits {
                max_scope_bindings: 9,
                ..Limits::DEFAULT
            }
        )
        .is_err());
}
#[test]
fn deep_sources_and_token_byte_limits_refuse_without_partial_functions() {
    let env = env();
    let source = format!(
        "fn value(){{{}print(1);{}}}",
        "if true{".repeat(129),
        "}".repeat(129)
    );
    assert!(matches!(
        env.check_function_source("f", &source, Limits::DEFAULT),
        Err(Error::Syntax(_))
    ));
    let source = "fn value(){}";
    let limits = Limits {
        frontend: sovra::compiler::resources::FrontendLimits {
            max_source_bytes: source.len(),
            max_tokens: 6,
            ..Limits::DEFAULT.frontend
        },
        max_parameters: 0,
        max_statements: 0,
        max_match_arms: 0,
        max_match_header_text_bytes: 0,
        max_scope_bindings: 0,
        ..Limits::DEFAULT
    };
    assert!(env.check_function_source("f", source, limits).is_ok());
    assert!(env
        .check_function_source(
            "f",
            source,
            Limits {
                frontend: sovra::compiler::resources::FrontendLimits {
                    max_tokens: 5,
                    ..limits.frontend
                },
                ..limits
            }
        )
        .is_err());
}

#[test]
fn every_truncated_function_refuses_without_partial_source_success() {
    let env = env();
    for end in 0..COMPLETE.len() {
        assert!(
            env.check_function_source("f", &COMPLETE[..end], Limits::DEFAULT)
                .is_err(),
            "accepted prefix {end}"
        );
    }
    assert!(env
        .check_function_source("f", COMPLETE, Limits::DEFAULT)
        .is_ok());
}
