//! Combined metadata never converts partial application checking into success.
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
            "sovra-combined-coverage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"combined-coverage\"\nentry = \"main.svr\"\n",
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
fn empty_combined_gaps_do_not_remove_incomplete_checker_diagnostics() {
    let fixture = Fixture::new("app Demo{}\nfn main(){}\n");
    let project = project::check_project(&fixture.0).unwrap();
    let existing = application::check_service_calls(&project);
    let combined = application::check_service_calls_with_combined_syntax(&project);
    assert_eq!(existing.files, combined.check.files);
    assert_eq!(existing.diagnostics.items, combined.check.diagnostics.items);
    assert!(combined
        .check
        .diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4096"));
    let files = combined.syntax.unwrap();
    assert_eq!(files[0].source_file, fixture.0.join("main.svr"));
    assert!(files[0].syntax.as_ref().unwrap().unparsed_ranges.is_empty());
    assert!(application::check_service_calls_with_syntax(&project)
        .syntax
        .unwrap()[0]
        .syntax
        .is_err());
}
#[test]
fn unknown_entry_fields_keep_original_ranges_in_project_results() {
    let source = "app Demo{\ntitle: \"λ 🦀\"\n}\nfn main(){}";
    let fixture = Fixture::new(source);
    let project = project::check_project(&fixture.0).unwrap();
    let combined = application::check_service_calls_with_combined_syntax(&project);
    let files = combined.syntax.unwrap();
    let parsed = files[0].syntax.as_ref().unwrap();
    assert_eq!(parsed.unparsed_ranges.len(), 1);
    let gap = parsed.unparsed_ranges[0];
    assert_eq!(&source[gap.start..gap.end], "title: \"λ 🦀\"");
}
#[test]
fn cumulative_projection_exhaustion_preserves_existing_semantic_errors() {
    let fixture = Fixture::new("fn main(){let value:Int=true;}");
    for n in 0..application::MAX_SYNTAX_FILES {
        std::fs::write(
            fixture.0.join(format!("extra{n}.svr")),
            format!("fn f{n}(){{}}\n"),
        )
        .unwrap();
    }
    let project = project::check_project(&fixture.0).unwrap();
    let existing = application::check_service_calls(&project);
    let combined = application::check_service_calls_with_combined_syntax(&project);
    assert_eq!(existing.files, combined.check.files);
    assert_eq!(existing.diagnostics.items, combined.check.diagnostics.items);
    assert!(!combined.check.diagnostics.is_empty());
    assert!(combined.syntax.unwrap_err().contains("file limit"));
}
