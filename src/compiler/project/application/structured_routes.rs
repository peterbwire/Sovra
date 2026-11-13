//! Existing local route rules beside opaque structured application headers.
use super::{
    syntax::{
        self, AppMemberKind, ApplicationAuthSyntax, ApplicationDataSyntax, ApplicationRenderSyntax,
    },
    RoutePlaceholderCheck,
};
use crate::compiler::diagnostics::Span;
use std::collections::HashSet;

/// Scoped route checks plus remaining structured application coverage.
#[derive(Debug, Clone)]
pub struct StructuredRoutePlaceholderCheck {
    /// Local route target identities, path/placeholder diagnostics and combined syntax.
    pub routes: RoutePlaceholderCheck,
    /// Auth headers, never grants or typed guards.
    pub auth: ApplicationAuthSyntax,
    /// Model/enum headers with opaque bodies, never executable constructors.
    pub data: ApplicationDataSyntax,
    /// Render headers with unresolved annotations/results and opaque UI bodies.
    pub render: ApplicationRenderSyntax,
    /// Original source byte length.
    pub source_bytes: usize,
    /// Original non-EOF token count, including excluded bodies.
    pub token_count: usize,
    /// Aggregate unparsed source ranges; checks do not erase coverage gaps.
    pub unparsed_ranges: Vec<Span>,
}

/// Apply existing same-source ordinary route target/path/placeholder rules beside
/// structured auth/data/render headers. No handler body/type, HTTP decoding,
/// extra-parameter binding, policy or render semantics are established.
/// Imports/services/tasks, qualified/ambiguous targets and repeated placeholders
/// retain their existing unavailable outcomes. A target matching an excluded
/// header spelling also requires context rather than choosing namespace semantics.
/// Older APIs/default checking remain strict; no execution or source writes occur.
pub fn check_structured_route_placeholders(
    source: &str,
) -> Result<StructuredRoutePlaceholderCheck, String> {
    let syntax::StructuredApplicationSyntax {
        source_bytes,
        token_count,
        base,
        auth,
        data,
        render,
        unparsed_ranges,
    } = syntax::parse_structured(source)?;
    let names: HashSet<_> = data
        .declarations
        .iter()
        .map(|d| d.name.as_str())
        .chain(render.declarations.iter().map(|d| d.name.as_str()))
        .chain(auth.blocks.iter().map(|block| block.name.as_str()))
        .collect();
    for entry in &base.entries.entries {
        for member in &entry.members {
            if let AppMemberKind::Route(route) = &member.kind {
                if names.contains(route.target.as_str()) {
                    return Err(
                        "route target matches excluded declaration spelling; context unavailable"
                            .into(),
                    );
                }
            }
        }
    }
    let targets = super::route_targets::check_targets_with_syntax(source, base)?;
    let routes = super::route_placeholders::check_placeholders_with_targets(source, targets)?;
    Ok(StructuredRoutePlaceholderCheck {
        routes,
        auth,
        data,
        render,
        source_bytes,
        token_count,
        unparsed_ranges,
    })
}
