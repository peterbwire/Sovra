//! Structured path checks share rules with scanner diagnostics, not target authorization.
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
            "sovra-entry-paths-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"entry-paths\"\nentry = \"main.svr\"\n",
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
fn structured_path_codes_and_messages_match_the_existing_project_scanner() {
    for member in [
        "route BAD \"/\" -> handler",
        "route GET \"relative\" -> handler",
        "route GET \"/a//b\" -> handler",
        "route GET \"/a/\" -> handler",
        "route GET \"/:9id\" -> handler",
        "route GET \"/a b\" -> handler",
        "page \"relative\" -> Home",
        "page \"/a/\" -> Home",
        "route GET \"/\" -> handler\nroute GET \"/\" -> handler",
        "page \"/\" -> Home\npage \"/\" -> Home",
    ] {
        let source = format!(
            "// λ\r\napp Demo{{\r\n{member}\r\n}}\r\nfn handler(){{}}\r\nview Home(){{}}\r\n"
        );
        let fixture = Fixture::new(&source);
        let scanner = project::check_project(&fixture.0).unwrap_err();
        let structured = application::check_entry_paths(&source).unwrap();
        let is_shape = |code: &str| {
            matches!(
                code,
                "E4030" | "E4031" | "E4032" | "E4034" | "E4040" | "E4041" | "E4043"
            )
        };
        let mut expected = scanner
            .items
            .iter()
            .filter(|diagnostic| is_shape(diagnostic.code))
            .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
            .collect::<Vec<_>>();
        let mut actual = structured
            .diagnostics
            .items
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
            .collect::<Vec<_>>();
        expected.sort();
        actual.sort();
        assert!(!expected.is_empty(), "{member}");
        assert_eq!(actual, expected, "{member}");
    }
}
#[test]
fn limited_path_success_does_not_resolve_missing_targets_or_validate_other_members() {
    let source = "app Demo{\nroute GET \"/:id\" -> absent.handler\npage \"/\" -> absent.page\ntitle: \"unknown\"\n}\n";
    let checked = application::check_entry_paths(source).unwrap();
    assert_eq!(checked.checked_routes, 1);
    assert_eq!(checked.checked_pages, 1);
    assert!(checked.diagnostics.is_empty());
    assert!(!checked.syntax.unparsed_ranges.is_empty());
    let fixture = Fixture::new(source);
    let diagnostics = project::check_project(&fixture.0).unwrap_err();
    assert!(diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4033"));
    assert!(diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4042"));
}
