//! Bounded E2 type-expression grammar; syntax is never checked type evidence.
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, Token, TokenKind},
    resources::FrontendLimits,
};
use std::sync::Arc;
/// Finite syntax admission, independent of eventual specialization quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericTypeLimits {
    /// Complete input bytes, including comments and whitespace.
    pub max_source_bytes: usize,
    /// Lexer token ceiling, including EOF.
    pub max_tokens: usize,
    /// Retained named/application/array nodes.
    pub max_nodes: usize,
    /// Cumulative generic-argument and array-element edges.
    pub max_edges: usize,
    /// Copied qualified-name bytes, including separators.
    pub max_name_bytes: usize,
    /// Nested type nodes; values above the fixed ceiling of 128 refuse.
    pub max_depth: usize,
}
impl GenericTypeLimits {
    /// Small opt-in annotation profile; no existing command profile changes.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 64 * 1024,
        max_tokens: 16_384,
        max_nodes: 4096,
        max_edges: 4096,
        max_name_bytes: 32 * 1024,
        max_depth: 128,
    };
}
/// A node in the immutable postorder type-syntax arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericTypeKind {
    /// A qualified name with explicit type arguments; empty arguments mean a bare name.
    Named {
        /// Written path with canonical separators, without trivia.
        name: String,
        /// Arena indices of type arguments in source order.
        arguments: Vec<usize>,
    },
    /// Existing structural array spelling `[T]`.
    Array {
        /// Arena index of the element type.
        element: usize,
    },
}
/// Original full type range, with a separately inspectable structural kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericTypeNode {
    /// Original byte/line/column range, excluding surrounding trivia.
    pub span: Span,
    /// Syntax only: no name resolution, arity, bound or visibility proof.
    pub kind: GenericTypeKind,
}
/// Source-owned, fully admitted syntax. Children precede parents; no cycles exist.
#[derive(Debug, Clone)]
pub struct GenericTypeSyntax {
    source: Arc<str>,
    nodes: Arc<[GenericTypeNode]>,
    name_bytes: usize,
    edges: usize,
    token_count: usize,
}
impl GenericTypeSyntax {
    /// Admitted original lexer tokens, including EOF.
    pub fn token_count(&self) -> usize {
        self.token_count
    }
    /// Original complete input, retaining comments and exact coordinates.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Immutable postorder nodes; all child indices address earlier entries.
    pub fn nodes(&self) -> &[GenericTypeNode] {
        &self.nodes
    }
    /// Root node, always the final arena entry.
    pub fn root(&self) -> &GenericTypeNode {
        self.nodes.last().expect("admitted root")
    }
    /// Cumulative copied qualified-name bytes.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
    /// Cumulative structural child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
}
/// Complete refusal; no partial successful syntax is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericTypeError {
    /// Independent logical admission dimension exhausted.
    Limit(&'static str),
    /// Existing bounded lexer rejection.
    Lexical(Diagnostics),
    /// Malformed type grammar at an original source range.
    Syntax {
        /// Original offending token/EOF range.
        span: Span,
        /// Fixed bounded explanation.
        message: &'static str,
    },
}
impl std::fmt::Display for GenericTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic type syntax refused: {self:?}")
    }
}
impl std::error::Error for GenericTypeError {}
struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    limits: GenericTypeLimits,
    nodes: Vec<GenericTypeNode>,
    name_bytes: usize,
    edges: usize,
}
impl Parser {
    fn token(&self) -> &Token {
        &self.tokens[self.cursor]
    }
    fn error(&self, message: &'static str) -> GenericTypeError {
        GenericTypeError::Syntax {
            span: self.token().span,
            message,
        }
    }
    fn consume(&mut self, kind: TokenKind) -> bool {
        if self.token().kind == kind {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn edge(&mut self) -> Result<(), GenericTypeError> {
        self.edges = self
            .edges
            .checked_add(1)
            .filter(|n| *n <= self.limits.max_edges)
            .ok_or(GenericTypeError::Limit("edges"))?;
        Ok(())
    }
    fn name(&mut self, name: &mut String, text: &str) -> Result<(), GenericTypeError> {
        self.name_bytes = self
            .name_bytes
            .checked_add(text.len())
            .filter(|n| *n <= self.limits.max_name_bytes)
            .ok_or(GenericTypeError::Limit("name bytes"))?;
        name.push_str(text);
        Ok(())
    }
    fn ty(&mut self, depth: usize) -> Result<usize, GenericTypeError> {
        if depth > self.limits.max_depth {
            return Err(GenericTypeError::Limit("depth"));
        }
        // Admit the parent before descending, rather than allocating uncharged
        // pending parents when a caller selects a tiny node allowance.
        if self
            .nodes
            .len()
            .checked_add(depth)
            .filter(|n| *n <= self.limits.max_nodes)
            .is_none()
        {
            return Err(GenericTypeError::Limit("nodes"));
        }
        let mut span = self.token().span;
        let kind = if self.consume(TokenKind::Punctuation('[')) {
            self.edge()?;
            let element = self.ty(depth + 1)?;
            if !self.consume(TokenKind::Punctuation(']')) {
                return Err(self.error("expected closing array bracket"));
            }
            GenericTypeKind::Array { element }
        } else {
            let mut name = String::new();
            loop {
                let TokenKind::Identifier(text) = &self.tokens[self.cursor].kind else {
                    return Err(self.error("expected type name"));
                };
                // Source token text is already bounded; admission precedes our copy.
                let text = text.as_str();
                let amount = text.len();
                self.name_bytes = self
                    .name_bytes
                    .checked_add(amount)
                    .filter(|n| *n <= self.limits.max_name_bytes)
                    .ok_or(GenericTypeError::Limit("name bytes"))?;
                name.push_str(text);
                self.cursor += 1;
                if !self.consume(TokenKind::Operator("::")) {
                    break;
                }
                self.name(&mut name, "::")?;
            }
            let mut arguments = Vec::new();
            if self.consume(TokenKind::Operator("<")) {
                loop {
                    self.edge()?;
                    arguments.push(self.ty(depth + 1)?);
                    if self.consume(TokenKind::Operator(">")) {
                        break;
                    }
                    if !self.consume(TokenKind::Punctuation(',')) {
                        return Err(self.error("expected comma or closing type argument bracket"));
                    }
                }
            }
            GenericTypeKind::Named { name, arguments }
        };
        span.end = self.tokens[self.cursor - 1].span.end;
        let index = self.nodes.len();
        self.nodes.push(GenericTypeNode { span, kind });
        Ok(index)
    }
}
/// Parse exactly one type expression using the approved `Name<T,E>`/`[T]` grammar.
/// This does not resolve names, check generic arity/bounds, specialize or execute.
/// Existing v1/v2 source checkers remain strict about unsupported generic syntax.
pub fn parse_generic_type(
    source: &str,
    limits: GenericTypeLimits,
) -> Result<GenericTypeSyntax, GenericTypeError> {
    if limits.max_depth > 128 {
        return Err(GenericTypeError::Limit("fixed depth ceiling"));
    }
    let tokens = type_tokens(source, limits.max_source_bytes, limits.max_tokens)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        limits,
        nodes: Vec::new(),
        name_bytes: 0,
        edges: 0,
    };
    parser.ty(1)?;
    if parser.token().kind != TokenKind::Eof {
        return Err(parser.error("expected end of type expression"));
    }
    Ok(GenericTypeSyntax {
        source: Arc::from(source),
        token_count: parser.tokens.len(),
        nodes: parser.nodes.into(),
        name_bytes: parser.name_bytes,
        edges: parser.edges,
    })
}

pub(super) fn type_tokens(
    source: &str,
    max_source_bytes: usize,
    max_tokens: usize,
) -> Result<Vec<Token>, GenericTypeError> {
    if source.len() > max_source_bytes {
        return Err(GenericTypeError::Limit("source bytes"));
    }
    let tokens = Lexer::new()
        .tokenize_with_limits(
            source,
            FrontendLimits {
                max_source_bytes,
                max_tokens,
                ..FrontendLimits::DEFAULT
            },
        )
        .map_err(GenericTypeError::Lexical)?;
    // The shared lexer excludes EOF from its token allowance; this annotation
    // profile explicitly includes the sentinel in retained-token admission.
    if tokens.len() > max_tokens {
        return Err(GenericTypeError::Limit("tokens"));
    }
    Ok(tokens)
}

/// Independent syntax admission for explicit call/constructor type arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericTypeArgumentLimits {
    /// Cumulative type syntax limits across the complete `::<...>` input.
    pub types: GenericTypeLimits,
    /// Number of top-level explicit arguments.
    pub max_arguments: usize,
}
impl GenericTypeArgumentLimits {
    /// Finite syntax-only profile; does not enable generic calls.
    pub const DEFAULT: Self = Self {
        types: GenericTypeLimits::DEFAULT,
        max_arguments: 128,
    };
}
/// Source-owned explicit type arguments sharing a flat postorder arena.
/// These are syntax, not checked arguments or a callable specialization.
#[derive(Debug, Clone)]
pub struct GenericTypeArgumentsSyntax {
    source: Arc<str>,
    nodes: Arc<[GenericTypeNode]>,
    roots: Arc<[usize]>,
    tokens: usize,
    edges: usize,
    name_bytes: usize,
    span: Span,
}
impl GenericTypeArgumentsSyntax {
    // Absence of a type-list on a checked nongeneric body call. This is never
    // returned by the explicit nonempty argument-list parser.
    pub(super) fn absent() -> Self {
        Self {
            source: Arc::from(""),
            nodes: Arc::from([]),
            roots: Arc::from([]),
            tokens: 0,
            edges: 0,
            name_bytes: 0,
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        }
    }

