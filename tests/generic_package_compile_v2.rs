//! Owned selected-root package compilation, CLI graph consumers and original-file diagnostics.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{diagnostics::Span, project::packages::*, v2::*};
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
            "sovra compiled package λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for name in ["app", "lib"] {
            std::fs::create_dir_all(path.join(name)).unwrap();
        }
        let f = Self(path);
        f.write(
            "app/sovra.toml",
            "[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.lib]\npath=\"../lib\"\n",
        );
        f.write(
            "lib/sovra.toml",
            "[project]\nname=\"lib\"\nentry=\"main.svr\"\n[modules]\nops=\"ops.svr\"\n",
        );
        f.write("app/main.svr","use lib::ops;fn main()->Int{let x=lib::ops::wrap::<Int>(21);return x.value*2;}fn selected()->Int{return 17;}");
        f.write("lib/main.svr", "fn main()->Int{return 999;}");
        f.write("lib/ops.svr","// λ😀\r\nmod ops {export struct Box<T>{value:T}export fn wrap<T>(x:T)->Box<T>{return Box::<T>{value:x};}export fn fail()->Int{let label=\"λ😀\";return 1/0;}}");
        f
    }
    fn root(&self) -> PathBuf {
        self.0.join("app")
    }
    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.0.join(path), text).unwrap();
    }
    fn capture(&self) -> CapturedPackageSources {
        capture_package_sources(self.root(), PackageCaptureLimits::DEFAULT).unwrap()
    }
    fn compile(
        &self,
        entry: &str,
        limits: GenericPackageCompilationLimits,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_package(self.capture(), entry, limits)
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
fn compile_selects_only_root_entry_and_runs_retained_bytes_after_inputs_are_removed() {
    let f = Fixture::new();
    let program = f
        .compile("main", GenericPackageCompilationLimits::DEFAULT)
        .unwrap();
    let other = f
        .compile("selected", GenericPackageCompilationLimits::DEFAULT)
        .unwrap();
    assert_eq!(
        execute_generic_package(&other, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_int(),
        Some(17)
    );
    assert!(program.validate_command_entry().is_err());
    let program = program.clone();
    std::fs::remove_dir_all(&f.0).unwrap();
    assert_eq!(
        execute_generic_package(&program, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_int(),
        Some(42)
    );
}
#[test]
fn missing_qualified_generic_and_parameterized_roots_refuse() {
    let f = Fixture::new();
    for name in ["missing", "lib::ops::wrap"] {
        assert!(f
            .compile(name, GenericPackageCompilationLimits::DEFAULT)
            .is_err());
    }
    f.write("app/main.svr", "fn selected<T>(){}fn main(){}");
    assert!(f
        .compile("selected", GenericPackageCompilationLimits::DEFAULT)
        .is_err());
    f.write("app/main.svr", "fn selected(x:Int){}fn main(){}");
    assert!(f
        .compile("selected", GenericPackageCompilationLimits::DEFAULT)
        .is_err());
    assert!(f
        .compile(&"x".repeat(1025), GenericPackageCompilationLimits::DEFAULT)
        .is_err());
}
#[test]
fn runtime_failure_maps_to_original_dependency_file_and_scalar_column() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        "use lib::ops;fn main()->Int{return lib::ops::fail();}",
    );
    let program = f
        .compile("main", GenericPackageCompilationLimits::DEFAULT)
        .unwrap();
    let original = std::fs::read_to_string(f.0.join("lib/ops.svr")).unwrap();
    f.write("lib/ops.svr", "changed on disk");
    let errors = execute_generic_package(&program, GenericVmPolicy::DEFAULT).unwrap_err();
    let error = &errors.items[0];
    assert_eq!(error.code, "E3125");
    assert!(error.source_file.as_ref().unwrap().ends_with("lib/ops.svr"));
    assert_eq!(&original[error.span.start..error.span.end], "1/0");
    assert_eq!(error.span.line, 1);
    assert_eq!(
        error.span.column,
        original[..error.span.start]
            .rsplit('\n')
            .next()
            .unwrap()
            .chars()
            .count()
    );
}
#[test]
fn public_location_refuses_out_of_range_indices_and_utf8_interiors_and_ignores_supplied_coordinates(
) {
    let f = Fixture::new();
    let program = f
        .compile("main", GenericPackageCompilationLimits::DEFAULT)
        .unwrap();
    let span = Span {
        start: 0,
        end: 1,
        line: usize::MAX,
        column: usize::MAX,
    };
    assert!(program.location(usize::MAX, span).is_none());
    let location = program.location(0, span).unwrap();
    assert!(location.source_file.ends_with("app/main.svr"));
    assert_eq!(location.span.start, "use lib::ops;".len());
    assert_eq!(location.span.line, 0);
    assert!(program
        .location(
            0,
            Span {
                start: 0,
                end: usize::MAX,
                ..span
            }
        )
        .is_none());
    f.write("app/main.svr", "fn main(){let label=\"λ😀\";}");
    let program = f
        .compile("main", GenericPackageCompilationLimits::DEFAULT)
        .unwrap();
    let syntax = program.bodies().types().declarations().declarations()[0].syntax();
    let GenericSourceDeclarationKind::Function(function) = syntax.kind() else {
        panic!()
    };
    let interior = function.source().find('λ').unwrap() + 1;
    assert!(program
        .location(
            0,
            Span {
                start: interior,
                end: interior + 1,
                ..span
            }
        )
        .is_none());
}
#[test]
fn admission_refusals_and_runtime_policy_have_no_unbounded_fallback() {
    let f = Fixture::new();
    let d = GenericPackageCompilationLimits::DEFAULT;
    for limits in [
        GenericPackageCompilationLimits {
            specialization: GenericSpecializationLimits {
                max_instances: 0,
                ..d.specialization
            },
            ..d
        },
        GenericPackageCompilationLimits {
            literals: GenericLiteralLimits {
                max_work: 0,
                ..d.literals
            },
            ..d
        },
        GenericPackageCompilationLimits {
            operations: GenericPropagationLayoutLimits {
                max_work: 0,
                ..d.operations
            },
            ..d
        },
    ] {
        assert!(f.compile("main", limits).is_err());
    }
    let program = f.compile("main", d).unwrap();
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_work = 0;
    assert_eq!(
        execute_generic_package(&program, policy).unwrap_err().items[0].code,
        "E3125"
    );
}
#[test]
fn locked_cli_check_run_and_graph_lock_verification_consume_generic_dependencies() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        "use lib::ops;fn main(){let x=lib::ops::wrap::<Int>(21);if x.value!=21{let bad=1/0;}}",
    );
    let out = f.cli("lock", &[]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let lock = std::fs::read(f.root().join("sovra.lock")).unwrap();
    assert!(text(&lock).contains("ops.svr"));
    let out = f.cli("check", &["--locked", "--format", "json"]);
    assert!(
        out.status.success(),
        "{} {}",
        text(&out.stdout),
        text(&out.stderr)
    );
    assert!(text(&out.stdout).contains("\"kind\":\"project\",\"success\":true"));
    assert!(out.stderr.is_empty());
    let out = f.cli("run", &["--locked"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
    assert!(f.cli("lock", &["--check"]).status.success());
    assert_eq!(std::fs::read(f.root().join("sovra.lock")).unwrap(), lock);
    f.write("lib/ops.svr","mod ops {export struct Box<T>{value:T}export fn wrap<T>(x:T)->Box<T>{let bad=1/0;return Box::<T>{value:x};}}");
    let out = f.cli("run", &["--locked"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E4165"));
    assert!(!text(&out.stderr).contains("E3125"));
    assert_eq!(std::fs::read(f.root().join("sovra.lock")).unwrap(), lock);
    let out = f.cli("run", &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E3125"));
    assert!(text(&out.stderr).contains("ops.svr"));
    assert!(f.cli("check", &["--format", "json"]).status.success());
}
#[test]
fn invalid_unused_dependency_diagnostics_and_missing_locks_precede_execution() {
    let f = Fixture::new();
    f.write("app/main.svr", "fn main(){let bad=1/0;}");
    let out = f.cli("run", &["--locked"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E4165"));
    assert!(!text(&out.stderr).contains("E3125"));
    f.write("lib/ops.svr", "mod ops {fn unused(){missing();}}");
    let out = f.cli("check", &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());
    let json = text(&out.stdout);
    assert!(json.contains("E3123"));
    assert!(json.contains("ops.svr"));
}
#[test]
fn multi_file_tests_execute_and_invalid_native_entry_refuses_without_side_effects() {
    let f = Fixture::new();
    f.write("app/main.svr", "fn main(){}fn test_ok(){}");
    let out = f.cli("test", &["--format", "json"]);
    assert!(
        out.status.success(),
        "{} {}",
        text(&out.stdout),
        text(&out.stderr)
    );
    assert!(text(&out.stdout).contains("\"passed\":1,\"failed\":0,\"total\":1"));
    f.write("app/main.svr", "fn main()->Int{return 1;}");
    let output = f.0.join("absent/output");
    let out = f.cli(
        "build",
        &["--emit", "exe", "--output", output.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("E3123"));
    assert!(!output.parent().unwrap().exists());
    assert_eq!(
        f.cli("check", &["--format", "json", "--symbols"])
            .status
            .code(),
        Some(2)
    );
}
