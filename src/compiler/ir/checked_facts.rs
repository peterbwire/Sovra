//! Lowering adapter for immutable compiler-owned facts; no type inference.

use super::{
    type_facts::{TypeFacts, ValueType},
    LoweredBinding,
};
use crate::compiler::{
    ast::{Expression, ExpressionKind, Function, Parameter, Statement},
    checked::{CheckedUnit, CoercionKind, NodeId, NodeKind, RecordedType, UseTarget},
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Debug)]
pub(super) struct CheckedFacts {
    unit: CheckedUnit,
    links: HashMap<(NodeKind, usize), NodeId>,
    values: HashMap<NodeId, ValueType>,
    results: HashMap<NodeId, ValueType>,
    contexts: HashMap<(NodeId, NodeId, CoercionKind), ValueType>,
    uses: HashMap<NodeId, UseTarget>,
    functions: HashMap<NodeId, (Option<usize>, usize)>,
    bindings: RefCell<HashMap<NodeId, String>>,
    imported_calls: HashMap<NodeId, Arc<str>>,
}

impl CheckedFacts {
    fn new(unit: &CheckedUnit) -> Self {
        let evidence = unit
            .semantic_evidence()
            .expect("checked lowering requires types");
        let flow = evidence
            .flow()
            .expect("checked lowering requires resolved uses");
        let lowering = evidence
            .lowering()
            .expect("checked lowering requires contexts");
        let mut types: Vec<ValueType> = Vec::with_capacity(evidence.types().len());
        for ty in evidence.types() {
            types.push(match ty {
                RecordedType::Float => ValueType::Float,
                RecordedType::Named(name) => ValueType::Record(Arc::clone(name)),
                RecordedType::Array(element) => {
                    ValueType::Array(Arc::new(types[element.ordinal()].clone()))
                }
                _ => ValueType::Other,
            });
        }
        let mut result = Self {
            unit: unit.clone(),
            links: unit.node_links(),
            values: evidence
                .facts()
                .iter()
                .map(|fact| (fact.node, types[fact.ty.ordinal()].clone()))
                .collect(),
            results: lowering
                .function_results()
                .iter()
                .map(|fact| (fact.function, types[fact.ty.ordinal()].clone()))
                .collect(),
            contexts: lowering
                .contexts()
                .iter()
                .map(|context| {
                    (
                        (context.owner, context.value, context.kind),
                        types[context.expected.ordinal()].clone(),
                    )
                })
                .collect(),
            uses: flow
                .uses()
                .iter()
                .map(|usage| (usage.node, usage.target))
                .collect(),
            functions: HashMap::new(),
            bindings: RefCell::new(HashMap::new()),
            imported_calls: evidence.package().map_or_else(HashMap::new, |package| {
                package
                    .calls()
                    .iter()
                    .map(|call| {
                        (
                            call.node,
                            Arc::clone(&package.imports()[call.import.ordinal()].symbol),
                        )
                    })
                    .collect()
            }),
        };
        for (index, function) in unit.program().functions.iter().enumerate() {
            result
                .functions
                .insert(result.id(function, NodeKind::Function), (None, index));
        }
        for (module, declaration) in unit.program().modules.iter().enumerate() {
            for (index, function) in declaration.functions.iter().enumerate() {
                result.functions.insert(
                    result.id(function, NodeKind::Function),
                    (Some(module), index),
                );
            }
        }
        result
    }

    fn id<T>(&self, node: &T, kind: NodeKind) -> NodeId {
        self.links[&(kind, node as *const T as usize)]
    }

    pub(super) fn expression(&self, expression: &Expression) -> ValueType {
        if let Some(ty) = self.values.get(&self.id(expression, NodeKind::Expression)) {
            return ty.clone();
        }
        // Assignment index roots are checked directly; their receiver has a fact.
        if let ExpressionKind::Index { target, .. } = &expression.kind {
            if let ValueType::Array(element) = self.expression(target) {
                return element.as_ref().clone();
            }
        }
        panic!("checked value expression has no retained type");
    }

    fn declared<T>(&self, declaration: &T, kind: NodeKind) -> ValueType {
        self.values[&self.id(declaration, kind)].clone()
    }

    fn register<T>(&self, declaration: &T, kind: NodeKind, name: &str) {
        self.bindings
            .borrow_mut()
            .insert(self.id(declaration, kind), name.to_owned());
    }

    fn target(&self, expression: &Expression) -> UseTarget {
        self.uses[&self.id(expression, NodeKind::Expression)]
    }

    fn value_name(&self, expression: &Expression) -> String {
        let UseTarget::Declaration(id) = self.target(expression) else {
            panic!("checked value is a binding");
        };
        self.bindings.borrow()[&id].clone()
    }

    fn call_name(&self, expression: &Expression) -> String {
        if let Some(symbol) = self
            .imported_calls
            .get(&self.id(expression, NodeKind::Expression))
        {
            return symbol.to_string();
        }
        match self.target(expression) {
            UseTarget::Builtin(name) => {
                // Preserve the established IR spelling of the compatibility alias.
                let bare_print = match &expression.kind {
                    ExpressionKind::Identifier(source) => source == "print",
                    ExpressionKind::QualifiedName { path } => path.len() == 1 && path[0] == "print",
                    _ => false,
                };
                if name == "std::print" && bare_print {
                    "print".to_owned()
                } else {
                    name.to_owned()
                }
            }
            UseTarget::Declaration(id) => match self.functions[&id] {
                (None, index) => self.unit.program().functions[index].name.clone(),
                (Some(module), index) => {
                    let module = &self.unit.program().modules[module];
                    format!("{}::{}", module.name, module.functions[index].name)
                }
            },
        }
    }
}