    /// Complete original input, shared across every argument node.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Original `::<...>` range, excluding surrounding trivia.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Flat immutable postorder nodes; children precede parents.
    pub fn nodes(&self) -> &[GenericTypeNode] {
        &self.nodes
    }
    /// Source-order top-level argument roots, not necessarily adjacent nodes.
    pub fn roots(&self) -> &[usize] {
        &self.roots
    }
    /// Admitted lexer tokens, including EOF.
    pub fn token_count(&self) -> usize {
        self.tokens
    }
    /// Structural child edges plus one list edge for each argument root.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Cumulative copied qualified-name bytes.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
}
/// Parse exactly one nonempty explicit `::<T,E>` list, preserving original ranges.
/// No callee lookup, argument binding, inference, specialization or execution occurs.
pub fn parse_generic_type_arguments(
    source: &str,
    limits: GenericTypeArgumentLimits,
) -> Result<GenericTypeArgumentsSyntax, GenericTypeError> {
    if limits.types.max_depth > 128 {
        return Err(GenericTypeError::Limit("fixed depth ceiling"));
    }
    let tokens = type_tokens(
        source,
        limits.types.max_source_bytes,
        limits.types.max_tokens,
    )?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        limits: limits.types,
        nodes: Vec::new(),
        name_bytes: 0,
        edges: 0,
    };
    let mut span = parser.token().span;
    if !parser.consume(TokenKind::Operator("::")) || !parser.consume(TokenKind::Operator("<")) {
        return Err(parser.error("expected explicit type argument introducer ::<"));
    }
    let mut roots = Vec::new();
    loop {
        if roots.len() >= limits.max_arguments {
            return Err(GenericTypeError::Limit("type arguments"));
        }
        parser.edge()?;
        roots.push(parser.ty(1)?);
        if parser.consume(TokenKind::Operator(">")) {
            break;
        }
        if !parser.consume(TokenKind::Punctuation(',')) {
            return Err(parser.error("expected comma or closing explicit type argument bracket"));
        }
    }
    span.end = parser.tokens[parser.cursor - 1].span.end;
    if parser.token().kind != TokenKind::Eof {
        return Err(parser.error("expected end of explicit type arguments"));
    }
    Ok(GenericTypeArgumentsSyntax {
        source: Arc::from(source),
        nodes: parser.nodes.into(),
        roots: roots.into(),
        tokens: parser.tokens.len(),
        edges: parser.edges,
        name_bytes: parser.name_bytes,
        span,
    })
}

