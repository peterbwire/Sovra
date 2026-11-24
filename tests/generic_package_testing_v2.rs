//! Shared snapshot suites, root-only discovery and all-specialization preflight.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{project::packages::*, testing::TestReport, v2::*};
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra graph suite λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for name in ["app", "lib"] {
            std::fs::create_dir_all(path.join(name)).unwrap();
        }
        let f = Self(path);
        f.write("app/sovra.toml","[project]\nname=\"app\"\nentry=\"main.svr\"\n[modules]\nlocal=\"local.svr\"\n[dependencies.lib]\npath=\"../lib\"\n");
        f.write(
            "lib/sovra.toml",
            "[project]\nname=\"lib\"\nentry=\"main.svr\"\n[modules]\nops=\"ops.svr\"\n",
        );
        f.write("app/main.svr","use lib::ops;fn main(){let bad=1/0;}fn test_first(){let x=lib::ops::boxed::<Int>(7);if x.value!=7{let bad=1/0;}}fn test_failure(){lib::ops::fail();}fn test_last(){let x=lib::ops::boxed::<String>(\"ok\");if x.value!=\"ok\"{let bad=1/0;}}");
        f.write(
            "app/local.svr",
            "mod local {export fn test_module(){let bad=1/0;}}",
        );
        f.write(
            "lib/main.svr",
            "fn main(){let bad=1/0;}fn test_dependency(){let bad=1/0;}",
        );
        f.write("lib/ops.svr","// λ😀\r\nmod ops {export struct Box<T>{value:T}export fn boxed<T>(x:T)->Box<T>{return Box::<T>{value:x};}export fn fail(){let label=\"λ😀\";let bad=1/0;}fn test_library(){let bad=1/0;}}");
        f
    }
    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.0.join(path), text).unwrap();
    }
    fn root(&self) -> PathBuf {
        self.0.join("app")
    }
    fn capture(&self) -> CapturedPackageSources {
        capture_package_sources(self.root(), PackageCaptureLimits::DEFAULT).unwrap()
    }
    fn suite(&self, limits: GenericPackageTestLimits) -> TestReport {
        test_generic_package(
            self.capture(),
            "independent package",
            GenericPackageCompilationLimits::DEFAULT,
            GenericVmPolicy::DEFAULT,
            limits,
        )
    }
    fn cli(&self, command: &str, flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg(command)
            .arg("--experimental-generics")
            .args(flags)
            .arg(self.root())
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}
#[test]
fn root_tests_run_in_source_order_with_imported_helpers_and_continue_after_runtime_failure() {
    let f = Fixture::new();
    let report = f.suite(GenericPackageTestLimits::DEFAULT);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(report.package);
    assert_eq!(report.target, "independent package");
    assert_eq!(
        report
            .tests
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["test_first", "test_failure", "test_last"]
    );
    assert_eq!(
        report.tests.iter().map(|t| t.passed).collect::<Vec<_>>(),
        [true, false, true]
    );
    assert_eq!(report.tests[1].exit_code, None);
    let error = report.tests[1].error.as_ref().unwrap();
    assert!(error.contains("ops.svr:2:"), "{error}");
    assert!(error.contains("E3125"));
    assert!(error.contains("division by zero"));
    assert!(report.tests.iter().all(|t| t.output.is_empty()));
}
#[test]
fn main_dependency_tests_and_registered_module_tests_are_not_discovered() {
    let f = Fixture::new();
    f.write("app/main.svr", "fn test_only(){}");
    let report = f.suite(GenericPackageTestLimits::DEFAULT);
    assert!(report.success());
    assert_eq!(report.tests.len(), 1);
    f.write("app/main.svr", "fn main(){}");
    let report = f.suite(GenericPackageTestLimits::DEFAULT);
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E5002");
}
#[test]
fn invalid_root_test_signatures_refuse_before_any_vm() {
    let f = Fixture::new();
    for source in [
        "fn test_a(x:Int){}",
        "fn test_a<T>(){}",
        "fn test_a()->Int{return 1;}",
    ] {
        f.write("app/main.svr", source);
        let report = f.suite(GenericPackageTestLimits::DEFAULT);
        assert!(report.tests.is_empty());
        assert_eq!(report.diagnostics.items[0].code, "E5001");
        assert!(report.diagnostics.items[0]
            .source_file
            .as_ref()
            .unwrap()
            .ends_with("app/main.svr"));
    }
}
#[test]
fn later_specialization_failure_prevents_all_test_execution() {
    let f = Fixture::new();
    f.write("app/main.svr","fn grow<T>(x:T){grow::<[T]>([x]);}fn test_first(){let bad=1/0;}fn test_later(){grow::<Int>(1);}");
    let mut compilation = GenericPackageCompilationLimits::DEFAULT;
    compilation.specialization.max_instances = 4;
    let report = test_generic_package(
        f.capture(),
        "suite",
        compilation,
        GenericVmPolicy::DEFAULT,
        GenericPackageTestLimits::DEFAULT,
    );
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E3123");
    assert!(report.diagnostics.items[0]
        .message
        .contains("specialization"));
}
#[test]
fn test_and_two_pass_instance_budgets_have_exact_aggregate_boundaries() {
    let f = Fixture::new();
    let exact = GenericPackageTestLimits {
        max_tests: 3,
        max_instances: 12,
    };
    let report = f.suite(exact);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.tests.len(), 3);
    for limits in [
        GenericPackageTestLimits {
            max_tests: 2,
            ..exact
        },
        GenericPackageTestLimits {
            max_instances: 11,
            ..exact
        },
        GenericPackageTestLimits {
            max_instances: 0,
            ..exact
        },
        GenericPackageTestLimits {
            max_tests: 0,
            ..exact
        },
    ] {
        let report = f.suite(limits);
        assert!(report.tests.is_empty());
        assert!(!report.diagnostics.is_empty());
    }
    f.write("app/main.svr", "fn test_a(){}fn test_b(){}");
    let report = f.suite(GenericPackageTestLimits {
        max_tests: 2,
        max_instances: 4,
    });
    assert!(report.success());
    let report = f.suite(GenericPackageTestLimits {
        max_tests: 2,
        max_instances: 3,
    });
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E5003");
    let source = (0..33)
        .map(|n| format!("fn test_{n}(){{}}"))
        .collect::<String>();
    f.write("app/main.svr", &source);
    let report = f.suite(GenericPackageTestLimits {
        max_tests: usize::MAX,
        max_instances: usize::MAX,
    });
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E5003");
}
#[test]
fn suites_use_captured_bytes_after_inputs_disappear_and_vm_budgets_are_fresh() {
    let f = Fixture::new();
    f.write("app/main.svr", "fn test_a(){}fn test_b(){}");
    let captured = f.capture();
    std::fs::remove_dir_all(&f.0).unwrap();
    let report = test_generic_package(
        captured,
        "captured",
        GenericPackageCompilationLimits::DEFAULT,
        GenericVmPolicy::DEFAULT,
        GenericPackageTestLimits::DEFAULT,
    );
    assert!(report.success());
    let f = Fixture::new();
    f.write("app/main.svr", "fn test_a(){}fn test_b(){}");
    let one = compile_generic_package(
        f.capture(),
        "test_a",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    let cost = execute_generic_package(&one, GenericVmPolicy::DEFAULT)
        .unwrap()
        .instructions();
    let mut exact_policy = GenericVmPolicy::DEFAULT;
    exact_policy.runtime.execution.max_instructions = Some(cost);
    let exact = test_generic_package(
        f.capture(),
        "fresh",
        GenericPackageCompilationLimits::DEFAULT,
        exact_policy,
        GenericPackageTestLimits::DEFAULT,
    );
    assert!(exact.success());
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_work = 0;
    let report = test_generic_package(
        f.capture(),
        "fuel",
        GenericPackageCompilationLimits::DEFAULT,
        policy,
        GenericPackageTestLimits::DEFAULT,
    );
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.tests.len(), 2);
    assert!(report.tests.iter().all(|t| !t.passed));
}
#[test]
fn unused_dependency_body_failure_has_original_file_and_no_partial_outcomes() {
    let f = Fixture::new();
    f.write("lib/ops.svr","mod ops {export struct Box<T>{value:T}export fn boxed<T>(x:T)->Box<T>{return Box::<T>{value:x};}export fn fail(){}fn unused(){missing();}}");
    let report = f.suite(GenericPackageTestLimits::DEFAULT);
    assert!(report.tests.is_empty());
    assert_eq!(report.diagnostics.items[0].code, "E3123");
    assert!(report.diagnostics.items[0]
        .source_file
        .as_ref()
        .unwrap()
        .ends_with("lib/ops.svr"));
}
#[test]
fn locked_cli_suites_publish_schema_one_failures_and_guard_before_execution() {
    let f = Fixture::new();
    let missing = f.cli("test", &["--locked", "--format", "json"]);
    assert_eq!(missing.status.code(), Some(1));
    let report = text(&missing.stdout);
    assert!(report.contains("E4165"));
    assert!(report.contains("\"tests\":[]"));
    assert!(missing.stderr.is_empty());
    assert!(f.cli("lock", &[]).status.success());
    let lock = std::fs::read(f.root().join("sovra.lock")).unwrap();
    let out = f.cli("test", &["--locked", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let json = text(&out.stdout);
    assert!(out.stderr.is_empty());
    assert!(json.contains("\"schema_version\":1"));
    assert!(json.contains("\"kind\":\"package\""));
    assert!(json.contains("\"passed\":2,\"failed\":1,\"total\":3"));
    assert!(json.contains("ops.svr:2:"));
    let source = std::fs::read_to_string(f.0.join("lib/ops.svr")).unwrap();
    f.write("lib/ops.svr", &(source + "\n// changed\n"));
    let out = f.cli("test", &["--locked", "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let json = text(&out.stdout);
    assert!(json.contains("E4165"));
    assert!(json.contains("\"tests\":[]"));
    assert!(!json.contains("E3125"));
    assert_eq!(std::fs::read(f.root().join("sovra.lock")).unwrap(), lock);
}

#[test]
fn explicit_collection_suites_execute_all_roots_with_fresh_policies_and_keep_default_gate() {
    let f = Fixture::new();
    f.write("app/main.svr","fn main(){}fn test_map(){let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,7,\"λ😀\");if std::to_string(full)!=\"std::Map { 7: λ😀 }\"{let bad=1/0;}}fn test_failure(){let s:std::Set<Int>=std::set_new();if std::set_contains(s,1)==false{let bad=1/0;}}fn test_last(){let m:std::Map<Int,Int>=std::map_new();let result=std::map_entries(std::map_insert(m,1,7));let first=result[0];if first.value!=7{let bad=1/0;}}");
    let report = test_generic_collection_package(
        capture_package_sources(f.root(), PackageCaptureLimits::DEFAULT).unwrap(),
        "collections",
        GenericPackageCompilationLimits::DEFAULT,
        GenericVmPolicy::DEFAULT,
        GenericPackageTestLimits::DEFAULT,
    );
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert_eq!(
        report.tests.iter().map(|t| t.passed).collect::<Vec<_>>(),
        [true, false, true]
    );
    assert!(!f
        .suite(GenericPackageTestLimits::DEFAULT)
        .diagnostics
        .is_empty());
}
#[test]
fn collection_suite_late_specialization_refusal_prevents_earlier_vm_execution() {
    let f = Fixture::new();
    f.write("app/main.svr","fn grow<T>(x:T){grow::<[T]>([x]);}fn test_first(){let m:std::Map<Int,Int>=std::map_new();let bad=1/0;}fn test_last(){grow::<Int>(1);}");
    let mut compilation = GenericPackageCompilationLimits::DEFAULT;
    compilation.specialization.max_instances = 4;
    let report = test_generic_collection_package(
        capture_package_sources(f.root(), PackageCaptureLimits::DEFAULT).unwrap(),
        "collections",
        compilation,
        GenericVmPolicy::DEFAULT,
        GenericPackageTestLimits::DEFAULT,
    );
    assert!(report.tests.is_empty());
    assert!(!report.diagnostics.is_empty());
}
