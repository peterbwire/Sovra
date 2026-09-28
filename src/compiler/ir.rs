//! Stable, minimal intermediate representation for M10 and later backends.

use crate::compiler::ast::{Expression, ExpressionKind, Program, Statement};
use crate::compiler::semantic::TypedProgram;

/// Maximum simultaneously active user-function frames in either execution engine.
pub(super) const MAX_CALL_DEPTH: usize = 256;

/// A lowered Sovra program.
#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    /// Lowered functions.
    pub functions: Vec<IrFunction>,
}

// Public IR can bypass source analysis. Reject ambiguous declarations before
// either engine chooses a function or initializes a parameter environment.
pub(super) fn validate_declarations(program: &IrProgram) -> Result<(), String> {
    let mut functions = std::collections::BTreeSet::new();
    for function in &program.functions {
        if !functions.insert(&function.name) {
            return Err(format!("duplicate IR function `{}`", function.name));
        }
        let mut parameters = std::collections::BTreeSet::new();
        for parameter in &function.parameters {
            if !parameters.insert(parameter) {
                return Err(format!(
                    "duplicate IR parameter `{parameter}` in function `{}`",
                    function.name
                ));
            }
        }
    }
    Ok(())
}

/// A lowered function.
#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    /// Function name.
    pub name: String,
    /// Parameter names, in call order.
    pub parameters: Vec<String>,
    /// Linear instruction sequence.
    pub instructions: Vec<Instruction>,
}

/// A typed literal value in IR.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// An integer literal.
    Integer(String),
    /// A floating-point literal.
    Float(String),
    /// A boolean literal.
    Boolean(bool),
    /// A string literal.
    String(String),
}

/// Backend-neutral instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Load a literal value.
    LoadLiteral(Literal),
    /// Load a named value.
    LoadName(String),
    /// Store a named value.
    StoreName(String),
    /// Convert an Int value to Float, preserving values already of type Float.
    WidenFloat,
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
/// Obtain the input through semantic analysis (or use [`lower_program`]). Directly
/// constructing a `TypedProgram` bypasses validation, including structural bounds.
pub fn lower(program: &TypedProgram) -> IrProgram {
    let mut functions = Vec::new();
    for function in &program.program.functions {
        functions.push(lower_function(function));
    }
    for module in &program.program.modules {
        for function in &module.functions {
            functions.push(lower_namespaced_function(&module.name, function));
        }
    }
    IrProgram { functions }
}

fn lower_namespaced_function(
    module_name: &str,
    function: &crate::compiler::ast::Function,
) -> IrFunction {
    let mut lowered = lower_function(function);
    lowered.name = format!("{module_name}::{name}", name = lowered.name);
    lowered
}

fn lower_function(function: &crate::compiler::ast::Function) -> IrFunction {
    let mut instructions = Vec::new();
    for parameter in &function.parameters {
        if parameter.type_name.as_deref() == Some("Float") {
            instructions.push(Instruction::LoadName(parameter.name.clone()));
            instructions.push(Instruction::WidenFloat);
            instructions.push(Instruction::StoreName(parameter.name.clone()));
        }
    }
    for statement in &function.body {
        lower_statement(
            statement,
            function.return_type.as_deref(),
            &mut instructions,
        );
    }
    IrFunction {
        name: function.name.clone(),
        parameters: function
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect(),
        instructions,
    }
}