impl TypeFacts {
    pub(super) fn from_checked(unit: &CheckedUnit) -> Self {
        Self {
            aliases: HashMap::new(),
            fields: HashMap::new(),
            returns: HashMap::new(),
            checked: Some(CheckedFacts::new(unit)),
        }
    }

    pub(super) fn parameter_type(&self, parameter: &Parameter) -> ValueType {
        self.checked.as_ref().map_or_else(
            || self.annotation(parameter.type_name.as_deref().unwrap_or("")),
            |facts| facts.declared(parameter, NodeKind::Parameter),
        )
    }

    pub(super) fn register_parameter(&self, parameter: &Parameter) {
        if let Some(facts) = &self.checked {
            facts.register(parameter, NodeKind::Parameter, &parameter.name);
        }
    }

    pub(super) fn binding_type(
        &self,
        statement: &Statement,
        annotation: Option<&str>,
        value: &Expression,
        locals: &HashMap<String, LoweredBinding>,
    ) -> ValueType {
        self.checked.as_ref().map_or_else(
            || {
                annotation
                    .map(|name| self.annotation(name))
                    .unwrap_or_else(|| self.expression(value, locals))
            },
            |facts| facts.declared(statement, NodeKind::Statement),
        )
    }

    pub(super) fn register_binding(&self, statement: &Statement, name: &str) {
        if let Some(facts) = &self.checked {
            facts.register(statement, NodeKind::Statement, name);
        }
    }

    pub(super) fn parameter_is_float(
        &self,
        parameter: &Parameter,
        aliases: &HashSet<String>,
    ) -> bool {
        if self.checked.is_some() {
            self.parameter_type(parameter) == ValueType::Float
        } else {
            super::is_float_type(parameter.type_name.as_deref(), aliases)
        }
    }

    pub(super) fn function_type(
        &self,
        function: &Function,
        aliases: &HashSet<String>,
    ) -> ValueType {
        if let Some(facts) = &self.checked {
            facts.results[&facts.id(function, NodeKind::Function)].clone()
        } else {
            if super::is_float_type(function.return_type.as_deref(), aliases) {
                ValueType::Float
            } else {
                self.annotation(function.return_type.as_deref().unwrap_or(""))
            }
        }
    }

    pub(super) fn value_name(
        &self,
        expression: &Expression,
        name: &str,
        locals: &HashMap<String, LoweredBinding>,
    ) -> String {
        self.checked.as_ref().map_or_else(
            || {
                locals
                    .get(name)
                    .map(|binding| binding.name.clone())
                    .unwrap_or_else(|| name.to_owned())
            },
            |facts| facts.value_name(expression),
        )
    }

    pub(super) fn call_name(&self, callee: &Expression) -> Option<String> {
        if let Some(facts) = &self.checked {
            return Some(facts.call_name(callee));
        }
        match &callee.kind {
            ExpressionKind::Identifier(name) => Some(name.clone()),
            ExpressionKind::QualifiedName { path } => Some(path.join("::")),
            _ => None,
        }
    }

    pub(super) fn assignment_type(
        &self,
        target: &Expression,
        name: &str,
        locals: &HashMap<String, LoweredBinding>,
    ) -> Option<ValueType> {
        if let Some(facts) = &self.checked {
            let UseTarget::Declaration(id) = facts.target(target) else {
                panic!("assignment binding target");
            };
            Some(facts.values[&id].clone())
        } else {
            locals.get(name).map(|binding| binding.value_type.clone())
        }
    }

    pub(super) fn growth_argument_type(
        &self,
        owner: &Expression,
        value: &Expression,
        first: &Expression,
        append: bool,
        locals: &HashMap<String, LoweredBinding>,
    ) -> ValueType {
        if let Some(facts) = &self.checked {
            facts.contexts[&(
                facts.id(owner, NodeKind::Expression),
                facts.id(value, NodeKind::Expression),
                CoercionKind::Argument,
            )]
                .clone()
        } else {
            let kind = self.expression(first, locals);
            if append {
                match kind {
                    ValueType::Array(element) => element.as_ref().clone(),
                    _ => ValueType::Other,
                }
            } else {
                kind
            }
        }
    }

    pub(super) fn record_name(&self, expression: &Expression, name: &str) -> String {
        if let Some(facts) = &self.checked {
            let ValueType::Record(name) = facts.expression(expression) else {
                panic!("checked record identity");
            };
            name.to_string()
        } else {
            self.canonical_name(name).to_owned()
        }
    }

    pub(super) fn field_type(
        &self,
        owner: &Expression,
        value: &Expression,
        record: &str,
        field: &str,
        legacy: &HashMap<String, Arc<HashSet<String>>>,
    ) -> ValueType {
        if let Some(facts) = &self.checked {
            facts.contexts[&(
                facts.id(owner, NodeKind::Expression),
                facts.id(value, NodeKind::Expression),
                CoercionKind::RecordField,
            )]
                .clone()
        } else if legacy
            .get(record)
            .is_some_and(|fields| fields.contains(field))
        {
            ValueType::Float
        } else {
            self.fields
                .get(self.canonical_name(record))
                .and_then(|fields| fields.get(field))
                .map_or(ValueType::Other, |name| self.annotation(name))
        }
    }
}
