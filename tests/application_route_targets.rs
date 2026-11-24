//! Limited local target resolution is not route authorization or body checking.
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
            "sovra-route-targets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"route-targets\"\nentry = \"main.svr\"\n",
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
fn missing_local_route_targets_keep_scanner_code_and_message_with_precise_token_span() {
    let source = "// λ\r\napp Demo{\r\nroute GET \"/\" -> absent\r\n}\r\nfn main(){}";
    let fixture = Fixture::new(source);
    let scanner = project::check_project(&fixture.0).unwrap_err();
    let checked = application::check_local_route_targets(source).unwrap();
    let expected = scanner
        .items
        .iter()
        .find(|diagnostic| diagnostic.code == "E4033")
        .unwrap();
    assert_eq!(checked.diagnostics.items.len(), 1);
    assert_eq!(checked.diagnostics.items[0].code, expected.code);
    assert_eq!(checked.diagnostics.items[0].message, expected.message);
    let span = checked.diagnostics.items[0].span;
    assert_eq!(&source[span.start..span.end], "absent");
}
#[test]
fn declaration_identity_is_distinct_from_handler_argument_or_body_validity() {
    let source = "app Demo{\nroute GET \"/:id\" -> handler\n}\nfn before(){}\nfn handler(other:Int){let value:Int=true;}";
    let checked = application::check_local_route_targets(source).unwrap();
    assert_eq!(checked.resolved.len(), 1);
    let target = &checked.resolved[0];
    assert_eq!(target.callable_index, 1);
    assert_eq!(
        checked.syntax.declarations.base.base.callables[target.callable_index]
            .function
            .name,
        "handler"
    );
    assert!(checked.diagnostics.is_empty()); // Name binding only; argument/body validation is separate.
    assert!(!application::check_local_functions(source)
        .unwrap()
        .diagnostics
        .is_empty());
    assert!(application::inspect_functions(source, &[]).is_err());
}
