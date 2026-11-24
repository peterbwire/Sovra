//! Syntax-independent captured graphs preserve bytes, identities and lock compatibility.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::project::packages::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "sovra raw graph λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn graph(&self) -> PathBuf {
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
fn captures_generic_dependency_modules_without_legacy_parsing_or_namespace_merging() {
    let f = Fixture::new();
    let root = f.graph();
    let generic = "// λ😀\r\nmodule arithmetic {export fn identity<T>(x:T)->T{return x;}}";
    std::fs::write(f.0.join("utilities/arithmetic.svr"), generic).unwrap();
    let captured = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    assert_eq!(captured.inventory().graph.packages.len(), 2);
    assert_eq!(captured.manifests().len(), 2);
    assert_eq!(captured.sources().len(), 4);
    let mut total = 0;
    for (index, source) in captured.sources().iter().enumerate() {
        assert_eq!(index, source.source_index());
        let registration = &captured.inventory().sources[index];
        assert_eq!(
            source.source(),
            std::fs::read_to_string(&registration.canonical_path).unwrap()
        );
        total += source.source().len();
    }
    assert_eq!(captured.source_bytes(), total);
    assert!(captured.sources().iter().any(|s| s.source() == generic));
    assert!(load_package_sources(&root).is_err());
    // Capture/lock success is not syntax validity or permission to execute a module.
    std::fs::write(f.0.join("utilities/arithmetic.svr"), "not source syntax").unwrap();
    assert!(capture_package_sources(&root, PackageCaptureLimits::DEFAULT).is_ok());
}
#[test]
fn raw_capture_locks_match_the_existing_parsed_lock_schema_exactly() {
    let f = Fixture::new();
    let root = f.graph();
    let raw = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    let parsed = load_package_sources(&root).unwrap();
    let lock = generate_captured_source_lock(&raw).unwrap();
    assert_eq!(lock, generate_source_lock(&parsed).unwrap());
    verify_source_lock(&parsed, lock.as_str()).unwrap();
    verify_captured_source_lock(&raw, lock.as_str()).unwrap();
    assert!(!root.join("sovra.lock").exists());
    for source in &raw.inventory().sources {
        std::fs::remove_file(&source.canonical_path).unwrap();
    }
    for manifest in raw.manifests() {
        std::fs::remove_file(manifest.path()).unwrap();
    }
    assert_eq!(generate_captured_source_lock(&raw).unwrap(), lock);
    verify_captured_source_lock(&raw, lock.as_str()).unwrap();
}
#[test]
fn dependency_and_manifest_changes_stale_locks_but_captured_bytes_remain_fixed() {
    let f = Fixture::new();
    let root = f.graph();
    let first = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    let lock = generate_captured_source_lock(&first).unwrap();
    std::fs::write(
        f.0.join("utilities/arithmetic.svr"),
        "module arithmetic {export fn id<T>(x:T)->T{return x;}}",
    )
    .unwrap();
    let changed = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    assert!(verify_captured_source_lock(&changed, lock.as_str()).is_err());
    verify_captured_source_lock(&first, lock.as_str()).unwrap();
    let lock = generate_captured_source_lock(&changed).unwrap();
    std::fs::write(root.join("unregistered.svr"), "ignored").unwrap();
    verify_captured_source_lock(
        &capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap(),
        lock.as_str(),
    )
    .unwrap();
    let manifest = root.join("sovra.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(manifest, format!("{text}\n# changed\n")).unwrap();
    assert!(verify_captured_source_lock(
        &capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap(),
        lock.as_str()
    )
    .is_err());
}
#[test]
fn source_count_aggregate_bytes_and_per_file_admission_are_exact() {
    let f = Fixture::new();
    let root = f.graph();
    let captured = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    let exact = PackageCaptureLimits {
        max_sources: captured.sources().len(),
        max_source_bytes: captured.source_bytes(),
        max_file_bytes: captured
            .sources()
            .iter()
            .map(|s| s.source().len())
            .max()
            .unwrap(),
    };
    assert!(capture_package_sources(&root, exact).is_ok());
    for limits in [
        PackageCaptureLimits {
            max_sources: exact.max_sources - 1,
            ..exact
        },
        PackageCaptureLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        PackageCaptureLimits {
            max_file_bytes: exact.max_file_bytes - 1,
            ..exact
        },
        PackageCaptureLimits {
            max_sources: 0,
            ..exact
        },
        PackageCaptureLimits {
            max_source_bytes: 0,
            ..exact
        },
        PackageCaptureLimits {
            max_file_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            capture_package_sources(&root, limits).unwrap_err().items[0].code,
            "E1003"
        );
    }
}
#[test]
fn guarded_existing_locks_reject_missing_malformed_stale_and_linked_inputs() {
    let f = Fixture::new();
    let root = f.graph();
    let captured = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    let path = root.join("sovra.lock");
    assert!(verify_existing_captured_source_lock(&captured).is_err());
    for text in ["{}", "{\"version\":1,\"version\":1}", "not json"] {
        std::fs::write(&path, text).unwrap();
        assert_eq!(
            verify_existing_captured_source_lock(&captured)
                .unwrap_err()
                .items[0]
                .code,
            "E4165"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
    let lock = generate_captured_source_lock(&captured).unwrap();
    std::fs::write(&path, lock.as_str()).unwrap();
    verify_existing_captured_source_lock(&captured).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::write(root.join("other.lock"), lock.as_str()).unwrap();
    std::os::unix::fs::symlink("other.lock", &path).unwrap();
    assert!(verify_existing_captured_source_lock(&captured).is_err());
}
#[test]
fn aliased_dependency_identity_is_captured_once_and_all_edges_are_locked() {
    let f = Fixture::new();
    let root = f.graph();
    let path = root.join("sovra.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{text}\n[dependencies.second]\npath=\"../utilities\"\n"),
    )
    .unwrap();
    let captured = capture_package_sources(&root, PackageCaptureLimits::DEFAULT).unwrap();
    assert_eq!(captured.sources().len(), 4);
    assert_eq!(captured.manifests().len(), 2);
    let lock = generate_captured_source_lock(&captured).unwrap();
    assert!(lock.as_str().contains("\"alias\":\"second\""));
    assert_eq!(lock.as_str().matches("\"id\":").count(), 2);
}
#[test]
fn symlinked_and_hard_linked_selected_sources_are_refused() {
    let f = Fixture::new();
    let root = f.graph();
    let path = root.join("application.svr");
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink("main.svr", &path).unwrap();
    assert!(capture_package_sources(&root, PackageCaptureLimits::DEFAULT).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::hard_link(root.join("main.svr"), path).unwrap();
    assert!(capture_package_sources(&root, PackageCaptureLimits::DEFAULT).is_err());
}

#[test]
fn callers_cannot_raise_the_existing_frontend_file_ceiling() {
    let f = Fixture::new();
    let root = f.graph();
    let too_large =
        "x".repeat(sovra::compiler::resources::FrontendLimits::DEFAULT.max_source_bytes + 1);
    std::fs::write(root.join("main.svr"), too_large).unwrap();
    let limits = PackageCaptureLimits {
        max_sources: usize::MAX,
        max_source_bytes: usize::MAX,
        max_file_bytes: usize::MAX,
    };
    assert_eq!(
        capture_package_sources(&root, limits).unwrap_err().items[0].code,
        "E1003"
    );
}
