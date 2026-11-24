//! Local lock determinism and exact retained-input verification, without publication.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
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
                "sovra-source-lock-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn example(&self) -> PathBuf {
            for relative in [
                "app/sovra.toml",
                "app/main.svr",
                "app/application.svr",
                "utilities/sovra.toml",
                "utilities/main.svr",
                "utilities/arithmetic.svr",
            ] {
                let target = self.0.join(relative);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::copy(
                    std::path::Path::new("examples/multi-file-packages").join(relative),
                    target,
                )
                .unwrap();
            }
            self.0.join("app")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn alias_diamonds_retain_all_edges_with_one_canonical_shared_identity() {
        let fixture = Fixture::new();
        for (name, deps) in [
            ("app", "[dependencies.a]\npath=\"../left\"\n[dependencies.b]\npath=\"../right\"\n"),
            ("left", "[dependencies.first]\npath=\"../shared\"\n[dependencies.second]\npath=\"../shared\"\n"),
            ("right", "[dependencies.shared]\npath=\"../shared\"\n"),
            ("shared", ""),
        ] {
            let root = fixture.0.join(name);
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("sovra.toml"), format!("[project]\nname=\"identical\"\nentry=\"main.svr\"\n{deps}")).unwrap();
            std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        }
        let inputs = packages::load_package_sources(fixture.0.join("app")).unwrap();
        let lock = packages::generate_source_lock(&inputs).unwrap();
        assert_eq!(lock.as_str().matches("\"id\":").count(), 4);
        assert_eq!(lock.as_str().matches("\"target\":\"p2\"").count(), 3);
        assert!(lock.as_str().contains("\"alias\":\"second\""));
        assert!(packages::verify_source_lock(&inputs, lock.as_str()).is_ok());
        assert!(packages::verify_source_lock(
            &inputs,
            &lock
                .as_str()
                .replace("\"target\":\"p2\"", "\"target\":\"p3\"")
        )
        .is_err());
    }
    #[test]
    fn relocated_graphs_generate_identical_locks_without_absolute_paths_or_writes() {
        let first = Fixture::new();
        let second = Fixture::new();
        let a = first.example();
        let b = second.example();
        let inputs = packages::load_package_sources(&a).unwrap();
        let lock = packages::generate_source_lock(&inputs).unwrap();
        let other =
            packages::generate_source_lock(&packages::load_package_sources(&b).unwrap()).unwrap();
        assert_eq!(lock, other);
        assert!(!lock.as_str().contains(first.0.to_str().unwrap()));
        assert!(!a.join("sovra.lock").exists());
        assert!(packages::verify_source_lock(&inputs, lock.as_str()).is_ok());
        assert!(packages::verify_source_lock(&inputs, &format!(" \n{}\n", lock.as_str())).is_ok());
    }
    #[test]
    fn raw_source_and_manifest_mutations_are_stale_while_retained_inputs_do_not_change() {
        let fixture = Fixture::new();
        let root = fixture.example();
        let inputs = packages::load_package_sources(&root).unwrap();
        let lock = packages::generate_source_lock(&inputs).unwrap();
        std::fs::write(
            root.join("main.svr"),
            "// changed comment\nfn main(){print(application::answer());}",
        )
        .unwrap();
        assert!(packages::verify_source_lock(&inputs, lock.as_str()).is_ok());
        assert!(packages::verify_source_lock(
            &packages::load_package_sources(&root).unwrap(),
            lock.as_str()
        )
        .is_err());
        let source = std::fs::read_to_string(root.join("main.svr")).unwrap();
        std::fs::write(root.join("main.svr"), source.replace('\n', "\r\n")).unwrap();
        assert!(packages::verify_source_lock(
            &packages::load_package_sources(&root).unwrap(),
            lock.as_str()
        )
        .is_err());
        let path = root.join("sovra.toml");
        let source = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, format!("# changed manifest\n{source}")).unwrap();
        assert!(packages::verify_source_lock(
            &packages::load_package_sources(&root).unwrap(),
            lock.as_str()
        )
        .is_err());
    }
    #[test]
    fn malformed_unknown_duplicate_and_forged_lock_data_never_verify() {
        let inputs = packages::load_package_sources("examples/multi-file-packages/app").unwrap();
        let lock = packages::generate_source_lock(&inputs).unwrap();
        for source in [
            "{}".to_owned(),
            lock.as_str().replace("\"version\":1", "\"version\":2"),
            lock.as_str().replacen('{', "{\"unknown\":0,", 1),
            lock.as_str()
                .replacen('{', "{\"format\":\"sovra-lock\",", 1),
            lock.as_str().replace("\"root\":\"p0\"", "\"root\":\"p1\""),
            format!("{} true", lock.as_str()),
        ] {
            assert!(packages::verify_source_lock(&inputs, &source).is_err());
        }
        let forged = lock.as_str().replacen("sha256\":\"", "sha256\":\"X", 1);
        assert!(packages::verify_source_lock(&inputs, &forged).is_err());
    }
}
