//! Checked discovery, one linked graph, isolated invocations and suite quotas.
use sovra::compiler::{
    execution::{ExecutionLimits, RuntimePolicy},
    v2::*,
};
fn check(source: &str) -> CheckedProgramSource {
    check_program_source("root.svr", source, ProgramSourceLimits::DEFAULT).unwrap()
}
fn run(program: &CheckedProgramSource) -> sovra::compiler::testing::TestReport {
    program.test(
        ProgramLoweringLimits::DEFAULT,
        RuntimeVmPolicy::DEFAULT,
        ProgramTestLimits::DEFAULT,
    )
}
#[test]
fn independent_enum_tests_preserve_order_aliases_and_linked_helpers_without_main() {
    let report = run(&check(include_str!("../examples/enum-tests/main.svr")));
    assert!(report.success(), "{report:?}");
    assert_eq!(report.passed(), 2);
    assert_eq!(report.tests[0].name, "test_value");
    assert_eq!(report.tests[0].output, vec!["14"]);
    assert_eq!(report.tests[1].name, "test_end");
    assert_eq!(report.tests[1].output, vec!["end"]);
}
#[test]
fn invalid_signatures_refuse_the_whole_suite_at_original_source_locations() {
    let source="// λ😀\r\nfn test_good(){print(\"hidden\");}\r\nfn test_bad(x:Int){}\r\nfn test_other()->Int{return 1;}";
    let report = run(&check(source));
    assert!(report.tests.is_empty());
    assert!(!report.success());
    assert_eq!(report.diagnostics.items.len(), 2);
    for (d, name) in report
        .diagnostics
        .items
        .iter()
        .zip(["test_bad", "test_other"])
    {
        assert_eq!(d.code, "E5001");
        assert_eq!(d.source_file.as_deref(), Some("root.svr"));
        assert_eq!(d.span.start, source.find(&format!("fn {name}")).unwrap());
    }
}
#[test]
fn no_tests_fail_and_main_is_never_automatically_called() {
    let report = run(&check("fn main(){print(\"secret\");}"));
    assert_eq!(report.diagnostics.items[0].code, "E5002");
    assert!(report.tests.is_empty());
    let report = run(&check(
        "fn main(){std::set_exit_code(9);print(\"secret\");}fn test_only(){}",
    ));
    assert!(report.success());
    assert_eq!(report.tests[0].output, Vec::<String>::new());
    assert_eq!(report.tests[0].exit_code, Some(0));
}
#[test]
fn fresh_state_status_and_output_survive_only_successful_vm_returns() {
    let report=run(&check("fn test_status(){print(\"status\");std::set_exit_code(7);}fn test_error(){print(\"hidden\");print(1/0);}fn test_next(){print(std::arg_count());}"));
    assert_eq!(report.tests.len(), 3);
    assert_eq!(report.passed(), 1);
    assert_eq!(report.tests[0].exit_code, Some(7));
    assert_eq!(report.tests[0].output, vec!["status"]);
    assert_eq!(report.tests[1].exit_code, None);
    assert!(report.tests[1].output.is_empty());
    assert!(report.tests[1].error.is_some());
    assert_eq!(report.tests[2].exit_code, Some(0));
    assert_eq!(report.tests[2].output, vec!["0"]);
}
#[test]
fn exact_test_count_and_zero_count_admission_happen_before_execution() {
    let program = check("fn test_a(){}fn test_b(){}");
    for (max_tests, success) in [(2, true), (1, false), (0, false)] {
        let report = program.test(
            ProgramLoweringLimits::DEFAULT,
            RuntimeVmPolicy::DEFAULT,
            ProgramTestLimits {
                max_tests,
                ..ProgramTestLimits::DEFAULT
            },
        );
        assert_eq!(report.success(), success);
        if !success {
            assert!(report.tests.is_empty());
            assert_eq!(report.diagnostics.items[0].code, "E5003");
            assert_eq!(report.diagnostics.items[0].span.end, 0);
        }
    }
}
#[test]
fn aggregate_output_counts_lf_and_failed_admission_does_not_consume_remaining_bytes() {
    let program =
        check("fn test_large(){print(\"large\");}fn test_exact(){print(\"x\");}fn test_empty(){}");
    let report = program.test(
        ProgramLoweringLimits::DEFAULT,
        RuntimeVmPolicy::DEFAULT,
        ProgramTestLimits {
            max_output_bytes: 2,
            ..ProgramTestLimits::DEFAULT
        },
    );
    assert_eq!(report.passed(), 2);
    assert!(report.tests[0].output.is_empty());
    assert!(report.tests[0]
        .error
        .as_deref()
        .unwrap()
        .contains("suite captured-output"));
    assert_eq!(report.tests[1].output, vec!["x"]);
    assert!(report.tests[2].passed);
    let empty = check("fn test_empty(){}").test(
        ProgramLoweringLimits::DEFAULT,
        RuntimeVmPolicy::DEFAULT,
        ProgramTestLimits {
            max_output_bytes: 0,
            ..ProgramTestLimits::DEFAULT
        },
    );
    assert!(empty.success());
}
#[test]
fn lowering_exhaustion_is_a_configuration_failure_without_partial_tests() {
    let program = check("fn test_a(){print(1);}fn test_b(){}");
    let mut limits = ProgramLoweringLimits::DEFAULT;
    limits.max_mapping_nodes = 0;
    let report = program.test(limits, RuntimeVmPolicy::DEFAULT, ProgramTestLimits::DEFAULT);
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E3025");
    assert_eq!(report.diagnostics.items[0].span.end, 0);
}
#[test]
fn each_test_gets_fresh_instruction_fuel_after_a_previous_refusal() {
    let program = check("fn test_loop(){while true{}}fn test_next(){print(\"after\");}");
    let policy = RuntimeVmPolicy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(100),
                max_output_bytes: Some(100),
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..RuntimeVmPolicy::DEFAULT
    };
    let report = program.test(
        ProgramLoweringLimits::DEFAULT,
        policy,
        ProgramTestLimits::DEFAULT,
    );
    assert!(!report.tests[0].passed);
    assert!(report.tests[1].passed);
    assert_eq!(report.tests[1].output, vec!["after"]);
}
#[test]
fn unused_file_and_input_calls_refuse_all_tests_without_services() {
    for operation in ["std::read_line();", "std::write_text(\"unused\",\"bad\");"] {
        let program = check(&format!(
            "fn main(){{{operation}}}fn test_a(){{print(\"secret\");}}fn test_b(){{}}"
        ));
        let report = run(&program);
        assert_eq!(report.tests.len(), 2);
        assert_eq!(report.passed(), 0);
        assert!(report
            .tests
            .iter()
            .all(|t| t.output.is_empty()
                && t.error.as_deref().unwrap().contains("UnsupportedStandard")));
    }
}
#[test]
fn invalid_signature_diagnostics_are_capped_and_keep_original_file_identity() {
    let source = (0..100)
        .map(|i| format!("fn test_{i}(x:Int){{}}\n"))
        .collect::<String>();
    let report = run(&check(&source));
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items.len(), PROGRAM_DIAGNOSTIC_LIMIT);
    assert_eq!(report.diagnostics.items.last().unwrap().code, "E3019");
    assert!(report
        .diagnostics
        .items
        .iter()
        .all(|d| d.source_file.as_deref() == Some("root.svr")));
}
