//! Callee-owned linked IR, shared invocation policy, explicit services and no fallback.
use sovra::compiler::{
    execution::{
        CancellationToken, ExecutionControl, ExecutionLimits, MaterializationLimits, RuntimePolicy,
    },
    v2::*,
};
fn check(s: &str) -> CheckedProgramSource {
    check_program_source("s", s, ProgramSourceLimits::DEFAULT).unwrap()
}
fn arg(n: i64) -> RuntimeValueV2 {
    RuntimeValueV2::int(
        n,
        &mut RuntimeValueFuel::new(MaterializationLimits::UNLIMITED),
    )
    .unwrap()
}
fn run(
    program: &CheckedProgramSource,
    name: &str,
    args: &[RuntimeValueV2],
    policy: RuntimeVmPolicy,
) -> Result<RuntimeVmOutcome, ProgramExecutionError> {
    program.execute(
        program.function_id(name).unwrap(),
        args,
        ProgramLoweringLimits::DEFAULT,
        policy,
    )
}
fn nodes(n: u64, t: u64) -> RuntimeVmPolicy {
    RuntimeVmPolicy {
        runtime: RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(n),
                max_text_bytes: Some(t),
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    }
}
#[test]
fn independent_enum_program_switches_callee_graphs_and_returns_nominal_records() {
    let program = check(include_str!("../examples/enum-functions/main.svr"));
    assert_eq!(
        run(&program, "main", &[], RuntimeVmPolicy::DEFAULT)
            .unwrap()
            .output,
        vec!["14"]
    );
    assert_eq!(
        run(&program, "twice", &[arg(9)], RuntimeVmPolicy::DEFAULT)
            .unwrap()
            .value
            .as_int(),
        Some(18)
    );
}
#[test]
fn forward_mutual_and_self_recursion_use_one_iterative_frame_stack() {
    let program=check("fn even(n:Int)->Bool{if n==0{return true;}else{return odd(n-1);}}fn odd(n:Int)->Bool{if n==0{return false;}else{return even(n-1);}}fn self_call(n:Int)->Int{if n>0{return self_call(n-1)+1;}else{return 0;}}");
    assert_eq!(
        run(&program, "even", &[arg(120)], RuntimeVmPolicy::DEFAULT)
            .unwrap()
            .value
            .as_bool(),
        Some(true)
    );
    assert_eq!(
        run(&program, "odd", &[arg(120)], RuntimeVmPolicy::DEFAULT)
            .unwrap()
            .value
            .as_bool(),
        Some(false)
    );
    assert_eq!(
        run(&program, "self_call", &[arg(120)], RuntimeVmPolicy::DEFAULT)
            .unwrap()
            .value
            .as_int(),
        Some(120)
    );
    assert!(matches!(
        run(
            &program,
            "even",
            &[arg(20)],
            RuntimeVmPolicy {
                max_call_depth: 10,
                ..RuntimeVmPolicy::DEFAULT
            }
        ),
        Err(ProgramExecutionError::Runtime(RuntimeVmError::CallDepth))
    ));
}
#[test]
fn callee_scopes_arguments_and_array_replacement_do_not_alias_caller_bindings() {
    let program=check("fn main(){let a=[1,2];let b=change(a);print(a);print(b);print(convert(3));}fn change(a:[Int])->[Int]{let mut b=a;b[0]=9;return b;}fn convert(n:Float)->Float{return n/2;}");
    assert_eq!(
        run(&program, "main", &[], RuntimeVmPolicy::DEFAULT)
            .unwrap()
            .output,
        vec!["[1, 2]", "[9, 2]", "1.5"]
    );
}
#[test]
fn linked_output_and_materialization_boundaries_match_legacy_across_calls() {
    let source = "fn main(){print(twice(7));print(twice(9));}fn twice(n:Int)->Int{return n+n;}";
    let program = check(source);
    let generous = run(&program, "main", &[], nodes(10000, 10000)).unwrap();
    let n = 10000 - generous.remaining.materialization.max_value_nodes.unwrap();
    let t = 10000 - generous.remaining.materialization.max_text_bytes.unwrap();
    let ast = sovra::compiler::parser::Parser::new()
        .parse_source(source)
        .unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&ast).unwrap();
    let old = sovra::compiler::interpreter::run_with_policy(&legacy, nodes(n, t).runtime).unwrap();
    assert_eq!(generous.output, old);
    let exact = run(&program, "main", &[], nodes(n, t)).unwrap();
    assert_eq!(exact.remaining.materialization.max_value_nodes, Some(0));
    assert_eq!(exact.remaining.materialization.max_text_bytes, Some(0));
    for policy in [nodes(n - 1, t), nodes(n, t - 1)] {
        assert!(run(&program, "main", &[], policy.clone()).is_err());
        assert!(sovra::compiler::interpreter::run_with_policy(&legacy, policy.runtime).is_err());
    }
    let output_policy = RuntimeVmPolicy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: None,
                max_output_bytes: Some(5),
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    assert!(
        matches!(run(&program,"main",&[],output_policy),Err(ProgramExecutionError::Runtime(RuntimeVmError::Execution(message))) if message.contains("output"))
    );
}
#[test]
fn instruction_and_control_allowances_are_shared_across_function_switches() {
    let program = check("fn a(){b();}fn b(){a();}");
    let policy = RuntimeVmPolicy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(30),
                max_output_bytes: None,
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    assert!(
        matches!(run(&program,"a",&[],policy),Err(ProgramExecutionError::Runtime(RuntimeVmError::Execution(message))) if message.contains("instruction"))
    );
    let token = CancellationToken::new();
    token.cancel();
    let policy = RuntimeVmPolicy {
        control: ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    assert!(
        matches!(run(&program,"a",&[],policy),Err(ProgramExecutionError::Runtime(RuntimeVmError::Execution(message))) if message.contains("cancelled"))
    );
}
#[derive(Default)]
struct Host {
    calls: usize,
    token: Option<CancellationToken>,
    written: Vec<String>,
}
impl RuntimeVmInput for Host {
    fn read_line(&mut self) -> Result<Option<String>, String> {
        self.calls += 1;
        Ok(Some("λ😀".into()))
    }
}
impl RuntimeVmFiles for Host {
    fn read_text(&mut self, _: &str) -> sovra::compiler::text_files::TextRead {
        unreachable!()
    }
    fn write_text(&mut self, _: &str, text: &str) -> sovra::compiler::text_files::TextWrite {
        self.written.push(text.into());
        if let Some(token) = &self.token {
            token.cancel();
        }
        sovra::compiler::text_files::TextWrite {
            ok: true,
            error: String::new(),
        }
    }
    fn write_text_if(
        &mut self,
        _: &str,
        _: bool,
        _: &str,
        _: &str,
    ) -> sovra::compiler::text_files::TextWrite {
        unreachable!()
    }
}
#[test]
fn explicit_input_arguments_and_exit_status_are_shared_by_callees() {
    let program=check("fn main(){helper();}fn helper(){let line=std::read_line();print(line.text);print(std::arg(0));std::set_exit_code(7);}");
    let mut host = Host::default();
    let result = program
        .execute_with_services(
            program.function_id("main").unwrap(),
            &[],
            &["arg".into()],
            RuntimeVmServices {
                input: Some(&mut host),
                files: None,
            },
            ProgramLoweringLimits::DEFAULT,
            RuntimeVmPolicy::DEFAULT,
        )
        .unwrap();
    assert_eq!(result.execution.output, vec!["λ😀", "arg"]);
    assert_eq!(result.exit_code, 7);
    assert_eq!(host.calls, 1);
}
#[test]
fn whole_program_service_preflight_precedes_effects_and_completed_writes_survive_cancel() {
    let program=check("fn main(){writer();}fn writer(){std::write_text(\"p\",\"new\");}fn unused(){std::read_line();}");
    let mut host = Host::default();
    let result = program.execute_with_services(
        program.function_id("main").unwrap(),
        &[],
        &[],
        RuntimeVmServices {
            input: None,
            files: Some(&mut host),
        },
        ProgramLoweringLimits::DEFAULT,
        RuntimeVmPolicy::DEFAULT,
    );
    assert!(matches!(
        result,
        Err(ProgramExecutionError::Runtime(
            RuntimeVmError::UnsupportedStandard("std::read_line")
        ))
    ));
    assert!(host.written.is_empty());
    let program = check("fn main(){writer();}fn writer(){std::write_text(\"p\",\"new\");}");
    let token = CancellationToken::new();
    host.token = Some(token.clone());
    let policy = RuntimeVmPolicy {
        control: ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    let result = program.execute_with_services(
        program.function_id("main").unwrap(),
        &[],
        &[],
        RuntimeVmServices {
            input: None,
            files: Some(&mut host),
        },
        ProgramLoweringLimits::DEFAULT,
        policy,
    );
    assert!(
        matches!(result,Err(ProgramExecutionError::Runtime(RuntimeVmError::Execution(message))) if message.contains("cancelled"))
    );
    assert_eq!(host.written, vec!["new"]);
}
#[test]
fn foreign_entries_and_aggregate_lowering_caps_refuse_without_fallback() {
    let program = check("fn main(){helper();}fn helper(){}");
    let foreign = check("fn main(){}");
    let id = program.function_id("main").unwrap();
    assert_eq!(
        program
            .execute(
                foreign.function_id("main").unwrap(),
                &[],
                ProgramLoweringLimits::DEFAULT,
                RuntimeVmPolicy::DEFAULT
            )
            .unwrap_err(),
        ProgramExecutionError::Entry
    );
    for limits in [
        ProgramLoweringLimits {
            max_mapping_nodes: 0,
            ..ProgramLoweringLimits::DEFAULT
        },
        ProgramLoweringLimits {
            max_fact_links: 0,
            ..ProgramLoweringLimits::DEFAULT
        },
        ProgramLoweringLimits {
            max_instructions: 0,
            ..ProgramLoweringLimits::DEFAULT
        },
        ProgramLoweringLimits {
            max_values: 0,
            ..ProgramLoweringLimits::DEFAULT
        },
    ] {
        assert!(program
            .execute(id, &[], limits, RuntimeVmPolicy::DEFAULT)
            .is_err());
    }
    assert!(run(&program, "main", &[], RuntimeVmPolicy::DEFAULT).is_ok());
}

#[test]
fn aggregate_graph_admission_is_exact_across_used_and_unused_functions() {
    let program = check("fn main(){leaf();}fn leaf(){}fn unused(){print(7);}");
    let (mut nodes, mut links, mut instructions, mut values) = (0, 0, 0, 0);
    for f in program.functions() {
        let map = f.map_for_lowering(MappingLimits::DEFAULT).unwrap();
        nodes += map.nodes().len();
        links += map.links().len();
        let graph = map.emit_expression_ir(ExpressionIrLimits::DEFAULT).unwrap();
        instructions += graph.instructions().len();
        values += graph.values().len();
        let function = graph.emit_function_ir(FunctionIrLimits::DEFAULT).unwrap();
        instructions += function.instructions().len();
    }
    let exact = ProgramLoweringLimits {
        max_mapping_nodes: nodes,
        max_fact_links: links,
        max_instructions: instructions,
        max_values: values,
        ..ProgramLoweringLimits::DEFAULT
    };
    let entry = program.function_id("main").unwrap();
    assert!(program
        .execute(entry, &[], exact, RuntimeVmPolicy::DEFAULT)
        .is_ok());
    for limits in [
        ProgramLoweringLimits {
            max_mapping_nodes: nodes - 1,
            ..exact
        },
        ProgramLoweringLimits {
            max_fact_links: links - 1,
            ..exact
        },
        ProgramLoweringLimits {
            max_instructions: instructions - 1,
            ..exact
        },
        ProgramLoweringLimits {
            max_values: values - 1,
            ..exact
        },
    ] {
        assert!(program
            .execute(entry, &[], limits, RuntimeVmPolicy::DEFAULT)
            .is_err());
    }
}
