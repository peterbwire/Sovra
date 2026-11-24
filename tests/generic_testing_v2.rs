//! Generic suites validate every selected specialization before execution.
use sovra::compiler::v2::*;
fn suite(source: &str, limits: GenericTestLimits) -> sovra::compiler::testing::TestReport {
    test_generic_source(
        source,
        "suite.svr",
        GenericPropagationSourceLimits::DEFAULT,
        GenericVmPolicy::DEFAULT,
        limits,
    )
}
#[test]
fn tests_have_fresh_vms_source_order_and_no_implicit_main() {
    let source="fn id<T>(x:T)->T{return x;}fn main(){let bad=1/0;}fn test_first(){let x=id::<Int>(1);}fn test_bad(){let bad=1/0;}fn test_last(){let x=id::<String>(\"ok\");}";
    let report = suite(source, GenericTestLimits::DEFAULT);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert_eq!(
        report
            .tests
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["test_first", "test_bad", "test_last"]
    );
    assert_eq!(
        report.tests.iter().map(|t| t.passed).collect::<Vec<_>>(),
        [true, false, true]
    );
    assert_eq!(report.tests[1].exit_code, None);
    assert!(report.tests[1]
        .error
        .as_ref()
        .unwrap()
        .contains("division by zero"));
}
#[test]
fn discovery_rejects_missing_tests_and_non_unit_parameterized_or_generic_roots() {
    for (source, code) in [
        ("fn main(){}", "E5002"),
        ("fn test_a(x:Int){}", "E5001"),
        ("fn test_a<T>(){}", "E5001"),
        ("fn test_a()->Int{return 1;}", "E5001"),
    ] {
        let report = suite(source, GenericTestLimits::DEFAULT);
        assert!(report.tests.is_empty());
        assert!(
            report.diagnostics.items.iter().any(|d| d.code == code),
            "{:?}",
            report
        );
    }
}
#[test]
fn later_invalid_specialization_prevents_all_execution() {
    let report = suite(
        "fn test_first(){let bad=1/0;}fn test_later(){let bad:Bool=1;}",
        GenericTestLimits::DEFAULT,
    );
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E3123");
    assert_eq!(
        report.diagnostics.items[0].source_file.as_deref(),
        Some("suite.svr")
    );
}
#[test]
fn test_count_and_whole_suite_compilation_admission_have_exact_boundaries() {
    let source = "fn test_a(){}fn test_b(){}";
    let exact = GenericTestLimits {
        max_tests: 2,
        max_compiled_source_bytes: source.len() * 5,
    };
    assert!(suite(source, exact).success());
    for limits in [
        GenericTestLimits {
            max_tests: 1,
            ..exact
        },
        GenericTestLimits {
            max_compiled_source_bytes: source.len() * 5 - 1,
            ..exact
        },
    ] {
        let report = suite(source, limits);
        assert!(report.tests.is_empty());
        assert_eq!(report.diagnostics.items[0].code, "E5003");
    }
}
#[test]
fn propagation_and_array_builtins_execute_in_test_roots_without_main() {
    let source="fn get<T>(x:std::Option<T>)->std::Option<T>{return std::Option::<T>::Some(x?);}fn test_generic(){let x=get::<Int>(std::Option::<Int>::Some(7));let a:[Int]=[1];let b=std::array_append(a,2);if std::array_len(b)!=2{let bad=1/0;}}";
    assert!(suite(source, GenericTestLimits::DEFAULT).success());
}

#[test]
fn specialization_refusal_in_later_root_prevents_earlier_runtime_failure() {
    let source="fn grow<T>(x:T){grow::<[T]>([x]);}fn test_first(){let bad=1/0;}fn test_later(){grow::<Int>(1);}";
    let mut compilation = GenericPropagationSourceLimits::DEFAULT;
    compilation.source.specialization.max_instances = 4;
    let report = test_generic_source(
        source,
        "growth.svr",
        compilation,
        GenericVmPolicy::DEFAULT,
        GenericTestLimits::DEFAULT,
    );
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E3123");
    assert!(report.diagnostics.items[0]
        .message
        .contains("Specialization"));
}
#[test]
fn per_test_runtime_refusal_does_not_enable_unbounded_fallback() {
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_work = 0;
    let report = test_generic_source(
        "fn test_a(){}fn test_b(){}",
        "fuel.svr",
        GenericPropagationSourceLimits::DEFAULT,
        policy,
        GenericTestLimits::DEFAULT,
    );
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.tests.len(), 2);
    assert!(report.tests.iter().all(|t| !t.passed));
    let source = (0..33)
        .map(|n| format!("fn test_{n}(){{}}"))
        .collect::<String>();
    let report = suite(
        &source,
        GenericTestLimits {
            max_tests: usize::MAX,
            ..GenericTestLimits::DEFAULT
        },
    );
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E5003");
}
