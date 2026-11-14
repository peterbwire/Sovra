//! Root-only guarded capture, ownership and existing package admission.
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
            "svr-root-capture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn package(&self, name: &str, source: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(
            p.join("sovra.toml"),
            "[project]\nname=\"app\"\nentry=\"src/main.svr\"\n",
        )
        .unwrap();
        std::fs::write(p.join("src/main.svr"), source).unwrap();
        p
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn root_capture_owns_exact_entry_and_manifest_bytes_with_canonical_identity() {
    let t = Temp::new();
    let source = "// λ😀\r\nfn main(){print(14);}\r\n";
    let root = t.package("app λ", source);
    let captured = capture_root_source(&root).unwrap();
    assert_eq!(captured.root(), std::fs::canonicalize(&root).unwrap());
    assert_eq!(
        captured.entry(),
        std::fs::canonicalize(root.join("src/main.svr")).unwrap()
    );
    assert_eq!(captured.source(), source);
    assert_eq!(captured.inventory().sources.len(), 1);
    assert_eq!(captured.manifests().len(), 1);
    assert_eq!(
        captured.manifests()[0].source(),
        std::fs::read_to_string(root.join("sovra.toml")).unwrap()
    );
}
#[test]
fn later_disk_edits_do_not_replace_captured_program_or_manifest() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){print(14);}");
    let captured = capture_root_source(&root).unwrap();
    std::fs::write(root.join("src/main.svr"), "fn main(){print(99);}").unwrap();
    std::fs::write(root.join("sovra.toml"), "changed").unwrap();
    assert!(captured.manifests()[0].source().contains("[project]"));
    let (entry, source) = captured.into_source();
    let checked = check_program_source(
        entry.to_str().unwrap(),
        &source,
        ProgramSourceLimits::DEFAULT,
    )
    .unwrap();
    let output = checked
        .execute(
            checked.function_id("main").unwrap(),
            &[],
            ProgramLoweringLimits::DEFAULT,
            RuntimeVmPolicy::DEFAULT,
        )
        .unwrap();
    assert_eq!(output.output, vec!["14"]);
}
#[test]
fn capture_does_not_parse_or_execute_source() {
    let t = Temp::new();
    let root = t.package("app", "not valid source !!!");
    let captured = capture_root_source(&root).unwrap();
    assert_eq!(captured.source(), "not valid source !!!");
    assert!(
        check_program_source("entry", captured.source(), ProgramSourceLimits::DEFAULT).is_err()
    );
}
#[test]
fn dependencies_and_registered_modules_are_rejected_before_source_capture() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    t.package("dep", "fn main(){}");
    for suffix in [
        "[dependencies.dep]\npath=\"../dep\"\n",
        "[modules]\nhelper=\"helper.svr\"\n",
    ] {
        std::fs::write(
            root.join("sovra.toml"),
            format!("[project]\nname=\"app\"\nentry=\"src/main.svr\"\n{suffix}"),
        )
        .unwrap();
        std::fs::write(root.join("helper.svr"), "module helper{}").unwrap();
        std::fs::write(root.join("src/main.svr"), [0xff]).unwrap();
        let errors = capture_root_source(&root).unwrap_err();
        assert!(errors.items.iter().any(|d| d.code == "E3124"), "{errors:?}");
    }
}
#[test]
fn outside_and_symlinked_outside_entries_retain_existing_path_refusal() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    let outside = t.0.join("outside.svr");
    std::fs::write(&outside, "fn main(){}").unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname=\"app\"\nentry=\"../outside.svr\"\n",
    )
    .unwrap();
    assert!(capture_root_source(&root).is_err());
    std::os::unix::fs::symlink(&outside, root.join("linked.svr")).unwrap();
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname=\"app\"\nentry=\"linked.svr\"\n",
    )
    .unwrap();
    assert!(capture_root_source(&root).is_err());
}
#[test]
fn source_size_and_utf8_fail_with_existing_guarded_reader_codes() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    let entry = root.join("src/main.svr");
    std::fs::File::create(&entry)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    let errors = capture_root_source(&root).unwrap_err();
    assert!(errors.items.iter().any(|d| d.code == "E1003"));
    std::fs::write(&entry, [0xff]).unwrap();
    let errors = capture_root_source(&root).unwrap_err();
    assert!(errors.items.iter().any(|d| d.code == "E4161"));
}
#[test]
fn unregistered_sources_are_not_loaded_and_locks_are_not_modified_or_claimed_verified() {
    let t = Temp::new();
    let root = t.package("app", "fn main(){}");
    std::fs::write(root.join("ignored.svr"), [0xff]).unwrap();
    std::fs::write(root.join("sovra.lock"), "not a lock").unwrap();
    let capture = capture_root_source(&root).unwrap();
    assert_eq!(capture.inventory().sources.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("sovra.lock")).unwrap(),
        "not a lock"
    );
}
