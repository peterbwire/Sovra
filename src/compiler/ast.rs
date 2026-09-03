//! Abstract syntax tree produced by the M2 parser.

use crate::compiler::diagnostics::Span;

/// A complete Sovra source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level function declarations.
    pub functions: Vec<Function>,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Function name.
    pub name: String,
    /// Function parameters.
    pub parameters: Vec<Parameter>,
    /// Optional declared return type.
    pub return_type: Option<String>,
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
    /// Optional parameter type.
    pub type_name: Option<String>,
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

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
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
