//! Internal structural bounds shared by source and application parsers.

use super::ast::{ExpressionKind, Program, Statement};
use super::diagnostics::Span;
use super::lexer::{Token, TokenKind};

pub(super) const MAX_STRUCTURAL_DEPTH: usize = 128;

pub(super) fn depth_message() -> String {
    format!("maximum structural depth of {MAX_STRUCTURAL_DEPTH} exceeded")
}

// Borrow-only traversal: reject caller-built trees before recursive analysis or
// cloning. Ownership and recursive destruction remain the caller's responsibility.
pub(super) fn check_program_depth(program: &Program) -> Result<(), Span> {
    for function in program
        .functions
        .iter()
        .chain(program.modules.iter().flat_map(|module| &module.functions))
    {
        for statement in &function.body {
            let expression = match statement {
                Statement::Let { value, .. } | Statement::Expression(value) => value,
                Statement::Return {
                    value: Some(value), ..
                } => value,
                Statement::Return { value: None, .. } => continue,
            };
            let mut pending = vec![(expression, 1)];
            while let Some((expression, depth)) = pending.pop() {
                if depth > MAX_STRUCTURAL_DEPTH {
                    return Err(expression.span);
                }
                match &expression.kind {
                    ExpressionKind::Binary { left, right, .. } => {
                        pending.push((right, depth + 1));
                        pending.push((left, depth + 1));
                    }
                    ExpressionKind::Call { callee, arguments } => {
                        pending
                            .extend(arguments.iter().rev().map(|argument| (argument, depth + 1)));
                        pending.push((callee, depth + 1));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::ast::Expression;

    #[test]
    fn ast_depth_checks_initializers_returns_callees_and_arguments() {
        let span = Span {
            start: 7,
            end: 8,
            line: 0,
            column: 7,
        };
        for callee_chain in [false, true] {
            for context in 0..3 {
                let mut expression = Expression {
                    kind: ExpressionKind::Integer("1".into()),
                    span,
                };
                for _ in 0..128 {
                    let leaf = Expression {
                        kind: ExpressionKind::Identifier("f".into()),
                        span,
                    };
                    expression = Expression {
                        kind: if callee_chain {
                            ExpressionKind::Call {
                                callee: Box::new(expression),
                                arguments: vec![leaf],
                            }
                        } else {
                            ExpressionKind::Call {
                                callee: Box::new(leaf),
                                arguments: vec![expression],
                            }
                        },
                        span,
                    };
                }
                let mut program = crate::compiler::parser::Parser::new()
                    .parse_source("fn main() {}")
                    .unwrap();
                program.functions[0].body = vec![match context {
                    0 => Statement::Let {
                        name: "x".into(),
                        type_name: None,
                        type_span: None,
                        value: expression,
                        span,
                    },
                    1 => Statement::Return {
                        value: Some(expression),
                        span,
                    },
                    _ => Statement::Expression(expression),
                }];
                assert_eq!(check_program_depth(&program), Err(span));
            }
        }
    }
}
