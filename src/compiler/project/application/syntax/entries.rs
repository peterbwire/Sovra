//! Structured entry-block syntax using the existing scanner's member grammar.
use crate::compiler::{
    diagnostics::Span,
    lexer::{Lexer, TokenKind},
    project::{AppPage, AppRoute, AppTask},
};

/// Maximum retained application entry containers per source.
pub const MAX_APP_BLOCKS: usize = 128;
/// Maximum cumulative entry member/unknown-line records per source.
pub const MAX_APP_MEMBERS: usize = 4096;

/// Existing entry binding syntax; no target/type validation is implied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMemberKind {
    /// API route declaration with its original method/path/target spelling.
    Route(AppRoute),
    /// Page route declaration.
    Page(AppPage),
    /// Scheduled task binding, not task execution.
    ScheduledTask(AppTask),
    /// Explicit source service names, not authorized service bindings.
    Services(Vec<String>),
    /// Explicit source model names, not generated model capabilities.
    Data(Vec<String>),
    /// Explicit auth target spelling, not a resolved policy.
    Auth(String),
}

/// One recognized entry member with its original UTF-8 byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMemberSyntax {
    /// Parsed member syntax only.
    pub kind: AppMemberKind,
    /// Source range excluding trailing comments/whitespace.
    pub span: Span,
}

/// Entry container syntax with explicit unsupported member ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntrySyntax {
    /// Declared application name.
    pub name: String,
    /// Source envelope including container braces.
    pub span: Span,
    /// Recognized members in source order, preserving duplicates.
    pub members: Vec<AppMemberSyntax>,
    /// Unknown member/nested syntax, never asserted as checked.
    pub unparsed_ranges: Vec<Span>,
}

/// Partial entry-only syntax; other top-level declarations remain coverage gaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntrySyntax {
    /// Original source byte length.
    pub source_bytes: usize,
    /// Recognized entry containers in source order.
    pub entries: Vec<AppEntrySyntax>,
    /// Tokens outside recognized headers/braces/members, including unknown fields.
    pub unparsed_ranges: Vec<Span>,
}

