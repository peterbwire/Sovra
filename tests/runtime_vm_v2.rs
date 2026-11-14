//! Actual sealed function-IR execution, shared model-v2 budgets and cooperative control.
use sovra::compiler::{
    execution::{
        CancellationToken, ExecutionControl, ExecutionLimits, MaterializationLimits, RuntimePolicy,
    },
    parser::Parser,
    v2::{
        check_enum_types, CheckedEnumTypes, EnumSource, EnumTypeLimits, ExpressionIrLimits,
        FunctionIrLimits, MappingLimits, RuntimeValueError, RuntimeValueFuel,
        RuntimeValueV2 as Value, RuntimeVmError as Error, RuntimeVmPolicy as Policy,
        ScopedCallableLimits,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "o",
        "c",
        &Parser::new().parse_source("struct R{x:Float}").unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{V(Int),Empty}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
macro_rules! ir {
    ($checked:ident,$map:ident,$expr:ident,$ir:ident,$source:expr) => {
        let $checked = env()
            .check_scoped_callable_function_source("f", $source, ScopedCallableLimits::DEFAULT)
            .unwrap();
        let $map = $checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
        let $expr = $map
            .emit_expression_ir(ExpressionIrLimits::DEFAULT)
            .unwrap();
        let $ir = $expr.emit_function_ir(FunctionIrLimits::DEFAULT).unwrap();
    };
}
fn nodes(n: u64, text: u64) -> Policy {
    Policy {
        runtime: RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(n),
                max_text_bytes: Some(text),
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..Policy::DEFAULT
    }
}
fn arg(n: i64) -> Value {
    Value::int(
        n,
        &mut RuntimeValueFuel::new(MaterializationLimits::UNLIMITED),
    )
    .unwrap()
}
#[test]
fn constructors_match_payloads_and_print_execute_with_exact_enum_costs() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(){let v=E::V(7);match v{E::V(n)=>{print(n);},E::Empty()=>{}}}"
    );
    let result = ir.execute(&[], nodes(8, 5)).unwrap();
    assert_eq!(result.output, vec!["7"]);
    assert_eq!(result.remaining.materialization.max_value_nodes, Some(0));
    assert_eq!(result.remaining.materialization.max_text_bytes, Some(0));
    assert!(matches!(
        ir.execute(&[], nodes(7, 5)),
        Err(Error::Value(RuntimeValueError::NodeLimit))
    ));
}
#[test]
fn equality_materializes_one_bool_while_match_dispatch_does_not() {
    ir!(checked, map, expr, ir, "fn f(){let v=E::V(7);print(v==v);}");
    let result = ir.execute(&[], nodes(9, 10)).unwrap();
    assert_eq!(result.output, vec!["true"]);
    assert_eq!(result.remaining.materialization.max_value_nodes, Some(0));
}
#[test]
fn loops_scopes_and_stores_execute_and_array_snapshots_do_not_alias() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->Int{let mut a=[1,2];let copy=a;a[0]=9;return copy[0]+a[0];}"
    );
    let result = ir.execute(&[], nodes(22, 0)).unwrap();
    assert_eq!(result.value.as_int(), Some(10));
    assert_eq!(result.remaining.materialization.max_value_nodes, Some(0));
    ir!(checked2,map2,expr2,ir2,"fn f()->Int{let mut n=0;while n<5{let delta=1;n=n+delta;}if n==5{return n;}else{return 0;}}");
    assert_eq!(
        ir2.execute(&[], Policy::DEFAULT).unwrap().value.as_int(),
        Some(5)
    );
}
#[test]
fn recursive_frames_share_limits_and_return_values_without_recursive_rust_execution() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(n:Int)->Int{if n>0{return f(n-1)+1;}else{return 0;}}"
    );
    assert_eq!(
        ir.execute(&[arg(20)], Policy::DEFAULT)
            .unwrap()
            .value
            .as_int(),
        Some(20)
    );
    assert!(matches!(
        ir.execute(
            &[arg(20)],
            Policy {
                max_call_depth: 10,
                ..Policy::DEFAULT
            }
        ),
        Err(Error::CallDepth)
    ));
    assert!(matches!(
        ir.execute(&[arg(20)], nodes(5, 0)),
        Err(Error::Value(RuntimeValueError::NodeLimit))
    ));
}
#[test]
fn instruction_admission_precedes_effects_and_counts_both_instruction_layers() {
    ir!(checked, map, expr, ir, "fn f()->Int{return 7;}");
    let exact = Policy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(5),
                max_output_bytes: None,
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(1),
                max_text_bytes: Some(0),
            },
        },
        ..Policy::DEFAULT
    };
    let result = ir.execute(&[], exact.clone()).unwrap();
    assert_eq!(result.value.as_int(), Some(7));
    assert_eq!(result.remaining.execution.max_instructions, Some(0));
    assert!(
        matches!(ir.execute(&[],Policy {runtime:RuntimePolicy {execution:ExecutionLimits {max_instructions:Some(4),max_output_bytes:None},..exact.runtime},..exact}),Err(Error::Execution(message)) if message.contains("instruction allowance"))
    );
}
#[test]
fn print_checks_utf8_plus_lf_before_materialization_and_final_unit_cost_remains() {
    ir!(checked, map, expr, ir, "fn f(){print(\"λ\");}");
    let exact = Policy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: None,
                max_output_bytes: Some(3),
            },
            ..nodes(3, 4).runtime
        },
        ..Policy::DEFAULT
    };
    let result = ir.execute(&[], exact.clone()).unwrap();
    assert_eq!(result.output, vec!["λ"]);
    assert_eq!(result.remaining.execution.max_output_bytes, Some(0));
    assert_eq!(result.remaining.materialization.max_value_nodes, Some(0));
    let denied = Policy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_output_bytes: Some(2),
                ..exact.runtime.execution
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(1),
                max_text_bytes: Some(2),
            },
        },
        ..exact
    };
    assert!(
        matches!(ir.execute(&[],denied),Err(Error::Execution(message)) if message.contains("output byte allowance"))
    );
}
#[test]
fn checked_arithmetic_bounds_and_unsupported_calls_refuse_without_legacy_fallback() {
    for (source, expected) in [
        ("fn f()->Int{return 1/0;}", "division by zero"),
        (
            "fn f()->Int{return 9223372036854775807+1;}",
            "integer overflow",
        ),
    ] {
        ir!(checked, map, expr, ir, source);
        assert!(
            matches!(ir.execute(&[],Policy::DEFAULT),Err(Error::Arithmetic(message)) if message==expected)
        );
    }
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->Int{let a=[1];return a[4];}"
    );
    assert!(matches!(
        ir.execute(&[], Policy::DEFAULT),
        Err(Error::Bounds)
    ));
    ir!(
        checked2,
        map2,
        expr2,
        ir2,
        "fn f(){print(\"before\");std::read_line();}"
    );
    assert!(matches!(
        ir2.execute(&[], Policy::DEFAULT),
        Err(Error::UnsupportedStandard("std::read_line"))
    ));
}
#[test]
fn pre_cancelled_and_zero_elapsed_controls_win_before_validation_or_execution() {
    ir!(checked, map, expr, ir, "fn f(){}");
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = Policy {
        control: ExecutionControl {
            cancellation: Some(cancellation),
            max_elapsed: None,
        },
        validation: FunctionIrLimits {
            max_validation_steps: 0,
            ..FunctionIrLimits::DEFAULT
        },
        ..Policy::DEFAULT
    };
    assert!(
        matches!(ir.execute(&[],cancelled),Err(Error::Execution(message)) if message=="execution cancelled")
    );
    assert!(
        matches!(ir.execute(&[],Policy {control:ExecutionControl {cancellation:None,max_elapsed:Some(std::time::Duration::ZERO)},..Policy::DEFAULT}),Err(Error::Execution(message)) if message.contains("elapsed"))
    );
}
#[test]
fn infinite_source_loops_observe_shared_instruction_limits() {
    ir!(checked, map, expr, ir, "fn f(){while true{}}");
    assert!(
        matches!(ir.execute(&[],Policy {runtime:RuntimePolicy {execution:ExecutionLimits {max_instructions:Some(50),max_output_bytes:None},..RuntimePolicy::UNLIMITED},..Policy::DEFAULT}),Err(Error::Execution(message)) if message.contains("instruction allowance"))
    );
}
#[test]
fn records_string_helpers_and_host_argument_type_guards_execute() {
    ir!(checked,map,expr,ir,"fn f(n:Float)->Float{let r=R{x:n};print(std::to_string(r));print(std::len(\"λ😀\"));print(std::array_len([1,2]));return r.x;}");
    let result = ir.execute(&[arg(7)], Policy::DEFAULT).unwrap();
    assert_eq!(result.value.as_float(), Some(7.0));
    assert_eq!(result.output, vec!["R { x: 7 }", "6", "2"]);
    let bad = Value::boolean(
        true,
        &mut RuntimeValueFuel::new(MaterializationLimits::UNLIMITED),
    )
    .unwrap();
    assert!(matches!(
        ir.execute(&[bad], nodes(0, 0)),
        Err(Error::Value(RuntimeValueError::TypeMismatch))
    ));
    assert!(matches!(
        ir.execute(&[], Policy::DEFAULT),
        Err(Error::Arity)
    ));
}

