//! Caller-scope type argument resolution, not callee binding or execution.
use super::{
    CheckedGenericDeclarations, ExpandedGenericType, GenericAliasExpansionError,
    GenericAliasExpansionLimits, GenericParameters, GenericTypeArgumentsSyntax,
};
use crate::compiler::diagnostics::Span;
use std::sync::Arc;
/// Cumulative input/copy/generated admission for one complete explicit list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericArgumentCheckLimits {
    /// Explicit argument records.
    pub max_arguments: usize,
    /// Complete original list bytes, counted once.
    pub max_source_bytes: usize,
    /// Original lexer tokens including EOF, counted once.
    pub max_tokens: usize,
    /// Copied argument syntax nodes.
    pub max_input_nodes: usize,
    /// Input child edges plus list-to-root edges.
    pub max_input_edges: usize,
    /// Copied qualified-name bytes across argument syntax.
    pub max_name_bytes: usize,
    /// Cumulative generated expanded nodes, including unused eager arguments.
    pub max_nodes: usize,
    /// Cumulative generated expanded child edges.
    pub max_edges: usize,
    /// Copy admission plus resolution/alias-expansion work across the list.
    pub max_work: usize,
    /// Independent per-argument resolution/expansion/depth profile.
    pub expansion: GenericAliasExpansionLimits,
}
impl GenericArgumentCheckLimits {
    /// Finite caller-scope profile; no inference or call specialization.
    pub const DEFAULT: Self = Self {
        max_arguments: 128,
        max_source_bytes: 64 * 1024,
        max_tokens: 16_384,
        max_input_nodes: 4096,
        max_input_edges: 4096,
        max_name_bytes: 32 * 1024,
        max_nodes: 16_384,
        max_edges: 32_768,
        max_work: 262_144,
        expansion: GenericAliasExpansionLimits::DEFAULT,
    };
}
/// Complete refusal without partial checked arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArgumentCheckError {
    /// A cumulative list/input/generated budget was exhausted.
    Limit(&'static str),
    /// An original argument failed name, arity, alias, bound or expansion admission.
    Argument {
        /// Source-order argument index.
        ordinal: usize,
        /// Original list-source argument range.
        span: Span,
        /// Inner input/alias provenance is retained unchanged.
        error: Box<GenericAliasExpansionError>,
    },
}
impl std::fmt::Display for GenericArgumentCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "explicit type arguments refused: {self:?}")
    }
}
impl std::error::Error for GenericArgumentCheckError {}
/// Sealed caller-scope arguments, not a checked callee, call or specialization.
#[derive(Debug, Clone)]
pub struct CheckedGenericTypeArguments {
    types: CheckedGenericDeclarations,
    syntax: GenericTypeArgumentsSyntax,
    parameters: GenericParameters,
    arguments: Arc<[ExpandedGenericType]>,
    nodes: usize,
    edges: usize,
    work: usize,
}
impl CheckedGenericTypeArguments {
    pub(super) fn absent(
        types: &CheckedGenericDeclarations,
        parameters: &GenericParameters,
        max_work: usize,
    ) -> Result<Self, GenericArgumentCheckError> {
        if max_work == 0 {
            return Err(GenericArgumentCheckError::Limit("empty argument work"));
        }
        Ok(Self {
            types: types.clone(),
            syntax: GenericTypeArgumentsSyntax::absent(),
            parameters: parameters.clone(),
            arguments: Arc::from([]),
            nodes: 0,
            edges: 0,
            work: 1,
        })
    }

    /// Complete checked type environment retained by these arguments.
    pub fn types(&self) -> &CheckedGenericDeclarations {
        &self.types
    }
    /// Original admitted explicit argument list, or sealed absence for a nongeneric body call.
    pub fn syntax(&self) -> &GenericTypeArgumentsSyntax {
        &self.syntax
    }
    /// Original caller's opaque parameter owner.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    /// All resolved/alias-expanded arguments in source order.
    pub fn arguments(&self) -> &[ExpandedGenericType] {
        &self.arguments
    }
    /// Generated nodes, including unused eager argument nodes.
    pub fn nodes(&self) -> usize {
        self.nodes
    }
    /// Generated child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Consumed copy/resolution/expansion work.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    amount: usize,
    dimension: &'static str,
) -> Result<(), GenericArgumentCheckError> {
    *left = left
        .checked_sub(amount)
        .ok_or(GenericArgumentCheckError::Limit(dimension))?;
    Ok(())
}
/// Resolve every explicit argument in one original caller scope. Names, type-head
/// arity, alias cycles/preparation and declared Key constraints must already be
/// supported by the complete checked environment. No callee arity or callee bounds
/// are proved by this API; those are separate call-binding obligations.
pub fn check_generic_type_arguments(
    types: &CheckedGenericDeclarations,
    syntax: &GenericTypeArgumentsSyntax,
    parameters: &GenericParameters,
    limits: GenericArgumentCheckLimits,
) -> Result<CheckedGenericTypeArguments, GenericArgumentCheckError> {
    for (used, max, dimension) in [
        (syntax.roots().len(), limits.max_arguments, "arguments"),
        (
            syntax.source().len(),
            limits.max_source_bytes,
            "source bytes",
        ),
        (syntax.token_count(), limits.max_tokens, "tokens"),
        (syntax.nodes().len(), limits.max_input_nodes, "input nodes"),
        (syntax.edges(), limits.max_input_edges, "input edges"),
        (
            syntax.name_bytes(),
            limits.max_name_bytes,
            "input name bytes",
        ),
    ] {
        if used > max {
            return Err(GenericArgumentCheckError::Limit(dimension));
        }
    }
    let mut work = limits.max_work;
    let mut nodes = limits.max_nodes;
    let mut edges = limits.max_edges;
    // Admit cloned node/name/child/root storage before copying argument syntax.
    charge(&mut work, syntax.nodes().len(), "copy work")?;
    charge(&mut work, syntax.edges(), "copy work")?;
    charge(&mut work, syntax.name_bytes(), "copy work")?;
    let mut arguments = Vec::new();
    for (ordinal, &root) in syntax.roots().iter().enumerate() {
        charge(&mut work, 1, "argument work")?;
        let input = syntax
            .argument_syntax(ordinal)
            .expect("admitted argument root");
        let mut expansion = limits.expansion;
        expansion.max_nodes = expansion.max_nodes.min(nodes);
        expansion.max_edges = expansion.max_edges.min(edges);
        expansion.max_work = expansion.max_work.min(work);
        let ty = types
            .environment()
            .expand(&input, parameters, expansion)
            .map_err(|error| GenericArgumentCheckError::Argument {
                ordinal,
                span: syntax.nodes()[root].span,
                error: Box::new(error),
            })?;
        charge(&mut nodes, ty.nodes().len(), "expanded nodes")?;
        charge(&mut edges, ty.edges(), "expanded edges")?;
        charge(&mut work, ty.work(), "argument work")?;
        arguments.push(ty);
    }
    Ok(CheckedGenericTypeArguments {
        types: types.clone(),
        syntax: syntax.clone(),
        parameters: parameters.clone(),
        arguments: arguments.into(),
        nodes: limits.max_nodes - nodes,
        edges: limits.max_edges - edges,
        work: limits.max_work - work,
    })
}
