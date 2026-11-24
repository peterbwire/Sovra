//! Page/view header syntax without choosing render results or UI body semantics.
use crate::compiler::{
    diagnostics::Span,
    lexer::{Lexer, TokenKind},
};

/// Maximum retained page/view declarations per source.
pub const MAX_RENDER_DECLARATIONS: usize = 4096;
/// Maximum cumulative page/view parameters per source.
pub const MAX_RENDER_PARAMETERS: usize = 8192;

/// Source render declaration category, not an executable function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDeclarationKind {
    /// A page declaration.
    Page,
    /// A view declaration.
    View,
}

/// An explicitly annotated but unresolved render parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderParameterSyntax {
    /// Source parameter name.
    pub name: String,
    /// Unresolved annotation text after structural comment/line normalization.
    pub annotation: String,
}

/// A page/view header with a deliberately unparsed render body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDeclarationSyntax {
    /// Page or view source category.
    pub kind: RenderDeclarationKind,
    /// Original declaration name.
    pub name: String,
    /// Exact name token range.
    pub name_span: Span,
    /// Full original declaration envelope.
    pub span: Span,
    /// Original header, excluding the body opening brace.
    pub header_span: Span,
    /// Explicit parameters, without inferred types or implicit context objects.
    pub parameters: Vec<RenderParameterSyntax>,
    /// Explicit unresolved return text only. None never means default Unit.
    pub explicit_return_annotation: Option<String>,
    /// Nontrivia body tokens, absent only for an empty source body.
    pub unparsed_body: Option<Span>,
}

/// Render-header-only coverage, never UI/body/type/render-result checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationRenderSyntax {
    /// Original UTF-8 byte length.
    pub source_bytes: usize,
    /// Ordered top-level page/view headers.
    pub declarations: Vec<RenderDeclarationSyntax>,
    /// All nonempty render bodies and other unsupported source tokens.
    pub unparsed_ranges: Vec<Span>,
}

