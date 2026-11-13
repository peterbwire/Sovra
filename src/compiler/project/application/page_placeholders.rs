//! Explicit page-page placeholder names checked against retained page/view parameters.
use super::{check_local_page_targets, syntax::AppMemberKind, PageTargetCheck};
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity},
    lexer::{Lexer, TokenKind},
};
use std::collections::{BTreeMap, BTreeSet};

/// Limited placeholder-name checking; not HTTP decoding or complete argument binding.
#[derive(Debug, Clone)]
pub struct PagePlaceholderCheck {
    /// Existing local target resolution with original syntax and identities.
    pub targets: PageTargetCheck,
    /// Placeholder names submitted for resolved, shape-valid render targets.
    pub checked_placeholders: usize,
    /// Missing targets, route shape errors and E4145 missing explicit parameters.
    pub diagnostics: Diagnostics,
}

fn append(
    diagnostics: &mut Diagnostics,
    bytes: &mut usize,
    diagnostic: Diagnostic,
) -> Result<(), String> {
    if diagnostics.items.len() >= super::MAX_ENTRY_PATH_DIAGNOSTICS {
        return Err("page placeholder diagnostic count limit exceeded".into());
    }
    *bytes = bytes
        .checked_add(diagnostic.message.len())
        .filter(|sum| *sum <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
        .ok_or("page placeholder diagnostic byte limit exceeded")?;
    diagnostics.push(diagnostic);
    Ok(())
}

/// Require each page placeholder to name an explicit local render target parameter.
/// Extra render target parameters, HTTP types/decoding and runtime injection remain
/// unvalidated. Repeated placeholder names and duplicate parameter names make
/// this limited binding context unavailable rather than choosing an interpretation.
/// Existing target-context and diagnostic quotas apply with no partial results.
pub fn check_page_placeholders(source: &str) -> Result<PagePlaceholderCheck, String> {
    let targets = check_local_page_targets(source)?;
    let resolved: BTreeMap<_, _> = targets
        .resolved
        .iter()
        .map(|target| (target.binding_span.start, target))
        .collect();
    // Build each referenced render target's parameter set once, even for many routes.
    let mut parameter_sets = BTreeMap::new();
    for target in &targets.resolved {
        if parameter_sets.contains_key(&target.declaration_index) {
            continue;
        }
        let declaration = &targets.render.declarations[target.declaration_index];
        let mut parameters = BTreeSet::new();
        if declaration
            .parameters
            .iter()
            .any(|parameter| !parameters.insert(parameter.name.as_str()))
        {
            return Err("ambiguous render target parameters; context unavailable".into());
        }
        parameter_sets.insert(target.declaration_index, parameters);
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "page placeholder lexical errors")?;
    let mut diagnostics = Diagnostics::new();
    let mut bytes = 0;
    for diagnostic in &targets.diagnostics.items {
        append(&mut diagnostics, &mut bytes, diagnostic.clone())?;
    }
    let mut seen_routes = BTreeSet::new();
    let mut checked_placeholders = 0;
    for entry in &targets.entries.entries {
        for member in &entry.members {
            let AppMemberKind::Page(route) = &member.kind else {
                continue;
            };
            let mut shape = Diagnostics::new();
            super::super::validate_page_shape(route, &mut seen_routes, &mut shape);
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
            let declaration = &targets.render.declarations[target.declaration_index];
            let parameters = parameter_sets
                .get(&target.declaration_index)
                .ok_or("render target parameter provenance unavailable")?;
            let start = tokens.partition_point(|token| token.span.start < member.span.start);
            let path_span = tokens[start..]
                .iter()
                .take_while(|token| token.span.end <= member.span.end)
                .find(|token| matches!(token.kind, TokenKind::String(_)))
                .ok_or("page path token provenance unavailable")?
                .span;
            let mut placeholders = BTreeSet::new();
            for name in route
                .path
                .split('/')
                .filter_map(|segment| segment.strip_prefix(':'))
            {
                if !placeholders.insert(name) {
                    return Err(
                        "repeated page placeholder requires binding context; context unavailable"
                            .into(),
                    );
                }
                checked_placeholders += 1;
                if !parameters.contains(name) {
                    append(&mut diagnostics, &mut bytes, Diagnostic {
                        source_file: None, severity: Severity::Error, code: "E4145",
                        message: diagnostic_message!("page placeholder `:{name}` requires an explicit parameter named `{name}` in render target `{}`", declaration.name),
                        span: path_span,
                    })?;
                }
            }
        }
    }
    diagnostics
        .items
        .sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
    Ok(PagePlaceholderCheck {
        targets,
        checked_placeholders,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_page_names_match_without_implying_types_results_or_extra_arguments() {
        let source = "// λ\r\napp Demo{\r\npage \"/jobs/:id/:detail\" -> Detail\r\n}\r\npage Detail(id:Unknown,user:Session){unparsed body}";
        let checked = check_page_placeholders(source).unwrap();
        assert_eq!(checked.checked_placeholders, 2);
        assert_eq!(checked.diagnostics.items.len(), 1);
        let diagnostic = &checked.diagnostics.items[0];
        assert_eq!(diagnostic.code, "E4145");
        assert_eq!(
            &source[diagnostic.span.start..diagnostic.span.end],
            "\"/jobs/:id/:detail\""
        );
        assert_eq!(
            checked.targets.render.declarations[0].explicit_return_annotation,
            None
        );
        assert!(checked.targets.render.declarations[0]
            .unparsed_body
            .is_some());
        let matched = source.replace(
            "id:Unknown,user:Session",
            "id:Unknown,detail:String,user:Session",
        );
        assert!(check_page_placeholders(&matched)
            .unwrap()
            .diagnostics
            .is_empty());
        assert!(check_page_placeholders(
            "app Demo{\npage \"/:id/:id\" -> Detail\n}\nview Detail(id:Int){}"
        )
        .unwrap_err()
        .contains("context unavailable"));
        assert!(check_page_placeholders(
            "app Demo{\npage \"/:id\" -> Detail\n}\npage Detail(id:Int,id:Int){}"
        )
        .is_err());
    }
    #[test]
    fn diagnostic_exhaustion_returns_no_partial_page_binding_result() {
        let path = (0..super::super::MAX_ENTRY_PATH_DIAGNOSTICS)
            .map(|n| format!("/:p{n}"))
            .collect::<String>();
        let exact = format!("app Demo{{\npage \"{path}\" -> Home\n}}\npage Home(){{}}");
        assert_eq!(
            check_page_placeholders(&exact)
                .unwrap()
                .diagnostics
                .items
                .len(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        let over = format!("app Demo{{\npage \"{path}/:extra\" -> Home\n}}\npage Home(){{}}");
        assert!(check_page_placeholders(&over)
            .unwrap_err()
            .contains("count limit"));
        let wide = (0..520)
            .map(|n| format!("/:p{}{n}", "x".repeat(1000)))
            .collect::<String>();
        assert!(check_page_placeholders(&format!(
            "app Demo{{\npage \"{wide}\" -> Home\n}}\npage Home(){{}}"
        ))
        .unwrap_err()
        .contains("byte limit"));
    }
}
