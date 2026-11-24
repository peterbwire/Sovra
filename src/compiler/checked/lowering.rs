//! Contextual types needed to preserve existing IR, including identity widening.

use super::{CoercionKind, NodeId, TypeId};

/// Explicit allowances for retained lowering contexts, not process memory limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoweringEvidenceLimits {
    /// Checked function-result records, including Unit functions.
    pub max_function_results: usize,
    /// Contextual value edges, including identity and unresolved-hole contexts.
    pub max_contexts: usize,
}

/// Checked declared/default result type of a source function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionResult {
    /// Structural function identity in the same snapshot.
    pub function: NodeId,
    /// Canonical result type, including implicit Unit.
    pub ty: TypeId,
}

/// Expected type at an already checked value edge. Unlike Coercion, this also
/// retains identity contexts, needed for exact legacy IR compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeContext {
    /// Statement or containing expression establishing the context.
    pub owner: NodeId,
    /// Source value expression.
    pub value: NodeId,
    /// Canonical expected type.
    pub expected: TypeId,
    /// Existing initialization/assignment/call/return/field/array/binary context.
    pub kind: CoercionKind,
}

/// Immutable checked lowering metadata; only explicit lowering constructors collect it.
#[derive(Debug, Default)]
pub struct LoweringEvidence {
    pub(super) results: Vec<FunctionResult>,
    pub(super) contexts: Vec<TypeContext>,
}

impl LoweringEvidence {
    /// Canonical declared/default function result types.
    pub fn function_results(&self) -> &[FunctionResult] {
        &self.results
    }
    /// Contextual expected types; intrinsic expression facts remain separate.
    pub fn contexts(&self) -> &[TypeContext] {
        &self.contexts
    }
}

pub(super) struct LoweringRecorder {
    pub limits: LoweringEvidenceLimits,
    pub evidence: LoweringEvidence,
}