/// Parse existing line-oriented entry members without resolving or executing them.
/// Unknown members remain explicit gaps; malformed recognized forms reject the
/// whole request. No default scanner or other syntax API behavior is changed.
pub fn parse_entry_blocks(source: &str) -> Result<ApplicationEntrySyntax, String> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err("application entry syntax exceeds 1 MiB source limit".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "application entry syntax lexical errors")?;
    if tokens.len().saturating_sub(1) > super::MAX_CALLABLE_TOKENS {
        return Err("application entry syntax token limit exceeded".into());
    }
    super::super::check_nesting(&tokens)?;
    let mut entries = Vec::new();
    let mut covered = Vec::new();
    let mut position = 0;
    let mut depth = 0usize;
    let mut member_count = 0;
    while position + 1 < tokens.len() {
        let first_on_line =
            position == 0 || tokens[position - 1].span.line != tokens[position].span.line;
        if depth == 0
            && first_on_line
            && matches!(&tokens[position].kind, TokenKind::Identifier(name) if name == "app")
        {
            if entries.len() == MAX_APP_BLOCKS {
                return Err("application entry container limit exceeded".into());
            }
            let start = tokens[position].span;
            let Some(TokenKind::Identifier(name)) =
                tokens.get(position + 1).map(|token| &token.kind)
            else {
                return Err("expected application name".into());
            };
            let name = name.clone();
            if tokens.get(position + 2).map(|token| &token.kind)
                != Some(&TokenKind::Punctuation('{'))
            {
                return Err("expected application opening brace".into());
            }
            let opening = position + 2;
            let mut end = opening + 1;
            let mut nesting = 1usize;
            while end < tokens.len() && tokens[end].kind != TokenKind::Eof {
                match tokens[end].kind {
                    TokenKind::Punctuation('{') => nesting += 1,
                    TokenKind::Punctuation('}') => {
                        nesting -= 1;
                        if nesting == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                end += 1;
            }
            if nesting != 0 {
                return Err("unclosed application entry block".into());
            }
            covered.push(Span {
                end: tokens[opening].span.end,
                ..start
            });
            covered.push(tokens[end].span);
            position = opening + 1;
            let mut members = Vec::new();
            let mut unknown = Vec::new();
            let mut member_depth = 0usize;
            while position < end {
                member_count += 1;
                if member_count > MAX_APP_MEMBERS {
                    return Err("application entry member limit exceeded".into());
                }
                let begin = position;
                let line = tokens[position].span.line;
                while position < end && tokens[position].span.line == line {
                    position += 1;
                }
                let span = Span {
                    end: tokens[position - 1].span.end,
                    ..tokens[begin].span
                };
                let text = &source[span.start..span.end];
                let key = match &tokens[begin].kind {
                    TokenKind::Identifier(name) => name.as_str(),
                    TokenKind::Keyword(name) => name,
                    _ => "",
                };
                let parsed = if member_depth == 0 {
                    match key {
                        "route" => Some(
                            super::super::super::parse_app_route(text).map(AppMemberKind::Route),
                        ),
                        "page" => {
                            Some(super::super::super::parse_app_page(text).map(AppMemberKind::Page))
                        }
                        "task" => Some(
                            super::super::super::parse_app_task(text)
                                .map(AppMemberKind::ScheduledTask),
                        ),
                        "services" => Some(
                            super::super::super::parse_named_list(text, "services")
                                .map(AppMemberKind::Services),
                        ),
                        "data" => Some(
                            super::super::super::parse_named_list(text, "data")
                                .map(AppMemberKind::Data),
                        ),
                        "auth" => Some(
                            super::super::super::parse_named_target(text, "auth")
                                .map(AppMemberKind::Auth),
                        ),
                        _ => None,
                    }
                } else {
                    None
                };
                match parsed {
                    Some(Some(kind)) => {
                        covered.push(span);
                        members.push(AppMemberSyntax { kind, span });
                    }
                    Some(None) => {
                        return Err(format!(
                            "malformed application entry member `{key}` at byte {}",
                            span.start
                        ))
                    }
                    None => unknown.push(span),
                }
                for token in &tokens[begin..position] {
                    match token.kind {
                        TokenKind::Punctuation('{') => member_depth += 1,
                        TokenKind::Punctuation('}') => {
                            member_depth = member_depth.saturating_sub(1)
                        }
                        _ => {}
                    }
                }
            }
            entries.push(AppEntrySyntax {
                name,
                span: Span {
                    end: tokens[end].span.end,
                    ..start
                },
                members,
                unparsed_ranges: unknown,
            });
            position = end + 1;
            continue;
        }
        match tokens[position].kind {
            TokenKind::Punctuation('{') => depth += 1,
            TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
            _ => {}
        }
        position += 1;
    }
    covered.sort_by_key(|span| span.start);
    let all = [Span {
        start: 0,
        end: source.len(),
        line: 0,
        column: 0,
    }];
    let unparsed_ranges = super::callables::unparsed(&tokens, &all, &covered);
    Ok(ApplicationEntrySyntax {
        source_bytes: source.len(),
        entries,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_existing_bindings_and_unknown_fields_with_original_ranges() {
        let source = "// λ\r\nuse app.jobs\r\napp Demo {\r\n title: \"future field\"\r\n services: [api, api,]\r\n data: [Item]\r\n auth: auth.session\r\n route GET \"/items/:id\" -> jobs.item // note\r\n page \"/\" -> pages.home\r\n task every 5 minutes -> jobs.refresh\r\n}\r\nfn main(){}";
        let parsed = parse_entry_blocks(source).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].members.len(), 6);
        assert_eq!(parsed.entries[0].unparsed_ranges.len(), 1);
        let span = parsed.entries[0].unparsed_ranges[0];
        assert_eq!(&source[span.start..span.end], "title: \"future field\"");
        let AppMemberKind::Services(names) = &parsed.entries[0].members[0].kind else {
            panic!("services")
        };
        assert_eq!(names, &["api", "api"]);
        let span = parsed.entries[0].members[3].span;
        assert_eq!(
            &source[span.start..span.end],
            "route GET \"/items/:id\" -> jobs.item"
        );
        assert_eq!(parsed.unparsed_ranges.len(), 3);
    }
    #[test]
    fn malformed_known_forms_fail_and_nested_unknown_forms_never_become_members() {
        for source in [
            "app Demo {\nroute GET \"/\"\n}",
            "app Demo{\nservices: [a,,b]\n}",
            "app Demo",
            "app Demo{",
            "app Demo{\nauth: missing target\n}",
        ] {
            assert!(parse_entry_blocks(source).is_err(), "{source}");
        }
        let parsed = parse_entry_blocks(
            "app Demo{\nunknown {\nroute GET \"/\" -> missing\n}\n}\nfn app(){}\n",
        )
        .unwrap();
        assert!(parsed.entries[0].members.is_empty());
        assert_eq!(parsed.entries[0].unparsed_ranges.len(), 3);
        let parsed =
            parse_entry_blocks("app Demo{\nroute BAD \"relative\" -> missing\n}\n").unwrap();
        assert_eq!(parsed.entries[0].members.len(), 1); // Syntax, not route validation.
    }
    #[test]
    fn entry_metadata_limits_accept_exact_boundaries() {
        let entries = "app Demo{}\n".repeat(MAX_APP_BLOCKS);
        assert_eq!(
            parse_entry_blocks(&entries).unwrap().entries.len(),
            MAX_APP_BLOCKS
        );
        assert!(parse_entry_blocks(&(entries + "app Extra{}\n"))
            .unwrap_err()
            .contains("container limit"));
        let members = "services: []\n".repeat(MAX_APP_MEMBERS);
        assert_eq!(
            parse_entry_blocks(&format!("app Demo{{\n{members}}}"))
                .unwrap()
                .entries[0]
                .members
                .len(),
            MAX_APP_MEMBERS
        );
        assert!(
            parse_entry_blocks(&format!("app Demo{{\n{members}data: []\n}}"))
                .unwrap_err()
                .contains("member limit")
        );
        assert!(parse_entry_blocks(&" ".repeat(super::super::MAX_SOURCE_BYTES + 1)).is_err());
        assert!(
            parse_entry_blocks(&"unknown\n".repeat(super::super::MAX_CALLABLE_TOKENS + 1))
                .unwrap_err()
                .contains("token limit")
        );
    }
}
