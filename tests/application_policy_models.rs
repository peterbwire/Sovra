//! Local model name references preserve policy guards and opaque data bodies.
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
            "sovra-policy-models-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"policy-models\"\nentry = \"main.svr\"\n",
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
fn model_reference_errors_share_scanner_messages_without_treating_enums_as_models() {
    let source = "// λ\r\nmodel Item{value:Unknown}\r\nenum Role{user}\r\nauth session{\r\nallow user to read on [Item, Role, Missing] where false\r\n}";
    let fixture = Fixture::new(source);
    let scanner = project::check_project(&fixture.0).unwrap_err();
    let checked = application::check_local_policy_models(source).unwrap();
    let expected = scanner
        .items
        .iter()
        .filter(|diagnostic| diagnostic.code == "E4081")
        .map(|diagnostic| (diagnostic.code, &diagnostic.message))
        .collect::<Vec<_>>();
    let actual = checked
        .diagnostics
        .items
        .iter()
        .map(|diagnostic| (diagnostic.code, &diagnostic.message))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(checked.resolved.len(), 1);
    let reference = &checked.resolved[0];
    assert_eq!(
        checked.data.declarations[reference.declaration_index].kind,
        syntax::DataDeclarationKind::Model
    );
    assert!(checked.data.declarations[reference.declaration_index]
        .unparsed_body
        .is_some());
    assert!(matches!(
        checked.auth.blocks[0].policies[0].condition,
        syntax::PolicyConditionSyntax::Unparsed { .. }
    ));
    let span = checked.diagnostics.items[0].span;
    assert!(!source[span.start..span.end].contains("where")); // Header reference only, not a validated guard.
}
#[test]
fn real_fielddesk_data_headers_do_not_assert_field_or_enum_variant_validity() {
    let source = include_str!("../examples/fielddesk/app/auth.svr");
    let parsed = syntax::parse_data_declarations(source).unwrap();
    assert_eq!(parsed.declarations.len(), 2);
    assert_eq!(parsed.declarations[0].name, "StaffUser");
    assert_eq!(parsed.declarations[1].name, "Role");
    assert_eq!(
        parsed.declarations[1].kind,
        syntax::DataDeclarationKind::Enum
    );
    assert!(parsed
        .declarations
        .iter()
        .all(|declaration| declaration.unparsed_body.is_some()));
    assert!(!parsed.unparsed_ranges.is_empty());
    assert!(syntax::parse_with_auth(source).is_err()); // Existing aggregate coverage stays strict about model bodies.
}
