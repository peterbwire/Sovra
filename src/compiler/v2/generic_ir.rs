//! Typed stack instructions and resolved control flow for concrete generic functions.
use super::generic_data_types::DataResultKind;
use super::*;
use crate::compiler::diagnostics::Span;
use std::{collections::BTreeMap, ops::Range, sync::Arc};
/// Cumulative instruction emission and typed control-flow validation admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericIrLimits {
    /// Function instances.
    pub max_functions: usize,
    /// Emitted instructions, including conversions/control flow.
    pub max_instructions: usize,
    /// Local binding slots, including parameters and match payloads.
    pub max_slots: usize,
    /// Stack operand and local-slot references in emitted instructions.
    pub max_operands: usize,
    /// Temporary labels admitted before fixup storage.
    pub max_labels: usize,
    /// Explicit jump/branch/switch target references.
    pub max_targets: usize,
    /// Simultaneously pending emission tasks, admitted before fan-out allocation.
    pub max_pending: usize,
    /// Canonical persistent typed-stack nodes used by validation.
    pub max_stack_nodes: usize,
    /// Maximum typed operand-stack depth per function.
    pub max_stack_depth: usize,
    /// Source traversal, emission, fixups, stack and control-flow validation work.
    pub max_work: usize,
}
impl GenericIrLimits {
    /// Finite library lowering profile; runtime and CLI integration remain separate.
    pub const DEFAULT: Self = Self {
        max_functions: 1024,
        max_instructions: 1_000_000,
        max_slots: 131_072,
        max_operands: 2_000_000,
        max_labels: 262_144,
        max_targets: 524_288,
        max_pending: 262_144,
        max_stack_nodes: 262_144,
        max_stack_depth: 65_536,
        max_work: 16_000_000,
    };
}
/// Complete instruction/validation refusal, never partially executable IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericIrError {
    /// Source propagation requires early-return/control-flow lowering integration.
    PropagationUnavailable,
    /// Nominal construction admission for opt-in propagation lowering.
    Construction(GenericConstructionError),
    /// Selected allowance exhausted.
    Limit(&'static str),
    /// Source/slot/layout/label/stack consistency failure.
    Invalid {
        /// Concrete function instance index.
        function: usize,
        /// Original operation range.
        span: Span,
        /// Bounded refusal description.
        reason: &'static str,
    },
}
impl std::fmt::Display for GenericIrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic IR lowering refused: {self:?}")
    }
}
impl std::error::Error for GenericIrError {}
/// Primitive/structural binary operation after explicit operand conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericIrBinary {
    /// Existing concrete numeric operation.
    Numeric(GenericNumericOperator),
    /// Existing concrete String operation.
    String(GenericStringOperator),
    /// Structural equality or inequality.
    Equality {
        /// True is inequality.
        negated: bool,
    },
}
/// Stack instructions; all addresses/slots/layouts/targets belong to the enclosing IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericIrOperation {
    /// Consume checked arguments and produce the operation's exact result.
    Builtin {
        /// Exact host-free operation.
        builtin: GenericBuiltin,
        /// Explicit right operand conversion target for append/concat.
        right_layout: Option<usize>,
        /// Checked primitive, array or compiler-owned record result layout.
        layout: usize,
    },
    /// Push an exact primitive payload.
    Constant {
        /// Decoded value.
        value: GenericLiteralValue,
        /// Value layout.
        layout: usize,
    },
    /// Materialize the existing Unit value convention, not a source literal.
    Unit {
        /// Unit layout.
        layout: usize,
    },
    /// Push a value snapshot from a local slot.
    Load {
        /// Original binding slot.
        slot: usize,
    },
    /// Pop a replacement into a local slot.
    Store {
        /// Original binding slot.
        slot: usize,
        /// True initializes/re-enters a lexical local; false is mutable assignment.
        initialize: bool,
    },
    /// Replace one mutable local array element, popping value then Int index.
    IndexStore {
        /// Mutable array slot.
        slot: usize,
    },
    /// Discard one value.
    Drop,
    /// Widen top-of-stack Int to Float.
    IntToFloat {
        /// Float layout.
        layout: usize,
    },
    /// Pop right then left and push the typed result.
    Binary {
        /// Existing operation.
        operator: GenericIrBinary,
        /// Result layout.
        layout: usize,
    },
    /// Pop source-order items and push an owned array value.
    Array {
        /// Array layout.
        layout: usize,
        /// Number of items.
        count: usize,
    },
    /// Pop source-order operands into the named declaration slots.
    Record {
        /// Record layout.
        layout: usize,
        /// Slots in original evaluation order.
        slots: Vec<usize>,
    },
    /// Construct one tagged enum, consuming a payload exactly when present.
    Enum {
        /// Enum layout.
        layout: usize,
        /// Declaration-order tag.
        tag: usize,
        /// Payload presence, including Unit.
        payload: bool,
    },
    /// Pop a record snapshot and push its field snapshot.
    Field {
        /// Receiver record layout.
        receiver: usize,
        /// Declaration-order slot.
        slot: usize,
        /// Result layout.
        layout: usize,
    },
    /// Pop Int index then array snapshot, and push an element snapshot.
    Index {
        /// Element result layout.
        layout: usize,
    },
    /// Pop value arguments in reverse source order and push the returned value, including Unit.
    Call {
        /// Concrete graph function instance.
        target: usize,
        /// Value argument count.
        arguments: usize,
    },
    /// Unconditional function-local instruction address.
    Jump {
        /// Resolved destination.
        target: usize,
    },
    /// Pop Bool and select a resolved instruction address.
    Branch {
        /// True destination.
        yes: usize,
        /// False destination.
        no: usize,
    },
    /// Inspect the retained top enum snapshot and dispatch by declaration-order tag.
    Switch {
        /// Expected enum layout.
        layout: usize,
        /// One resolved target per tag; each begins with MatchBind.
        targets: Vec<usize>,
    },
    /// Pop the selected enum snapshot and initialize its immutable payload destination.
    MatchBind {
        /// Enum layout.
        layout: usize,
        /// Selected declaration-order tag.
        tag: usize,
        /// Payload slot, including Unit; None is nullary.
        binding: Option<usize>,
    },
    /// Pop one wrapper. Success pushes its payload and continues. Failure constructs
    /// the enclosing return wrapper and exits, discarding all current-frame operands
    /// and locals. Execution requires the explicit propagation VM entry point.
    Propagate {
        /// Operand wrapper layout.
        input: usize,
        /// Extracted successful payload layout.
        success: usize,
        /// Enclosing function return wrapper layout.
        result: usize,
        /// Standard success tag.
        success_tag: usize,
        /// Standard failure tag shared by input and return wrappers.
        failure_tag: usize,
        /// Exact failure payload layout, or None for nullary Option failure.
        failure: Option<usize>,
    },
    /// Pop exactly one value of the function's result layout and exit.
    Return,
    /// Unreachable terminal sentinel; validation rejects any reachable path here.
    Unreachable,
}
/// One owned instruction with original source provenance.
#[derive(Debug, Clone)]
pub struct GenericIrInstruction {
    /// Complete operation with resolved function-local addresses.
    pub operation: GenericIrOperation,
    /// Original full-function source range.
    pub span: Span,
}
/// Complete concrete function instruction stream.
#[derive(Debug, Clone)]
pub struct GenericIrFunction {
    parameter_slots: Arc<[usize]>,
    slot_layouts: Arc<[usize]>,
    slot_kinds: Arc<[GenericCopyBindingKind]>,
    result: usize,
    code: Arc<[GenericIrInstruction]>,
    max_stack: usize,
}
impl GenericIrFunction {
    /// Incoming arguments initialize these source-order local slots.
    pub fn parameter_slots(&self) -> &[usize] {
        &self.parameter_slots
    }
    /// Concrete local slot layouts.
    pub fn slot_layouts(&self) -> &[usize] {
        &self.slot_layouts
    }
    /// Original source binding origins/mutability, not caller-editable slot authority.
    pub fn slot_kinds(&self) -> &[GenericCopyBindingKind] {
        &self.slot_kinds
    }
    /// Concrete result layout.
    pub fn return_layout(&self) -> usize {
        self.result
    }
    /// Resolved, structurally validated instructions.
    pub fn instructions(&self) -> &[GenericIrInstruction] {
        &self.code
    }
    /// Maximum reachable operand-stack depth established by typed CFG validation.
    pub fn max_stack(&self) -> usize {
        self.max_stack
    }
}
/// Exact emission/validation usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericIrUsage {
    /// Concrete functions.
    pub functions: usize,
    /// Instructions.
    pub instructions: usize,
    /// Local slots.
    pub slots: usize,
    /// Stack/local operand references.
    pub operands: usize,
    /// Allocated temporary labels.
    pub labels: usize,
    /// Explicit control-flow target references.
    pub targets: usize,
    /// Peak simultaneous emission tasks, including temporary fan-out sequences.
    pub peak_pending: usize,
    /// Canonical validation stack nodes.
    pub stack_nodes: usize,
    /// Logical processing work.
    pub work: usize,
}
/// Owned executable-form generic instructions, not a runtime, image codec or CLI authorization.
#[derive(Debug, Clone)]
pub struct CheckedGenericIr {
    collections: bool,
    data: bool,
    plans: CheckedGenericConstruction,
    functions: Arc<[GenericIrFunction]>,
    usage: GenericIrUsage,
}
impl CheckedGenericIr {
    /// Whether this IR requires explicitly qualified scalar data execution.
    pub fn has_data_operations(&self) -> bool {
        self.data
    }
    /// Whether this IR requires explicit collection execution and a future native feature.
    pub fn has_collection_operations(&self) -> bool {
        self.collections
    }
    /// Retained source/type/layout/nominal operation proofs.
    pub fn plans(&self) -> &CheckedGenericConstruction {
        &self.plans
    }
    /// Concrete graph instance order, with root index zero.
    pub fn functions(&self) -> &[GenericIrFunction] {
        &self.functions
    }
    /// Exact cumulative lowering/validation admission.
    pub fn usage(&self) -> GenericIrUsage {
        self.usage
    }
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<(), GenericIrError> {
    *left = left.checked_sub(n).ok_or(GenericIrError::Limit(name))?;
    Ok(())
}
struct Admission {
    left: GenericIrLimits,
    function: usize,
    span: Span,
    pending_limit: usize,
    pending_base: usize,
    peak_pending: usize,
}
impl Admission {
    fn step(&mut self) -> Result<(), GenericIrError> {
        charge(&mut self.left.max_work, 1, "generic IR work")
    }
    fn schedule(&mut self, count: usize) -> Result<(), GenericIrError> {
        let total = self
            .pending_base
            .checked_add(count)
            .ok_or(GenericIrError::Limit("generic IR pending tasks"))?;
        if total > self.pending_limit {
            return Err(GenericIrError::Limit("generic IR pending tasks"));
        }
        charge(&mut self.left.max_work, count, "generic IR work")?;
        self.peak_pending = self.peak_pending.max(total);
        Ok(())
    }
    fn fanout(&self, count: usize) -> Result<(), GenericIrError> {
        if count > self.pending_limit.saturating_sub(self.pending_base) {
            return Err(GenericIrError::Limit("generic IR pending tasks"));
        }
        Ok(())
    }
    fn invalid(&self, reason: &'static str) -> GenericIrError {
        GenericIrError::Invalid {
            function: self.function,
            span: self.span,
            reason,
        }
    }
}
// Labels use temporary ordinals only inside emission; all are resolved before publication.
enum Work {
    Block(Range<usize>),
    Expr(GenericCopyExpressionId),
    Emit(GenericIrOperation, Span),
    Mark(usize),
}
struct Emitter<'a, 'b> {
    plans: &'a CheckedGenericConstruction,
    body: &'a GenericConstructionBody,
    a: &'b mut Admission,
    labels: Vec<Option<usize>>,
    code: Vec<GenericIrInstruction>,
    calls: BTreeMap<usize, usize>,
    float: Option<usize>,
    propagations: BTreeMap<usize, GenericPropagationLayoutPlan>,
}
impl Emitter<'_, '_> {
    fn label(&mut self) -> Result<usize, GenericIrError> {
        charge(&mut self.a.left.max_labels, 1, "generic IR labels")?;
        self.a.step()?;
        let id = self.labels.len();
        self.labels.push(None);
        Ok(id)
    }
    fn layout(&self, id: GenericCopyExpressionId) -> Result<usize, GenericIrError> {
        self.body
            .body()
            .expression_layout(id)
            .ok_or_else(|| self.a.invalid("source expression identity"))
    }
    fn slot(&self, id: GenericCopyBindingId) -> Result<usize, GenericIrError> {
        self.body
            .body()
            .binding_layout(id)
            .ok_or_else(|| self.a.invalid("source binding identity"))?;
        Ok(id.ordinal())
    }
    fn convert(
        &self,
        conversion: GenericCopyConversion,
        span: Span,
    ) -> Result<Option<Work>, GenericIrError> {
        Ok(match conversion {
            GenericCopyConversion::Identity => None,
            GenericCopyConversion::IntToFloat => Some(Work::Emit(
                GenericIrOperation::IntToFloat {
                    layout: self
                        .float
                        .ok_or_else(|| self.a.invalid("missing Float layout"))?,
                },
                span,
            )),
        })
    }
    fn value(
        &self,
        id: GenericCopyExpressionId,
        conversion: GenericCopyConversion,
        span: Span,
        out: &mut Vec<Work>,
    ) -> Result<(), GenericIrError> {
        out.push(Work::Expr(id));
        if let Some(work) = self.convert(conversion, span)? {
            out.push(work);
        }
        Ok(())
    }
    fn emit(&mut self, operation: GenericIrOperation, span: Span) -> Result<(), GenericIrError> {
        self.a.span = span;
        self.a.step()?;
        charge(
            &mut self.a.left.max_instructions,
            1,
            "generic IR instructions",
        )?;
        let operands = match &operation {
            GenericIrOperation::Builtin { builtin, .. } => builtin.arity(),
            GenericIrOperation::Load { .. } | GenericIrOperation::Propagate { .. } => 1,
            GenericIrOperation::Store { .. } => 2,
            GenericIrOperation::IndexStore { .. } => 3,
            GenericIrOperation::Drop
            | GenericIrOperation::IntToFloat { .. }
            | GenericIrOperation::Field { .. }
            | GenericIrOperation::Branch { .. }
            | GenericIrOperation::Return
            | GenericIrOperation::Switch { .. } => 1,
            GenericIrOperation::Binary { .. } | GenericIrOperation::Index { .. } => 2,
            GenericIrOperation::Array { count, .. } => *count,
            GenericIrOperation::Record { slots, .. } => slots.len(),
            GenericIrOperation::Enum { payload, .. } => usize::from(*payload),
            GenericIrOperation::Call { arguments, .. } => *arguments,
            GenericIrOperation::MatchBind { binding, .. } => 1 + usize::from(binding.is_some()),
            _ => 0,
        };
        charge(
            &mut self.a.left.max_operands,
            operands,
            "generic IR operands",
        )?;
        let targets = match &operation {
            GenericIrOperation::Propagate { .. } => 2,
            GenericIrOperation::Jump { .. } => 1,
            GenericIrOperation::Branch { .. } => 2,
            GenericIrOperation::Switch { targets, .. } => targets.len(),
            _ => 0,
        };
        charge(&mut self.a.left.max_targets, targets, "generic IR targets")?;
        self.code.push(GenericIrInstruction { operation, span });
        Ok(())
    }
    fn expression(
        &mut self,
        id: GenericCopyExpressionId,
        out: &mut Vec<Work>,
    ) -> Result<(), GenericIrError> {
        let source = self
            .body
            .body()
            .body()
            .body()
            .expression(id)
            .ok_or_else(|| self.a.invalid("source expression"))?;
        self.a.span = source.span;
        let span = source.span;
        let layout = self.layout(id)?;
        let value_frames = |conversion: GenericCopyConversion| {
            1 + usize::from(conversion == GenericCopyConversion::IntToFloat)
        };
        let frames = match &source.kind {
            GenericCopyExpressionKind::Builtin { arguments, .. } => 1 + arguments.len(),
            GenericCopyExpressionKind::Propagate { .. } => 2,
            GenericCopyExpressionKind::Call { arguments, .. } => {
                self.a.fanout(arguments.len())?;
                1 + arguments
                    .iter()
                    .map(|v| value_frames(v.conversion))
                    .sum::<usize>()
            }
            GenericCopyExpressionKind::Array(items) => {
                self.a.fanout(items.len())?;
                1 + items
                    .iter()
                    .map(|v| value_frames(v.conversion))
                    .sum::<usize>()
            }
            GenericCopyExpressionKind::Record { fields, .. } => {
                self.a.fanout(fields.len())?;
                1 + fields
                    .iter()
                    .map(|v| value_frames(v.conversion))
                    .sum::<usize>()
            }
            GenericCopyExpressionKind::Enum { payload, .. } => {
                1 + payload.as_ref().map_or(0, |v| value_frames(v.conversion))
            }
            GenericCopyExpressionKind::Field { .. } => 2,
            GenericCopyExpressionKind::Index { .. } | GenericCopyExpressionKind::String { .. } => 3,
            GenericCopyExpressionKind::Logical { .. } => 8,
            GenericCopyExpressionKind::Numeric {
                left_conversion,
                right_conversion,
                ..
            }
            | GenericCopyExpressionKind::Equality {
                left_conversion,
                right_conversion,
                ..
            } => 1 + value_frames(*left_conversion) + value_frames(*right_conversion),
            GenericCopyExpressionKind::Literal(_) | GenericCopyExpressionKind::Read(_) => 1,
        };
        self.a.schedule(frames)?;
        match &source.kind {
            GenericCopyExpressionKind::Propagate {
                operand, question, ..
            } => {
                let p = self
                    .propagations
                    .remove(&id.ordinal())
                    .ok_or_else(|| self.a.invalid("missing propagation operation plan"))?;
                if p.instance != self.a.function
                    || p.expression != id
                    || p.operand != *operand
                    || p.question != *question
                    || p.success != layout
                {
                    return Err(self.a.invalid("propagation operation provenance"));
                }
                out.push(Work::Expr(*operand));
                out.push(Work::Emit(
                    GenericIrOperation::Propagate {
                        input: p.input,
                        success: p.success,
                        result: p.result,
                        success_tag: p.success_tag,
                        failure_tag: p.failure_tag,
                        failure: p.failure,
                    },
                    *question,
                ));
            }
            GenericCopyExpressionKind::Builtin { builtin, arguments } => {
                for id in arguments {
                    out.push(Work::Expr(*id));
                }
                out.push(Work::Emit(
                    GenericIrOperation::Builtin {
                        builtin: *builtin,
                        right_layout: if matches!(
                            builtin,
                            GenericBuiltin::ArrayAppend | GenericBuiltin::ArrayConcat
                        ) {
                            Some(if *builtin == GenericBuiltin::ArrayConcat {
                                layout
                            } else {
                                match self.plans.layouts().layout(layout).map(|l| l.kind()) {
                                    Some(GenericRuntimeLayoutKind::Array { element }) => *element,
                                    _ => return Err(self.a.invalid("append result array")),
                                }
                            })
                        } else {
                            None
                        },
                        layout,
                    },
                    span,
                ));
            }
            GenericCopyExpressionKind::Literal(_) => {
                let literal = self.plans.layouts().literals().bodies()[self.a.function]
                    .literal(id)
                    .ok_or_else(|| self.a.invalid("missing decoded literal"))?;
                out.push(Work::Emit(
                    GenericIrOperation::Constant {
                        value: literal.value.clone(),
                        layout,
                    },
                    span,
                ));
            }
            GenericCopyExpressionKind::Read(binding) => out.push(Work::Emit(
                GenericIrOperation::Load {
                    slot: self.slot(*binding)?,
                },
                span,
            )),
            GenericCopyExpressionKind::Call { arguments, .. } => {
                for argument in arguments {
                    self.value(argument.expression, argument.conversion, span, out)?;
                }
                out.push(Work::Emit(
                    GenericIrOperation::Call {
                        target: *self
                            .calls
                            .get(&id.ordinal())
                            .ok_or_else(|| self.a.invalid("concrete call target"))?,
                        arguments: arguments.len(),
                    },
                    span,
                ));
            }
            GenericCopyExpressionKind::Array(items) => {
                for item in items {
                    self.value(item.value, item.conversion, span, out)?;
                }
                out.push(Work::Emit(
                    GenericIrOperation::Array {
                        layout,
                        count: items.len(),
                    },
                    span,
                ));
            }
            GenericCopyExpressionKind::Index { target, index } => {
                out.push(Work::Expr(*target));
                out.push(Work::Expr(*index));
                out.push(Work::Emit(GenericIrOperation::Index { layout }, span));
            }
            GenericCopyExpressionKind::Record { .. }
            | GenericCopyExpressionKind::Enum { .. }
            | GenericCopyExpressionKind::Field { .. } => {
                let plan = self
                    .body
                    .expression(id)
                    .ok_or_else(|| self.a.invalid("nominal expression plan"))?;
                match &plan.kind {
                    GenericConstructionKind::Record { fields } => {
                        for field in fields {
                            self.value(field.value.expression, field.value.conversion, span, out)?;
                        }
                        out.push(Work::Emit(
                            GenericIrOperation::Record {
                                layout,
                                slots: fields.iter().map(|f| f.slot).collect(),
                            },
                            span,
                        ));
                    }
                    GenericConstructionKind::Enum { tag, payload } => {
                        if let Some(payload) = payload {
                            self.value(payload.expression, payload.conversion, span, out)?;
                        }
                        out.push(Work::Emit(
                            GenericIrOperation::Enum {
                                layout,
                                tag: *tag,
                                payload: payload.is_some(),
                            },
                            span,
                        ));
                    }
                    GenericConstructionKind::Field {
                        receiver,
                        receiver_layout,
                        slot,
                    } => {
                        out.push(Work::Expr(*receiver));
                        out.push(Work::Emit(
                            GenericIrOperation::Field {
                                receiver: *receiver_layout,
                                slot: *slot,
                                layout,
                            },
                            span,
                        ));
                    }
                }
            }
            GenericCopyExpressionKind::Logical {
                operator,
                left,
                right,
            } => {
                let rhs = self.label()?;
                let constant = self.label()?;
                let end = self.label()?;
                let and = *operator == GenericLogicalOperator::And;
                out.push(Work::Expr(*left));
                out.push(Work::Emit(
                    GenericIrOperation::Branch {
                        yes: if and { rhs } else { constant },
                        no: if and { constant } else { rhs },
                    },
                    span,
                ));
                out.push(Work::Mark(constant));
                out.push(Work::Emit(
                    GenericIrOperation::Constant {
                        value: GenericLiteralValue::Bool(!and),
                        layout,
                    },
                    span,
                ));
                out.push(Work::Emit(GenericIrOperation::Jump { target: end }, span));
                out.push(Work::Mark(rhs));
                out.push(Work::Expr(*right));
                out.push(Work::Mark(end));
            }
            GenericCopyExpressionKind::Numeric {
                operator,
                left,
                right,
                left_conversion,
                right_conversion,
            } => {
                self.value(*left, *left_conversion, span, out)?;
                self.value(*right, *right_conversion, span, out)?;
                out.push(Work::Emit(
                    GenericIrOperation::Binary {
                        operator: GenericIrBinary::Numeric(*operator),
                        layout,
                    },
                    span,
                ));
            }
            GenericCopyExpressionKind::Equality {
                left,
                right,
                negated,
                left_conversion,
                right_conversion,
            } => {
                self.value(*left, *left_conversion, span, out)?;
                self.value(*right, *right_conversion, span, out)?;
                out.push(Work::Emit(
                    GenericIrOperation::Binary {
                        operator: GenericIrBinary::Equality { negated: *negated },
                        layout,
                    },
                    span,
                ));
            }
            GenericCopyExpressionKind::String {
                operator,
                left,
                right,
            } => {
                out.push(Work::Expr(*left));
                out.push(Work::Expr(*right));
                out.push(Work::Emit(
                    GenericIrOperation::Binary {
                        operator: GenericIrBinary::String(*operator),
                        layout,
                    },
                    span,
                ));
            }
        }
        Ok(())
    }
    fn block(&mut self, range: Range<usize>, out: &mut Vec<Work>) -> Result<(), GenericIrError> {
        if range.is_empty() {
            return Ok(());
        }
        let statements = self.body.body().body().body().statements();
        let index = range.start;
        let source = statements
            .get(index)
            .ok_or_else(|| self.a.invalid("statement range"))?;
        let value_frames = |conversion: GenericCopyConversion| {
            1 + usize::from(conversion == GenericCopyConversion::IntToFloat)
        };
        let (source_next, frames) = match source {
            GenericCopyStatement::Let { conversion, .. }
            | GenericCopyStatement::Assign { conversion, .. } => {
                (index + 1, 1 + value_frames(*conversion))
            }
            GenericCopyStatement::IndexAssign { conversion, .. } => {
                (index + 1, 2 + value_frames(*conversion))
            }
            GenericCopyStatement::Return {
                value, conversion, ..
            } => (
                index + 1,
                if value.is_some() {
                    1 + value_frames(*conversion)
                } else {
                    2
                },
            ),
            GenericCopyStatement::Expression { .. } => (index + 1, 2),
            GenericCopyStatement::If {
                then_statements,
                else_statements,
                ..
            } => (
                else_statements
                    .as_ref()
                    .map_or(then_statements.end, |r| r.end),
                7 + usize::from(else_statements.is_some()),
            ),
            GenericCopyStatement::While {
                body_statements, ..
            } => (body_statements.end, 7),
            GenericCopyStatement::Match { arms, .. } => {
                self.a.fanout(arms.len())?;
                (
                    arms.last().map_or(index + 1, |a| a.statements.end),
                    arms.len()
                        .checked_mul(4)
                        .and_then(|n| n.checked_add(3))
                        .ok_or(GenericIrError::Limit("generic IR pending tasks"))?,
                )
            }
        };
        if source_next <= index || source_next > range.end {
            return Err(self.a.invalid("nested statement range"));
        }
        self.a
            .schedule(frames + usize::from(source_next < range.end))?;
        let mut next = index + 1;
        match source {
            GenericCopyStatement::Let {
                binding,
                value,
                conversion,
                span,
                ..
            } => {
                self.value(*value, *conversion, *span, out)?;
                out.push(Work::Emit(
                    GenericIrOperation::Store {
                        slot: self.slot(*binding)?,
                        initialize: true,
                    },
                    *span,
                ));
            }
            GenericCopyStatement::Assign {
                binding,
                value,
                conversion,
                span,
                ..
            } => {
                self.value(*value, *conversion, *span, out)?;
                out.push(Work::Emit(
                    GenericIrOperation::Store {
                        slot: self.slot(*binding)?,
                        initialize: false,
                    },
                    *span,
                ));
            }
            GenericCopyStatement::IndexAssign {
                binding,
                index,
                value,
                conversion,
                span,
                ..
            } => {
                out.push(Work::Expr(*index));
                self.value(*value, *conversion, *span, out)?;
                out.push(Work::Emit(
                    GenericIrOperation::IndexStore {
                        slot: self.slot(*binding)?,
                    },
                    *span,
                ));
            }
            GenericCopyStatement::Expression { value } => {
                out.push(Work::Expr(*value));
                let span = self
                    .body
                    .body()
                    .body()
                    .body()
                    .expression(*value)
                    .ok_or_else(|| self.a.invalid("discard expression"))?
                    .span;
                out.push(Work::Emit(GenericIrOperation::Drop, span));
            }
            GenericCopyStatement::Return {
                value,
                conversion,
                span,
            } => {
                if let Some(value) = value {
                    self.value(*value, *conversion, *span, out)?;
                } else {
                    out.push(Work::Emit(
                        GenericIrOperation::Unit {
                            layout: self.body.body().return_layout(),
                        },
                        *span,
                    ));
                }
                out.push(Work::Emit(GenericIrOperation::Return, *span));
            }
            GenericCopyStatement::If {
                condition,
                then_statements,
                else_statements,
                span,
            } => {
                let yes = self.label()?;
                let no = self.label()?;
                let end = self.label()?;
                next = else_statements
                    .as_ref()
                    .map_or(then_statements.end, |r| r.end);
                out.push(Work::Expr(*condition));
                out.push(Work::Emit(GenericIrOperation::Branch { yes, no }, *span));
                out.push(Work::Mark(yes));
                out.push(Work::Block(then_statements.clone()));
                out.push(Work::Emit(GenericIrOperation::Jump { target: end }, *span));
                out.push(Work::Mark(no));
                if let Some(otherwise) = else_statements {
                    out.push(Work::Block(otherwise.clone()));
                }
                out.push(Work::Mark(end));
            }
            GenericCopyStatement::While {
                condition,
                body_statements,
                span,
            } => {
                let test = self.label()?;
                let run = self.label()?;
                let end = self.label()?;
                next = body_statements.end;
                out.push(Work::Mark(test));
                out.push(Work::Expr(*condition));
                out.push(Work::Emit(
                    GenericIrOperation::Branch { yes: run, no: end },
                    *span,
                ));
                out.push(Work::Mark(run));
                out.push(Work::Block(body_statements.clone()));
                out.push(Work::Emit(GenericIrOperation::Jump { target: test }, *span));
                out.push(Work::Mark(end));
            }
            GenericCopyStatement::Match { span, .. } => {
                let plan = self
                    .body
                    .match_statement(index)
                    .ok_or_else(|| self.a.invalid("match dispatch plan"))?;
                let end = self.label()?;
                let mut labels = Vec::new();
                for _ in &plan.arms {
                    labels.push(self.label()?);
                }
                out.push(Work::Expr(plan.scrutinee));
                out.push(Work::Emit(
                    GenericIrOperation::Switch {
                        layout: plan.layout,
                        targets: plan.dispatch.iter().map(|a| labels[*a]).collect(),
                    },
                    *span,
                ));
                for (arm, label) in plan.arms.iter().zip(labels) {
                    out.push(Work::Mark(label));
                    out.push(Work::Emit(
                        GenericIrOperation::MatchBind {
                            layout: plan.layout,
                            tag: arm.tag,
                            binding: arm.payload.map(|p| p.binding.ordinal()),
                        },
                        arm.span,
                    ));
                    out.push(Work::Block(arm.statements.clone()));
                    out.push(Work::Emit(
                        GenericIrOperation::Jump { target: end },
                        arm.span,
                    ));
                    next = arm.statements.end;
                }
                out.push(Work::Mark(end));
            }
        }
        if next <= index || next > range.end {
            return Err(self.a.invalid("nested statement range"));
        }
        if next < range.end {
            out.push(Work::Block(next..range.end));
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Vec<GenericIrInstruction>, GenericIrError> {
        let source = self.body.body().body().body();
        let span = source.signature().syntax().body_span();
        self.a.pending_base = 0;
        self.a
            .schedule(if source.implicit_unit_return() { 4 } else { 2 })?;
        let mut pending = vec![Work::Emit(GenericIrOperation::Unreachable, span)];
        if source.implicit_unit_return() {
            pending.push(Work::Emit(GenericIrOperation::Return, span));
            pending.push(Work::Emit(
                GenericIrOperation::Unit {
                    layout: self.body.body().return_layout(),
                },
                span,
            ));
        }
        pending.push(Work::Block(0..source.statements().len()));
        while let Some(work) = pending.pop() {
            self.a.pending_base = pending.len();
            self.a.step()?;
            let mut sequence = Vec::new();
            match work {
                Work::Emit(operation, span) => self.emit(operation, span)?,
                Work::Mark(label) => {
                    let target = self
                        .labels
                        .get_mut(label)
                        .ok_or_else(|| self.a.invalid("unknown label"))?;
                    if target.replace(self.code.len()).is_some() {
                        return Err(self.a.invalid("duplicate label"));
                    }
                }
                Work::Expr(id) => self.expression(id, &mut sequence)?,
                Work::Block(range) => self.block(range, &mut sequence)?,
            }
            pending.extend(sequence.into_iter().rev());
        }
        let code_len = self.code.len();
        for instruction in &mut self.code {
            self.a.span = instruction.span;
            self.a.step()?;
            let resolve = |label: &mut usize, a: &mut Admission| -> Result<(), GenericIrError> {
                a.step()?;
                *label = self
                    .labels
                    .get(*label)
                    .and_then(|p| *p)
                    .filter(|p| *p < code_len)
                    .ok_or_else(|| a.invalid("unresolved instruction label"))?;
                Ok(())
            };
            match &mut instruction.operation {
                GenericIrOperation::Jump { target } => resolve(target, self.a)?,
                GenericIrOperation::Branch { yes, no } => {
                    resolve(yes, self.a)?;
                    resolve(no, self.a)?;
                }
                GenericIrOperation::Switch { targets, .. } => {
                    for target in targets {
                        resolve(target, self.a)?;
                    }
                }
                _ => {}
            }
        }
        if !self.propagations.is_empty() {
            return Err(self.a.invalid("unemitted propagation operations"));
        }
        Ok(self.code)
    }
}
#[derive(Clone, Copy)]
struct StackNode {
    layout: usize,
    tail: usize,
    depth: usize,
}
struct Stacks {
    nodes: Vec<StackNode>,
    index: BTreeMap<(usize, usize), usize>,
    depth_limit: usize,
}
impl Stacks {
    fn depth(&self, id: usize) -> usize {
        if id == 0 {
            0
        } else {
            self.nodes[id - 1].depth
        }
    }
    fn push(
        &mut self,
        tail: usize,
        layout: usize,
        a: &mut Admission,
    ) -> Result<usize, GenericIrError> {
        a.step()?;
        let depth = self
            .depth(tail)
            .checked_add(1)
            .ok_or_else(|| a.invalid("stack depth overflow"))?;
        if depth > self.depth_limit {
            return Err(GenericIrError::Limit("generic IR stack depth"));
        }
        if let Some(id) = self.index.get(&(tail, layout)) {
            return Ok(*id);
        }
        charge(&mut a.left.max_stack_nodes, 1, "generic IR stack nodes")?;
        let id = self.nodes.len() + 1;
        self.nodes.push(StackNode {
            layout,
            tail,
            depth,
        });
        self.index.insert((tail, layout), id);
        Ok(id)
    }
    fn pop(&self, id: &mut usize, a: &mut Admission) -> Result<usize, GenericIrError> {
        a.step()?;
        let node = id
            .checked_sub(1)
            .and_then(|i| self.nodes.get(i))
            .ok_or_else(|| a.invalid("operand stack underflow"))?;
        *id = node.tail;
        Ok(node.layout)
    }
    fn expect(
        &self,
        id: &mut usize,
        expected: usize,
        a: &mut Admission,
    ) -> Result<(), GenericIrError> {
        if self.pop(id, a)? != expected {
            return Err(a.invalid("operand layout mismatch"));
        }
        Ok(())
    }
}
fn shape<'a>(
    layouts: &'a CheckedGenericLayouts,
    id: usize,
    a: &Admission,
) -> Result<&'a GenericRuntimeLayoutKind, GenericIrError> {
    layouts
        .layout(id)
        .map(|l| l.kind())
        .ok_or_else(|| a.invalid("unknown layout index"))
}
fn map_entry_arguments(
    layouts: &CheckedGenericLayouts,
    layout: usize,
    a: &mut Admission,
) -> Result<(usize, usize), GenericIrError> {
    let GenericRuntimeLayoutKind::Record {
        declaration,
        arguments,
        fields,
    } = shape(layouts, layout, a)?
    else {
        return Err(a.invalid("standard MapEntry record layout"));
    };
    let types = layouts
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let Some(CheckedGenericDefinition::Record(def)) = types.definition(*declaration) else {
        return Err(a.invalid("standard MapEntry definition"));
    };
    if !def.syntax().is_standard_map_entry()
        || arguments.len() != 2
        || fields.as_slice() != arguments.as_slice()
    {
        return Err(a.invalid("standard MapEntry shape"));
    }
    Ok((arguments[0], arguments[1]))
}
fn json_shape(
    layouts: &CheckedGenericLayouts,
    layout: usize,
    a: &mut Admission,
) -> Result<(), GenericIrError> {
    let GenericRuntimeLayoutKind::Enum {
        declaration,
        arguments,
        variants,
    } = shape(layouts, layout, a)?
    else {
        return Err(a.invalid("JSON layout"));
    };
    let types = layouts
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    if !arguments.is_empty()
        || variants.len() != 7
        || !matches!(types.definition(*declaration),Some(CheckedGenericDefinition::Enum(d))if d.syntax().standard_identity()==Some(GenericStandardEnum::JsonValue))
    {
        return Err(a.invalid("JSON schema identity"));
    }
    if variants[0].is_some() {
        return Err(a.invalid("JSON null schema"));
    }
    for (tag, kind) in [
        (1, GenericPrimitive::Bool),
        (2, GenericPrimitive::Int),
        (3, GenericPrimitive::Float),
        (4, GenericPrimitive::String),
    ] {
        is_primitive(
            layouts,
            variants[tag].ok_or_else(|| a.invalid("JSON scalar payload"))?,
            kind,
            a,
        )?;
    }
    let array = variants[5].ok_or_else(|| a.invalid("JSON array payload"))?;
    if !matches!(shape(layouts,array,a)?,GenericRuntimeLayoutKind::Array{element}if *element==layout)
    {
        return Err(a.invalid("JSON recursive array"));
    }
    let map = variants[6].ok_or_else(|| a.invalid("JSON object payload"))?;
    let GenericRuntimeLayoutKind::Map { key, value, .. } = shape(layouts, map, a)? else {
        return Err(a.invalid("JSON object map"));
    };
    if *value != layout {
        return Err(a.invalid("JSON recursive object"));
    }
    is_primitive(layouts, *key, GenericPrimitive::String, a)
}
fn byte_result_layout(
    layouts: &CheckedGenericLayouts,
    layout: usize,
    a: &mut Admission,
) -> Result<(usize, usize), GenericIrError> {
    data_result_layout(layouts, layout, a, DataResultKind::Byte)
}
fn data_result_layout(
    layouts: &CheckedGenericLayouts,
    layout: usize,
    a: &mut Admission,
    expected: DataResultKind,
) -> Result<(usize, usize), GenericIrError> {
    let GenericRuntimeLayoutKind::Enum {
        declaration,
        arguments,
        variants,
    } = shape(layouts, layout, a)?
    else {
        return Err(a.invalid("byte Result layout"));
    };
    let types = layouts
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    if !matches!(types.definition(*declaration),Some(CheckedGenericDefinition::Enum(e)) if e.syntax().standard_identity()==Some(GenericStandardEnum::Result))
        || arguments.len() != 2
        || variants.as_slice() != [Some(arguments[0]), Some(arguments[1])]
    {
        return Err(a.invalid("byte Result schema"));
    }
    let (byte, error) = (arguments[0], arguments[1]);
    let success = shape(layouts, byte, a)?;
    let correct = match (expected, success) {
        (DataResultKind::JsonValue, _) => {
            json_shape(layouts, byte, a)?;
            true
        }
        (DataResultKind::String, GenericRuntimeLayoutKind::Primitive(GenericPrimitive::String)) => {
            true
        }
        (DataResultKind::Byte, GenericRuntimeLayoutKind::Byte { .. })
        | (DataResultKind::Bytes, GenericRuntimeLayoutKind::Bytes { .. }) => true,
        (DataResultKind::ByteArray, GenericRuntimeLayoutKind::Array { element }) => matches!(
            shape(layouts, *element, a)?,
            GenericRuntimeLayoutKind::Byte { .. }
        ),
        _ => false,
    };
    if !correct {
        return Err(a.invalid("data Result success"));
    }
    let GenericRuntimeLayoutKind::Record {
        declaration,
        fields,
        ..
    } = shape(layouts, error, a)?
    else {
        return Err(a.invalid("byte Error layout"));
    };
    if !matches!(types.definition(*declaration),Some(CheckedGenericDefinition::Record(r)) if r.syntax().is_standard_error())
        || fields.len() != 3
    {
        return Err(a.invalid("byte Error schema"));
    }
    is_primitive(layouts, fields[0], GenericPrimitive::String, a)?;
    is_primitive(layouts, fields[1], GenericPrimitive::String, a)?;
    let offset = option_payload(layouts, fields[2], a)?;
    is_primitive(layouts, offset, GenericPrimitive::Int, a)?;
    Ok((byte, error))
}
fn option_payload(
    layouts: &CheckedGenericLayouts,
    layout: usize,
    a: &mut Admission,
) -> Result<usize, GenericIrError> {
    let GenericRuntimeLayoutKind::Enum {
        declaration,
        arguments,
        variants,
    } = shape(layouts, layout, a)?
    else {
        return Err(a.invalid("lookup Option layout"));
    };
    let types = layouts
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let Some(CheckedGenericDefinition::Enum(def)) = types.definition(*declaration) else {
        return Err(a.invalid("lookup Option definition"));
    };
    if def.syntax().standard_identity() != Some(GenericStandardEnum::Option)
        || arguments.len() != 1
        || variants.as_slice() != [Some(arguments[0]), None]
    {
        return Err(a.invalid("lookup standard Option shape"));
    }
    Ok(arguments[0])
}
fn is_primitive(
    layouts: &CheckedGenericLayouts,
    id: usize,
    primitive: GenericPrimitive,
    a: &Admission,
) -> Result<(), GenericIrError> {
    if shape(layouts, id, a)? != &GenericRuntimeLayoutKind::Primitive(primitive) {
        return Err(a.invalid("primitive layout mismatch"));
    }
    Ok(())
}
fn array_compatible_layout(
    layouts: &CheckedGenericLayouts,
    mut actual: usize,
    mut target: usize,
    a: &mut Admission,
) -> Result<(), GenericIrError> {
    loop {
        a.step()?;
        if actual == target {
            return Ok(());
        }
        match (shape(layouts, actual, a)?, shape(layouts, target, a)?) {
            (
                GenericRuntimeLayoutKind::Array { element: x },
                GenericRuntimeLayoutKind::Array { element: y },
            ) => {
                actual = *x;
                target = *y;
            }
            (
                GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int),
                GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Float),
            ) => return Ok(()),
            _ => return Err(a.invalid("array operand conversion")),
        }
    }
}
fn structural(
    function: &GenericIrFunction,
    all: &[GenericIrFunction],
    layouts: &CheckedGenericLayouts,
    a: &mut Admission,
) -> Result<(), GenericIrError> {
    let code = &function.code;
    if let Some(first) = code.first() {
        a.span = first.span;
    }
    if code.is_empty() || function.slot_kinds.len() != function.slot_layouts.len() {
        return Err(a.invalid("function table shape"));
    }
    shape(layouts, function.result, a)?;
    for (slot, layout) in function.slot_layouts.iter().enumerate() {
        a.step()?;
        shape(layouts, *layout, a)?;
        if let GenericCopyBindingKind::Parameter(index) = function.slot_kinds[slot] {
            if function.parameter_slots.get(index) != Some(&slot) {
                return Err(a.invalid("parameter slot mapping"));
            }
        }
    }
    for (index, slot) in function.parameter_slots.iter().enumerate() {
        a.step()?;
        if function.slot_kinds.get(*slot) != Some(&GenericCopyBindingKind::Parameter(index)) {
            return Err(a.invalid("parameter slot authority"));
        }
    }
    for instruction in code.iter() {
        a.span = instruction.span;
        a.step()?;
        let slot = |index: usize| {
            function
                .slot_layouts
                .get(index)
                .copied()
                .ok_or_else(|| a.invalid("local slot index"))
        };
        let address = |target: usize| {
            if target < code.len() {
                Ok(())
            } else {
                Err(a.invalid("instruction address"))
            }
        };
        match &instruction.operation {
            GenericIrOperation::Builtin {
                builtin,
                layout,
                right_layout,
            } => {
                if matches!(
                    builtin,
                    GenericBuiltin::JsonParse
                        | GenericBuiltin::JsonStringify
                        | GenericBuiltin::HostCall
                ) {
                    if right_layout.is_some() {
                        return Err(a.invalid("JSON target"));
                    }
                    data_result_layout(
                        layouts,
                        *layout,
                        a,
                        if *builtin == GenericBuiltin::JsonStringify {
                            DataResultKind::String
                        } else {
                            DataResultKind::JsonValue
                        },
                    )?;
                    continue;
                }
                if matches!(
                    builtin,
                    GenericBuiltin::BytesToHex
                        | GenericBuiltin::BytesToBase64
                        | GenericBuiltin::BytesToUtf8
                ) {
                    if right_layout.is_some() {
                        return Err(a.invalid("bytes_to_hex target"));
                    }
                    data_result_layout(layouts, *layout, a, DataResultKind::String)?;
                    continue;
                }
                if *builtin == GenericBuiltin::BytesToArray {
                    if right_layout.is_some() {
                        return Err(a.invalid("bytes_to_array target"));
                    }
                    data_result_layout(layouts, *layout, a, DataResultKind::ByteArray)?;
                    continue;
                }
                if *builtin == GenericBuiltin::BytesGet {
                    if right_layout.is_some() {
                        return Err(a.invalid("bytes_get target"));
                    }
                    byte_result_layout(layouts, *layout, a)?;
                    continue;
                }
                if matches!(
                    builtin,
                    GenericBuiltin::BytesConcat
                        | GenericBuiltin::BytesSlice
                        | GenericBuiltin::BytesFromArray
                        | GenericBuiltin::BytesFromText
                        | GenericBuiltin::BytesFromHex
                        | GenericBuiltin::BytesFromBase64
                ) {
                    if right_layout.is_some() {
                        return Err(a.invalid("bytes_concat target"));
                    }
                    data_result_layout(layouts, *layout, a, DataResultKind::Bytes)?;
                    continue;
                }
                if *builtin == GenericBuiltin::ByteFromInt {
                    if right_layout.is_some() {
                        return Err(a.invalid("byte conversion target"));
                    }
                    byte_result_layout(layouts, *layout, a)?;
                    continue;
                }
                if builtin.collection_new() || builtin.collection_insert() {
                    let correct = matches!(
                        (builtin.collection_kind(), shape(layouts, *layout, a)?),
                        (
                            Some(super::GenericCollectionLayoutKind::Map),
                            GenericRuntimeLayoutKind::Map { .. }
                        ) | (
                            Some(super::GenericCollectionLayoutKind::Set),
                            GenericRuntimeLayoutKind::Set { .. }
                        )
                    );
                    if !correct || right_layout.is_some() {
                        return Err(a.invalid("collection_new result layout"));
                    }
                    continue;
                }
                if builtin.collection_iteration() {
                    if right_layout.is_some()
                        || !matches!(
                            shape(layouts, *layout, a)?,
                            GenericRuntimeLayoutKind::Array { .. }
                        )
                    {
                        return Err(a.invalid("collection iteration result array"));
                    }
                    if *builtin == GenericBuiltin::MapEntries {
                        let GenericRuntimeLayoutKind::Array { element } =
                            shape(layouts, *layout, a)?
                        else {
                            unreachable!()
                        };
                        map_entry_arguments(layouts, *element, a)?;
                    }
                    continue;
                }
                if builtin.collection_get() || builtin.collection_remove() {
                    if right_layout.is_some() {
                        return Err(a.invalid("lookup conversion target"));
                    }
                    option_payload(layouts, *layout, a)?;
                    continue;
                }
                if builtin.array_operation() {
                    let GenericRuntimeLayoutKind::Array { element } = shape(layouts, *layout, a)?
                    else {
                        return Err(a.invalid("array operation result"));
                    };
                    let expected = match builtin {
                        GenericBuiltin::ArrayAppend => Some(*element),
                        GenericBuiltin::ArrayConcat => Some(*layout),
                        _ => None,
                    };
                    if *right_layout != expected {
                        return Err(a.invalid("array operation conversion target"));
                    }
                    continue;
                }
                if right_layout.is_some() {
                    return Err(a.invalid("unexpected builtin conversion target"));
                }
                if let Some(kind) = builtin.result_primitive() {
                    is_primitive(layouts, *layout, kind, a)?;
                    continue;
                }
                let GenericRuntimeLayoutKind::Record {
                    declaration,
                    fields,
                    arguments,
                } = shape(layouts, *layout, a)?
                else {
                    return Err(a.invalid("builtin result layout"));
                };
                let definitions = layouts
                    .literals()
                    .concrete()
                    .graph()
                    .bodies()
                    .signatures()
                    .types();
                if !arguments.is_empty()
                    || !matches!(definitions.definition(*declaration),Some(CheckedGenericDefinition::Record(record)) if record.syntax().is_standard_result() && record.syntax().qualified_name()==builtin.result_name())
                {
                    return Err(a.invalid("builtin standard result identity"));
                }
                let kinds: &[GenericPrimitive] = match builtin {
                    GenericBuiltin::ParseInt => &[GenericPrimitive::Bool, GenericPrimitive::Int],
                    GenericBuiltin::SplitOnce => &[
                        GenericPrimitive::Bool,
                        GenericPrimitive::String,
                        GenericPrimitive::String,
                    ],
                    _ => return Err(a.invalid("primitive builtin reached record validation")),
                };
                if fields.len() != kinds.len() {
                    return Err(a.invalid("builtin result fields"));
                }
                for (field, kind) in fields.iter().zip(kinds) {
                    a.step()?;
                    is_primitive(layouts, *field, *kind, a)?;
                }
            }
            GenericIrOperation::Propagate {
                input,
                success,
                result,
                success_tag,
                failure_tag,
                failure,
            } => {
                a.step()?;
                if *result != function.result || success_tag == failure_tag {
                    return Err(a.invalid("propagation caller return/tag"));
                }
                let GenericRuntimeLayoutKind::Enum {
                    declaration,
                    arguments,
                    variants,
                } = shape(layouts, *input, a)?
                else {
                    return Err(a.invalid("propagation input wrapper"));
                };
                let GenericRuntimeLayoutKind::Enum {
                    declaration: ret_decl,
                    arguments: ret_args,
                    variants: ret_tags,
                } = shape(layouts, *result, a)?
                else {
                    return Err(a.invalid("propagation return wrapper"));
                };
                let types = layouts
                    .literals()
                    .concrete()
                    .graph()
                    .bodies()
                    .signatures()
                    .types();
                let Some(CheckedGenericDefinition::Enum(def)) = types.definition(*declaration)
                else {
                    return Err(a.invalid("propagation standard declaration"));
                };
                let arity = match def.syntax().standard_identity() {
                    Some(GenericStandardEnum::Option) if failure.is_none() => 1,
                    Some(GenericStandardEnum::Result) if failure.is_some() => 2,
                    _ => return Err(a.invalid("propagation standard identity")),
                };
                if declaration != ret_decl
                    || arguments.len() != arity
                    || ret_args.len() != arity
                    || variants.len() != 2
                    || ret_tags.len() != 2
                    || *success_tag != 0
                    || *failure_tag != 1
                    || arguments.first() != Some(success)
                    || variants.get(*success_tag) != Some(&Some(*success))
                    || ret_tags.get(*success_tag) != Some(&Some(ret_args[0]))
                    || variants.get(*failure_tag) != Some(failure)
                    || ret_tags.get(*failure_tag) != Some(failure)
                    || (arity == 2
                        && (arguments.get(1) != failure.as_ref()
                            || ret_args.get(1) != failure.as_ref()))
                {
                    return Err(a.invalid("propagation wrapper payload contract"));
                }
                shape(layouts, *success, a)?;
            }
            GenericIrOperation::Constant { value, layout } => {
                is_primitive(layouts, *layout, value.primitive(), a)?;
                if matches!(value, GenericLiteralValue::FloatBits(bits) if !f64::from_bits(*bits).is_finite())
                {
                    return Err(a.invalid("nonfinite constant payload"));
                }
            }
            GenericIrOperation::Unit { layout } => {
                is_primitive(layouts, *layout, GenericPrimitive::Unit, a)?
            }
            GenericIrOperation::Load { slot: index } => {
                slot(*index)?;
            }
            GenericIrOperation::Store {
                slot: index,
                initialize,
            } => {
                slot(*index)?;
                let kind = function.slot_kinds[*index];
                if !matches!(
                    (initialize, kind),
                    (
                        true,
                        GenericCopyBindingKind::Local | GenericCopyBindingKind::MutableLocal
                    ) | (false, GenericCopyBindingKind::MutableLocal)
                ) {
                    return Err(a.invalid("store binding authority"));
                }
            }
            GenericIrOperation::IndexStore { slot: index } => {
                let ty = slot(*index)?;
                if function.slot_kinds[*index] != GenericCopyBindingKind::MutableLocal
                    || !matches!(
                        shape(layouts, ty, a)?,
                        GenericRuntimeLayoutKind::Array { .. }
                    )
                {
                    return Err(a.invalid("indexed store authority"));
                }
            }
            GenericIrOperation::IntToFloat { layout } => {
                is_primitive(layouts, *layout, GenericPrimitive::Float, a)?
            }
            GenericIrOperation::Binary { layout, .. } | GenericIrOperation::Index { layout } => {
                shape(layouts, *layout, a)?;
            }
            GenericIrOperation::Array { layout, .. } => {
                if !matches!(
                    shape(layouts, *layout, a)?,
                    GenericRuntimeLayoutKind::Array { .. }
                ) {
                    return Err(a.invalid("array layout"));
                }
            }
            GenericIrOperation::Record { layout, slots } => {
                let GenericRuntimeLayoutKind::Record { fields, .. } = shape(layouts, *layout, a)?
                else {
                    return Err(a.invalid("record layout"));
                };
                if fields.len() != slots.len() {
                    return Err(a.invalid("record slot coverage"));
                }
                let mut seen = vec![false; fields.len()];
                for slot in slots {
                    a.step()?;
                    let visited = seen
                        .get_mut(*slot)
                        .ok_or_else(|| a.invalid("record slot"))?;
                    if *visited {
                        return Err(a.invalid("duplicate record slot"));
                    }
                    *visited = true;
                }
            }
            GenericIrOperation::Enum {
                layout,
                tag,
                payload,
            } => {
                let GenericRuntimeLayoutKind::Enum { variants, .. } = shape(layouts, *layout, a)?
                else {
                    return Err(a.invalid("enum layout"));
                };
                if variants.get(*tag).map(Option::is_some) != Some(*payload) {
                    return Err(a.invalid("enum tag/payload"));
                }
            }
            GenericIrOperation::Field {
                receiver,
                slot,
                layout,
            } => {
                let GenericRuntimeLayoutKind::Record { fields, .. } = shape(layouts, *receiver, a)?
                else {
                    return Err(a.invalid("field receiver"));
                };
                if fields.get(*slot) != Some(layout) {
                    return Err(a.invalid("field slot/result"));
                }
            }
            GenericIrOperation::Call { target, arguments } => {
                if all.get(*target).map(|f| f.parameter_slots.len()) != Some(*arguments) {
                    return Err(a.invalid("call target/arity"));
                }
            }
            GenericIrOperation::Jump { target } => address(*target)?,
            GenericIrOperation::Branch { yes, no } => {
                address(*yes)?;
                address(*no)?;
            }
            GenericIrOperation::Switch { layout, targets } => {
                let GenericRuntimeLayoutKind::Enum { variants, .. } = shape(layouts, *layout, a)?
                else {
                    return Err(a.invalid("switch layout"));
                };
                if variants.len() != targets.len() {
                    return Err(a.invalid("switch coverage"));
                }
                for (tag, target) in targets.iter().enumerate() {
                    a.step()?;
                    if !matches!(code.get(*target).map(|i|&i.operation),Some(GenericIrOperation::MatchBind{layout:actual,tag:actual_tag,..})if actual==layout && *actual_tag==tag)
                    {
                        return Err(a.invalid("switch arm target"));
                    }
                }
            }
            GenericIrOperation::MatchBind {
                layout,
                tag,
                binding,
            } => {
                let GenericRuntimeLayoutKind::Enum { variants, .. } = shape(layouts, *layout, a)?
                else {
                    return Err(a.invalid("match binding layout"));
                };
                let expected = variants.get(*tag).ok_or_else(|| a.invalid("match tag"))?;
                match (expected, binding) {
                    (None, None) => {}
                    (Some(expected), Some(index))
                        if slot(*index)? == *expected
                            && matches!(function.slot_kinds[*index],GenericCopyBindingKind::MatchPayload{variant_index,..}if variant_index==*tag) =>
                        {}
                    _ => return Err(a.invalid("match payload destination")),
                }
            }
            GenericIrOperation::Drop
            | GenericIrOperation::Return
            | GenericIrOperation::Unreachable => {}
        }
    }
    Ok(())
}
fn validate(
    function: &GenericIrFunction,
    all: &[GenericIrFunction],
    layouts: &CheckedGenericLayouts,
    stacks: &mut Stacks,
    a: &mut Admission,
) -> Result<usize, GenericIrError> {
    let mut states = vec![None; function.code.len()];
    let mut pending = vec![0usize];
    states[0] = Some(0usize);
    let mut maximum = 0;
    while let Some(pc) = pending.pop() {
        let instruction = &function.code[pc];
        a.span = instruction.span;
        a.step()?;
        let mut state = states[pc].expect("queued typed state");
        maximum = maximum.max(stacks.depth(state));
        let mut destinations = Vec::new();
        let mut fallthrough = true;
        match &instruction.operation {
            GenericIrOperation::Builtin {
                builtin,
                layout,
                right_layout,
            } => {
                if matches!(
                    builtin,
                    GenericBuiltin::JsonParse
                        | GenericBuiltin::JsonStringify
                        | GenericBuiltin::HostCall
                ) {
                    let input = stacks.pop(&mut state, a)?;
                    if *builtin == GenericBuiltin::JsonParse {
                        is_primitive(layouts, input, GenericPrimitive::String, a)?;
                        data_result_layout(layouts, *layout, a, DataResultKind::JsonValue)?;
                    } else {
                        let GenericRuntimeLayoutKind::Enum { declaration, .. } =
                            shape(layouts, input, a)?
                        else {
                            return Err(a.invalid("JSON receiver"));
                        };
                        let types = layouts
                            .literals()
                            .concrete()
                            .graph()
                            .bodies()
                            .signatures()
                            .types();
                        if !matches!(types.definition(*declaration),Some(CheckedGenericDefinition::Enum(d)) if d.syntax().standard_identity()==Some(GenericStandardEnum::JsonValue))
                        {
                            return Err(a.invalid("JSON receiver identity"));
                        }
                        json_shape(layouts, input, a)?;
                        data_result_layout(
                            layouts,
                            *layout,
                            a,
                            if *builtin == GenericBuiltin::HostCall {
                                DataResultKind::JsonValue
                            } else {
                                DataResultKind::String
                            },
                        )?;
                    }
                    state = stacks.push(state, *layout, a)?;
                } else if matches!(
                    builtin,
                    GenericBuiltin::BytesFromArray
                        | GenericBuiltin::BytesFromText
                        | GenericBuiltin::BytesLen
                        | GenericBuiltin::BytesGet
                        | GenericBuiltin::BytesConcat
                        | GenericBuiltin::BytesSlice
                        | GenericBuiltin::BytesToArray
                        | GenericBuiltin::BytesToHex
                        | GenericBuiltin::BytesFromHex
                        | GenericBuiltin::BytesToBase64
                        | GenericBuiltin::BytesFromBase64
                        | GenericBuiltin::BytesToUtf8
                ) {
                    let end = if *builtin == GenericBuiltin::BytesSlice {
                        Some(stacks.pop(&mut state, a)?)
                    } else {
                        None
                    };
                    let right = if matches!(
                        builtin,
                        GenericBuiltin::BytesGet
                            | GenericBuiltin::BytesConcat
                            | GenericBuiltin::BytesSlice
                    ) {
                        Some(stacks.pop(&mut state, a)?)
                    } else {
                        None
                    };
                    let input = stacks.pop(&mut state, a)?;
                    match builtin {
                        GenericBuiltin::BytesToHex
                        | GenericBuiltin::BytesToBase64
                        | GenericBuiltin::BytesToUtf8 => {
                            if !matches!(
                                shape(layouts, input, a)?,
                                GenericRuntimeLayoutKind::Bytes { .. }
                            ) {
                                return Err(a.invalid("bytes_to_hex receiver"));
                            }
                            data_result_layout(layouts, *layout, a, DataResultKind::String)?;
                        }
                        GenericBuiltin::BytesToArray => {
                            let GenericRuntimeLayoutKind::Bytes { byte, .. } =
                                shape(layouts, input, a)?
                            else {
                                return Err(a.invalid("bytes_to_array receiver"));
                            };
                            let array =
                                data_result_layout(layouts, *layout, a, DataResultKind::ByteArray)?
                                    .0;
                            let GenericRuntimeLayoutKind::Array { element } =
                                shape(layouts, array, a)?
                            else {
                                return Err(a.invalid("bytes_to_array array"));
                            };
                            if element != byte {
                                return Err(a.invalid("bytes_to_array Byte identity"));
                            }
                        }
                        GenericBuiltin::BytesGet => {
                            is_primitive(
                                layouts,
                                right.expect("binary operand"),
                                GenericPrimitive::Int,
                                a,
                            )?;
                            let GenericRuntimeLayoutKind::Bytes { byte, .. } =
                                shape(layouts, input, a)?
                            else {
                                return Err(a.invalid("bytes_get receiver"));
                            };
                            if byte_result_layout(layouts, *layout, a)?.0 != *byte {
                                return Err(a.invalid("bytes_get payload identity"));
                            }
                        }
                        GenericBuiltin::BytesSlice => {
                            is_primitive(
                                layouts,
                                right.expect("slice start"),
                                GenericPrimitive::Int,
                                a,
                            )?;
                            is_primitive(
                                layouts,
                                end.expect("slice end"),
                                GenericPrimitive::Int,
                                a,
                            )?;
                            if !matches!(
                                shape(layouts, input, a)?,
                                GenericRuntimeLayoutKind::Bytes { .. }
                            ) || data_result_layout(layouts, *layout, a, DataResultKind::Bytes)?
                                .0
                                != input
                            {
                                return Err(a.invalid("bytes_slice layouts"));
                            }
                        }
                        GenericBuiltin::BytesConcat => {
                            if !matches!(
                                shape(layouts, input, a)?,
                                GenericRuntimeLayoutKind::Bytes { .. }
                            ) || data_result_layout(layouts, *layout, a, DataResultKind::Bytes)?
                                .0
                                != input
                                || right != Some(input)
                            {
                                return Err(a.invalid("bytes_concat layouts"));
                            }
                        }
                        GenericBuiltin::BytesLen => {
                            if !matches!(
                                shape(layouts, input, a)?,
                                GenericRuntimeLayoutKind::Bytes { .. }
                            ) {
                                return Err(a.invalid("bytes_len operand"));
                            }
                            is_primitive(layouts, *layout, GenericPrimitive::Int, a)?;
                        }
                        GenericBuiltin::BytesFromText
                        | GenericBuiltin::BytesFromHex
                        | GenericBuiltin::BytesFromBase64 => {
                            is_primitive(layouts, input, GenericPrimitive::String, a)?;
                            data_result_layout(layouts, *layout, a, DataResultKind::Bytes)?;
                        }
                        _ => {
                            let GenericRuntimeLayoutKind::Array { element } =
                                shape(layouts, input, a)?
                            else {
                                return Err(a.invalid("bytes_from_array operand"));
                            };
                            let buffer =
                                data_result_layout(layouts, *layout, a, DataResultKind::Bytes)?.0;
                            let GenericRuntimeLayoutKind::Bytes { byte, .. } =
                                shape(layouts, buffer, a)?
                            else {
                                return Err(a.invalid("bytes_from_array result"));
                            };
                            if element != byte {
                                return Err(a.invalid("bytes_from_array Byte identity"));
                            }
                        }
                    }
                    state = stacks.push(state, *layout, a)?;
                } else if *builtin == GenericBuiltin::ByteFromInt {
                    let input = stacks.pop(&mut state, a)?;
                    is_primitive(layouts, input, GenericPrimitive::Int, a)?;
                    byte_result_layout(layouts, *layout, a)?;
                    state = stacks.push(state, *layout, a)?;
                } else if *builtin == GenericBuiltin::ByteToInt {
                    let input = stacks.pop(&mut state, a)?;
                    if !matches!(
                        shape(layouts, input, a)?,
                        GenericRuntimeLayoutKind::Byte { .. }
                    ) {
                        return Err(a.invalid("byte_to_int operand"));
                    }
                    is_primitive(layouts, *layout, GenericPrimitive::Int, a)?;
                    state = stacks.push(state, *layout, a)?;
                } else if builtin.collection_iteration() {
                    let receiver = stacks.pop(&mut state, a)?;
                    let expected = match (builtin, shape(layouts, receiver, a)?) {
                        (GenericBuiltin::MapKeys, GenericRuntimeLayoutKind::Map { key, .. })
                        | (GenericBuiltin::SetValues, GenericRuntimeLayoutKind::Set { key, .. }) => {
                            *key
                        }
                        (
                            GenericBuiltin::MapValues,
                            GenericRuntimeLayoutKind::Map { value, .. },
                        ) => *value,
                        (
                            GenericBuiltin::MapEntries,
                            GenericRuntimeLayoutKind::Map { key, value, .. },
                        ) => {
                            let (key, value) = (*key, *value);
                            let GenericRuntimeLayoutKind::Array { element } =
                                shape(layouts, *layout, a)?
                            else {
                                return Err(a.invalid("entry result array"));
                            };
                            let element = *element;
                            if map_entry_arguments(layouts, element, a)? != (key, value) {
                                return Err(a.invalid("entry receiver arguments"));
                            }
                            element
                        }
                        _ => return Err(a.invalid("collection iteration receiver")),
                    };
                    if !matches!(shape(layouts,*layout,a)?,GenericRuntimeLayoutKind::Array{element} if *element==expected)
                    {
                        return Err(a.invalid("collection iteration element type"));
                    }
                    state = stacks.push(state, *layout, a)?;
                } else if builtin.collection_remove() {
                    let key_type = stacks.pop(&mut state, a)?;
                    let receiver = stacks.pop(&mut state, a)?;
                    let key = match (builtin, shape(layouts, receiver, a)?) {
                        (GenericBuiltin::MapRemove, GenericRuntimeLayoutKind::Map { key, .. })
                        | (GenericBuiltin::SetRemove, GenericRuntimeLayoutKind::Set { key, .. }) => {
                            *key
                        }
                        _ => return Err(a.invalid("collection removal receiver")),
                    };
                    if key_type != key || option_payload(layouts, *layout, a)? != receiver {
                        return Err(a.invalid("collection removal operand/result types"));
                    }
                    state = stacks.push(state, *layout, a)?;
                } else if builtin.collection_get() {
                    let key_type = stacks.pop(&mut state, a)?;
                    let receiver = stacks.pop(&mut state, a)?;
                    let (key, payload) = match (builtin, shape(layouts, receiver, a)?) {
                        (
                            GenericBuiltin::MapGet,
                            GenericRuntimeLayoutKind::Map { key, value, .. },
                        ) => (*key, *value),
                        (GenericBuiltin::SetGet, GenericRuntimeLayoutKind::Set { key, .. }) => {
                            (*key, *key)
                        }
                        _ => return Err(a.invalid("collection lookup receiver")),
                    };
                    if key_type != key || option_payload(layouts, *layout, a)? != payload {
                        return Err(a.invalid("collection lookup operand/result types"));
                    }
                    state = stacks.push(state, *layout, a)?;
                } else if builtin.collection_contains() {
                    let key_type = stacks.pop(&mut state, a)?;
                    let receiver = stacks.pop(&mut state, a)?;
                    let expected = match (builtin, shape(layouts, receiver, a)?) {
                        (
                            GenericBuiltin::MapContains,
                            GenericRuntimeLayoutKind::Map { key, .. },
                        )
                        | (
                            GenericBuiltin::SetContains,
                            GenericRuntimeLayoutKind::Set { key, .. },
                        ) => *key,
                        _ => return Err(a.invalid("collection membership receiver")),
                    };
                    if key_type != expected {
                        return Err(a.invalid("collection membership key"));
                    }
                    is_primitive(layouts, *layout, GenericPrimitive::Bool, a)?;
                    state = stacks.push(state, *layout, a)?;
                } else if builtin.collection_insert() {
                    match (builtin, shape(layouts, *layout, a)?) {
                        (
                            GenericBuiltin::MapInsert,
                            GenericRuntimeLayoutKind::Map { key, value, .. },
                        ) => {
                            stacks.expect(&mut state, *value, a)?;
                            stacks.expect(&mut state, *key, a)?;
                        }
                        (GenericBuiltin::SetInsert, GenericRuntimeLayoutKind::Set { key, .. }) => {
                            stacks.expect(&mut state, *key, a)?;
                        }
                        _ => return Err(a.invalid("collection insertion result")),
                    }
                    stacks.expect(&mut state, *layout, a)?;
                    state = stacks.push(state, *layout, a)?;
                } else if builtin.array_operation() {
                    if *builtin == GenericBuiltin::ArraySlice {
                        for _ in 0..2 {
                            let bound = stacks.pop(&mut state, a)?;
                            is_primitive(layouts, bound, GenericPrimitive::Int, a)?;
                        }
                    } else {
                        let right = stacks.pop(&mut state, a)?;
                        array_compatible_layout(
                            layouts,
                            right,
                            right_layout.ok_or_else(|| a.invalid("array conversion target"))?,
                            a,
                        )?;
                    }
                    stacks.expect(&mut state, *layout, a)?;
                    state = stacks.push(state, *layout, a)?;
                } else {
                    for _ in 0..builtin.arity() {
                        let ty = stacks.pop(&mut state, a)?;
                        match builtin {
                            GenericBuiltin::ToString => {
                                shape(layouts, ty, a)?;
                            }
                            GenericBuiltin::MapLen | GenericBuiltin::SetLen => {
                                if !matches!(
                                    (builtin.collection_kind(), shape(layouts, ty, a)?),
                                    (
                                        Some(super::GenericCollectionLayoutKind::Map),
                                        GenericRuntimeLayoutKind::Map { .. }
                                    ) | (
                                        Some(super::GenericCollectionLayoutKind::Set),
                                        GenericRuntimeLayoutKind::Set { .. }
                                    )
                                ) {
                                    return Err(a.invalid("collection_len operand layout"));
                                }
                            }
                            GenericBuiltin::ArrayLen => {
                                if !matches!(
                                    shape(layouts, ty, a)?,
                                    GenericRuntimeLayoutKind::Array { .. }
                                ) {
                                    return Err(a.invalid("array_len operand"));
                                }
                            }
                            _ => is_primitive(layouts, ty, GenericPrimitive::String, a)?,
                        }
                    }
                    state = stacks.push(state, *layout, a)?;
                }
            }
            GenericIrOperation::Propagate { input, success, .. } => {
                // Structural validation proves the failure return layout. Failure is a
                // terminal edge that abandons this frame, including pending operands.
                stacks.expect(&mut state, *input, a)?;
                state = stacks.push(state, *success, a)?;
            }
            GenericIrOperation::Constant { layout, .. } | GenericIrOperation::Unit { layout } => {
                state = stacks.push(state, *layout, a)?
            }
            GenericIrOperation::Load { slot } => {
                state = stacks.push(state, function.slot_layouts[*slot], a)?
            }
            GenericIrOperation::Store { slot, .. } => {
                stacks.expect(&mut state, function.slot_layouts[*slot], a)?
            }
            GenericIrOperation::IndexStore { slot } => {
                let value = stacks.pop(&mut state, a)?;
                let index = stacks.pop(&mut state, a)?;
                is_primitive(layouts, index, GenericPrimitive::Int, a)?;
                if !matches!(shape(layouts,function.slot_layouts[*slot],a)?,GenericRuntimeLayoutKind::Array{element}if *element==value)
                {
                    return Err(a.invalid("indexed replacement type"));
                }
            }
            GenericIrOperation::Drop => {
                stacks.pop(&mut state, a)?;
            }
            GenericIrOperation::IntToFloat { layout } => {
                let source = stacks.pop(&mut state, a)?;
                is_primitive(layouts, source, GenericPrimitive::Int, a)?;
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Binary { operator, layout } => {
                let right = stacks.pop(&mut state, a)?;
                let left = stacks.pop(&mut state, a)?;
                match operator {
                    GenericIrBinary::Equality { .. } => {
                        if left != right {
                            return Err(a.invalid("equality operand types"));
                        }
                        is_primitive(layouts, *layout, GenericPrimitive::Bool, a)?;
                    }
                    GenericIrBinary::String(operator) => {
                        is_primitive(layouts, left, GenericPrimitive::String, a)?;
                        is_primitive(layouts, right, GenericPrimitive::String, a)?;
                        is_primitive(
                            layouts,
                            *layout,
                            if *operator == GenericStringOperator::Concatenate {
                                GenericPrimitive::String
                            } else {
                                GenericPrimitive::Bool
                            },
                            a,
                        )?;
                    }
                    GenericIrBinary::Numeric(operator) => {
                        if left != right
                            || !matches!(
                                shape(layouts, left, a)?,
                                GenericRuntimeLayoutKind::Primitive(
                                    GenericPrimitive::Int | GenericPrimitive::Float
                                )
                            )
                        {
                            return Err(a.invalid("numeric operand types"));
                        }
                        let ordering = matches!(
                            operator,
                            GenericNumericOperator::Less
                                | GenericNumericOperator::LessEqual
                                | GenericNumericOperator::Greater
                                | GenericNumericOperator::GreaterEqual
                        );
                        if ordering {
                            is_primitive(layouts, *layout, GenericPrimitive::Bool, a)?;
                        } else if *layout != left {
                            return Err(a.invalid("numeric result type"));
                        }
                    }
                }
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Array { layout, count } => {
                let GenericRuntimeLayoutKind::Array { element } = shape(layouts, *layout, a)?
                else {
                    unreachable!()
                };
                for _ in 0..*count {
                    stacks.expect(&mut state, *element, a)?;
                }
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Record { layout, slots } => {
                let GenericRuntimeLayoutKind::Record { fields, .. } = shape(layouts, *layout, a)?
                else {
                    unreachable!()
                };
                for slot in slots.iter().rev() {
                    stacks.expect(&mut state, fields[*slot], a)?;
                }
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Enum { layout, tag, .. } => {
                let GenericRuntimeLayoutKind::Enum { variants, .. } = shape(layouts, *layout, a)?
                else {
                    unreachable!()
                };
                if let Some(expected) = variants[*tag] {
                    stacks.expect(&mut state, expected, a)?;
                }
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Field {
                receiver, layout, ..
            } => {
                stacks.expect(&mut state, *receiver, a)?;
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Index { layout } => {
                let index = stacks.pop(&mut state, a)?;
                is_primitive(layouts, index, GenericPrimitive::Int, a)?;
                let array = stacks.pop(&mut state, a)?;
                if !matches!(shape(layouts,array,a)?,GenericRuntimeLayoutKind::Array{element}if element==layout)
                {
                    return Err(a.invalid("array element result type"));
                }
                state = stacks.push(state, *layout, a)?;
            }
            GenericIrOperation::Call { target, .. } => {
                let callee = &all[*target];
                for slot in callee.parameter_slots.iter().rev() {
                    stacks.expect(&mut state, callee.slot_layouts[*slot], a)?;
                }
                state = stacks.push(state, callee.result, a)?;
            }
            GenericIrOperation::Jump { target } => {
                fallthrough = false;
                destinations.push(*target);
            }
            GenericIrOperation::Branch { yes, no } => {
                let condition = stacks.pop(&mut state, a)?;
                is_primitive(layouts, condition, GenericPrimitive::Bool, a)?;
                fallthrough = false;
                destinations.extend([*yes, *no]);
            }
            GenericIrOperation::Switch { layout, targets } => {
                let mut peek = state;
                stacks.expect(&mut peek, *layout, a)?;
                fallthrough = false;
                destinations.extend(targets.iter().copied());
            }
            GenericIrOperation::MatchBind { layout, .. } => {
                stacks.expect(&mut state, *layout, a)?
            }
            GenericIrOperation::Return => {
                stacks.expect(&mut state, function.result, a)?;
                if state != 0 {
                    return Err(a.invalid("return leaves operand values"));
                }
                fallthrough = false;
            }
            GenericIrOperation::Unreachable => return Err(a.invalid("reachable terminal sentinel")),
        }
        maximum = maximum.max(stacks.depth(state));
        if fallthrough {
            destinations.push(pc + 1);
        }
        for target in destinations {
            a.step()?;
            let existing = states
                .get_mut(target)
                .ok_or_else(|| a.invalid("control falls out of function"))?;
            match existing {
                Some(expected) if *expected != state => {
                    return Err(a.invalid("incompatible typed stack join"))
                }
                Some(_) => {}
                None => {
                    *existing = Some(state);
                    pending.push(target);
                }
            }
        }
    }
    Ok(maximum)
}
/// Lower every concrete source body into owned typed instructions, resolve all
/// labels, check every instruction structure, and validate reachable typed stack
/// joins. Unreachable source operations remain structurally/type checked by their
/// retained proofs; this API does not run a VM or enable public Program/CLI generics.
pub fn lower_generic_ir(
    plans: &CheckedGenericConstruction,
    limits: GenericIrLimits,
) -> Result<CheckedGenericIr, GenericIrError> {
    if plans.layouts().has_data() {
        return Err(GenericIrError::Construction(
            GenericConstructionError::DataOperationsUnavailable,
        ));
    }
    if plans.layouts().has_collections() {
        return Err(GenericIrError::Construction(
            GenericConstructionError::CollectionOperationsUnavailable,
        ));
    }
    if plans
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .propagation_usage()
        .is_some_and(|usage| usage.sites > 0)
    {
        return Err(GenericIrError::PropagationUnavailable);
    }
    lower_mode(plans, limits, None, false, false)
}

/// Opt-in early-return IR. Execution requires `execute_generic_propagation_ir`.
#[derive(Debug, Clone)]
pub struct CheckedGenericPropagationIr {
    ir: CheckedGenericIr,
}
impl CheckedGenericPropagationIr {
    pub(super) fn checked_ir(&self) -> &CheckedGenericIr {
        &self.ir
    }
    /// Retained layout ownership for all instruction indices.
    pub fn layouts(&self) -> &CheckedGenericLayouts {
        self.ir.plans().layouts()
    }
    /// Concrete instance-order typed instructions, including implicit terminal edges.
    pub fn functions(&self) -> &[GenericIrFunction] {
        self.ir.functions()
    }
    /// Complete shared lowering and validation usage.
    pub fn usage(&self) -> GenericIrUsage {
        self.ir.usage()
    }
}
/// Opt-in bounded early-return lowering, without source/CLI/native enablement.
pub fn lower_generic_propagation_ir(
    propagation: &CheckedGenericPropagationLayouts,
    construction_limits: GenericConstructionLimits,
    limits: GenericIrLimits,
) -> Result<CheckedGenericPropagationIr, GenericIrError> {
    let plans = plan_generic_construction(propagation.layouts(), construction_limits)
        .map_err(GenericIrError::Construction)?;
    Ok(CheckedGenericPropagationIr {
        ir: lower_mode(&plans, limits, Some(propagation), false, false)?,
    })
}
/// Lower the explicit collection profile; ordinary lowering retains its existing gate.
pub fn lower_generic_collection_propagation_ir(
    propagation: &CheckedGenericPropagationLayouts,
    construction_limits: GenericConstructionLimits,
    limits: GenericIrLimits,
) -> Result<CheckedGenericPropagationIr, GenericIrError> {
    let plans =
        super::plan_generic_collection_construction(propagation.layouts(), construction_limits)
            .map_err(GenericIrError::Construction)?;
    Ok(CheckedGenericPropagationIr {
        ir: lower_mode(&plans, limits, Some(propagation), true, false)?,
    })
}
/// Lower the explicit data profile with retained data feature admission.
pub fn lower_generic_data_propagation_ir(
    propagation: &CheckedGenericPropagationLayouts,
    construction_limits: GenericConstructionLimits,
    limits: GenericIrLimits,
) -> Result<CheckedGenericPropagationIr, GenericIrError> {
    let plans = super::plan_generic_data_construction(propagation.layouts(), construction_limits)
        .map_err(GenericIrError::Construction)?;
    Ok(CheckedGenericPropagationIr {
        ir: lower_mode(&plans, limits, Some(propagation), true, true)?,
    })
}
fn lower_mode(
    plans: &CheckedGenericConstruction,
    limits: GenericIrLimits,
    propagation: Option<&CheckedGenericPropagationLayouts>,
    collections: bool,
    data: bool,
) -> Result<CheckedGenericIr, GenericIrError> {
    let mut a = Admission {
        left: limits,
        function: 0,
        pending_limit: limits.max_pending,
        pending_base: 0,
        peak_pending: 0,
        span: plans.bodies()[0]
            .body()
            .body()
            .body()
            .signature()
            .syntax()
            .body_span(),
    };
    let mut float = None;
    for (index, layout) in plans.layouts().layouts().iter().enumerate() {
        a.step()?;
        if layout.kind() == &GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Float) {
            float = Some(index);
        }
    }
    let mut functions = Vec::new();
    let mut propagation_sites = propagation.into_iter().flat_map(|p| p.plans()).peekable();
    for (index, body) in plans.bodies().iter().enumerate() {
        a.function = index;
        a.step()?;
        charge(&mut a.left.max_functions, 1, "generic IR functions")?;
        let source = body.body().body().body();
        a.span = source.signature().syntax().body_span();
        charge(
            &mut a.left.max_slots,
            source.bindings().len(),
            "generic IR slots",
        )?;
        let mut parameters = vec![None; body.body().parameter_layouts().len()];
        let mut slots = Vec::new();
        let mut kinds = Vec::new();
        for binding in source.bindings() {
            a.step()?;
            slots.push(
                body.body()
                    .binding_layout(binding.id)
                    .ok_or_else(|| a.invalid("binding layout"))?,
            );
            kinds.push(binding.kind);
            if let GenericCopyBindingKind::Parameter(ordinal) = binding.kind {
                let entry = parameters
                    .get_mut(ordinal)
                    .ok_or_else(|| a.invalid("parameter slot"))?;
                if entry.replace(binding.id.ordinal()).is_some() {
                    return Err(a.invalid("duplicate parameter slot"));
                }
            }
        }
        let parameters = parameters
            .into_iter()
            .map(|p| p.ok_or_else(|| a.invalid("missing parameter slot")))
            .collect::<Result<Vec<_>, _>>()?;
        let mut calls = BTreeMap::new();
        let call_sites = plans.layouts().literals().concrete().graph().instances()[index].calls();
        if call_sites.len() > a.left.max_instructions {
            return Err(GenericIrError::Limit("generic IR call mappings"));
        }
        for call in call_sites {
            a.step()?;
            calls.insert(call.expression.ordinal(), call.target);
        }
        let mut propagations = BTreeMap::new();
        while propagation_sites
            .peek()
            .is_some_and(|p| p.instance == index)
        {
            a.step()?;
            let site = propagation_sites.next().expect("peeked propagation site");
            charge(
                &mut a.left.max_operands,
                1,
                "generic IR propagation mappings",
            )?;
            if propagations
                .insert(site.expression.ordinal(), site.clone())
                .is_some()
            {
                return Err(a.invalid("duplicate propagation operation"));
            }
        }
        let code = Emitter {
            plans,
            body,
            a: &mut a,
            labels: Vec::new(),
            code: Vec::new(),
            calls,
            float,
            propagations,
        }
        .finish()?;
        functions.push(GenericIrFunction {
            parameter_slots: parameters.into(),
            slot_layouts: slots.into(),
            slot_kinds: kinds.into(),
            result: body.body().return_layout(),
            code: code.into(),
            max_stack: 0,
        });
    }
    if propagation_sites.next().is_some() {
        return Err(a.invalid("unconsumed propagation operations"));
    }
    // Validate all callee tables before any flow pass indexes a call signature.
    for (index, function) in functions.iter().enumerate() {
        a.function = index;
        structural(function, &functions, plans.layouts(), &mut a)?;
    }
    let mut stacks = Stacks {
        nodes: Vec::new(),
        index: BTreeMap::new(),
        depth_limit: limits.max_stack_depth,
    };
    for index in 0..functions.len() {
        a.function = index;
        let maximum = validate(
            &functions[index],
            &functions,
            plans.layouts(),
            &mut stacks,
            &mut a,
        )?;
        functions[index].max_stack = maximum;
    }
    Ok(CheckedGenericIr {
        collections,
        data,
        plans: plans.clone(),
        functions: functions.into(),
        usage: GenericIrUsage {
            functions: limits.max_functions - a.left.max_functions,
            instructions: limits.max_instructions - a.left.max_instructions,
            slots: limits.max_slots - a.left.max_slots,
            operands: limits.max_operands - a.left.max_operands,
            labels: limits.max_labels - a.left.max_labels,
            targets: limits.max_targets - a.left.max_targets,
            peak_pending: a.peak_pending,
            stack_nodes: limits.max_stack_nodes - a.left.max_stack_nodes,
            work: limits.max_work - a.left.max_work,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(source: &str) -> CheckedGenericIr {
        let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
        let env = prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT)
            .unwrap();
        let types =
            check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
        let root = parse_specialization_root(source, GenericFunctionLimits::DEFAULT).unwrap();
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
        let layouts = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
        let plans =
            plan_generic_construction(&layouts, GenericConstructionLimits::DEFAULT).unwrap();
        lower_generic_ir(&plans, GenericIrLimits::DEFAULT).unwrap()
    }
    fn verify(
        ir: &CheckedGenericIr,
        functions: &[GenericIrFunction],
    ) -> Result<(), GenericIrError> {
        let mut a = Admission {
            left: GenericIrLimits::DEFAULT,
            function: 0,
            span: functions[0].code.first().map_or(
                Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
                |i| i.span,
            ),
            pending_limit: GenericIrLimits::DEFAULT.max_pending,
            pending_base: 0,
            peak_pending: 0,
        };
        for (index, function) in functions.iter().enumerate() {
            a.function = index;
            structural(function, functions, ir.plans.layouts(), &mut a)?;
        }
        let mut stacks = Stacks {
            nodes: Vec::new(),
            index: BTreeMap::new(),
            depth_limit: GenericIrLimits::DEFAULT.max_stack_depth,
        };
        for (index, function) in functions.iter().enumerate() {
            a.function = index;
            validate(function, functions, ir.plans.layouts(), &mut stacks, &mut a)?;
        }
        Ok(())
    }
    #[test]
    fn equal_height_but_different_type_joins_are_refused() {
        let ir = fixture("fn root(flag:Bool){flag&&(1==1);}");
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let int = ir
            .plans
            .layouts()
            .layouts()
            .iter()
            .position(|l| l.kind() == &GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int))
            .unwrap();
        let source = code
            .iter_mut()
            .find(|i| {
                matches!(
                    i.operation,
                    GenericIrOperation::Constant {
                        value: GenericLiteralValue::Bool(false),
                        ..
                    }
                )
            })
            .unwrap();
        source.operation = GenericIrOperation::Constant {
            value: GenericLiteralValue::Int(0),
            layout: int,
        };
        functions[0].code = code.into();
        assert!(matches!(
            verify(&ir, &functions),
            Err(GenericIrError::Invalid {
                reason: "incompatible typed stack join",
                ..
            })
        ));
    }
    #[test]
    fn malformed_targets_stack_underflow_and_empty_code_do_not_publish_ir() {
        let ir = fixture("fn root(){}");
        for operation in [
            GenericIrOperation::Jump { target: usize::MAX },
            GenericIrOperation::Call {
                target: usize::MAX,
                arguments: 0,
            },
            GenericIrOperation::Drop,
        ] {
            let mut functions = ir.functions.to_vec();
            let mut code = functions[0].code.to_vec();
            code[0].operation = operation;
            functions[0].code = code.into();
            assert!(verify(&ir, &functions).is_err());
        }
        let floats = fixture("fn root()->Float{return 1.0;}");
        let mut functions = floats.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let GenericIrOperation::Constant { value, .. } = &mut code[0].operation else {
            unreachable!()
        };
        *value = GenericLiteralValue::FloatBits(f64::INFINITY.to_bits());
        functions[0].code = code.into();
        assert!(matches!(
            verify(&floats, &functions),
            Err(GenericIrError::Invalid {
                reason: "nonfinite constant payload",
                ..
            })
        ));
        let mut functions = ir.functions.to_vec();
        functions[0].code = Arc::from([]);
        assert!(verify(&ir, &functions).is_err());
    }
    #[cfg(target_os = "linux")]
    fn collection_fixture(source: &str) -> CompiledGenericPackage {
        let root = std::env::temp_dir().join(format!(
            "sovra-lookup-ir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname=\"lookup_ir\"\nentry=\"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"), source).unwrap();
        let program = compile_generic_collection_package(
            crate::compiler::project::packages::capture_package_sources(
                &root,
                crate::compiler::project::packages::PackageCaptureLimits::DEFAULT,
            )
            .unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
        .unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        program
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn lookup_and_removal_ir_reject_wrong_wrappers_payloads_and_receivers() {
        let program=collection_fixture("enum Fake<T>{Some(T),None}fn main()->std::Option<String>{let other=std::Option::<Bool>::Some(true);let fake=Fake::<String>::Some(\"x\");let m:std::Map<Int,String>=std::map_new();let removed=std::map_remove(m,1);return std::map_get(m,1);}");
        let ir = program.ir().checked_ir();
        assert!(verify(ir, &ir.functions).is_ok());
        let layouts = ir.plans().layouts();
        let bool_layout = layouts
            .layouts()
            .iter()
            .position(|l| {
                matches!(
                    l.kind(),
                    GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Bool)
                )
            })
            .unwrap();
        let types = layouts
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .types();
        let option_bool = layouts.layouts().iter().position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Enum { declaration, arguments, .. } if arguments.as_slice() == [bool_layout] && matches!(types.definition(*declaration), Some(CheckedGenericDefinition::Enum(def)) if def.syntax().standard_identity() == Some(GenericStandardEnum::Option)))).unwrap();
        let fake = layouts.layouts().iter().position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Enum { declaration, .. } if matches!(types.definition(*declaration), Some(CheckedGenericDefinition::Enum(def)) if def.syntax().standard_identity().is_none()))).unwrap();
        for selected in [GenericBuiltin::MapGet, GenericBuiltin::MapRemove] {
            for change in 0..6 {
                let mut functions = ir.functions.to_vec();
                let mut code = functions[0].code.to_vec();
                let op = code
                    .iter_mut()
                    .find(|i| {
                        matches!(
                            i.operation,
                            GenericIrOperation::Builtin {
                                builtin,
                                ..
                            } if builtin == selected
                        )
                    })
                    .unwrap();
                let GenericIrOperation::Builtin {
                    builtin,
                    layout,
                    right_layout,
                } = &mut op.operation
                else {
                    unreachable!()
                };
                match change {
                    0 => {
                        *builtin = if selected == GenericBuiltin::MapGet {
                            GenericBuiltin::SetGet
                        } else {
                            GenericBuiltin::SetRemove
                        }
                    }
                    1 => {
                        *layout = option_bool;
                        functions[0].result = option_bool;
                    }
                    2 => {
                        *layout = fake;
                        functions[0].result = fake;
                    }
                    3 => *right_layout = Some(bool_layout),
                    4 => *layout = usize::MAX,
                    _ => {
                        *builtin = if selected == GenericBuiltin::MapGet {
                            GenericBuiltin::MapRemove
                        } else {
                            GenericBuiltin::MapGet
                        }
                    }
                }
                functions[0].code = code.into();
                assert!(verify(ir, &functions).is_err(), "mutation {change}");
            }
        }
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn collection_iteration_ir_rejects_wrong_elements_constructors_and_conversion_targets() {
        let program=collection_fixture("fn main()->[String]{let bools:[Bool]=[true];let m:std::Map<Int,String>=std::map_new();let keys=std::map_keys(m);let s:std::Set<Int>=std::set_new();let set=std::set_values(s);return std::map_values(m);}");
        let ir = program.ir().checked_ir();
        assert!(verify(ir, &ir.functions).is_ok());
        let layouts = ir.plans().layouts();
        let bool_layout = layouts
            .layouts()
            .iter()
            .position(|l| {
                matches!(
                    l.kind(),
                    GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Bool)
                )
            })
            .unwrap();
        let bool_array=layouts.layouts().iter().position(|l|matches!(l.kind(),GenericRuntimeLayoutKind::Array{element} if *element==bool_layout)).unwrap();
        for selected in [
            GenericBuiltin::MapKeys,
            GenericBuiltin::MapValues,
            GenericBuiltin::SetValues,
        ] {
            for change in 0..5 {
                let mut functions = ir.functions.to_vec();
                let mut code = functions[0].code.to_vec();
                let op=code.iter_mut().find(|i|matches!(i.operation,GenericIrOperation::Builtin{builtin,..} if builtin==selected)).unwrap();
                let GenericIrOperation::Builtin {
                    builtin,
                    layout,
                    right_layout,
                } = &mut op.operation
                else {
                    unreachable!()
                };
                match change {
                    0 => {
                        *builtin = if selected == GenericBuiltin::SetValues {
                            GenericBuiltin::MapKeys
                        } else {
                            GenericBuiltin::SetValues
                        }
                    }
                    1 => *layout = bool_array,
                    2 => *layout = bool_layout,
                    3 => *right_layout = Some(bool_layout),
                    _ => *layout = usize::MAX,
                }
                functions[0].code = code.into();
                assert!(
                    verify(ir, &functions).is_err(),
                    "{selected:?} mutation {change}"
                );
            }
        }
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn map_entries_ir_rejects_user_homonyms_wrong_arguments_and_non_entry_arrays() {
        let program=collection_fixture("struct MapEntry<K:std::Key,V>{key:K,value:V}fn main()->[std::MapEntry<Int,String>]{let fake:[MapEntry<Int,String>]=[MapEntry::<Int,String>{key:7,value:\"x\"}];let other:[std::MapEntry<String,Int>]=[std::MapEntry::<String,Int>{key:\"x\",value:7}];let m:std::Map<Int,String>=std::map_new();let keys=std::map_keys(m);return std::map_entries(m);}");
        let ir = program.ir().checked_ir();
        assert!(verify(ir, &ir.functions).is_ok());
        let layouts = ir.plans().layouts();
        let types = layouts
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .types();
        let mut arrays = Vec::new();
        for (index, l) in layouts.layouts().iter().enumerate() {
            let GenericRuntimeLayoutKind::Array { element } = l.kind() else {
                continue;
            };
            if let GenericRuntimeLayoutKind::Record {
                declaration,
                arguments,
                ..
            } = layouts.layouts()[*element].kind()
            {
                let Some(CheckedGenericDefinition::Record(def)) = types.definition(*declaration)
                else {
                    continue;
                };
                if !def.syntax().is_standard_map_entry()
                    || matches!(
                        layouts.layouts()[arguments[0]].kind(),
                        GenericRuntimeLayoutKind::Primitive(GenericPrimitive::String)
                    )
                {
                    arrays.push(index);
                }
            } else {
                arrays.push(index);
            }
        }
        assert_eq!(arrays.len(), 3);
        for alternative in arrays {
            let mut functions = ir.functions.to_vec();
            let mut code = functions[0].code.to_vec();
            let op = code
                .iter_mut()
                .find(|i| {
                    matches!(
                        i.operation,
                        GenericIrOperation::Builtin {
                            builtin: GenericBuiltin::MapEntries,
                            ..
                        }
                    )
                })
                .unwrap();
            let GenericIrOperation::Builtin { layout, .. } = &mut op.operation else {
                unreachable!()
            };
            *layout = alternative;
            functions[0].result = alternative;
            functions[0].code = code.into();
            assert!(verify(ir, &functions).is_err());
        }
        for builtin in [GenericBuiltin::MapKeys, GenericBuiltin::SetValues] {
            let mut functions = ir.functions.to_vec();
            let mut code = functions[0].code.to_vec();
            let op = code
                .iter_mut()
                .find(|i| {
                    matches!(
                        i.operation,
                        GenericIrOperation::Builtin {
                            builtin: GenericBuiltin::MapEntries,
                            ..
                        }
                    )
                })
                .unwrap();
            let GenericIrOperation::Builtin {
                builtin: actual, ..
            } = &mut op.operation
            else {
                unreachable!()
            };
            *actual = builtin;
            functions[0].code = code.into();
            assert!(verify(ir, &functions).is_err());
        }
    }
    fn propagation_fixture(source: &str) -> CheckedGenericPropagationIr {
        let base =
            compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
        let types = base
            .ir()
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .types();
        let syntax =
            parse_generic_propagation_function(source, GenericFunctionLimits::DEFAULT).unwrap();
        let signatures =
            check_generic_signatures(types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
        let bodies = check_generic_propagation_bodies(
            &signatures,
            GenericCopyBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
        let graph = discover_generic_propagation_specializations(
            &bodies,
            signatures.find("root").unwrap(),
            GenericSpecializationLimits::DEFAULT,
        )
        .unwrap();
        let concrete = recheck_generic_propagation_concrete_bodies(
            &graph,
            GenericConcreteBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
        let literals = decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap();
        let layouts = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
        let operations =
            plan_generic_propagation_layouts(&layouts, GenericPropagationLayoutLimits::DEFAULT)
                .unwrap();
        lower_generic_propagation_ir(
            &operations,
            GenericConstructionLimits::DEFAULT,
            GenericIrLimits::DEFAULT,
        )
        .unwrap()
    }
    #[test]
    fn propagation_instruction_corruption_is_independently_rejected() {
        let p = propagation_fixture("fn root(x:std::Option<Int>)->std::Option<Bool>{let n=1+x?;return std::Option::<Bool>::Some(true);}");
        let ir = &p.ir;
        assert!(verify(ir, &ir.functions).is_ok());
        for change in 0..5 {
            let mut functions = ir.functions.to_vec();
            let mut code = functions[0].code.to_vec();
            let op = code
                .iter_mut()
                .find(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
                .unwrap();
            let GenericIrOperation::Propagate {
                input,
                success,
                result,
                success_tag,
                failure,
                ..
            } = &mut op.operation
            else {
                unreachable!()
            };
            match change {
                0 => *result = *success,
                1 => *success = *input,
                2 => *success_tag = 1,
                3 => *failure = Some(*success),
                _ => *input = usize::MAX,
            }
            functions[0].code = code.into();
            assert!(verify(ir, &functions).is_err());
        }
    }
    #[test]
    fn vm_refuses_inspection_ir_before_root_arguments_or_fuel() {
        let p = propagation_fixture(
            "fn root(x:std::Option<Int>)->std::Option<Int>{let n=x?;return x;}",
        );
        assert!(matches!(
            execute_generic_ir(
                &p.ir,
                GenericVmPolicy {
                    max_work: 0,
                    ..GenericVmPolicy::DEFAULT
                }
            ),
            Err(GenericVmError {
                kind: GenericVmErrorKind::PropagationUnavailable,
                ..
            })
        ));
    }
    #[test]
    fn result_failure_payload_corruption_is_rejected() {
        let p = propagation_fixture("fn root(x:std::Result<Int,String>)->std::Result<Bool,String>{let n=x?;return std::Result::<Bool,String>::Ok(true);}");
        let mut functions = p.ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let op = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Propagate { .. }))
            .unwrap();
        let GenericIrOperation::Propagate {
            success, failure, ..
        } = &mut op.operation
        else {
            unreachable!()
        };
        *failure = Some(*success);
        functions[0].code = code.into();
        assert!(verify(&p.ir, &functions).is_err());
    }
    #[test]
    fn builtin_ir_rejects_wrong_standard_identity_and_argument_stack_types() {
        let p = compile_generic_builtin_source(
            "fn main()->std::ParsedInt{return std::parse_int(\"7\");}",
            "main",
            GenericPropagationSourceLimits::DEFAULT,
        )
        .unwrap();
        let ir = p.ir();
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let builtin = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Builtin { .. }))
            .unwrap();
        let GenericIrOperation::Builtin { builtin, .. } = &mut builtin.operation else {
            unreachable!()
        };
        *builtin = GenericBuiltin::SplitOnce;
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let int = ir
            .plans()
            .layouts()
            .layouts()
            .iter()
            .position(|l| l.kind() == &GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int))
            .unwrap();
        code[0].operation = GenericIrOperation::Constant {
            value: GenericLiteralValue::Int(7),
            layout: int,
        };
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
    }
    #[test]
    fn primitive_builtin_result_and_return_layout_corruption_is_refused() {
        let p = compile_generic_string_source(
            "fn main()->Int{let b=true;return std::len(\"é\");}",
            "main",
            GenericPropagationSourceLimits::DEFAULT,
        )
        .unwrap();
        let ir = p.ir();
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let op = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Builtin { .. }))
            .unwrap();
        let GenericIrOperation::Builtin { builtin, .. } = &mut op.operation else {
            unreachable!()
        };
        *builtin = GenericBuiltin::LinesUnique;
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
        let mut functions = ir.functions.to_vec();
        let bool_layout = ir
            .plans()
            .layouts()
            .layouts()
            .iter()
            .position(|l| l.kind() == &GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Bool))
            .unwrap();
        functions[0].result = bool_layout;
        assert!(verify(ir, &functions).is_err());
    }
    #[test]
    fn array_length_ir_refuses_polymorphic_result_or_nonarray_stack_corruption() {
        let p = compile_generic_value_source(
            "fn main()->Int{let xs:[Int]=[1,2];return std::array_len(xs);}",
            "main",
            GenericPropagationSourceLimits::DEFAULT,
        )
        .unwrap();
        let ir = p.ir();
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let op = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Builtin { .. }))
            .unwrap();
        let GenericIrOperation::Builtin { builtin, .. } = &mut op.operation else {
            unreachable!()
        };
        *builtin = GenericBuiltin::ToString;
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let load = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Load { .. }))
            .unwrap();
        let int = ir
            .plans()
            .layouts()
            .layouts()
            .iter()
            .position(|l| l.kind() == &GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int))
            .unwrap();
        load.operation = GenericIrOperation::Constant {
            value: GenericLiteralValue::Int(1),
            layout: int,
        };
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
    }
    #[test]
    fn array_builtin_conversion_targets_and_first_operand_are_independently_checked() {
        let p = compile_generic_array_source(
            "fn main()->[Float]{let a:[Float]=[];let b:[Int]=[1];return std::array_concat(a,b);}",
            "main",
            GenericPropagationSourceLimits::DEFAULT,
        )
        .unwrap();
        let ir = p.ir();
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let op = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Builtin { .. }))
            .unwrap();
        let GenericIrOperation::Builtin { right_layout, .. } = &mut op.operation else {
            unreachable!()
        };
        *right_layout = None;
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
        let mut functions = ir.functions.to_vec();
        let mut code = functions[0].code.to_vec();
        let load = code
            .iter_mut()
            .find(|i| matches!(i.operation, GenericIrOperation::Load { .. }))
            .unwrap();
        let GenericIrOperation::Load { slot } = &mut load.operation else {
            unreachable!()
        };
        *slot = 1;
        functions[0].code = code.into();
        assert!(verify(ir, &functions).is_err());
    }
}
