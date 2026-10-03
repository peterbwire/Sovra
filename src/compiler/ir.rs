//! Stable, minimal intermediate representation for M10 and later backends.

use std::collections::{HashMap, HashSet};

mod type_facts;
use type_facts::{TypeFacts, ValueType};

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
    /// Replace the element at an index in an array-like value.
    StoreIndex,
    /// Convert an Int value to Float, preserving values already of type Float.
    WidenFloat,
    /// Convert numeric leaves in an array to Float without changing its shape.
    WidenFloatArray {
        /// Number of array levels above each numeric leaf; must be positive.
        depth: usize,
    },
    /// Apply an operator.
    Binary(String),
    /// Construct a homogeneous array from the top `length` stack values.
    MakeArray {
        /// Number of stack values consumed to assemble the array.
        length: usize,
    },
    /// Construct a record from the top stack values in field order.
    MakeStruct {
        /// Declared record type name.
        type_name: String,
        /// Field names corresponding to consumed values.
        fields: Vec<String>,
    },
    /// Read a field from the top record value.
    LoadField(String),
    /// Index an array-like value using the top two stack items.
    Index,
    /// Jump to a concrete instruction index.
    Jump {
        /// Instruction index to jump to.
        target: usize,
    },
    /// Jump to a concrete instruction index when the condition is false.
    JumpIfFalse {
        /// Instruction index to jump to when the condition is false.
        target: usize,
    },
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
    lower_with_imports(program, &[])
}

// Package-qualified signatures retain imported result types during local
// inference. Do not resolve their annotations against consumer-owned aliases.
pub(crate) fn lower_with_imports(
    program: &TypedProgram,
    imports: &[(String, &crate::compiler::ast::Function)],
) -> IrProgram {
    let float_aliases = collect_float_aliases(&program.program);
    let type_facts = TypeFacts::new(&program.program, imports);
    let float_struct_fields = collect_float_struct_fields(&program.program, &float_aliases);
    let mut functions = Vec::new();
    for function in &program.program.functions {
        functions.push(lower_function(
            function,
            &float_aliases,
            &type_facts,
            &float_struct_fields,
        ));
    }
    for module in &program.program.modules {
        for function in &module.functions {
            functions.push(lower_namespaced_function(
                &module.name,
                function,
                &float_aliases,
                &type_facts,
                &float_struct_fields,
            ));
        }
    }
    IrProgram { functions }
}

