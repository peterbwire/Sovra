//! Canonical package body calls and nominal operations through the generic VM.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{project::packages::*, v2::*};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra package bodies λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for name in ["app", "tools", "core"] {
            std::fs::create_dir_all(path.join(name)).unwrap();
        }
        let f = Self(path);
        f.write("app/sovra.toml","[project]\nname=\"app\"\nentry=\"main.svr\"\n[modules]\nlocal=\"local.svr\"\n[dependencies.tools]\npath=\"../tools\"\n");
        f.write("tools/sovra.toml","[project]\nname=\"tools\"\nentry=\"main.svr\"\n[modules]\nshared=\"shared.svr\"\n[dependencies.core]\npath=\"../core\"\n");
        f.write(
            "core/sovra.toml",
            "[project]\nname=\"core\"\nentry=\"main.svr\"\n[modules]\nvalues=\"values.svr\"\n",
        );
        f.write("app/main.svr","use tools::shared;struct Box<T>{value:T}fn main()->Int{let boxed=tools::shared::wrap::<Int>(7);return boxed.value;}");
        f.write(
            "app/local.svr",
            "mod local {export fn get()->Int{return 1;}}",
        );
        f.write("tools/main.svr", "fn main(){}");
        f.write("tools/shared.svr","use core::values;mod shared {export struct Box<T>{value:T}export enum Choice<T>{One(T),End}fn private(x:Int)->Int{return x;}fn id<T>(x:T)->T{return x;}export fn answer()->Int{return shared::private(42);}export fn wrap<T>(x:T)->Box<T>{let y=core::values::id::<T>(x);return Box::<T>{value:id::<T>(y)};}}");
        f.write("core/main.svr", "fn main(){}");
        f.write(
            "core/values.svr",
            "mod values {export fn id<T>(x:T)->T{return x;}}",
        );
        f
    }
    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.0.join(path), text).unwrap();
    }
    fn check(
        &self,
        limits: GenericCopyBodyLimits,
    ) -> Result<CheckedGenericPackageBodies, sovra::compiler::diagnostics::Diagnostics> {
        let raw =
            capture_package_sources(self.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap();
        let declarations =
            bind_generic_package_declarations(raw, GenericPackageDeclarationLimits::DEFAULT)
                .unwrap();
        let types =
            check_generic_package_types(declarations, GenericPackageTypeLimits::DEFAULT).unwrap();
        check_generic_package_bodies(types, limits, GenericPropagationLimits::DEFAULT)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn execute(p: &CheckedGenericPackageBodies) -> GenericVmOutcome {
    let types = p.types();
    let table = types.declarations();
    let entry = table
        .resolve(
            table.source_id(0).unwrap(),
            &[],
            GenericPackageNamespace::Function,
            "main",
        )
        .unwrap();
    let root = types.signature_id(entry).unwrap();
    let graph = discover_generic_propagation_specializations(
        p.bodies(),
        root,
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap();
    let concrete = recheck_generic_propagation_concrete_bodies(
        &graph,
        GenericConcreteBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap();
    let literals = decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap();
    let layouts = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    let operations =
        plan_generic_propagation_layouts(&layouts, GenericPropagationLayoutLimits::DEFAULT)
            .unwrap();
    let ir = lower_generic_propagation_ir(
        &operations,
        GenericConstructionLimits::DEFAULT,
        GenericIrLimits::DEFAULT,
    )
    .unwrap();
    execute_generic_propagation_ir(&ir, GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn imported_generic_calls_transitive_helpers_and_remote_records_execute() {
    let f = Fixture::new();
    let p = f.check(GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(execute(&p).value().as_int(), Some(7));
    let calls = p
        .bodies()
        .bodies()
        .iter()
        .flat_map(|b| b.expressions())
        .filter(|e| matches!(e.kind, GenericCopyExpressionKind::Call { .. }))
        .count();
    assert_eq!(calls, 4);
    let table = p.types().declarations();
    for body in p.bodies().bodies() {
        let caller = table
            .declarations()
            .iter()
            .find(|d| p.types().signature_id(d.id()) == Some(body.signature().id()))
            .unwrap();
        for expression in body.expressions() {
            if let GenericCopyExpressionKind::Call {
                signature,
                callee_span,
                ..
            } = &expression.kind
            {
                let text = &body.signature().syntax().source()[callee_span.start..callee_span.end];
                let parts: Vec<_> = text.split("::").collect();
                let target = table
                    .resolve(
                        caller.source(),
                        &parts[..parts.len() - 1],
                        GenericPackageNamespace::Function,
                        parts[parts.len() - 1],
                    )
                    .unwrap();
                assert_eq!(
                    Some(signature.binding().callee()),
                    p.types().signature_id(target)
                );
            }
        }
    }
}
#[test]
fn ordinary_imported_and_same_module_private_calls_execute_with_original_path_facts() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        "use tools::shared;fn main()->Int{return (tools::shared::answer)()+local::get();}",
    );
    let p = f.check(GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(execute(&p).value().as_int(), Some(43));
    for body in p.bodies().bodies() {
        for expression in body.expressions() {
            if let GenericCopyExpressionKind::Call { callee_span, .. } = &expression.kind {
                let text = &body.signature().syntax().source()[callee_span.start..callee_span.end];
                assert!(!text.contains('('));
                assert!(!text.contains('<'));
                assert!(!text.is_empty());
            }
        }
    }
}
#[test]
fn imported_enum_construction_and_exhaustive_patterns_keep_nominal_owners() {
    let f = Fixture::new();
    f.write("app/main.svr","use tools::shared;enum Choice<T>{One(T),End}fn main()->Int{let x=tools::shared::Choice::<Int>::One(9);match x {tools::shared::Choice::One(value)=>{return value;},tools::shared::Choice::End()=>{return 0;}}}");
    let p = f.check(GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(execute(&p).value().as_int(), Some(9));
    f.write("app/main.svr","use tools::shared;enum Choice<T>{One(T),End}fn main()->Int{let x=tools::shared::Choice::<Int>::One(9);match x {Choice::One(value)=>{return value;},Choice::End()=>{return 0;}}}");
    assert!(f.check(GenericCopyBodyLimits::DEFAULT).is_err());
}
#[test]
fn scoped_local_annotations_and_standard_propagation_work_in_imported_helpers() {
    let f = Fixture::new();
    f.write("tools/shared.svr","mod shared {export fn get<T>(x:std::Option<T>)->std::Option<T>{return std::Option::<T>::Some(x?);}}");
    f.write("app/main.svr","use tools::shared;fn main()->Int{let x:std::Option<Int>=tools::shared::get::<Int>(std::Option::<Int>::Some(8));match x {std::Option::Some(value)=>{return value;},std::Option::None()=>{return 0;}}}");
    let p = f.check(GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(execute(&p).value().as_int(), Some(8));
}
#[test]
fn inaccessible_calls_and_imports_never_fall_back_to_global_names() {
    let f = Fixture::new();
    for text in [
        "fn main()->Int{return tools::shared::answer();}",
        "use tools::shared;fn main()->Int{return tools::shared::private(1);}",
        "use tools::shared;fn main()->Int{return core::values::id::<Int>(1);}",
        "use tools::shared;fn main()->Int{let tools=1;return tools::shared::answer();}",
        "use tools::shared;fn main()->Int{return id::<Int>(1);}",
    ] {
        f.write("app/main.svr", text);
        let errors = f.check(GenericCopyBodyLimits::DEFAULT).unwrap_err();
        assert_eq!(errors.items[0].code, "E3123");
        assert!(errors.items[0]
            .source_file
            .as_ref()
            .unwrap()
            .ends_with("app/main.svr"));
    }
}
#[test]
fn invalid_unused_dependency_bodies_and_sibling_import_leaks_are_rejected() {
    let f = Fixture::new();
    f.write(
        "core/values.svr",
        "// λ😀\r\nmod values {export fn id<T>(x:T)->T{return x;}fn bad(){missing();}}",
    );
    let errors = f.check(GenericCopyBodyLimits::DEFAULT).unwrap_err();
    assert!(errors.items[0]
        .source_file
        .as_ref()
        .unwrap()
        .ends_with("core/values.svr"));
    assert_eq!(errors.items[0].span.line, 1);
    f.write(
        "core/values.svr",
        "mod values {export fn id<T>(x:T)->T{return x;}}",
    );
    f.write(
        "app/local.svr",
        "mod local {export fn get()->Int{return tools::shared::answer();}}",
    );
    assert!(f.check(GenericCopyBodyLimits::DEFAULT).is_err());
}
#[test]
fn imported_record_constructors_and_nominal_argument_mismatches_use_scoped_types() {
    let f = Fixture::new();
    f.write("app/main.svr","use tools::shared;fn main()->Int{let x=tools::shared::Box::<Int>{value:12};return x.value;}");
    let p = f.check(GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(execute(&p).value().as_int(), Some(12));
    f.write("app/main.svr","use tools::shared;struct Box<T>{value:T}fn local(x:Box<Int>)->Int{return x.value;}fn main()->Int{let x=tools::shared::wrap::<Int>(1);return local(x);}");
    assert!(f.check(GenericCopyBodyLimits::DEFAULT).is_err());
}
#[test]
fn whole_package_call_and_body_limits_do_not_reset_between_sources() {
    let f = Fixture::new();
    let defaults = GenericCopyBodyLimits::DEFAULT;
    assert!(f
        .check(GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_calls: 4,
                ..defaults.calls
            },
            ..defaults
        })
        .is_ok());
    assert!(f
        .check(GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_calls: 3,
                ..defaults.calls
            },
            ..defaults
        })
        .is_err());

    for limits in [
        GenericCopyBodyLimits {
            max_work: 0,
            ..defaults
        },
        GenericCopyBodyLimits {
            max_functions: 2,
            ..defaults
        },
        GenericCopyBodyLimits {
            calls: GenericBodyCallLimits {
                max_calls: 1,
                ..defaults.calls
            },
            ..defaults
        },
    ] {
        assert!(f.check(limits).is_err());
    }
}