#[test]
fn ordinary_subset_matches_legacy_output_and_common_materialization_costs() {
    let source =
        "fn main(){let mut a=[1,2];let copy=a;a[0]=9;print(copy);print(a);print(\"λ😀\");}";
    let policy = Policy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: None,
                max_output_bytes: Some(21),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(22),
                max_text_bytes: Some(24),
            },
        },
        ..Policy::DEFAULT
    };
    let program = Parser::new().parse_source(source).unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
    let old = sovra::compiler::interpreter::run_with_policy(&legacy, policy.runtime).unwrap();
    ir!(checked, map, expr, ir, source);
    let new = ir.execute(&[], policy).unwrap();
    assert_eq!(old, new.output);
    assert_eq!(new.remaining.materialization.max_value_nodes, Some(0));
    assert_eq!(new.remaining.materialization.max_text_bytes, Some(0));
    assert_eq!(new.remaining.execution.max_output_bytes, Some(0));
}

#[test]
fn compound_array_replacement_rebases_owned_enum_subtrees_without_aliasing() {
    ir!(checked,map,expr,ir,"fn f(){let mut a=[E::V(1),E::Empty()];let before=a;a[0]=E::Empty();a[1]=E::V(9);print(before[0]);print(a[0]);print(a[1]);}");
    assert_eq!(
        ir.execute(&[], Policy::DEFAULT).unwrap().output,
        vec!["E::V(1)", "E::Empty()", "E::V(9)"]
    );
}
#[test]
fn host_supplied_foreign_enum_arguments_refuse_before_entry_snapshot_fuel() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(input:E)->Int{match input{E::V(n)=>{return n;},E::Empty()=>{return 0;}}}"
    );
    let other = env();
    let foreign = Value::enum_value(
        &other,
        other.find("E").unwrap(),
        1,
        None,
        &mut RuntimeValueFuel::new(MaterializationLimits::UNLIMITED),
    )
    .unwrap();
    assert!(matches!(
        ir.execute(&[foreign], nodes(0, 0)),
        Err(Error::Value(RuntimeValueError::TypeMismatch))
    ));
}

