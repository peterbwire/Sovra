//! File-owned generic declarations, source-scoped imports and explicit visibility.
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
        let p = std::env::temp_dir().join(format!(
            "sovra generic binding λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(p.join("app")).unwrap();
        std::fs::create_dir_all(p.join("tools")).unwrap();
        std::fs::write(p.join("app/sovra.toml"),"[project]\nname=\"same\"\nentry=\"main.svr\"\n[modules]\nlocal=\"local.svr\"\n[dependencies.tools]\npath=\"../tools\"\n").unwrap();
        std::fs::write(
            p.join("tools/sovra.toml"),
            "[project]\nname=\"same\"\nentry=\"main.svr\"\n[modules]\nshared=\"shared.svr\"\n",
        )
        .unwrap();
        let f = Self(p);
        f.write(
            "app/main.svr",
            "use tools::shared;struct Box<T>{value:T}fn Box(){}fn root_private(){}fn main(){}",
        );
        f.write(
            "app/local.svr",
            "mod local {fn private(){}export fn public(){} }",
        );
        f.write("tools/main.svr", "fn main(){}");
        f.write("tools/shared.svr","// λ😀\r\nmod shared {\n export fn id<T>(x:T)->T{return x;}fn secret(){}export struct Box<T>{value:T}export enum Choice<T>{One(T),End}type Alias<T>=T;} ");
        f
    }
    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.0.join(path), text).unwrap();
    }
    fn bind(
        &self,
        limits: GenericPackageDeclarationLimits,
    ) -> Result<GenericPackageDeclarations, sovra::compiler::diagnostics::Diagnostics> {
        bind_generic_package_declarations(
            capture_package_sources(self.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            limits,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn source(table: &GenericPackageDeclarations, relative: &str) -> GenericPackageSourceId {
    let index = table
        .inputs()
        .inventory()
        .sources
        .iter()
        .position(|s| s.canonical_path.ends_with(relative))
        .unwrap();
    table.source_id(index).unwrap()
}
#[test]
fn canonical_sources_and_namespaces_keep_identical_spellings_distinct() {
    let f = Fixture::new();
    let table = f.bind(GenericPackageDeclarationLimits::DEFAULT).unwrap();
    let root = source(&table, "app/main.svr");
    let local = table
        .resolve(root, &[], GenericPackageNamespace::Type, "Box")
        .unwrap();
    let remote = table
        .resolve(
            root,
            &["tools", "shared"],
            GenericPackageNamespace::Type,
            "Box",
        )
        .unwrap();
    let function = table
        .resolve(root, &[], GenericPackageNamespace::Function, "Box")
        .unwrap();
    assert_ne!(local, remote);
    assert_ne!(local, function);
    assert_ne!(
        table.declaration(local).unwrap().source(),
        table.declaration(remote).unwrap().source()
    );
    let root_main = table
        .resolve(root, &[], GenericPackageNamespace::Function, "main")
        .unwrap();
    let tools = source(&table, "tools/main.svr");
    assert_ne!(
        root_main,
        table
            .resolve(tools, &[], GenericPackageNamespace::Function, "main")
            .unwrap()
    );
    assert!(table.source_id(usize::MAX).is_none());
    let other = f.bind(GenericPackageDeclarationLimits::DEFAULT).unwrap();
    assert!(other.declaration(local).is_none());
    assert!(other
        .resolve(root, &[], GenericPackageNamespace::Function, "main")
        .is_none());
}
#[test]
fn imports_are_file_scoped_and_private_members_do_not_cross_sources() {
    let f = Fixture::new();
    let table = f.bind(GenericPackageDeclarationLimits::DEFAULT).unwrap();
    let root = source(&table, "app/main.svr");
    let local = source(&table, "app/local.svr");
    let shared = source(&table, "tools/shared.svr");
    let ns = GenericPackageNamespace::Function;
    assert!(table
        .resolve(root, &["tools", "shared"], ns, "id")
        .is_some());
    assert!(table
        .resolve(local, &["tools", "shared"], ns, "id")
        .is_none());
    assert!(table.resolve(root, &["shared"], ns, "id").is_none());
    assert!(table
        .resolve(root, &["tools", "shared"], ns, "secret")
        .is_none());
    assert!(table.resolve(shared, &[], ns, "secret").is_some());
    assert!(table.resolve(shared, &["shared"], ns, "secret").is_some());
    assert!(table.resolve(root, &["local"], ns, "public").is_some());
    assert!(table.resolve(root, &["local"], ns, "private").is_none());
    assert!(table.resolve(local, &[], ns, "root_private").is_none());
    assert!(table
        .resolve(
            root,
            &["tools", "shared"],
            GenericPackageNamespace::Type,
            "Choice"
        )
        .is_some());
    assert!(table
        .resolve(
            root,
            &["tools", "shared"],
            GenericPackageNamespace::Type,
            "Alias"
        )
        .is_none());
    assert!(table
        .resolve(shared, &[], GenericPackageNamespace::Type, "Alias")
        .is_some());
}
#[test]
fn declarations_preserve_original_module_offsets_unicode_and_crlf() {
    let f = Fixture::new();
    let table = f.bind(GenericPackageDeclarationLimits::DEFAULT).unwrap();
    let root = source(&table, "app/main.svr");
    let id = table
        .resolve(
            root,
            &["tools", "shared"],
            GenericPackageNamespace::Function,
            "id",
        )
        .unwrap();
    let declaration = table.declaration(id).unwrap();
    let file = table.source_registration(declaration.source()).unwrap();
    let text = std::fs::read_to_string(&file.canonical_path).unwrap();
    let span = declaration.syntax().span();
    assert_eq!(
        &text[span.start..span.end],
        "export fn id<T>(x:T)->T{return x;}"
    );
    assert_eq!((span.line, span.column), (2, 1));
    for declaration in table.declarations() {
        let registration = table.source_registration(declaration.source()).unwrap();
        let text = std::fs::read_to_string(&registration.canonical_path).unwrap();
        let span = declaration.syntax().span();
        assert!(text[span.start..span.end].contains(declaration.syntax().name()));
    }
}
#[test]
fn import_binding_refuses_missing_indirect_duplicate_and_malformed_imports() {
    let f = Fixture::new();
    for source in [
        "use unknown::shared;fn main(){}",
        "use tools::missing;fn main(){}",
        "use tools::shared;use tools::shared;fn main(){}",
        "use tools.shared;fn main(){}",
        "fn main(){}use tools::shared;",
    ] {
        f.write("app/main.svr", source);
        let error = f
            .bind(GenericPackageDeclarationLimits::DEFAULT)
            .unwrap_err();
        assert_eq!(error.items[0].code, "E3124");
        assert!(error.items[0]
            .source_file
            .as_ref()
            .unwrap()
            .ends_with("app/main.svr"));
    }
    // A dependency's import cannot expose the root's dependency aliases transitively.
    f.write("app/main.svr", "fn main(){}");
    f.write(
        "tools/shared.svr",
        "use tools::shared;mod shared {export fn id<T>(x:T)->T{return x;}}",
    );
    assert!(f.bind(GenericPackageDeclarationLimits::DEFAULT).is_err());
}
#[test]
fn matching_module_wrappers_and_duplicate_local_names_are_required() {
    let f = Fixture::new();
    for text in [
        "mod wrong {fn f(){}}",
        "fn f(){}",
        "mod local {}mod local {}",
        "mod local {fn same(){}fn same(){}}",
        "mod local {struct A<T>{x:T}enum A<T>{One(T)}}",
    ] {
        f.write("app/local.svr", text);
        let error = f
            .bind(GenericPackageDeclarationLimits::DEFAULT)
            .unwrap_err();
        assert!(error.items[0]
            .source_file
            .as_ref()
            .unwrap()
            .ends_with("app/local.svr"));
    }
}
#[test]
fn imports_and_syntax_budgets_are_cumulative_and_refuse_without_a_partial_table() {
    let f = Fixture::new();
    let d = GenericPackageDeclarationLimits::DEFAULT;
    let count = f.bind(d).unwrap().declarations().len();
    assert!(f
        .bind(GenericPackageDeclarationLimits {
            syntax: GenericSourceSyntaxLimits {
                max_declarations: count,
                ..d.syntax
            },
            ..d
        })
        .is_ok());
    assert!(f
        .bind(GenericPackageDeclarationLimits {
            syntax: GenericSourceSyntaxLimits {
                max_declarations: count - 1,
                ..d.syntax
            },
            ..d
        })
        .is_err());

    for limits in [
        GenericPackageDeclarationLimits {
            max_imports: 0,
            ..d
        },
        GenericPackageDeclarationLimits {
            syntax: GenericSourceSyntaxLimits {
                max_declarations: 1,
                ..d.syntax
            },
            ..d
        },
        GenericPackageDeclarationLimits {
            syntax: GenericSourceSyntaxLimits {
                max_work: 0,
                ..d.syntax
            },
            ..d
        },
        GenericPackageDeclarationLimits {
            syntax: GenericSourceSyntaxLimits {
                max_tokens: 0,
                ..d.syntax
            },
            ..d
        },
        GenericPackageDeclarationLimits {
            syntax: GenericSourceSyntaxLimits {
                max_name_bytes: 0,
                ..d.syntax
            },
            ..d
        },
    ] {
        assert!(f.bind(limits).is_err());
    }
    f.write(
        "app/local.svr",
        "use tools::shared;mod local {export fn public(){}}",
    );
    assert!(f
        .bind(GenericPackageDeclarationLimits {
            max_imports: 1,
            ..d
        })
        .is_err());
    assert!(f
        .bind(GenericPackageDeclarationLimits {
            max_imports: 2,
            ..d
        })
        .is_ok());
}
#[test]
fn declaration_import_tables_do_not_claim_to_typecheck_function_bodies() {
    let f = Fixture::new();
    f.write("app/main.svr", "fn main(){missing();}");
    let table = f.bind(GenericPackageDeclarationLimits::DEFAULT).unwrap();
    assert!(!table.declarations().is_empty());
}
