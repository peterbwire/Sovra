//! Bounded nominal constructor, field and enum match plans before instruction lowering.
use super::{
    CheckedGenericLayouts, GenericCopyBindingId, GenericCopyBindingKind, GenericCopyConversion,
    GenericCopyExpression, GenericCopyExpressionId, GenericCopyExpressionKind,
    GenericCopyStatement, GenericInstanceLayouts, GenericPrimitive, GenericRuntimeLayoutKind,
};
use crate::compiler::diagnostics::Span;
use std::{ops::Range, sync::Arc};
/// Cumulative nominal operation plan admission across the entire concrete graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericConstructionLimits {
    /// Concrete instance wrappers.
    pub max_instances: usize,
    /// Nominal constructor/field/match plan records.
    pub max_plans: usize,
    /// Source-order record field mappings and temporary coverage slots.
    pub max_fields: usize,
    /// Source expression operands, including receivers and match scrutinees.
    pub max_operands: usize,
    /// Source-order match arm plans.
    pub max_arms: usize,
    /// Exhaustive tag-to-arm entries and temporary coverage slots.
    pub max_dispatch: usize,
    /// Instance, source fact, conversion, field and arm processing work.
    pub max_work: usize,
}
impl GenericConstructionLimits {
    /// Finite planning profile; no value allocation, instruction emission or execution.
    pub const DEFAULT: Self = Self {
        max_instances: 1024,
        max_plans: 131_072,
        max_fields: 131_072,
        max_operands: 262_144,
        max_arms: 65_536,
        max_dispatch: 65_536,
        max_work: 4_000_000,
    };
}
/// Complete plan refusal without partially linked operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericConstructionError {
    /// Data layouts require separately qualified byte operations and result admission.
    DataOperationsUnavailable,
    /// Collection layouts require separately qualified collection operations/IR/VM support.
    CollectionOperationsUnavailable,
    /// Selected admission dimension exhausted.
    Limit(&'static str),
    /// A source identity, nominal slot/tag, layout, range or conversion is inconsistent.
    Context {
        /// Concrete graph instance index.
        instance: usize,
        /// Original function-source operation range.
        span: Span,
        /// Bounded refusal description.
        reason: &'static str,
    },
}
impl std::fmt::Display for GenericConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic nominal planning refused: {self:?}")
    }
}
impl std::error::Error for GenericConstructionError {}
/// Source operand with explicit source/target layout and retained conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPlannedOperand {
    /// Earlier original body expression to evaluate.
    pub expression: GenericCopyExpressionId,
    /// Original value layout, before contextual conversion.
    pub layout: usize,
    /// Expected field/payload layout after conversion.
    pub target_layout: usize,
    /// Original explicit conversion; no coercion is inferred here.
    pub conversion: GenericCopyConversion,
}
/// One record initializer in source evaluation order, with its destination slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericFieldInitialization {
    /// Original declaration-order field index.
    pub slot: usize,
    /// Original checked initializer operand.
    pub value: GenericPlannedOperand,
}
/// Nominal expression operation linked to one selected instance's layouts.
#[derive(Debug, Clone)]
pub enum GenericConstructionKind {
    /// Complete record construction in source evaluation order.
    Record {
        /// Destination slots can be out of order; operand order must be preserved.
        fields: Vec<GenericFieldInitialization>,
    },
    /// Enum construction with an exact declaration-order tag and payload shape.
    Enum {
        /// Original variant index in the nominal layout.
        tag: usize,
        /// None is nullary; a Unit payload remains Some.
        payload: Option<GenericPlannedOperand>,
    },
    /// Read a snapshot of an original receiver's declaration-order field.
    Field {
        /// Earlier original receiver expression.
        receiver: GenericCopyExpressionId,
        /// Receiver's exact record layout.
        receiver_layout: usize,
        /// Original field slot; output uses the enclosing expression plan layout.
        slot: usize,
    },
}
/// One original expression with a checked nominal operation plan.
#[derive(Debug, Clone)]
pub struct GenericConstructionExpression {
    /// Original source body arena identity.
    pub expression: GenericCopyExpressionId,
    /// Concrete result layout in the retained table.
    pub layout: usize,
    /// Original full-source expression range.
    pub span: Span,
    /// Complete nominal operation links.
    pub kind: GenericConstructionKind,
}
/// Immutable arm-local binding destination; runtime snapshot creation remains to be lowered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericMatchPayload {
    /// Original arm binding identity.
    pub binding: GenericCopyBindingId,
    /// Exact payload layout.
    pub layout: usize,
}
/// Original arm with its tag, optional payload destination and flat body range.
#[derive(Debug, Clone)]
pub struct GenericMatchArm {
    /// Declaration-order variant tag.
    pub tag: usize,
    /// None is nullary, including the distinction from a Unit payload binding.
    pub payload: Option<GenericMatchPayload>,
    /// Original statement range, including nested facts; not executable jump addresses.
    pub statements: Range<usize>,
    /// Original arm source range.
    pub span: Span,
}
/// Complete exhaustive dispatch for one source match statement.
#[derive(Debug, Clone)]
pub struct GenericMatchDispatch {
    /// Original flat statement index within the selected body.
    pub statement: usize,
    /// Single original source binding read; future execution must evaluate it once.
    pub scrutinee: GenericCopyExpressionId,
    /// Exact nominal enum layout.
    pub layout: usize,
    /// Original source-order arm plans.
    pub arms: Vec<GenericMatchArm>,
    /// Declaration-order tag maps to the source-order arm index.
    pub dispatch: Vec<usize>,
    /// Original full-source match range.
    pub span: Span,
}
/// Sparse nominal plans for a selected concrete instance.
#[derive(Debug, Clone)]
pub struct GenericConstructionBody {
    body: GenericInstanceLayouts,
    expressions: Arc<[GenericConstructionExpression]>,
    matches: Arc<[GenericMatchDispatch]>,
}
impl GenericConstructionBody {
    /// Original concrete source-to-layout mappings for this instance.
    pub fn body(&self) -> &GenericInstanceLayouts {
        &self.body
    }
    /// Nominal expression plans in original expression order.
    pub fn expressions(&self) -> &[GenericConstructionExpression] {
        &self.expressions
    }
    /// Match plans in original flat statement order.
    pub fn matches(&self) -> &[GenericMatchDispatch] {
        &self.matches
    }
    /// Lookup validates the original source arena; non-nominal expressions return None.
    pub fn expression(
        &self,
        id: GenericCopyExpressionId,
    ) -> Option<&GenericConstructionExpression> {
        self.body.body().body().expression(id)?;
        let at = self
            .expressions
            .binary_search_by_key(&id.ordinal(), |p| p.expression.ordinal())
            .ok()?;
        self.expressions.get(at)
    }
    /// Look up a match using this body's original statement index.
    pub fn match_statement(&self, index: usize) -> Option<&GenericMatchDispatch> {
        let at = self
            .matches
            .binary_search_by_key(&index, |p| p.statement)
            .ok()?;
        self.matches.get(at)
    }
}
/// Measured cumulative nominal planning usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericConstructionUsage {
    /// Concrete instance wrappers.
    pub instances: usize,
    /// Nominal expression/match plans.
    pub plans: usize,
    /// Record field slots.
    pub fields: usize,
    /// Source operands/receivers/scrutinees.
    pub operands: usize,
    /// Source match arms.
    pub arms: usize,
    /// Exhaustive dispatch entries.
    pub dispatch: usize,
    /// Consumed logical work.
    pub work: usize,
}
/// Complete linked nominal plans; this is not executable generic IR.
#[derive(Debug, Clone)]
pub struct CheckedGenericConstruction {
    layouts: CheckedGenericLayouts,
    bodies: Arc<[GenericConstructionBody]>,
    usage: GenericConstructionUsage,
}
impl CheckedGenericConstruction {
    /// Complete original layout/literal/concrete/source proofs.
    pub fn layouts(&self) -> &CheckedGenericLayouts {
        &self.layouts
    }
    /// Plans in specialization instance order.
    pub fn bodies(&self) -> &[GenericConstructionBody] {
        &self.bodies
    }
    /// Measured cumulative admission.
    pub fn usage(&self) -> GenericConstructionUsage {
        self.usage
    }
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<(), GenericConstructionError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericConstructionError::Limit(name))?;
    Ok(())
}
struct Builder<'a> {
    layouts: &'a CheckedGenericLayouts,
    left: GenericConstructionLimits,
    instance: usize,
}
impl Builder<'_> {
    fn error(&self, span: Span, reason: &'static str) -> GenericConstructionError {
        GenericConstructionError::Context {
            instance: self.instance,
            span,
            reason,
        }
    }
    fn step(&mut self) -> Result<(), GenericConstructionError> {
        charge(&mut self.left.max_work, 1, "nominal plan work")
    }
    fn shape(
        &self,
        index: usize,
        span: Span,
    ) -> Result<&GenericRuntimeLayoutKind, GenericConstructionError> {
        self.layouts
            .layout(index)
            .map(|l| l.kind())
            .ok_or_else(|| self.error(span, "unknown layout"))
    }
    fn expression_layout(
        &self,
        body: &GenericInstanceLayouts,
        id: GenericCopyExpressionId,
        span: Span,
    ) -> Result<usize, GenericConstructionError> {
        body.expression_layout(id)
            .ok_or_else(|| self.error(span, "foreign expression identity"))
    }
    fn operand(
        &mut self,
        body: &GenericInstanceLayouts,
        id: GenericCopyExpressionId,
        target: usize,
        conversion: GenericCopyConversion,
        at: usize,
        span: Span,
    ) -> Result<GenericPlannedOperand, GenericConstructionError> {
        charge(&mut self.left.max_operands, 1, "nominal plan operands")?;
        self.step()?;
        let layout = self.expression_layout(body, id, span)?;
        if id.ordinal() >= at {
            return Err(self.error(span, "operand must precede its expression"));
        }
        let valid = match conversion {
            GenericCopyConversion::Identity => layout == target,
            GenericCopyConversion::IntToFloat => {
                matches!(
                    self.shape(layout, span)?,
                    GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int)
                ) && matches!(
                    self.shape(target, span)?,
                    GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Float)
                )
            }
        };
        if !valid {
            return Err(self.error(span, "explicit operand conversion mismatch"));
        }
        Ok(GenericPlannedOperand {
            expression: id,
            layout,
            target_layout: target,
            conversion,
        })
    }
    fn expression(
        &mut self,
        body: &GenericInstanceLayouts,
        source: &GenericCopyExpression,
    ) -> Result<Option<GenericConstructionExpression>, GenericConstructionError> {
        self.step()?;
        if !matches!(
            source.kind,
            GenericCopyExpressionKind::Record { .. }
                | GenericCopyExpressionKind::Enum { .. }
                | GenericCopyExpressionKind::Field { .. }
        ) {
            return Ok(None);
        }
        charge(&mut self.left.max_plans, 1, "nominal plans")?;
        let layout = self.expression_layout(body, source.id, source.span)?;
        // Borrow the retained immutable table independently of mutable admission.
        let layouts = self.layouts;
        let kind = match &source.kind {
            GenericCopyExpressionKind::Record {
                declaration,
                fields,
                ..
            } => {
                let Some(GenericRuntimeLayoutKind::Record {
                    declaration: actual,
                    fields: slots,
                    ..
                }) = layouts.layout(layout).map(|l| l.kind())
                else {
                    return Err(self.error(source.span, "record result layout"));
                };
                if actual != declaration || slots.len() != fields.len() {
                    return Err(self.error(source.span, "record identity or field coverage"));
                }
                charge(
                    &mut self.left.max_fields,
                    slots.len(),
                    "nominal plan fields",
                )?;
                let mut seen = vec![false; slots.len()];
                let mut planned = Vec::new();
                for field in fields {
                    self.step()?;
                    let slot = field.field_index;
                    let expected = *slots
                        .get(slot)
                        .ok_or_else(|| self.error(source.span, "unknown field slot"))?;
                    if seen[slot] {
                        return Err(self.error(source.span, "duplicate field slot"));
                    }
                    seen[slot] = true;
                    planned.push(GenericFieldInitialization {
                        slot,
                        value: self.operand(
                            body,
                            field.value,
                            expected,
                            field.conversion,
                            source.id.ordinal(),
                            source.span,
                        )?,
                    });
                }
                GenericConstructionKind::Record { fields: planned }
            }
            GenericCopyExpressionKind::Enum {
                declaration,
                variant_index,
                payload,
                ..
            } => {
                let Some(GenericRuntimeLayoutKind::Enum {
                    declaration: actual,
                    variants,
                    ..
                }) = layouts.layout(layout).map(|l| l.kind())
                else {
                    return Err(self.error(source.span, "enum result layout"));
                };
                if actual != declaration {
                    return Err(self.error(source.span, "enum nominal identity"));
                }
                let expected = variants
                    .get(*variant_index)
                    .ok_or_else(|| self.error(source.span, "unknown enum tag"))?;
                let payload = match (expected, payload) {
                    (Some(expected), Some(value)) => Some(self.operand(
                        body,
                        value.value,
                        *expected,
                        value.conversion,
                        source.id.ordinal(),
                        source.span,
                    )?),
                    (None, None) => None,
                    _ => return Err(self.error(source.span, "enum payload shape")),
                };
                GenericConstructionKind::Enum {
                    tag: *variant_index,
                    payload,
                }
            }
            GenericCopyExpressionKind::Field {
                receiver,
                declaration,
                field_index,
                ..
            } => {
                charge(&mut self.left.max_operands, 1, "nominal plan operands")?;
                self.step()?;
                let receiver_layout = self.expression_layout(body, *receiver, source.span)?;
                if receiver.ordinal() >= source.id.ordinal() {
                    return Err(
                        self.error(source.span, "field receiver must precede its expression")
                    );
                }
                let Some(GenericRuntimeLayoutKind::Record {
                    declaration: actual,
                    fields,
                    ..
                }) = layouts.layout(receiver_layout).map(|l| l.kind())
                else {
                    return Err(self.error(source.span, "field receiver layout"));
                };
                if actual != declaration || fields.get(*field_index) != Some(&layout) {
                    return Err(self.error(source.span, "field identity or result layout"));
                }
                GenericConstructionKind::Field {
                    receiver: *receiver,
                    receiver_layout,
                    slot: *field_index,
                }
            }
            _ => unreachable!("nominal expression filter"),
        };
        Ok(Some(GenericConstructionExpression {
            expression: source.id,
            layout,
            span: source.span,
            kind,
        }))
    }
    fn match_statement(
        &mut self,
        body: &GenericInstanceLayouts,
        index: usize,
        source: &GenericCopyStatement,
    ) -> Result<Option<GenericMatchDispatch>, GenericConstructionError> {
        self.step()?;
        let GenericCopyStatement::Match {
            scrutinee,
            declaration,
            arms,
            span,
        } = source
        else {
            return Ok(None);
        };
        charge(&mut self.left.max_plans, 1, "nominal plans")?;
        charge(&mut self.left.max_operands, 1, "nominal plan operands")?;
        let layout = self.expression_layout(body, *scrutinee, *span)?;
        if !matches!(
            body.body().body().expression(*scrutinee).map(|e| &e.kind),
            Some(GenericCopyExpressionKind::Read(_))
        ) {
            return Err(self.error(*span, "match requires a single source binding read"));
        }
        let layouts = self.layouts;
        let Some(GenericRuntimeLayoutKind::Enum {
            declaration: actual,
            variants,
            ..
        }) = layouts.layout(layout).map(|l| l.kind())
        else {
            return Err(self.error(*span, "match scrutinee layout"));
        };
        if actual != declaration || arms.len() != variants.len() {
            return Err(self.error(*span, "match identity or coverage"));
        }
        charge(&mut self.left.max_arms, arms.len(), "nominal plan arms")?;
        charge(
            &mut self.left.max_dispatch,
            variants.len(),
            "nominal plan dispatch",
        )?;
        let mut dispatch = vec![usize::MAX; variants.len()];
        let mut planned = Vec::new();
        let mut end = index + 1;
        for (arm_index, arm) in arms.iter().enumerate() {
            self.step()?;
            let expected = variants
                .get(arm.variant_index)
                .ok_or_else(|| self.error(arm.span, "unknown match tag"))?;
            if dispatch[arm.variant_index] != usize::MAX {
                return Err(self.error(arm.span, "duplicate match tag"));
            }
            if arm.statements.start != end
                || arm.statements.end < end
                || arm.statements.end > body.body().body().statements().len()
            {
                return Err(self.error(arm.span, "match arm statement range"));
            }
            for statement in &body.body().body().statements()[arm.statements.clone()] {
                self.step()?;
                let statement_span = match statement {
                    GenericCopyStatement::Let { span, .. }
                    | GenericCopyStatement::Assign { span, .. }
                    | GenericCopyStatement::IndexAssign { span, .. }
                    | GenericCopyStatement::If { span, .. }
                    | GenericCopyStatement::While { span, .. }
                    | GenericCopyStatement::Return { span, .. }
                    | GenericCopyStatement::Match { span, .. } => *span,
                    GenericCopyStatement::Expression { value } => {
                        body.body()
                            .body()
                            .expression(*value)
                            .ok_or_else(|| self.error(arm.span, "foreign arm expression"))?
                            .span
                    }
                };
                if statement_span.start < arm.span.start || statement_span.end > arm.span.end {
                    return Err(self.error(arm.span, "match arm range escapes its source arm"));
                }
            }
            end = arm.statements.end;
            let payload = match (expected, arm.payload_binding) {
                (Some(expected), Some(binding)) => {
                    let layout = body
                        .binding_layout(binding)
                        .ok_or_else(|| self.error(arm.span, "foreign payload binding"))?;
                    let kind = body.body().body().binding(binding).map(|b| b.kind);
                    if layout != *expected
                        || kind
                            != Some(GenericCopyBindingKind::MatchPayload {
                                scrutinee: *scrutinee,
                                variant_index: arm.variant_index,
                            })
                    {
                        return Err(self.error(arm.span, "payload binding layout or origin"));
                    }
                    Some(GenericMatchPayload { binding, layout })
                }
                (None, None) => None,
                _ => return Err(self.error(arm.span, "match payload shape")),
            };
            dispatch[arm.variant_index] = arm_index;
            planned.push(GenericMatchArm {
                tag: arm.variant_index,
                payload,
                statements: arm.statements.clone(),
                span: arm.span,
            });
        }
        Ok(Some(GenericMatchDispatch {
            statement: index,
            scrutinee: *scrutinee,
            layout,
            arms: planned,
            dispatch,
            span: *span,
        }))
    }
}
/// Link every nominal constructor, field read and exhaustive match to concrete
/// layouts while preserving source operand/arm order and conversion edges.
/// This produces neither runtime values nor executable instructions/jump addresses.
pub fn plan_generic_construction(
    layouts: &CheckedGenericLayouts,
    limits: GenericConstructionLimits,
) -> Result<CheckedGenericConstruction, GenericConstructionError> {
    plan_construction(layouts, limits, false, false)
}
/// Opt-in nominal planning for the qualified collection operation IR path.
pub fn plan_generic_collection_construction(
    layouts: &CheckedGenericLayouts,
    limits: GenericConstructionLimits,
) -> Result<CheckedGenericConstruction, GenericConstructionError> {
    plan_construction(layouts, limits, true, false)
}
/// Plan explicitly admitted Byte and immutable Bytes operations.
pub fn plan_generic_data_construction(
    layouts: &CheckedGenericLayouts,
    limits: GenericConstructionLimits,
) -> Result<CheckedGenericConstruction, GenericConstructionError> {
    plan_construction(layouts, limits, true, true)
}
fn plan_construction(
    layouts: &CheckedGenericLayouts,
    limits: GenericConstructionLimits,
    collections: bool,
    data: bool,
) -> Result<CheckedGenericConstruction, GenericConstructionError> {
    if layouts.has_data() && !data {
        return Err(GenericConstructionError::DataOperationsUnavailable);
    }
    if layouts.has_collections() && !collections {
        return Err(GenericConstructionError::CollectionOperationsUnavailable);
    }
    let mut b = Builder {
        layouts,
        left: limits,
        instance: 0,
    };
    let mut bodies = Vec::new();
    for (instance, body) in layouts.bodies().iter().enumerate() {
        b.instance = instance;
        charge(&mut b.left.max_instances, 1, "nominal plan instances")?;
        b.step()?;
        let mut expressions = Vec::new();
        let mut matches = Vec::new();
        for source in body.body().body().expressions() {
            if let Some(plan) = b.expression(body, source)? {
                expressions.push(plan);
            }
        }
        for (index, source) in body.body().body().statements().iter().enumerate() {
            if let Some(plan) = b.match_statement(body, index, source)? {
                matches.push(plan);
            }
        }
        bodies.push(GenericConstructionBody {
            body: body.clone(),
            expressions: expressions.into(),
            matches: matches.into(),
        });
    }
    Ok(CheckedGenericConstruction {
        layouts: layouts.clone(),
        bodies: bodies.into(),
        usage: GenericConstructionUsage {
            instances: limits.max_instances - b.left.max_instances,
            plans: limits.max_plans - b.left.max_plans,
            fields: limits.max_fields - b.left.max_fields,
            operands: limits.max_operands - b.left.max_operands,
            arms: limits.max_arms - b.left.max_arms,
            dispatch: limits.max_dispatch - b.left.max_dispatch,
            work: limits.max_work - b.left.max_work,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::v2::*;
    fn fixture() -> CheckedGenericLayouts {
        let pair = parse_generic_record(
            "struct Pair<A,B>{left:A,right:B}",
            GenericRecordLimits::DEFAULT,
        )
        .unwrap();
        let choice =
            parse_generic_enum("enum Choice<T>{End,One(T)}", GenericEnumLimits::DEFAULT).unwrap();
        let definitions = [
            GenericTypeDeclaration::Record(&pair),
            GenericTypeDeclaration::Enum(&choice),
        ];
        let registry =
            register_generic_types(&definitions, GenericRegistryLimits::DEFAULT).unwrap();
        let env = prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT)
            .unwrap();
        let types =
            check_generic_declarations(&env, &definitions, GenericDeclarationLimits::DEFAULT)
                .unwrap();
        let root=parse_specialization_root("fn root(value:Choice<Int>){let pair=Pair::<Int,Float>{right:2,left:1};match value {Choice::End()=>{},Choice::One(x)=>{}}let after=3;}",GenericFunctionLimits::DEFAULT).unwrap();
        let signatures =
            check_generic_signatures(&types, &[&root], GenericSignatureLimits::DEFAULT).unwrap();
        let bodies =
            check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
        let graph = discover_generic_specializations(
            &bodies,
            signatures.find("root").unwrap(),
            GenericSpecializationLimits::DEFAULT,
        )
        .unwrap();
        let concrete =
            recheck_generic_concrete_bodies(&graph, GenericConcreteBodyLimits::DEFAULT).unwrap();
        let literals = decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap();
        plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap()
    }
    fn builder(layouts: &CheckedGenericLayouts) -> Builder<'_> {
        Builder {
            layouts,
            left: GenericConstructionLimits::DEFAULT,
            instance: 0,
        }
    }
    #[test]
    fn duplicate_slots_and_incorrect_stored_conversions_are_not_repaired() {
        let layouts = fixture();
        let body = &layouts.bodies()[0];
        let source = body
            .body()
            .body()
            .expressions()
            .iter()
            .find(|e| matches!(e.kind, GenericCopyExpressionKind::Record { .. }))
            .unwrap();
        let mut duplicate = source.clone();
        let GenericCopyExpressionKind::Record { fields, .. } = &mut duplicate.kind else {
            unreachable!()
        };
        let slot = fields[0].field_index;
        fields[1].field_index = slot;
        assert!(matches!(
            builder(&layouts).expression(body, &duplicate),
            Err(GenericConstructionError::Context {
                reason: "duplicate field slot",
                ..
            })
        ));
        let mut conversion = source.clone();
        let GenericCopyExpressionKind::Record { fields, .. } = &mut conversion.kind else {
            unreachable!()
        };
        fields[0].conversion = GenericCopyConversion::Identity;
        assert!(matches!(
            builder(&layouts).expression(body, &conversion),
            Err(GenericConstructionError::Context {
                reason: "explicit operand conversion mismatch",
                ..
            })
        ));
    }
    #[test]
    fn duplicate_dispatch_ranges_and_missing_payload_destinations_refuse() {
        let layouts = fixture();
        let body = &layouts.bodies()[0];
        let (index, source) = body
            .body()
            .body()
            .statements()
            .iter()
            .enumerate()
            .find(|(_, s)| matches!(s, GenericCopyStatement::Match { .. }))
            .unwrap();
        let mut duplicate = source.clone();
        let GenericCopyStatement::Match { arms, .. } = &mut duplicate else {
            unreachable!()
        };
        arms[1].variant_index = arms[0].variant_index;
        assert!(matches!(
            builder(&layouts).match_statement(body, index, &duplicate),
            Err(GenericConstructionError::Context {
                reason: "duplicate match tag",
                ..
            })
        ));
        let mut range = source.clone();
        let GenericCopyStatement::Match { arms, .. } = &mut range else {
            unreachable!()
        };
        arms[0].statements.start = index;
        assert!(matches!(
            builder(&layouts).match_statement(body, index, &range),
            Err(GenericConstructionError::Context {
                reason: "match arm statement range",
                ..
            })
        ));
        let mut escaped = source.clone();
        let GenericCopyStatement::Match { arms, .. } = &mut escaped else {
            unreachable!()
        };
        arms[1].statements.end = body.body().body().statements().len();
        assert!(matches!(
            builder(&layouts).match_statement(body, index, &escaped),
            Err(GenericConstructionError::Context {
                reason: "match arm range escapes its source arm",
                ..
            })
        ));
        let mut payload = source.clone();
        let GenericCopyStatement::Match { arms, .. } = &mut payload else {
            unreachable!()
        };
        arms[1].payload_binding = None;
        assert!(matches!(
            builder(&layouts).match_statement(body, index, &payload),
            Err(GenericConstructionError::Context {
                reason: "match payload shape",
                ..
            })
        ));
    }
}