#[test]
fn array_adapters_preserve_snapshots_widening_empty_shapes_and_nominal_payloads() {
    ir!(
        checked,
        map,
        expr,
        ir,
        r#"fn f(){
        let mut a=[E::V(1),E::Empty()];
        let b=std::array_slice(a,0,1);
        let c=std::array_append(b,E::Empty());
        let d=std::array_concat(c,a);
        a[0]=E::V(9);
        print(b);print(c);print(d);print(a);
        print(std::array_slice(a,2,2));
        print(std::array_concat([1.0],[3])[1]/2);
        print(std::array_append([[1]],[]));
    }"#
    );
    assert_eq!(
        ir.execute(&[], Policy::DEFAULT).unwrap().output,
        vec![
            "[E::V(1)]",
            "[E::V(1), E::Empty()]",
            "[E::V(1), E::Empty(), E::V(1), E::Empty()]",
            "[E::V(9), E::Empty()]",
            "[]",
            "1.5",
            "[[1], []]"
        ]
    );
}

#[test]
fn array_adapters_match_legacy_exact_materialization_costs_and_refusal() {
    for (source, n, text) in [
        ("fn main(){let a=std::array_append([1],2);print(a);}", 11, 6),
        (
            "fn main(){let a=std::array_concat([1],[2]);print(a);}",
            12,
            6,
        ),
        (
            "fn main(){let a=std::array_slice([1,2,3],1,3);print(a);}",
            14,
            6,
        ),
    ] {
        let program = Parser::new().parse_source(source).unwrap();
        let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
        let policy = nodes(n, text);
        let old = sovra::compiler::interpreter::run_with_policy(&legacy, policy.runtime).unwrap();
        ir!(checked, map, expr, ir, source);
        let new = ir.execute(&[], policy).unwrap();
        assert_eq!(new.output, old);
        assert_eq!(new.remaining.materialization.max_value_nodes, Some(0));
        assert_eq!(new.remaining.materialization.max_text_bytes, Some(0));
        assert!(matches!(
            ir.execute(&[], nodes(n - 1, text)),
            Err(Error::Value(RuntimeValueError::NodeLimit))
        ));
        assert!(matches!(
            ir.execute(&[], nodes(n, text - 1)),
            Err(Error::Value(RuntimeValueError::TextLimit))
        ));
    }
}

#[test]
fn slice_invalid_bounds_refuse_before_result_fuel_and_empty_slice_costs_one_node() {
    for (start, end) in [("0-1", 0), ("1", 0), ("0", 2), ("2", 2)] {
        let source = format!("fn f()->[Int]{{return std::array_slice([1],{start},{end});}}");
        ir!(checked, map, expr, ir, &source);
        assert!(matches!(
            ir.execute(&[], nodes(if start == "0-1" { 6 } else { 4 }, 0)),
            Err(Error::Bounds)
        ));
    }
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->[Int]{return std::array_slice([1],1,1);}"
    );
    let result = ir.execute(&[], nodes(5, 0)).unwrap();
    assert_eq!(result.value.array_len(), Some(0));
    assert_eq!(result.remaining.materialization.max_value_nodes, Some(0));
}

#[test]
fn unicode_length_matches_legacy_utf8_bytes_instead_of_scalar_count() {
    let source = "fn main(){print(std::len(\"λ😀\"));}";
    let program = Parser::new().parse_source(source).unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
    let old =
        sovra::compiler::interpreter::run_with_policy(&legacy, RuntimePolicy::UNLIMITED).unwrap();
    assert_eq!(old, vec!["6"]);
    ir!(checked, map, expr, ir, source);
    assert_eq!(ir.execute(&[], Policy::DEFAULT).unwrap().output, old);
}

