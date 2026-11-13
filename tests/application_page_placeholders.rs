//! Page placeholder names never establish render types, context injection or UI validity.
use sovra::compiler::project::application;
#[test]
fn explicit_page_and_view_parameters_are_required_after_target_resolution() {
    let source = "// λ\r\napp Demo{\r\npage \"/jobs/:id\" -> Detail\r\n}\r\npage Detail(user:Session){future body}";
    assert!(application::check_local_page_targets(source)
        .unwrap()
        .diagnostics
        .is_empty());
    let checked = application::check_page_placeholders(source).unwrap();
    assert_eq!(checked.checked_placeholders, 1);
    assert_eq!(checked.diagnostics.items[0].code, "E4145");
    let span = checked.diagnostics.items[0].span;
    assert_eq!(&source[span.start..span.end], "\"/jobs/:id\"");
    let matching = source.replace("user:Session", "id:Unknown,user:Session");
    let checked = application::check_page_placeholders(&matching).unwrap();
    assert!(checked.diagnostics.is_empty()); // Parameter/result types and extra user value remain unvalidated.
    assert!(checked.targets.render.declarations[0]
        .unparsed_body
        .is_some());
    assert_eq!(
        checked.targets.render.declarations[0].explicit_return_annotation,
        None
    );
    let view = "app Demo{\npage \"/:slug\" -> Card\n}\nview Card(slug:String){future body}";
    assert!(application::check_page_placeholders(view)
        .unwrap()
        .diagnostics
        .is_empty());
    assert!(application::inspect_functions(&matching, &[]).is_err()); // Default contract unchanged.
}
#[test]
fn target_and_shape_failures_remain_visible_without_invented_parameter_bindings() {
    let missing =
        application::check_page_placeholders("app Demo{\npage \"/:id\" -> Missing\n}").unwrap();
    assert_eq!(missing.checked_placeholders, 0);
    assert_eq!(missing.diagnostics.items[0].code, "E4042");
    let malformed = application::check_page_placeholders(
        "app Demo{\npage \"relative/:id\" -> Detail\n}\npage Detail(){}",
    )
    .unwrap();
    assert_eq!(malformed.checked_placeholders, 0);
    assert_eq!(malformed.diagnostics.items[0].code, "E4040");
    assert!(application::check_page_placeholders(
        "use app.pages;\napp Demo{\npage \"/:id\" -> pages.Detail\n}"
    )
    .unwrap_err()
    .contains("context unavailable"));
}
