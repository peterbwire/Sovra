//! Structured versioned conditions/loops enclosing matches and vice versa.
use sovra::compiler::v2::*;
fn check(s: &str) -> CheckedProgramSource {
    check_program_source("s", s, ProgramSourceLimits::DEFAULT).unwrap()
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
fn matches_in_then_else_and_else_if_prove_return_coverage() {
    let source="enum E{N(Int),End}fn main(){print(f(1));print(f(0));print(f(2));}fn f(n:Int)->Int{let v=E::N(7);if n==1{match v{E::N(x)=>{return x;},E::End()=>{return 0;}}}else if n==0{match v{E::N(x)=>{return x+1;},E::End()=>{return 0;}}}else{match v{E::N(x)=>{return x+2;},E::End()=>{return 0;}}}}";
    assert_eq!(run(source), vec!["7", "8", "9"]);
}
#[test]
fn loop_matches_release_snapshots_and_rebind_locals_each_iteration() {
    let source="enum E{N(Int),End}fn main(){let mut n=0;let mut sum=0;while n<4{let v=E::N(n);match v{E::N(x)=>{sum=sum+x;},E::End()=>{}}n=n+1;}print(sum);print(n);}";
    assert_eq!(run(source), vec!["6", "4"]);
    assert_eq!(
        run(include_str!("../examples/control-matches/main.svr")),
        vec!["9"]
    );
}
#[test]
fn controls_inside_arms_and_matches_inside_controls_share_preorder_facts() {
    let source="enum E{N(Int),End}fn main(){let v=E::N(7);match v{E::N(x)=>{if x>0{let mut n=0;while n<2{match v{E::N(y)=>{print(x+y+n);},E::End()=>{}}n=n+1;}}else{print(0);}},E::End()=>{print(1);}}}";
    let p = check(source);
    assert_eq!(
        p.functions()[0]
            .typed()
            .callable()
            .function()
            .matches()
            .len(),
        2
    );
    assert_eq!(run(source), vec!["14", "15"]);
}
#[test]
fn branch_and_loop_bindings_do_not_escape_and_payloads_stay_immutable() {
    for source in [
        "enum E{N(Int)}fn main(){let v=E::N(7);if true{match v{E::N(x)=>{print(x);}}}print(x);}",
        "enum E{N(Int)}fn main(){let v=E::N(7);while false{match v{E::N(x)=>{print(x);}}}print(x);}",
        "enum E{N(Int)}fn main(){let mut x=1;let v=E::N(7);match v{E::N(x)=>{if true{x=2;}}}}",
        "enum E{N(Int)}fn main(){let v=E::N(7);if true{let local=1;}else{print(local);}}",
    ]{assert!(check_program_source("s",source,ProgramSourceLimits::DEFAULT).is_err(),"{source}");}
    assert_eq!(run("enum E{N(Int)}fn main(){let mut x=1;let v=E::N(7);if true{match v{E::N(x)=>{print(x);}}}print(x);}"),vec!["7","1"]);
}
#[test]
fn invalid_conditions_partial_returns_and_nonexhaustive_unreachable_matches_refuse() {
    for source in [
        "enum E{A}fn main(){let v=E::A();if 1{match v{E::A()=>{}}}}",
        "enum E{A}fn main(){let v=E::A();while 1{match v{E::A()=>{}}}}",
        "enum E{A}fn f()->Int{let v=E::A();if true{match v{E::A()=>{return 1;}}}}",
        "enum E{A}fn f()->Int{let v=E::A();while true{match v{E::A()=>{return 1;}}}}",
        "enum E{A,B}fn main(){let v=E::A();if false{match v{E::A()=>{}}}}",
    ] {
        assert!(
            check_program_source("s", source, ProgramSourceLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn ordinary_control_subset_keeps_legacy_output_and_common_fuel_costs() {
    use sovra::compiler::execution::{MaterializationLimits, RuntimePolicy};
    let source="fn main(){let mut n=0;while n<3{if n==1{print(7);}else if n==2{print(8);}else print(9);n=n+1;}}";
    let p = check(source);
    let entry = p.function_id("main").unwrap();
    let budget = |n, t| RuntimeVmPolicy {
        runtime: RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(n),
                max_text_bytes: Some(t),
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    let result = p
        .execute(
            entry,
            &[],
            ProgramLoweringLimits::DEFAULT,
            budget(10000, 10000),
        )
        .unwrap();
    let n = 10000 - result.remaining.materialization.max_value_nodes.unwrap();
    let t = 10000 - result.remaining.materialization.max_text_bytes.unwrap();
    let ast = sovra::compiler::parser::Parser::new()
        .parse_source(source)
        .unwrap();
    let (old, _) = sovra::compiler::compile_program(&ast).unwrap();
    assert_eq!(
        result.output,
        sovra::compiler::interpreter::run_with_policy(&old, budget(n, t).runtime).unwrap()
    );
    for policy in [budget(n - 1, t), budget(n, t - 1)] {
        assert!(p
            .execute(entry, &[], ProgramLoweringLimits::DEFAULT, policy.clone())
            .is_err());
        assert!(sovra::compiler::interpreter::run_with_policy(&old, policy.runtime).is_err());
    }
}
#[test]
fn unbraced_else_if_chains_are_depth_bounded_without_brace_nesting() {
    let source = format!(
        "fn main(){{{}{{print(1);}}}}",
        "if false{}else ".repeat(150)
    );
    let error = check_program_source("s", &source, ProgramSourceLimits::DEFAULT).unwrap_err();
    assert!(format!("{error:?}").contains("E2007"));
    assert!(check_entry_source(
        "s",
        "enum E{A}fn main(){let v=E::A();if true{match v{E::A()=>{}}}}",
        EntrySourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn control_match_scopes_share_existing_admission_and_infinite_loops_obey_fuel() {
    let source = "enum E{A}fn main(){let v=E::A();if true{match v{E::A()=>{print(7);}}}}";
    let mut limits = ProgramSourceLimits::DEFAULT;
    limits.source.callable.bindings.max_scope_frames = 3;
    assert!(check_program_source("s", source, limits).is_ok());
    limits.source.callable.bindings.max_scope_frames = 2;
    assert!(check_program_source("s", source, limits).is_err());
    let p = check("enum E{A}fn main(){let v=E::A();while true{match v{E::A()=>{}}}}");
    let policy = RuntimeVmPolicy {
        runtime: sovra::compiler::execution::RuntimePolicy {
            execution: sovra::compiler::execution::ExecutionLimits {
                max_instructions: Some(40),
                max_output_bytes: None,
            },
            ..sovra::compiler::execution::RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    assert!(
        matches!(p.execute(p.function_id("main").unwrap(),&[],ProgramLoweringLimits::DEFAULT,policy),Err(ProgramExecutionError::Runtime(RuntimeVmError::Execution(message))) if message.contains("instruction"))
    );
}
#[test]
fn call_conditions_are_evaluated_once_per_test_and_returns_keep_callee_identity() {
    let source="enum E{A}fn main(){let mut n=0;let v=E::A();while tick(n){match v{E::A()=>{n=n+1;}}}print(n);}fn tick(n:Int)->Bool{print(n);return n<2;}";
    assert_eq!(run(source), vec!["0", "1", "2", "2"]);
}
