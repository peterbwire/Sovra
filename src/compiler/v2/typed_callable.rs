//! Opt-in evaluated-expression types and links beside existing callable evidence.
use super::{CallableLimits, CheckedCallableFunction, CheckedEnumTypes, EnumFunctionError};
use crate::compiler::{
    ast::{Expression, ExpressionKind},
    diagnostics::Span,
    semantic::Type,
};
use std::{collections::HashMap, sync::atomic::AtomicU64};
/// Independent retained expression admission; cumulative type copies are charged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionEvidenceLimits {
    /// Core value-expression callbacks; call heads, direct writes and match scrutinees
    /// are excluded and await separate declaration/use integration.
    pub max_expressions: usize,
    /// Scalar/named leaves and array wrappers in retained expression types.
    pub max_type_nodes: usize,
    /// Nominal-name UTF-8 bytes in retained expression types, charged per copy.
    pub max_type_text_bytes: usize,
}
impl ExpressionEvidenceLimits {
    /// Opt-in finite limits, independent of call/context quotas.
    pub const DEFAULT: Self = Self {
        max_expressions: 250_000,
        max_type_nodes: 1_000_000,
        max_type_text_bytes: 16 * 1024 * 1024,
    };
}
/// Admission for typed callable checking; previous callable APIs remain unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedCallableLimits {
    /// Signature/call/context and source limits; array-element contexts share these counters.
    pub callable: CallableLimits,
    /// Additional evaluated-expression/type-copy limits.
    pub expressions: ExpressionEvidenceLimits,
}
impl TypedCallableLimits {
    /// Opt-in defaults.
    pub const DEFAULT: Self = Self {
        callable: CallableLimits::DEFAULT,
        expressions: ExpressionEvidenceLimits::DEFAULT,
    };
}
/// Identity scoped to one immutable typed checked function; never a byte-offset key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypedExpressionId {
    unit: u64,
    ordinal: usize,
}
impl TypedExpressionId {
    /// Position in this unit's expression facts, in semantic completion order.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Core type for one evaluated value expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedExpressionFact {
    /// This typed unit's checked expression identity.
    pub id: TypedExpressionId,
    /// Original expression range.
    pub span: Span,
    /// Canonical core type; existing empty-array holes remain explicit Unknown leaves.
    pub ty: Type,
}
/// Link from one existing callable context to its evaluated value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedContextLink {
    /// Index in the wrapped callable evidence's contexts table.
    pub context: usize,
    /// Checked value identity in this typed unit.
    pub value: TypedExpressionId,
}
/// Link from one resolved call to the type of its result expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedCallLink {
    /// Index in the wrapped callable evidence's call table.
    pub call: usize,
    /// Checked call-result expression, not its untyped function-name head.
    pub result: TypedExpressionId,
}
/// Immutable evaluated-expression facts; scoped declaration/value-use facts remain open.
#[derive(Debug, Clone)]
pub struct TypedCallableEvidence {
    unit: u64,
    expressions: Vec<TypedExpressionFact>,
    contexts: Vec<TypedContextLink>,
    calls: Vec<TypedCallLink>,
}
impl TypedCallableEvidence {
    /// Facts in semantic completion order; calls complete after their arguments.
    pub fn expressions(&self) -> &[TypedExpressionFact] {
        &self.expressions
    }
    /// Refuse identities from other checked units, including identical source text.
    pub fn expression(&self, id: TypedExpressionId) -> Option<&TypedExpressionFact> {
        if id.unit == self.unit {
            self.expressions.get(id.ordinal)
        } else {
            None
        }
    }
    /// One value link per retained callable context, including array elements.
    pub fn contexts(&self) -> &[TypedContextLink] {
        &self.contexts
    }
    /// One result link per resolved call.
    pub fn calls(&self) -> &[TypedCallLink] {
        &self.calls
    }
}
/// Source checks plus call/context facts and evaluated-expression identities/types.
#[derive(Debug, Clone)]
pub struct CheckedTypedCallableFunction {
    pub(super) callable: CheckedCallableFunction,
    pub(super) types: TypedCallableEvidence,
}
impl CheckedTypedCallableFunction {
    /// Existing constructor/self-call and source-function evidence.
    pub fn callable(&self) -> &CheckedCallableFunction {
        &self.callable
    }
    /// Additional unit-scoped evaluated-expression facts and links.
    pub fn types(&self) -> &TypedCallableEvidence {
        &self.types
    }
}
#[derive(Debug)]
pub(super) struct TypedRecorder {
    limits: ExpressionEvidenceLimits,
    evidence: TypedCallableEvidence,
    pointers: HashMap<usize, TypedExpressionId>,
    heads: HashMap<usize, usize>,
    nodes: usize,
    text: usize,
}
impl TypedRecorder {
    pub(super) fn new(limits: ExpressionEvidenceLimits) -> Result<Self, EnumFunctionError> {
        static NEXT_UNIT: AtomicU64 = AtomicU64::new(0);
        let unit = super::enum_types::allocate_identity(&NEXT_UNIT)
            .map_err(EnumFunctionError::Coverage)?;
        Ok(Self {
            limits,
            evidence: TypedCallableEvidence {
                unit,
                expressions: Vec::new(),
                contexts: Vec::new(),
                calls: Vec::new(),
            },
            pointers: HashMap::new(),
            heads: HashMap::new(),
            nodes: 0,
            text: 0,
        })
    }
    pub(super) fn unit(&self) -> u64 {
        self.evidence.unit
    }
    pub(super) fn expression_id(&self, value: &Expression) -> Option<TypedExpressionId> {
        self.pointers
            .get(&(value as *const Expression as usize))
            .copied()
    }
    pub(super) fn head(&mut self, head: &Expression, call: usize) -> bool {
        self.heads
            .insert(head as *const Expression as usize, call)
            .is_none()
    }
    pub(super) fn ty(&self, value: &Expression) -> Option<&Type> {
        let id = self.pointers.get(&(value as *const Expression as usize))?;
        Some(&self.evidence.expressions[id.ordinal].ty)
    }
    pub(super) fn context(&mut self, value: &Expression, context: usize) -> bool {
        let Some(id) = self
            .pointers
            .get(&(value as *const Expression as usize))
            .copied()
        else {
            return false;
        };
        self.evidence
            .contexts
            .push(TypedContextLink { context, value: id });
        true
    }
    pub(super) fn expression(
        &mut self,
        value: &Expression,
        ty: &Type,
        names: &HashMap<String, Type>,
    ) -> bool {
        let pointer = value as *const Expression as usize;
        let ty = ty.canonicalize(names);
        if let Some(id) = self.pointers.get(&pointer) {
            return self.evidence.expressions[id.ordinal].ty == ty;
        }
        if self.evidence.expressions.len() >= self.limits.max_expressions {
            return false;
        }
        let mut leaf = &ty;
        loop {
            let Some(nodes) = self
                .nodes
                .checked_add(1)
                .filter(|n| *n <= self.limits.max_type_nodes)
            else {
                return false;
            };
            self.nodes = nodes;
            match leaf {
                Type::Array(item) => leaf = item,
                Type::Named(name) => {
                    let Some(text) = self
                        .text
                        .checked_add(name.len())
                        .filter(|n| *n <= self.limits.max_type_text_bytes)
                    else {
                        return false;
                    };
                    self.text = text;
                    break;
                }
                _ => break,
            }
        }
        let id = TypedExpressionId {
            unit: self.evidence.unit,
            ordinal: self.evidence.expressions.len(),
        };
        self.evidence.expressions.push(TypedExpressionFact {
            id,
            span: value.span,
            ty,
        });
        self.pointers.insert(pointer, id);
        if let ExpressionKind::Call { callee, .. } = &value.kind {
            let Some(call) = self
                .heads
                .get(&(callee.as_ref() as *const Expression as usize))
                .copied()
            else {
                return false;
            };
            self.evidence.calls.push(TypedCallLink { call, result: id });
        }
        true
    }
    pub(super) fn finish(self) -> TypedCallableEvidence {
        self.evidence
    }
}
impl CheckedEnumTypes {
    /// Opt into evaluated-expression types, result/context identities and array-element contexts.
    /// This is still not complete lowering evidence: scoped declarations/uses remain open.
    pub fn check_typed_callable_function_source(
        &self,
        label: &str,
        source: &str,
        limits: TypedCallableLimits,
    ) -> Result<CheckedTypedCallableFunction, EnumFunctionError> {
        let result = self.check_function_source_mode(
            label,
            source,
            limits.callable.function,
            Some(super::function::FunctionEvidenceMode {
                callable: limits.callable,
                expressions: Some(limits.expressions),
                bindings: None,
            }),
        )?;
        Ok(CheckedTypedCallableFunction {
            callable: CheckedCallableFunction {
                function: result.function,
                evidence: result.evidence.expect("typed callable mode records calls"),
            },
            types: result.types.expect("typed callable mode records types"),
        })
    }
}
