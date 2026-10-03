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
        let mut statements: Vec<_> = function
            .body
            .iter()
            .rev()
            .map(|statement| (statement, 1))
            .collect();
        while let Some((statement, block_depth)) = statements.pop() {
            let span = match statement {
                Statement::Let { span, .. }
                | Statement::Assign { span, .. }
                | Statement::Return { span, .. }
                | Statement::If { span, .. }
                | Statement::While { span, .. } => *span,
                Statement::Expression(expression) => expression.span,
            };
            if block_depth > MAX_STRUCTURAL_DEPTH {
                return Err(span);
            }
            let mut pending = Vec::new();
            match statement {
                Statement::Let { value, .. } | Statement::Expression(value) => {
                    pending.push((value, 1))
                }
                Statement::Assign { target, value, .. } => {
                    pending.push((value, 1));
                    pending.push((target, 1));
                }
                Statement::Return { value, .. } => {
                    pending.extend(value.iter().map(|value| (value, 1)))
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if block_depth == MAX_STRUCTURAL_DEPTH {
                        return Err(span);
                    }
                    pending.push((condition, 1));
                    if let Some(body) = else_block {
                        statements.extend(
                            body.iter()
                                .rev()
                                .map(|statement| (statement, block_depth + 1)),
                        );
                    }
                    statements.extend(
                        then_block
                            .iter()
                            .rev()
                            .map(|statement| (statement, block_depth + 1)),
                    );
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if block_depth == MAX_STRUCTURAL_DEPTH {
                        return Err(span);
                    }
                    pending.push((condition, 1));
                    statements.extend(
                        body.iter()
                            .rev()
                            .map(|statement| (statement, block_depth + 1)),
                    );
                }
            }
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
                    ExpressionKind::FieldAccess { receiver, .. } => {
                        pending.push((receiver, depth + 1))
                    }
                    ExpressionKind::StructLiteral { fields, .. } => {
                        pending.extend(fields.iter().rev().map(|(_, value)| (value, depth + 1)))
                    }
                    ExpressionKind::ArrayLiteral(items) => {
                        pending.extend(items.iter().rev().map(|value| (value, depth + 1)))
                    }
                    ExpressionKind::Index { target, index } => {
                        pending.push((index, depth + 1));
                        pending.push((target, depth + 1));
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
    let mut brackets = 0usize;
    for token in tokens {
        match token.kind {
            TokenKind::Punctuation('(') => parentheses += 1,
            TokenKind::Punctuation(')') => parentheses = parentheses.saturating_sub(1),
            TokenKind::Punctuation('[') => brackets += 1,
            TokenKind::Punctuation(']') => brackets = brackets.saturating_sub(1),
            TokenKind::Punctuation('{') if include_blocks => blocks += 1,
            TokenKind::Punctuation('}') if include_blocks => blocks = blocks.saturating_sub(1),
            _ => {}
        }
        if parentheses > MAX_STRUCTURAL_DEPTH
            || blocks > MAX_STRUCTURAL_DEPTH
            || brackets > MAX_STRUCTURAL_DEPTH
        {
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
    fn ast_guard_checks_nested_bodies_assignment_targets_and_block_depth() {
        let span = Span {
            start: 0,
            end: 1,
            line: 0,
            column: 0,
        };
        let leaf = || Expression {
            kind: ExpressionKind::Boolean(true),
            span,
        };
        for context in 0..4 {
            let mut expression = leaf();
            for _ in 0..128 {
                expression = Expression {
                    kind: ExpressionKind::FieldAccess {
                        receiver: Box::new(expression),
                        field: "x".into(),
                    },
                    span,
                };
            }
            let statement = match context {
                0 => Statement::If {
                    condition: leaf(),
                    then_block: vec![],
                    else_block: Some(vec![Statement::Expression(expression)]),
                    span,
                },
                1 => Statement::While {
                    condition: leaf(),
                    body: vec![Statement::Expression(expression)],
                    span,
                },
                2 => Statement::Assign {
                    target: expression,
                    value: leaf(),
                    span,
                },
                _ => {
                    expression.drop_iterative();
                    let mut statement = Statement::Expression(leaf());
                    for _ in 0..128 {
                        statement = Statement::If {
                            condition: leaf(),
                            then_block: vec![statement],
                            else_block: None,
                            span,
                        };
                    }
                    statement
                }
            };
            let mut program = crate::compiler::parser::Parser::new()
                .parse_source("fn main() {}")
                .unwrap();
            program.functions[0].body.push(statement);
            assert_eq!(
                check_program_depth(&program),
                Err(span),
                "context {context}"
            );
        }
    }

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
                        is_mutable: false,
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
