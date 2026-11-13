//! Route/page shape checks shared with the partial scanner, without target resolution.
use super::syntax::{self, AppMemberKind, ApplicationEntrySyntax};
use crate::compiler::diagnostics::Diagnostics;
use std::collections::BTreeSet;

/// Maximum diagnostics admitted by the entry-path projection.
pub const MAX_ENTRY_PATH_DIAGNOSTICS: usize = 1024;
/// Maximum cumulative UTF-8 diagnostic message bytes in the entry-path projection.
pub const MAX_ENTRY_PATH_DIAGNOSTIC_BYTES: usize = 1024 * 1024;

/// Limited route/page validation, separate from target/body/type checking.
#[derive(Debug, Clone)]
pub struct EntryPathCheck {
    /// Original entry-only syntax and explicit unsupported source ranges.
    pub syntax: ApplicationEntrySyntax,
    /// Number of route declarations submitted, including invalid ones.
    pub checked_routes: usize,
    /// Number of page bindings submitted, including invalid ones.
    pub checked_pages: usize,
    /// Existing scanner codes with exact original member spans.
    pub diagnostics: Diagnostics,
}

/// Validate supported methods, public path shape and duplicate route/page bindings.
///
/// Shares scanner rules/codes; target resolution, placeholder argument binding,
/// policy checks, schedules, other members and callable bodies remain unvalidated.
/// Diagnostic exhaustion rejects the whole request, never a partial success.
/// No filesystem reads, source writes or application execution occur.
pub fn check_entry_paths(source: &str) -> Result<EntryPathCheck, String> {
    let syntax = syntax::parse_entry_blocks(source)?;
    let mut routes = BTreeSet::new();
    let mut pages = BTreeSet::new();
    let mut diagnostics = Diagnostics::new();
    let mut message_bytes = 0usize;
    let mut checked_routes = 0;
    let mut checked_pages = 0;
    for entry in &syntax.entries {
        for member in &entry.members {
            let mut local = Diagnostics::new();
            match &member.kind {
                AppMemberKind::Route(route) => {
                    checked_routes += 1;
                    super::super::validate_route_shape(route, &mut routes, &mut local);
                }
                AppMemberKind::Page(page) => {
                    checked_pages += 1;
                    super::super::validate_page_shape(page, &mut pages, &mut local);
                }
                _ => continue,
            }
            for mut diagnostic in local.items {
                if diagnostics.items.len() == MAX_ENTRY_PATH_DIAGNOSTICS {
                    return Err("entry path diagnostic count limit exceeded".into());
                }
                message_bytes = message_bytes
                    .checked_add(diagnostic.message.len())
                    .filter(|bytes| *bytes <= MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                    .ok_or("entry path diagnostic byte limit exceeded")?;
                diagnostic.span = member.span;
                diagnostics.push(diagnostic);
            }
        }
    }
    Ok(EntryPathCheck {
        syntax,
        checked_routes,
        checked_pages,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shape_errors_use_existing_codes_and_original_member_ranges() {
        let source = "// λ\r\napp Demo{\r\nroute BAD \"relative path\" -> missing\r\nroute GET \"/ok\" -> missing\r\nroute GET \"/ok\" -> another\r\npage \"relative\" -> missing\r\npage \"/\" -> missing\r\npage \"/\" -> another\r\n}\r\n";
        let checked = check_entry_paths(source).unwrap();
        assert_eq!(checked.checked_routes, 3);
        assert_eq!(checked.checked_pages, 3);
        assert_eq!(
            checked
                .diagnostics
                .items
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["E4030", "E4031", "E4034", "E4032", "E4040", "E4041"]
        );
        for diagnostic in &checked.diagnostics.items {
            let text = &source[diagnostic.span.start..diagnostic.span.end];
            assert!(text.starts_with("route ") || text.starts_with("page "));
            assert!(diagnostic.source_file.is_none());
        }
        assert!(check_entry_paths(
            "app Demo{\nroute GET \"/items/:id\" -> missing\npage \"/\" -> missing\n}\n"
        )
        .unwrap()
        .diagnostics
        .is_empty());
    }
    #[test]
    fn diagnostic_limits_fail_closed_at_exact_count_and_cumulative_bytes() {
        let exact = format!(
            "app Demo{{\n{}}}",
            "route GET \"/\" -> target\n".repeat(MAX_ENTRY_PATH_DIAGNOSTICS + 1)
        );
        assert_eq!(
            check_entry_paths(&exact).unwrap().diagnostics.items.len(),
            MAX_ENTRY_PATH_DIAGNOSTICS
        );
        let over = format!(
            "app Demo{{\n{}}}",
            "route GET \"/\" -> target\n".repeat(MAX_ENTRY_PATH_DIAGNOSTICS + 2)
        );
        assert!(check_entry_paths(&over)
            .unwrap_err()
            .contains("count limit"));
        let members = (0..260)
            .map(|n| {
                format!(
                    "route BAD \"relative space {}{n}\" -> target\n",
                    "x".repeat(2048)
                )
            })
            .collect::<String>();
        assert!(check_entry_paths(&format!("app Demo{{\n{members}}}"))
            .unwrap_err()
            .contains("byte limit"));
    }
}
