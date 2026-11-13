//! Auth declaration syntax only: untyped conditions never become validated policies.
use crate::compiler::{
    diagnostics::Span,
    lexer::{Lexer, TokenKind},
    project::AuthPolicy,
};

/// Maximum retained auth containers per source.
pub const MAX_AUTH_BLOCKS: usize = 128;
/// Maximum cumulative nontrivia body lines admitted in auth containers.
pub const MAX_AUTH_MEMBERS: usize = 4096;
/// Maximum cumulative action/model items retained in policy headers.
pub const MAX_AUTH_ITEMS: usize = 8192;

/// Policy condition coverage, never an authorization truth value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyConditionSyntax {
    /// No condition was written; this does not establish policy validity.
    Absent,
    /// Original where clause is explicitly outside typed/validated syntax.
    Unparsed {
        /// Source envelope including the where keyword and expression tokens.
        span: Span,
    },
}

/// A recognized allow header with an explicitly separate unvalidated condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthPolicyHeaderSyntax {
    /// Scanner-compatible role/actions/models; not a resolved permission grant.
    pub header: AuthPolicy,
    /// Entire original declaration range, including any condition.
    pub span: Span,
    /// Recognized header range excluding the condition.
    pub header_span: Span,
    /// Explicit condition coverage.
    pub condition: PolicyConditionSyntax,
}

/// Auth container syntax with unsupported fields/nested forms retained as gaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthBlockSyntax {
    /// Source auth declaration name, not a resolved policy identity.
    pub name: String,
    /// Original container envelope including braces.
    pub span: Span,
    /// Ordered policy headers, preserving list order and duplicates.
    pub policies: Vec<AuthPolicyHeaderSyntax>,
    /// Unsupported fields, nested forms and conditions.
    pub unparsed_ranges: Vec<Span>,
}

/// Auth-only partial syntax; no model/type/context or permission checking occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationAuthSyntax {
    /// Original source byte length.
    pub source_bytes: usize,
    /// Retained top-level auth containers.
    pub blocks: Vec<AuthBlockSyntax>,
    /// Tokens outside recognized containers/headers, including every where clause.
    pub unparsed_ranges: Vec<Span>,
}

