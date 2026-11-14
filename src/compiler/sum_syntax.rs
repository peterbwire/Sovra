//! Opt-in, bounded single-enum declaration syntax. No executable enum semantics.
use super::{
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    lexer::{LexLimitKind, Lexer, Token, TokenKind},
    limits::MAX_STRUCTURAL_DEPTH,
    resources::FrontendLimits,
};

/// Independent quotas for this additive metadata API; zero is a real limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumSyntaxLimits {
    /// UTF-8 source bytes admitted before scanning.
    pub max_source_bytes: usize,
    /// Nontrivia non-EOF tokens, admitted by the lexer before retention.
    pub max_tokens: usize,
    /// Ordered variants, including duplicate names.
    pub max_variants: usize,
    /// Cumulative payload leaf and array-wrapper nodes.
    pub max_payload_type_nodes: usize,
    /// Owned enum/variant names and canonical payload spelling UTF-8 bytes.
    pub max_retained_text_bytes: usize,
}
impl EnumSyntaxLimits {
    /// Opt-in syntax defaults; ordinary compiler/CLI defaults are unchanged.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 1024 * 1024,
        max_tokens: 100_000,
        max_variants: 4096,
        max_payload_type_nodes: 16_384,
        max_retained_text_bytes: 256 * 1024,
    };
}

/// A quota dimension, separate from source grammar or semantic validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumSyntaxLimit {
    /// Input UTF-8 bytes.
    SourceBytes,
    /// Non-EOF lexical tokens.
    Tokens,
    /// Retained ordered variants.
    Variants,
    /// Cumulative payload type nodes.
    PayloadTypeNodes,
    /// Retained canonical names/type text.
    RetainedTextBytes,
    /// Shared structural depth, including the payload leaf.
    TypeDepth,
    /// Existing lexer diagnostic guard, not caller-selected by this API.
    LexicalDiagnostics,
}

/// Complete refusal, with no partially successful declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumSyntaxError {
    /// Existing lexical/parser codes with original source locations.
    Diagnostics(Diagnostics),
    /// A refused dimension and its selected cap (or shared fixed guard).
    Limit {
        /// Exhausted dimension.
        dimension: EnumSyntaxLimit,
        /// Selected cap; depth includes the leaf node.
        limit: usize,
    },
}
impl std::fmt::Display for EnumSyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Limit { dimension, limit } => {
                write!(f, "enum syntax {dimension:?} limit of {limit} exceeded")
            }
            Self::Diagnostics(errors) => write!(
                f,
                "enum syntax rejected: {}",
                errors
                    .items
                    .first()
                    .map_or("invalid syntax", |error| error.message.as_str())
            ),
        }
    }
}
impl std::error::Error for EnumSyntaxError {}

/// Syntax only: neither export context nor nominal type identity is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDeclarationSyntax {
    /// Whether the declaration starts with export.
    pub is_exported: bool,
    /// Owned enum name.
    pub name: String,
    /// Exact original name token.
    pub name_span: Span,
    /// From export/enum through the closing brace, excluding surrounding trivia.
    pub span: Span,
    /// Ordered variants; duplicates and an empty list remain syntactic data.
    pub variants: Vec<EnumVariantSyntax>,
}

/// One nullary or single-payload variant; payload names remain unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantSyntax {
    /// Owned variant name.
    pub name: String,
    /// Exact name token.
    pub name_span: Span,
    /// Name through payload closing parenthesis, excluding separator/trivia.
    pub span: Span,
    /// Canonical qualified/array annotation spelling, without trivia.
    pub payload_type: Option<String>,
    /// Original annotation token range, excluding payload parentheses.
    pub payload_type_span: Option<Span>,
}

fn limit(dimension: EnumSyntaxLimit, cap: usize) -> EnumSyntaxError {
    EnumSyntaxError::Limit {
        dimension,
        limit: cap,
    }
}
fn admitted(
    current: usize,
    amount: usize,
    cap: usize,
    dimension: EnumSyntaxLimit,
) -> Result<usize, EnumSyntaxError> {
    current
        .checked_add(amount)
        .filter(|total| *total <= cap)
        .ok_or_else(|| limit(dimension, cap))
}
fn joined(first: Span, last: Span) -> Span {
    Span {
        end: last.end,
        ..first
    }
}

