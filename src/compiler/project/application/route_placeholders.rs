//! Explicit route-placeholder names checked against retained local handler parameters.
use super::{check_local_route_targets, syntax::AppMemberKind, LocalRouteTargetCheck};
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity},
    lexer::{Lexer, TokenKind},
};
use std::collections::{BTreeMap, BTreeSet};

/// Limited placeholder-name checking; not HTTP decoding or complete argument binding.
#[derive(Debug, Clone)]
pub struct RoutePlaceholderCheck {
    /// Existing local target resolution with original syntax and identities.
    pub targets: LocalRouteTargetCheck,
    /// Placeholder names submitted for resolved, shape-valid handlers.
    pub checked_placeholders: usize,
    /// Missing targets, route shape errors and E4144 missing explicit parameters.
    pub diagnostics: Diagnostics,
}

fn append(
    diagnostics: &mut Diagnostics,
    bytes: &mut usize,
    diagnostic: Diagnostic,
) -> Result<(), String> {
    if diagnostics.items.len() >= super::MAX_ENTRY_PATH_DIAGNOSTICS {
        return Err("route placeholder diagnostic count limit exceeded".into());
    }
    *bytes = bytes
        .checked_add(diagnostic.message.len())
        .filter(|sum| *sum <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
        .ok_or("route placeholder diagnostic byte limit exceeded")?;
    diagnostics.push(diagnostic);
    Ok(())
}

/// Require each route placeholder to name an explicit local handler parameter.
/// Extra handler parameters, HTTP types/decoding and runtime injection remain
/// unvalidated. Repeated placeholder names and duplicate parameter names make
/// this limited binding context unavailable rather than choosing an interpretation.
/// Existing target-context and diagnostic quotas apply with no partial results.
pub fn check_route_placeholders(source: &str) -> Result<RoutePlaceholderCheck, String> {
    let targets = check_local_route_targets(source)?;
    check_placeholders_with_targets(source, targets)
}

pub(super) fn check_placeholders_with_targets(
    source: &str,
    targets: LocalRouteTargetCheck,
) -> Result<RoutePlaceholderCheck, String> {
    let resolved: BTreeMap<_, _> = targets
        .resolved
        .iter()
        .map(|target| (target.route_span.start, target))
        .collect();
    // Build each referenced handler's parameter set once, even for many routes.
    let mut parameter_sets = BTreeMap::new();
    for target in &targets.resolved {
        if parameter_sets.contains_key(&target.callable_index) {
            continue;
        }
        let function =
            &targets.syntax.declarations.base.base.callables[target.callable_index].function;
        let mut parameters = BTreeSet::new();
        if function
            .parameters
            .iter()
            .any(|parameter| !parameters.insert(parameter.name.as_str()))
        {
            return Err("ambiguous handler parameters; context unavailable".into());
        }
        parameter_sets.insert(target.callable_index, parameters);
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "route placeholder lexical errors")?;
    let mut diagnostics = Diagnostics::new();
    let mut bytes = 0;
    for diagnostic in &targets.diagnostics.items {
        append(&mut diagnostics, &mut bytes, diagnostic.clone())?;
    }
    let mut seen_routes = BTreeSet::new();
    let mut checked_placeholders = 0;
    for entry in &targets.syntax.entries.entries {
        for member in &entry.members {
            let AppMemberKind::Route(route) = &member.kind else {
                continue;
            };
            let mut shape = Diagnostics::new();
            super::super::validate_route_shape(route, &mut seen_routes, &mut shape);
            if !shape.is_empty() {
                for mut diagnostic in shape.items {
                    diagnostic.span = member.span;
                    append(&mut diagnostics, &mut bytes, diagnostic)?;
                }
                continue;
            }
            let Some(target) = resolved.get(&member.span.start) else {
                continue;
            };
            let function =
                &targets.syntax.declarations.base.base.callables[target.callable_index].function;
            let parameters = parameter_sets
                .get(&target.callable_index)
                .ok_or("handler parameter provenance unavailable")?;
            let start = tokens.partition_point(|token| token.span.start < member.span.start);
            let path_span = tokens[start..]
                .iter()
                .take_while(|token| token.span.end <= member.span.end)
                .find(|token| matches!(token.kind, TokenKind::String(_)))
                .ok_or("route path token provenance unavailable")?
                .span;
            let mut placeholders = BTreeSet::new();
            for name in route
                .path
                .split('/')
                .filter_map(|segment| segment.strip_prefix(':'))
            {
                if !placeholders.insert(name) {
                    return Err(
                        "repeated route placeholder requires binding context; context unavailable"
                            .into(),
                    );
                }
                checked_placeholders += 1;
                if !parameters.contains(name) {
                    append(&mut diagnostics, &mut bytes, Diagnostic {
                        source_file: None, severity: Severity::Error, code: "E4144",
                        message: diagnostic_message!("route placeholder `:{name}` requires an explicit parameter named `{name}` in handler `{}`", function.name),
                        span: path_span,
                    })?;
                }
            }
        }
    }
    diagnostics
        .items
        .sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
    Ok(RoutePlaceholderCheck {
        targets,
        checked_placeholders,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholders_require_explicit_names_and_preserve_literal_source_ranges() {
        let source = "// λ\r\napp Demo{\r\nroute GET \"/items/:id/:detail\" -> handler\r\n}\r\nfn handler(id:Unknown){}";
        let checked = check_route_placeholders(source).unwrap();
        assert_eq!(checked.checked_placeholders, 2);
        assert_eq!(checked.diagnostics.items.len(), 1);
        let diagnostic = &checked.diagnostics.items[0];
        assert_eq!(diagnostic.code, "E4144");
        assert_eq!(
            &source[diagnostic.span.start..diagnostic.span.end],
            "\"/items/:id/:detail\""
        );
        assert!(check_route_placeholders(
            "app Demo{\nroute GET \"/:id\" -> handler\n}\nfn handler(id:Int,other:String){}"
        )
        .unwrap()
        .diagnostics
        .is_empty());
        assert!(check_route_placeholders(
            "app Demo{\nroute GET \"/:id/:id\" -> handler\n}\nfn handler(id:Int){}"
        )
        .unwrap_err()
        .contains("context unavailable"));
        assert!(check_route_placeholders(
            "app Demo{\nroute GET \"/:id\" -> handler\n}\nfn handler(id:Int,id:Int){}"
        )
        .unwrap_err()
        .contains("duplicate"));
    }
    #[test]
    fn diagnostic_exhaustion_never_returns_partial_binding_results() {
        let path = (0..super::super::MAX_ENTRY_PATH_DIAGNOSTICS)
            .map(|n| format!("/:p{n}"))
            .collect::<String>();
        let exact = format!("app Demo{{\nroute GET \"{path}\" -> handler\n}}\nfn handler(){{}}");
        assert_eq!(
            check_route_placeholders(&exact)
                .unwrap()
                .diagnostics
                .items
                .len(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        let over =
            format!("app Demo{{\nroute GET \"{path}/:extra\" -> handler\n}}\nfn handler(){{}}");
        assert!(check_route_placeholders(&over)
            .unwrap_err()
            .contains("count limit"));
        let wide = (0..520)
            .map(|n| format!("/:p{}{n}", "x".repeat(1000)))
            .collect::<String>();
        let source = format!("app Demo{{\nroute GET \"{wide}\" -> handler\n}}\nfn handler(){{}}");
        assert!(check_route_placeholders(&source)
            .unwrap_err()
            .contains("byte limit"));
    }
}
