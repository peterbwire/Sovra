//! Abstract syntax tree produced by the M2 parser.

use crate::compiler::diagnostics::Span;

/// A complete Sovra source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level function declarations.
    pub functions: Vec<Function>,
    /// Named source modules defined in the file.
    pub modules: Vec<Module>,
}

impl Program {
    /// Consume this AST without recursive expression destruction.
    /// Useful for caller-built trees that exceed parser bounds. Ordinary Rust
    /// drop remains recursive; this does not impose an allocation/work budget.
    pub fn drop_iterative(mut self) {
        for function in self.functions.iter_mut().chain(
            self.modules
                .iter_mut()
                .flat_map(|module| &mut module.functions),
        ) {
            for statement in function.body.drain(..) {
                match statement {
                    Statement::Let { value, .. }
                    | Statement::Expression(value)
                    | Statement::Return {
                        value: Some(value), ..
                    } => value.drop_iterative(),
                    Statement::Return { value: None, .. } => {}
                }
            }
        }
    }
}

/// A named source module.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module name.
    pub name: String,
    /// Exported functions in the module.
    pub functions: Vec<Function>,
    /// Location of the module declaration.
    pub span: Span,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Function name.
    pub name: String,
    /// Whether the function is exported from a module.
    pub is_exported: bool,
    /// Function parameters.
    pub parameters: Vec<Parameter>,
    /// Optional declared return type.
    pub return_type: Option<String>,
    /// Exact return type token range, when an annotation is present.
    pub return_type_span: Option<Span>,
    /// Function body.
    pub body: Vec<Statement>,
    /// Location of the declaration.
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// Parameter annotation, required by semantic analysis. None preserves
    /// incomplete declarations so the analyzer can diagnose the missing type.
    pub type_name: Option<String>,
    /// Exact parameter type token range, when an annotation is present.
    pub type_span: Option<Span>,
    /// Location of the parameter.
    pub span: Span,
}

/// A statement in a function body.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// A local binding.
    Let {
        /// Binding name.
        name: String,
        /// Optional declared binding type.
        type_name: Option<String>,
        /// Exact binding type token range, when an annotation is present.
        type_span: Option<Span>,
        /// Initializer expression.
        value: Expression,
        /// Statement location.
        span: Span,
    },
    /// A return statement.
    Return {
        /// Optional returned expression.
        value: Option<Expression>,
        /// Statement location.
        span: Span,
    },
    /// An expression used for its effects.
    Expression(Expression),
}

/// An expression with its complete source range.
#[derive(Debug, PartialEq)]
pub struct Expression {
    /// Expression structure and operands.
    pub kind: ExpressionKind,
    /// Source range, including grouping parentheses when present.
    pub span: Span,
}

impl Expression {
    /// Consume an expression tree using a heap worklist rather than recursive drop.
    /// Automatic Rust drop is unchanged; callers must explicitly use this method
    /// when disposing of their own arbitrarily deep expression trees.
    pub fn drop_iterative(self) {
        let mut pending = vec![self];
        while let Some(expression) = pending.pop() {
            match expression.kind {
                ExpressionKind::Binary { left, right, .. } => {
                    pending.push(*left);
                    pending.push(*right);
                }
                ExpressionKind::Call { callee, arguments } => {
                    pending.push(*callee);
                    pending.extend(arguments);
                }
                _ => {}
            }
        }
    }
}

