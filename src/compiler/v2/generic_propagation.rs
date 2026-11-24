//! Sealed symbolic propagation compatibility, before postfix syntax or control-flow lowering.
use super::*;
use crate::compiler::diagnostics::Span;
use std::{collections::BTreeSet, sync::Arc};
/// Whole-request admission; comparisons share these allowances across all sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPropagationLimits {
    /// Selected expression sites, including duplicate-refusal inputs.
    pub max_sites: usize,
    /// Retained projected type contexts (two for Option, three for Result).
    pub max_contexts: usize,
    /// Cumulative exact error-type comparison pairs.
    pub max_pairs: usize,
    /// Cumulative exact error-type comparison edges.
    pub max_edges: usize,
    /// Site/context checks and exact comparison work.
    pub max_work: usize,
}
impl GenericPropagationLimits {
    /// Finite library profile; no implicit source/CLI selection.
    pub const DEFAULT: Self = Self {
        max_sites: 4096,
        max_contexts: 12288,
        max_pairs: 65536,
        max_edges: 131072,
        max_work: 1_000_000,
    };
}
/// A selected operand in an existing sealed symbolic body, not a parsed question mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPropagationRequest {
    /// Signature-table-owned enclosing function.
    pub function: GenericSignatureId,
    /// Exact body-arena-owned operand expression.
    pub operand: GenericCopyExpressionId,
}
/// Complete refusal; no partial plans or implicit conversions are returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericPropagationError {
    /// A whole-batch logical allowance exhausted.
    Limit(&'static str),
    /// Foreign function, body arena, expression or environment.
    Ownership,
    /// Repeated source operand site.
    Duplicate,
    /// Operand is not a compiler-owned standard Option/Result.
    Operand {
        /// Enclosing checked function whose original fragment owns the range.
        function: GenericSignatureId,
        /// Original operand span within that function source.
        span: Span,
    },
    /// Enclosing function does not explicitly return the same standard wrapper.
    Return {
        /// Enclosing checked function whose original fragment owns the range.
        function: GenericSignatureId,
        /// Original operand span within that function source.
        span: Span,
    },
    /// Result failure payload types are not exactly equal.
    ErrorType {
        /// Enclosing checked function whose original fragment owns the range.
        function: GenericSignatureId,
        /// Original operand span within that function source.
        span: Span,
    },
    /// Bounded exact type comparison refused.
    Comparison(GenericTypeEquivalenceError),
    /// Compiler-owned variant shape is unavailable/inconsistent.
    Shape,
}
impl std::fmt::Display for GenericPropagationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic propagation planning refused: {self:?}")
    }
}
impl std::error::Error for GenericPropagationError {}
/// Immutable compatibility proof; does not authorize execution or host services.
#[derive(Debug, Clone)]
pub struct GenericPropagationPlan {
    body: CheckedGenericCopyBody,
    operand: GenericCopyExpressionId,
    kind: GenericStandardEnum,
    declaration: GenericTypeDeclarationId,
    success_variant: usize,
    failure_variant: usize,
    success: ExpandedGenericType,
    failure: Option<ExpandedGenericType>,
    result: ExpandedGenericType,
}
impl GenericPropagationPlan {
    /// Original immutable symbolic function body and source provenance.
    pub fn body(&self) -> &CheckedGenericCopyBody {
        &self.body
    }
    /// Exact source-owned operand; a future lowering must evaluate it once.
    pub fn operand(&self) -> GenericCopyExpressionId {
        self.operand
    }
    /// Compiler-owned wrapper family.
    pub fn kind(&self) -> GenericStandardEnum {
        self.kind
    }
    /// Nominal standard identity shared by operand and enclosing return type.
    pub fn declaration(&self) -> GenericTypeDeclarationId {
        self.declaration
    }
    /// Original standard declaration's success tag.
    pub fn success_variant(&self) -> usize {
        self.success_variant
    }
    /// Original standard declaration's failure tag.
    pub fn failure_variant(&self) -> usize {
        self.failure_variant
    }
    /// Successful extraction type; no relationship to the caller's success type is required.
    pub fn success_type(&self) -> &ExpandedGenericType {
        &self.success
    }
    /// Failure payload to snapshot, or None for nullary Option failure.
    pub fn failure_type(&self) -> Option<&ExpandedGenericType> {
        self.failure.as_ref()
    }
    /// Enclosing function's exact return wrapper; failure must construct this type.
    pub fn return_type(&self) -> &ExpandedGenericType {
        &self.result
    }
}
/// Measured whole-batch logical admission; projections retain immutable arenas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPropagationUsage {
    /// Accepted sites.
    pub sites: usize,
    /// Retained type projections.
    pub contexts: usize,
    /// Exact comparison pairs.
    pub pairs: usize,
    /// Exact comparison edges.
    pub edges: usize,
    /// Consumed logical work.
    pub work: usize,
}
/// Complete ordered batch; input body/source contexts remain owned by each plan.
#[derive(Debug, Clone)]
pub struct CheckedGenericPropagations {
    plans: Arc<[GenericPropagationPlan]>,
    usage: GenericPropagationUsage,
}
impl CheckedGenericPropagations {
    /// Plans in original request order.
    pub fn plans(&self) -> &[GenericPropagationPlan] {
        &self.plans
    }
    /// Complete batch costs.
    pub fn usage(&self) -> GenericPropagationUsage {
        self.usage
    }
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<(), GenericPropagationError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericPropagationError::Limit(name))?;
    Ok(())
}
fn standard<'a>(
    types: &'a CheckedGenericDeclarations,
    ty: &ExpandedGenericType,
) -> Option<(
    GenericTypeDeclarationId,
    GenericStandardEnum,
    &'a CheckedGenericEnum,
)> {
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &ty.nodes()[ty.root()].kind
    else {
        return None;
    };
    let Some(CheckedGenericDefinition::Enum(definition)) = types.definition(*declaration) else {
        return None;
    };
    let kind = definition.syntax().standard_identity()?;
    if arguments.len()
        != match kind {
            GenericStandardEnum::Option => 1,
            GenericStandardEnum::Result => 2,
            GenericStandardEnum::JsonValue => return None,
        }
    {
        return None;
    }
    Some((*declaration, kind, definition))
}
/// Check exact failure compatibility for selected existing operands. Success
/// payloads may differ from the enclosing function's success payload. This does
/// not parse `?`, insert returns, prove concrete substitutions or emit instructions.
pub fn plan_generic_propagations(
    bodies: &CheckedGenericCopyBodies,
    requests: &[GenericPropagationRequest],
    limits: GenericPropagationLimits,
) -> Result<CheckedGenericPropagations, GenericPropagationError> {
    if requests.len() > limits.max_sites {
        return Err(GenericPropagationError::Limit("propagation sites"));
    }
    let mut left = limits;
    let mut seen = BTreeSet::new();
    let mut plans = Vec::new();
    for request in requests {
        charge(&mut left.max_work, 1, "propagation work")?;
        let body = bodies
            .body(request.function)
            .ok_or(GenericPropagationError::Ownership)?;
        let operand = body
            .expression(request.operand)
            .ok_or(GenericPropagationError::Ownership)?;
        if !seen.insert((request.function.ordinal(), request.operand.ordinal())) {
            return Err(GenericPropagationError::Duplicate);
        }
        let comparison = GenericTypeEquivalenceLimits {
            max_pairs: left.max_pairs,
            max_edges: left.max_edges,
            max_work: left.max_work,
        };
        let types = check_propagation_type(
            body.signature(),
            &operand.ty,
            operand.span,
            &mut left,
            comparison,
        )?;
        let GenericPropagationTypes {
            kind,
            declaration,
            success_variant,
            failure_variant,
            success,
            failure,
            result,
        } = types;
        plans.push(GenericPropagationPlan {
            body: body.clone(),
            operand: request.operand,
            kind,
            declaration,
            success_variant,
            failure_variant,
            success,
            failure,
            result,
        });
    }
    Ok(CheckedGenericPropagations {
        usage: GenericPropagationUsage {
            sites: plans.len(),
            contexts: limits.max_contexts - left.max_contexts,
            pairs: limits.max_pairs - left.max_pairs,
            edges: limits.max_edges - left.max_edges,
            work: limits.max_work - left.max_work,
        },
        plans: plans.into(),
    })
}

