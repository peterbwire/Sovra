//! Borrowed AST admission checks using iterator frames, not a wide cloned queue.

use crate::compiler::{
    ast::{Expression, ExpressionKind, Function, Program, Statement, StructDeclaration},
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
};

/// Count AST nodes within a caller-selected allowance before semantic cloning.
/// Modules, aliases, records/fields, functions/parameters, statements and expressions
/// each count one; each array annotation wrapper also counts one. The Program
/// and collection containers do not. Iterator frames
/// avoid allocating a work queue proportional to a wide body. This does not bound
/// parsing, repeated semantic passes, string sizes or caller-owned AST destruction.
pub fn check_ast_budget(program: &Program, max_nodes: usize) -> Result<usize, Diagnostics> {
    struct Counter {
        remaining: usize,
        limit: usize,
    }
    impl Counter {
        fn charge(&mut self, span: Span) -> Result<(), Diagnostics> {
            if let Some(remaining) = self.remaining.checked_sub(1) {
                self.remaining = remaining;
                return Ok(());
            }
            let mut errors = Diagnostics::new();
            errors.push_bounded(Diagnostic {
                severity: Severity::Error,
                code: "E3020",
                message: format!("AST node limit of {} exceeded", self.limit),
                span,
                source_file: None,
            });
            Err(errors)
        }
        fn annotation(&mut self, name: &str, span: Span) -> Result<(), Diagnostics> {
            for _ in name.bytes().take_while(|byte| *byte == b'[') {
                self.charge(span)?;
            }
            Ok(())
        }
        fn record(&mut self, record: &StructDeclaration) -> Result<(), Diagnostics> {
            self.charge(record.span)?;
            for field in &record.fields {
                self.charge(field.span)?;
                self.annotation(&field.type_name, field.type_span.unwrap_or(field.span))?;
            }
            Ok(())
        }
        fn function(&mut self, function: &Function) -> Result<(), Diagnostics> {
            self.charge(function.span)?;
            for parameter in &function.parameters {
                self.charge(parameter.span)?;
                if let Some(name) = &parameter.type_name {
                    self.annotation(name, parameter.type_span.unwrap_or(parameter.span))?;
                }
            }
            if let Some(name) = &function.return_type {
                self.annotation(name, function.return_type_span.unwrap_or(function.span))?;
            }
            enum Work<'a> {
                Statement(&'a Statement),
                Statements(std::slice::Iter<'a, Statement>),
                Expression(&'a Expression),
                Expressions(std::slice::Iter<'a, Expression>),
                Fields(std::slice::Iter<'a, (String, Expression)>),
            }
            let mut pending = vec![Work::Statements(function.body.iter())];
            while let Some(work) = pending.pop() {
                match work {
                    Work::Statements(mut values) => {
                        if let Some(value) = values.next() {
                            pending.push(Work::Statements(values));
                            pending.push(Work::Statement(value));
                        }
                    }
                    Work::Expressions(mut values) => {
                        if let Some(value) = values.next() {
                            pending.push(Work::Expressions(values));
                            pending.push(Work::Expression(value));
                        }
                    }
                    Work::Fields(mut values) => {
                        if let Some((_, value)) = values.next() {
                            pending.push(Work::Fields(values));
                            pending.push(Work::Expression(value));
                        }
                    }
                    Work::Statement(statement) => {
                        let span = match statement {
                            Statement::Expression(value) => value.span,
                            Statement::Let { span, .. }
                            | Statement::Assign { span, .. }
                            | Statement::Return { span, .. }
                            | Statement::If { span, .. }
                            | Statement::While { span, .. } => *span,
                        };
                        self.charge(span)?;
                        if let Statement::Let {
                            type_name: Some(name),
                            type_span,
                            ..
                        } = statement
                        {
                            self.annotation(name, type_span.unwrap_or(span))?;
                        }
                        match statement {
                            Statement::Let { value, .. } | Statement::Expression(value) => {
                                pending.push(Work::Expression(value))
                            }
                            Statement::Assign { target, value, .. } => {
                                pending.push(Work::Expression(value));
                                pending.push(Work::Expression(target));
                            }
                            Statement::Return { value, .. } => {
                                if let Some(value) = value {
                                    pending.push(Work::Expression(value));
                                }
                            }
                            Statement::If {
                                condition,
                                then_block,
                                else_block,
                                ..
                            } => {
                                if let Some(body) = else_block {
                                    pending.push(Work::Statements(body.iter()));
                                }
                                pending.push(Work::Statements(then_block.iter()));
                                pending.push(Work::Expression(condition));
                            }
                            Statement::While {
                                condition, body, ..
                            } => {
                                pending.push(Work::Statements(body.iter()));
                                pending.push(Work::Expression(condition));
                            }
                        }
                    }
                    Work::Expression(expression) => {
                        self.charge(expression.span)?;
                        match &expression.kind {
                            ExpressionKind::Binary { left, right, .. } => {
                                pending.push(Work::Expression(right));
                                pending.push(Work::Expression(left));
                            }
                            ExpressionKind::Call { callee, arguments } => {
                                pending.push(Work::Expressions(arguments.iter()));
                                pending.push(Work::Expression(callee));
                            }
                            ExpressionKind::FieldAccess { receiver, .. } => {
                                pending.push(Work::Expression(receiver))
                            }
                            ExpressionKind::StructLiteral { fields, .. } => {
                                pending.push(Work::Fields(fields.iter()))
                            }
                            ExpressionKind::ArrayLiteral(values) => {
                                pending.push(Work::Expressions(values.iter()))
                            }
                            ExpressionKind::Index { target, index } => {
                                pending.push(Work::Expression(index));
                                pending.push(Work::Expression(target));
                            }
                            ExpressionKind::String(_)
                            | ExpressionKind::Integer(_)
                            | ExpressionKind::Float(_)
                            | ExpressionKind::Boolean(_)
                            | ExpressionKind::Identifier(_)
                            | ExpressionKind::QualifiedName { path: _ } => {}
                        }
                    }
                }
            }
            Ok(())
        }
    }
    let mut counter = Counter {
        remaining: max_nodes,
        limit: max_nodes,
    };
    for alias in &program.type_declarations {
        counter.charge(alias.span)?;
        counter.annotation(&alias.target, alias.span)?;
    }
    for record in &program.struct_declarations {
        counter.record(record)?;
    }
    for function in &program.functions {
        counter.function(function)?;
    }
    for module in &program.modules {
        counter.charge(module.span)?;
        for alias in &module.type_declarations {
            counter.charge(alias.span)?;
            counter.annotation(&alias.target, alias.span)?;
        }
        for record in &module.struct_declarations {
            counter.record(record)?;
        }
        for function in &module.functions {
            counter.function(function)?;
        }
    }
    Ok(max_nodes - counter.remaining)
}