/// Parse exactly one declaration under explicit limits; never resolves or executes it.
/// Existing parsers, contextual identifiers and whole-project checks are unchanged.
pub fn parse_enum_declaration(
    source: &str,
    limits: EnumSyntaxLimits,
) -> Result<EnumDeclarationSyntax, EnumSyntaxError> {
    if source.len() > limits.max_source_bytes {
        return Err(limit(EnumSyntaxLimit::SourceBytes, limits.max_source_bytes));
    }
    let lexer_limits = FrontendLimits {
        max_source_bytes: limits.max_source_bytes,
        max_tokens: limits.max_tokens,
        ..FrontendLimits::DEFAULT
    };
    let tokens = Lexer::new()
        .tokenize_with_limit_kind(source, lexer_limits)
        .map_err(|(diagnostics, reason)| match reason {
            Some(LexLimitKind::SourceBytes) => {
                limit(EnumSyntaxLimit::SourceBytes, limits.max_source_bytes)
            }
            Some(LexLimitKind::Tokens) => limit(EnumSyntaxLimit::Tokens, limits.max_tokens),
            Some(LexLimitKind::Diagnostics) => limit(
                EnumSyntaxLimit::LexicalDiagnostics,
                lexer_limits.max_diagnostics.max(1),
            ),
            None => EnumSyntaxError::Diagnostics(diagnostics),
        })?;
    parse_enum_tokens(&tokens, source.len(), limits)
}

// Trusted source-unit integration preserves original token spans without relexing slices.
pub(crate) fn parse_enum_tokens(
    tokens: &[Token],
    source_bytes: usize,
    limits: EnumSyntaxLimits,
) -> Result<EnumDeclarationSyntax, EnumSyntaxError> {
    if source_bytes > limits.max_source_bytes {
        return Err(limit(EnumSyntaxLimit::SourceBytes, limits.max_source_bytes));
    }
    if tokens.len().saturating_sub(1) > limits.max_tokens {
        return Err(limit(EnumSyntaxLimit::Tokens, limits.max_tokens));
    }
    SyntaxParser {
        tokens,
        cursor: 0,
        limits,
        text: 0,
        type_nodes: 0,
    }
    .declaration()
}