#[test]
fn pure_string_adapters_match_legacy_output_and_exact_fuel_boundaries() {
    for source in [
        r#"fn main(){let s=std::split_once("λ::😀","::");print(s);}"#,
        r#"fn main(){let s=std::split_once("λ","");print(s);}"#,
        r#"fn main(){let s=std::split_once("λ","missing");print(s);}"#,
        r#"fn main(){print(std::parse_int("-9223372036854775808"));}"#,
        r#"fn main(){print(std::parse_int("9223372036854775808"));}"#,
        r#"fn main(){print(std::lines_unique("λ\n😀\n"));}"#,
        r#"fn main(){print(std::lines_unique("λ\nλ\n"));}"#,
        r#"fn main(){std::println(std::escape_terminal("λ\n\t😀"));}"#,
    ] {
        ir!(checked, map, expr, ir, source);
        let generous = ir.execute(&[], nodes(10_000, 10_000)).unwrap();
        let used_nodes = 10_000 - generous.remaining.materialization.max_value_nodes.unwrap();
        let used_text = 10_000 - generous.remaining.materialization.max_text_bytes.unwrap();
        let program = Parser::new().parse_source(source).unwrap();
        let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
        let exact = nodes(used_nodes, used_text);
        let old = sovra::compiler::interpreter::run_with_policy(&legacy, exact.runtime).unwrap();
        assert_eq!(generous.output, old, "{source}");
        assert_eq!(ir.execute(&[], exact).unwrap().output, old);
        for policy in [
            nodes(used_nodes - 1, used_text),
            nodes(used_nodes, used_text - 1),
        ] {
            assert!(
                sovra::compiler::interpreter::run_with_policy(&legacy, policy.runtime).is_err(),
                "{source}"
            );
            assert!(ir.execute(&[], policy).is_err(), "{source}");
        }
    }
}

#[test]
fn parsed_int_preserves_strict_decimal_grammar_and_record_fields() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(s:String)->std::ParsedInt{return std::parse_int(s);}"
    );
    for (text, expected) in [
        ("0", Some(0)),
        ("-0", Some(0)),
        ("-9223372036854775808", Some(i64::MIN)),
        ("9223372036854775807", Some(i64::MAX)),
        ("9223372036854775808", None),
        ("-9223372036854775809", None),
        ("01", None),
        ("+1", None),
        (" 1", None),
        ("1\n", None),
        ("", None),
        ("λ", None),
    ] {
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
        let arg = Value::string(text, &mut fuel).unwrap();
        let result = ir.execute(&[arg], Policy::DEFAULT).unwrap().value;
        assert_eq!(
            result.field_snapshot("ok", &mut fuel).unwrap().as_bool(),
            Some(expected.is_some()),
            "{text}"
        );
        assert_eq!(
            result.field_snapshot("value", &mut fuel).unwrap().as_int(),
            Some(expected.unwrap_or_default()),
            "{text}"
        );
    }
}

#[test]
fn terminal_escape_covers_c0_del_c1_without_changing_other_unicode() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(s:String)->String{return std::escape_terminal(s);}"
    );
    let text = "\0\n\t\u{7f}\u{80}\u{9f}λ😀\\";
    let expected = "\\u{0000}\\u{000A}\\u{0009}\\u{007F}\\u{0080}\\u{009F}λ😀\\";
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    let input = Value::string(text, &mut fuel).unwrap();
    assert_eq!(
        ir.execute(&[input], Policy::DEFAULT)
            .unwrap()
            .value
            .as_str(),
        Some(expected)
    );
}

#[test]
fn explicit_command_arguments_and_exit_status_match_legacy_exact_common_costs() {
    struct Host(Vec<String>);
    impl sovra::compiler::interpreter::RuntimeHost for Host {
        fn arguments(&self) -> &[String] {
            &self.0
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            panic!("unexpected input")
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
    }
    let source = "fn main(){print(std::arg_count());print(std::arg(0));std::set_exit_code(7);}";
    let args = vec!["λ😀".to_owned()];
    let policy = Policy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: None,
                max_output_bytes: Some(9),
            },
            ..nodes(8, 13).runtime
        },
        ..Policy::DEFAULT
    };
    let program = Parser::new().parse_source(source).unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
    let old = sovra::compiler::interpreter::run_with_host_status_and_policy(
        &legacy,
        &mut Host(args.clone()),
        policy.runtime,
    )
    .unwrap();
    ir!(checked, map, expr, ir, source);
    let new = ir.execute_with_arguments(&[], &args, policy).unwrap();
    assert_eq!(new.execution.output, old.output);
    assert_eq!(new.exit_code, old.exit_code);
    assert_eq!(new.exit_code, 7);
    assert_eq!(
        new.execution.remaining.materialization.max_value_nodes,
        Some(0)
    );
    assert_eq!(
        new.execution.remaining.materialization.max_text_bytes,
        Some(0)
    );
    assert_eq!(new.execution.remaining.execution.max_output_bytes, Some(0));
    assert!(matches!(
        ir.execute_with_arguments(&[], &args, nodes(7, 13)),
        Err(Error::Value(RuntimeValueError::NodeLimit))
    ));
    assert!(matches!(
        ir.execute_with_arguments(&[], &args, nodes(8, 12)),
        Err(Error::Value(RuntimeValueError::TextLimit))
    ));
    assert_eq!(args, vec!["λ😀"]);
}