/// Retain existing page/view signature structure with explicit parameter annotations.
/// Reuses the scanner signature grammar, without assigning Unit/render result types,
/// introducing generic semantics, executing UI bodies or validating components.
pub fn parse_render_declarations(source: &str) -> Result<ApplicationRenderSyntax, String> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err("render syntax exceeds 1 MiB source limit".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "render syntax lexical errors")?;
    if tokens.len().saturating_sub(1) > super::MAX_CALLABLE_TOKENS {
        return Err("render syntax token limit exceeded".into());
    }
    super::super::check_nesting(&tokens)?;
    let mut declarations = Vec::new();
    let mut covered = Vec::new();
    let mut position = 0;
    let mut depth = 0usize;
    let mut parameter_count = 0usize;
    while position + 2 < tokens.len() {
        let first = position == 0 || tokens[position - 1].span.line != tokens[position].span.line;
        let category = match &tokens[position].kind {
            TokenKind::Identifier(name) if name == "page" => Some(RenderDeclarationKind::Page),
            TokenKind::Identifier(name) if name == "view" => Some(RenderDeclarationKind::View),
            _ => None,
        };
        if depth == 0
            && first
            && category.is_some()
            && matches!(tokens[position + 1].kind, TokenKind::Identifier(_))
            && tokens[position + 2].kind == TokenKind::Punctuation('(')
        {
            if declarations.len() == MAX_RENDER_DECLARATIONS {
                return Err("render syntax declaration limit exceeded".into());
            }
            let start = tokens[position].span;
            let name_span = tokens[position + 1].span;
            let keyword_end = start.end;
            let mut at = position + 3;
            let mut parentheses = 1usize;
            while at < tokens.len() && tokens[at].kind != TokenKind::Eof {
                match tokens[at].kind {
                    TokenKind::Punctuation('(') => parentheses += 1,
                    TokenKind::Punctuation(')') => {
                        parentheses -= 1;
                        if parentheses == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                at += 1;
            }
            if parentheses != 0 {
                return Err("unclosed render parameter list".into());
            }
            at += 1;
            while at < tokens.len()
                && !matches!(
                    tokens[at].kind,
                    TokenKind::Punctuation('{' | '}' | ';') | TokenKind::Eof
                )
            {
                // Render/function-type result grammar is not selected here. A
                // following declaration must never supply this header's body.
                if matches!(
                    tokens[at].kind,
                    TokenKind::Keyword(
                        "fn" | "export" | "struct" | "type" | "task" | "use" | "let" | "return"
                    )
                ) {
                    return Err("render header interrupted by another declaration".into());
                }
                let first = at == 0 || tokens[at - 1].span.line != tokens[at].span.line;
                if first
                    && matches!(&tokens[at].kind, TokenKind::Identifier(name) if matches!(name.as_str(), "page" | "view" | "service" | "app" | "auth" | "model" | "enum"))
                    && tokens
                        .get(at + 1)
                        .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
                {
                    return Err("render header interrupted by another declaration".into());
                }
                at += 1;
            }
            if tokens.get(at).map(|token| &token.kind) != Some(&TokenKind::Punctuation('{')) {
                return Err("render declaration body unavailable".into());
            }
            let opening = at;
            let raw = source[keyword_end..tokens[opening].span.start]
                .lines()
                .map(|line| super::super::super::strip_line_comment(line, "//"))
                .collect::<Vec<_>>()
                .join("\n");
            let adapter = format!("fn {raw}");
            let operation = super::super::super::service_contract::parse_operation(&adapter)
                .map_err(str::to_owned)?
                .ok_or("missing render signature")?;
            let mut parameters = Vec::new();
            for parameter in &operation.declarations {
                let annotation = parameter
                    .annotation
                    .ok_or("render parameter requires explicit annotation")?;
                parameter_count += 1;
                if parameter_count > MAX_RENDER_PARAMETERS {
                    return Err("render syntax parameter limit exceeded".into());
                }
                parameters.push(RenderParameterSyntax {
                    name: parameter.name.to_owned(),
                    annotation: annotation.to_owned(),
                });
            }
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
                return Err("unclosed render declaration body".into());
            }
            let header_span = Span {
                end: tokens[opening - 1].span.end,
                ..start
            };
            covered.push(Span {
                end: tokens[opening].span.end,
                ..start
            });
            covered.push(tokens[end].span);
            let unparsed_body = (opening + 1 < end).then(|| Span {
                end: tokens[end - 1].span.end,
                ..tokens[opening + 1].span
            });
            declarations.push(RenderDeclarationSyntax {
                kind: category.ok_or("missing render category")?,
                name: operation.name.to_owned(),
                name_span,
                span: Span {
                    end: tokens[end].span.end,
                    ..start
                },
                header_span,
                parameters,
                explicit_return_annotation: operation.return_annotation.map(str::to_owned),
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
    Ok(ApplicationRenderSyntax {
        source_bytes: source.len(),
        declarations,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headers_keep_unresolved_annotations_and_never_assign_unit_or_parse_ui_bodies() {
        let source = "// λ\r\npage Home(user: Unknown){\r\nview Nested(user)\r\n}\r\nview Card(id: Id<Item>)->Render{}";
        let parsed = parse_render_declarations(source).unwrap();
        assert_eq!(parsed.declarations.len(), 2);
        assert_eq!(parsed.declarations[0].kind, RenderDeclarationKind::Page);
        assert_eq!(parsed.declarations[0].explicit_return_annotation, None);
        assert!(parsed.declarations[0].unparsed_body.is_some());
        assert_eq!(parsed.declarations[1].parameters[0].annotation, "Id<Item>");
        assert_eq!(
            parsed.declarations[1].explicit_return_annotation.as_deref(),
            Some("Render")
        );
        assert_eq!(
            &source[parsed.declarations[0].name_span.start..parsed.declarations[0].name_span.end],
            "Home"
        );
        for source in [
            "page Home(user){}",
            "view Card(id:Int,id:Int){}",
            "page Home(id:Int){",
            "page Home(id:Int);",
            "page Home(\nview Other(){}",
            "page Home()->Unknown\nfn following(){}",
            "page Home()->Unknown\nview Following(){}",
        ] {
            assert!(parse_render_declarations(source).is_err(), "{source}");
        }
    }
    #[test]
    fn render_declaration_and_parameter_limits_are_exact() {
        let exact = "page Home(){}\n".repeat(MAX_RENDER_DECLARATIONS);
        assert_eq!(
            parse_render_declarations(&exact)
                .unwrap()
                .declarations
                .len(),
            MAX_RENDER_DECLARATIONS
        );
        assert!(parse_render_declarations(&(exact + "view Extra(){}\n"))
            .unwrap_err()
            .contains("declaration limit"));
        let parameters = (0..MAX_RENDER_PARAMETERS)
            .map(|n| format!("p{n}:Int"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            parse_render_declarations(&format!("page Home({parameters}){{}}"))
                .unwrap()
                .declarations[0]
                .parameters
                .len(),
            MAX_RENDER_PARAMETERS
        );
        assert!(
            parse_render_declarations(&format!("page Home({parameters},extra:Int){{}}"))
                .unwrap_err()
                .contains("parameter limit")
        );
        assert!(
            parse_render_declarations(&" ".repeat(super::super::MAX_SOURCE_BYTES + 1)).is_err()
        );
        assert!(parse_render_declarations(
            &"unknown\n".repeat(super::super::MAX_CALLABLE_TOKENS + 1)
        )
        .is_err());
    }
}