struct SyntaxParser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    limits: EnumSyntaxLimits,
    text: usize,
    type_nodes: usize,
}
impl SyntaxParser<'_> {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }
    fn advance(&mut self) -> usize {
        let index = self.cursor;
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
        index
    }
    fn error(&self, code: &'static str, message: &'static str) -> EnumSyntaxError {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            span: self.peek().span,
            source_file: None,
        });
        EnumSyntaxError::Diagnostics(diagnostics)
    }
    fn punctuation(&mut self, value: char, message: &'static str) -> Result<Span, EnumSyntaxError> {
        if self.peek().kind == TokenKind::Punctuation(value) {
            let index = self.advance();
            Ok(self.tokens[index].span)
        } else {
            Err(self.error("E2005", message))
        }
    }
    fn consume(&mut self, value: char) -> bool {
        if self.peek().kind == TokenKind::Punctuation(value) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn identifier(&mut self) -> Result<usize, EnumSyntaxError> {
        if matches!(self.peek().kind, TokenKind::Identifier(_)) {
            Ok(self.advance())
        } else {
            Err(self.error("E2002", "expected identifier"))
        }
    }
    fn own_name(&mut self, index: usize) -> Result<String, EnumSyntaxError> {
        let TokenKind::Identifier(name) = &self.tokens[index].kind else {
            unreachable!("identifier validated")
        };
        self.text = admitted(
            self.text,
            name.len(),
            self.limits.max_retained_text_bytes,
            EnumSyntaxLimit::RetainedTextBytes,
        )?;
        Ok(name.clone())
    }
    fn declaration(&mut self) -> Result<EnumDeclarationSyntax, EnumSyntaxError> {
        let first = self.peek().span;
        let is_exported = self.peek().kind == TokenKind::Keyword("export");
        if is_exported {
            self.advance();
        }
        if !matches!(&self.peek().kind, TokenKind::Identifier(name) if name == "enum") {
            return Err(self.error("E2003", "expected enum declaration"));
        }
        self.advance();
        let name_index = self.identifier()?;
        let name = self.own_name(name_index)?;
        let name_span = self.tokens[name_index].span;
        self.punctuation('{', "expected `{`")?;
        let mut variants = Vec::new();
        while self.peek().kind != TokenKind::Punctuation('}') {
            // Validate the next variant header before admitting its metadata.
            let index = self.identifier()?;
            admitted(
                variants.len(),
                1,
                self.limits.max_variants,
                EnumSyntaxLimit::Variants,
            )?;
            let name = self.own_name(index)?;
            let name_span = self.tokens[index].span;
            let mut span = name_span;
            let (payload_type, payload_type_span) = if self.consume('(') {
                let (spelling, type_span) = self.payload_type()?;
                span = joined(name_span, self.punctuation(')', "expected `)`")?);
                (Some(spelling), Some(type_span))
            } else {
                (None, None)
            };
            variants.push(EnumVariantSyntax {
                name,
                name_span,
                span,
                payload_type,
                payload_type_span,
            });
            if !self.consume(',') {
                break;
            }
        }
        let end = self.punctuation('}', "expected `}` or comma-separated variant")?;
        if self.peek().kind != TokenKind::Eof {
            return Err(self.error("E2001", "expected end of declaration"));
        }
        Ok(EnumDeclarationSyntax {
            is_exported,
            name,
            name_span,
            span: joined(first, end),
            variants,
        })
    }
    fn payload_type(&mut self) -> Result<(String, Span), EnumSyntaxError> {
        let first = self.peek().span;
        let mut arrays = 0;
        while self.consume('[') {
            arrays += 1;
            if arrays >= MAX_STRUCTURAL_DEPTH {
                return Err(limit(EnumSyntaxLimit::TypeDepth, MAX_STRUCTURAL_DEPTH));
            }
        }
        let leaf = self.identifier()?;
        let mut bytes = arrays * 2;
        let mut end = self.tokens[leaf].span;
        let mut path_end = leaf;
        loop {
            let TokenKind::Identifier(name) = &self.tokens[path_end].kind else {
                unreachable!("identifier validated")
            };
            bytes = admitted(
                bytes,
                name.len(),
                usize::MAX,
                EnumSyntaxLimit::RetainedTextBytes,
            )
            .map_err(|_| {
                limit(
                    EnumSyntaxLimit::RetainedTextBytes,
                    self.limits.max_retained_text_bytes,
                )
            })?;
            if self.peek().kind != TokenKind::Operator("::") {
                break;
            }
            self.advance();
            bytes = admitted(bytes, 2, usize::MAX, EnumSyntaxLimit::RetainedTextBytes).map_err(
                |_| {
                    limit(
                        EnumSyntaxLimit::RetainedTextBytes,
                        self.limits.max_retained_text_bytes,
                    )
                },
            )?;
            path_end = self.identifier()?;
            end = self.tokens[path_end].span;
        }
        for _ in 0..arrays {
            end = self.punctuation(']', "expected `]`")?;
        }
        let nodes = admitted(
            self.type_nodes,
            arrays + 1,
            self.limits.max_payload_type_nodes,
            EnumSyntaxLimit::PayloadTypeNodes,
        )?;
        let text = admitted(
            self.text,
            bytes,
            self.limits.max_retained_text_bytes,
            EnumSyntaxLimit::RetainedTextBytes,
        )?;
        self.type_nodes = nodes;
        self.text = text;
        let mut spelling = String::with_capacity(bytes);
        for _ in 0..arrays {
            spelling.push('[');
        }
        for token in &self.tokens[leaf..=path_end] {
            match &token.kind {
                TokenKind::Identifier(name) => spelling.push_str(name),
                TokenKind::Operator("::") => spelling.push_str("::"),
                _ => unreachable!("qualified type path validated"),
            }
        }
        for _ in 0..arrays {
            spelling.push(']');
        }
        Ok((spelling, joined(first, end)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_admission_never_wraps_or_turns_zero_into_unlimited() {
        assert_eq!(admitted(0, 0, 0, EnumSyntaxLimit::PayloadTypeNodes), Ok(0));
        assert_eq!(
            admitted(
                usize::MAX,
                0,
                usize::MAX,
                EnumSyntaxLimit::RetainedTextBytes
            ),
            Ok(usize::MAX)
        );
        assert_eq!(
            admitted(
                usize::MAX,
                1,
                usize::MAX,
                EnumSyntaxLimit::RetainedTextBytes
            ),
            Err(limit(EnumSyntaxLimit::RetainedTextBytes, usize::MAX))
        );
        assert_eq!(
            admitted(0, 1, 0, EnumSyntaxLimit::Variants),
            Err(limit(EnumSyntaxLimit::Variants, 0))
        );
    }
}
