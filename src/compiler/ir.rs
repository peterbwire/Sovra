//! Stable, minimal intermediate representation for M10 and later backends.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

mod budget;
mod checked_facts;
mod type_facts;
pub(crate) use budget::Budget;
use budget::Instructions;
use type_facts::{TypeFacts, ValueType};

use crate::compiler::ast::{Expression, ExpressionKind, Program, Statement};
use crate::compiler::semantic::TypedProgram;
use crate::compiler::{diagnostics::Diagnostics, resources::LoweringLimits};

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
    lower_internal(program, imports, None)
        .expect("unbounded lowering cannot exhaust emission budgets")
}

/// Lower checked input with aggregate retained instruction/string budgets.
/// Exhaustion returns E3021/E3022, never a partial successful program. Helpers
/// and temporary payloads are not a process-memory quota. TypedProgram remains
/// a trusted input; use lower_program_with_limits for semantic validation.
pub fn lower_with_limits(
    program: &TypedProgram,
    limits: LoweringLimits,
) -> Result<IrProgram, Diagnostics> {
    lower_internal(program, &[], Some(limits))
}

/// Checked lowering requires explicit complete lowering-ready metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedLoweringError {
    /// Snapshot was built without the required types/uses/context records.
    EvidenceUnavailable,
    /// Existing E3021/E3022 emission diagnostics; no partial IR is returned.
    Emission(Diagnostics),
}

impl std::fmt::Display for CheckedLoweringError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EvidenceUnavailable => {
                formatter.write_str("checked lowering evidence unavailable")
            }
            Self::Emission(errors) => write!(
                formatter,
                "IR emission failed with {} diagnostics",
                errors.items.len()
            ),
        }
    }
}

impl std::error::Error for CheckedLoweringError {}

/// Lower an immutable standalone snapshot using retained types, resolved targets
/// and contextual facts. No rechecking, type reconstruction or partial fallback.
/// Use CheckedUnit::from_source_for_lowering or from_program_for_lowering first.
/// Legacy lowering APIs and normal CLI/package paths retain their current behavior.
pub fn lower_checked(
    unit: &crate::compiler::checked::CheckedUnit,
    limits: LoweringLimits,
) -> Result<IrProgram, CheckedLoweringError> {
    let mut budget = Budget::new(Some(limits));
    lower_checked_with_budget(unit, &mut budget)
}

pub(crate) fn lower_checked_with_budget(
    unit: &crate::compiler::checked::CheckedUnit,
    budget: &mut Budget,
) -> Result<IrProgram, CheckedLoweringError> {
    if !unit
        .semantic_evidence()
        .is_some_and(|evidence| evidence.flow().is_some() && evidence.lowering().is_some())
    {
        return Err(CheckedLoweringError::EvidenceUnavailable);
    }
    let facts = TypeFacts::from_checked(unit);
    lower_unit(
        unit.program(),
        &HashSet::new(),
        &facts,
        &HashMap::new(),
        budget,
    )
    .map_err(CheckedLoweringError::Emission)
}

fn lower_internal(
    program: &TypedProgram,
    imports: &[(String, &crate::compiler::ast::Function)],
    limits: Option<LoweringLimits>,
) -> Result<IrProgram, Diagnostics> {
    let mut budget = Budget::new(limits);
    lower_with_budget(program, imports, &mut budget)
}

pub(crate) fn lower_with_budget(
    program: &TypedProgram,
    imports: &[(String, &crate::compiler::ast::Function)],
    budget: &mut Budget,
) -> Result<IrProgram, Diagnostics> {
    let aliases = reverse_aliases(&program.program);
    let float_aliases = collect_float_aliases(&aliases);
    let type_facts = TypeFacts::new(&program.program, imports);
    let float_struct_fields =
        collect_float_struct_fields(&program.program, &float_aliases, &aliases);
    lower_unit(
        &program.program,
        &float_aliases,
        &type_facts,
        &float_struct_fields,
        budget,
    )
}