#[test]
fn command_context_is_explicit_and_io_still_refuses_in_preflight() {
    for source in [
        "fn f(){print(std::arg_count());}",
        "fn f(){if false{print(std::arg(0));}}",
        "fn f(){std::set_exit_code(0);}",
    ] {
        ir!(checked, map, expr, ir, source);
        assert!(matches!(
            ir.execute(&[], nodes(0, 0)),
            Err(Error::UnsupportedStandard(_))
        ));
        ir.execute_with_arguments(&[], &[], Policy::DEFAULT)
            .unwrap();
    }
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(){if false{std::read_line();}}"
    );
    assert!(matches!(
        ir.execute_with_arguments(&[], &[], nodes(0, 0)),
        Err(Error::UnsupportedStandard("std::read_line"))
    ));
}

#[test]
fn command_argument_bounds_and_text_admission_preserve_existing_order() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(i:Int)->String{return std::arg(i);}"
    );
    let args = vec!["λ😀".to_owned()];
    for index in [-1, 1, i64::MAX] {
        assert!(matches!(
            ir.execute_with_arguments(&[arg(index)], &args, nodes(2, 0)),
            Err(Error::Bounds)
        ));
    }
    assert!(matches!(
        ir.execute_with_arguments(&[arg(0)], &args, nodes(2, 6)),
        Err(Error::Value(RuntimeValueError::NodeLimit))
    ));
    assert!(matches!(
        ir.execute_with_arguments(&[arg(0)], &args, nodes(3, 5)),
        Err(Error::Value(RuntimeValueError::TextLimit))
    ));
    let result = ir
        .execute_with_arguments(&[arg(0)], &args, nodes(3, 6))
        .unwrap();
    assert_eq!(result.execution.value.as_str(), Some("λ😀"));
    assert_eq!(result.exit_code, 0);
}

#[test]
fn command_status_range_shared_recursion_and_invocation_reset() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(n:Int){if n>0{std::set_exit_code(n);f(n-1);}else{print(std::arg(0));}}"
    );
    let args = vec!["one".to_owned()];
    let result = ir
        .execute_with_arguments(&[arg(3)], &args, Policy::DEFAULT)
        .unwrap();
    assert_eq!(result.execution.output, vec!["one"]);
    assert_eq!(result.exit_code, 1);
    let result = ir
        .execute_with_arguments(&[arg(0)], &args, Policy::DEFAULT)
        .unwrap();
    assert_eq!(result.exit_code, 0);
    ir!(
        checked2,
        map2,
        expr2,
        ir2,
        "fn f(code:Int){std::set_exit_code(code);}"
    );
    for code in [-1, 126, 256, i64::MAX] {
        assert!(matches!(
            ir2.execute_with_arguments(&[arg(code)], &[], nodes(2, 0)),
            Err(Error::ExitCode)
        ));
    }
    for code in [0, 125] {
        assert_eq!(
            ir2.execute_with_arguments(&[arg(code)], &[], nodes(4, 0))
                .unwrap()
                .exit_code,
            code as u8
        );
    }
}