/// Retain existing auth/policy header grammar without interpreting conditions.
/// Malformed recognized policy headers and unclosed containers reject the request.
/// Unrecognized syntax remains gaps; nothing authorizes access or executes code.
pub fn parse_auth_blocks(source: &str) -> Result<ApplicationAuthSyntax, String> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err("auth syntax exceeds 1 MiB source limit".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "auth syntax lexical errors")?;
    if tokens.len().saturating_sub(1) > super::MAX_CALLABLE_TOKENS {
        return Err("auth syntax token limit exceeded".into());
    }
    super::super::check_nesting(&tokens)?;
    let mut blocks = Vec::new();
    let mut covered = Vec::new();
    let mut position = 0;
    let mut depth = 0usize;
    let mut members = 0usize;
    let mut items = 0usize;
    while position + 2 < tokens.len() {
        let first_on_line =
            position == 0 || tokens[position - 1].span.line != tokens[position].span.line;
        let header = depth == 0
            && first_on_line
            && matches!(&tokens[position].kind, TokenKind::Identifier(name) if name == "auth")
            && matches!(tokens[position + 1].kind, TokenKind::Identifier(_))
            && tokens[position + 2].kind == TokenKind::Punctuation('{');
        if header {
            if blocks.len() == MAX_AUTH_BLOCKS {
                return Err("auth syntax container limit exceeded".into());
            }
            let start = tokens[position].span;
            let TokenKind::Identifier(name) = &tokens[position + 1].kind else {
                unreachable!()
            };
            let name = name.clone();
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
                return Err("unclosed auth syntax container".into());
            }
            covered.push(Span {
                end: tokens[opening].span.end,
                ..start
            });
            covered.push(tokens[end].span);
            position = opening + 1;
            let mut policies = Vec::new();
            let mut unknown = Vec::new();
            let mut nested = 0usize;
            while position < end {
                members += 1;
                if members > MAX_AUTH_MEMBERS {
                    return Err("auth syntax member limit exceeded".into());
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
                let is_policy = nested == 0
                    && matches!(&tokens[begin].kind, TokenKind::Identifier(name) if name == "allow");
                if is_policy {
                    let text = &source[span.start..span.end];
                    let policy = super::super::super::parse_auth_policy(text).ok_or_else(|| {
                        format!("malformed auth policy header at byte {}", span.start)
                    })?;
                    items = items
                        .checked_add(policy.actions.len())
                        .and_then(|sum| sum.checked_add(policy.models.len()))
                        .filter(|sum| *sum <= MAX_AUTH_ITEMS)
                        .ok_or("auth syntax action/model item limit exceeded")?;
                    let (header_span, condition) = if let Some(offset) = text.find(" where ") {
                        let at = span.start + offset + 1;
                        let index = tokens.partition_point(|token| token.span.start < at);
                        if index <= begin || tokens[index].span.start != at {
                            return Err("auth condition token provenance unavailable".into());
                        }
                        let condition_span = Span {
                            end: span.end,
                            ..tokens[index].span
                        };
                        unknown.push(condition_span);
                        (
                            Span {
                                end: tokens[index - 1].span.end,
                                ..span
                            },
                            PolicyConditionSyntax::Unparsed {
                                span: condition_span,
                            },
                        )
                    } else {
                        (span, PolicyConditionSyntax::Absent)
                    };
                    covered.push(header_span);
                    policies.push(AuthPolicyHeaderSyntax {
                        header: policy,
                        span,
                        header_span,
                        condition,
                    });
                } else {
                    unknown.push(span);
                }
                for token in &tokens[begin..position] {
                    match token.kind {
                        TokenKind::Punctuation('{') => nested += 1,
                        TokenKind::Punctuation('}') => nested = nested.saturating_sub(1),
                        _ => {}
                    }
                }
            }
            blocks.push(AuthBlockSyntax {
                name,
                span: Span {
                    end: tokens[end].span.end,
                    ..start
                },
                policies,
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
    Ok(ApplicationAuthSyntax {
        source_bytes: source.len(),
        blocks,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guards_and_unknown_fields_never_become_validated_policy_coverage() {
        let source = "// λ\r\nauth session{\r\nprovider: unknown\r\nallow manager to [read, write] on [Item, Item]\r\nallow user to read on Item where false // retained\r\n}\r\nfn other(){}";
        let parsed = parse_auth_blocks(source).unwrap();
        assert_eq!(parsed.blocks[0].policies.len(), 2);
        assert_eq!(parsed.blocks[0].policies[0].header.models, ["Item", "Item"]);
        assert_eq!(
            parsed.blocks[0].policies[0].condition,
            PolicyConditionSyntax::Absent
        );
        let PolicyConditionSyntax::Unparsed { span } = parsed.blocks[0].policies[1].condition
        else {
            panic!("condition coverage")
        };
        assert_eq!(&source[span.start..span.end], "where false");
        assert!(parsed
            .unparsed_ranges
            .iter()
            .any(|range| range.start <= span.start && span.end <= range.end));
        assert_eq!(parsed.blocks[0].unparsed_ranges.len(), 2);
    }
    #[test]
    fn malformed_headers_and_unknown_nested_content_are_never_promoted() {
        assert!(parse_auth_blocks("auth session{\nallow user to [] on Item\n}").is_err());
        assert!(parse_auth_blocks("auth session{\nallow user to read on Item where\n}").is_err());
        assert!(parse_auth_blocks("auth session{").is_err());
        let parsed = parse_auth_blocks(
            "auth session{\nunknown{\nallow user to read on Item\n}\n}\nfn auth(){}",
        )
        .unwrap();
        assert!(parsed.blocks[0].policies.is_empty());
        assert!(!parsed.unparsed_ranges.is_empty());
    }
    #[test]
    fn auth_metadata_accepts_exact_and_rejects_exceeded_limits() {
        let blocks = "auth session{}\n".repeat(MAX_AUTH_BLOCKS);
        assert_eq!(
            parse_auth_blocks(&blocks).unwrap().blocks.len(),
            MAX_AUTH_BLOCKS
        );
        assert!(parse_auth_blocks(&(blocks + "auth extra{}\n"))
            .unwrap_err()
            .contains("container limit"));
        let policies = "allow user to read on Item\n".repeat(MAX_AUTH_MEMBERS);
        assert_eq!(
            parse_auth_blocks(&format!("auth session{{\n{policies}}}"))
                .unwrap()
                .blocks[0]
                .policies
                .len(),
            MAX_AUTH_MEMBERS
        );
        assert!(
            parse_auth_blocks(&format!("auth session{{\n{policies}unknown: true\n}}"))
                .unwrap_err()
                .contains("member limit")
        );
        let actions = (0..MAX_AUTH_ITEMS - 1)
            .map(|n| format!("a{n}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_auth_blocks(&format!(
            "auth session{{\nallow user to [{actions}] on Item\n}}"
        ))
        .is_ok());
        assert!(parse_auth_blocks(&format!(
            "auth session{{\nallow user to [{actions},extra] on Item\n}}"
        ))
        .unwrap_err()
        .contains("item limit"));
        assert!(
            parse_auth_blocks(&" ".repeat(super::super::MAX_SOURCE_BYTES + 1))
                .unwrap_err()
                .contains("source limit")
        );
        assert!(
            parse_auth_blocks(&"unknown\n".repeat(super::super::MAX_CALLABLE_TOKENS + 1))
                .unwrap_err()
                .contains("token limit")
        );
    }
}
