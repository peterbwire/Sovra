//! Page/view header identity never establishes render body/result validity.
use sovra::compiler::project::{
    self,
    application::{self, syntax},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra-page-targets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"page-targets\"\nentry = \"main.svr\"\n",
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
fn page_target_messages_match_scanner_but_functions_do_not_become_pages() {
    let source = "// λ\r\napp Demo{\r\npage \"/\" -> Home\r\npage \"/missing\" -> ordinary\r\n}\r\npage Home(id:Unknown){untyped body}\r\nfn ordinary(){}";
    let fixture = Fixture::new(source);
    let scanner = project::check_project(&fixture.0).unwrap_err();
    let checked = application::check_local_page_targets(source).unwrap();
    let expected = scanner
        .items
        .iter()
        .find(|diagnostic| diagnostic.code == "E4042")
        .unwrap();
    assert_eq!(checked.diagnostics.items[0].code, expected.code);
    assert_eq!(checked.diagnostics.items[0].message, expected.message);
    assert_eq!(checked.resolved.len(), 1);
    let target = &checked.resolved[0];
    assert_eq!(
        &source[target.target_span.start..target.target_span.end],
        "Home"
    );
    assert!(checked.render.declarations[target.declaration_index]
        .unparsed_body
        .is_some());
    assert_eq!(
        checked.render.declarations[target.declaration_index].explicit_return_annotation,
        None
    );
}
#[test]
fn real_fielddesk_headers_keep_unresolved_types_and_render_bodies_as_gaps() {
    let source = include_str!("../examples/fielddesk/app/pages.svr");
    let parsed = syntax::parse_render_declarations(source).unwrap();
    assert_eq!(parsed.declarations.len(), 3);
    assert_eq!(parsed.declarations[0].name, "dashboard");
    assert_eq!(parsed.declarations[1].parameters[0].annotation, "Id<Job>");
    assert_eq!(
        parsed.declarations[2].kind,
        syntax::RenderDeclarationKind::View
    );
    assert!(parsed.declarations.iter().all(|declaration| declaration
        .explicit_return_annotation
        .is_none()
        && declaration.unparsed_body.is_some()));
    assert!(!parsed.unparsed_ranges.is_empty());
    assert!(application::check_local_page_targets(source)
        .unwrap_err()
        .contains("context unavailable"));
    assert!(syntax::parse_with_auth(source).is_err()); // Existing aggregate coverage stays strict.
}
