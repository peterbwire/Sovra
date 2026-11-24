//! Placeholder names do not imply HTTP value typing, injection or complete binding.
use sovra::compiler::project::application;
#[test]
fn retained_local_handler_parameters_are_required_without_implicit_request_values() {
    let source = "// λ\r\napp Demo{\r\nroute GET \"/jobs/:id\" -> handler\r\n}\r\nfn handler(request:String){}";
    let names = application::check_local_route_targets(source).unwrap();
    assert!(names.diagnostics.is_empty());
    let checked = application::check_route_placeholders(source).unwrap();
    assert_eq!(checked.checked_placeholders, 1);
    assert_eq!(checked.diagnostics.items[0].code, "E4144");
    assert!(checked.diagnostics.items[0].message.contains("named `id`"));
    let correct = source.replace("request:String", "id:Int");
    assert!(application::check_route_placeholders(&correct)
        .unwrap()
        .diagnostics
        .is_empty());
    assert!(application::inspect_functions(&correct, &[]).is_err()); // Default inspection is unchanged.
}
#[test]
fn missing_targets_and_path_errors_remain_errors_and_extra_parameters_stay_unvalidated() {
    let missing = application::check_route_placeholders(
        "app Demo{\nroute GET \"/:id\" -> missing\n}\nfn main(){}",
    )
    .unwrap();
    assert_eq!(missing.checked_placeholders, 0);
    assert_eq!(missing.diagnostics.items[0].code, "E4033");
    let invalid = application::check_route_placeholders(
        "app Demo{\nroute BAD \"/:id\" -> handler\n}\nfn handler(){}",
    )
    .unwrap();
    assert_eq!(invalid.checked_placeholders, 0);
    assert_eq!(invalid.diagnostics.items[0].code, "E4030");
    let extra = application::check_route_placeholders(
        "app Demo{\nroute GET \"/:id\" -> handler\n}\nfn handler(id:Unknown,extra:String){}",
    )
    .unwrap();
    assert!(extra.diagnostics.is_empty()); // Names only; neither type nor extra-value binding is established.
}