fn lower_unit(
    program: &Program,
    float_aliases: &HashSet<String>,
    type_facts: &TypeFacts,
    float_struct_fields: &HashMap<String, Arc<HashSet<String>>>,
    budget: &mut Budget,
) -> Result<IrProgram, Diagnostics> {
    let mut functions = Vec::new();
    for function in &program.functions {
        functions.push(lower_function(
            function,
            &function.name,
            float_aliases,
            type_facts,
            float_struct_fields,
            budget,
        )?);
    }
    for module in &program.modules {
        for function in &module.functions {
            functions.push(lower_namespaced_function(
                &module.name,
                function,
                float_aliases,
                type_facts,
                float_struct_fields,
                budget,
            )?);
        }
    }
    Ok(IrProgram { functions })
}

fn collect_float_struct_fields(
    program: &Program,
    float_aliases: &HashSet<String>,
    aliases: &HashMap<String, Vec<String>>,
) -> HashMap<String, Arc<HashSet<String>>> {
    let mut structs = HashMap::new();
    for declaration in &program.struct_declarations {
        let fields = declaration
            .fields
            .iter()
            .filter(|field| is_float_type(Some(&field.type_name), float_aliases))
            .map(|field| field.name.clone())
            .collect();
        structs.insert(declaration.name.clone(), Arc::new(fields));
    }
    for module in &program.modules {
        for declaration in &module.struct_declarations {
            let fields: HashSet<_> = declaration
                .fields
                .iter()
                .filter(|field| is_float_type(Some(&field.type_name), float_aliases))
                .map(|field| field.name.clone())
                .collect();
            structs.insert(
                format!("{}::{}", module.name, declaration.name),
                Arc::new(fields),
            );
        }
    }
    // Propagate from declarations to their aliases once, sharing immutable
    // metadata instead of repeatedly scanning chains and cloning every field.
    let mut pending: Vec<_> = structs.keys().cloned().collect();
    while let Some(target) = pending.pop() {
        let fields = Arc::clone(&structs[&target]);
        if let Some(dependents) = aliases.get(&target) {
            for name in dependents {
                if !structs.contains_key(name) {
                    structs.insert(name.clone(), Arc::clone(&fields));
                    pending.push(name.clone());
                }
            }
        }
    }
    structs
}

fn is_float_type(name: Option<&str>, float_aliases: &HashSet<String>) -> bool {
    name.is_some_and(|name| name == "Float" || float_aliases.contains(name))
}

fn reverse_aliases(program: &Program) -> HashMap<String, Vec<String>> {
    let mut targets = HashMap::new();
    for declaration in &program.type_declarations {
        targets.insert(declaration.name.clone(), declaration.target.clone());
    }
    for module in &program.modules {
        for declaration in &module.type_declarations {
            targets.insert(
                format!("{}::{}", module.name, declaration.name),
                declaration.target.clone(),
            );
        }
    }
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for (name, target) in targets {
        dependents.entry(target).or_default().push(name);
    }
    dependents
}

fn collect_float_aliases(aliases: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut result = HashSet::new();
    let mut pending = vec!["Float".to_owned()];
    while let Some(target) = pending.pop() {
        if let Some(dependents) = aliases.get(&target) {
            for name in dependents {
                if result.insert(name.clone()) {
                    pending.push(name.clone());
                }
            }
        }
    }
    result
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
    float_struct_fields: &HashMap<String, Arc<HashSet<String>>>,
    budget: &mut Budget,
) -> Result<IrFunction, Diagnostics> {
    let name = format!("{module_name}::{}", function.name);
    lower_function(
        function,
        &name,
        float_aliases,
        type_facts,
        float_struct_fields,
        budget,
    )
}

