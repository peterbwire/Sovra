//! Integrated header coverage preserves all incomplete semantics and body gaps.
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
            "sovra-structured-coverage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"structured-coverage\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(path.join("main.svr"), "fn main(){}\n").unwrap();
        std::fs::write(path.join("declarations.svr"), source).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn project_header_coverage_never_suppresses_existing_incomplete_semantic_errors() {
    let fixture = Fixture::new(
        "model Item{}\nenum Role{}\npage Home(id:Int){}\nview Card(){}\nauth session{}\n",
    );
    let project = project::check_project(&fixture.0).unwrap();
    let old = application::check_service_calls(&project);
    let structured = application::check_service_calls_with_structured_syntax(&project);
    assert_eq!(old.files, structured.check.files);
    assert_eq!(old.diagnostics.items, structured.check.diagnostics.items);
    assert!(structured
        .check
        .diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4096"));
    let files = structured.syntax.unwrap();
    let parsed = files
        .iter()
        .find(|file| file.source_file == fixture.0.join("declarations.svr"))
        .unwrap()
        .syntax
        .as_ref()
        .unwrap();
    assert_eq!(parsed.data.declarations.len(), 2);
    assert_eq!(parsed.render.declarations.len(), 2);
    assert!(parsed.unparsed_ranges.is_empty()); // Headers only, no type/authorization validity.
    let previous = application::check_service_calls_with_auth_syntax(&project)
        .syntax
        .unwrap();
    assert!(previous
        .iter()
        .find(|file| file.source_file == fixture.0.join("declarations.svr"))
        .unwrap()
        .syntax
        .is_err());
}
#[test]
fn real_fielddesk_auth_data_bodies_and_guards_remain_unparsed_after_composition() {
    let source = include_str!("../examples/fielddesk/app/auth.svr");
    let parsed = syntax::parse_structured(source).unwrap();
    assert_eq!(parsed.auth.blocks[0].policies.len(), 3);
    assert_eq!(parsed.data.declarations.len(), 2);
    for declaration in &parsed.data.declarations {
        let body = declaration.unparsed_body.unwrap();
        assert!(parsed
            .unparsed_ranges
            .iter()
            .any(|gap| gap.start <= body.start && body.end <= gap.end));
    }
    for policy in &parsed.auth.blocks[0].policies {
        if let syntax::PolicyConditionSyntax::Unparsed { span } = policy.condition {
            assert!(parsed
                .unparsed_ranges
                .iter()
                .any(|gap| gap.start <= span.start && span.end <= gap.end));
        }
    }
    assert!(syntax::parse_with_auth(source).is_err());
    // Ordinary unsupported DSL bodies outside render containers are not hidden.
    let pages = include_str!("../examples/fielddesk/app/pages.svr");
    assert!(syntax::parse_structured(pages).is_err());
}

#[test]
fn optional_model_field_facts_never_suppress_project_incomplete_checking() {
    let source = "model Item{\nid:Int\ntitle:String,\n}\n";
    let fixture = Fixture::new(source);
    let project = project::check_project(&fixture.0).unwrap();
    let before = application::check_service_calls(&project);
    let fields = application::check_model_primitive_fields(source).unwrap();
    assert_eq!(fields.resolved.len(), 2);
    let after = application::check_service_calls_with_structured_syntax(&project);
    assert_eq!(before.files, after.check.files);
    assert_eq!(before.diagnostics.items, after.check.diagnostics.items);
    assert!(after
        .check
        .diagnostics
        .items
        .iter()
        .any(|d| d.code == "E4096"));
    let source = "model Item{id:Money}\nauth session{\nallow user to read on [Item] where false\n}";
    assert!(!application::check_model_primitive_fields(source)
        .unwrap()
        .diagnostics
        .is_empty());
    let policy = application::check_local_policy_models(source).unwrap();
    assert_eq!(policy.resolved.len(), 1); // Name reference only, never a grant.
    assert!(matches!(
        policy.auth.blocks[0].policies[0].condition,
        syntax::PolicyConditionSyntax::Unparsed { .. }
    ));
}

#[test]
fn project_model_fields_are_optional_and_diagnostics_keep_file_ownership() {
    let fixture = Fixture::new("model Item{\nid:Int\ntitle:String\n}");
    std::fs::write(fixture.0.join("bad.svr"), "model Bad{amount:Money}").unwrap();
    std::fs::write(
        fixture.0.join("unsupported.svr"),
        "model Unsupported{id:Int=1}",
    )
    .unwrap();
    let project = project::check_project(&fixture.0).unwrap();
    let old = application::check_service_calls(&project);
    let report = application::check_service_calls_with_model_fields(&project);
    assert_eq!(old.files, report.check.files);
    assert_eq!(old.diagnostics.items, report.check.diagnostics.items);
    assert!(report
        .check
        .diagnostics
        .items
        .iter()
        .any(|d| d.code == "E4096"));
    let files = report.model_fields.unwrap();
    let good = files
        .iter()
        .find(|f| f.source_file == fixture.0.join("declarations.svr"))
        .unwrap()
        .fields
        .as_ref()
        .unwrap();
    assert_eq!(good.resolved.len(), 2);
    let bad = files
        .iter()
        .find(|f| f.source_file == fixture.0.join("bad.svr"))
        .unwrap()
        .fields
        .as_ref()
        .unwrap();
    assert_eq!(bad.diagnostics.items[0].code, "E3017");
    assert_eq!(
        &"model Bad{amount:Money}"
            [bad.diagnostics.items[0].span.start..bad.diagnostics.items[0].span.end],
        "Money"
    );
    assert!(bad.resolved.is_empty());
    assert!(files
        .iter()
        .find(|f| f.source_file == fixture.0.join("unsupported.svr"))
        .unwrap()
        .fields
        .is_err());
}

#[test]
fn structured_local_functions_preserve_project_incomplete_diagnostics() {
    let source =
        "model Item{amount:Money}\npage Home(){fn hidden(){}}\nfn helper(){let value:Int=true;}";
    let fixture = Fixture::new(source);
    let project = project::check_project(&fixture.0).unwrap();
    let before = application::check_service_calls(&project);
    let checked = application::check_structured_local_functions(source).unwrap();
    assert_eq!(checked.checked_functions, 1);
    assert!(!checked.diagnostics.is_empty());
    assert!(checked
        .diagnostics
        .items
        .iter()
        .all(|d| d.span.start >= source.find("fn helper").unwrap()));
    let after = application::check_service_calls(&project);
    assert_eq!(before.files, after.files);
    assert_eq!(before.diagnostics.items, after.diagnostics.items);
    assert!(after.diagnostics.items.iter().any(|d| d.code == "E4096"));
}

#[test]
fn optional_project_local_function_checks_keep_core_spans_and_e4096() {
    let source = "// λ\r\nmodel Item{amount:Money}\r\nfn helper(){let value:Int=true;}";
    let fixture = Fixture::new(source);
    std::fs::write(
        fixture.0.join("context.svr"),
        "model Other{}\nfn contextual(value:Other){}",
    )
    .unwrap();
    let project = project::check_project(&fixture.0).unwrap();
    let old = application::check_service_calls(&project);
    let report = application::check_service_calls_with_local_functions(&project);
    assert_eq!(old.files, report.check.files);
    assert_eq!(old.diagnostics.items, report.check.diagnostics.items);
    assert!(report
        .check
        .diagnostics
        .items
        .iter()
        .any(|d| d.code == "E4096"));
    let files = report.local_functions.unwrap();
    let checked = files
        .iter()
        .find(|f| f.source_file == fixture.0.join("declarations.svr"))
        .unwrap()
        .functions
        .as_ref()
        .unwrap();
    let expected = application::check_structured_local_functions(source).unwrap();
    assert_eq!(checked.diagnostics.items, expected.diagnostics.items);
    assert_eq!(checked.checked_functions, 1);
    assert_eq!(checked.diagnostics.items[0].span.line, 2);
    assert!(files
        .iter()
        .find(|f| f.source_file == fixture.0.join("context.svr"))
        .unwrap()
        .functions
        .is_err());
    assert_eq!(
        files
            .iter()
            .find(|f| f.source_file == fixture.0.join("main.svr"))
            .unwrap()
            .functions
            .as_ref()
            .unwrap()
            .checked_functions,
        1
    );
}

#[test]
fn structured_route_rules_keep_default_project_errors_intact() {
    let source="model Item{amount:Money}\napp Demo{\nroute GET \"/:id\" -> handler\n}\nfn handler(id:Int){}";
    let fixture = Fixture::new(source);
    let project = project::check_project(&fixture.0).unwrap();
    let old = application::check_service_calls(&project);
    let routes = application::check_structured_route_placeholders(source).unwrap();
    assert!(routes.routes.diagnostics.is_empty());
    assert_eq!(routes.routes.targets.resolved.len(), 1);
    assert_eq!(routes.routes.checked_placeholders, 1);
    let after = application::check_service_calls_with_structured_syntax(&project);
    assert_eq!(old.files, after.check.files);
    assert_eq!(old.diagnostics.items, after.check.diagnostics.items);
    assert!(after
        .check
        .diagnostics
        .items
        .iter()
        .any(|d| d.code == "E4096"));
}

#[test]
fn optional_project_route_checks_keep_file_owned_diagnostics_and_e4096() {
    let source="// λ\r\nmodel Item{amount:Money}\r\napp Demo{\r\nroute GET \"/:id\" -> handler\r\n}\r\nfn handler(){}";
    let fixture = Fixture::new(source);
    std::fs::write(
        fixture.0.join("context.svr"),
        "model Other{}\napp OtherApp{\nroute GET \"/other\" -> Other\n}\nfn Other(){}",
    )
    .unwrap();
    let project = project::check_project(&fixture.0).unwrap();
    let old = application::check_service_calls(&project);
    let report = application::check_service_calls_with_routes(&project);
    assert_eq!(old.files, report.check.files);
    assert_eq!(old.diagnostics.items, report.check.diagnostics.items);
    assert!(report
        .check
        .diagnostics
        .items
        .iter()
        .any(|d| d.code == "E4096"));
    let files = report.routes.unwrap();
    let checked = files
        .iter()
        .find(|f| f.source_file == fixture.0.join("declarations.svr"))
        .unwrap()
        .routes
        .as_ref()
        .unwrap();
    let expected = application::check_structured_route_placeholders(source).unwrap();
    assert_eq!(
        checked.routes.diagnostics.items,
        expected.routes.diagnostics.items
    );
    assert_eq!(checked.routes.diagnostics.items[0].code, "E4144");
    assert_eq!(checked.routes.diagnostics.items[0].span.line, 3);
    assert!(files
        .iter()
        .find(|f| f.source_file == fixture.0.join("context.svr"))
        .unwrap()
        .routes
        .is_err());
}