#[derive(Default)]
struct InputHost {
    lines: std::collections::VecDeque<Result<Option<String>, String>>,
    calls: usize,
    cancel: Option<CancellationToken>,
}
impl sovra::compiler::v2::RuntimeVmInput for InputHost {
    fn read_line(&mut self) -> Result<Option<String>, String> {
        self.calls += 1;
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        self.lines.pop_front().unwrap_or(Ok(None))
    }
}
impl sovra::compiler::interpreter::RuntimeHost for InputHost {
    fn arguments(&self) -> &[String] {
        &[]
    }
    fn read_line(&mut self) -> Result<Option<String>, String> {
        sovra::compiler::v2::RuntimeVmInput::read_line(self)
    }
    fn write_line(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
}
fn input_lines(lines: &[Option<&str>]) -> InputHost {
    InputHost {
        lines: lines.iter().map(|s| Ok(s.map(str::to_owned))).collect(),
        ..InputHost::default()
    }
}

#[test]
fn explicit_line_input_matches_legacy_unicode_empty_eof_and_exact_fuel() {
    let source =
        "fn main(){print(std::read_line());print(std::read_line());print(std::read_line());}";
    ir!(checked, map, expr, ir, source);
    let mut host = input_lines(&[Some("λ😀"), Some(""), None]);
    let result = ir
        .execute_with_input(&[], &[], &mut host, nodes(10_000, 10_000))
        .unwrap();
    assert_eq!(host.calls, 3);
    let n = 10_000
        - result
            .execution
            .remaining
            .materialization
            .max_value_nodes
            .unwrap();
    let text = 10_000
        - result
            .execution
            .remaining
            .materialization
            .max_text_bytes
            .unwrap();
    let program = Parser::new().parse_source(source).unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
    let policy = nodes(n, text);
    let old = sovra::compiler::interpreter::run_with_host_status_and_policy(
        &legacy,
        &mut input_lines(&[Some("λ😀"), Some(""), None]),
        policy.runtime,
    )
    .unwrap();
    assert_eq!(result.execution.output, old.output);
    assert_eq!(result.exit_code, old.exit_code);
    let exact = ir
        .execute_with_input(
            &[],
            &[],
            &mut input_lines(&[Some("λ😀"), Some(""), None]),
            policy,
        )
        .unwrap();
    assert_eq!(
        exact.execution.remaining.materialization.max_value_nodes,
        Some(0)
    );
    assert_eq!(
        exact.execution.remaining.materialization.max_text_bytes,
        Some(0)
    );
    for policy in [nodes(n - 1, text), nodes(n, text - 1)] {
        assert!(ir
            .execute_with_input(
                &[],
                &[],
                &mut input_lines(&[Some("λ😀"), Some(""), None]),
                policy.clone()
            )
            .is_err());
        assert!(
            sovra::compiler::interpreter::run_with_host_status_and_policy(
                &legacy,
                &mut input_lines(&[Some("λ😀"), Some(""), None]),
                policy.runtime
            )
            .is_err()
        );
    }
}

#[test]
fn input_minimum_preflight_precedes_consumption_but_unknown_text_can_refuse_after_read() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->std::InputLine{return std::read_line();}"
    );
    let labels = ("std::InputLine".len() + "eof".len() + "text".len()) as u64;
    for policy in [nodes(2, labels), nodes(3, labels - 1)] {
        let mut host = input_lines(&[Some("λ😀")]);
        assert!(matches!(
            ir.execute_with_input(&[], &[], &mut host, policy),
            Err(Error::Value(_))
        ));
        assert_eq!(host.calls, 0);
        assert_eq!(host.lines.len(), 1);
    }
    let mut host = input_lines(&[Some("λ😀")]);
    assert!(matches!(
        ir.execute_with_input(&[], &[], &mut host, nodes(3, labels)),
        Err(Error::Value(RuntimeValueError::TextLimit))
    ));
    assert_eq!(host.calls, 1);
    assert!(host.lines.is_empty());
    let mut host = input_lines(&[Some("λ😀")]);
    let result = ir
        .execute_with_input(&[], &[], &mut host, nodes(3, labels + 6))
        .unwrap();
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    assert_eq!(
        result
            .execution
            .value
            .field_snapshot("text", &mut fuel)
            .unwrap()
            .as_str(),
        Some("λ😀")
    );
    assert_eq!(
        result
            .execution
            .value
            .field_snapshot("eof", &mut fuel)
            .unwrap()
            .as_bool(),
        Some(false)
    );
}

#[test]
fn input_errors_and_oversize_returns_refuse_without_alternate_host() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->std::InputLine{return std::read_line();}"
    );
    let mut host = InputHost {
        lines: [Err("denied".to_owned())].into(),
        ..InputHost::default()
    };
    assert!(
        matches!(ir.execute_with_input(&[],&[],&mut host,Policy::DEFAULT),Err(Error::Host(message)) if message=="denied")
    );
    assert_eq!(host.calls, 1);
    let long = "x".repeat(sovra::compiler::stdlib::MAX_INPUT_LINE_BYTES + 1);
    let mut host = input_lines(&[Some(&long)]);
    assert!(matches!(
        ir.execute_with_input(&[], &[], &mut host, nodes(3, 100)),
        Err(Error::InputLimit)
    ));
    assert_eq!(host.calls, 1);
}

#[test]
fn input_observes_cancellation_before_and_after_host_call() {
    ir!(checked, map, expr, ir, "fn f(){std::read_line();}");
    for pre_cancel in [true, false] {
        let token = CancellationToken::new();
        if pre_cancel {
            token.cancel();
        }
        let mut host = input_lines(&[Some("consumed")]);
        host.cancel = Some(token.clone());
        let policy = Policy {
            control: ExecutionControl {
                cancellation: Some(token),
                max_elapsed: None,
            },
            ..Policy::DEFAULT
        };
        assert!(
            matches!(ir.execute_with_input(&[],&[],&mut host,policy),Err(Error::Execution(message)) if message.contains("cancelled"))
        );
        assert_eq!(host.calls, usize::from(!pre_cancel));
    }
}

#[test]
fn input_context_is_shared_by_recursion_and_keeps_file_calls_unavailable() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(n:Int){if n>0{let line=std::read_line();print(line.text);f(n-1);}}"
    );
    let mut host = input_lines(&[Some("one"), Some("two"), Some("three")]);
    let result = ir
        .execute_with_input(&[arg(3)], &[], &mut host, Policy::DEFAULT)
        .unwrap();
    assert_eq!(result.execution.output, vec!["one", "two", "three"]);
    assert_eq!(host.calls, 3);
    ir!(
        checked2,
        map2,
        expr2,
        ir2,
        "fn f(){std::read_line();if false{std::read_text(\"file\");}}"
    );
    assert!(matches!(
        ir2.execute_with_input(&[], &[], &mut host, Policy::DEFAULT),
        Err(Error::UnsupportedStandard("std::read_text"))
    ));
    assert_eq!(host.calls, 3);
}

