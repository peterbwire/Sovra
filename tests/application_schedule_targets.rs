//! Schedule target identity is distinct from task body checking and runtime scheduling.
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
            "sovra-schedule-targets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("sovra.toml"),
            "[project]\nname = \"schedule-targets\"\nentry = \"main.svr\"\n",
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
fn structured_schedule_diagnostics_match_existing_scanner_codes_and_messages() {
    let source = "// λ\r\napp Demo{\r\ntask daily -> job\r\ntask daily -> job\r\ntask hourly -> absent\r\n}\r\ntask job(){}";
    let fixture = Fixture::new(source);
    let scanner = project::check_project(&fixture.0).unwrap_err();
    let checked = application::check_local_schedule_targets(source).unwrap();
    let mut expected = scanner
        .items
        .iter()
        .filter(|diagnostic| matches!(diagnostic.code, "E4071" | "E4072"))
        .map(|diagnostic| (diagnostic.code, &diagnostic.message))
        .collect::<Vec<_>>();
    let mut actual = checked
        .diagnostics
        .items
        .iter()
        .map(|diagnostic| (diagnostic.code, &diagnostic.message))
        .collect::<Vec<_>>();
    assert_eq!(
        actual.iter().map(|(code, _)| *code).collect::<Vec<_>>(),
        ["E4071", "E4072"]
    );
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
    assert_eq!(checked.resolved.len(), 1);
    let target = &checked.resolved[0];
    assert_eq!(
        &source[target.target_span.start..target.target_span.end],
        "job"
    );
    assert_eq!(
        checked.syntax.declarations.base.base.callables[target.callable_index]
            .function
            .name,
        "job"
    );
}
#[test]
fn schedule_name_success_never_validates_task_returns_or_schedule_execution() {
    let body = "task job()->Int{return \"bad\";}";
    let source = format!("app Demo{{\ntask deliberately opaque schedule -> job\n}}\n{body}");
    let checked = application::check_local_schedule_targets(&source).unwrap();
    assert!(checked.diagnostics.is_empty());
    assert_eq!(checked.resolved.len(), 1);
    let fixture = Fixture::new(body);
    let project = project::check_project(&fixture.0).unwrap();
    let contracts = application::check_service_calls(&project);
    assert!(contracts
        .diagnostics
        .items
        .iter()
        .any(|diagnostic| diagnostic.code == "E4131"));
    assert!(application::check_local_route_targets(&source).is_err()); // Task route context remains unavailable.
}