impl Clone for Expression {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Expression),
            Finish(&'a Expression),
        }
        let mut pending = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(work) = pending.pop() {
            match work {
                Work::Visit(expression) => match &expression.kind {
                    ExpressionKind::Binary { left, right, .. } => {
                        pending.push(Work::Finish(expression));
                        pending.push(Work::Visit(right));
                        pending.push(Work::Visit(left));
                    }
                    ExpressionKind::Call { callee, arguments } => {
                        pending.push(Work::Finish(expression));
                        pending.extend(arguments.iter().rev().map(Work::Visit));
                        pending.push(Work::Visit(callee));
                    }
                    _ => completed.push(Self {
                        kind: expression.kind.clone(),
                        span: expression.span,
                    }),
                },
                Work::Finish(expression) => {
                    let kind = match &expression.kind {
                        ExpressionKind::Binary { operator, .. } => {
                            let right = completed.pop().expect("right clone completed");
                            let left = completed.pop().expect("left clone completed");
                            ExpressionKind::Binary {
                                left: Box::new(left),
                                operator: operator.clone(),
                                right: Box::new(right),
                            }
                        }
                        ExpressionKind::Call { arguments, .. } => {
                            let arguments = completed.split_off(completed.len() - arguments.len());
                            let callee = completed.pop().expect("callee clone completed");
                            ExpressionKind::Call {
                                callee: Box::new(callee),
                                arguments,
                            }
                        }
                        _ => unreachable!("only composite expressions need finishing"),
                    };
                    completed.push(Self {
                        kind,
                        span: expression.span,
                    });
                }
            }
        }
        completed.pop().expect("root clone completed")
    }
}

/// The structure of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    /// A string literal.
    String(String),
    /// An integer literal.
    Integer(String),
    /// A floating-point literal.
    Float(String),
    /// A boolean literal.
    Boolean(bool),
    /// A variable reference.
    Identifier(String),
    /// A module-qualified function or value reference.
    QualifiedName {
        /// Path segments.
        path: Vec<String>,
    },
    /// A function call.
    Call {
        /// Expression resolving to the called function.
        callee: Box<Expression>,
        /// Arguments passed to the function.
        arguments: Vec<Expression>,
    },
    /// A binary operator expression.
    Binary {
        /// Left operand.
        left: Box<Expression>,
        /// Operator spelling.
        operator: String,
        /// Right operand.
        right: Box<Expression>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterative_clone_preserves_structure_and_spans() {
        let program = crate::compiler::parser::Parser::new().parse_source(
            "fn main() { let x = f(1 + 2, std::len(\"λ\"), true, 1.5); return; } mod m { fn helper() { 3; } }"
        ).unwrap();
        let cloned = program.clone();
        assert_eq!(cloned, program);
        program.drop_iterative();
        cloned.drop_iterative();
    }

    #[test]
    fn deep_caller_ast_clones_and_disposes_iteratively() {
        let span = Span {
            start: 1,
            end: 2,
            line: 0,
            column: 1,
        };
        let leaf = || Expression {
            kind: ExpressionKind::Integer("1".into()),
            span,
        };
        let mut expression = leaf();
        for index in 0..10_000 {
            expression = Expression {
                kind: if index % 2 == 0 {
                    ExpressionKind::Binary {
                        left: Box::new(expression),
                        operator: "+".into(),
                        right: Box::new(leaf()),
                    }
                } else {
                    ExpressionKind::Call {
                        callee: Box::new(leaf()),
                        arguments: vec![expression, leaf()],
                    }
                },
                span,
            };
        }
        let mut program = crate::compiler::parser::Parser::new()
            .parse_source("fn main() {}")
            .unwrap();
        program.functions[0].body.push(Statement::Return {
            value: Some(expression),
            span,
        });
        let cloned = program.clone();
        let Statement::Return {
            value: Some(root), ..
        } = &cloned.functions[0].body[0]
        else {
            unreachable!()
        };
        let mut pending = vec![root];
        let mut composite_count = 0;
        while let Some(expression) = pending.pop() {
            assert_eq!(expression.span, span);
            match &expression.kind {
                ExpressionKind::Binary { left, right, .. } => {
                    composite_count += 1;
                    pending.push(left);
                    pending.push(right);
                }
                ExpressionKind::Call { callee, arguments } => {
                    composite_count += 1;
                    pending.push(callee);
                    pending.extend(arguments);
                }
                _ => {}
            }
        }
        program.drop_iterative();
        cloned.drop_iterative();
        assert_eq!(composite_count, 10_000);
    }
}
