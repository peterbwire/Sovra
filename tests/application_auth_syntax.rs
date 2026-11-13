//! Auth headers cannot hide conditions or imply permission grants.
use sovra::compiler::project::{
    self,
    application::syntax::{parse_auth_blocks, PolicyConditionSyntax},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new(auth_source: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sovra-auth-syntax-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"auth-syntax\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(path.join("main.svr"), "fn main(){}\n").unwrap();
        std::fs::write(path.join("policies.svr"), auth_source).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn fielddesk_auth_conditions_and_context_fields_stay_explicitly_unparsed() {
    let source = include_str!("../examples/fielddesk/app/auth.svr");
    let parsed = parse_auth_blocks(source).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.blocks[0].policies.len(), 3);
    assert_eq!(parsed.blocks[0].unparsed_ranges.len(), 4);
    assert_eq!(
        parsed.blocks[0]
            .policies
            .iter()
            .filter(|policy| matches!(policy.condition, PolicyConditionSyntax::Unparsed { .. }))
            .count(),
        2
    );
    for policy in &parsed.blocks[0].policies {
        if let PolicyConditionSyntax::Unparsed { span } = policy.condition {
            assert!(source[span.start..span.end].starts_with("where "));
            assert!(parsed
                .unparsed_ranges
                .iter()
                .any(|range| range.start <= span.start && span.end <= range.end));
        }
    }
}
#[test]
fn scanner_header_parity_does_not_remove_an_unvalidated_false_condition() {
    let source =
        "// λ\r\nauth session{\r\nallow user to read on Item where false\r\n}\r\nmodel Item{}";
    let fixture = Fixture::new(source);
    let checked = project::check_project(&fixture.0).unwrap();
    let parsed = parse_auth_blocks(source).unwrap();
    assert_eq!(checked.auth_policies.len(), 1);
    assert_eq!(
        parsed.blocks[0].policies[0].header,
        checked.auth_policies[0]
    );
    let PolicyConditionSyntax::Unparsed { span } = parsed.blocks[0].policies[0].condition else {
        panic!("where condition must remain unparsed")
    };
    assert_eq!(&source[span.start..span.end], "where false");
    assert!(!parsed.unparsed_ranges.is_empty());
}
