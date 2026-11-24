//! Bounded annotation substitution for an already-bound generic callee.
use super::{
    CheckedGenericCallBinding, ExpandedGenericType, GenericAliasExpansionError,
    GenericAliasExpansionLimits, GenericSignatureReturn,
};
use std::sync::Arc;
/// Cumulative and per-annotation graph limits for one callee signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSignatureInstantiationLimits {
    /// Explicit parameter/return annotations to instantiate.
    pub max_annotations: usize,
    /// Cumulative generated nodes and their provenance slots.
    pub max_nodes: usize,
    /// Cumulative generated child edges.
    pub max_edges: usize,
    /// Cumulative copy/mapping/structural work.
    pub max_work: usize,
    /// Independent per-annotation generated node limit.
    pub max_nodes_per_type: usize,
    /// Independent per-annotation generated edge limit.
    pub max_edges_per_type: usize,
    /// Independent per-annotation substitution work limit.
    pub max_work_per_type: usize,
    /// Generated structural depth, at most 128.
    pub max_depth: usize,
}
impl GenericSignatureInstantiationLimits {
    /// Finite symbolic signature profile; no code specialization.
    pub const DEFAULT: Self = Self {
        max_annotations: 1024,
        max_nodes: 65_536,
        max_edges: 131_072,
        max_work: 1_000_000,
        max_nodes_per_type: 16_384,
        max_edges_per_type: 32_768,
        max_work_per_type: 262_144,
        max_depth: 128,
    };
}
/// Original callee annotation position, not generated source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericInstantiationPosition {
    /// Source-order value parameter annotation.
    Parameter(usize),
    /// Explicit return annotation.
    Return,
}
/// Complete refusal without partial instantiated signature publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericSignatureInstantiationError {
    /// Cumulative annotation/node/edge/work admission exhausted.
    Limit(&'static str),
    /// A bounded substitution failed at an original annotation position.
    Annotation {
        /// Original callee annotation role.
        position: GenericInstantiationPosition,
        /// Exact graph/owner/depth refusal.
        error: Box<GenericAliasExpansionError>,
    },
}
impl std::fmt::Display for GenericSignatureInstantiationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic signature instantiation refused: {self:?}")
    }
}
impl std::error::Error for GenericSignatureInstantiationError {}
/// Sealed caller-scoped callee annotation types; not a checked value/body call.
#[derive(Debug, Clone)]
pub struct InstantiatedGenericSignature {
    binding: CheckedGenericCallBinding,
    parameters: Arc<[ExpandedGenericType]>,
    result: GenericSignatureReturn,
    annotations: usize,
    nodes: usize,
    edges: usize,
    work: usize,
}
impl InstantiatedGenericSignature {
    /// Original sealed caller/callee argument and bound matching.
    pub fn binding(&self) -> &CheckedGenericCallBinding {
        &self.binding
    }
    /// Substituted source-order value parameter types.
    pub fn parameter_types(&self) -> &[ExpandedGenericType] {
        &self.parameters
    }
    /// Substituted explicit result or unchanged implicit Unit convention.
    pub fn return_type(&self) -> &GenericSignatureReturn {
        &self.result
    }
    /// Instantiated explicit annotation count.
    pub fn annotations(&self) -> usize {
        self.annotations
    }
    /// Generated nodes, including retained unused eager arguments.
    pub fn nodes(&self) -> usize {
        self.nodes
    }
    /// Generated child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Consumed substitution work; prior matching/resolution is separate.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    amount: usize,
    dimension: &'static str,
) -> Result<(), GenericSignatureInstantiationError> {
    *left = left
        .checked_sub(amount)
        .ok_or(GenericSignatureInstantiationError::Limit(dimension))?;
    Ok(())
}
/// Substitute every original callee annotation after sealed type/bound matching.
/// Does not recheck a body, validate supplied value expressions or emit code.
pub fn instantiate_generic_signature(
    binding: &CheckedGenericCallBinding,
    limits: GenericSignatureInstantiationLimits,
) -> Result<InstantiatedGenericSignature, GenericSignatureInstantiationError> {
    let callee = binding
        .signatures()
        .signature(binding.callee())
        .expect("sealed callee owner");
    let count = callee.parameter_types().len()
        + usize::from(matches!(
            callee.return_type(),
            GenericSignatureReturn::Explicit(_)
        ));
    if count > limits.max_annotations {
        return Err(GenericSignatureInstantiationError::Limit("annotations"));
    }
    // Reject invalid profiles even when there are no explicit annotations.
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(GenericSignatureInstantiationError::Limit(
            "structural depth profile",
        ));
    }
    let mut nodes = limits.max_nodes;
    let mut edges = limits.max_edges;
    let mut work = limits.max_work;
    charge(&mut work, 1, "instantiation work")?;
    let mut instantiate = |ty: &ExpandedGenericType, position| {
        charge(&mut work, 1, "instantiation work")?;
        let expansion = GenericAliasExpansionLimits {
            max_nodes: nodes.min(limits.max_nodes_per_type),
            max_edges: edges.min(limits.max_edges_per_type),
            max_work: work.min(limits.max_work_per_type),
            max_depth: limits.max_depth,
            ..GenericAliasExpansionLimits::DEFAULT
        };
        let result = ty
            .substitute_arguments(
                callee.syntax().parameters(),
                binding.arguments().arguments(),
                binding.arguments().parameters(),
                expansion,
            )
            .map_err(|error| GenericSignatureInstantiationError::Annotation {
                position,
                error: Box::new(error),
            })?;
        charge(&mut nodes, result.nodes().len(), "generated nodes")?;
        charge(&mut edges, result.edges(), "generated edges")?;
        charge(&mut work, result.work(), "instantiation work")?;
        Ok(result)
    };
    let mut parameters = Vec::new();
    for (index, ty) in callee.parameter_types().iter().enumerate() {
        parameters.push(instantiate(
            ty,
            GenericInstantiationPosition::Parameter(index),
        )?);
    }
    let result = match callee.return_type() {
        GenericSignatureReturn::ImplicitUnit => GenericSignatureReturn::ImplicitUnit,
        GenericSignatureReturn::Explicit(ty) => {
            GenericSignatureReturn::Explicit(instantiate(ty, GenericInstantiationPosition::Return)?)
        }
    };
    Ok(InstantiatedGenericSignature {
        binding: binding.clone(),
        parameters: parameters.into(),
        result,
        annotations: count,
        nodes: limits.max_nodes - nodes,
        edges: limits.max_edges - edges,
        work: limits.max_work - work,
    })
}
