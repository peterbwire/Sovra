//! Source-backed explicit call envelopes with opaque, balanced value arguments.
use super::{
    generic_types::type_tokens, parse_generic_type_arguments, GenericTypeArgumentLimits,
    GenericTypeArgumentsSyntax, GenericTypeError,
};
use crate::compiler::{diagnostics::Span, lexer::TokenKind};
use std::sync::Arc;
/// Whole-call and independently bounded explicit-list syntax admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCallSyntaxLimits {
    /// Complete original call source bytes.
    pub max_source_bytes: usize,
    /// Complete original lexer tokens including EOF.
    pub max_tokens: usize,
    /// Copied canonical callee path bytes, including `::` separators.
    pub max_name_bytes: usize,
    /// Source-order opaque value argument ranges.
    pub max_values: usize,
    /// Balanced value/call delimiters, including the outer call; at most 128.
    pub max_depth: usize,
    /// Independent explicit type-list syntax profile.
    pub types: GenericTypeArgumentLimits,
}
impl GenericCallSyntaxLimits {
    /// Finite envelope profile; value expressions are not checked by this stage.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 64 * 1024,
        max_tokens: 16_384,
        max_name_bytes: 4096,
        max_values: 1024,
        max_depth: 128,
        types: GenericTypeArgumentLimits::DEFAULT,
    };
}
/// Original explicit call envelope, not a checked callee or value-expression AST.
#[derive(Debug, Clone)]
pub struct GenericCallSyntax {
    source: Arc<str>,
    callee: String,
    callee_span: Span,
    span: Span,
    types: GenericTypeArgumentsSyntax,
    values: Arc<[Span]>,
    tokens: usize,
}
impl GenericCallSyntax {
    /// Complete retained original source.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Canonical root/qualified callee path spelling, without trivia.
    pub fn callee(&self) -> &str {
        &self.callee
    }
    /// Original full callee path range.
    pub fn callee_span(&self) -> Span {
        self.callee_span
    }
    /// Original call range excluding surrounding trivia.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Checked grammar only; roots/spans address this same original call source.
    pub fn type_arguments(&self) -> &GenericTypeArgumentsSyntax {
        &self.types
    }
    /// Nonempty balanced value fragments in source order; expressions remain opaque.
    pub fn value_ranges(&self) -> &[Span] {
        &self.values
    }
    /// Admitted whole-call lexer tokens, including EOF.
    pub fn token_count(&self) -> usize {
        self.tokens
    }
}
fn syntax(span: Span, message: &'static str) -> GenericTypeError {
    GenericTypeError::Syntax { span, message }
}
/// Parse one `name::<T>(...)` envelope. The value fragments are lexed and balanced
/// but not expression-parsed, typed or executed. Qualified paths are syntax only.
pub fn parse_generic_call(
    source: &str,
    limits: GenericCallSyntaxLimits,
) -> Result<GenericCallSyntax, GenericTypeError> {
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(GenericTypeError::Limit("call delimiter depth profile"));
    }
    let tokens = type_tokens(source, limits.max_source_bytes, limits.max_tokens)?;
    let mut at = 0;
    let start = tokens[at].span;
    let mut callee = String::new();
    loop {
        let TokenKind::Identifier(name) = &tokens[at].kind else {
            return Err(syntax(tokens[at].span, "expected generic callee name"));
        };
        let needed = callee
            .len()
            .checked_add(name.len())
            .filter(|n| *n <= limits.max_name_bytes)
            .ok_or(GenericTypeError::Limit("callee name bytes"))?;
        callee.reserve(needed - callee.len());
        callee.push_str(name);
        at += 1;
        if tokens[at].kind != TokenKind::Operator("::") {
            return Err(syntax(tokens[at].span, "expected explicit type arguments"));
        }
        if tokens
            .get(at + 1)
            .is_some_and(|t| t.kind == TokenKind::Operator("<"))
        {
            break;
        }
        if callee
            .len()
            .checked_add(2)
            .map_or(true, |n| n > limits.max_name_bytes)
        {
            return Err(GenericTypeError::Limit("callee name bytes"));
        }
        callee.push_str("::");
        at += 1;
    }
    let callee_span = Span {
        end: tokens[at - 1].span.end,
        ..start
    };
    let type_origin = tokens[at].span;
    let type_start = at;
    at += 2;
    let mut angle = 1usize;
    while angle > 0 {
        match tokens[at].kind {
            TokenKind::Operator("<") => angle += 1,
            TokenKind::Operator(">") => angle -= 1,
            TokenKind::Eof => {
                return Err(syntax(
                    tokens[at].span,
                    "unterminated explicit type arguments",
                ))
            }
            _ => {}
        }
        at += 1;
    }
    let type_end = tokens[at - 1].span.end;
    let types = parse_generic_type_arguments(
        &source[tokens[type_start].span.start..type_end],
        limits.types,
    )
    .map_err(|error| super::generic_aliases::translate_error(error, type_origin))?;
    if tokens[at].kind != TokenKind::Punctuation('(') {
        return Err(syntax(
            tokens[at].span,
            "expected generic call value arguments",
        ));
    }
    at += 1;
    let mut values = Vec::new();
    if tokens[at].kind != TokenKind::Punctuation(')') {
        loop {
            if values.len() >= limits.max_values {
                return Err(GenericTypeError::Limit("call value arguments"));
            }
            let value_start = at;
            let mut stack = Vec::new();
            let mut angles = 0usize;
            loop {
                match tokens[at].kind {
                    TokenKind::Eof => {
                        return Err(syntax(tokens[at].span, "unterminated call value arguments"))
                    }
                    TokenKind::Punctuation(',') if stack.is_empty() => break,
                    TokenKind::Punctuation(')') if stack.is_empty() => break,
                    TokenKind::Punctuation(open @ ('(' | '[' | '{')) => {
                        if stack.len() + 2 > limits.max_depth {
                            return Err(GenericTypeError::Limit("call delimiter depth"));
                        }
                        stack.push(match open {
                            '(' => ')',
                            '[' => ']',
                            _ => '}',
                        });
                    }
                    TokenKind::Operator("<")
                        if angles > 0
                            || (at > value_start
                                && tokens[at - 1].kind == TokenKind::Operator("::")) =>
                    {
                        if stack.len() + 2 > limits.max_depth {
                            return Err(GenericTypeError::Limit("call delimiter depth"));
                        }
                        stack.push('>');
                        angles += 1;
                    }
                    TokenKind::Operator(">") if angles > 0 => {
                        if stack.pop() != Some('>') {
                            return Err(syntax(
                                tokens[at].span,
                                "mismatched call argument delimiter",
                            ));
                        }
                        angles -= 1;
                    }
                    TokenKind::Punctuation(close @ (')' | ']' | '}'))
                        if stack.pop() != Some(close) =>
                    {
                        return Err(syntax(
                            tokens[at].span,
                            "mismatched call argument delimiter",
                        ));
                    }
                    _ => {}
                }
                at += 1;
            }
            if at == value_start {
                return Err(syntax(tokens[at].span, "expected nonempty value argument"));
            }
            values.push(Span {
                end: tokens[at - 1].span.end,
                ..tokens[value_start].span
            });
            if tokens[at].kind == TokenKind::Punctuation(')') {
                break;
            }
            at += 1;
        }
    }
    let span = Span {
        end: tokens[at].span.end,
        ..start
    };
    at += 1;
    if tokens[at].kind != TokenKind::Eof {
        return Err(syntax(tokens[at].span, "expected end of generic call"));
    }
    let source: Arc<str> = Arc::from(source);
    let types = types.in_source(source.clone(), type_origin);
    Ok(GenericCallSyntax {
        source,
        callee,
        callee_span,
        span,
        types,
        values: values.into(),
        tokens: tokens.len(),
    })
}
