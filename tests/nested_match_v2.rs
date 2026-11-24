//! Program-only matches inside match arms, through checking/IR/linked execution.
use sovra::compiler::v2::*;
const TYPES: &str = "enum I{N(Int),Empty} enum O{Some(I),None}";
fn check(s: &str) -> CheckedProgramSource {
    check_program_source("nested.svr", s, ProgramSourceLimits::DEFAULT).unwrap()
}
fn run(s: &str) -> Vec<String> {
    let p = check(s);
    p.execute(
        p.function_id("main").unwrap(),
        &[],
        ProgramLoweringLimits::DEFAULT,
        RuntimeVmPolicy::DEFAULT,
    )
    .unwrap()
    .output
}
#[test]
fn nested_payloads_return_through_linked_functions_on_every_path() {
    let source=format!("{TYPES} fn main(){{print(unwrap(O::Some(I::N(7))));print(unwrap(O::Some(I::Empty())));print(unwrap(O::None()));}}fn unwrap(v:O)->Int{{match v{{O::Some(i)=>{{match i{{I::N(n)=>{{return n;}},I::Empty()=>{{return 0;}}}}}},O::None()=>{{return 0;}}}}}}");
    let p = check(&source);
    assert_eq!(
        p.functions()[1]
            .typed()
            .callable()
            .function()
            .matches()
            .len(),
        2
    );
    assert_eq!(run(&source), vec!["7", "0", "0"]);
    assert_eq!(
        run(include_str!("../examples/nested-matches/main.svr")),
        vec!["7", "0", "0"]
    );
}
#[test]
fn snapshot_is_once_captured_and_outer_mutable_bindings_remain_distinct() {
    let source=format!("{TYPES} fn main(){{let mut result=0;let mut v=O::Some(I::N(7));match v{{O::Some(i)=>{{v=O::None();match i{{I::N(n)=>{{result=n;}},I::Empty()=>{{result=1;}}}}}},O::None()=>{{result=2;}}}}print(result);print(v);}}");
    assert_eq!(run(&source), vec!["7", "O::None()"]);
}
#[test]
fn inner_payload_shadow_is_immutable_and_does_not_escape() {
    let body = "match v{O::Some(i)=>{match i{I::N(n)=>{print(n);},I::Empty()=>{}}},O::None()=>{}}";
    let source =
        format!("{TYPES} fn main(){{let mut n=99;let v=O::Some(I::N(7));{body}print(n);}}");
    assert_eq!(run(&source), vec!["7", "99"]);
    assert!(check_program_source(
        "s",
        &source.replace("print(n);},I::Empty", "n=1;},I::Empty"),
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
    assert!(check_program_source(
        "s",
        &source.replace("}print(n);", "}print(i);"),
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn incomplete_coverage_types_and_partial_returns_refuse() {
    for body in [
        "match v{O::Some(i)=>{match i{I::N(n)=>{return n;}}},O::None()=>{return 0;}}",
        "match v{O::Some(i)=>{match i{I::N(n)=>{return n;},I::Empty()=>{}}},O::None()=>{return 0;}}",
        "match v{O::Some(i)=>{match i{I::N(n)=>{return true;},I::Empty()=>{return 0;}}},O::None()=>{return 0;}}",
        "match v{O::Some(i)=>{match i{I::N(n)=>{return n;},I::N(x)=>{return x;}}},O::None()=>{return 0;}}",
    ] {assert!(check_program_source("s",&format!("{TYPES} fn f(v:O)->Int{{{body}}}"),ProgramSourceLimits::DEFAULT).is_err());}
}
#[test]
fn arm_header_and_scope_admission_is_cumulative() {
    let source=format!("{TYPES} fn main(){{let v=O::Some(I::N(7));match v{{O::Some(i)=>{{match i{{I::N(n)=>{{print(n);}},I::Empty()=>{{}}}}}},O::None()=>{{}}}}}}");
    let mut limits = ProgramSourceLimits::DEFAULT;
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_arms = 4;
    assert!(check_program_source("s", &source, limits).is_ok());
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_arms = 3;
    assert!(check_program_source("s", &source, limits).is_err());
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_arms = 4;
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_header_text_bytes = 0;
    assert!(check_program_source("s", &source, limits).is_err());
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_header_text_bytes = 1000;
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_scope_bindings = 0;
    assert!(check_program_source("s", &source, limits).is_err());
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_scope_bindings = 8;
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_header_text_bytes = 22;
    assert!(check_program_source("s", &source, limits).is_ok());
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_header_text_bytes = 21;
    assert!(check_program_source("s", &source, limits).is_err());
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_match_header_text_bytes = 22;
    limits
        .source
        .callable
        .typed
        .callable
        .function
        .max_scope_bindings = 7;
    assert!(check_program_source("s", &source, limits).is_err());
}
#[test]
fn sibling_and_three_level_matches_keep_preorder_plans_and_distinct_snapshots() {
    let source="enum E{Next(E),End}fn main(){let v=E::Next(E::Next(E::End()));match v{E::Next(a)=>{match a{E::Next(b)=>{match b{E::Next(c)=>{print(1);},E::End()=>{print(2);}}},E::End()=>{print(3);}}match a{E::Next(b)=>{print(4);},E::End()=>{print(5);}}},E::End()=>{print(6);}}}";
    assert_eq!(
        check(source).functions()[0]
            .typed()
            .callable()
            .function()
            .matches()
            .len(),
        4
    );
    assert_eq!(run(source), vec!["2", "4"]);
}
#[test]
fn earlier_apis_stay_strict_as_program_control_bodies_expand() {
    let source=format!("{TYPES} fn main(){{let v=O::Some(I::N(7));match v{{O::Some(i)=>{{match i{{I::N(n)=>{{print(n);}},I::Empty()=>{{}}}}}},O::None()=>{{}}}}}}");
    assert!(check_entry_source("s", &source, EntrySourceLimits::DEFAULT).is_err());
    assert!(parse_match_statement(
        "match x{E::A(n)=>{match n{E::B()=>{}}}}",
        MatchSyntaxLimits::DEFAULT
    )
    .is_err());
    for s in [
        "enum E{A}fn main(){let v=E::A();if true{match v{E::A()=>{}}}}",
        "enum E{A}fn main(){let v=E::A();while false{match v{E::A()=>{}}}}",
    ] {
        assert!(check_entry_source("s", s, EntrySourceLimits::DEFAULT).is_err());
    }
}
#[test]
fn contextual_match_calls_and_unicode_brace_strings_keep_core_behavior() {
    assert_eq!(
        run("fn main(){print(match(7));}fn match(n:Int)->Int{return n;}"),
        vec!["7"]
    );
    assert_eq!(run("enum E{A}fn main(){let v=E::A();match v{E::A()=>{print(\"λ😀{}\");match v{E::A()=>{print(7);}}}}}"),vec!["λ😀{}","7"]);
}

#[test]
fn permuted_nested_arm_orders_dispatch_by_nominal_tag_not_plan_position() {
    let source=format!("{TYPES} fn main(){{let v=O::Some(I::N(7));match v{{O::None()=>{{print(0);}},O::Some(i)=>{{match i{{I::Empty()=>{{print(1);}},I::N(n)=>{{print(n);}}}}}}}}}}");
    assert_eq!(run(&source), vec!["7"]);
}
#[test]
fn nested_snapshots_and_payloads_preserve_exact_model_v2_costs() {
    use sovra::compiler::execution::{MaterializationLimits, RuntimePolicy};
    let p=check("enum I{N(Int)}enum O{Some(I)}fn main(){let v=O::Some(I::N(7));match v{O::Some(i)=>{match i{I::N(n)=>{print(n);}}}}}");
    let entry = p.function_id("main").unwrap();
    for (n, t, expected) in [
        (14, 19, None),
        (13, 19, Some(RuntimeValueError::NodeLimit)),
        (14, 18, Some(RuntimeValueError::TextLimit)),
    ] {
        let policy = RuntimeVmPolicy {
            runtime: RuntimePolicy {
                materialization: MaterializationLimits {
                    max_value_nodes: Some(n),
                    max_text_bytes: Some(t),
                },
                ..RuntimePolicy::UNLIMITED
            },
            ..RuntimeVmPolicy::DEFAULT
        };
        let result = p.execute(entry, &[], ProgramLoweringLimits::DEFAULT, policy);
        if let Some(error) = expected {
            assert_eq!(
                result.unwrap_err(),
                ProgramExecutionError::Runtime(RuntimeVmError::Value(error))
            );
        } else {
            let result = result.unwrap();
            assert_eq!(result.output, vec!["7"]);
            assert_eq!(result.remaining.materialization.max_value_nodes, Some(0));
            assert_eq!(result.remaining.materialization.max_text_bytes, Some(0));
        }
    }
}

#[test]
fn recursive_source_depth_stays_bounded_before_recursive_parsing() {
    fn source(depth: usize) -> String {
        let mut value = "E::End()".to_owned();
        let mut body = format!("print({depth});");
        for i in (0..depth).rev() {
            value = format!("E::Next({value})");
            body = format!(
                "match v{i}{{E::Next(v{})=>{{{body}}},E::End()=>{{print({i});}}}}",
                i + 1
            );
        }
        format!("enum E{{Next(E),End}}fn main(){{let v0={value};{body}}}")
    }
    assert_eq!(run(&source(16)), vec!["16"]);
    assert!(
        matches!(check_program_source("s",&source(80),ProgramSourceLimits::DEFAULT),Err(ProgramSourceError::Source(EntrySourceError::Syntax(d))) if d.items.iter().any(|d|d.code=="E2007"))
    );
}
