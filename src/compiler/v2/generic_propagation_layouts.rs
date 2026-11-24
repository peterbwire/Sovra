//! Bounded logical propagation operations. Does not authorize execution.
use super::*;
use crate::compiler::diagnostics::Span;

/// Shared admission for the complete layout table.
#[derive(Debug, Clone, Copy)]
pub struct GenericPropagationLayoutLimits {
    /// Function instances visited.
    pub max_instances: usize,
    /// Propagation operations retained.
    pub max_plans: usize,
    /// Shape checks and type comparison work across all instances.
    pub max_work: usize,
    /// Shared exact type comparison allowance.
    pub comparison: GenericTypeEquivalenceLimits,
}
impl GenericPropagationLayoutLimits {
    /// Finite opt-in library profile.
    pub const DEFAULT: Self = Self {
        max_instances: 1024,
        max_plans: 4096,
        max_work: 1_000_000,
        comparison: GenericTypeEquivalenceLimits::DEFAULT,
    };
}
/// Logical exhaustion or inconsistent retained proofs; no partial output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericPropagationLayoutError {
    /// Whole-table allowance exhausted.
    Limit(&'static str),
    /// A checked mapping and its wrapper shape disagree.
    Context,
    /// Bounded exact comparison refused.
    Comparison(GenericTypeEquivalenceError),
}
impl std::fmt::Display for GenericPropagationLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic propagation layout refused: {self:?}")
    }
}
impl std::error::Error for GenericPropagationLayoutError {}
/// One extraction and possible caller-return construction, in source fact order.
#[derive(Debug, Clone)]
pub struct GenericPropagationLayoutPlan {
    /// Owning concrete function instance in the retained table.
    pub instance: usize,
    /// Source expression whose value is the extracted payload.
    pub expression: GenericCopyExpressionId,
    /// Operand evaluated once by future lowering.
    pub operand: GenericCopyExpressionId,
    /// Original question-mark range in the owning function.
    pub question: Span,
    /// Input wrapper layout.
    pub input: usize,
    /// Successful extraction layout.
    pub success: usize,
    /// Enclosing function's return wrapper layout.
    pub result: usize,
    /// Success variant tag.
    pub success_tag: usize,
    /// Failure variant tag in both standard wrappers.
    pub failure_tag: usize,
    /// Exact failure payload layout; nullary Option failure has none.
    pub failure: Option<usize>,
}
/// Complete immutable owner plus logical operations; not executable IR.
#[derive(Debug, Clone)]
pub struct CheckedGenericPropagationLayouts {
    layouts: CheckedGenericLayouts,
    plans: Vec<GenericPropagationLayoutPlan>,
}
impl CheckedGenericPropagationLayouts {
    /// Layout indices refer exclusively to this retained owner.
    pub fn layouts(&self) -> &CheckedGenericLayouts {
        &self.layouts
    }
    /// All concrete propagation sites, grouped by instance then source order.
    pub fn plans(&self) -> &[GenericPropagationLayoutPlan] {
        &self.plans
    }
}
fn charge(
    left: &mut usize,
    n: usize,
    name: &'static str,
) -> Result<(), GenericPropagationLayoutError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericPropagationLayoutError::Limit(name))?;
    Ok(())
}
fn same(
    a: &GenericCopyType,
    b: &GenericCopyType,
    left: &mut GenericPropagationLayoutLimits,
) -> Result<(), GenericPropagationLayoutError> {
    charge(&mut left.max_work, 1, "work")?;
    match (a.primitive(), b.primitive()) {
        (Some(a), Some(b)) if a == b => {
            charge(&mut left.comparison.max_pairs, 1, "comparison pairs")?;
            charge(&mut left.comparison.max_work, 1, "comparison work")?;
            Ok(())
        }
        (None, None) => {
            let mut cap = left.comparison;
            cap.max_work = cap.max_work.min(left.max_work);
            let result = compare_generic_types(
                a.expanded().ok_or(GenericPropagationLayoutError::Context)?,
                b.expanded().ok_or(GenericPropagationLayoutError::Context)?,
                cap,
            )
            .map_err(GenericPropagationLayoutError::Comparison)?;
            charge(
                &mut left.comparison.max_pairs,
                result.pairs(),
                "comparison pairs",
            )?;
            charge(
                &mut left.comparison.max_edges,
                result.edges(),
                "comparison edges",
            )?;
            charge(
                &mut left.comparison.max_work,
                result.work(),
                "comparison work",
            )?;
            charge(&mut left.max_work, result.work(), "work")?;
            if result.equivalent() {
                Ok(())
            } else {
                Err(GenericPropagationLayoutError::Context)
            }
        }
        _ => Err(GenericPropagationLayoutError::Context),
    }
}
/// Plan extraction/failure layouts from sealed concrete propagation checks.
/// Successful payloads may differ from the caller's successful return payload.
pub fn plan_generic_propagation_layouts(
    layouts: &CheckedGenericLayouts,
    mut limits: GenericPropagationLayoutLimits,
) -> Result<CheckedGenericPropagationLayouts, GenericPropagationLayoutError> {
    let mut plans = Vec::new();
    for (instance, mapped) in layouts.bodies().iter().enumerate() {
        charge(&mut limits.max_instances, 1, "instances")?;
        charge(&mut limits.max_work, 1, "work")?;
        let body = mapped.body();
        for p in body.propagations() {
            charge(&mut limits.max_plans, 1, "plans")?;
            charge(&mut limits.max_work, 8, "work")?;
            let input = mapped
                .expression_layout(p.operand())
                .ok_or(GenericPropagationLayoutError::Context)?;
            let success = mapped
                .expression_layout(p.expression())
                .ok_or(GenericPropagationLayoutError::Context)?;
            let result = mapped.return_layout();
            let get = |id| {
                layouts
                    .layout(id)
                    .ok_or(GenericPropagationLayoutError::Context)
            };
            same(
                get(input)?.ty(),
                body.expression_type(p.operand())
                    .ok_or(GenericPropagationLayoutError::Context)?,
                &mut limits,
            )?;
            same(
                get(success)?.ty(),
                body.expression_type(p.expression())
                    .ok_or(GenericPropagationLayoutError::Context)?,
                &mut limits,
            )?;
            same(get(result)?.ty(), body.return_type(), &mut limits)?;
            let shape = |id| -> Result<_, GenericPropagationLayoutError> {
                match get(id)?.kind() {
                    GenericRuntimeLayoutKind::Enum {
                        declaration,
                        arguments,
                        variants,
                    } if *declaration == p.types().declaration() && variants.len() == 2 => {
                        Ok((arguments, variants))
                    }
                    _ => Err(GenericPropagationLayoutError::Context),
                }
            };
            let (args, tags) = shape(input)?;
            let (ret_args, ret_tags) = shape(result)?;
            let s = p.types().success_variant();
            let f = p.types().failure_variant();
            if tags.get(s) != Some(&Some(success))
                || args.first() != Some(&success)
                || ret_tags.get(s) != ret_args.first().map(|v| Some(*v)).as_ref()
            {
                return Err(GenericPropagationLayoutError::Context);
            }
            let failure = match p.types().kind() {
                GenericStandardEnum::Option
                    if args.len() == 1
                        && ret_args.len() == 1
                        && tags.get(f) == Some(&None)
                        && ret_tags.get(f) == Some(&None) =>
                {
                    None
                }
                GenericStandardEnum::Result
                    if args.len() == 2
                        && ret_args.len() == 2
                        && args[1] == ret_args[1]
                        && tags.get(f) == Some(&Some(args[1]))
                        && ret_tags.get(f) == Some(&Some(args[1])) =>
                {
                    Some(args[1])
                }
                _ => return Err(GenericPropagationLayoutError::Context),
            };
            plans.push(GenericPropagationLayoutPlan {
                instance,
                expression: p.expression(),
                operand: p.operand(),
                question: p.question(),
                input,
                success,
                result,
                success_tag: s,
                failure_tag: f,
                failure,
            });
        }
    }
    Ok(CheckedGenericPropagationLayouts {
        layouts: layouts.clone(),
        plans,
    })
}
