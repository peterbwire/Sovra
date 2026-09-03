//! Stable, minimal intermediate representation for M3.

use crate::compiler::ast::{Expression, Program, Statement};
use crate::compiler::semantic::TypedProgram;

/// A lowered Sovra program.
#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    /// Lowered functions.
    pub functions: Vec<IrFunction>,
}

/// A lowered function.
#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    /// Function name.
    pub name: String,
    /// Linear instruction sequence.
    pub instructions: Vec<Instruction>,
}

/// M3's backend-neutral instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Load a literal value.
    LoadLiteral(String),
    /// Load a named value.
    LoadName(String),
    /// Store a named value.
    StoreName(String),
    /// Apply an operator.
    Binary(String),
    /// Call a function with an argument count.
    Call {
        /// Function name.
        name: String,
        /// Number of arguments consumed from the value stack.
        arguments: usize,
    },
    /// Return from the current function.
    Return,
    /// Discard the top value.
    Pop,
}

/// Lower a semantically valid program into the minimal IR.
pub fn lower(program: &TypedProgram) -> IrProgram {
    IrProgram {
        functions: program
            .program
            .functions
            .iter()
            .map(lower_function)
            .collect(),
    }
}

fn lower_function(function: &crate::compiler::ast::Function) -> IrFunction {
    let mut instructions = Vec::new();
    for statement in &function.body {
        lower_statement(statement, &mut instructions);
    }
    IrFunction {
        name: function.name.clone(),
        instructions,
    }
}

fn lower_statement(statement: &Statement, instructions: &mut Vec<Instruction>) {
    match statement {
        Statement::Let { name, value, .. } => {
            lower_expression(value, instructions);
            instructions.push(Instruction::StoreName(name.clone()));
        }
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                lower_expression(value, instructions);
            }
            instructions.push(Instruction::Return);
        }
        Statement::Expression(expression) => {
            lower_expression(expression, instructions);
            instructions.push(Instruction::Pop);
        }
    }
}

fn lower_expression(expression: &Expression, instructions: &mut Vec<Instruction>) {
    match expression {
        Expression::String(value) => instructions.push(Instruction::LoadLiteral(value.clone())),
        Expression::Integer(value) => instructions.push(Instruction::LoadLiteral(value.clone())),
        Expression::Float(value) => instructions.push(Instruction::LoadLiteral(value.clone())),
        Expression::Boolean(value) => {
            instructions.push(Instruction::LoadLiteral(value.to_string()))
        }
        Expression::Identifier(name) => instructions.push(Instruction::LoadName(name.clone())),
        Expression::Call { callee, arguments } => {
            for argument in arguments {
                lower_expression(argument, instructions);
            }
            if let Expression::Identifier(name) = callee.as_ref() {
                instructions.push(Instruction::Call {
                    name: name.clone(),
                    arguments: arguments.len(),
                });
            }
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            lower_expression(left, instructions);
            lower_expression(right, instructions);
            instructions.push(Instruction::Binary(operator.clone()));
        }
    }
}

/// Lower a program after semantic analysis.
pub fn lower_program(
    program: &Program,
) -> Result<IrProgram, crate::compiler::diagnostics::Diagnostics> {
    crate::compiler::semantic::SemanticAnalyzer::new()
        .analyze(program)
        .map(|typed| lower(&typed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{parser::Parser, semantic::SemanticAnalyzer};

    #[test]
    fn lowers_bindings_and_calls() {
        let program = Parser::new()
            .parse_source("fn main() { let value = 1 + 2; print(value) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should be valid");
        let ir = lower(&typed);
        assert!(ir.functions[0].instructions.contains(&Instruction::Call {
            name: "print".into(),
            arguments: 1,
        }));
    }
}
