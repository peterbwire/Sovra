//! Name resolution and basic type checking for M3.

use std::collections::HashMap;

use crate::compiler::ast::{Expression, ExpressionKind, Function, Program, Statement};
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
            check_function_body(function, &functions, &mut diagnostics);
        }
        for module in &program.modules {
            for function in &module.functions {
                check_function_body(function, &functions, &mut diagnostics);
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

fn check_function_body(
    function: &Function,
    functions: &HashMap<String, &Function>,
    diagnostics: &mut Diagnostics,
) {
    let mut scope: HashMap<String, Type> = HashMap::new();
    for parameter in &function.parameters {
        if scope.contains_key(&parameter.name) {
            diagnostics.push(diagnostic(
                "E3011",
                format!("duplicate parameter `{}`", parameter.name),
                parameter.span,
            ));
        }
        let parameter_type = match parameter.type_name.as_deref() {
            Some(type_name) => type_from_name(type_name),
            None => {
                diagnostics.push(diagnostic(
                    "E3014",
                    format!(
                        "parameter `{name}` requires an explicit type annotation; write `{name}: Type`",
                        name = parameter.name
                    ),
                    parameter.span,
                ));
                // Unknown is error recovery, not parameter type inference.
                Type::Unknown
            }
        };
        scope.insert(parameter.name.clone(), parameter_type);
    }
    let expected_return = function
        .return_type
        .as_deref()
        .map(type_from_name)
        .unwrap_or(Type::Unit);
    // The current AST has only straight-line statements. Branches and loops
    // will require control-flow-aware return analysis when they are introduced.
    if expected_return != Type::Unit
        && !function
            .body
            .iter()
            .any(|statement| matches!(statement, Statement::Return { .. }))
    {
        diagnostics.push(diagnostic(
            "E3013",
            format!(
                "function `{}` must return {expected_return:?}",
                function.name
            ),
            function.span,
        ));
    }
    for statement in &function.body {
        check_statement(
            statement,
            &mut scope,
            functions,
            &expected_return,
            diagnostics,
        );
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
            let value_type = check_expression(value, scope, functions, diagnostics);
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
                .map(|expression| check_expression(expression, scope, functions, diagnostics))
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
            check_expression(expression, scope, functions, diagnostics);
        }
    }
}

fn check_expression(
    expression: &Expression,
    scope: &HashMap<String, Type>,
    functions: &HashMap<String, &Function>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let span = expression.span;
    match &expression.kind {
        ExpressionKind::String(_) => Type::String,
        ExpressionKind::Integer(value) => {
            if value.parse::<i64>().is_err() {
                diagnostics.push(diagnostic(
                    "E3012",
                    "integer literal is outside the signed 64-bit range",
                    span,
                ));
            }
            Type::Int
        }
        ExpressionKind::Float(_) => Type::Float,
        ExpressionKind::Boolean(_) => Type::Bool,
        ExpressionKind::Identifier(name) => scope.get(name).cloned().unwrap_or_else(|| {
            diagnostics.push(diagnostic(
                "E3001",
                format!("undefined variable `{name}`"),
                span,
            ));
            Type::Unknown
        }),
        ExpressionKind::QualifiedName { path } => {
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
        ExpressionKind::Call { callee, arguments } => {
            let name = match &callee.kind {
                ExpressionKind::Identifier(name) => Some(name.clone()),
                ExpressionKind::QualifiedName { path } => Some(path.join("::")),
                _ => {
                    diagnostics.push(diagnostic(
                        "E3003",
                        "call target must be a function name",
                        callee.span,
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
                    let argument_type = check_expression(argument, scope, functions, diagnostics);
                    if let Some(parameter_type) = parameter.type_name.as_deref() {
                        let expected = type_from_name(parameter_type);
                        if !types_compatible(&expected, &argument_type) {
                            diagnostics.push(diagnostic(
                                "E3007",
                                format!(
                                    "argument type mismatch for `{name}`: expected {expected:?}, found {argument_type:?}"
                                ),
                                argument.span,
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
                    check_expression(argument, scope, functions, diagnostics);
                }
                diagnostics.push(diagnostic(
                    "E3004",
                    format!("undefined function `{name}`"),
                    callee.span,
                ));
                Type::Unknown
            }
        }
        ExpressionKind::Binary {
            left,
            operator,
            right,
        } => {
            let left_type = check_expression(left, scope, functions, diagnostics);
            let right_type = check_expression(right, scope, functions, diagnostics);
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
                arithmetic_result_type(&left_type, &right_type)
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
        let argument_type = check_expression(argument, scope, functions, diagnostics);
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
                argument.span,
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

fn arithmetic_result_type(left: &Type, right: &Type) -> Type {
    match (left, right) {
        (Type::String, Type::String) => Type::String,
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
        source_file: None,
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
    fn concatenation_preserves_string_type_contracts() {
        for (source, code) in [
            ("fn main() { let number: Int = \"a\" + \"b\" }", "E3002"),
            ("fn value() -> Int { return \"a\" + \"b\" }", "E3002"),
            (
                "fn take(value: Int) {} fn main() { take(\"a\" + \"b\") }",
                "E3007",
            ),
            (
                "fn main() { let text = \"a\" + \"b\"; let number: Int = text }",
                "E3002",
            ),
        ] {
            let program = Parser::new().parse_source(source).unwrap();
            let errors = SemanticAnalyzer::new()
                .analyze(&program)
                .expect_err("concatenation is String");
            assert!(
                errors.items.iter().any(|error| error.code == code),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn inferred_concatenation_can_be_used_as_string() {
        let source = "fn join(value: String) -> String { return value + \"!\" } fn main() { let text = \"a\" + \"b\"; print(join(text + \"c\")); print(std::len(text)); print(text == \"ab\") }";
        let program = Parser::new().parse_source(source).unwrap();
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("inferred String");
        let output =
            crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed)).unwrap();
        assert_eq!(output, vec!["abc!", "2", "true"]);
    }

    #[test]
    fn expression_diagnostics_identify_the_failing_source() {
        for (body, code, spelling) in [
            ("print(missing)", "E3001", "missing"),
            ("let value = missing", "E3001", "missing"),
            ("return missing", "E3001", "missing"),
            ("print(9223372036854775808)", "E3012", "9223372036854775808"),
            ("print(true + 1)", "E3005", "true + 1"),
            ("std::len(42)", "E3007", "42"),
            ("helper(42)", "E3007", "42"),
            ("absent(1)", "E3004", "absent"),
            ("std::absent(1)", "E3004", "std::absent"),
            ("std::len()", "E3006", "std::len()"),
            ("42()", "E3003", "42"),
        ] {
            for prefix in ["fn main() {", "mod sample { fn private() {"] {
                let source = format!(
                    "fn helper(value: String) {{}}\n{prefix}\n  print(\"é\"); {body}\n}}{}",
                    if prefix.starts_with("mod") { "}" } else { "" }
                );
                let program = Parser::new().parse_source(&source).expect("valid syntax");
                let errors = SemanticAnalyzer::new()
                    .analyze(&program)
                    .expect_err("invalid body");
                let error = errors
                    .items
                    .iter()
                    .find(|error| error.code == code)
                    .unwrap();
                let start = source.rfind(spelling).unwrap();
                assert_eq!(error.span.start, start, "{source}: {error:?}");
                assert_eq!(error.span.end, start + spelling.len(), "{source}");
                assert_eq!(error.span.line, 2);
                let line_start = source[..start].rfind('\n').unwrap() + 1;
                assert_eq!(error.span.column, source[line_start..start].chars().count());
            }
        }
    }

    #[test]
    fn requires_parameter_annotations_in_all_functions() {
        for source in [
            "fn helper(value) {} fn main() {}",
            "export fn helper(value) {} fn main() {}",
            "mod sample { fn helper(value) {} } fn main() {}",
            "mod sample { export fn helper(value) {} } fn main() {}",
            include_str!("../../tests/fixtures/untyped-parameter.svr"),
        ] {
            let parsed = Parser::new()
                .parse_source(source)
                .expect("recoverable syntax");
            let diagnostics = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("every function parameter requires a type");
            let missing_types: Vec<_> = diagnostics
                .items
                .iter()
                .filter(|diagnostic| diagnostic.code == "E3014")
                .collect();
            assert_eq!(missing_types.len(), 1, "{source}: {diagnostics:?}");
            assert_eq!(
                missing_types[0].message,
                "parameter `value` requires an explicit type annotation; write `value: Type`"
            );
            let span = missing_types[0].span;
            assert_eq!(&source[span.start..span.end], "value");
        }
    }

    #[test]
    fn reports_each_missing_parameter_annotation() {
        let source = "fn helper(first, typed: Int,\n    last) {} fn main() {}";
        let parsed = Parser::new()
            .parse_source(source)
            .expect("recoverable syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&parsed)
            .expect_err("missing types");
        let missing: Vec<_> = diagnostics
            .items
            .iter()
            .filter(|diagnostic| diagnostic.code == "E3014")
            .map(|diagnostic| {
                let span = diagnostic.span;
                (&source[span.start..span.end], span.line, span.column)
            })
            .collect();
        assert_eq!(missing, vec![("first", 0, 10), ("last", 1, 4)]);
    }

    #[test]
    fn typed_parameters_preserve_local_inference() {
        let parsed = Parser::new()
            .parse_source(include_str!("../../examples/functions/main.svr"))
            .expect("valid function example");
        let typed = SemanticAnalyzer::new()
            .analyze(&parsed)
            .expect("inferred locals");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("function example executes");
        assert_eq!(output, vec!["5"]);

        let invalid = Parser::new()
            .parse_source(
                "fn length(value: String) -> Int { return std::len(value) }
                fn main() { let number = 42; length(number) }",
            )
            .expect("valid syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&invalid)
            .expect_err("wrong argument type");
        assert!(diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == "E3007"));
        assert!(!diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == "E3014"));
    }

    #[test]
    fn rejects_missing_required_return() {
        for source in [
            "fn value() -> Int {} fn main() {}",
            "fn value() -> String { let result = \"hi\" } fn main() {}",
            "mod values { export fn value() -> Float {} } fn main() {}",
            "mod values { fn value() -> Int { print(42) } } fn main() {}",
        ] {
            let parsed = Parser::new().parse_source(source).expect("valid syntax");
            let errors = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("value-returning function cannot fall through");
            assert!(errors.items.iter().any(|error| error.code == "E3013"));
        }
        let parsed = Parser::new()
            .parse_source("fn noop() -> Unit {} fn value() -> Int { return 42 } fn main() {}")
            .expect("valid syntax");
        assert!(SemanticAnalyzer::new().analyze(&parsed).is_ok());
    }

    #[test]
    fn numeric_integer_literals_must_fit_i64() {
        for source in [
            "fn main() { print(9223372036854775808) }",
            "mod values { export fn bad() -> Int { return 99999999999999999999 } } fn main() {}",
        ] {
            let parsed = Parser::new().parse_source(source).expect("valid syntax");
            let errors = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("out-of-range literals must be diagnosed");
            assert!(errors.items.iter().any(|error| error.code == "E3012"));
        }
    }

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
    fn rejects_invalid_module_function_bodies() {
        let cases = [
            ("fn bad() { print(missing) }", "E3001"),
            ("fn bad() -> Int { return \"wrong\" }", "E3002"),
            ("fn bad(value: Int, value: Int) {}", "E3011"),
            ("fn bad() { let value: Int = \"wrong\" }", "E3002"),
            ("fn bad() { missing() }", "E3004"),
            ("fn bad() { std::len() }", "E3006"),
            ("fn bad() { std::len(42) }", "E3007"),
            ("fn bad() { print(true + 1) }", "E3005"),
        ];
        for visibility in ["", "export "] {
            for (function, code) in cases {
                let source = format!("mod sample {{ {visibility}{function} }} fn main() {{}}");
                let program = Parser::new().parse_source(&source).expect("valid syntax");
                let diagnostics = SemanticAnalyzer::new()
                    .analyze(&program)
                    .expect_err(&source);
                assert!(
                    diagnostics.items.iter().any(|item| item.code == code),
                    "expected {code} for {source}: {diagnostics:?}"
                );
            }
        }
    }

    #[test]
    fn module_functions_have_independent_local_scopes() {
        let program = Parser::new()
            .parse_source(
                "mod sample { export fn first(value: Int) -> Int { return value }
                 export fn second() -> Int { return value } } fn main() {}",
            )
            .expect("valid syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("parameters must not leak into another function");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3001"));
    }

    #[test]
    fn module_main_is_an_ordinary_function() {
        let program = Parser::new()
            .parse_source(
                "mod sample { export fn main(value: Int) -> Int { return value } }
                 fn main() { print(sample::main(42)) }",
            )
            .expect("valid syntax");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("entry restrictions apply only to top-level main");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("exported function should execute");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn module_example_checks_and_executes() {
        let program = Parser::new()
            .parse_source(include_str!("../../examples/modules/main.svr"))
            .expect("module example should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("module example should check");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("module example should execute");
        assert_eq!(output, vec!["42"]);
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