fn lower_statement(
    statement: &Statement,
    return_type: Option<&str>,
    instructions: &mut Vec<Instruction>,
) {
    match statement {
        Statement::Let {
            name,
            type_name,
            value,
            ..
        } => {
            lower_expression(value, instructions);
            if type_name.as_deref() == Some("Float") {
                instructions.push(Instruction::WidenFloat);
            }
            instructions.push(Instruction::StoreName(name.clone()));
        }
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                lower_expression(value, instructions);
                if return_type == Some("Float") {
                    instructions.push(Instruction::WidenFloat);
                }
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
    enum Work<'a> {
        Visit(&'a Expression),
        Emit(Instruction),
    }
    let mut pending = vec![Work::Visit(expression)];
    while let Some(work) = pending.pop() {
        let expression = match work {
            Work::Visit(expression) => expression,
            Work::Emit(instruction) => {
                instructions.push(instruction);
                continue;
            }
        };
        match &expression.kind {
            ExpressionKind::String(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::String(value.clone())));
            }
            ExpressionKind::Integer(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::Integer(value.clone())));
            }
            ExpressionKind::Float(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::Float(value.clone())));
            }
            ExpressionKind::Boolean(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::Boolean(*value)));
            }
            ExpressionKind::Identifier(name) => {
                instructions.push(Instruction::LoadName(name.clone()))
            }
            ExpressionKind::QualifiedName { path } => {
                instructions.push(Instruction::LoadName(path.join("::")));
            }
            ExpressionKind::Call { callee, arguments } => {
                let name = match &callee.kind {
                    ExpressionKind::Identifier(name) => Some(name.clone()),
                    ExpressionKind::QualifiedName { path } => Some(path.join("::")),
                    _ => None,
                };
                if let Some(name) = name {
                    pending.push(Work::Emit(Instruction::Call {
                        name,
                        arguments: arguments.len(),
                    }));
                }
                // LIFO work preserves left-to-right argument evaluation.
                pending.extend(arguments.iter().rev().map(Work::Visit));
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                pending.push(Work::Emit(Instruction::Binary(operator.clone())));
                pending.push(Work::Visit(right));
                pending.push(Work::Visit(left));
            }
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
    fn direct_lowering_uses_a_worklist_for_deep_expressions() {
        let mut program = Parser::new().parse_source("fn main() { 1; }").unwrap();
        let Statement::Expression(mut expression) = program.functions[0].body.pop().unwrap() else {
            unreachable!()
        };
        let span = expression.span;
        for _ in 0..10_000 {
            expression = Expression {
                kind: ExpressionKind::Binary {
                    left: Box::new(expression),
                    operator: "+".into(),
                    right: Box::new(Expression {
                        kind: ExpressionKind::Integer("1".into()),
                        span,
                    }),
                },
                span,
            };
        }
        program.functions[0]
            .body
            .push(Statement::Expression(expression));
        let mut typed = TypedProgram { program };
        let ir = lower(&typed);
        // The test owns a deliberately unchecked AST. Dismantle it iteratively
        // so testing lowering does not exercise the separate recursive Drop issue.
        let Statement::Expression(expression) = typed.program.functions[0].body.pop().unwrap()
        else {
            unreachable!()
        };
        let mut pending = vec![expression];
        while let Some(expression) = pending.pop() {
            if let ExpressionKind::Binary { left, right, .. } = expression.kind {
                pending.push(*left);
                pending.push(*right);
            }
        }
        assert_eq!(ir.functions[0].instructions.len(), 20_002);
        assert!(crate::compiler::interpreter::run(&ir).unwrap().is_empty());
    }

    #[test]
    fn lowering_preserves_operand_and_argument_order() {
        let program = Parser::new().parse_source("fn pair(a: Int, b: Int) -> Int { return a - b } fn main() { let value = pair(1 + 2, 3 * 4); print(value); }").unwrap();
        let ir = lower_program(&program).unwrap();
        let main = ir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .unwrap();
        assert_eq!(
            main.instructions,
            vec![
                Instruction::LoadLiteral(Literal::Integer("1".into())),
                Instruction::LoadLiteral(Literal::Integer("2".into())),
                Instruction::Binary("+".into()),
                Instruction::LoadLiteral(Literal::Integer("3".into())),
                Instruction::LoadLiteral(Literal::Integer("4".into())),
                Instruction::Binary("*".into()),
                Instruction::Call {
                    name: "pair".into(),
                    arguments: 2
                },
                Instruction::StoreName("value".into()),
                Instruction::LoadName("value".into()),
                Instruction::Call {
                    name: "print".into(),
                    arguments: 1
                },
                Instruction::Pop,
            ]
        );
        assert_eq!(crate::compiler::interpreter::run(&ir).unwrap(), ["-9"]);
    }

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
