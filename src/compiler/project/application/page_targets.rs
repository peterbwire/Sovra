//! Same-source page-route name bindings, without render/body/argument semantics.
use super::syntax::{
    self, AppMemberKind, ApplicationEntrySyntax, ApplicationRenderSyntax, RenderDeclarationKind,
};
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, TokenKind},
};
use std::collections::BTreeMap;

/// Page route linked to one explicit local page/view header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPageTarget {
    /// Original route binding range.
    pub binding_span: Span,
    /// Exact target identifier range.
    pub target_span: Span,
    /// Index in retained render declarations.
    pub declaration_index: usize,
    /// Original declaration envelope.
    pub declaration_span: Span,
    /// Page or view source category, not a function result type.
    pub kind: RenderDeclarationKind,
}

/// Limited local page-route name resolution, not complete render validation.
#[derive(Debug, Clone)]
pub struct PageTargetCheck {
    /// Entry nodes and explicit unknown source ranges.
    pub entries: ApplicationEntrySyntax,
    /// Render headers with opaque bodies/unresolved annotations.
    pub render: ApplicationRenderSyntax,
    /// Bindings linked by retained declaration identity.
    pub resolved: Vec<LocalPageTarget>,
    /// Existing E4042 missing-page-target diagnostics.
    pub diagnostics: Diagnostics,
}

/// Resolve unqualified page-route names to local page/view declarations.
/// Imports, qualified targets and ambiguous render names remain unavailable.
/// Ordinary functions do not satisfy page targets. No Unit default, implicit
/// argument injection, HTTP typing or runtime rendering is established.
pub fn check_local_page_targets(source: &str) -> Result<PageTargetCheck, String> {
    let entries = syntax::parse_entry_blocks(source)?;
    let render = syntax::parse_render_declarations(source)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "page target lexical errors")?;
    let mut depth = 0usize;
    for token in &tokens {
        if depth == 0 && token.kind == TokenKind::Keyword("use") {
            return Err("page target imports require module context; context unavailable".into());
        }
        match token.kind {
            TokenKind::Punctuation('{') => depth += 1,
            TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let mut names = BTreeMap::new();
    for (index, declaration) in render.declarations.iter().enumerate() {
        if names.insert(declaration.name.as_str(), index).is_some() {
            return Err("ambiguous page/view names; context unavailable".into());
        }
    }
    let mut diagnostics = Diagnostics::new();
    let mut bytes = 0usize;
    let mut resolved = Vec::new();
    for entry in &entries.entries {
        for member in &entry.members {
            let AppMemberKind::Page(page) = &member.kind else {
                continue;
            };
            if page.target.contains('.') {
                return Err(
                    "qualified page target requires module context; context unavailable".into(),
                );
            }
            let start = member
                .span
                .end
                .checked_sub(page.target.len())
                .ok_or("page target range mismatch")?;
            let index = tokens.partition_point(|token| token.span.start < start);
            let token = tokens
                .get(index)
                .filter(|token| token.span.start == start && token.span.end == member.span.end)
                .ok_or("page target token provenance unavailable")?;
            if source[token.span.start..token.span.end] != page.target {
                return Err("page target spelling mismatch".into());
            }
            let declaration = names.get(page.target.as_str()).copied();
            let mut local = Diagnostics::new();
            super::super::validate_page_target(page, declaration.is_some(), &mut local);
            for mut diagnostic in local.items {
                if diagnostics.items.len() == super::MAX_ENTRY_PATH_DIAGNOSTICS {
                    return Err("page target diagnostic count limit exceeded".into());
                }
                bytes = bytes
                    .checked_add(diagnostic.message.len())
                    .filter(|sum| *sum <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                    .ok_or("page target diagnostic byte limit exceeded")?;
                diagnostic.span = token.span;
                diagnostics.push(diagnostic);
            }
            if let Some(declaration_index) = declaration {
                resolved.push(LocalPageTarget {
                    binding_span: member.span,
                    target_span: token.span,
                    declaration_index,
                    declaration_span: render.declarations[declaration_index].span,
                    kind: render.declarations[declaration_index].kind,
                });
            }
        }
    }
    Ok(PageTargetCheck {
        entries,
        render,
        resolved,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_and_view_identity_is_distinct_from_ordinary_functions_and_render_validity() {
        let source = "// λ\r\napp Demo{\r\npage \"/\" -> Home\r\npage \"/card\" -> Card\r\npage \"/missing\" -> ordinary\r\n}\r\npage Home(user:Unknown){future body}\r\nview Card(){}\r\nfn ordinary(){}";
        let checked = check_local_page_targets(source).unwrap();
        assert_eq!(checked.resolved.len(), 2);
        assert_eq!(checked.resolved[0].kind, RenderDeclarationKind::Page);
        assert_eq!(checked.resolved[1].kind, RenderDeclarationKind::View);
        assert_eq!(checked.diagnostics.items[0].code, "E4042");
        let span = checked.diagnostics.items[0].span;
        assert_eq!(&source[span.start..span.end], "ordinary");
        assert!(checked.render.declarations[0].unparsed_body.is_some());
        for source in [
            "use app.pages;",
            "page Home(){}\nview Home(){}",
            "app Demo{\npage \"/\" -> pages.Home\n}",
        ] {
            assert!(check_local_page_targets(source)
                .unwrap_err()
                .contains("context unavailable"));
        }
    }
    #[test]
    fn missing_page_diagnostics_are_bounded_without_partial_bindings() {
        let members = "page \"/\" -> missing\n".repeat(super::super::MAX_ENTRY_PATH_DIAGNOSTICS);
        assert_eq!(
            check_local_page_targets(&format!("app Demo{{\n{members}}}"))
                .unwrap()
                .diagnostics
                .items
                .len(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        assert!(check_local_page_targets(&format!(
            "app Demo{{\n{members}page \"/extra\" -> missing\n}}"
        ))
        .unwrap_err()
        .contains("count limit"));
        let members = format!("page \"/\" -> {}\n", "x".repeat(1000)).repeat(1024);
        assert!(
            check_local_page_targets(&format!("app Demo{{\n{members}}}"))
                .unwrap_err()
                .contains("byte limit")
        );
    }
}
