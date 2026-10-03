//! Resolve local type spellings before checking bodies or lowering values.

use std::collections::{HashMap, HashSet};

use crate::compiler::ast::{Expression, ExpressionKind, Function, Program, Statement};
use crate::compiler::diagnostics::Diagnostics;

use super::{diagnostic, Type};

/// Preserve declaration names and spans, but qualify module-local references.
/// Existing same-file qualified access remains valid; package exports are
/// enforced by the package compiler rather than inferred from these names.
pub(super) fn resolve(program: &Program) -> Result<Program, Diagnostics> {
    let mut resolved = program.clone();
    let mut diagnostics = Diagnostics::new();
    for module in &mut resolved.modules {
        let local_names: HashSet<_> = module
            .type_declarations
            .iter()
            .map(|decl| decl.name.clone())
            .chain(
                module
                    .struct_declarations
                    .iter()
                    .map(|decl| decl.name.clone()),
            )
            .collect();
        for (name, span) in module
            .type_declarations
            .iter()
            .map(|decl| (&decl.name, decl.span))
            .chain(
                module
                    .struct_declarations
                    .iter()
                    .map(|decl| (&decl.name, decl.span)),
            )
        {
            if Type::from_name_with_known(name, None) != Type::Unknown {
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate type `{name}`"),
                    span,
                ));
            }
        }
        let qualify = |name: &mut String| {
            if local_names.contains(name) {
                *name = format!("{}::{name}", module.name);
            }
        };
        for declaration in &mut module.type_declarations {
            qualify(&mut declaration.target);
        }
        for record in &mut module.struct_declarations {
            for field in &mut record.fields {
                qualify(&mut field.type_name);
            }
        }
        for function in &mut module.functions {
            rewrite_function(function, &qualify);
        }
    }
    if diagnostics.is_empty() {
        Ok(resolved)
    } else {
        Err(diagnostics)
    }
}

/// Retain resolved alias targets for lowering and package interfaces.
pub(super) fn canonicalize_alias_targets(program: &mut Program, known: &HashMap<String, Type>) {
    let canonical = |name: &str, original: &str| match known.get(name) {
        Some(Type::Unit) => "Unit".to_owned(),
        Some(Type::Bool) => "Bool".to_owned(),
        Some(Type::Int) => "Int".to_owned(),
        Some(Type::Float) => "Float".to_owned(),
        Some(Type::String) => "String".to_owned(),
        // Keep root-record alias paths: replacing a root alias with a bare
        // record spelling could bind a shadowing module type on re-analysis.
        Some(Type::Named(identity)) if identity.contains("::") => identity.clone(),
        _ => original.to_owned(),
    };
    for alias in &mut program.type_declarations {
        alias.target = canonical(&alias.name, &alias.target);
    }
    for module in &mut program.modules {
        for alias in &mut module.type_declarations {
            alias.target = canonical(&format!("{}::{}", module.name, alias.name), &alias.target);
        }
    }
}

fn rewrite_function(function: &mut Function, qualify: &impl Fn(&mut String)) {
    for parameter in &mut function.parameters {
        if let Some(name) = &mut parameter.type_name {
            qualify(name);
        }
    }
    if let Some(name) = &mut function.return_type {
        qualify(name);
    }
    let mut statements: Vec<_> = function.body.iter_mut().collect();
    while let Some(statement) = statements.pop() {
        match statement {
            Statement::Let {
                type_name, value, ..
            } => {
                if let Some(name) = type_name {
                    qualify(name);
                }
                rewrite_expression(value, qualify);
            }
            Statement::Assign { target, value, .. } => {
                rewrite_expression(target, qualify);
                rewrite_expression(value, qualify);
            }
            Statement::Expression(value)
            | Statement::Return {
                value: Some(value), ..
            } => rewrite_expression(value, qualify),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                rewrite_expression(condition, qualify);
                statements.extend(then_block);
                if let Some(body) = else_block {
                    statements.extend(body);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                rewrite_expression(condition, qualify);
                statements.extend(body);
            }
            Statement::Return { value: None, .. } => {}
        }
    }
}

fn rewrite_expression(expression: &mut Expression, qualify: &impl Fn(&mut String)) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &mut expression.kind {
            ExpressionKind::StructLiteral { type_name, fields } => {
                qualify(type_name);
                pending.extend(fields.iter_mut().map(|(_, value)| value));
            }
            ExpressionKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExpressionKind::Call { callee, arguments } => {
                pending.push(callee);
                pending.extend(arguments);
            }
            ExpressionKind::FieldAccess { receiver, .. } => pending.push(receiver),
            ExpressionKind::ArrayLiteral(items) => pending.extend(items),
            ExpressionKind::Index { target, index } => {
                pending.push(target);
                pending.push(index);
            }
            _ => {}
        }
    }
}