fn collect_float_struct_fields(
    program: &Program,
    float_aliases: &HashSet<String>,
) -> HashMap<String, HashSet<String>> {
    let mut structs = HashMap::new();
    for declaration in &program.struct_declarations {
        let fields = declaration
            .fields
            .iter()
            .filter(|field| is_float_type(Some(&field.type_name), float_aliases))
            .map(|field| field.name.clone())
            .collect();
        structs.insert(declaration.name.clone(), fields);
    }
    for module in &program.modules {
        for declaration in &module.struct_declarations {
            let fields: HashSet<_> = declaration
                .fields
                .iter()
                .filter(|field| is_float_type(Some(&field.type_name), float_aliases))
                .map(|field| field.name.clone())
                .collect();
            structs.insert(format!("{}::{}", module.name, declaration.name), fields);
        }
    }
    // Imported record spellings are aliases to canonical declarations. Preserve
    // field widening through those aliases as well as ordinary local aliases.
    loop {
        let mut changed = false;
        for (name, target) in program
            .type_declarations
            .iter()
            .map(|decl| (decl.name.clone(), &decl.target))
            .chain(program.modules.iter().flat_map(|module| {
                module
                    .type_declarations
                    .iter()
                    .map(move |decl| (format!("{}::{}", module.name, decl.name), &decl.target))
            }))
        {
            if !structs.contains_key(&name) {
                if let Some(fields) = structs.get(target).cloned() {
                    structs.insert(name, fields);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    structs
}

fn is_float_type(name: Option<&str>, float_aliases: &HashSet<String>) -> bool {
    name.is_some_and(|name| name == "Float" || float_aliases.contains(name))
}

fn collect_float_aliases(program: &Program) -> std::collections::HashSet<String> {
    let mut aliases = std::collections::HashMap::new();
    for declaration in &program.type_declarations {
        aliases.insert(declaration.name.clone(), declaration.target.clone());
    }
    for module in &program.modules {
        for declaration in &module.type_declarations {
            aliases.insert(
                format!("{}::{}", module.name, declaration.name),
                declaration.target.clone(),
            );
        }
    }

    let mut float_aliases = std::collections::HashSet::new();
    for name in aliases.keys() {
        let mut current = name.as_str();
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(current) {
                break;
            }
            match aliases.get(current).map(String::as_str) {
                Some("Float") => {
                    float_aliases.insert(name.clone());
                    break;
                }
                Some(target) => current = target,
                None => break,
            }
        }
    }
    float_aliases
}

#[derive(Clone)]
struct LoweredBinding {
    name: String,
    value_type: ValueType,
}

fn lower_namespaced_function(
    module_name: &str,
    function: &crate::compiler::ast::Function,
    float_aliases: &HashSet<String>,
    type_facts: &TypeFacts,
    float_struct_fields: &HashMap<String, HashSet<String>>,
) -> IrFunction {
    let mut lowered = lower_function(function, float_aliases, type_facts, float_struct_fields);
    lowered.name = format!("{module_name}::{name}", name = lowered.name);
    lowered
}

fn lower_function(
    function: &crate::compiler::ast::Function,
    float_aliases: &HashSet<String>,
    type_facts: &TypeFacts,
    float_struct_fields: &HashMap<String, HashSet<String>>,
) -> IrFunction {
    let mut locals: HashMap<String, LoweredBinding> = function
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                LoweredBinding {
                    name: parameter.name.clone(),
                    value_type: type_facts.annotation(parameter.type_name.as_deref().unwrap_or("")),
                },
            )
        })
        .collect();
    let mut next_shadow_id = 0;
    let mut instructions = Vec::new();
    for parameter in &function.parameters {
        if parameter
            .type_name
            .as_ref()
            .is_some_and(|name| name == "Float" || float_aliases.contains(name))
        {
            instructions.push(Instruction::LoadName(parameter.name.clone()));
            instructions.push(Instruction::WidenFloat);
            instructions.push(Instruction::StoreName(parameter.name.clone()));
        }
    }
    for statement in &function.body {
        lower_statement(
            statement,
            StatementTypes {
                return_type: function.return_type.as_deref(),
                float_aliases,
                type_facts,
                float_struct_fields,
            },
            &mut locals,
            &mut next_shadow_id,
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

#[derive(Clone, Copy)]
struct StatementTypes<'a> {
    return_type: Option<&'a str>,
    float_aliases: &'a HashSet<String>,
    type_facts: &'a TypeFacts,
    float_struct_fields: &'a HashMap<String, HashSet<String>>,
}

fn lower_statement(
    statement: &Statement,
    types: StatementTypes<'_>,
    locals: &mut HashMap<String, LoweredBinding>,
    next_shadow_id: &mut usize,
    instructions: &mut Vec<Instruction>,
) {
    let StatementTypes {
        return_type,
        float_aliases,
        type_facts,
        float_struct_fields,
    } = types;
    match statement {
        Statement::Let {
            name,
            type_name,
            value,
            ..
        } => {
            lower_expression(value, locals, type_facts, float_struct_fields, instructions);
            let is_declared_float = is_float_type(type_name.as_deref(), float_aliases);
            let value_type = type_name
                .as_deref()
                .map(|name| type_facts.annotation(name))
                .unwrap_or_else(|| type_facts.expression(value, locals));
            if is_declared_float {
                instructions.push(Instruction::WidenFloat);
            }
            let lowered_name = if locals.contains_key(name) {
                let lowered_name = format!("\0svr-local-{}", *next_shadow_id);
                *next_shadow_id += 1;
                lowered_name
            } else {
                name.clone()
            };
            locals.insert(
                name.clone(),
                LoweredBinding {
                    name: lowered_name.clone(),
                    value_type,
                },
            );
            instructions.push(Instruction::StoreName(lowered_name));
        }
        Statement::Assign { target, value, .. } => match &target.kind {
            ExpressionKind::Identifier(name) => {
                lower_expression(value, locals, type_facts, float_struct_fields, instructions);
                if let Some(instruction) = locals
                    .get(name)
                    .and_then(|binding| widening_instruction(&binding.value_type))
                {
                    instructions.push(instruction);
                }
                instructions.push(Instruction::StoreName(
                    locals
                        .get(name)
                        .map(|binding| binding.name.clone())
                        .unwrap_or_else(|| name.clone()),
                ));
            }
            ExpressionKind::Index {
                target: base,
                index,
            } => {
                lower_expression(base, locals, type_facts, float_struct_fields, instructions);
                lower_expression(index, locals, type_facts, float_struct_fields, instructions);
                lower_expression(value, locals, type_facts, float_struct_fields, instructions);
                if let Some(instruction) =
                    widening_instruction(&type_facts.expression(target, locals))
                {
                    instructions.push(instruction);
                }
                instructions.push(Instruction::StoreIndex);
                if let ExpressionKind::Identifier(name) = &base.kind {
                    instructions.push(Instruction::StoreName(
                        locals
                            .get(name)
                            .map(|binding| binding.name.clone())
                            .unwrap_or_else(|| name.clone()),
                    ));
                } else {
                    instructions.push(Instruction::Pop);
                }
            }
            _ => {
                lower_expression(value, locals, type_facts, float_struct_fields, instructions);
            }
        },
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                lower_expression(value, locals, type_facts, float_struct_fields, instructions);
                if is_float_type(return_type, float_aliases) {
                    instructions.push(Instruction::WidenFloat);
                }
            }
            instructions.push(Instruction::Return);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            lower_expression(
                condition,
                locals,
                type_facts,
                float_struct_fields,
                instructions,
            );
            let false_jump = instructions.len();
            instructions.push(Instruction::JumpIfFalse { target: 0 });
            let mut then_locals = locals.clone();
            for statement in then_block {
                lower_statement(
                    statement,
                    types,
                    &mut then_locals,
                    next_shadow_id,
                    instructions,
                );
            }
            if let Some(else_block) = else_block {
                let end_jump = instructions.len();
                instructions.push(Instruction::Jump { target: 0 });
                let else_start = instructions.len();
                instructions[false_jump] = Instruction::JumpIfFalse { target: else_start };
                let mut else_locals = locals.clone();
                for statement in else_block {
                    lower_statement(
                        statement,
                        types,
                        &mut else_locals,
                        next_shadow_id,
                        instructions,
                    );
                }
                instructions[end_jump] = Instruction::Jump {
                    target: instructions.len(),
                };
            } else {
                instructions[false_jump] = Instruction::JumpIfFalse {
                    target: instructions.len(),
                };
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            let loop_start = instructions.len();
            lower_expression(
                condition,
                locals,
                type_facts,
                float_struct_fields,
                instructions,
            );
            let exit_jump = instructions.len();
            instructions.push(Instruction::JumpIfFalse { target: 0 });
            let mut loop_locals = locals.clone();
            for statement in body {
                lower_statement(
                    statement,
                    types,
                    &mut loop_locals,
                    next_shadow_id,
                    instructions,
                );
            }
            instructions.push(Instruction::Jump { target: loop_start });
            instructions[exit_jump] = Instruction::JumpIfFalse {
                target: instructions.len(),
            };
        }
        Statement::Expression(expression) => {
            lower_expression(
                expression,
                locals,
                type_facts,
                float_struct_fields,
                instructions,
            );
            instructions.push(Instruction::Pop);
        }
    }
}

fn lower_expression(
    expression: &Expression,
    locals: &HashMap<String, LoweredBinding>,
    type_facts: &TypeFacts,
    float_struct_fields: &HashMap<String, HashSet<String>>,
    instructions: &mut Vec<Instruction>,
) {
    enum Work<'a> {
        Visit(&'a Expression),
        Emit(Instruction),
        FinishAnd(&'a Expression),
        FinishAndRight { false_jump: usize },
        FinishOr(&'a Expression),
        FinishOrRight { end_jump: usize },
    }
    let mut pending = vec![Work::Visit(expression)];
    while let Some(work) = pending.pop() {
        let expression = match work {
            Work::Visit(expression) => expression,
            Work::Emit(instruction) => {
                instructions.push(instruction);
                continue;
            }
            Work::FinishAnd(right) => {
                let false_jump = instructions.len();
                instructions.push(Instruction::JumpIfFalse { target: 0 });
                pending.push(Work::FinishAndRight { false_jump });
                pending.push(Work::Visit(right));
                continue;
            }
            Work::FinishAndRight { false_jump } => {
                let end_jump = instructions.len();
                instructions.push(Instruction::Jump { target: 0 });
                let false_target = instructions.len();
                instructions[false_jump] = Instruction::JumpIfFalse {
                    target: false_target,
                };
                instructions.push(Instruction::LoadLiteral(Literal::Boolean(false)));
                instructions[end_jump] = Instruction::Jump {
                    target: instructions.len(),
                };
                continue;
            }
            Work::FinishOr(right) => {
                let right_target_jump = instructions.len();
                instructions.push(Instruction::JumpIfFalse { target: 0 });
                instructions.push(Instruction::LoadLiteral(Literal::Boolean(true)));
                let end_jump = instructions.len();
                instructions.push(Instruction::Jump { target: 0 });
                let right_target = instructions.len();
                instructions[right_target_jump] = Instruction::JumpIfFalse {
                    target: right_target,
                };
                pending.push(Work::FinishOrRight { end_jump });
                pending.push(Work::Visit(right));
                continue;
            }
            Work::FinishOrRight { end_jump } => {
                instructions[end_jump] = Instruction::Jump {
                    target: instructions.len(),
                };
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
            ExpressionKind::Identifier(name) => instructions.push(Instruction::LoadName(
                locals
                    .get(name)
                    .map(|binding| binding.name.clone())
                    .unwrap_or_else(|| name.clone()),
            )),
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
            ExpressionKind::FieldAccess { receiver, field } => {
                pending.push(Work::Emit(Instruction::LoadField(field.clone())));
                pending.push(Work::Visit(receiver));
            }
            ExpressionKind::StructLiteral { type_name, fields } => {
                pending.push(Work::Emit(Instruction::MakeStruct {
                    type_name: type_name.clone(),
                    fields: fields.iter().map(|(name, _)| name.clone()).collect(),
                }));
                let float_fields = float_struct_fields.get(type_name);
                for (field_name, value) in fields.iter().rev() {
                    if float_fields.is_some_and(|names| names.contains(field_name)) {
                        pending.push(Work::Emit(Instruction::WidenFloat));
                    }
                    pending.push(Work::Visit(value));
                }
            }
            ExpressionKind::ArrayLiteral(items) => {
                pending.push(Work::Emit(Instruction::MakeArray {
                    length: items.len(),
                }));
                let element_widening = match type_facts.expression(expression, locals) {
                    ValueType::Array(element) => widening_instruction(&element),
                    _ => None,
                };
                for value in items.iter().rev() {
                    if let Some(instruction) = &element_widening {
                        pending.push(Work::Emit(instruction.clone()));
                    }
                    pending.push(Work::Visit(value));
                }
            }
            ExpressionKind::Index { target, index } => {
                pending.push(Work::Emit(Instruction::Index));
                pending.push(Work::Visit(index));
                pending.push(Work::Visit(target));
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                match operator.as_str() {
                    "&&" => pending.push(Work::FinishAnd(right)),
                    "||" => pending.push(Work::FinishOr(right)),
                    _ => {
                        pending.push(Work::Emit(Instruction::Binary(operator.clone())));
                        pending.push(Work::Visit(right));
                    }
                }
                pending.push(Work::Visit(left));
            }
        }
    }
}

fn widening_instruction(value_type: &ValueType) -> Option<Instruction> {
    let mut leaf = value_type;
    let mut depth = 0;
    while let ValueType::Array(element) = leaf {
        depth += 1;
        leaf = element;
    }
    match (leaf, depth) {
        (ValueType::Float, 0) => Some(Instruction::WidenFloat),
        (ValueType::Float, depth) => Some(Instruction::WidenFloatArray { depth }),
        _ => None,
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
