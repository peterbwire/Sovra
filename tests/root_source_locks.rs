//! Existing source-lock schema over guarded root-only captured bytes.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{project::packages::*, v2::*};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "svr-root-lock-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn package(&self, name: &str, source: &str) -> PathBuf {
        let root = self.0.join(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname=\"app\"\nversion=\"0.1.0\"\nentry=\"src/main.svr\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src/main.svr"), source).unwrap();
        root
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn refused(result: Result<(), sovra::compiler::diagnostics::Diagnostics>) {
    let d = result.unwrap_err();
    assert!(d.items.iter().any(|d| d.code == "E4165"), "{d:?}");
}
#[test]
fn ordinary_root_locks_match_existing_schema_and_legacy_generation_exactly() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){print(14);}");
    let root_inputs = capture_root_source(&root).unwrap();
    let old_inputs = load_package_sources(&root).unwrap();
    let lock = generate_root_source_lock(&root_inputs).unwrap();
    assert_eq!(lock, generate_source_lock(&old_inputs).unwrap());
    verify_source_lock(&old_inputs, lock.as_str()).unwrap();
    verify_root_source_lock(&root_inputs, lock.as_str()).unwrap();
    assert!(!lock.as_str().contains(t.0.to_str().unwrap()));
}
#[test]
fn relocated_inputs_and_retained_snapshots_do_not_reread_disk_sources() {
    let t = Temp::new();
    let source = "// λ😀\r\nenum E{A(Int)}fn main(){let e=E::A(14);}";
    let a = t.package("a", source);
    let b = t.package("b", source);
    let captured = capture_root_source(&a).unwrap();
    let lock = generate_root_source_lock(&captured).unwrap();
    assert_eq!(
        lock,
        generate_root_source_lock(&capture_root_source(&b).unwrap()).unwrap()
    );
    std::fs::remove_dir_all(&a).unwrap();
    verify_root_source_lock(&captured, lock.as_str()).unwrap();
    let checked =
        check_program_source("captured", captured.source(), ProgramSourceLimits::DEFAULT).unwrap();
    assert!(checked.function_id("main").is_some());
}
#[test]
fn equal_length_source_and_manifest_changes_are_stale() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){print(14);}");
    let captured = capture_root_source(&root).unwrap();
    let lock = generate_root_source_lock(&captured).unwrap();
    std::fs::write(root.join("src/main.svr"), "fn main(){print(99);}").unwrap();
    refused(verify_root_source_lock(
        &capture_root_source(&root).unwrap(),
        lock.as_str(),
    ));
    std::fs::write(root.join("src/main.svr"), captured.source()).unwrap();
    let original = std::fs::read_to_string(root.join("sovra.toml")).unwrap();
    std::fs::write(root.join("sovra.toml"), original.replace("0.1.0", "0.2.0")).unwrap();
    refused(verify_root_source_lock(
        &capture_root_source(&root).unwrap(),
        lock.as_str(),
    ));
}
#[test]
fn malformed_duplicate_unknown_version_and_oversized_documents_fail_strictly() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    let inputs = capture_root_source(root).unwrap();
    let lock = generate_root_source_lock(&inputs).unwrap();
    let text = lock.as_str();
    for invalid in [
        text[..text.len() - 2].to_string(),
        text.replace("\"version\":1", "\"version\":2"),
        text.replacen('{', "{\"extra\":true,", 1),
        text.replace("\"version\":1", "\"version\":1,\"version\":1"),
        " ".repeat(MAX_LOCK_BYTES + 1),
    ] {
        refused(verify_root_source_lock(&inputs, &invalid));
    }
    let pretty = text.replace(',', ",\n ");
    verify_root_source_lock(&inputs, &pretty).unwrap();
}
#[test]
fn guarded_publication_and_readonly_verification_preserve_the_existing_lock() {
    use std::os::unix::fs::PermissionsExt;
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    write_root_source_lock(&root).unwrap();
    let path = root.join("sovra.lock");
    let before = std::fs::read(&path).unwrap();
    check_root_source_lock(&root).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
    check_root_source_lock(&root).unwrap();
    assert!(write_root_source_lock(&root).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!std::fs::read_dir(&root).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".svr-lock-")));
}
#[test]
fn symlink_read_and_alias_publication_refuse_without_touching_other_files() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    let inputs = capture_root_source(&root).unwrap();
    let lock = generate_root_source_lock(&inputs).unwrap();
    let outside = t.0.join("other.lock");
    std::fs::write(&outside, lock.as_str()).unwrap();
    let path = root.join("sovra.lock");
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    refused(verify_existing_root_source_lock(&inputs));
    assert!(write_root_source_lock(&root).is_err());
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), lock.as_str());
    std::fs::remove_file(&path).unwrap();
    std::fs::hard_link(&outside, &path).unwrap();
    assert!(write_root_source_lock(&root).is_err());
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), lock.as_str());
}
#[test]
fn lock_integrity_does_not_claim_semantics_and_unsupported_packages_do_not_publish() {
    let t = Temp::new();
    let root = t.package("app", "not valid syntax !!!");
    write_root_source_lock(&root).unwrap();
    check_root_source_lock(&root).unwrap();
    assert!(check_program_source(
        "bad",
        capture_root_source(&root).unwrap().source(),
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
    let before = std::fs::read(root.join("sovra.lock")).unwrap();
    std::fs::write(root.join("helper.svr"), "module helper{}").unwrap();
    let manifest = std::fs::read_to_string(root.join("sovra.toml")).unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        manifest + "[modules]\nhelper=\"helper.svr\"\n",
    )
    .unwrap();
    let d = write_root_source_lock(&root).unwrap_err();
    assert!(d.items.iter().any(|d| d.code == "E3124"));
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), before);
}
#[test]
fn stale_locks_require_explicit_regeneration_and_failed_checks_do_not_rewrite() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){print(1);}");
    write_root_source_lock(&root).unwrap();
    let old = std::fs::read(root.join("sovra.lock")).unwrap();
    std::fs::write(root.join("src/main.svr"), "fn main(){print(2);}").unwrap();
    refused(check_root_source_lock(&root));
    assert_eq!(std::fs::read(root.join("sovra.lock")).unwrap(), old);
    write_root_source_lock(&root).unwrap();
    assert_ne!(std::fs::read(root.join("sovra.lock")).unwrap(), old);
    check_root_source_lock(&root).unwrap();
}
