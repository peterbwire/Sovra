//! Bounded original-source match syntax with ordinary, untyped arm bodies.
use super::super::{
    ast::Statement,
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    lexer::{Lexer, Token, TokenKind},
    parser::Parser,
    resources::FrontendLimits,
};

/// Admission limits for one standalone match statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchSyntaxLimits {
    /// Source/token/diagnostic admission, before parsing.
    pub frontend: FrontendLimits,
    /// Number of retained arm bodies, checked before parsing each body.
    pub max_arms: usize,
    /// Cumulative UTF-8 bytes in scrutinee, enum/variant and binding names.
    /// Body storage is bounded by frontend source/token limits, not this counter.
    pub max_header_text_bytes: usize,
}
impl MatchSyntaxLimits {
    /// Opt-in profile; existing CLI syntax remains unchanged.
    pub const DEFAULT: Self = Self {
        frontend: FrontendLimits::DEFAULT,
        max_arms: 4096,
        max_header_text_bytes: 256 * 1024,
    };
}
/// A parsed statement; no name resolution, body typing or execution proof.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchStatementSyntax {
    /// Bound local to evaluate once when executable lowering is integrated.
    pub scrutinee: String,
    /// Original scrutinee location.
    pub scrutinee_span: Span,
    /// Ordered arms; coverage is a separate semantic operation.
    pub arms: Vec<MatchArmSyntax>,
    /// Complete original statement range.
    pub span: Span,
}
/// One nullary or single-binding arm and its ordinary core body.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArmSyntax {
    /// Qualified enum name, excluding the variant.
    pub enum_name: String,
    /// Variant name.
    pub variant_name: String,
    /// Optional immutable payload binding; mutability is checked in future body typing.
    pub binding: Option<String>,
    /// Original binding location, if present.
    pub binding_span: Option<Span>,
    /// Parsed core statements; no semantic validation. Nested matches are unavailable.
    pub body: Vec<Statement>,
    /// Original arm range, excluding its separating comma.
    pub span: Span,
}

/// Parse exactly one match statement, retaining original spans and no partial success.
/// Contextual `match` and ordinary bodies reuse the existing lexer/parser. This
/// API does not enable matches in v1 functions or establish executable enum support.
pub fn parse_match_statement(
    source: &str,
    limits: MatchSyntaxLimits,
) -> Result<MatchStatementSyntax, Diagnostics> {
    let tokens = Lexer::new().tokenize_with_limits(source, limits.frontend)?;
    if let Err(span) = super::super::limits::check_nesting(&tokens, true) {
        return Err(error("E2007", super::super::limits::depth_message(), span));
    }
    MatchParser {
        tokens: &tokens,
        cursor: 0,
        limits,
        text: 0,
    }
    .parse()
}
// Tokens are a suffix of an already admitted/nesting-checked lexer stream.
pub(super) fn parse_match_prefix(
    tokens: &[Token],
    limits: MatchSyntaxLimits,
) -> Result<(MatchStatementSyntax, usize, usize), Diagnostics> {
    let mut parser = MatchParser {
        tokens,
        cursor: 0,
        limits,
        text: 0,
    };
    let syntax = parser.parse_prefix()?;
    Ok((syntax, parser.cursor, parser.text))
}
fn error(code: &'static str, message: impl Into<String>, span: Span) -> Diagnostics {
    Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
        }],
    }
}
struct MatchParser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    limits: MatchSyntaxLimits,
    text: usize,
}
impl MatchParser<'_> {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }
    fn fail(&self, message: &'static str) -> Diagnostics {
        error("E2010", message, self.peek().span)
    }
    fn charge(&mut self, bytes: usize) -> Result<(), Diagnostics> {
        self.text = self
            .text
            .checked_add(bytes)
            .filter(|n| *n <= self.limits.max_header_text_bytes)
            .ok_or_else(|| {
                error(
                    "E1003",
                    "match header text limit exceeded",
                    self.peek().span,
                )
            })?;
        Ok(())
    }
    fn name(&mut self) -> Result<(String, Span), Diagnostics> {
        let TokenKind::Identifier(name) = &self.peek().kind else {
            return Err(self.fail("expected identifier"));
        };
        let span = self.peek().span;
        self.charge(name.len())?;
        let TokenKind::Identifier(name) = &self.peek().kind else {
            unreachable!()
        };
        let name = name.clone();
        self.cursor += 1;
        Ok((name, span))
    }
    fn punctuation(&mut self, ch: char) -> Result<(), Diagnostics> {
        if self.peek().kind != TokenKind::Punctuation(ch) {
            return Err(self.fail("unexpected match punctuation"));
        }
        self.cursor += 1;
        Ok(())
    }
    fn operator(&mut self, op: &'static str) -> Result<(), Diagnostics> {
        if self.peek().kind != TokenKind::Operator(op) {
            return Err(self.fail("expected match pattern operator"));
        }
        self.cursor += 1;
        Ok(())
    }
    fn parse(mut self) -> Result<MatchStatementSyntax, Diagnostics> {
        let syntax = self.parse_prefix()?;
        if self.peek().kind != TokenKind::Eof {
            return Err(self.fail("unexpected input after match statement"));
        }
        Ok(syntax)
    }
    fn parse_prefix(&mut self) -> Result<MatchStatementSyntax, Diagnostics> {
        let start = self.peek().span;
        if !matches!(&self.peek().kind, TokenKind::Identifier(name) if name == "match") {
            return Err(self.fail("expected match statement"));
        }
        self.cursor += 1;
        let (scrutinee, scrutinee_span) = self.name()?;
        self.punctuation('{')?;
        let mut arms = Vec::new();
        while self.peek().kind != TokenKind::Punctuation('}') {
            if arms.len() >= self.limits.max_arms {
                return Err(error("E1003", "match arm limit exceeded", self.peek().span));
            }
            let (mut enum_name, first) = self.name()?;
            self.operator("::")?;
            let (mut variant_name, _) = self.name()?;
            while self.peek().kind == TokenKind::Operator("::") {
                self.charge(2)?;
                enum_name.push_str("::");
                enum_name.push_str(&variant_name);
                self.cursor += 1;
                variant_name = self.name()?.0;
            }
            self.punctuation('(')?;
            let (binding, binding_span) = if self.peek().kind == TokenKind::Punctuation(')') {
                (None, None)
            } else {
                let (name, span) = self.name()?;
                if name == "_" {
                    return Err(error("E2010", "wildcard bindings are unavailable", span));
                }
                (Some(name), Some(span))
            };
            self.punctuation(')')?;
            self.operator("=>")?;
            let (body, consumed) = Parser::new().parse_block_prefix(
                &self.tokens[self.cursor..],
                self.limits.frontend.max_diagnostics,
            )?;
            self.cursor += consumed;
            let span = Span {
                end: self.tokens[self.cursor - 1].span.end,
                ..first
            };
            arms.push(MatchArmSyntax {
                enum_name,
                variant_name,
                binding,
                binding_span,
                body,
                span,
            });
            if self.peek().kind == TokenKind::Punctuation('}') {
                break;
            }
            self.punctuation(',')?;
        }
        let end = self.peek().span.end;
        self.punctuation('}')?;
        Ok(MatchStatementSyntax {
            scrutinee,
            scrutinee_span,
            arms,
            span: Span { end, ..start },
        })
    }
}
