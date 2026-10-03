//! Type facts needed to preserve numeric widening during lowering.

use std::collections::{HashMap, HashSet};

use super::LoweredBinding;
use crate::compiler::ast::{Expression, ExpressionKind, Function, Program};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ValueType {
    Float,
    Record(String),
    Array(Box<ValueType>),
    Other,
}

#[derive(Debug)]
pub(super) struct TypeFacts {
    aliases: HashMap<String, String>,
    fields: HashMap<String, HashMap<String, String>>,
    returns: HashMap<String, String>,
}

impl TypeFacts {
    pub(super) fn new(program: &Program, imports: &[(String, &Function)]) -> Self {
        let mut facts = Self {
            aliases: HashMap::new(),
            fields: HashMap::new(),
            returns: HashMap::new(),
        };
        for declaration in &program.type_declarations {
            facts
                .aliases
                .insert(declaration.name.clone(), declaration.target.clone());
        }
        for record in &program.struct_declarations {
            facts.fields.insert(
                record.name.clone(),
                record
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.type_name.clone()))
                    .collect(),
            );
        }
        for function in &program.functions {
            facts.returns.insert(
                function.name.clone(),
                function.return_type.clone().unwrap_or_default(),
            );
        }
        for module in &program.modules {
            for declaration in &module.type_declarations {
                facts.aliases.insert(
                    format!("{}::{}", module.name, declaration.name),
                    declaration.target.clone(),
                );
            }
            for record in &module.struct_declarations {
                let fields: HashMap<_, _> = record
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.type_name.clone()))
                    .collect();
                facts
                    .fields
                    .insert(format!("{}::{}", module.name, record.name), fields);
            }
            for function in &module.functions {
                facts.returns.insert(
                    format!("{}::{}", module.name, function.name),
                    function.return_type.clone().unwrap_or_default(),
                );
            }
        }
        for (name, function) in imports {
            facts.returns.insert(
                name.clone(),
                function.return_type.clone().unwrap_or_default(),
            );
        }
        facts
    }

    pub(super) fn annotation(&self, name: &str) -> ValueType {
        let mut current = name;
        let mut seen = HashSet::new();
        while let Some(target) = self.aliases.get(current) {
            if !seen.insert(current) {
                return ValueType::Other;
            }
            current = target;
        }
        if current == "Float" {
            ValueType::Float
        } else if self.fields.contains_key(current) {
            ValueType::Record(current.to_owned())
        } else {
            ValueType::Other
        }
    }

    pub(super) fn expression(
        &self,
        expression: &Expression,
        locals: &HashMap<String, LoweredBinding>,
    ) -> ValueType {
        match &expression.kind {
            ExpressionKind::Float(_) => ValueType::Float,
            ExpressionKind::Identifier(name) => locals
                .get(name)
                .map(|binding| binding.value_type.clone())
                .unwrap_or(ValueType::Other),
            ExpressionKind::StructLiteral { type_name, .. } => self.annotation(type_name),
            ExpressionKind::Call { callee, .. } => {
                let name = match &callee.kind {
                    ExpressionKind::Identifier(name) => name.clone(),
                    ExpressionKind::QualifiedName { path } => path.join("::"),
                    _ => return ValueType::Other,
                };
                self.returns
                    .get(&name)
                    .map(|name| self.annotation(name))
                    .unwrap_or(ValueType::Other)
            }
            ExpressionKind::FieldAccess { receiver, field } => {
                let ValueType::Record(record) = self.expression(receiver, locals) else {
                    return ValueType::Other;
                };
                self.fields
                    .get(&record)
                    .and_then(|fields| fields.get(field))
                    .map(|name| self.annotation(name))
                    .unwrap_or(ValueType::Other)
            }
            ExpressionKind::ArrayLiteral(items) => {
                let mut element = ValueType::Other;
                for (index, item) in items.iter().enumerate() {
                    let value = self.expression(item, locals);
                    if index == 0 || value == ValueType::Float {
                        element = value;
                    }
                }
                ValueType::Array(Box::new(element))
            }
            ExpressionKind::Index { target, .. } => match self.expression(target, locals) {
                ValueType::Array(element) => *element,
                _ => ValueType::Other,
            },
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } if matches!(operator.as_str(), "+" | "-" | "*" | "/") => {
                if self.expression(left, locals) == ValueType::Float
                    || self.expression(right, locals) == ValueType::Float
                {
                    ValueType::Float
                } else {
                    ValueType::Other
                }
            }
            _ => ValueType::Other,
        }
    }
}
