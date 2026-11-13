//! Model/enum declaration envelopes; bodies never become typed declarations here.
use crate::compiler::{
    diagnostics::Span,
    lexer::{Lexer, TokenKind},
};

/// Maximum retained model/enum declaration headers per source.
pub const MAX_DATA_DECLARATIONS: usize = 4096;

/// Source declaration category, not a resolved nominal type or runtime API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDeclarationKind {
    /// An explicit model declaration.
    Model,
    /// An explicit enum declaration, without representation/variant semantics.
    Enum,
}

/// Named braced declaration with an explicitly unparsed body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDeclarationSyntax {
    /// Model or enum source category.
    pub kind: DataDeclarationKind,
    /// Original source name.
    pub name: String,
    /// Exact declaration-name token span.
    pub name_span: Span,
    /// Original declaration envelope, including braces.
    pub span: Span,
    /// Nontrivia body token envelope, absent only for an empty body.
    pub unparsed_body: Option<Span>,
}

/// Data-header-only source coverage, never type/field/model capability validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationDataSyntax {
    /// Original UTF-8 byte length.
    pub source_bytes: usize,
    /// Ordered source model/enum headers.
    pub declarations: Vec<DataDeclarationSyntax>,
    /// All bodies and other unparsed source tokens.
    pub unparsed_ranges: Vec<Span>,
}

/// Retain existing named model/enum envelopes without choosing body grammar.
/// Unrecognized headers stay gaps; unclosed recognized containers reject the request.
/// No fields, variants, generic/optional syntax or model operations are introduced.
pub fn parse_data_declarations(source: &str) -> Result<ApplicationDataSyntax, String> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err("data syntax exceeds 1 MiB source limit".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "data syntax lexical errors")?;
    if tokens.len().saturating_sub(1) > super::MAX_CALLABLE_TOKENS {
        return Err("data syntax token limit exceeded".into());
    }
    super::super::check_nesting(&tokens)?;
    let mut declarations = Vec::new();
    let mut covered = Vec::new();
    let mut position = 0;
    let mut depth = 0usize;
    while position + 2 < tokens.len() {
        let first = position == 0 || tokens[position - 1].span.line != tokens[position].span.line;
        let category = match &tokens[position].kind {
            TokenKind::Identifier(name) if name == "model" => Some(DataDeclarationKind::Model),
            TokenKind::Identifier(name) if name == "enum" => Some(DataDeclarationKind::Enum),
            _ => None,
        };
        if depth == 0
            && first
            && category.is_some()
            && matches!(tokens[position + 1].kind, TokenKind::Identifier(_))
            && tokens[position + 2].kind == TokenKind::Punctuation('{')
        {
            if declarations.len() == MAX_DATA_DECLARATIONS {
                return Err("data syntax declaration limit exceeded".into());
            }
            let start = tokens[position].span;
            let TokenKind::Identifier(name) = &tokens[position + 1].kind else {
                unreachable!()
            };
            let name = name.clone();
            let name_span = tokens[position + 1].span;
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
                return Err("unclosed data syntax declaration".into());
            }
            covered.push(Span {
                end: tokens[opening].span.end,
                ..start
            });
            covered.push(tokens[end].span);
            let unparsed_body = (opening + 1 < end).then(|| Span {
                end: tokens[end - 1].span.end,
                ..tokens[opening + 1].span
            });
            declarations.push(DataDeclarationSyntax {
                kind: category.ok_or("missing data category")?,
                name,
                name_span,
                span: Span {
                    end: tokens[end].span.end,
                    ..start
                },
                unparsed_body,
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
    Ok(ApplicationDataSyntax {
        source_bytes: source.len(),
        declarations,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fields_and_variants_remain_explicitly_unparsed_and_nested_names_are_not_promoted() {
        let source = "// λ\r\nmodel Item{\r\nfield: Unknown\r\n}\r\nenum Role{user admin}\r\nunknown{\nmodel Hidden{}\n}";
        let parsed = parse_data_declarations(source).unwrap();
        assert_eq!(parsed.declarations.len(), 2);
        assert_eq!(parsed.declarations[1].kind, DataDeclarationKind::Enum);
        for declaration in &parsed.declarations {
            let body = declaration.unparsed_body.unwrap();
            assert!(parsed
                .unparsed_ranges
                .iter()
                .any(|gap| gap.start <= body.start && body.end <= gap.end));
            assert_eq!(
                &source[declaration.name_span.start..declaration.name_span.end],
                declaration.name
            );
        }
        assert!(parse_data_declarations("model Broken{").is_err());
        assert!(parse_data_declarations("model Empty{}")
            .unwrap()
            .declarations[0]
            .unparsed_body
            .is_none());
    }
    #[test]
    fn data_syntax_limits_are_exact_and_unknown_forms_never_become_types() {
        let exact = "model Item{}\n".repeat(MAX_DATA_DECLARATIONS);
        assert_eq!(
            parse_data_declarations(&exact).unwrap().declarations.len(),
            MAX_DATA_DECLARATIONS
        );
        assert!(parse_data_declarations(&(exact + "enum Extra{}\n"))
            .unwrap_err()
            .contains("declaration limit"));
        assert!(parse_data_declarations("model Generic<T>{}")
            .unwrap()
            .declarations
            .is_empty());
        assert!(parse_data_declarations(&" ".repeat(super::super::MAX_SOURCE_BYTES + 1)).is_err());
        assert!(parse_data_declarations(
            &"unknown\n".repeat(super::super::MAX_CALLABLE_TOKENS + 1)
        )
        .unwrap_err()
        .contains("token limit"));
    }
}
