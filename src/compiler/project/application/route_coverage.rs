//! Opt-in file-local route checking from the service checker source snapshots.
use super::{
    syntax_coverage::project_coverage_with_budget, ServiceCheck, StructuredRoutePlaceholderCheck,
};
use std::{collections::BTreeMap, path::PathBuf};

/// Maximum submitted route bindings in the optional project report.
pub const MAX_PROJECT_ROUTE_BINDINGS: usize = 4096;
/// Maximum checked placeholders in the optional project report.
pub const MAX_PROJECT_ROUTE_PLACEHOLDERS: usize = 8192;

/// One owning source snapshot's limited route-binding outcome.
#[derive(Debug, Clone)]
pub struct FileRouteCheck {
    /// Owning file; all nested diagnostic spans belong to this path.
    pub source_file: PathBuf,
    /// Local route checks or explicit syntax/context/read unavailability.
    pub routes: Result<StructuredRoutePlaceholderCheck, String>,
}

/// Unchanged partial service checking plus independent local route outcomes.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithRoutes {
    /// Original service diagnostics, including E4096.
    pub check: ServiceCheck,
    /// Per-file route outcomes; cumulative exhaustion exposes no partial report.
    pub routes: Result<Vec<FileRouteCheck>, String>,
}

/// Retain file-local route checks from the service checker's loaded source strings.
/// No extra source reads, implicit cross-file binding or default API/CLI changes.
/// Callers must inspect both original diagnostics and every optional outcome.
pub fn check_service_calls_with_routes(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithRoutes {
    let inspected = super::imports::inspect_snapshot(project);
    let routes = coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithRoutes { check, routes }
}

fn coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileRouteCheck>, String> {
    let mut bindings = 0usize;
    let mut placeholders = 0usize;
    let mut diagnostics = 0usize;
    let mut bytes = 0usize;
    let parsed = project_coverage_with_budget(
        sources,
        super::check_structured_route_placeholders,
        |checked| checked.token_count,
        |checked| {
            bindings = bindings
                .checked_add(checked.routes.targets.checked_routes)
                .filter(|count| *count <= MAX_PROJECT_ROUTE_BINDINGS)
                .ok_or("project routes cumulative binding limit exceeded")?;
            placeholders = placeholders
                .checked_add(checked.routes.checked_placeholders)
                .filter(|count| *count <= MAX_PROJECT_ROUTE_PLACEHOLDERS)
                .ok_or("project routes cumulative placeholder limit exceeded")?;
            diagnostics = diagnostics
                .checked_add(
                    checked.routes.diagnostics.items.len()
                        + checked.routes.targets.diagnostics.items.len(),
                )
                .filter(|count| *count <= super::MAX_ENTRY_PATH_DIAGNOSTICS)
                .ok_or("project routes cumulative diagnostic count limit exceeded")?;
            bytes = bytes
                .checked_add(
                    checked
                        .routes
                        .diagnostics
                        .items
                        .iter()
                        .chain(&checked.routes.targets.diagnostics.items)
                        .map(|d| d.message.len())
                        .sum::<usize>(),
                )
                .filter(|count| *count <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                .ok_or("project routes cumulative diagnostic byte limit exceeded")?;
            Ok(())
        },
    )?;
    Ok(parsed
        .into_iter()
        .map(|(source_file, routes)| FileRouteCheck {
            source_file,
            routes,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources(values: &[String]) -> BTreeMap<PathBuf, Result<String, String>> {
        values
            .iter()
            .enumerate()
            .map(|(n, source)| {
                (
                    PathBuf::from(format!("snapshot-{n}.svr")),
                    Ok(source.clone()),
                )
            })
            .collect()
    }
    fn routes(count: usize, path: &str, handler: &str, parameters: &str) -> String {
        let members = (0..count)
            .map(|n| format!("route GET \"/p{n}{path}\" -> {handler}\n"))
            .collect::<String>();
        format!(
            "model Item{{amount:Money}}\napp Demo{{\n{members}}}\nfn {handler}({parameters}){{}}"
        )
    }
    #[test]
    fn snapshot_outcomes_keep_source_ownership_and_local_scope() {
        let mut files = sources(&[
            routes(1, "/:id", "handler", "id:Int"),
            routes(1, "/:id", "handler", ""),
            "model Item{}\napp Demo{\nroute GET \"/\" -> Item\n}".into(),
            "app Demo{\nroute GET \"/\" -> handler\n}".into(),
        ]);
        files.insert(
            PathBuf::from("missing.svr"),
            Err("snapshot read failed".into()),
        );
        let report = coverage(&files).unwrap();
        assert_eq!(report.iter().filter(|f| f.routes.is_err()).count(), 2);
        let get = |n: usize| {
            let path = PathBuf::from(format!("snapshot-{n}.svr"));
            report
                .iter()
                .find(|f| f.source_file == path)
                .unwrap()
                .routes
                .as_ref()
                .unwrap()
        };
        assert!(get(0).routes.diagnostics.is_empty());
        assert_eq!(get(1).routes.diagnostics.items[0].code, "E4144");
        assert_eq!(get(3).routes.diagnostics.items[0].code, "E4033"); // No cross-file target fallback.
        assert!(report
            .iter()
            .find(|f| f.source_file == std::path::Path::new("missing.svr"))
            .unwrap()
            .routes
            .as_ref()
            .unwrap_err()
            .contains("read failed"));
    }
    #[test]
    fn binding_and_placeholder_quotas_are_cumulative_and_exact() {
        let source = routes(MAX_PROJECT_ROUTE_BINDINGS / 2, "/:id", "handler", "id:Int");
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.routes.as_ref().unwrap().routes.targets.checked_routes)
                .sum::<usize>(),
            MAX_PROJECT_ROUTE_BINDINGS
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(routes(1, "/:id", "handler", "id:Int")),
        );
        assert!(coverage(&files).unwrap_err().contains("binding limit"));
        let source = routes(1024, "/:a/:b/:c/:d", "handler", "a:Int,b:Int,c:Int,d:Int");
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.routes.as_ref().unwrap().routes.checked_placeholders)
                .sum::<usize>(),
            MAX_PROJECT_ROUTE_PLACEHOLDERS
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(routes(1, "/:id", "handler", "id:Int")),
        );
        assert!(coverage(&files).unwrap_err().contains("placeholder limit"));
    }
    #[test]
    fn diagnostic_count_and_message_quotas_are_cumulative_and_exact() {
        let source = routes(512, "/:id", "handler", "");
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.routes.as_ref().unwrap().routes.diagnostics.items.len())
                .sum::<usize>(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(routes(1, "/:id", "handler", "")),
        );
        assert!(coverage(&files).unwrap_err().contains("diagnostic count"));
        // Missing-target diagnostics are retained twice by the existing API.
        let source = routes(256, "", "handler", "").replace("fn handler(){}", "fn unused(){}");
        let mut files = sources(&[source.clone(), source]);
        let exact = coverage(&files).unwrap();
        assert_eq!(
            exact
                .iter()
                .map(|f| {
                    let checked = &f.routes.as_ref().unwrap().routes;
                    checked.diagnostics.items.len() + checked.targets.diagnostics.items.len()
                })
                .sum::<usize>(),
            1024
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(routes(1, "/:id", "handler", "")),
        );
        assert!(coverage(&files).unwrap_err().contains("diagnostic count"));
        let handler = "h".repeat(5000);
        let source =
            routes(64, "", &handler, "").replace(&format!("fn {handler}(){{}}"), "fn unused(){}");
        let mut files = sources(&[source.clone(), source]);
        let exact = coverage(&files).unwrap();
        assert_eq!(
            exact
                .iter()
                .map(|f| {
                    let checked = &f.routes.as_ref().unwrap().routes;
                    checked
                        .diagnostics
                        .items
                        .iter()
                        .chain(&checked.targets.diagnostics.items)
                        .map(|d| d.message.len())
                        .sum::<usize>()
                })
                .sum::<usize>(),
            1024 * 1024
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(routes(1, "/:id", "handler", "")),
        );
        assert!(coverage(&files).unwrap_err().contains("diagnostic byte"));
        let source = routes(128, "/:id", &"h".repeat(5000), "");
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            coverage(&files)
                .unwrap()
                .iter()
                .flat_map(|f| &f.routes.as_ref().unwrap().routes.diagnostics.items)
                .map(|d| d.message.len())
                .sum::<usize>(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(routes(1, "/:id", "handler", "")),
        );
        assert!(coverage(&files).unwrap_err().contains("diagnostic byte"));
    }
    #[test]
    fn shared_file_and_original_opaque_token_limits_apply() {
        let mut files = (0..super::super::MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(String::new())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            coverage(&files).unwrap().len(),
            super::super::MAX_SYNTAX_FILES
        );
        files.insert(PathBuf::from("extra.svr"), Err("read failed".into()));
        assert!(coverage(&files).unwrap_err().contains("file limit"));
        let source = format!(
            "enum Opaque{{{}}}",
            "value ".repeat(super::super::syntax::MAX_CALLABLE_TOKENS - 4)
        );
        let mut files = sources(&vec![source; 4]);
        assert_eq!(
            coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.routes.as_ref().unwrap().token_count)
                .sum::<usize>(),
            super::super::MAX_SYNTAX_TOKENS
        );
        files.insert(PathBuf::from("extra.svr"), Ok("model Empty{}".into()));
        assert!(coverage(&files).unwrap_err().contains("token limit"));
    }
}
