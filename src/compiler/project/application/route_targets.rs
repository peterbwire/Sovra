//! Local route target name resolution against retained ordinary function declarations.
use super::syntax::{self, AppMemberKind, CallableKind, CombinedApplicationSyntax};
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span},
    lexer::Lexer,
};
use std::collections::BTreeMap;

/// A route target resolved by declaration identity, not substring matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRouteTarget {
    /// Original route declaration range.
    pub route_span: Span,
    /// Exact original target identifier token range.
    pub target_span: Span,
    /// Index in `syntax.declarations.base.base.callables`.
    pub callable_index: usize,
    /// Original target callable declaration envelope.
    pub declaration_span: Span,
}

/// Limited local target resolution, never full route/body/argument validation.
#[derive(Debug, Clone)]
pub struct LocalRouteTargetCheck {
    /// Original syntax with explicit unknown source ranges.
    pub syntax: CombinedApplicationSyntax,
    /// Number of route names submitted to resolution, including missing ones.
    pub checked_routes: usize,
    /// Uniquely resolved ordinary function declarations in route order.
    pub resolved: Vec<LocalRouteTarget>,
    /// Missing-target diagnostics; other semantic validation remains separate.
    pub diagnostics: Diagnostics,
}

/// Resolve unqualified same-file ordinary function targets in structured routes.
///
/// Imports, services/tasks, qualified targets and ambiguous callable names make
/// the request explicitly unavailable. Missing names produce E4033 at the target
/// token. Visibility, body/type validity, HTTP shape and argument binding are not
/// established. No filesystem/import resolution, execution or source writes occur.
pub fn check_local_route_targets(source: &str) -> Result<LocalRouteTargetCheck, String> {
    let syntax = syntax::parse_combined(source)?;
    check_targets_with_syntax(source, syntax)
}

pub(super) fn check_targets_with_syntax(
    source: &str,
    syntax: CombinedApplicationSyntax,
) -> Result<LocalRouteTargetCheck, String> {
    if !syntax.declarations.imports.is_empty() || !syntax.declarations.base.services.is_empty() {
        return Err(
            "local route resolution requires import/service context; context unavailable".into(),
        );
    }
    let callables = &syntax.declarations.base.base.callables;
    if callables
        .iter()
        .any(|callable| callable.kind != CallableKind::Function)
    {
        return Err(
            "local route resolution excludes task/service callables; context unavailable".into(),
        );
    }
    let mut names = BTreeMap::new();
    for (index, callable) in callables.iter().enumerate() {
        if names
            .insert(callable.function.name.as_str(), index)
            .is_some()
        {
            return Err(
                "local route resolution has ambiguous callable names; context unavailable".into(),
            );
        }
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "route target lexical errors")?;
    let mut checked_routes = 0;
    let mut resolved = Vec::new();
    let mut diagnostics = Diagnostics::new();
    let mut message_bytes = 0usize;
    for entry in &syntax.entries.entries {
        for member in &entry.members {
            let AppMemberKind::Route(route) = &member.kind else {
                continue;
            };
            if route.target.contains('.') {
                return Err(
                    "qualified route target requires module context; context unavailable".into(),
                );
            }
            let start = member
                .span
                .end
                .checked_sub(route.target.len())
                .ok_or("route target range mismatch")?;
            let index = tokens.partition_point(|token| token.span.start < start);
            let token = tokens
                .get(index)
                .filter(|token| token.span.start == start && token.span.end == member.span.end)
                .ok_or("route target token provenance unavailable")?;
            if source[token.span.start..token.span.end] != route.target {
                return Err("route target spelling mismatch".into());
            }
            checked_routes += 1;
            if let Some(&callable_index) = names.get(route.target.as_str()) {
                resolved.push(LocalRouteTarget {
                    route_span: member.span,
                    target_span: token.span,
                    callable_index,
                    declaration_span: callables[callable_index].span,
                });
            } else {
                if diagnostics.items.len() == super::MAX_ENTRY_PATH_DIAGNOSTICS {
                    return Err("local route diagnostic count limit exceeded".into());
                }
                let message = diagnostic_message!("route target `{}` was not found", route.target);
                message_bytes = message_bytes
                    .checked_add(message.len())
                    .filter(|bytes| *bytes <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                    .ok_or("local route diagnostic byte limit exceeded")?;
                diagnostics.push(Diagnostic {
                    source_file: None,
                    severity: Severity::Error,
                    code: "E4033",
                    message,
                    span: token.span,
                });
            }
        }
    }
    Ok(LocalRouteTargetCheck {
        syntax,
        checked_routes,
        resolved,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_targets_use_exact_identifier_ranges_and_retained_declaration_indices() {
        let source = "// λ\r\napp Demo{\r\nroute GET \"/handler\" -> handler // handler\r\nroute POST \"/\" -> absent\r\n}\r\nfn handler(x:Int){}";
        let checked = check_local_route_targets(source).unwrap();
        assert_eq!(checked.checked_routes, 2);
        assert_eq!(checked.resolved.len(), 1);
        let target = &checked.resolved[0];
        assert_eq!(
            &source[target.target_span.start..target.target_span.end],
            "handler"
        );
        assert_eq!(target.callable_index, 0);
        assert!(
            source[target.declaration_span.start..target.declaration_span.end]
                .starts_with("fn handler")
        );
        assert_eq!(checked.diagnostics.items[0].code, "E4033");
        let missing = checked.diagnostics.items[0].span;
        assert_eq!(&source[missing.start..missing.end], "absent");
    }
    #[test]
    fn unsupported_contexts_and_duplicate_declarations_never_select_a_target() {
        for source in [
            "use app.jobs; fn handler(){}",
            "service Api{} fn handler(){}",
            "task handler(){}",
            "fn handler(){} fn handler(){}",
            "app Demo{\nroute GET \"/\" -> jobs.handler\n}\nfn handler(){}",
        ] {
            assert!(
                check_local_route_targets(source)
                    .unwrap_err()
                    .contains("context unavailable"),
                "{source}"
            );
        }
        let source = format!(
            "app Demo{{\n{}}}",
            "route GET \"/\" -> missing\n".repeat(super::super::MAX_ENTRY_PATH_DIAGNOSTICS)
        );
        assert_eq!(
            check_local_route_targets(&source)
                .unwrap()
                .diagnostics
                .items
                .len(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        let over = source.replacen("app Demo{", "app Demo{\nroute GET \"/\" -> extra", 1);
        assert!(check_local_route_targets(&over)
            .unwrap_err()
            .contains("count limit"));
        let wide = format!(
            "app Demo{{\n{}}}",
            format!("route GET \"/\" -> {}\n", "x".repeat(1000))
                .repeat(super::super::MAX_ENTRY_PATH_DIAGNOSTICS)
        );
        assert!(wide.len() <= syntax::MAX_SOURCE_BYTES);
        assert!(check_local_route_targets(&wide)
            .unwrap_err()
            .contains("byte limit"));
    }
}
