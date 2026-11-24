//! Opt-in module registration: bounds, source ownership, containment and old API refusal.
use sovra::compiler::project::packages;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra-source-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn package(&self, name: &str, mappings: &str, deps: &str) -> PathBuf {
        let root = self.0.join(name);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        self.manifest(name, mappings, deps);
        root
    }
    fn manifest(&self, name: &str, mappings: &str, deps: &str) {
        let modules = if mappings.is_empty() {
            String::new()
        } else {
            format!("[modules]\n{mappings}\n")
        };
        std::fs::write(
            self.0.join(name).join("sovra.toml"),
            format!("# λ\r\n[project]\r\nname=\"same\"\r\nentry=\"main.svr\"\r\n{modules}{deps}"),
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn old_loader_and_compiler_refuse_new_registrations_instead_of_ignoring_them() {
    let fixture = Fixture::new();
    let root = fixture.package("app", "math=\"math.svr\"", "");
    std::fs::write(root.join("math.svr"), "mod math{}").unwrap();
    for error in [
        packages::resolve(&root).unwrap_err(),
        packages::compile(&root).unwrap_err(),
    ] {
        assert!(error
            .items
            .iter()
            .any(|d| d.code == "E4010" && d.message.contains("modules")));
    }
}
#[cfg(not(unix))]
#[test]
fn unavailable_file_identity_is_explicit_on_this_host() {
    let fixture = Fixture::new();
    let root = fixture.package("app", "", "");
    let errors = packages::inspect_source_inventory(&root).unwrap_err();
    assert!(errors
        .items
        .iter()
        .any(|d| d.code == "E4160" && d.message.contains("unavailable")));
}
#[cfg(unix)]
mod unix {
    use super::*;
    use packages::{PackageSourceKind, MAX_INVENTORY_SOURCES, MAX_PACKAGE_MODULES};
    #[test]
    fn inventories_are_deterministic_owned_deduplicated_and_do_not_parse_sources() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "",
            "[dependencies.z]\npath=\"../library\"\n[dependencies.a]\npath=\"../library\"\n",
        );
        let library = fixture.package("library", "zeta=\"z.svr\"\nalpha=\"a.svr\"", "");
        std::fs::write(library.join("a.svr"), [0xff]).unwrap(); // Inventory is not UTF-8/source validation.
        std::fs::write(library.join("z.svr"), "mod WrongHeader{}").unwrap();
        std::fs::write(library.join("ignored.svr"), "fn broken(){let =;}").unwrap();
        let first = packages::inspect_source_inventory(&root).unwrap();
        assert_eq!(first, packages::inspect_source_inventory(&root).unwrap());
        assert_eq!(first.graph.packages.len(), 2);
        assert_eq!(first.sources.len(), 4);
        assert_eq!(
            first
                .sources
                .iter()
                .map(|s| s.kind.clone())
                .collect::<Vec<_>>(),
            [
                PackageSourceKind::Entry,
                PackageSourceKind::Entry,
                PackageSourceKind::Module("alpha".into()),
                PackageSourceKind::Module("zeta".into())
            ]
        );
        assert_eq!(
            first.sources[2].package_root,
            library.canonicalize().unwrap()
        );
        assert_eq!(first.sources[2].observed_bytes, 1);
        let manifest = std::fs::read_to_string(library.join("sovra.toml")).unwrap();
        let span = first.sources[2].manifest_span;
        assert_eq!(&manifest[span.start..span.end], "alpha=\"a.svr\"");
        assert!(first
            .sources
            .iter()
            .all(|s| s.canonical_path.starts_with(&s.package_root)));
    }
    #[test]
    fn entry_only_graph_identity_and_cycles_retain_existing_rules() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "", "[dependencies.library]\npath=\"../library\"\n");
        fixture.package("library", "", "");
        let inventory = packages::inspect_source_inventory(&root).unwrap();
        assert_eq!(inventory.graph, packages::resolve(&root).unwrap());
        assert_eq!(inventory.sources.len(), 2);
        fixture.manifest("library", "", "[dependencies.app]\npath=\"../app\"\n");
        let old = packages::resolve(&root).unwrap_err();
        let new = packages::inspect_source_inventory(&root).unwrap_err();
        assert_eq!(new, old);
    }
    #[test]
    fn malformed_keys_paths_and_case_collisions_have_manifest_owned_errors() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "", "");
        for mapping in [
            "bad-name=\"x.svr\"",
            "math=unquoted",
            "math=\"../outside.svr\"",
            "math=\"/absolute.svr\"",
            "math=\"C:/drive.svr\"",
            "math=\"CON.svr\"",
            "math=\"dir./x.svr\"",
            "math=\"bad.txt\"",
            "math=\"same.svr\"\nother=\"SAME.svr\"",
            "math=\"main.svr\"",
            "math=\"x.svr\"\nmath=\"y.svr\"",
        ] {
            fixture.manifest("app", mapping, "");
            let errors = packages::inspect_source_inventory(&root).unwrap_err();
            assert!(
                errors
                    .items
                    .iter()
                    .all(|d| d.source_file.as_deref()
                        == Some(root.join("sovra.toml").to_str().unwrap())),
                "{mapping}"
            );
        }
    }
    #[test]
    fn regular_files_containment_and_symlink_hardlink_aliases_are_checked() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let root = fixture.package("app", "math=\"math.svr\"", "");
        std::fs::write(fixture.0.join("outside.svr"), "mod math{}").unwrap();
        symlink(fixture.0.join("outside.svr"), root.join("math.svr")).unwrap();
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.code == "E4113"));
        std::fs::remove_file(root.join("math.svr")).unwrap();
        std::fs::create_dir(root.join("math.svr")).unwrap();
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.message.contains("regular file")));
        std::fs::remove_dir(root.join("math.svr")).unwrap();
        std::fs::hard_link(root.join("main.svr"), root.join("math.svr")).unwrap();
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.message.contains("identity")));
        std::fs::remove_file(root.join("math.svr")).unwrap();
        symlink(root.join("main.svr"), root.join("math.svr")).unwrap();
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.message.contains("identity")));
        std::fs::remove_file(root.join("math.svr")).unwrap();
        assert!(std::process::Command::new("mkfifo")
            .arg(root.join("math.svr"))
            .status()
            .unwrap()
            .success());
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.message.contains("regular file")));
    }
    #[test]
    fn mapping_and_graph_source_limits_admit_exact_counts_before_excess_paths() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "", "");
        let mappings = (0..MAX_PACKAGE_MODULES)
            .map(|n| {
                std::fs::write(root.join(format!("m{n}.svr")), "").unwrap();
                format!("m{n}=\"m{n}.svr\"\n")
            })
            .collect::<String>();
        fixture.manifest("app", &mappings, "");
        assert_eq!(
            packages::inspect_source_inventory(&root)
                .unwrap()
                .sources
                .len(),
            MAX_PACKAGE_MODULES + 1
        );
        fixture.manifest("app", &(mappings + "extra=\"missing.svr\""), "");
        assert_eq!(
            packages::inspect_source_inventory(&root).unwrap_err().items[0].code,
            "E4112"
        );
        let deps="[dependencies.b]\npath=\"../b\"\n[dependencies.c]\npath=\"../c\"\n[dependencies.d]\npath=\"../d\"\n";
        for name in ["app", "b", "c", "d"] {
            let package = if name == "app" {
                root.clone()
            } else {
                fixture.package(name, "", "")
            };
            let mappings = (0..1023)
                .map(|n| {
                    std::fs::write(package.join(format!("m{n}.svr")), "").unwrap();
                    format!("m{n}=\"m{n}.svr\"\n")
                })
                .collect::<String>();
            fixture.manifest(name, &mappings, if name == "app" { deps } else { "" });
        }
        assert_eq!(
            packages::inspect_source_inventory(&root)
                .unwrap()
                .sources
                .len(),
            MAX_INVENTORY_SOURCES
        );
        let manifest = std::fs::read_to_string(root.join("sovra.toml"))
            .unwrap()
            .replace("[modules]", "[modules]\nextra=\"m1023.svr\"");
        std::fs::write(root.join("sovra.toml"), manifest).unwrap();
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.code == "E4112" && d.message.contains("4096")));
    }
    #[test]
    fn observed_size_limits_cover_all_selected_sources_without_reading_them() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "a=\"a.svr\"\nb=\"b.svr\"\nc=\"c.svr\"", "");
        for file in ["main.svr", "a.svr", "b.svr", "c.svr"] {
            std::fs::File::create(root.join(file))
                .unwrap()
                .set_len(16 * 1024 * 1024)
                .unwrap();
        }
        let inventory = packages::inspect_source_inventory(&root).unwrap();
        assert_eq!(
            inventory
                .sources
                .iter()
                .map(|s| s.observed_bytes)
                .sum::<u64>(),
            64 * 1024 * 1024
        );
        std::fs::File::create(root.join("extra.svr"))
            .unwrap()
            .set_len(1)
            .unwrap();
        fixture.manifest(
            "app",
            "a=\"a.svr\"\nb=\"b.svr\"\nc=\"c.svr\"\nz=\"extra.svr\"",
            "",
        );
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.code == "E1003" && d.message.contains("64 MiB")));
        std::fs::File::create(root.join("a.svr"))
            .unwrap()
            .set_len(16 * 1024 * 1024 + 1)
            .unwrap();
        assert!(packages::inspect_source_inventory(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.message.contains("16 MiB")));
    }
}
