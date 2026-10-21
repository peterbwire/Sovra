//! Span-free AST comparison without changing public compiler types.

use crate::compiler::ast::*;
use crate::compiler::diagnostics::Span;

const ZERO: Span = Span {
    start: 0,
    end: 0,
    line: 0,
    column: 0,
};

pub(super) fn normalize(program: &mut Program) {
    declarations(
        &mut program.type_declarations,
        &mut program.struct_declarations,
    );
    for function in &mut program.functions {
        normalize_function(function);
    }
    for module in &mut program.modules {
        module.span = ZERO;
        declarations(
            &mut module.type_declarations,
            &mut module.struct_declarations,
        );
        for function in &mut module.functions {
            normalize_function(function);
        }
    }
}

fn declarations(aliases: &mut [TypeDeclaration], records: &mut [StructDeclaration]) {
    for alias in aliases {
        alias.span = ZERO;
    }
    for record in records {
        record.span = ZERO;
        for field in &mut record.fields {
            field.span = ZERO;
            field.type_span = field.type_span.map(|_| ZERO);
        }
    }
}

fn normalize_function(function: &mut Function) {
    function.span = ZERO;
    function.return_type_span = function.return_type_span.map(|_| ZERO);
    for parameter in &mut function.parameters {
        parameter.span = ZERO;
        parameter.type_span = parameter.type_span.map(|_| ZERO);
    }
    statements(&mut function.body);
}

fn statements(body: &mut [Statement]) {
    for statement in body {
        match statement {
            Statement::Let {
                value,
                span,
                type_span,
                ..
            } => {
                *span = ZERO;
                *type_span = type_span.map(|_| ZERO);
                expression(value);
            }
            Statement::Assign {
                target,
                value,
                span,
            } => {
                *span = ZERO;
                expression(target);
                expression(value);
            }
            Statement::Return { value, span } => {
                *span = ZERO;
                if let Some(value) = value {
                    expression(value);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                *span = ZERO;
                expression(condition);
                statements(then_block);
                if let Some(body) = else_block {
                    statements(body);
                }
            }
            Statement::While {
                condition,
                body,
                span,
            } => {
                *span = ZERO;
                expression(condition);
                statements(body);
            }
            Statement::Expression(value) => expression(value),
        }
    }
}

fn expression(value: &mut Expression) {
    value.span = ZERO;
    match &mut value.kind {
        ExpressionKind::Binary { left, right, .. } => {
            expression(left);
            expression(right);
        }
        ExpressionKind::Call { callee, arguments } => {
            expression(callee);
            for argument in arguments {
                expression(argument);
            }
        }
        ExpressionKind::StructLiteral { fields, .. } => {
            for (_, field) in fields {
                expression(field);
            }
        }
        ExpressionKind::ArrayLiteral(items) => {
            for item in items {
                expression(item);
            }
        }
        ExpressionKind::FieldAccess { receiver, .. } => expression(receiver),
        ExpressionKind::Index { target, index } => {
            expression(target);
            expression(index);
        }
        ExpressionKind::String(_)
        | ExpressionKind::Integer(_)
        | ExpressionKind::Float(_)
        | ExpressionKind::Boolean(_)
        | ExpressionKind::Identifier(_)
        | ExpressionKind::QualifiedName { .. } => {}
    }
}
