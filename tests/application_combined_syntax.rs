//! Combined coverage remains source-owned and never substitutes for checking.
use sovra::compiler::project::application::{self, syntax};

#[test]
fn fielddesk_combines_bindings_imports_and_main_without_covering_unknown_title() {
    let source = include_str!("../examples/fielddesk/app/main.svr");
    let parsed = syntax::parse_combined(source).unwrap();
    assert_eq!(parsed.source_bytes, source.len());
    assert_eq!(parsed.entries.entries[0].members.len(), 9);
    assert_eq!(parsed.declarations.imports.len(), 5);
    assert_eq!(
        parsed.declarations.base.base.callables[0].function.name,
        "main"
    );
    assert_eq!(parsed.unparsed_ranges.len(), 1);
    let span = parsed.unparsed_ranges[0];
    assert_eq!(&source[span.start..span.end], "title: \"Fielddesk\"");
    assert!(application::inspect_functions(source, &[]).is_err()); // Existing contract unchanged.
    assert!(syntax::parse_file(source).is_err());
}

#[test]
fn ordinary_declaration_projection_is_identical_when_no_entries_are_present() {
    let source = "type Count = Int; struct Item {count: Count}\nservice Api{fn request(x:Int);}\nfn main(){print(1);}";
    let original = syntax::parse_file(source).unwrap();
    let combined = syntax::parse_combined(source).unwrap();
    assert_eq!(original, combined.declarations);
    assert!(combined.entries.entries.is_empty());
    assert!(combined.unparsed_ranges.is_empty());
    assert_eq!(combined.token_count, original.token_count);
}
