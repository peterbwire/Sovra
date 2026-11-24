//! Auth syntax coverage never establishes authorization or semantic success.
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
            "sovra-auth-coverage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"auth-coverage\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(path.join("main.svr"), "fn main(){}\n").unwrap();
        std::fs::write(path.join("policies.svr"), source).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn empty_auth_syntax_gaps_preserve_existing_incomplete_semantic_diagnostics() {
    let fixture = Fixture::new("auth session{}\n");
    let project = project::check_project(&fixture.0).unwrap();
    let previous = application::check_service_calls(&project);
    let combined = application::check_service_calls_with_auth_syntax(&project);
    assert_eq!(combined.check.files, previous.files);
    assert_eq!(combined.check.diagnostics.items, previous.diagnostics.items);
    assert!(combined
        .check
        .diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4096"));
    let coverage = combined.syntax.unwrap();
    let auth = coverage
        .iter()
        .find(|file| file.source_file == fixture.0.join("policies.svr"))
        .unwrap()
        .syntax
        .as_ref()
        .unwrap();
    assert!(auth.unparsed_ranges.is_empty()); // Syntax only.
    let old = application::check_service_calls_with_combined_syntax(&project)
        .syntax
        .unwrap();
    assert!(old
        .iter()
        .find(|file| file.source_file == fixture.0.join("policies.svr"))
        .unwrap()
        .syntax
        .is_err());
}
#[test]
fn project_auth_coverage_keeps_false_guard_and_context_fields_unparsed() {
    let source = "auth session{\nprovider: unknown\nallow user to read on Missing where false\n}\n";
    let fixture = Fixture::new(source);
    let scanner_failure = project::check_project(&fixture.0).unwrap_err();
    assert!(scanner_failure
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4081"));
    // Make the partial scanner accept the declared model, while combined syntax
    // remains conservative about unsupported model declarations in a separate file.
    std::fs::write(fixture.0.join("models.svr"), "model Missing{}\n").unwrap();
    let checked = project::check_project(&fixture.0).unwrap();
    let result = application::check_service_calls_with_auth_syntax(&checked);
    let files = result.syntax.unwrap();
    let auth = files
        .iter()
        .find(|file| file.source_file == fixture.0.join("policies.svr"))
        .unwrap()
        .syntax
        .as_ref()
        .unwrap();
    let application::syntax::PolicyConditionSyntax::Unparsed { span } =
        auth.auth.blocks[0].policies[0].condition
    else {
        panic!("guard must remain unparsed")
    };
    assert_eq!(&source[span.start..span.end], "where false");
    assert!(auth
        .unparsed_ranges
        .iter()
        .any(|gap| gap.start <= span.start && span.end <= gap.end));
    assert!(files
        .iter()
        .find(|file| file.source_file == fixture.0.join("models.svr"))
        .unwrap()
        .syntax
        .is_err());
}
