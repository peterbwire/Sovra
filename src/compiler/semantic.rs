//! Name resolution and basic type checking for M3.

use std::collections::HashMap;

use crate::compiler::ast::{Expression, Function, Program, Statement};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::compiler::stdlib;

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
        let mut diagnostics = Diagnostics::new();
        let mut declared_functions = HashMap::new();
        for function in &program.functions {
            if declared_functions
                .insert(function.name.clone(), function.span)
                .is_some()
            {
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate function `{}`", function.name),
                    function.span,
                ));
            }
        }
        let mut module_names = HashMap::new();
        for module in &program.modules {
            let mut seen = HashMap::new();
            for function in &module.functions {
                if function.is_exported
                    && seen.insert(function.name.clone(), function.span).is_some()
                {
                    diagnostics.push(diagnostic(
                        "E3008",
                        format!(
                            "duplicate function `{}` in module `{}`",
                            function.name, module.name
                        ),
                        function.span,
                    ));
                }
            }
            if module_names
                .insert(module.name.clone(), module.span)
                .is_some()
            {
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate module `{}`", module.name),
                    module.span,
                ));
            }
        }
        let mut functions: HashMap<String, &Function> = program
            .functions
            .iter()
            .map(|function| (function.name.clone(), function))
            .collect();
        for module in &program.modules {
            for function in &module.functions {
                if function.is_exported {
                    functions.insert(format!("{}::{}", module.name, function.name), function);
                }
            }
        }
        for function in &program.functions {
            if function.name == "main" && !function.parameters.is_empty() {
                diagnostics.push(diagnostic(
                    "E3009",
                    "entry function `main` cannot declare parameters",
                    function.span,
                ));
            }
            if function.name == "main"
                && function
                    .return_type
                    .as_deref()
                    .is_some_and(|return_type| type_from_name(return_type) != Type::Unit)
            {
                diagnostics.push(diagnostic(
                    "E3010",
                    "entry function `main` must return Unit",
                    function.span,
                ));
            }
            let mut scope: HashMap<String, Type> = HashMap::new();
            for parameter in &function.parameters {
                if scope.contains_key(&parameter.name) {
                    diagnostics.push(diagnostic(
                        "E3011",
                        format!("duplicate parameter `{}`", parameter.name),
                        parameter.span,
                    ));
                }
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
        Statement::Let {
            name,
            type_name,
            value,
            span,
        } => {
            let value_type = check_expression(value, scope, functions, diagnostics, *span);
            let declared_type = type_name
                .as_deref()
                .map(type_from_name)
                .unwrap_or(value_type.clone());
            if !types_compatible(&declared_type, &value_type) {
                diagnostics.push(diagnostic(
                    "E3002",
                    format!(
                        "binding type mismatch for `{name}`: expected {declared_type:?}, found {value_type:?}"
                    ),
                    *span,
                ));
            }
            scope.insert(name.clone(), declared_type);
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
        Expression::QualifiedName { path } => {
            let qualified = path.join("::");
            if let Some(function) = stdlib::lookup(&qualified) {
                type_from_name(function.return_type)
            } else if let Some(function) = functions.get(&qualified) {
                function
                    .return_type
                    .as_deref()
                    .map(type_from_name)
                    .unwrap_or(Type::Unit)
            } else {
                diagnostics.push(diagnostic(
                    "E3004",
                    format!("undefined function `{qualified}`"),
                    span,
                ));
                Type::Unknown
            }
        }
        Expression::Call { callee, arguments } => {
            let name = match callee.as_ref() {
                Expression::Identifier(name) => Some(name.clone()),
                Expression::QualifiedName { path } => Some(path.join("::")),
                _ => {
                    diagnostics.push(diagnostic(
                        "E3003",
                        "call target must be a function name",
                        span,
                    ));
                    return Type::Unknown;
                }
            };
            let Some(name) = name else {
                return Type::Unknown;
            };
            if let Some(function) = stdlib::lookup(&name) {
                check_std_call(
                    function,
                    &name,
                    arguments,
                    scope,
                    functions,
                    diagnostics,
                    span,
                );
                return type_from_name(function.return_type);
            }
            if let Some(function) = functions.get(&name) {
                if arguments.len() != function.parameters.len() {
                    diagnostics.push(diagnostic(
                        "E3006",
                        format!(
                            "function `{name}` expects {} argument(s), found {}",
                            function.parameters.len(),
                            arguments.len()
                        ),
                        span,
                    ));
                }
                for (argument, parameter) in arguments.iter().zip(&function.parameters) {
                    let argument_type =
                        check_expression(argument, scope, functions, diagnostics, span);
                    if let Some(parameter_type) = parameter.type_name.as_deref() {
                        let expected = type_from_name(parameter_type);
                        if !types_compatible(&expected, &argument_type) {
                            diagnostics.push(diagnostic(
                                "E3007",
                                format!(
                                    "argument type mismatch for `{name}`: expected {expected:?}, found {argument_type:?}"
                                ),
                                span,
                            ));
                        }
                    }
                }
                function
                    .return_type
                    .as_deref()
                    .map(type_from_name)
                    .unwrap_or(Type::Unit)
            } else {
                for argument in arguments {
                    check_expression(argument, scope, functions, diagnostics, span);
                }
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
            let comparable = numeric_or_string_comparison_compatible(&left_type, &right_type)
                && match operator.as_str() {
                    "==" | "!=" => {
                        matches!(
                            &left_type,
                            Type::Bool | Type::Int | Type::Float | Type::String
                        ) || matches!(
                            &right_type,
                            Type::Bool | Type::Int | Type::Float | Type::String
                        )
                    }
                    "<" | "<=" | ">" | ">=" => matches!(
                        (&left_type, &right_type),
                        (Type::Int, Type::Int)
                            | (Type::Float, Type::Float)
                            | (Type::Int, Type::Float)
                            | (Type::Float, Type::Int)
                            | (Type::String, Type::String)
                    ),
                    _ => false,
                };
            let arithmetic = numeric_or_string_arithmetic_compatible(&left_type, &right_type)
                && match operator.as_str() {
                    "+" => matches!(
                        (&left_type, &right_type),
                        (Type::Int, Type::Int)
                            | (Type::Float, Type::Float)
                            | (Type::Int, Type::Float)
                            | (Type::Float, Type::Int)
                            | (Type::String, Type::String)
                    ),
                    "-" | "*" | "/" => matches!(
                        (&left_type, &right_type),
                        (Type::Int, Type::Int)
                            | (Type::Float, Type::Float)
                            | (Type::Int, Type::Float)
                            | (Type::Float, Type::Int)
                    ),
                    _ => false,
                };
            if comparable {
                Type::Bool
            } else if arithmetic {
                numeric_result_type(&left_type, &right_type)
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

fn check_std_call(
    function: stdlib::StdFunction,
    source_name: &str,
    arguments: &[Expression],
    scope: &HashMap<String, Type>,
    functions: &HashMap<String, &Function>,
    diagnostics: &mut Diagnostics,
    span: Span,
) {
    if arguments.len() != function.parameters.len() {
        diagnostics.push(diagnostic(
            "E3006",
            format!(
                "function `{source_name}` expects {} argument(s), found {}",
                function.parameters.len(),
                arguments.len()
            ),
            span,
        ));
    }
    for (argument, expected) in arguments.iter().zip(function.parameters) {
        let argument_type = check_expression(argument, scope, functions, diagnostics, span);
        if stdlib::is_any_type(expected) {
            continue;
        }
        let expected = type_from_name(expected);
        if !types_compatible(&expected, &argument_type) {
            diagnostics.push(diagnostic(
                "E3007",
                format!(
                    "argument type mismatch for `{source_name}`: expected {expected:?}, found {argument_type:?}"
                ),
                span,
            ));
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

fn numeric_result_type(left: &Type, right: &Type) -> Type {
    match (left, right) {
        (Type::Float, _) | (_, Type::Float) => Type::Float,
        (Type::Int, Type::Int) => Type::Int,
        _ => Type::Unknown,
    }
}

fn numeric_or_string_arithmetic_compatible(left: &Type, right: &Type) -> bool {
    matches!(
        (left, right),
        (Type::String, Type::String)
            | (Type::Unknown, _)
            | (_, Type::Unknown)
            | (Type::Int, Type::Int)
            | (Type::Int, Type::Float)
            | (Type::Float, Type::Int)
            | (Type::Float, Type::Float)
    )
}

fn numeric_or_string_comparison_compatible(left: &Type, right: &Type) -> bool {
    matches!(
        (left, right),
        (Type::String, Type::String)
            | (Type::Unknown, _)
            | (_, Type::Unknown)
            | (Type::Int, Type::Int)
            | (Type::Int, Type::Float)
            | (Type::Float, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
    )
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    if matches!(actual, Type::Unknown) || matches!(expected, Type::Unknown) {
        return true;
    }
    if expected == actual {
        return true;
    }
    matches!(
        (expected, actual),
        (Type::Float, Type::Int) | (Type::String, Type::String)
    )
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

    #[test]
    fn checks_function_call_arity_and_parameter_types() {
        let program = Parser::new()
            .parse_source("fn add(value: Int) -> Int { return value } fn main() { add(\"no\") }")
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3007"));
    }

    #[test]
    fn accepts_typed_local_bindings_and_numeric_widening() {
        let program = Parser::new()
            .parse_source(
                "fn main() { let scaled: Float = 2; let total = scaled + 3.5; print(total) }",
            )
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn resolves_module_exported_functions() {
        let program = Parser::new()
            .parse_source(
                "mod math { export fn add(a: Int, b: Int) -> Int { return a + b } } fn main() { print(math::add(2, 3)) }",
            )
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn resolves_std_library_calls() {
        let program = Parser::new()
            .parse_source("fn main() { std::println(42); let text = std::to_string(42); print(std::len(text)) }")
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn rejects_duplicate_declarations_and_invalid_main() {
        let program = Parser::new()
            .parse_source(
                "fn helper() {} fn helper() {} \
                 fn main(value: Int, value: Int) -> Int {}",
            )
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3008"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E3009"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E3010"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E3011"));
    }
}
