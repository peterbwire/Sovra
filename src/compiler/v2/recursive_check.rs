//! Scoped recursive-arm checking against the same callable/type evidence recorder.
use super::*;
use crate::compiler::{
    ast::Function,
    checked::Recorder,
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    semantic::{DetachedScope, Type},
};
use std::collections::HashMap;
pub(super) struct RecursiveChecker<'a, 'f> {
    pub env: &'a CheckedEnumTypes,
    pub label: &'a str,
    pub expected: &'a Type,
    pub known: &'a HashMap<String, Type>,
    pub fields: &'a HashMap<String, HashMap<String, Type>>,
    pub functions: &'a HashMap<String, &'f Function>,
    pub recorder: &'a mut Recorder,
    pub plans: &'a mut Vec<EnumMatchPlan>,
    pub copies: &'a mut usize,
    pub limits: EnumFunctionLimits,
}
fn invalid(message: &'static str, span: Span) -> EnumFunctionError {
    EnumFunctionError::Diagnostics(Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code: "E3108",
            message: message.into(),
            span,
        }],
    })
}
impl RecursiveChecker<'_, '_> {
    fn body(
        &mut self,
        body: &[EnumFunctionStatement],
        scope: &mut DetachedScope,
    ) -> Result<bool, EnumFunctionError> {
        let mut returns = false;
        for statement in body {
            returns |= self.statement(statement, scope)?;
        }
        Ok(returns)
    }
    pub fn statement(
        &mut self,
        statement: &EnumFunctionStatement,
        scope: &mut DetachedScope,
    ) -> Result<bool, EnumFunctionError> {
        match statement {
            EnumFunctionStatement::Core(s) => scope
                .check_recorded(
                    s,
                    self.expected,
                    self.known,
                    self.fields,
                    self.functions,
                    self.recorder,
                )
                .map_err(EnumFunctionError::Diagnostics),
            EnumFunctionStatement::RecursiveMatch(s) => self.check(s, scope),
            EnumFunctionStatement::Control(s) => {
                use crate::compiler::ast::Statement;
                let (is_if, expected) = match &s.header {
                    Statement::If { else_block, .. } => {
                        (true, if else_block.is_some() { 2 } else { 1 })
                    }
                    Statement::While { .. } => (false, 1),
                    _ => return Err(EnumFunctionError::Limit("control header")),
                };
                if s.bodies.len() != expected {
                    return Err(EnumFunctionError::Limit("control body coverage"));
                }
                scope
                    .check_control_condition(
                        &s.header,
                        self.known,
                        self.fields,
                        self.functions,
                        self.recorder,
                    )
                    .map_err(EnumFunctionError::Diagnostics)?;
                let mut all = true;
                for body in &s.bodies {
                    self.recorder.enter_scope();
                    let mut child = scope.fork();
                    all &= self.body(body, &mut child)?;
                    self.recorder.exit_scope();
                }
                Ok(is_if && expected == 2 && all)
            }
            EnumFunctionStatement::Match(_) => {
                Err(EnumFunctionError::Limit("mixed recursive match bodies"))
            }
        }
    }
    pub fn check(
        &mut self,
        syntax: &RecursiveMatchSyntax,
        scope: &mut DetachedScope,
    ) -> Result<bool, EnumFunctionError> {
        let header = &syntax.header;
        if syntax.bodies.len() != header.arms.len() {
            return Err(EnumFunctionError::Limit("recursive arm coverage"));
        }
        let Some(Type::Named(name)) = scope.get(&header.scrutinee) else {
            return Err(invalid(
                "match scrutinee must be a bound enum",
                header.scrutinee_span,
            ));
        };
        let id = self
            .env
            .find(name)
            .ok_or_else(|| invalid("match scrutinee is not an enum", header.scrutinee_span))?;
        let arms: Vec<_> = header
            .arms
            .iter()
            .map(|a| EnumMatchArm {
                enum_name: &a.enum_name,
                variant_name: &a.variant_name,
                has_payload_binding: a.binding.is_some(),
                span: a.span,
            })
            .collect();
        let plan = self
            .env
            .check_match(id, self.label, header.span, &arms)
            .map_err(EnumFunctionError::Coverage)?;
        let count = scope
            .len()
            .checked_mul(header.arms.len())
            .and_then(|n| n.checked_add(header.arms.iter().filter(|a| a.binding.is_some()).count()))
            .and_then(|n| n.checked_add(*self.copies))
            .ok_or(EnumFunctionError::Limit("scope bindings"))?;
        if count > self.limits.max_scope_bindings {
            return Err(EnumFunctionError::Limit("scope bindings"));
        }
        *self.copies = count;
        let index = self.plans.len();
        let variants = plan.variant_order().to_vec();
        self.plans.push(plan);
        self.recorder
            .detached_scrutinee(&header.scrutinee, header.scrutinee_span, index);
        let mut all_return = true;
        for ((arm, body), variant) in header.arms.iter().zip(&syntax.bodies).zip(variants) {
            let payload = self.env.declarations()[id.ordinal()].variants[variant]
                .payload
                .map(|id| self.env.body_payload_type(id))
                .transpose()
                .map_err(EnumFunctionError::Coverage)?;
            self.recorder.enter_scope();
            let mut child = scope.fork();
            if let (Some(name), Some(ty)) = (&arm.binding, payload) {
                self.recorder.detached_declare(
                    name,
                    arm.binding_span.unwrap_or(arm.span),
                    ScopedDeclarationKind::Payload {
                        match_index: index,
                        variant,
                    },
                    &ty,
                    self.known,
                );
                child.insert_immutable(name, ty);
            }
            let returns = self.body(body, &mut child)?;
            self.recorder.exit_scope();
            all_return &= returns;
        }
        Ok(all_return)
    }
}