fn lower_function(
    function: &crate::compiler::ast::Function,
    lowered_name: &str,
    float_aliases: &HashSet<String>,
    type_facts: &TypeFacts,
    float_struct_fields: &HashMap<String, Arc<HashSet<String>>>,
    budget: &mut Budget,
) -> Result<IrFunction, Diagnostics> {
    let lowered_name = budget.function_name(lowered_name, function.span)?;
    for parameter in &function.parameters {
        budget.string(parameter.name.len(), parameter.span)?;
    }
    let mut locals: HashMap<String, LoweredBinding> = function
        .parameters
        .iter()
        .map(|parameter| {
            type_facts.register_parameter(parameter);
            (
                parameter.name.clone(),
                LoweredBinding {
                    name: parameter.name.clone(),
                    value_type: type_facts.parameter_type(parameter),
                },
            )
        })
        .collect();
    let mut next_shadow_id = 0;
    let mut instructions = Instructions::new(budget, function.span);
    for parameter in &function.parameters {
        if let Some(widening) = if type_facts.parameter_is_float(parameter, float_aliases) {
            Some(Instruction::WidenFloat)
        } else {
            widening_instruction(&type_facts.parameter_type(parameter))
        } {
            instructions.push(Instruction::LoadName(parameter.name.clone()))?;
            instructions.push(widening)?;
            instructions.push(Instruction::StoreName(parameter.name.clone()))?;
        }
    }
    let return_type = type_facts.function_type(function, float_aliases);
    for statement in &function.body {
        lower_statement(
            statement,
            StatementTypes {
                return_type: &return_type,
                type_facts,
                float_struct_fields,
            },
            &mut locals,
            &mut next_shadow_id,
            &mut instructions,
        )?;
    }
    Ok(IrFunction {
        name: lowered_name,
        parameters: function
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect(),
        instructions: instructions.finish(),
    })
}

#[derive(Clone, Copy)]
struct StatementTypes<'a> {
    return_type: &'a ValueType,
    type_facts: &'a TypeFacts,
    float_struct_fields: &'a HashMap<String, Arc<HashSet<String>>>,
}

fn lower_statement(
    statement: &Statement,
    types: StatementTypes<'_>,
    locals: &mut HashMap<String, LoweredBinding>,
    next_shadow_id: &mut usize,
    instructions: &mut Instructions<'_>,
) -> Result<(), Diagnostics> {
    instructions.span = match statement {
        Statement::Expression(value) => value.span,
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::Return { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. } => *span,
    };
    let StatementTypes {
        return_type,
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
            lower_expression(value, locals, type_facts, float_struct_fields, instructions)?;
            let value_type =
                type_facts.binding_type(statement, type_name.as_deref(), value, locals);
            if type_name.is_some() {
                if let Some(widening) = widening_instruction(&value_type) {
                    instructions.push(widening)?;
                }
            }
            let lowered_name = if locals.contains_key(name) {
                let lowered_name = format!("\0svr-local-{}", *next_shadow_id);
                *next_shadow_id += 1;
                lowered_name
            } else {
                name.clone()
            };
            type_facts.register_binding(statement, &lowered_name);
            locals.insert(
                name.clone(),
                LoweredBinding {
                    name: lowered_name.clone(),
                    value_type,
                },
            );
            instructions.push(Instruction::StoreName(lowered_name))?;
        }
        Statement::Assign { target, value, .. } => match &target.kind {
            ExpressionKind::Identifier(name) => {
                lower_expression(value, locals, type_facts, float_struct_fields, instructions)?;
                if let Some(instruction) = type_facts
                    .assignment_type(target, name, locals)
                    .as_ref()
                    .and_then(widening_instruction)
                {
                    instructions.push(instruction)?;
                }
                instructions.push(Instruction::StoreName(
                    type_facts.value_name(target, name, locals),
                ))?;
            }
            ExpressionKind::Index {
                target: base,
                index,
            } => {
                lower_expression(base, locals, type_facts, float_struct_fields, instructions)?;
                lower_expression(index, locals, type_facts, float_struct_fields, instructions)?;
                lower_expression(value, locals, type_facts, float_struct_fields, instructions)?;
                if let Some(instruction) =
                    widening_instruction(&type_facts.expression(target, locals))
                {
                    instructions.push(instruction)?;
                }
                instructions.push(Instruction::StoreIndex)?;
                if let ExpressionKind::Identifier(name) = &base.kind {
                    instructions.push(Instruction::StoreName(
                        type_facts.value_name(base, name, locals),
                    ))?;
                } else {
                    instructions.push(Instruction::Pop)?;
                }
            }
            _ => {
                lower_expression(value, locals, type_facts, float_struct_fields, instructions)?;
            }
        },
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                lower_expression(value, locals, type_facts, float_struct_fields, instructions)?;
                if let Some(widening) = widening_instruction(return_type) {
                    instructions.push(widening)?;
                }
            }
            instructions.push(Instruction::Return)?;
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
            )?;
            let false_jump = instructions.len();
            instructions.push(Instruction::JumpIfFalse { target: 0 })?;
            let mut then_locals = locals.clone();
            for statement in then_block {
                lower_statement(
                    statement,
                    types,
                    &mut then_locals,
                    next_shadow_id,
                    instructions,
                )?;
            }
            if let Some(else_block) = else_block {
                let end_jump = instructions.len();
                instructions.push(Instruction::Jump { target: 0 })?;
                let else_start = instructions.len();
                instructions.patch_jump(false_jump, else_start);
                let mut else_locals = locals.clone();
                for statement in else_block {
                    lower_statement(
                        statement,
                        types,
                        &mut else_locals,
                        next_shadow_id,
                        instructions,
                    )?;
                }
                instructions.patch_jump(end_jump, instructions.len());
            } else {
                instructions.patch_jump(false_jump, instructions.len());
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
            )?;
            let exit_jump = instructions.len();
            instructions.push(Instruction::JumpIfFalse { target: 0 })?;
            let mut loop_locals = locals.clone();
            for statement in body {
                lower_statement(
                    statement,
                    types,
                    &mut loop_locals,
                    next_shadow_id,
                    instructions,
                )?;
            }
            instructions.push(Instruction::Jump { target: loop_start })?;
            instructions.patch_jump(exit_jump, instructions.len());
        }
        Statement::Expression(expression) => {
            lower_expression(
                expression,
                locals,
                type_facts,
                float_struct_fields,
                instructions,
            )?;
            instructions.push(Instruction::Pop)?;
        }
    }
    Ok(())
}

