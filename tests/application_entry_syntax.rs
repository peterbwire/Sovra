//! Entry syntax remains independent of scanner validation and application execution.
use sovra::compiler::project::application::syntax::{parse_entry_blocks, AppMemberKind};

#[test]
fn fielddesk_entry_keeps_supported_bindings_and_explicit_unparsed_showcase_fields() {
    let source = include_str!("../examples/fielddesk/app/main.svr");
    let syntax = parse_entry_blocks(source).unwrap();
    assert_eq!(syntax.entries.len(), 1);
    let entry = &syntax.entries[0];
    assert_eq!(entry.name, "Fielddesk");
    assert_eq!(entry.members.len(), 9);
    assert_eq!(entry.unparsed_ranges.len(), 1);
    let unknown = entry.unparsed_ranges[0];
    assert_eq!(&source[unknown.start..unknown.end], "title: \"Fielddesk\"");
    assert_eq!(
        entry
            .members
            .iter()
            .filter(|member| matches!(member.kind, AppMemberKind::Route(_)))
            .count(),
        3
    );
    assert!(!syntax.unparsed_ranges.is_empty()); // Imports and main are outside entry-only coverage.
}

#[test]
fn entry_nodes_preserve_raw_targets_without_authorizing_or_resolving_them() {
    let source = "app Demo {\nroute BAD \"relative\" -> missing.handler\nauth: unknown.policy\n}\n";
    let syntax = parse_entry_blocks(source).unwrap();
    let AppMemberKind::Route(route) = &syntax.entries[0].members[0].kind else {
        panic!("route")
    };
    assert_eq!(route.method, "BAD");
    assert_eq!(route.target, "missing.handler");
    assert!(syntax.unparsed_ranges.is_empty()); // Syntax coverage only.
    assert!(parse_entry_blocks("app Demo{\nroute GET \"/\" -> missing;\n}").is_err());
    // Preserve existing target grammar.
}
