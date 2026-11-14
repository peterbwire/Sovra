//! Closed-scope enum match body checking without executable program claims.
use sovra::compiler::{
    parser::Parser,
    v2::{
        check_enum_types, CheckedEnumTypes, EnumSource, EnumTypeLimits, MatchBodyError as Error,
        MatchBodyLimits as Limits, MatchOuterBinding as Binding,
    },
};
fn environment(context: &str, declaration: &str) -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "context",
        &Parser::new().parse_source(context).unwrap(),
        &[EnumSource {
            label: "enum",
            source: declaration,
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
fn binding<'a>(name: &'a str, ty: &'a str, mutable: bool) -> Binding<'a> {
    Binding {
        name,
        type_name: ty,
        is_mutable: mutable,
    }
}
fn codes(error: Error) -> Vec<&'static str> {
    let Error::Diagnostics(errors) = error else {
        panic!("expected body diagnostics: {error:?}")
    };
    errors.items.into_iter().map(|e| e.code).collect()
}
#[test]
fn payload_types_standard_calls_and_all_arm_return_flow_are_checked() {
    let env = environment("", "enum E{Number(Int),Empty}");
    let source =
        "match result{E::Number(n)=>{if(flag){return n;}else{return 0;}},E::Empty()=>{return 1;}}";
    let checked = env
        .check_match_bodies(
            env.find("E").unwrap(),
            "match",
            source,
            &[
                binding("result", "E", false),
                binding("flag", "Bool", false),
            ],
            "Float",
            Limits::DEFAULT,
        )
        .unwrap();
    assert_eq!(checked.arm_returns(), &[true, true]);
    assert!(checked.guarantees_return());
    assert_eq!(checked.coverage().enum_id(), env.find("E").unwrap());
    let checked = env
        .check_match_bodies(
            env.find("E").unwrap(),
            "match",
            "match result{E::Empty()=>{print(0);},E::Number(n)=>{return n;}}",
            &[binding("result", "E", false)],
            "Int",
            Limits::DEFAULT,
        )
        .unwrap();
    assert_eq!(checked.arm_returns(), &[false, true]);
    assert!(!checked.guarantees_return());
}
#[test]
fn payload_bindings_are_immutable_even_when_shadowing_mutable_outer_locals() {
    let env = environment("", "enum E{Value(Int)}");
    let source = "// λ😀\r\nmatch result{E::Value(n)=>{n=2;}}";
    let Error::Diagnostics(errors) = env
        .check_match_bodies(
            env.find("E").unwrap(),
            "match",
            source,
            &[binding("result", "E", false), binding("n", "String", true)],
            "Unit",
            Limits::DEFAULT,
        )
        .unwrap_err()
    else {
        panic!("expected immutable assignment")
    };
    let error = errors.items.iter().find(|e| e.code == "E3011").unwrap();
    assert_eq!(&source[error.span.start..error.span.end], "n");
    assert_eq!(error.span.line, 1);
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "match",
            "match result{E::Value(n)=>{let mut n=n+1;n=3;}}",
            &[binding("result", "E", false)],
            "Unit",
            Limits::DEFAULT
        )
        .is_ok());
}
#[test]
fn arrays_are_immutable_payloads_and_core_type_errors_are_preserved() {
    let env = environment("", "enum E{Rows([Int])}");
    let id = env.find("E").unwrap();
    let outer = [binding("result", "E", false)];
    assert!(codes(
        env.check_match_bodies(
            id,
            "m",
            "match result{E::Rows(rows)=>{rows[0]=1;}}",
            &outer,
            "Unit",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3011"));
    assert!(codes(
        env.check_match_bodies(
            id,
            "m",
            "match result{E::Rows(rows)=>{let n:Bool=std::array_len(rows);}}",
            &outer,
            "Unit",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3002"));
    assert!(codes(
        env.check_match_bodies(
            id,
            "m",
            "match result{E::Rows(rows)=>{return rows;}}",
            &outer,
            "Int",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3002"));
}
#[test]
fn arm_local_names_do_not_leak_and_outer_mutability_is_preserved() {
    let env = environment("", "enum E{Value(Int),Empty}");
    let outer = [binding("result", "E", false), binding("total", "Int", true)];
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "m",
            "match result{E::Value(n)=>{total=n;let tmp=1;},E::Empty()=>{total=0;}}",
            &outer,
            "Unit",
            Limits::DEFAULT
        )
        .is_ok());
    assert!(codes(
        env.check_match_bodies(
            env.find("E").unwrap(),
            "m",
            "match result{E::Value(n)=>{let tmp=n;},E::Empty()=>{print(tmp);}}",
            &outer,
            "Unit",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3001"));
    assert!(codes(
        env.check_match_bodies(
            env.find("E").unwrap(),
            "m",
            "match result{E::Value(n)=>{},E::Empty()=>{print(n);}}",
            &outer,
            "Unit",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3001"));
}
#[test]
fn loops_and_partial_conditionals_do_not_prove_return_coverage() {
    let env = environment("", "enum E{Value(Int)}");
    for body in ["while true{return n;}", "if true{return n;}", "print(n);"] {
        let source = format!("match result{{E::Value(n)=>{{{body}}}}}");
        let checked = env
            .check_match_bodies(
                env.find("E").unwrap(),
                "m",
                &source,
                &[binding("result", "E", false)],
                "Int",
                Limits::DEFAULT,
            )
            .unwrap();
        assert!(!checked.guarantees_return());
    }
}
#[test]
fn records_are_checked_but_enum_probe_records_never_authorize_values_or_fields() {
    let env = environment("struct Item{value:Int}", "enum E{Item(Item),Child(E)}");
    let outer = [binding("result", "E", false)];
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "m",
            "match result{E::Item(item)=>{return item.value;},E::Child(child)=>{return 0;}}",
            &outer,
            "Int",
            Limits::DEFAULT
        )
        .unwrap()
        .guarantees_return());
    for source in [
        "match result{E::Item(item)=>{let fake=E{Item:item,Child:result};},E::Child(child)=>{}}",
        "match result{E::Item(item)=>{},E::Child(child)=>{print(child.Item);}}",
    ] {
        assert!(env
            .check_match_bodies(
                env.find("E").unwrap(),
                "m",
                source,
                &outer,
                "Unit",
                Limits::DEFAULT
            )
            .is_err());
    }
}
#[test]
fn coverage_nominal_scrutinee_and_closed_context_are_required() {
    let env = environment("", "enum E{Value(Int),Empty}");
    let id = env.find("E").unwrap();
    let source = "match result{E::Value(n)=>{},E::Empty()=>{}}";
    for outer in [
        vec![],
        vec![binding("result", "Int", false)],
        vec![binding("result", "Missing", false)],
        vec![binding("result", "E", false), binding("result", "E", true)],
        vec![
            binding("result", "E", false),
            binding("bad:name", "Int", false),
        ],
    ] {
        assert!(matches!(
            env.check_match_bodies(id, "m", source, &outer, "Unit", Limits::DEFAULT),
            Err(Error::Diagnostics(_))
        ));
    }
    assert!(matches!(
        env.check_match_bodies(
            id,
            "m",
            "match result{E::Empty()=>{}}",
            &[binding("result", "E", false)],
            "Unit",
            Limits::DEFAULT
        ),
        Err(Error::Coverage(_))
    ));
    assert!(matches!(
        env.check_match_bodies(
            id,
            "m",
            source,
            &[binding("result", "E", false)],
            "Missing",
            Limits::DEFAULT
        ),
        Err(Error::Diagnostics(_))
    ));
    let foreign = environment("", "enum E{Value(Int),Empty}");
    assert!(matches!(
        env.check_match_bodies(
            foreign.find("E").unwrap(),
            "m",
            source,
            &[binding("result", "E", false)],
            "Unit",
            Limits::DEFAULT
        ),
        Err(Error::Coverage(_))
    ));
}
#[test]
fn scope_and_context_limits_refuse_one_under_and_recover_freshly() {
    let env = environment("", "enum E{Value(Int),Empty}");
    let source = "match result{E::Value(n)=>{},E::Empty()=>{}}";
    let outer = [binding("result", "E", false)];
    let exact = Limits {
        max_outer_bindings: 1,
        max_context_text_bytes: 12,
        max_scope_bindings: 3,
        ..Limits::DEFAULT
    };
    assert!(env
        .check_match_bodies(env.find("E").unwrap(), "m", source, &outer, "Unit", exact)
        .is_ok());
    for limits in [
        Limits {
            max_outer_bindings: 0,
            ..exact
        },
        Limits {
            max_context_text_bytes: 11,
            ..exact
        },
        Limits {
            max_scope_bindings: 2,
            ..exact
        },
    ] {
        assert!(matches!(
            env.check_match_bodies(env.find("E").unwrap(), "m", source, &outer, "Unit", limits),
            Err(Error::Limit(_))
        ));
        assert!(env
            .check_match_bodies(env.find("E").unwrap(), "m", source, &outer, "Unit", exact)
            .is_ok());
    }
}
#[test]
fn builtin_record_fields_work_and_user_calls_remain_explicitly_unavailable() {
    let env = environment("fn helper()->Int{return 1;}", "enum E{Value(Int)}");
    let outer = [binding("result", "E", false)];
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "m",
            "match result{E::Value(n)=>{let parsed=std::parse_int(\"1\");return parsed.value+n;}}",
            &outer,
            "Int",
            Limits::DEFAULT
        )
        .is_ok());
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "m",
            "match result{E::Value(n)=>{return helper();}}",
            &outer,
            "Int",
            Limits::DEFAULT
        )
        .is_err());
}

#[test]
fn nominal_return_types_and_alias_scrutinees_are_not_structural_records() {
    let context = Parser::new().parse_source("type Alias=E;").unwrap();
    let env = check_enum_types(
        "owner",
        "context",
        &context,
        &[
            EnumSource {
                label: "e",
                source: "enum E{Child(E)}",
            },
            EnumSource {
                label: "f",
                source: "enum F{Child(F)}",
            },
        ],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap();
    let source = "match result{E::Child(child)=>{return child;}}";
    let outer = [binding("result", "Alias", false)];
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "m",
            source,
            &outer,
            "Alias",
            Limits::DEFAULT
        )
        .is_ok());
    assert!(codes(
        env.check_match_bodies(
            env.find("E").unwrap(),
            "m",
            source,
            &outer,
            "F",
            Limits::DEFAULT
        )
        .unwrap_err()
    )
    .contains(&"E3002"));
}

#[test]
fn outer_names_follow_core_keyword_and_trivia_rules() {
    let env = environment("", "enum E{Value(Int)}");
    let source = "match result{E::Value(n)=>{}}";
    for name in ["return", "true", "x ", " x", "x//comment", "", "λ"] {
        let outer = [binding("result", "E", false), binding(name, "Int", false)];
        assert!(
            matches!(
                env.check_match_bodies(
                    env.find("E").unwrap(),
                    "m",
                    source,
                    &outer,
                    "Unit",
                    Limits::DEFAULT
                ),
                Err(Error::Diagnostics(_))
            ),
            "{name}"
        );
    }
    let outer = [
        binding("result", "E", false),
        binding("match", "Int", false),
        binding("enum", "Int", false),
    ];
    assert!(env
        .check_match_bodies(
            env.find("E").unwrap(),
            "m",
            source,
            &outer,
            "Unit",
            Limits::DEFAULT
        )
        .is_ok());
}