#[derive(Default)]
struct FileHost {
    files: std::collections::BTreeMap<String, String>,
    events: Vec<String>,
    cancel: Option<CancellationToken>,
}
impl sovra::compiler::v2::RuntimeVmFiles for FileHost {
    fn read_text(&mut self, path: &str) -> sovra::compiler::text_files::TextRead {
        self.events.push(format!("read:{path}"));
        match self.files.get(path) {
            Some(text) => sovra::compiler::text_files::TextRead {
                ok: true,
                text: text.clone(),
                error: String::new(),
            },
            None => sovra::compiler::text_files::TextRead {
                ok: false,
                text: String::new(),
                error: "not_found".into(),
            },
        }
    }
    fn write_text(&mut self, path: &str, text: &str) -> sovra::compiler::text_files::TextWrite {
        self.events.push(format!("write:{path}:{text}"));
        self.files.insert(path.into(), text.into());
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        sovra::compiler::text_files::TextWrite {
            ok: true,
            error: String::new(),
        }
    }
    fn write_text_if(
        &mut self,
        path: &str,
        exists: bool,
        expected: &str,
        text: &str,
    ) -> sovra::compiler::text_files::TextWrite {
        self.events
            .push(format!("if:{path}:{exists}:{expected}:{text}"));
        let actual = self.files.get(path);
        if actual.is_some() != exists || (exists && actual.map(String::as_str) != Some(expected)) {
            return sovra::compiler::text_files::TextWrite {
                ok: false,
                error: "conflict".into(),
            };
        }
        self.files.insert(path.into(), text.into());
        sovra::compiler::text_files::TextWrite {
            ok: true,
            error: String::new(),
        }
    }
}
impl sovra::compiler::interpreter::RuntimeHost for FileHost {
    fn arguments(&self) -> &[String] {
        &[]
    }
    fn read_line(&mut self) -> Result<Option<String>, String> {
        panic!("unexpected input")
    }
    fn write_line(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn read_text(&mut self, path: &str) -> sovra::compiler::text_files::TextRead {
        sovra::compiler::v2::RuntimeVmFiles::read_text(self, path)
    }
    fn write_text(&mut self, path: &str, text: &str) -> sovra::compiler::text_files::TextWrite {
        sovra::compiler::v2::RuntimeVmFiles::write_text(self, path, text)
    }
    fn write_text_if(
        &mut self,
        path: &str,
        exists: bool,
        expected: &str,
        text: &str,
    ) -> sovra::compiler::text_files::TextWrite {
        sovra::compiler::v2::RuntimeVmFiles::write_text_if(self, path, exists, expected, text)
    }
}
fn file_services(
    host: &mut dyn sovra::compiler::v2::RuntimeVmFiles,
) -> sovra::compiler::v2::RuntimeVmServices<'_> {
    sovra::compiler::v2::RuntimeVmServices {
        input: None,
        files: Some(host),
    }
}

#[test]
fn explicit_files_match_legacy_records_effect_order_and_exact_fuel() {
    let source = r#"fn main(){print(std::read_text("missing"));print(std::write_text("p","λ"));print(std::read_text("p"));print(std::write_text_if("p",true,"stale","lost"));print(std::write_text_if("p",true,"λ","😀"));print(std::read_text("p"));}"#;
    ir!(checked, map, expr, ir, source);
    let mut host = FileHost::default();
    let new = ir
        .execute_with_services(&[], &[], file_services(&mut host), nodes(100_000, 100_000))
        .unwrap();
    let n = 100_000
        - new
            .execution
            .remaining
            .materialization
            .max_value_nodes
            .unwrap();
    let text = 100_000
        - new
            .execution
            .remaining
            .materialization
            .max_text_bytes
            .unwrap();
    let program = Parser::new().parse_source(source).unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
    let mut old_host = FileHost::default();
    let old = sovra::compiler::interpreter::run_with_host_status_and_policy(
        &legacy,
        &mut old_host,
        nodes(n, text).runtime,
    )
    .unwrap();
    assert_eq!(new.execution.output, old.output);
    assert_eq!(host.events, old_host.events);
    assert_eq!(host.files, old_host.files);
    assert_eq!(host.files.get("p").map(String::as_str), Some("😀"));
    let exact = ir
        .execute_with_services(
            &[],
            &[],
            file_services(&mut FileHost::default()),
            nodes(n, text),
        )
        .unwrap();
    assert_eq!(
        exact.execution.remaining.materialization.max_value_nodes,
        Some(0)
    );
    assert_eq!(
        exact.execution.remaining.materialization.max_text_bytes,
        Some(0)
    );
    for policy in [nodes(n - 1, text), nodes(n, text - 1)] {
        assert!(ir
            .execute_with_services(
                &[],
                &[],
                file_services(&mut FileHost::default()),
                policy.clone()
            )
            .is_err());
        assert!(
            sovra::compiler::interpreter::run_with_host_status_and_policy(
                &legacy,
                &mut FileHost::default(),
                policy.runtime
            )
            .is_err()
        );
    }
}

