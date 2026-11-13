//! Opt-in project syntax stays separate from existing semantic checking.
use sovra::compiler::project::{self, application};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra-syntax-coverage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"syntax-coverage\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(path.join("main.svr"), source).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn syntax_coverage_never_suppresses_existing_semantic_diagnostics() {
    let fixture = Fixture::new("fn main(){let value:Int=true;}");
    let project = project::check_project(&fixture.0).unwrap();
    let existing = application::check_service_calls(&project);
    let combined = application::check_service_calls_with_syntax(&project);
    assert_eq!(existing.files, combined.check.files);
    assert_eq!(existing.diagnostics.items, combined.check.diagnostics.items);
    assert!(!combined.check.diagnostics.is_empty());
    let files = combined.syntax.unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].syntax.as_ref().unwrap().unparsed_ranges.is_empty());
    assert_eq!(files[0].source_file, fixture.0.join("main.svr"));
}
#[test]
fn unsupported_file_coverage_remains_an_error_without_changing_checker_results() {
    let fixture = Fixture::new("app Unsupported {}\n");
    let project = project::check_project(&fixture.0).unwrap();
    let existing = application::check_service_calls(&project);
    let combined = application::check_service_calls_with_syntax(&project);
    assert_eq!(existing.files, combined.check.files);
    assert_eq!(existing.diagnostics.items, combined.check.diagnostics.items);
    assert!(combined.syntax.unwrap()[0].syntax.is_err());
}
