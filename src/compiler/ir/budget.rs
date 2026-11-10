//! Retained IR emission accounting shared across functions.

use super::{Instruction, Literal};
use crate::compiler::{
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    resources::LoweringLimits,
};
use std::collections::HashMap;

pub(crate) struct Budget {
    limits: Option<LoweringLimits>,
    instructions: usize,
    strings: usize,
    function_names: HashMap<String, String>,
    record_names: HashMap<String, String>,
}

impl Budget {
    pub(crate) fn new(limits: Option<LoweringLimits>) -> Self {
        Self {
            limits,
            instructions: 0,
            strings: 0,
            function_names: HashMap::new(),
            record_names: HashMap::new(),
        }
    }
    pub(crate) fn link_names(
        &mut self,
        functions: HashMap<String, String>,
        records: HashMap<String, String>,
    ) {
        self.function_names = functions;
        self.record_names = records;
    }
    pub(super) fn function_name(&mut self, name: &str, span: Span) -> Result<String, Diagnostics> {
        let bytes = self
            .function_names
            .get(name)
            .map_or(name.len(), String::len);
        self.string(bytes, span)?;
        Ok(self
            .function_names
            .get(name)
            .map_or(name, String::as_str)
            .to_owned())
    }
    pub(super) fn string(&mut self, bytes: usize, span: Span) -> Result<(), Diagnostics> {
        if let Some(limits) = self.limits {
            if bytes > limits.max_string_bytes.saturating_sub(self.strings) {
                return Err(error(
                    "E3022",
                    format!(
                        "IR string byte limit of {} exceeded",
                        limits.max_string_bytes
                    ),
                    span,
                ));
            }
            self.strings += bytes;
        }
        Ok(())
    }
    fn instruction(&mut self, value: &Instruction, span: Span) -> Result<(), Diagnostics> {
        if let Some(limits) = self.limits {
            if self.instructions >= limits.max_instructions {
                return Err(error(
                    "E3021",
                    format!(
                        "IR instruction limit of {} exceeded",
                        limits.max_instructions
                    ),
                    span,
                ));
            }
            match value {
                Instruction::LoadLiteral(
                    Literal::Integer(text) | Literal::Float(text) | Literal::String(text),
                )
                | Instruction::LoadName(text)
                | Instruction::StoreName(text)
                | Instruction::Binary(text)
                | Instruction::LoadField(text) => self.string(text.len(), span)?,
                Instruction::Call { name, arguments: _ } => self.string(name.len(), span)?,
                Instruction::MakeStruct { type_name, fields } => {
                    self.string(type_name.len(), span)?;
                    for field in fields {
                        self.string(field.len(), span)?;
                    }
                }
                Instruction::LoadLiteral(Literal::Boolean(_))
                | Instruction::StoreIndex
                | Instruction::WidenFloat
                | Instruction::WidenFloatArray { depth: _ }
                | Instruction::MakeArray { length: _ }
                | Instruction::Index
                | Instruction::Jump { target: _ }
                | Instruction::JumpIfFalse { target: _ }
                | Instruction::Return
                | Instruction::Pop => {}
            }
            self.instructions += 1;
        }
        Ok(())
    }
}

pub(super) struct Instructions<'a> {
    values: Vec<Instruction>,
    budget: &'a mut Budget,
    pub(super) span: Span,
}
impl<'a> Instructions<'a> {
    pub(super) fn new(budget: &'a mut Budget, span: Span) -> Self {
        Self {
            values: Vec::new(),
            budget,
            span,
        }
    }
    pub(super) fn len(&self) -> usize {
        self.values.len()
    }
    pub(super) fn finish(self) -> Vec<Instruction> {
        self.values
    }
    pub(super) fn push(&mut self, mut instruction: Instruction) -> Result<(), Diagnostics> {
        match &mut instruction {
            Instruction::Call { name, .. } => {
                if let Some(linked) = self.budget.function_names.get(name) {
                    *name = linked.clone();
                }
            }
            Instruction::MakeStruct { type_name, .. } => {
                if let Some(linked) = self.budget.record_names.get(type_name) {
                    *type_name = linked.clone();
                }
            }
            _ => {}
        }
        self.budget.instruction(&instruction, self.span)?;
        self.values.push(instruction);
        Ok(())
    }
    // Backpatching changes only numeric jump targets; it cannot bypass string
    // or instruction accounting through mutable access to the underlying Vec.
    pub(super) fn patch_jump(&mut self, index: usize, target: usize) {
        match &mut self.values[index] {
            Instruction::Jump { target: slot } | Instruction::JumpIfFalse { target: slot } => {
                *slot = target
            }
            _ => unreachable!("lowering backpatches only previously emitted jumps"),
        }
    }
}

pub(super) fn error(code: &'static str, message: String, span: Span) -> Diagnostics {
    let mut errors = Diagnostics::new();
    errors.push_bounded(Diagnostic {
        severity: Severity::Error,
        code,
        message,
        span,
        source_file: None,
    });
    errors
}
