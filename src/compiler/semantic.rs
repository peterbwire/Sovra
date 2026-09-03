//! Name resolution and basic type checking for M3.

use std::collections::HashMap;

use crate::compiler::ast::{Expression, Function, Program, Statement};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};

/// The types understood by the initial semantic checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// No value.
    Unit,
    /// Boolean value.
    Bool,
    /// Integer value.
    Int,
    /// Floating-point value.
    Float,
    /// UTF-8 string value.
    String,
    /// A named type reserved for future declarations.
    Named(String),
    /// An unresolved or invalid type.
    Unknown,
}

/// Result of successful semantic analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    /// The validated source program.
    pub program: Program,
}

/// Semantic analyzer.
#[derive(Debug, Default)]
pub struct SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Construct an analyzer.
    pub const fn new() -> Self {
        Self
    }

    /// Resolve names and validate the M2 AST.
    pub fn analyze(&self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        let functions: HashMap<String, &Function> = program
            .functions
            .iter()
            .map(|function| (function.name.clone(), function))
            .collect();
        let mut diagnostics = Diagnostics::new();
        for function in &program.functions {
            let mut scope: HashMap<String, Type> = HashMap::new();
            for parameter in &function.parameters {
                let parameter_type = parameter
                    .type_name
                    .as_deref()
                    .map(type_from_name)
                    .unwrap_or(Type::Unknown);
                scope.insert(parameter.name.clone(), parameter_type);
            }
            let expected_return = function
                .return_type
                .as_deref()
                .map(type_from_name)
                .unwrap_or(Type::Unit);
            for statement in &function.body {
                check_statement(
                    statement,
                    &mut scope,
                    &functions,
                    &expected_return,
                    &mut diagnostics,
                );
            }
        }
        if diagnostics.is_empty() {
            Ok(TypedProgram {
                program: program.clone(),
            })
        } else {
            Err(diagnostics)
        }
    }
}

fn check_statement(
    statement: &Statement,
    scope: &mut HashMap<String, Type>,
    functions: &HashMap<String, &Function>,
    expected_return: &Type,
    diagnostics: &mut Diagnostics,
) {
    match statement {
        Statement::Let { name, value, span } => {
            let value_type = check_expression(value, scope, functions, diagnostics, *span);
            scope.insert(name.clone(), value_type);
        }
        Statement::Return { value, span } => {
            let actual = value
                .as_ref()
                .map(|expression| {
                    check_expression(expression, scope, functions, diagnostics, *span)
                })
                .unwrap_or(Type::Unit);
            if !types_compatible(expected_return, &actual) {
                diagnostics.push(diagnostic(
                    "E3002",
                    format!("return type mismatch: expected {expected_return:?}, found {actual:?}"),
                    *span,
                ));
            }
        }
        Statement::Expression(expression) => {
            check_expression(
                expression,
                scope,
                functions,
                diagnostics,
                Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
            );
        }
    }
}

fn check_expression(
    expression: &Expression,
    scope: &HashMap<String, Type>,
    functions: &HashMap<String, &Function>,
    diagnostics: &mut Diagnostics,
    span: Span,
) -> Type {
    match expression {
        Expression::String(_) => Type::String,
        Expression::Integer(_) => Type::Int,
        Expression::Float(_) => Type::Float,
        Expression::Boolean(_) => Type::Bool,
        Expression::Identifier(name) => scope.get(name).cloned().unwrap_or_else(|| {
            diagnostics.push(diagnostic(
                "E3001",
                format!("undefined variable `{name}`"),
                span,
            ));
            Type::Unknown
        }),
        Expression::Call { callee, arguments } => {
            let name = match callee.as_ref() {
                Expression::Identifier(name) => name,
                _ => {
                    diagnostics.push(diagnostic(
                        "E3003",
                        "call target must be a function name",
                        span,
                    ));
                    return Type::Unknown;
                }
            };
            for argument in arguments {
                check_expression(argument, scope, functions, diagnostics, span);
            }
            if name == "print" {
                return Type::Unit;
            }
            if let Some(function) = functions.get(name) {
                function
                    .return_type
                    .as_deref()
                    .map(type_from_name)
                    .unwrap_or(Type::Unit)
            } else {
                diagnostics.push(diagnostic(
                    "E3004",
                    format!("undefined function `{name}`"),
                    span,
                ));
                Type::Unknown
            }
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let left_type = check_expression(left, scope, functions, diagnostics, span);
            let right_type = check_expression(right, scope, functions, diagnostics, span);
            if matches!(operator.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") {
                Type::Bool
            } else if types_compatible(&left_type, &right_type)
                && matches!(left_type, Type::Int | Type::Float | Type::String)
            {
                left_type
            } else {
                diagnostics.push(diagnostic(
                    "E3005",
                    format!("operator `{operator}` cannot be applied to these types"),
                    span,
                ));
                Type::Unknown
            }
        }
    }
}

fn type_from_name(name: &str) -> Type {
    match name {
        "Unit" => Type::Unit,
        "Bool" => Type::Bool,
        "Int" => Type::Int,
        "Float" => Type::Float,
        "String" => Type::String,
        _ => Type::Named(name.to_owned()),
    }
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual || matches!(actual, Type::Unknown) || matches!(expected, Type::Unknown)
}

fn diagnostic(code: &'static str, message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code,
        message: message.into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parser::Parser;

    #[test]
    fn resolves_bindings_and_builtin_print() {
        let program = Parser::new()
            .parse_source("fn main() { let message = \"hi\"; print(message) }")
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn reports_undefined_names() {
        let program = Parser::new()
            .parse_source("fn main() { print(missing) }")
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3001"));
    }

    #[test]
    fn checks_return_types() {
        let program = Parser::new()
            .parse_source("fn main() -> Int { return \"no\" }")
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3002"));
    }
}
