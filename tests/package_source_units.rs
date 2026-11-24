//! Owned registered source parsing: exact spans, header admission and context separation.
use sovra::compiler::project::packages;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "sovra-source-units-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn package(&self, name: &str, modules: &str, deps: &str) -> PathBuf {
        let root = self.0.join(name);
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            format!("[project]\nname=\"{name}\"\nentry=\"main.svr\"\n{modules}{deps}"),
        )
        .unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        root
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
)))]
#[test]
fn unsupported_guarded_capture_is_explicit() {
    let fixture = Fixture::new();
    let root = fixture.package("app", "", "");
    let errors = packages::load_package_sources(root).unwrap_err();
    assert!(errors
        .items
        .iter()
        .any(|d| matches!(d.code, "E4160" | "E4161") && d.message.contains("unavailable")));
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use super::*;
    #[test]
    fn source_units_keep_original_buffers_ast_spans_and_file_local_imports() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "[modules]\nlocal=\"local.svr\"\n",
            "[dependencies.library]\npath=\"../library\"\n",
        );
        let library = fixture.package("library", "[modules]\nmath=\"math.svr\"\n", "");
        let entry = "// λ\r\nuse library::math;\r\nfn main(){print(library::math::square(7));}";
        let local = "// λ local\r\nmod local{export fn value()->Int{return 3;}}";
        std::fs::write(root.join("main.svr"), entry).unwrap();
        std::fs::write(root.join("local.svr"), local).unwrap();
        std::fs::write(library.join("main.svr"), "").unwrap();
        std::fs::write(
            library.join("math.svr"),
            "mod math{export fn square(value:Int)->Int{return value*value;}}",
        )
        .unwrap();
        let loaded = packages::load_package_sources(&root).unwrap();
        assert_eq!(loaded.units().len(), 4);
        assert!(loaded.token_count() > 0);
        assert!(loaded.ast_nodes() > 0);
        let unit = &loaded.units()[0];
        assert_eq!(unit.source(), entry);
        assert_eq!(unit.imports()[0].alias, "library");
        let span = unit.imports()[0].span;
        assert_eq!(&entry[span.start..span.end], "use library::math;");
        assert_eq!(span.line, 1);
        let unit = &loaded.units()[1];
        assert_eq!(unit.source(), local);
        assert!(unit.imports().is_empty());
        assert_eq!(
            unit.program(),
            &sovra::compiler::parser::Parser::new()
                .parse_source(local)
                .unwrap()
        );
        let registration = &loaded.inventory().sources[unit.source_index()];
        assert_eq!(registration.package_root, root.canonicalize().unwrap());
        std::fs::write(root.join("local.svr"), "mod local{fn changed(){}}").unwrap();
        assert_eq!(unit.source(), local);
        assert_eq!(unit.program().modules[0].functions[0].name, "value");
        assert!(packages::compile(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.code == "E4010"));
    }
    #[test]
    fn matching_header_shape_and_duplicate_module_names_are_enforced() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nmath=\"math.svr\"\n", "");
        for source in [
            "",
            "mod wrong{}",
            "mod math{} mod extra{}",
            "fn root(){} mod math{}",
            "type Root=Int; mod math{}",
            "struct Root{} mod math{}",
        ] {
            std::fs::write(root.join("math.svr"), source).unwrap();
            let errors = packages::load_package_sources(&root).unwrap_err();
            assert_eq!(errors.items[0].code, "E4162", "{source}");
            assert_eq!(
                errors.items[0].source_file.as_deref(),
                Some(root.join("math.svr").to_str().unwrap())
            );
        }
        std::fs::write(root.join("math.svr"), "mod math{}").unwrap();
        std::fs::write(root.join("main.svr"), "mod math{}").unwrap();
        let errors = packages::load_package_sources(&root).unwrap_err();
        assert_eq!(errors.items[0].code, "E3008");
        assert_eq!(
            errors.items[0].source_file.as_deref(),
            Some(root.join("math.svr").to_str().unwrap())
        );
    }
    #[test]
    fn lexical_parser_and_utf8_failures_keep_their_owning_file() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nmath=\"math.svr\"\n", "");
        for source in [
            "// λ\r\nmod math{fn bad(){let value=;}}",
            "// λ\r\nmod math{fn bad(){print(@);}}",
            "use malformed;\nmod math{}",
        ] {
            std::fs::write(root.join("math.svr"), source).unwrap();
            let errors = packages::load_package_sources(&root).unwrap_err();
            assert!(
                errors
                    .items
                    .iter()
                    .all(|d| d.source_file.as_deref()
                        == Some(root.join("math.svr").to_str().unwrap()))
            );
            assert!(errors.items.iter().any(|d| d.span.start < source.len()));
        }
        std::fs::write(root.join("math.svr"), [0xff]).unwrap();
        assert!(packages::load_package_sources(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.code == "E4161" && d.message.contains("UTF-8")));
    }
    #[test]
    fn parsing_never_claims_semantic_body_or_import_resolution_success() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nmath=\"math.svr\"\n", "");
        std::fs::write(
            root.join("math.svr"),
            "use undeclared::missing;\nmod math{export fn bad()->Int{return true;}}",
        )
        .unwrap();
        let loaded = packages::load_package_sources(&root).unwrap();
        let unit = &loaded.units()[1];
        assert_eq!(unit.imports()[0].alias, "undeclared");
        assert_eq!(
            unit.program().modules[0].functions[0]
                .return_type
                .as_deref(),
            Some("Int")
        );
    }
    #[test]
    fn manifests_retain_the_exact_parsed_bytes_and_declared_dependency_paths() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "", "[dependencies.library]\npath=\"../library\"\n");
        fixture.package("library", "", "");
        let text = std::fs::read_to_string(root.join("sovra.toml")).unwrap();
        let text = format!("# λ\r\n{}", text.replace('\n', "\r\n"));
        std::fs::write(root.join("sovra.toml"), &text).unwrap();
        let loaded = packages::load_package_sources(&root).unwrap();
        assert_eq!(loaded.manifests().len(), 2);
        let manifest = &loaded.manifests()[0];
        assert_eq!(manifest.source(), text);
        assert_eq!(manifest.dependencies()["library"], "../library");
        assert_eq!(manifest.package_root(), root.canonicalize().unwrap());
        assert_eq!(manifest.sha256().len(), 64);
        let digest = manifest.sha256();
        std::fs::write(root.join("sovra.toml"), "changed").unwrap();
        assert_eq!(manifest.source(), text);
        assert_eq!(manifest.sha256(), digest);
    }
}
