//! Parse opaque generic call values into original-coordinate expression ASTs.
use super::{generic_types::type_tokens, GenericCallSyntax, GenericTypeError};
use crate::compiler::{
    ast::{Expression, ExpressionKind},
    diagnostics::{Diagnostics, Span},
    parser::Parser,
};
use std::sync::Arc;
/// Original-source and cumulative AST retention admission for one call envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCallExpressionLimits {
    /// Complete retained original call source bytes before relexing.
    pub max_source_bytes: usize,
    /// Relexed whole-call tokens including EOF.
    pub max_tokens: usize,
    /// Parsed value roots.
    pub max_values: usize,
    /// Cumulative expression AST nodes, including callee nodes in nested plain calls.
    pub max_nodes: usize,
    /// Cumulative retained AST child edges.
    pub max_edges: usize,
    /// Admitted tokens, root visits and AST node/edge traversal work.
    pub max_work: usize,
    /// Parser diagnostic cap; zero retains minimum one. Lexer diagnostics use its bounded default.
    pub max_diagnostics: usize,
}
impl GenericCallExpressionLimits {
    /// Finite syntax profile; existing parser depth limits remain active.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 64 * 1024,
        max_tokens: 16_384,
        max_values: 1024,
        max_nodes: 4096,
        max_edges: 4096,
        max_work: 65_536,
        max_diagnostics: 64,
    };
}
/// Complete expression-list refusal without partial parsed value publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericCallExpressionError {
    /// Whole-source lexer/admission failure.
    Input(GenericTypeError),
    /// Cumulative root/AST/work admission exhausted.
    Limit(&'static str),
    /// Existing parser rejected an original value range.
    Parse {
        /// Source-order value index.
        argument: usize,
        /// Original whole-call coordinates; no synthetic wrapper is used.
        diagnostics: Diagnostics,
    },
    /// Legacy raw parser admitted an operator outside the expression profile.
    Operator {
        /// Source-order value index.
        argument: usize,
        /// Original binary expression range.
        span: Span,
    },
    /// Parser prefix did not consume exactly the admitted value range.
    Boundary {
        /// Source-order value index.
        argument: usize,
        /// Original offending remainder or fragment range.
        span: Span,
    },
}
impl std::fmt::Display for GenericCallExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic call expressions refused: {self:?}")
    }
}
impl std::error::Error for GenericCallExpressionError {}
#[derive(Debug)]
struct Expressions(Vec<Expression>);
impl Drop for Expressions {
    fn drop(&mut self) {
        for expression in self.0.drain(..) {
            expression.drop_iterative();
        }
    }
}
/// Fully parsed original value expressions, not typed values or a checked call.
#[derive(Debug, Clone)]
pub struct ParsedGenericCallExpressions {
    syntax: GenericCallSyntax,
    expressions: Arc<Expressions>,
    tokens: usize,
    nodes: usize,
    edges: usize,
    work: usize,
}
impl ParsedGenericCallExpressions {
    /// Original envelope retaining source, callee and explicit type ranges.
    pub fn syntax(&self) -> &GenericCallSyntax {
        &self.syntax
    }
    /// Parsed source-order expression roots with original full-call ranges.
    pub fn expressions(&self) -> &[Expression] {
        &self.expressions.0
    }
    /// Admitted original tokens including EOF.
    pub fn tokens(&self) -> usize {
        self.tokens
    }
    /// Cumulative admitted AST nodes.
    pub fn nodes(&self) -> usize {
        self.nodes
    }
    /// Cumulative admitted child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Consumed token/root/AST traversal work, not parser CPU time.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    amount: usize,
    dimension: &'static str,
) -> Result<(), GenericCallExpressionError> {
    *left = left
        .checked_sub(amount)
        .ok_or(GenericCallExpressionError::Limit(dimension))?;
    Ok(())
}
/// Parse all envelope values with the existing expression grammar and exact original
/// token boundaries. Unsupported explicit nested generic calls still refuse; this
/// API does not resolve names, contextualize literals, type-check or execute.
pub fn parse_generic_call_expressions(
    syntax: &GenericCallSyntax,
    limits: GenericCallExpressionLimits,
) -> Result<ParsedGenericCallExpressions, GenericCallExpressionError> {
    if syntax.value_ranges().len() > limits.max_values {
        return Err(GenericCallExpressionError::Limit("value roots"));
    }
    let tokens = type_tokens(syntax.source(), limits.max_source_bytes, limits.max_tokens)
        .map_err(GenericCallExpressionError::Input)?;
    let mut work = limits.max_work;
    let mut nodes = limits.max_nodes;
    let mut edges = limits.max_edges;
    charge(&mut work, tokens.len(), "expression work")?;
    let mut expressions = Expressions(Vec::new());
    for (argument, range) in syntax.value_ranges().iter().enumerate() {
        charge(&mut work, 1, "expression work")?;
        let start = tokens
            .binary_search_by_key(&range.start, |token| token.span.start)
            .map_err(|_| GenericCallExpressionError::Boundary {
                argument,
                span: *range,
            })?;
        let (expression, consumed) = Parser::new()
            .parse_expression_prefix(&tokens[start..], limits.max_diagnostics)
            .map_err(|diagnostics| GenericCallExpressionError::Parse {
                argument,
                diagnostics,
            })?;
        let mut guard = Expressions(vec![expression]);
        if consumed == 0 || tokens[start + consumed - 1].span.end != range.end {
            let span = tokens
                .get(start + consumed)
                .map_or(*range, |token| token.span);
            return Err(GenericCallExpressionError::Boundary { argument, span });
        }
        let mut pending = vec![&guard.0[0]];
        while let Some(expression) = pending.pop() {
            charge(&mut work, 1, "expression work")?;
            charge(&mut nodes, 1, "expression nodes")?;
            let mut push = |child| {
                charge(&mut edges, 1, "expression edges")?;
                charge(&mut work, 1, "expression work")?;
                pending.push(child);
                Ok::<(), GenericCallExpressionError>(())
            };
            match &expression.kind {
                ExpressionKind::Binary {
                    left,
                    operator,
                    right,
                } => {
                    if !matches!(
                        operator.as_str(),
                        "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/"
                    ) {
                        return Err(GenericCallExpressionError::Operator {
                            argument,
                            span: expression.span,
                        });
                    }
                    push(right)?;
                    push(left)?;
                }
                ExpressionKind::Call { callee, arguments } => {
                    for child in arguments.iter().rev() {
                        push(child)?;
                    }
                    push(callee)?;
                }
                ExpressionKind::ArrayLiteral(items) => {
                    for child in items.iter().rev() {
                        push(child)?;
                    }
                }
                ExpressionKind::StructLiteral { fields, .. } => {
                    for (_, child) in fields.iter().rev() {
                        push(child)?;
                    }
                }
                ExpressionKind::FieldAccess { receiver, .. } => push(receiver)?,
                ExpressionKind::Index { target, index } => {
                    push(index)?;
                    push(target)?;
                }
                _ => {}
            }
        }
        expressions
            .0
            .push(guard.0.pop().expect("owned parsed expression"));
    }
    Ok(ParsedGenericCallExpressions {
        syntax: syntax.clone(),
        expressions: Arc::new(expressions),
        tokens: tokens.len(),
        nodes: limits.max_nodes - nodes,
        edges: limits.max_edges - edges,
        work: limits.max_work - work,
    })
}
