//! Value type matching for a sealed instantiated callee, not source-call execution.
use super::{
    compare_generic_types, CheckedGenericCopyBody, GenericCopyConversion, GenericCopyExpressionId,
    GenericPrimitive, GenericSignatureReturn, GenericTypeEquivalenceError,
    GenericTypeEquivalenceLimits, InstantiatedGenericSignature,
};
use crate::compiler::diagnostics::Span;
use std::sync::Arc;
/// Independent and cumulative matching admission for one supplied argument list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCallValueLimits {
    /// Source-order supplied expression IDs retained on success.
    pub max_arguments: usize,
    /// Cumulative exact-comparison pairs, including scalar comparisons.
    pub max_pairs: usize,
    /// Cumulative queued comparison edges.
    pub max_edges: usize,
    /// Owner/argument visits and comparison work.
    pub max_work: usize,
    /// Independent per-comparison profile, reduced by remaining cumulative budgets.
    pub comparison: GenericTypeEquivalenceLimits,
}
impl GenericCallValueLimits {
    /// Finite type-matching profile; does not enable calls or inference.
    pub const DEFAULT: Self = Self {
        max_arguments: 1024,
        max_pairs: 65_536,
        max_edges: 131_072,
        max_work: 262_144,
        comparison: GenericTypeEquivalenceLimits::DEFAULT,
    };
}
/// Complete value-list refusal without partial matched arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericCallValueError {
    /// Matching admission exhausted.
    Limit(&'static str),
    /// Supplied body does not own the original bound caller signature.
    Caller,
    /// Value count differs from the substituted callee signature.
    Arity {
        /// Expected value count.
        expected: usize,
        /// Supplied value count.
        actual: usize,
    },
    /// Expression ID does not belong to the supplied caller body.
    Expression {
        /// Source-order value argument index.
        argument: usize,
    },
    /// Checked expression type cannot match the substituted parameter type.
    Type {
        /// Source-order value argument index.
        argument: usize,
        /// Original caller expression range.
        span: Span,
    },
    /// Comparison exhausted; never interpreted as a type mismatch or equality.
    Comparison {
        /// Source-order value argument index.
        argument: usize,
        /// Original caller expression range.
        span: Span,
        /// Exact independent/cumulative comparison refusal.
        error: GenericTypeEquivalenceError,
    },
}
impl std::fmt::Display for GenericCallValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic value arguments refused: {self:?}")
    }
}
impl std::error::Error for GenericCallValueError {}
/// One original caller expression and its parameter-context conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCallValueArgument {
    /// Original checked caller expression ID.
    pub expression: GenericCopyExpressionId,
    /// Exact identity or existing concrete scalar widening.
    pub conversion: GenericCopyConversion,
}
/// Sealed type compatibility for supplied caller expressions; not a source call.
#[derive(Debug, Clone)]
pub struct CheckedGenericCallValues {
    signature: InstantiatedGenericSignature,
    body: CheckedGenericCopyBody,
    values: Arc<[GenericCallValueArgument]>,
    pairs: usize,
    edges: usize,
    work: usize,
}
impl CheckedGenericCallValues {
    /// Original sealed caller/callee and substituted annotation evidence.
    pub fn signature(&self) -> &InstantiatedGenericSignature {
        &self.signature
    }
    /// Original caller body owning every supplied expression ID.
    pub fn body(&self) -> &CheckedGenericCopyBody {
        &self.body
    }
    /// Source-order supplied arguments and explicit conversions.
    pub fn arguments(&self) -> &[GenericCallValueArgument] {
        &self.values
    }
    /// Callee result convention; matching values does not evaluate this result.
    pub fn return_type(&self) -> &GenericSignatureReturn {
        self.signature.return_type()
    }
    /// Consumed comparison pairs.
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    /// Consumed comparison child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Consumed owner/argument/comparison work.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    amount: usize,
    dimension: &'static str,
) -> Result<(), GenericCallValueError> {
    *left = left
        .checked_sub(amount)
        .ok_or(GenericCallValueError::Limit(dimension))?;
    Ok(())
}
/// Match already-checked caller expressions to substituted parameter types.
/// No parsing, literal contextualization, call-source association or evaluation
/// occurs. Future body integration must establish the actual source-call edges.
pub fn check_generic_call_values(
    signature: &InstantiatedGenericSignature,
    body: &CheckedGenericCopyBody,
    values: &[GenericCopyExpressionId],
    limits: GenericCallValueLimits,
) -> Result<CheckedGenericCallValues, GenericCallValueError> {
    let mut work = limits.max_work;
    let mut pairs = limits.max_pairs;
    let mut edges = limits.max_edges;
    charge(&mut work, 1, "value matching work")?;
    if body.signature().id() != signature.binding().caller() {
        return Err(GenericCallValueError::Caller);
    }
    if values.len() != signature.parameter_types().len() {
        return Err(GenericCallValueError::Arity {
            expected: signature.parameter_types().len(),
            actual: values.len(),
        });
    }
    if values.len() > limits.max_arguments {
        return Err(GenericCallValueError::Limit("value arguments"));
    }
    let mut matched = Vec::new();
    for (index, (id, expected)) in values.iter().zip(signature.parameter_types()).enumerate() {
        charge(&mut work, 1, "value matching work")?;
        let actual = body
            .expression(*id)
            .ok_or(GenericCallValueError::Expression { argument: index })?;
        let comparison = GenericTypeEquivalenceLimits {
            max_pairs: limits.comparison.max_pairs.min(pairs),
            max_edges: limits.comparison.max_edges.min(edges),
            max_work: limits.comparison.max_work.min(work),
        };
        let failure = |error| GenericCallValueError::Comparison {
            argument: index,
            span: actual.span,
            error,
        };
        let same = if let Some(expanded) = actual.ty.expanded() {
            let result = compare_generic_types(expanded, expected, comparison).map_err(failure)?;
            charge(&mut pairs, result.pairs(), "comparison pairs")?;
            charge(&mut edges, result.edges(), "comparison edges")?;
            charge(&mut work, result.work(), "value matching work")?;
            result.equivalent()
        } else {
            if comparison.max_pairs == 0 || comparison.max_work == 0 {
                return Err(failure(GenericTypeEquivalenceError::Limit(
                    "scalar comparison admission",
                )));
            }
            charge(&mut pairs, 1, "comparison pairs")?;
            charge(&mut work, 1, "value matching work")?;
            matches!(expected.nodes()[expected.root()].kind,super::ResolvedGenericTypeKind::Primitive(p) if actual.ty.primitive()==Some(p))
        };
        let conversion = if same {
            GenericCopyConversion::Identity
        } else if actual.ty.primitive() == Some(GenericPrimitive::Int)
            && matches!(
                expected.nodes()[expected.root()].kind,
                super::ResolvedGenericTypeKind::Primitive(GenericPrimitive::Float)
            )
        {
            GenericCopyConversion::IntToFloat
        } else {
            return Err(GenericCallValueError::Type {
                argument: index,
                span: actual.span,
            });
        };
        matched.push(GenericCallValueArgument {
            expression: *id,
            conversion,
        });
    }
    Ok(CheckedGenericCallValues {
        signature: signature.clone(),
        body: body.clone(),
        values: matched.into(),
        pairs: limits.max_pairs - pairs,
        edges: limits.max_edges - edges,
        work: limits.max_work - work,
    })
}