impl GenericTypeArgumentsSyntax {
    // The parser produces disjoint contiguous postorder argument subarenas.
    // The caller admits the complete copied node/edge/name totals first.
    pub(super) fn argument_syntax(&self, ordinal: usize) -> Option<GenericTypeSyntax> {
        let end = self.roots.get(ordinal)?.checked_add(1)?;
        let start = if ordinal == 0 {
            0
        } else {
            self.roots[ordinal - 1] + 1
        };
        let mut edges = 0;
        let mut name_bytes = 0;
        let nodes: Vec<_> = self.nodes[start..end]
            .iter()
            .map(|node| {
                let kind = match &node.kind {
                    GenericTypeKind::Array { element } => {
                        edges += 1;
                        GenericTypeKind::Array {
                            element: element - start,
                        }
                    }
                    GenericTypeKind::Named { name, arguments } => {
                        edges += arguments.len();
                        name_bytes += name.len();
                        GenericTypeKind::Named {
                            name: name.clone(),
                            arguments: arguments.iter().map(|child| child - start).collect(),
                        }
                    }
                };
                GenericTypeNode {
                    span: node.span,
                    kind,
                }
            })
            .collect();
        Some(GenericTypeSyntax {
            source: self.source.clone(),
            nodes: nodes.into(),
            name_bytes,
            edges,
            token_count: self.tokens,
        })
    }
}

impl GenericTypeArgumentsSyntax {
    // Exact fragment parse relocated into an owning call source, without text edits.
    pub(super) fn in_source(self, source: Arc<str>, origin: Span) -> Self {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .cloned()
            .map(|mut node| {
                node.span = super::generic_aliases::translate(node.span, origin);
                node
            })
            .collect();
        Self {
            source,
            nodes: nodes.into(),
            span: super::generic_aliases::translate(self.span, origin),
            ..self
        }
    }
}

impl GenericTypeArgumentsSyntax {
    // Validated explicit constructor grammar supplies the original nominal head.
    // The caller admits one extra node/name and original prefix tokens/bytes.
    pub(super) fn constructor_type(
        self,
        source: Arc<str>,
        origin: Span,
        name: &str,
        span: Span,
        prefix_tokens: usize,
    ) -> GenericTypeSyntax {
        let arguments = self.in_source(source.clone(), origin);
        let mut nodes = arguments.nodes.to_vec();
        nodes.push(GenericTypeNode {
            span,
            kind: GenericTypeKind::Named {
                name: name.to_owned(),
                arguments: arguments.roots.to_vec(),
            },
        });
        GenericTypeSyntax {
            source,
            nodes: nodes.into(),
            name_bytes: arguments.name_bytes + name.len(),
            edges: arguments.edges,
            token_count: arguments.tokens + prefix_tokens,
        }
    }
}
