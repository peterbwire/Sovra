//! Text backend for inspecting M4 IR output.

use std::fmt::Write;

use crate::compiler::ir::{Instruction, IrProgram};

/// Render IR as a stable human-readable build artifact.
pub fn render(program: &IrProgram) -> String {
    let mut output = String::new();
    for function in &program.functions {
        let _ = writeln!(output, "function {}:", function.name);
        for instruction in &function.instructions {
            let _ = writeln!(output, "  {}", instruction_text(instruction));
        }
    }
    output
}

fn instruction_text(instruction: &Instruction) -> String {
    match instruction {
        Instruction::LoadLiteral(value) => format!("load {value:?}"),
        Instruction::LoadName(name) => format!("load-name {name}"),
        Instruction::StoreName(name) => format!("store-name {name}"),
        Instruction::Binary(operator) => format!("binary {operator}"),
        Instruction::Call { name, arguments } => format!("call {name} {arguments}"),
        Instruction::Return => "return".to_owned(),
        Instruction::Pop => "pop".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_function_header() {
        let output = render(&IrProgram {
            functions: vec![crate::compiler::ir::IrFunction {
                name: "main".into(),
                parameters: Vec::new(),
                instructions: vec![Instruction::Return],
            }],
        });
        assert!(output.contains("function main:"));
        assert!(output.contains("return"));
    }
}
