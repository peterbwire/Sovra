//! Bounded syntax-only postfix capture without exporting an unwrapped legacy AST.
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, TokenKind},
    parser::Parser,
    resources::FrontendLimits,
};
use std::{collections::BTreeMap, sync::Arc};
/// Exact original source ranges for one postfix application, in parse evaluation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPropagationSyntaxSite {
    /// Whole operand range, including earlier postfix markers when chained.
    pub operand: Span,
    /// The original single question-mark token.
    pub question: Span,
    /// Whole resulting expression range.
    pub expression: Span,
}
/// Finite standalone block syntax admission, separate from typing and execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPropagationSyntaxLimits {
    /// Source, original non-EOF tokens and diagnostics.
    pub frontend: FrontendLimits,
    /// Retained postfix application records.
    pub max_sites: usize,
    /// Questions in one postfix chain, including questions separated by field/index/call suffixes.
    pub max_chain: usize,
    /// Original bytes/tokens plus retained marker-range validation work.
    pub max_work: usize,
}
impl GenericPropagationSyntaxLimits {
    /// Finite additive profile; no ordinary compiler/CLI grammar changes.
    pub const DEFAULT: Self = Self {
        frontend: FrontendLimits {
            max_source_bytes: 128 * 1024,
            max_tokens: 32768,
            max_diagnostics: 64,
        },
        max_sites: 4096,
        max_chain: 128,
        max_work: 1_000_000,
    };
}
/// Complete syntax refusal; no partially retained sites or legacy AST is published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericPropagationSyntaxError {
    /// Logical input/metadata admission exhausted.
    Limit(&'static str),
    /// Original lexer/parser failure.
    Diagnostics(Diagnostics),
    /// The original source must be exactly one complete block.
    Source(Span),
}
impl std::fmt::Display for GenericPropagationSyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "propagation syntax refused: {self:?}")
    }
}
impl std::error::Error for GenericPropagationSyntaxError {}
/// Owned original block and private AST-free marker projection; not a typing proof.
#[derive(Debug, Clone)]
pub struct GenericPropagationSyntax {
    source: Arc<str>,
    sites: Arc<[GenericPropagationSyntaxSite]>,
    tokens: usize,
    work: usize,
}
impl GenericPropagationSyntax {
    /// Unchanged original source, including comments/trivia.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Sites in parse evaluation order; nested operands precede their containing marker.
    pub fn sites(&self) -> &[GenericPropagationSyntaxSite] {
        &self.sites
    }
    /// Original token count, including EOF.
    pub fn tokens(&self) -> usize {
        self.tokens
    }
    /// Original input and retained marker validation costs.
    pub fn work(&self) -> usize {
        self.work
    }
}
/// Parse exactly one ordinary/generic block with syntax-only postfix markers.
/// No legacy AST escapes this adapter, and no marker is authorized for execution.
pub fn parse_generic_propagation_block(
    source: &str,
    limits: GenericPropagationSyntaxLimits,
) -> Result<GenericPropagationSyntax, GenericPropagationSyntaxError> {
    if limits.max_chain > 128 {
        return Err(GenericPropagationSyntaxError::Limit(
            "propagation syntax chain profile",
        ));
    }
    let mut work = limits.max_work;
    let charge = |left: &mut usize, n| {
        *left = left
            .checked_sub(n)
            .ok_or(GenericPropagationSyntaxError::Limit(
                "propagation syntax work",
            ))?;
        Ok::<(), GenericPropagationSyntaxError>(())
    };
    charge(&mut work, source.len())?;
    let tokens = Lexer::new()
        .tokenize_propagation(source, limits.frontend)
        .map_err(GenericPropagationSyntaxError::Diagnostics)?;
    charge(&mut work, tokens.len())?;
    // All possible marker validation is admitted before parsing/capture allocation.
    let questions = tokens
        .iter()
        .filter(|t| t.kind == TokenKind::Punctuation('?'))
        .count();
    if questions > limits.max_sites {
        return Err(GenericPropagationSyntaxError::Limit(
            "propagation syntax sites",
        ));
    }
    charge(
        &mut work,
        questions
            .checked_mul(3)
            .ok_or(GenericPropagationSyntaxError::Limit(
                "propagation syntax work",
            ))?,
    )?;
    if let Err(span) = super::super::limits::check_nesting(&tokens, true) {
        return Err(GenericPropagationSyntaxError::Source(span));
    }
    let (mut body, used, _, _matches, sites) = Parser::new()
        .parse_propagation_block_prefix(
            &tokens,
            limits.frontend.max_diagnostics,
            limits.max_sites,
            limits.max_chain,
        )
        .map_err(GenericPropagationSyntaxError::Diagnostics)?;
    super::generic_copy_bodies::drop_body(std::mem::take(&mut body));
    if used != tokens.len() - 1 {
        return Err(GenericPropagationSyntaxError::Source(
            tokens[used.min(tokens.len() - 1)].span,
        ));
    }
    validate_sites(&tokens, &sites).map_err(GenericPropagationSyntaxError::Source)?;
    Ok(GenericPropagationSyntax {
        source: Arc::from(source),
        sites: sites.into(),
        tokens: tokens.len(),
        work: limits.max_work - work,
    })
}

pub(super) fn validate_sites(
    tokens: &[crate::compiler::lexer::Token],
    sites: &[GenericPropagationSyntaxSite],
) -> Result<(), Span> {
    let mut captured = BTreeMap::new();
    for site in sites {
        if captured
            .insert(site.question.start, site.question)
            .is_some()
        {
            return Err(site.question);
        }
    }
    for token in tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Punctuation('?'))
    {
        if captured.remove(&token.span.start) != Some(token.span) {
            return Err(token.span);
        }
    }
    if let Some((_, span)) = captured.first_key_value() {
        return Err(*span);
    }
    Ok(())
}
