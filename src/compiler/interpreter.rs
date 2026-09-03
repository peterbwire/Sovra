//! Minimal stack-based interpreter for the M3 IR.

use std::collections::HashMap;

use crate::compiler::ir::{Instruction, IrFunction, IrProgram};

const MAX_CALL_DEPTH: usize = 256;

/// Runtime values supported by the M4 interpreter.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer.
    Int(i64),
    /// A floating-point number.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A string.
    String(String),
    /// No value.
    Unit,
}

impl Value {
    fn display(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Unit => String::new(),
        }
    }
}

/// Execute the `main` function and return captured `print` output.
pub fn run(program: &IrProgram) -> Result<Vec<String>, String> {
    let function = program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or_else(|| "entry function `main` was not found".to_owned())?;
    let (output, _) = execute_function(function, &[], program, 0)?;
    Ok(output)
}

fn execute_function(
    function: &IrFunction,
    arguments: &[Value],
    program: &IrProgram,
    depth: usize,
) -> Result<(Vec<String>, Value), String> {
    if depth >= MAX_CALL_DEPTH {
        return Err(format!(
            "maximum call depth of {MAX_CALL_DEPTH} exceeded in `{}`",
            function.name
        ));
    }
    if arguments.len() != function.parameters.len() {
        return Err(format!(
            "function `{}` expects {} argument(s), found {}",
            function.name,
            function.parameters.len(),
            arguments.len()
        ));
    }
    let mut stack = Vec::new();
    let mut names = HashMap::new();
    for (name, value) in function.parameters.iter().zip(arguments) {
        names.insert(name.clone(), value.clone());
    }
    let mut output = Vec::new();
    for instruction in &function.instructions {
        match instruction {
            Instruction::LoadLiteral(value) => stack.push(parse_literal(value)),
            Instruction::LoadName(name) => stack.push(
                names
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("runtime name `{name}` was not found"))?,
            ),
            Instruction::StoreName(name) => {
                let value = stack
                    .pop()
                    .ok_or_else(|| "stack underflow on store".to_owned())?;
                names.insert(name.clone(), value);
            }
            Instruction::Binary(operator) => {
                let right = stack
                    .pop()
                    .ok_or_else(|| "stack underflow on binary operator".to_owned())?;
                let left = stack
                    .pop()
                    .ok_or_else(|| "stack underflow on binary operator".to_owned())?;
                stack.push(binary(operator, left, right)?);
            }
            Instruction::Call { name, arguments } => {
                let mut call_arguments = Vec::with_capacity(*arguments);
                for _ in 0..*arguments {
                    call_arguments.push(
                        stack
                            .pop()
                            .ok_or_else(|| "stack underflow on call".to_owned())?,
                    );
                }
                call_arguments.reverse();
                if matches!(name.as_str(), "print" | "std::print") {
                    if *arguments != 1 {
                        return Err(format!("{name} expects exactly one argument"));
                    }
                    output.push(call_arguments[0].display());
                    stack.push(Value::Unit);
                } else if name == "std::println" {
                    if *arguments != 1 {
                        return Err(format!("{name} expects exactly one argument"));
                    }
                    output.push(call_arguments[0].display());
                    stack.push(Value::Unit);
                } else if name == "std::len" {
                    if *arguments != 1 {
                        return Err(format!("{name} expects exactly one argument"));
                    }
                    let value = match &call_arguments[0] {
                        Value::String(value) => Value::Int(value.len() as i64),
                        _ => return Err(format!("{name} expects a String argument")),
                    };
                    stack.push(value);
                } else if name == "std::to_string" {
                    if *arguments != 1 {
                        return Err(format!("{name} expects exactly one argument"));
                    }
                    let value = match &call_arguments[0] {
                        Value::Int(value) => Value::String(value.to_string()),
                        Value::Float(value) => Value::String(value.to_string()),
                        Value::Bool(value) => Value::String(value.to_string()),
                        Value::String(value) => Value::String(value.clone()),
                        Value::Unit => Value::String(String::new()),
                    };
                    stack.push(value);
                } else {
                    let callee = program
                        .functions
                        .iter()
                        .find(|function| function.name == name.as_str())
                        .ok_or_else(|| format!("runtime function `{name}` was not found"))?;
                    let (callee_output, return_value) =
                        execute_function(callee, &call_arguments, program, depth + 1)?;
                    output.extend(callee_output);
                    stack.push(return_value);
                }
            }
            Instruction::Pop => {
                stack
                    .pop()
                    .ok_or_else(|| "stack underflow on pop".to_owned())?;
            }
            Instruction::Return => {
                return Ok((output, stack.pop().unwrap_or(Value::Unit)));
            }
        }
    }
    Ok((output, stack.pop().unwrap_or(Value::Unit)))
}