#[test]
fn file_minimum_preflight_blocks_effects_and_unknown_read_text_can_refuse_after_observation() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->std::TextRead{return std::read_text(\"p\");}"
    );
    let labels = ("std::TextRead".len() + "oktexterror".len()) as u64;
    let mut host = FileHost::default();
    host.files.insert("p".into(), "λ😀".into());
    assert!(matches!(
        ir.execute_with_services(&[], &[], file_services(&mut host), nodes(4, 100)),
        Err(Error::Value(RuntimeValueError::NodeLimit))
    ));
    assert!(host.events.is_empty());
    assert!(matches!(
        ir.execute_with_services(&[], &[], file_services(&mut host), nodes(5, labels)),
        Err(Error::Value(RuntimeValueError::TextLimit))
    ));
    assert!(host.events.is_empty());
    assert!(matches!(
        ir.execute_with_services(&[], &[], file_services(&mut host), nodes(5, labels + 1)),
        Err(Error::Value(RuntimeValueError::TextLimit))
    ));
    assert_eq!(host.events, vec!["read:p"]);
    ir!(
        checked2,
        map2,
        expr2,
        ir2,
        "fn f()->std::TextWrite{return std::write_text(\"p\",\"new\");}"
    );
    let mut host = FileHost::default();
    assert!(matches!(
        ir2.execute_with_services(&[], &[], file_services(&mut host), nodes(4, 100)),
        Err(Error::Value(RuntimeValueError::NodeLimit))
    ));
    assert!(host.events.is_empty());
    assert!(host.files.is_empty());
}

#[test]
fn completed_file_write_is_not_rolled_back_on_post_host_cancellation() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(){std::write_text(\"p\",\"new\");}"
    );
    let token = CancellationToken::new();
    let mut host = FileHost {
        cancel: Some(token.clone()),
        ..FileHost::default()
    };
    let policy = Policy {
        control: ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        },
        ..Policy::DEFAULT
    };
    assert!(
        matches!(ir.execute_with_services(&[],&[],file_services(&mut host),policy),Err(Error::Execution(message)) if message.contains("cancelled"))
    );
    assert_eq!(host.files.get("p").map(String::as_str), Some("new"));
}

#[test]
fn services_are_independent_and_file_only_context_never_grants_input() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(){std::write_text(\"p\",\"new\");if false{std::read_line();}}"
    );
    let mut host = FileHost::default();
    assert!(matches!(
        ir.execute_with_services(&[], &[], file_services(&mut host), Policy::DEFAULT),
        Err(Error::UnsupportedStandard("std::read_line"))
    ));
    assert!(host.events.is_empty());
    ir!(
        checked2,
        map2,
        expr2,
        ir2,
        "fn f(){if false{std::read_text(\"p\");}}"
    );
    assert!(matches!(
        ir2.execute_with_services(
            &[],
            &[],
            sovra::compiler::v2::RuntimeVmServices::default(),
            Policy::DEFAULT
        ),
        Err(Error::UnsupportedStandard("std::read_text"))
    ));
    ir!(checked3,map3,expr3,ir3,"fn f(){let line=std::read_line();std::write_text(\"p\",line.text);let value=std::read_text(\"p\");print(value.text);}");
    let mut input = input_lines(&[Some("λ😀")]);
    let services = sovra::compiler::v2::RuntimeVmServices {
        input: Some(&mut input),
        files: Some(&mut host),
    };
    assert_eq!(
        ir3.execute_with_services(&[], &[], services, Policy::DEFAULT)
            .unwrap()
            .execution
            .output,
        vec!["λ😀"]
    );
    assert_eq!(input.calls, 1);
}

#[test]
fn oversized_file_host_returns_refuse_before_full_result_fuel() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->std::TextRead{return std::read_text(\"p\");}"
    );
    let mut host = FileHost::default();
    host.files.insert(
        "p".into(),
        "x".repeat(sovra::compiler::stdlib::MAX_TEXT_FILE_BYTES + 1),
    );
    assert!(matches!(
        ir.execute_with_services(&[], &[], file_services(&mut host), nodes(5, 100)),
        Err(Error::FileLimit)
    ));
    assert_eq!(host.events, vec!["read:p"]);
}

#[test]
fn explicit_process_file_adapter_performs_real_atomic_and_conditional_replacement() {
    let directory = std::env::temp_dir().join(format!(
        "sovra-v2-files-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("state.txt");
    ir!(
        checked,
        map,
        expr,
        ir,
        r#"fn f(path:String){let first=std::write_text(path,"λ");print(first.ok);let stale=std::write_text_if(path,true,"old","lost");print(stale.error);let next=std::write_text_if(path,true,"λ","😀");print(next.ok);let read=std::read_text(path);print(read.text);}"#
    );
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    let argument = Value::string(path.to_str().unwrap(), &mut fuel).unwrap();
    let result = ir
        .execute_with_services(
            &[argument],
            &[],
            file_services(&mut sovra::compiler::interpreter::ProcessHost::new(vec![])),
            Policy::DEFAULT,
        )
        .unwrap();
    assert_eq!(
        result.execution.output,
        vec!["true", "conflict", "true", "😀"]
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "😀");
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
