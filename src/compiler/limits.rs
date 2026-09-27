//! Internal structural bounds shared by source and application parsers.

use super::diagnostics::Span;
use super::lexer::{Token, TokenKind};

pub(super) const MAX_STRUCTURAL_DEPTH: usize = 128;

pub(super) fn depth_message() -> String {
    format!("maximum structural depth of {MAX_STRUCTURAL_DEPTH} exceeded")
}

// Scan iteratively before any recursive parsing. Strings/comments are already
// tokenized, so delimiter text inside them does not consume nesting depth.
pub(super) fn check_nesting(tokens: &[Token], include_blocks: bool) -> Result<(), Span> {
    let mut parentheses = 0usize;
    let mut blocks = 0usize;
    for token in tokens {
        match token.kind {
            TokenKind::Punctuation('(') => parentheses += 1,
            TokenKind::Punctuation(')') => parentheses = parentheses.saturating_sub(1),
            TokenKind::Punctuation('{') if include_blocks => blocks += 1,
            TokenKind::Punctuation('}') if include_blocks => blocks = blocks.saturating_sub(1),
            _ => {}
        }
        if parentheses > MAX_STRUCTURAL_DEPTH || blocks > MAX_STRUCTURAL_DEPTH {
            return Err(token.span);
        }
    }
    Ok(())
}