fn lower_expression(
    expression: &Expression,
    locals: &HashMap<String, LoweredBinding>,
    type_facts: &TypeFacts,
    float_struct_fields: &HashMap<String, Arc<HashSet<String>>>,
    instructions: &mut Instructions<'_>,
) -> Result<(), Diagnostics> {
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
                instructions.push(instruction)?;
                continue;
            }
            Work::FinishAnd(right) => {
                let false_jump = instructions.len();
                instructions.push(Instruction::JumpIfFalse { target: 0 })?;
                pending.push(Work::FinishAndRight { false_jump });
                pending.push(Work::Visit(right));
                continue;
            }
            Work::FinishAndRight { false_jump } => {
                let end_jump = instructions.len();
                instructions.push(Instruction::Jump { target: 0 })?;
                let false_target = instructions.len();
                instructions.patch_jump(false_jump, false_target);
                instructions.push(Instruction::LoadLiteral(Literal::Boolean(false)))?;
                instructions.patch_jump(end_jump, instructions.len());
                continue;
            }
            Work::FinishOr(right) => {
                let right_target_jump = instructions.len();
                instructions.push(Instruction::JumpIfFalse { target: 0 })?;
                instructions.push(Instruction::LoadLiteral(Literal::Boolean(true)))?;
                let end_jump = instructions.len();
                instructions.push(Instruction::Jump { target: 0 })?;
                let right_target = instructions.len();
                instructions.patch_jump(right_target_jump, right_target);
                pending.push(Work::FinishOrRight { end_jump });
                pending.push(Work::Visit(right));
                continue;
            }
            Work::FinishOrRight { end_jump } => {
                instructions.patch_jump(end_jump, instructions.len());
                continue;
            }
        };
        instructions.span = expression.span;
        match &expression.kind {
            ExpressionKind::String(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::String(value.clone())))?;
            }
            ExpressionKind::Integer(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::Integer(value.clone())))?;
            }
            ExpressionKind::Float(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::Float(value.clone())))?;
            }
            ExpressionKind::Boolean(value) => {
                instructions.push(Instruction::LoadLiteral(Literal::Boolean(*value)))?;
            }
            ExpressionKind::Identifier(name) => instructions.push(Instruction::LoadName(
                type_facts.value_name(expression, name, locals),
            ))?,
            ExpressionKind::QualifiedName { path } => {
                instructions.push(Instruction::LoadName(path.join("::")))?;
            }
            ExpressionKind::Call { callee, arguments } => {
                let name = type_facts.call_name(callee);
                let growth = matches!(
                    name.as_deref(),
                    Some("std::array_append" | "std::array_concat")
                );
                let append = name.as_deref() == Some("std::array_append");
                if let Some(name) = name {
                    pending.push(Work::Emit(Instruction::Call {
                        name,
                        arguments: arguments.len(),
                    }));
                }
                // LIFO preserves evaluation order; dependent numeric coercion is
                // emitted immediately after the second argument, before the call.
                for (index, argument) in arguments.iter().enumerate().rev() {
                    if growth && index == 1 {
                        if let Some(widening) =
                            widening_instruction(&type_facts.growth_argument_type(
                                expression,
                                argument,
                                &arguments[0],
                                append,
                                locals,
                            ))
                        {
                            pending.push(Work::Emit(widening));
                        }
                    }
                    pending.push(Work::Visit(argument));
                }
            }
            ExpressionKind::FieldAccess { receiver, field } => {
                pending.push(Work::Emit(Instruction::LoadField(field.clone())));
                pending.push(Work::Visit(receiver));
            }
            ExpressionKind::StructLiteral { type_name, fields } => {
                pending.push(Work::Emit(Instruction::MakeStruct {
                    type_name: type_facts.record_name(expression, type_name),
                    fields: fields.iter().map(|(name, _)| name.clone()).collect(),
                }));
                for (field_name, value) in fields.iter().rev() {
                    if let Some(widening) = widening_instruction(&type_facts.field_type(
                        expression,
                        value,
                        type_name,
                        field_name,
                        float_struct_fields,
                    )) {
                        pending.push(Work::Emit(widening));
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
    Ok(())
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

/// Check and lower one source unit with approved AST and retained IR ceilings.
pub fn lower_program(
    program: &Program,
) -> Result<IrProgram, crate::compiler::diagnostics::Diagnostics> {
    lower_program_with_limits(
        program,
        crate::compiler::resources::DEFAULT_AST_NODES,
        LoweringLimits::DEFAULT,
    )
}

/// Semantically check and lower one source AST with explicit admission/emission
/// budgets. Package linking uses separate accounting. Admission happens
/// after parsing but before semantic cloning; emission failure discards partial IR.
pub fn lower_program_with_limits(
    program: &Program,
    max_ast_nodes: usize,
    limits: LoweringLimits,
) -> Result<IrProgram, Diagnostics> {
    let typed = crate::compiler::semantic::SemanticAnalyzer::new()
        .analyze_with_node_limit(program, max_ast_nodes)?;
    lower_with_limits(&typed, limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_worklists_share_record_metadata_and_ignore_unreachable_cycles() {
        let mut source = String::from("struct Point { value: Float, other: Int }\n");
        for index in 0..5000 {
            let record_target = if index == 4999 {
                "Point".to_owned()
            } else {
                format!("Record{}", index + 1)
            };
            let float_target = if index == 4999 {
                "Float".to_owned()
            } else {
                format!("Scalar{}", index + 1)
            };
            source.push_str(&format!(
                "type Record{index} = {record_target}; type Scalar{index} = {float_target};\n"
            ));
        }
        // These helpers are also defensive when trusted-stage callers supply
        // unvalidated alias metadata; disconnected cycles must terminate.
        source.push_str("type CycleA = CycleB; type CycleB = CycleA; type Missing = Absent; mod area { type Scalar = Float; type Record = Point; }");
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(&source)
            .unwrap();
        let aliases = reverse_aliases(&parsed);
        let floats = collect_float_aliases(&aliases);
        assert_eq!(floats.len(), 5001);
        assert!(floats.contains("Scalar0") && floats.contains("area::Scalar"));
        assert!(!floats.contains("CycleA") && !floats.contains("Missing"));
        let records = collect_float_struct_fields(&parsed, &floats, &aliases);
        assert_eq!(records.len(), 5002);
        let fields = &records["Point"];
        assert_eq!(fields.len(), 1);
        assert!(fields.contains("value"));
        for index in 0..5000 {
            assert!(Arc::ptr_eq(fields, &records[&format!("Record{index}")]));
        }
        assert!(Arc::ptr_eq(fields, &records["area::Record"]));
        assert!(!records.contains_key("CycleA") && !records.contains_key("Missing"));
    }
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
