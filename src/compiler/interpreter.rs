//! Minimal stack-based interpreter for the M3 IR.

use std::collections::HashMap;

use crate::compiler::ir::{Instruction, IrFunction, IrProgram};

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
    execute_function(function)
}

fn execute_function(function: &IrFunction) -> Result<Vec<String>, String> {
    let mut stack = Vec::new();
    let mut names = HashMap::new();
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
                if name != "print" {
                    return Err(format!("runtime function `{name}` is not available"));
                }
                if *arguments != 1 {
                    return Err("print expects exactly one argument".to_owned());
                }
                output.push(
                    stack
                        .pop()
                        .ok_or_else(|| "stack underflow on print".to_owned())?
                        .display(),
                );
                stack.push(Value::Unit);
            }
            Instruction::Pop => {
                stack
                    .pop()
                    .ok_or_else(|| "stack underflow on pop".to_owned())?;
            }
            Instruction::Return => break,
        }
    }
    Ok(output)
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
    match (operator, left, right) {
        ("+", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left + right)),
        ("-", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left - right)),
        ("*", Value::Int(left), Value::Int(right)) => Ok(Value::Int(left * right)),
        ("/", Value::Int(left), Value::Int(right)) if right != 0 => Ok(Value::Int(left / right)),
        ("+", Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
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
}