fn parse_literal(value: &str) -> Value {
    if value == "true" {
        Value::Bool(true)
    } else if value == "false" {
        Value::Bool(false)
    } else if let Ok(integer) = value.parse() {
        Value::Int(integer)
    } else if let Ok(float) = value.parse() {
        Value::Float(float)
    } else {
        Value::String(value.to_owned())
    }
}

fn binary(operator: &str, left: Value, right: Value) -> Result<Value, String> {
    if operator == "/" {
        let zero = match &right {
            Value::Int(value) => *value == 0,
            Value::Float(value) => *value == 0.0,
            _ => false,
        };
        if zero {
            return Err("division by zero".to_owned());
        }
    }
    match (operator, left, right) {
        ("+", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left + right)),
        ("-", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left - right)),
        ("*", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left * right)),
        ("/", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left / right)),
        ("+", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
        ("-", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left - right)),
        ("*", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left * right)),
        ("/", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
        ("+", Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
        ("==", left, right) => Ok(Value::Bool(left == right)),
        ("!=", left, right) => Ok(Value::Bool(left != right)),
        ("<", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left < right)),
        ("<=", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left <= right)),
        (">", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left > right)),
        (">=", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left >= right)),
        ("<", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left < right)),
        ("<=", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left <= right)),
        (">", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left > right)),
        (">=", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left >= right)),
        ("<", Value::String(left), Value::String(right)) => Ok(Value::Bool(left < right)),
        ("<=", Value::String(left), Value::String(right)) => Ok(Value::Bool(left <= right)),
        (">", Value::String(left), Value::String(right)) => Ok(Value::Bool(left > right)),
        (">=", Value::String(left), Value::String(right)) => Ok(Value::Bool(left >= right)),
        _ => Err(format!("unsupported runtime operation `{operator}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{ir, parser::Parser, semantic::SemanticAnalyzer};

    #[test]
    fn executes_print_program() {
        let program = Parser::new()
            .parse_source("fn main() { print(\"Hello, Sovra!\") }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["Hello, Sovra!"]);
    }

    #[test]
    fn executes_std_library_helpers() {
        let program = Parser::new()
            .parse_source("fn main() { let text = std::to_string(42); print(std::len(text)); std::println(text) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["2", "42"]);
    }

    #[test]
    fn executes_user_function_and_return_value() {
        let program = Parser::new()
            .parse_source(
                "fn add(left: Int, right: Int) -> Int { return left + right } \
                 fn main() { print(add(2, 3)) }",
            )
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["5"]);
    }

    #[test]
    fn executes_float_and_comparison_operations() {
        let program = Parser::new()
            .parse_source("fn main() { print(1.5 + 2.5); print(3 > 2) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["4", "true"]);
    }

    #[test]
    fn reports_division_by_zero() {
        let program = Parser::new()
            .parse_source("fn main() { print(1 / 0) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let error = run(&ir::lower(&typed)).expect_err("program should fail");
        assert_eq!(error, "division by zero");
    }

    #[test]
    fn reports_excessive_call_depth() {
        let program = IrProgram {
            functions: vec![
                IrFunction {
                    name: "main".into(),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::Call {
                        name: "loop".into(),
                        arguments: 0,
                    }],
                },
                IrFunction {
                    name: "loop".into(),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::Call {
                        name: "loop".into(),
                        arguments: 0,
                    }],
                },
            ],
        };
        let error = run(&program).expect_err("recursive program should fail");
        assert!(error.contains("maximum call depth"));
    }
}
