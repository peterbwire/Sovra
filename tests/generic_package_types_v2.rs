//! Core generic type semantics under canonical per-source package visibility.
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
            "sovra package types λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for p in ["app", "tools"] {
            std::fs::create_dir_all(path.join(p)).unwrap();
        }
        let f = Self(path);
        f.write("app/sovra.toml","[project]\nname=\"same\"\nentry=\"main.svr\"\n[modules]\nlocal=\"local.svr\"\n[dependencies.tools]\npath=\"../tools\"\n");
        f.write(
            "tools/sovra.toml",
            "[project]\nname=\"same\"\nentry=\"main.svr\"\n[modules]\nshared=\"shared.svr\"\n",
        );
        f.write("app/main.svr","use tools::shared;struct Box<T>{value:T}type Alias<T>=tools::shared::Box<T>;fn local(x:Box<Int>)->Box<Int>{return x;}fn imported(x:tools::shared::Box<Int>)->Alias<Int>{return x;}fn main(){}");
        f.write("app/local.svr","mod local {export fn same(x:std::Option<Int>)->std::Result<Int,std::Error>{missing();}}");
        f.write("tools/main.svr", "fn main(){}");
        f.write("tools/shared.svr","mod shared {export struct Box<T>{value:T}struct Secret<T>{value:T}type Alias<T>=T;export enum Choice<T>{One(T),End}fn same(x:Box<String>)->Box<String>{return x;}}");
        f
    }
    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.0.join(path), text).unwrap();
    }
    fn check(
        &self,
        limits: GenericPackageTypeLimits,
    ) -> Result<CheckedGenericPackageTypes, sovra::compiler::diagnostics::Diagnostics> {
        let captured =
            capture_package_sources(self.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap();
        let table =
            bind_generic_package_declarations(captured, GenericPackageDeclarationLimits::DEFAULT)
                .unwrap();
        check_generic_package_types(table, limits)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn source(p: &CheckedGenericPackageTypes, relative: &str) -> GenericPackageSourceId {
    let index = p
        .declarations()
        .inputs()
        .inventory()
        .sources
        .iter()
        .position(|s| s.canonical_path.ends_with(relative))
        .unwrap();
    p.declarations().source_id(index).unwrap()
}
fn signature<'a>(
    p: &'a CheckedGenericPackageTypes,
    file: &str,
    name: &str,
) -> &'a CheckedGenericSignature {
    let declaration = p
        .declarations()
        .resolve(
            source(p, file),
            &[],
            GenericPackageNamespace::Function,
            name,
        )
        .unwrap();
    p.signatures()
        .signature(p.signature_id(declaration).unwrap())
        .unwrap()
}
#[test]
fn imported_nominal_types_keep_canonical_identity_and_aliases_expand_in_their_own_scope() {
    let f = Fixture::new();
    let p = f.check(GenericPackageTypeLimits::DEFAULT).unwrap();
    let local = &signature(&p, "app/main.svr", "local").parameter_types()[0];
    let imported = &signature(&p, "app/main.svr", "imported").parameter_types()[0];
    assert!(
        !compare_generic_types(local, imported, GenericTypeEquivalenceLimits::DEFAULT)
            .unwrap()
            .equivalent()
    );
    let GenericSignatureReturn::Explicit(result) =
        signature(&p, "app/main.svr", "imported").return_type()
    else {
        panic!()
    };
    assert!(
        compare_generic_types(imported, result, GenericTypeEquivalenceLimits::DEFAULT)
            .unwrap()
            .equivalent()
    );
    let root = source(&p, "app/main.svr");
    let local_id = p
        .declarations()
        .resolve(root, &[], GenericPackageNamespace::Type, "Box")
        .unwrap();
    let remote_id = p
        .declarations()
        .resolve(
            root,
            &["tools", "shared"],
            GenericPackageNamespace::Type,
            "Box",
        )
        .unwrap();
    assert_ne!(p.type_id(local_id), p.type_id(remote_id));
    assert_ne!(
        signature(&p, "app/local.svr", "same").id(),
        signature(&p, "tools/shared.svr", "same").id()
    );
    assert_ne!(
        signature(&p, "app/main.svr", "main").id(),
        signature(&p, "tools/main.svr", "main").id()
    );
}
#[test]
fn module_alias_targets_use_definition_imports_not_the_callers_imports() {
    let f = Fixture::new();
    f.write("tools/shared.svr","mod shared {type Hidden<T>=T;export struct Box<T>{value:Hidden<T>}export fn id<T>(x:Box<T>)->Box<T>{return x;}}");
    let p = f.check(GenericPackageTypeLimits::DEFAULT).unwrap();
    let root = source(&p, "app/main.svr");
    let id = p
        .declarations()
        .resolve(
            root,
            &["tools", "shared"],
            GenericPackageNamespace::Type,
            "Box",
        )
        .unwrap();
    let CheckedGenericDefinition::Record(record) =
        p.types().definition(p.type_id(id).unwrap()).unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        record.field_types()[0].nodes()[record.field_types()[0].root()].kind,
        ResolvedGenericTypeKind::Parameter(_)
    ));
}
#[test]
fn unimported_private_missing_and_wrong_arity_types_refuse_in_original_file() {
    let f = Fixture::new();
    for text in [
        "fn main(x:tools::shared::Box<Int>){}",
        "use tools::shared;fn main(x:tools::shared::Secret<Int>){}",
        "use tools::shared;fn main(x:tools::shared::Box<Int,String>){}",
        "use tools::shared;fn main(x:tools::shared::Missing<Int>){}",
    ] {
        f.write("app/main.svr", text);
        let errors = f.check(GenericPackageTypeLimits::DEFAULT).unwrap_err();
        assert_eq!(errors.items[0].code, "E3123");
        assert!(errors.items[0]
            .source_file
            .as_ref()
            .unwrap()
            .ends_with("app/main.svr"));
    }
}
#[test]
fn imports_do_not_leak_into_sibling_module_annotations() {
    let f = Fixture::new();
    f.write(
        "app/local.svr",
        "mod local {fn same(x:tools::shared::Box<Int>) {}}",
    );
    let errors = f.check(GenericPackageTypeLimits::DEFAULT).unwrap_err();
    assert!(errors.items[0]
        .source_file
        .as_ref()
        .unwrap()
        .ends_with("app/local.svr"));
    f.write(
        "app/local.svr",
        "use tools::shared;mod local {fn same(x:tools::shared::Box<Int>) {}}",
    );
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_ok());
}
#[test]
fn imported_fields_payloads_and_key_bounds_use_existing_core_validation() {
    let f = Fixture::new();
    f.write(
        "tools/shared.svr",
        "mod shared {export struct Box<T:std::Key>{value:T}}",
    );
    f.write(
        "app/main.svr",
        "use tools::shared;fn main(x:tools::shared::Box<Int>){}",
    );
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_ok());
    f.write(
        "app/main.svr",
        "use tools::shared;fn main(x:tools::shared::Box<Float>){}",
    );
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_err());
    f.write("app/main.svr","use tools::shared;fn f<T:std::Key>(x:tools::shared::Box<T>)->tools::shared::Box<T>{return x;}fn main(){}");
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_ok());
    f.write(
        "app/main.svr",
        "use tools::shared;struct Container<T>{value:tools::shared::Box<T>}fn main(){}",
    );
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_err());
    f.write(
        "app/main.svr",
        "use tools::shared;enum Container<T:std::Key>{One(tools::shared::Box<T>),End}fn main(){}",
    );
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_ok());
}
#[test]
fn package_body_checking_and_unscoped_function_lookup_stay_gated() {
    let f = Fixture::new();
    let p = f.check(GenericPackageTypeLimits::DEFAULT).unwrap();
    assert!(p.signatures().find("main").is_none());
    assert!(check_generic_copy_bodies(p.signatures(), GenericCopyBodyLimits::DEFAULT).is_err());
    assert!(check_generic_propagation_bodies(
        p.signatures(),
        GenericCopyBodyLimits::DEFAULT,
        GenericPropagationLimits::DEFAULT
    )
    .is_err());
    let other = f.check(GenericPackageTypeLimits::DEFAULT).unwrap();
    let id = p
        .declarations()
        .resolve(
            source(&p, "app/main.svr"),
            &[],
            GenericPackageNamespace::Type,
            "Box",
        )
        .unwrap();
    assert!(other.type_id(id).is_none());
    let other_signature = signature(&other, "app/main.svr", "imported");
    assert!(p
        .types()
        .environment()
        .expand(
            other_signature.syntax().value_parameters()[0]
                .ty
                .annotation
                .syntax(),
            other_signature.syntax().parameters(),
            GenericAliasExpansionLimits::DEFAULT
        )
        .is_err());
}
#[test]
fn package_type_and_signature_admission_is_cumulative_and_finite() {
    let f = Fixture::new();
    let d = GenericPackageTypeLimits::DEFAULT;
    for limits in [
        GenericPackageTypeLimits { max_scopes: 0, ..d },
        GenericPackageTypeLimits {
            max_context_work: 0,
            ..d
        },
        GenericPackageTypeLimits {
            registry: GenericRegistryLimits {
                max_declarations: 1,
                ..d.registry
            },
            ..d
        },
        GenericPackageTypeLimits {
            aliases: GenericAliasEnvironmentLimits {
                max_aliases: 0,
                ..d.aliases
            },
            ..d
        },
        GenericPackageTypeLimits {
            signatures: GenericSignatureLimits {
                max_functions: 2,
                ..d.signatures
            },
            ..d
        },
        GenericPackageTypeLimits {
            declarations: GenericDeclarationLimits {
                max_members: 0,
                ..d.declarations
            },
            ..d
        },
    ] {
        assert!(f.check(limits).is_err());
    }
}
#[test]
fn alias_cycles_and_same_named_aliases_in_separate_modules_are_checked() {
    let f = Fixture::new();
    f.write("app/main.svr", "type Alias<T>=T;fn main(){}");
    assert!(f.check(GenericPackageTypeLimits::DEFAULT).is_ok());
    f.write(
        "tools/shared.svr",
        "mod shared {type Alias<T>=Loop<T>;type Loop<T>=Alias<T>;export struct Box<T>{value:T}}",
    );
    let errors = f.check(GenericPackageTypeLimits::DEFAULT).unwrap_err();
    assert!(errors.items[0]
        .source_file
        .as_ref()
        .unwrap()
        .ends_with("tools/shared.svr"));
    assert!(errors.items[0].message.contains("Cycle"));
}