/// Body-independent sealed symbolic propagation types; not concrete/runtime proof.
#[derive(Debug, Clone)]
pub struct GenericPropagationTypes {
    kind: GenericStandardEnum,
    declaration: GenericTypeDeclarationId,
    success_variant: usize,
    failure_variant: usize,
    success: ExpandedGenericType,
    failure: Option<ExpandedGenericType>,
    result: ExpandedGenericType,
}
impl GenericPropagationTypes {
    /// Compiler-owned wrapper family.
    pub fn kind(&self) -> GenericStandardEnum {
        self.kind
    }
    /// Exact standard nominal declaration.
    pub fn declaration(&self) -> GenericTypeDeclarationId {
        self.declaration
    }
    /// Successful source variant index.
    pub fn success_variant(&self) -> usize {
        self.success_variant
    }
    /// Early-return source variant index.
    pub fn failure_variant(&self) -> usize {
        self.failure_variant
    }
    /// Successful extraction type in caller scope.
    pub fn success_type(&self) -> &ExpandedGenericType {
        &self.success
    }
    /// Exact error payload, absent for Option failure.
    pub fn failure_type(&self) -> Option<&ExpandedGenericType> {
        self.failure.as_ref()
    }
    /// Caller return wrapper to construct on failure.
    pub fn return_type(&self) -> &ExpandedGenericType {
        &self.result
    }
}
pub(super) fn check_propagation_type(
    signature: &CheckedGenericSignature,
    operand: &GenericCopyType,
    span: Span,
    left: &mut GenericPropagationLimits,
    comparison: GenericTypeEquivalenceLimits,
) -> Result<GenericPropagationTypes, GenericPropagationError> {
    let GenericSignatureReturn::Explicit(result) = signature.return_type() else {
        return Err(GenericPropagationError::Return {
            function: signature.id(),
            span,
        });
    };
    check_propagation_context(
        signature.types(),
        signature.id(),
        operand,
        result,
        span,
        left,
        comparison,
    )
}
pub(super) fn check_propagation_context(
    types: &CheckedGenericDeclarations,
    function: GenericSignatureId,
    operand: &GenericCopyType,
    result: &ExpandedGenericType,
    span: Span,
    left: &mut GenericPropagationLimits,
    comparison: GenericTypeEquivalenceLimits,
) -> Result<GenericPropagationTypes, GenericPropagationError> {
    charge(&mut left.max_sites, 1, "propagation sites")?;
    charge(&mut left.max_work, 1, "propagation work")?;
    let source = operand
        .expanded()
        .ok_or(GenericPropagationError::Operand { function, span })?;
    if !source.environment().same_environment(types.environment())
        || !result.environment().same_environment(types.environment())
    {
        return Err(GenericPropagationError::Ownership);
    }
    charge(&mut left.max_work, 2, "propagation work")?;
    let (declaration, kind, definition) =
        standard(types, source).ok_or(GenericPropagationError::Operand { function, span })?;
    let (target, target_kind, _) =
        standard(types, result).ok_or(GenericPropagationError::Return { function, span })?;
    if declaration != target || kind != target_kind {
        return Err(GenericPropagationError::Return { function, span });
    }
    let contexts = if kind == GenericStandardEnum::Option {
        2
    } else {
        3
    };
    charge(&mut left.max_contexts, contexts, "propagation contexts")?;
    charge(&mut left.max_work, contexts, "propagation work")?;
    let success = source
        .declaration_argument(declaration, 0)
        .ok_or(GenericPropagationError::Shape)?;
    let failure = if kind == GenericStandardEnum::Result {
        charge(&mut left.max_work, 1, "propagation work")?;
        let input = source
            .declaration_argument(declaration, 1)
            .ok_or(GenericPropagationError::Shape)?;
        let output = result
            .declaration_argument(declaration, 1)
            .ok_or(GenericPropagationError::Shape)?;
        let comparison = compare_generic_types(
            &input,
            &output,
            GenericTypeEquivalenceLimits {
                max_pairs: left.max_pairs.min(comparison.max_pairs),
                max_edges: left.max_edges.min(comparison.max_edges),
                max_work: left.max_work.min(comparison.max_work),
            },
        )
        .map_err(GenericPropagationError::Comparison)?;
        charge(&mut left.max_pairs, comparison.pairs(), "propagation pairs")?;
        charge(&mut left.max_edges, comparison.edges(), "propagation edges")?;
        charge(&mut left.max_work, comparison.work(), "propagation work")?;
        if !comparison.equivalent() {
            return Err(GenericPropagationError::ErrorType { function, span });
        }
        Some(input)
    } else {
        None
    };
    let (success_name, failure_name) = match kind {
        GenericStandardEnum::Option => ("Some", "None"),
        GenericStandardEnum::Result => ("Ok", "Err"),
        GenericStandardEnum::JsonValue => return Err(GenericPropagationError::Shape),
    };
    let mut success_variant = None;
    let mut failure_variant = None;
    if definition.syntax().variants().len() != 2 || definition.payload_types().len() != 2 {
        return Err(GenericPropagationError::Shape);
    }
    for (index, variant) in definition.syntax().variants().iter().enumerate() {
        charge(&mut left.max_work, 1, "propagation work")?;
        if variant.name == success_name && definition.payload_types()[index].is_some() {
            success_variant = Some(index);
        }
        if variant.name == failure_name
            && definition.payload_types()[index].is_some() == failure.is_some()
        {
            failure_variant = Some(index);
        }
    }
    let success_variant = success_variant.ok_or(GenericPropagationError::Shape)?;
    let failure_variant = failure_variant.ok_or(GenericPropagationError::Shape)?;
    if success_variant == failure_variant {
        return Err(GenericPropagationError::Shape);
    }
    Ok(GenericPropagationTypes {
        kind,
        declaration,
        success_variant,
        failure_variant,
        success,
        failure,
        result: result.clone(),
    })
}
