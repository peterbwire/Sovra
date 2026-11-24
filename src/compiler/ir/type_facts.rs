//! Type facts needed to preserve numeric widening during lowering.

use std::{collections::HashMap, sync::Arc};

use super::LoweredBinding;
use crate::compiler::ast::{Expression, ExpressionKind, Function, Program};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ValueType {
    Float,
    Record(Arc<str>),
    Array(Arc<ValueType>),
    Other,
}

#[derive(Debug)]
pub(super) struct TypeFacts {
    pub(super) aliases: HashMap<String, Arc<str>>,
    pub(super) fields: HashMap<Arc<str>, HashMap<String, String>>,
    pub(super) returns: HashMap<String, String>,
    pub(super) checked: Option<super::checked_facts::CheckedFacts>,
}

// Resolve the functional alias graph iteratively. Alias members of a cycle
// retain their own spelling; incoming paths retain the first cycle member they
// encounter, matching the previous per-lookup walker on trusted-stage inputs.
fn canonical_aliases(targets: &HashMap<String, String>) -> HashMap<String, Arc<str>> {
    let mut resolved: HashMap<String, Arc<str>> = HashMap::new();
    let mut terminals: HashMap<&str, Arc<str>> = HashMap::new();
    for start in targets.keys() {
        if resolved.contains_key(start) {
            continue;
        }
        let mut current = start.as_str();
        let mut path: Vec<&str> = Vec::new();
        let mut visiting = HashMap::new();
        let terminal = loop {
            if let Some(terminal) = resolved.get(current) {
                break Arc::clone(terminal);
            }
            if let Some(&cycle_start) = visiting.get(current) {
                for alias in path.drain(cycle_start..) {
                    resolved.insert(alias.to_owned(), Arc::from(alias));
                }
                break Arc::clone(&resolved[current]);
            }
            let Some(target) = targets.get(current) else {
                break Arc::clone(
                    terminals
                        .entry(current)
                        .or_insert_with(|| Arc::from(current)),
                );
            };
            visiting.insert(current, path.len());
            path.push(current);
            current = target;
        };
        for alias in path {
            resolved.insert(alias.to_owned(), Arc::clone(&terminal));
        }
    }
    resolved
}

impl TypeFacts {
    pub(super) fn new(program: &Program, imports: &[(String, &Function)]) -> Self {
        let mut aliases = HashMap::new();
        let mut facts = Self {
            aliases: HashMap::new(),
            fields: HashMap::new(),
            returns: HashMap::new(),
            checked: None,
        };
        for declaration in &program.type_declarations {
            aliases.insert(declaration.name.clone(), declaration.target.clone());
        }
        for record in &program.struct_declarations {
            facts.fields.insert(
                Arc::from(record.name.as_str()),
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
                aliases.insert(
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
                facts.fields.insert(
                    Arc::from(format!("{}::{}", module.name, record.name)),
                    fields,
                );
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
        facts.aliases = canonical_aliases(&aliases);
        facts
    }

    // Aliases name the same declaration; they must not become runtime identities.
    pub(super) fn canonical_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases
            .get(name)
            .map_or(name, |terminal| terminal.as_ref())
    }

    pub(super) fn annotation(&self, name: &str) -> ValueType {
        let mut current = name;
        let mut arrays = 0;
        loop {
            current = self.canonical_name(current);
            let Some((depth, leaf)) = crate::compiler::semantic::array_parts(current) else {
                return ValueType::Other;
            };
            arrays += depth;
            if arrays >= crate::compiler::limits::MAX_STRUCTURAL_DEPTH {
                return ValueType::Other;
            }
            if depth == 0 {
                break;
            }
            current = leaf;
        }
        let mut kind = if current == "Float" {
            ValueType::Float
        } else if let Some((identity, _)) = self.fields.get_key_value(current) {
            ValueType::Record(Arc::clone(identity))
        } else {
            ValueType::Other
        };
        for _ in 0..arrays {
            kind = ValueType::Array(Arc::new(kind));
        }
        kind
    }

    pub(super) fn expression(
        &self,
        expression: &Expression,
        locals: &HashMap<String, LoweredBinding>,
    ) -> ValueType {
        if let Some(checked) = &self.checked {
            return checked.expression(expression);
        }
        match &expression.kind {
            ExpressionKind::Float(_) => ValueType::Float,
            ExpressionKind::Identifier(name) => locals
                .get(name)
                .map(|binding| binding.value_type.clone())
                .unwrap_or(ValueType::Other),
            ExpressionKind::StructLiteral { type_name, .. } => self.annotation(type_name),
            ExpressionKind::Call { callee, arguments } => {
                let name = match &callee.kind {
                    ExpressionKind::Identifier(name) => name.clone(),
                    ExpressionKind::QualifiedName { path } => path.join("::"),
                    _ => return ValueType::Other,
                };
                if matches!(
                    name.as_str(),
                    "std::array_slice" | "std::array_append" | "std::array_concat"
                ) {
                    return arguments
                        .first()
                        .map_or(ValueType::Other, |value| self.expression(value, locals));
                }
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
                for item in items {
                    let value = self.expression(item, locals);
                    element = refine_array_facts(element, value);
                }
                ValueType::Array(Arc::new(element))
            }
            ExpressionKind::Index { target, .. } => match self.expression(target, locals) {
                ValueType::Array(element) => element.as_ref().clone(),
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

// Semantic checking already guarantees compatibility. Retain widening facts
// through empty nested rows instead of treating the first row as definitive.
fn refine_array_facts(current: ValueType, observed: ValueType) -> ValueType {
    match (current, observed) {
        (ValueType::Other, observed) => observed,
        (ValueType::Array(current), ValueType::Array(observed)) => ValueType::Array(Arc::new(
            refine_array_facts(current.as_ref().clone(), observed.as_ref().clone()),
        )),
        (_, ValueType::Float) => ValueType::Float,
        (current, _) => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn walk<'a>(targets: &'a HashMap<String, String>, name: &'a str) -> &'a str {
        let mut current = name;
        let mut seen = HashSet::new();
        while let Some(target) = targets.get(current) {
            if !seen.insert(current) {
                break;
            }
            current = target;
        }
        current
    }

    #[test]
    fn canonical_table_matches_original_walk_for_cycles_tails_and_missing_targets() {
        let mut state = 29u64;
        for _ in 0..32 {
            let mut targets = HashMap::new();
            for index in 0..256 {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let target = match state % 19 {
                    0 => "Float".to_owned(),
                    1 => "Missing".to_owned(),
                    2 => "Point".to_owned(),
                    _ => format!("Alias{}", (state >> 16) % 256),
                };
                targets.insert(format!("Alias{index}"), target);
            }
            targets.insert("Self".into(), "Self".into());
            targets.insert("CycleA".into(), "CycleB".into());
            targets.insert("CycleB".into(), "CycleA".into());
            targets.insert("Tail".into(), "CycleB".into());
            let table = canonical_aliases(&targets);
            assert_eq!(table.len(), targets.len());
            for name in targets.keys() {
                assert_eq!(table[name].as_ref(), walk(&targets, name), "{name}");
            }
        }
    }

    #[test]
    fn long_chains_and_fanout_share_a_single_terminal_string() {
        let terminal = format!("@package1::type::api::Record{}", "x".repeat(4096));
        let mut targets = HashMap::new();
        for index in 0..10000 {
            let next = if index == 9999 {
                terminal.clone()
            } else {
                format!("Chain{}", index + 1)
            };
            targets.insert(format!("Chain{index}"), next);
            targets.insert(format!("Fanout{index}"), terminal.clone());
        }
        let table = canonical_aliases(&targets);
        let first = &table["Chain0"];
        assert_eq!(first.as_ref(), terminal);
        for value in table.values() {
            assert!(Arc::ptr_eq(first, value));
        }
    }

    #[test]
    fn canonical_lookup_keeps_unknown_spellings_and_is_fresh_per_lowering() {
        let parser = crate::compiler::parser::Parser::new();
        let first = parser
            .parse_source("type Number = Float; fn main() {}")
            .unwrap();
        let second = parser
            .parse_source("type Number = Int; fn main() {}")
            .unwrap();
        let first = TypeFacts::new(&first, &[]);
        let second = TypeFacts::new(&second, &[]);
        assert_eq!(first.canonical_name("Number"), "Float");
        assert_eq!(second.canonical_name("Number"), "Int");
        assert_eq!(first.canonical_name("Unknown"), "Unknown");
        assert_eq!(first.annotation("Number"), ValueType::Float);
        assert_eq!(second.annotation("Number"), ValueType::Other);
    }

    #[test]
    fn record_facts_share_identity_storage_through_aliases_and_local_copies() {
        let name = format!("Record{}", "x".repeat(4096));
        let source = format!("struct {name} {{ value: Float }} type Alias = {name}; mod area {{ struct Other {{ value: Int }} }}");
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(&source)
            .unwrap();
        let facts = TypeFacts::new(&parsed, &[]);
        let ValueType::Record(identity) = facts.annotation(&name) else {
            panic!("expected a record fact");
        };
        let ValueType::Record(alias) = facts.annotation("Alias") else {
            panic!("expected an alias fact");
        };
        assert!(Arc::ptr_eq(&identity, &alias));
        assert!(Arc::ptr_eq(
            &identity,
            facts.fields.get_key_value(name.as_str()).unwrap().0
        ));
        let ValueType::Record(other) = facts.annotation("area::Other") else {
            panic!("expected a module record fact");
        };
        assert!(!Arc::ptr_eq(&identity, &other));
        let expression = crate::compiler::ast::Expression {
            kind: ExpressionKind::Identifier("p".into()),
            span: parsed.struct_declarations[0].span,
        };
        let locals = HashMap::from([(
            "p".into(),
            LoweredBinding {
                name: "p".into(),
                value_type: ValueType::Record(Arc::clone(&identity)),
            },
        )]);
        let copies: Vec<_> = (0..10000)
            .map(|_| facts.expression(&expression, &locals))
            .collect();
        for copy in &copies {
            let ValueType::Record(copy) = copy else {
                panic!("expected a copied record fact");
            };
            assert!(Arc::ptr_eq(&identity, copy));
            assert_eq!(copy.as_ref(), name);
        }
        // New lowering contexts must not retain process-wide interning state.
        let fresh = TypeFacts::new(&parsed, &[]);
        let ValueType::Record(fresh) = fresh.annotation(&name) else {
            panic!("expected fresh record fact");
        };
        assert!(!Arc::ptr_eq(&identity, &fresh));
    }
}
