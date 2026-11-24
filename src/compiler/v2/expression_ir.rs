//! Checked register instructions for expression trees; function control flow remains separate.
use super::{
    CallableTarget, MappedCallable, MappedExpressionRole, MappedFact, MappedNodeId, MappedSyntax,
    ScopedDeclarationId, ScopedUseId, ScopedUseKind, TypedExpressionId,
};
use crate::compiler::{ast::ExpressionKind, checked::CoercionKind, semantic::Type};
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicU64,
};
/// Admission for expression instruction emission and structural/type validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionIrLimits {
    /// Instruction count, including explicit conversion contexts.
    pub max_instructions: usize,
    /// Typed register definitions; each instruction defines one value.
    pub max_values: usize,
    /// Cumulative input-register references across instructions.
    pub max_operands: usize,
    /// Independent expression roots awaiting statement/control-flow integration.
    pub max_roots: usize,
    /// Initialization/assignment/return contexts deferred to statement emission.
    pub max_deferred_contexts: usize,
    /// Validation steps (instructions, operand references, roots and deferred contexts).
    pub max_validation_steps: usize,
}
impl ExpressionIrLimits {
    /// Opt-in finite expression IR profile; no existing command path changes.
    pub const DEFAULT: Self = Self {
        max_instructions: 1_000_000,
        max_values: 1_000_000,
        max_operands: 2_000_000,
        max_roots: 250_000,
        max_deferred_contexts: 250_000,
        max_validation_steps: 5_000_000,
    };
}
/// Register identity local to one emitted graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpressionValueId {
    unit: u64,
    ordinal: usize,
}
impl ExpressionValueId {
    /// Definition ordinal; operands must reference an earlier definition.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Literal payload borrowed from the immutable checked source AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionLiteral<'a> {
    /// Validated integer spelling.
    Int(&'a str),
    /// Validated floating-point spelling.
    Float(&'a str),
    /// Decoded source text, borrowed without another payload copy.
    String(&'a str),
    /// Boolean value.
    Bool(bool),
}
/// Backend-neutral expression operation. This is not a function execution stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionOperation<'a> {
    /// Materialize a source literal.
    Literal(ExpressionLiteral<'a>),
    /// Snapshot a source-resolved value binding.
    Read {
        /// Checked read-use identity.
        usage: ScopedUseId,
        /// Checked selected binding.
        declaration: ScopedDeclarationId,
    },
    /// Apply a checked binary operator to ordered operands.
    Binary {
        /// Original operator spelling.
        operator: &'a str,
        /// Left value.
        left: ExpressionValueId,
        /// Right value.
        right: ExpressionValueId,
    },
    /// Call a canonical standard/self/nominal constructor target.
    Call {
        /// Index in the retained checked call table.
        call: usize,
        /// Resolved target.
        target: CallableTarget,
        /// Ordered converted arguments.
        arguments: Vec<ExpressionValueId>,
    },
    /// Construct an array after per-item conversions.
    Array(Vec<ExpressionValueId>),
    /// Construct the checked nominal record, with source field order preserved.
    Record(Vec<(&'a str, ExpressionValueId)>),
    /// Project an existing checked record field.
    Field {
        /// Checked receiver.
        receiver: ExpressionValueId,
        /// Original field name.
        field: &'a str,
    },
    /// Read a checked array index.
    Index {
        /// Array value.
        target: ExpressionValueId,
        /// Int index.
        index: ExpressionValueId,
    },
    /// Apply one explicit checked type/transfer context, including identity contexts.
    Convert {
        /// Intrinsic value before conversion.
        value: ExpressionValueId,
        /// Index in the checked context table.
        context: usize,
    },
}
impl ExpressionOperation<'_> {
    fn operands(&self) -> Vec<ExpressionValueId> {
        match self {
            Self::Literal(_) | Self::Read { .. } => Vec::new(),
            Self::Binary { left, right, .. } => vec![*left, *right],
            Self::Call { arguments, .. } | Self::Array(arguments) => arguments.clone(),
            Self::Record(fields) => fields.iter().map(|(_, v)| *v).collect(),
            Self::Field { receiver, .. } => vec![*receiver],
            Self::Index { target, index } => vec![*target, *index],
            Self::Convert { value, .. } => vec![*value],
        }
    }
}
/// Typed register definition, borrowing its type from checked expression/context facts.
#[derive(Debug, Clone)]
pub struct ExpressionValue<'a> {
    /// Graph-local identity.
    pub id: ExpressionValueId,
    /// Original mapped value-expression node.
    pub source: MappedNodeId,
    /// Checked output type; no type inference occurs during emission.
    pub ty: &'a Type,
    /// Intrinsic expression identity; absent for conversion temporaries.
    pub expression: Option<TypedExpressionId>,
    /// Checked context for conversion temporaries; absent for intrinsic expressions.
    pub context: Option<usize>,
}
/// One ordered expression instruction with a single output register.
#[derive(Debug, Clone)]
pub struct ExpressionInstruction<'a> {
    /// Register defined by this instruction.
    pub result: ExpressionValueId,
    /// Backend-neutral checked operation.
    pub operation: ExpressionOperation<'a>,
}
/// Expression root and its intrinsic result, awaiting statement/control-flow placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionRoot {
    /// Root structural node.
    pub source: MappedNodeId,
    /// Intrinsic output register.
    pub result: ExpressionValueId,
}
/// Complete refusal; no partial graph or legacy lowering fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionIrError {
    /// A selected emission/validation dimension was exhausted.
    Limit(&'static str),
    /// Source/fact/register/type/order agreement was unavailable.
    Invalid(&'static str),
    /// Graph identity allocation cannot advance without reuse.
    IdentityExhausted,
}
impl std::fmt::Display for ExpressionIrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expression IR refused: {self:?}")
    }
}
impl std::error::Error for ExpressionIrError {}
/// Validated expression instruction graph. It is not an executable function/program.
#[derive(Debug, Clone)]
pub struct ExpressionIr<'a> {
    mapping: &'a MappedCallable<'a>,
    unit: u64,
    instructions: Vec<ExpressionInstruction<'a>>,
    values: Vec<ExpressionValue<'a>>,
    roots: Vec<ExpressionRoot>,
    deferred: Vec<usize>,
}
impl<'a> ExpressionIr<'a> {
    /// Borrowed checked structural map/source owner.
    pub fn mapping(&self) -> &'a MappedCallable<'a> {
        self.mapping
    }
    /// Ordered instructions within each expression tree, preserving operand evaluation order.
    pub fn instructions(&self) -> &[ExpressionInstruction<'a>] {
        &self.instructions
    }
    /// Register definitions in instruction order.
    pub fn values(&self) -> &[ExpressionValue<'a>] {
        &self.values
    }
    /// Refuse register identities from another emitted graph.
    pub fn value(&self, id: ExpressionValueId) -> Option<&ExpressionValue<'a>> {
        if id.unit == self.unit {
            self.values.get(id.ordinal)
        } else {
            None
        }
    }
    /// Independent roots, in structural source order; these are not all executed unconditionally.
    pub fn roots(&self) -> &[ExpressionRoot] {
        &self.roots
    }
    /// Checked statement-boundary contexts for subsequent statement emission.
    pub fn deferred_contexts(&self) -> &[usize] {
        &self.deferred
    }
    /// Revalidate ownership, source operation shape, dependencies, checked types and conversion placement.
    pub fn validate(&self, max_steps: usize) -> Result<(), ExpressionIrError> {
        let index = Index::new(self.mapping)?;
        let mut steps = 0usize;
        let mut charge = || -> Result<(), ExpressionIrError> {
            if steps >= max_steps {
                return Err(ExpressionIrError::Limit("validation steps"));
            }
            steps += 1;
            Ok(())
        };
        if self.instructions.len() != self.values.len() {
            return Err(ExpressionIrError::Invalid("definition count"));
        }
        let mut expressions = HashSet::new();
        let mut contexts = HashSet::new();
        for (ordinal, (instruction, value)) in
            self.instructions.iter().zip(&self.values).enumerate()
        {
            charge()?;
            if value.id.unit != self.unit
                || value.id.ordinal != ordinal
                || instruction.result != value.id
            {
                return Err(ExpressionIrError::Invalid("register definition"));
            }
            let operands = instruction.operation.operands();
            for operand in &operands {
                charge()?;
                if self.value(*operand).is_none() || operand.ordinal >= ordinal {
                    return Err(ExpressionIrError::Invalid("operand ownership/order"));
                }
            }
            if let ExpressionOperation::Convert {
                value: input,
                context,
            } = &instruction.operation
            {
                let input = self
                    .value(*input)
                    .ok_or(ExpressionIrError::Invalid("conversion operand"))?;
                let fact = index
                    .contexts
                    .get(*context)
                    .ok_or(ExpressionIrError::Invalid("conversion context"))?;
                if boundary(fact.kind)
                    || value.context != Some(*context)
                    || value.expression.is_some()
                    || value.source != input.source
                    || input.context.is_some()
                    || *input.ty != fact.actual
                    || *value.ty != fact.expected
                    || index.context_node(*context)? != value.source
                    || !contexts.insert(*context)
                {
                    return Err(ExpressionIrError::Invalid("conversion type/placement"));
                }
                continue;
            }
            let (expression, ty) = index.expression(value.source)?;
            if value.context.is_some()
                || value.expression != Some(expression)
                || value.ty != ty
                || !expressions.insert(expression)
            {
                return Err(ExpressionIrError::Invalid("intrinsic type/coverage"));
            }
            let source = index.syntax(value.source)?;
            let children = &index.children[value.source.ordinal()];
            if children.len() != operands.len() {
                return Err(ExpressionIrError::Invalid("operand count"));
            }
            for (child, operand) in children.iter().zip(&operands) {
                let actual = self
                    .value(*operand)
                    .ok_or(ExpressionIrError::Invalid("operand"))?;
                let context = index.operand_context(*child, operand_kind(&source.kind))?;
                let expected = if let Some(c) = context {
                    &index.contexts[c].expected
                } else {
                    index.expression(*child)?.1
                };
                if actual.source != *child || actual.context != context || actual.ty != expected {
                    return Err(ExpressionIrError::Invalid("operand checked context"));
                }
            }
            let shape = match (&source.kind, &instruction.operation) {
                (
                    ExpressionKind::Integer(n),
                    ExpressionOperation::Literal(ExpressionLiteral::Int(v)),
                ) => n == v,
                (
                    ExpressionKind::Float(n),
                    ExpressionOperation::Literal(ExpressionLiteral::Float(v)),
                ) => n == v,
                (
                    ExpressionKind::String(n),
                    ExpressionOperation::Literal(ExpressionLiteral::String(v)),
                ) => n == v,
                (
                    ExpressionKind::Boolean(n),
                    ExpressionOperation::Literal(ExpressionLiteral::Bool(v)),
                ) => n == v,
                (
                    ExpressionKind::Identifier(_),
                    ExpressionOperation::Read { usage, declaration },
                ) => index.reads.get(&value.source) == Some(&(*usage, *declaration)),
                (
                    ExpressionKind::Binary { operator, .. },
                    ExpressionOperation::Binary { operator: op, .. },
                ) => operator == op,
                (ExpressionKind::Call { .. }, ExpressionOperation::Call { call, target, .. }) => {
                    index.calls.get(&value.source) == Some(call)
                        && index
                            .call_table
                            .get(*call)
                            .is_some_and(|c| c.target == *target)
                }
                (ExpressionKind::ArrayLiteral(_), ExpressionOperation::Array(_)) => true,
                (
                    ExpressionKind::StructLiteral { fields, .. },
                    ExpressionOperation::Record(actual),
                ) => fields
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .eq(actual.iter().map(|(name, _)| *name)),
                (
                    ExpressionKind::FieldAccess { field, .. },
                    ExpressionOperation::Field { field: actual, .. },
                ) => field == actual,
                (ExpressionKind::Index { .. }, ExpressionOperation::Index { .. }) => true,
                _ => false,
            };
            if !shape {
                return Err(ExpressionIrError::Invalid("source operation"));
            }
        }
        if expressions.len() != index.expressions.len() {
            return Err(ExpressionIrError::Invalid("expression coverage"));
        }
        let expected_roots = index.roots();
        if expected_roots.len() != self.roots.len() {
            return Err(ExpressionIrError::Invalid("root coverage"));
        }
        for (expected, root) in expected_roots.iter().zip(&self.roots) {
            charge()?;
            let value = self
                .value(root.result)
                .ok_or(ExpressionIrError::Invalid("root value"))?;
            if root.source != *expected || value.source != *expected || value.context.is_some() {
                return Err(ExpressionIrError::Invalid("root identity"));
            }
        }
        for deferred in &self.deferred {
            charge()?;
            if !index
                .contexts
                .get(*deferred)
                .is_some_and(|c| boundary(c.kind))
                || !contexts.insert(*deferred)
            {
                return Err(ExpressionIrError::Invalid("deferred context"));
            }
        }
        if contexts.len() != index.contexts.len() {
            return Err(ExpressionIrError::Invalid("context coverage"));
        }
        Ok(())
    }
}
fn boundary(kind: CoercionKind) -> bool {
    matches!(
        kind,
        CoercionKind::Initialization
            | CoercionKind::Assignment
            | CoercionKind::IndexedAssignment
            | CoercionKind::Return
    )
}
fn operand_count(kind: &ExpressionKind) -> usize {
    match kind {
        ExpressionKind::Binary { .. } | ExpressionKind::Index { .. } => 2,
        ExpressionKind::Call { arguments, .. } => arguments.len(),
        ExpressionKind::ArrayLiteral(items) => items.len(),
        ExpressionKind::StructLiteral { fields, .. } => fields.len(),
        ExpressionKind::FieldAccess { .. } => 1,
        _ => 0,
    }
}
fn operand_kind(kind: &ExpressionKind) -> Option<CoercionKind> {
    match kind {
        ExpressionKind::Binary { .. } => Some(CoercionKind::BinaryOperand),
        ExpressionKind::Call { .. } => Some(CoercionKind::Argument),
        ExpressionKind::ArrayLiteral(_) => Some(CoercionKind::ArrayElement),
        ExpressionKind::StructLiteral { .. } => Some(CoercionKind::RecordField),
        _ => None,
    }
}
struct Index<'a> {
    mapping: &'a MappedCallable<'a>,
    children: Vec<Vec<MappedNodeId>>,
    expressions: HashMap<MappedNodeId, TypedExpressionId>,
    reads: HashMap<MappedNodeId, (ScopedUseId, ScopedDeclarationId)>,
    calls: HashMap<MappedNodeId, usize>,
    context_nodes: HashMap<usize, MappedNodeId>,
    by_context: HashMap<(MappedNodeId, CoercionKind), usize>,
    contexts: &'a [super::CallableContext],
    call_table: &'a [super::CallableCall],
}
impl<'a> Index<'a> {
    fn new(mapping: &'a MappedCallable<'a>) -> Result<Self, ExpressionIrError> {
        let owner = mapping.checked();
        let contexts = owner.typed().callable().evidence().contexts();
        let mut index = Self {
            mapping,
            children: vec![Vec::new(); mapping.nodes().len()],
            expressions: HashMap::new(),
            reads: HashMap::new(),
            calls: HashMap::new(),
            context_nodes: HashMap::new(),
            by_context: HashMap::new(),
            contexts,
            call_table: owner.typed().callable().evidence().calls(),
        };
        for node in mapping.nodes() {
            if matches!(
                node.syntax,
                MappedSyntax::Expression {
                    role: MappedExpressionRole::Value,
                    ..
                }
            ) {
                if let Some(parent) = node.parent {
                    if matches!(
                        mapping.node(parent).map(|n| n.syntax),
                        Some(MappedSyntax::Expression {
                            role: MappedExpressionRole::Value,
                            ..
                        })
                    ) {
                        index.children[parent.ordinal()].push(node.id);
                    }
                }
            }
        }
        for link in mapping.links() {
            match link.fact {
                MappedFact::Expression(id) => {
                    index.expressions.insert(link.node, id);
                }
                MappedFact::Use(id) => {
                    let usage = owner
                        .bindings()
                        .usage(id)
                        .ok_or(ExpressionIrError::Invalid("use identity"))?;
                    if usage.kind == ScopedUseKind::Read {
                        index.reads.insert(link.node, (id, usage.declaration));
                    }
                }
                MappedFact::Call(call) => {
                    index.calls.insert(link.node, call);
                }
                MappedFact::Context(context) => {
                    let fact = contexts
                        .get(context)
                        .ok_or(ExpressionIrError::Invalid("context index"))?;
                    if index
                        .by_context
                        .insert((link.node, fact.kind), context)
                        .is_some()
                    {
                        return Err(ExpressionIrError::Invalid("ambiguous context kind"));
                    }
                    index.context_nodes.insert(context, link.node);
                }
                _ => {}
            }
        }
        Ok(index)
    }
    fn syntax(
        &self,
        node: MappedNodeId,
    ) -> Result<&'a crate::compiler::ast::Expression, ExpressionIrError> {
        match self.mapping.node(node).map(|n| n.syntax) {
            Some(MappedSyntax::Expression {
                syntax,
                role: MappedExpressionRole::Value,
            }) => Ok(syntax),
            _ => Err(ExpressionIrError::Invalid("value source")),
        }
    }
    fn expression(
        &self,
        node: MappedNodeId,
    ) -> Result<(TypedExpressionId, &'a Type), ExpressionIrError> {
        let id = *self
            .expressions
            .get(&node)
            .ok_or(ExpressionIrError::Invalid("expression identity"))?;
        let fact = self
            .mapping
            .checked()
            .typed()
            .types()
            .expression(id)
            .ok_or(ExpressionIrError::Invalid("expression type"))?;
        Ok((id, &fact.ty))
    }
    fn operand_context(
        &self,
        node: MappedNodeId,
        kind: Option<CoercionKind>,
    ) -> Result<Option<usize>, ExpressionIrError> {
        Ok(kind.and_then(|kind| self.by_context.get(&(node, kind)).copied()))
    }
    fn context_node(&self, context: usize) -> Result<MappedNodeId, ExpressionIrError> {
        self.context_nodes
            .get(&context)
            .copied()
            .ok_or(ExpressionIrError::Invalid("context owner"))
    }
    fn roots(&self) -> Vec<MappedNodeId> {
        self.mapping
            .nodes()
            .iter()
            .filter(|n| {
                self.expressions.contains_key(&n.id)
                    && n.parent
                        .map_or(true, |p| !self.expressions.contains_key(&p))
            })
            .map(|n| n.id)
            .collect()
    }
}
enum Task {
    Visit(MappedNodeId),
    Bind {
        parent: MappedNodeId,
        slot: usize,
        child: MappedNodeId,
        kind: Option<CoercionKind>,
    },
    Finish(MappedNodeId),
}
struct Emitter<'a> {
    graph: ExpressionIr<'a>,
    index: Index<'a>,
    limits: ExpressionIrLimits,
    operands: usize,
    intrinsic: HashMap<MappedNodeId, ExpressionValueId>,
    inputs: HashMap<(MappedNodeId, usize), ExpressionValueId>,
}
impl<'a> Emitter<'a> {
    fn admit(&self, operands: usize) -> Result<(), ExpressionIrError> {
        if self.graph.instructions.len() >= self.limits.max_instructions {
            return Err(ExpressionIrError::Limit("instructions"));
        }
        if self.graph.values.len() >= self.limits.max_values {
            return Err(ExpressionIrError::Limit("values"));
        }
        if self
            .operands
            .checked_add(operands)
            .filter(|n| *n <= self.limits.max_operands)
            .is_none()
        {
            return Err(ExpressionIrError::Limit("operands"));
        }
        Ok(())
    }
    fn push(
        &mut self,
        source: MappedNodeId,
        expression: Option<TypedExpressionId>,
        context: Option<usize>,
        ty: &'a Type,
        operation: ExpressionOperation<'a>,
    ) -> Result<ExpressionValueId, ExpressionIrError> {
        let operands = operation.operands().len();
        self.admit(operands)?;
        self.operands += operands;
        let id = ExpressionValueId {
            unit: self.graph.unit,
            ordinal: self.graph.values.len(),
        };
        self.graph.values.push(ExpressionValue {
            id,
            source,
            ty,
            expression,
            context,
        });
        self.graph.instructions.push(ExpressionInstruction {
            result: id,
            operation,
        });
        Ok(id)
    }
    fn emit(&mut self) -> Result<(), ExpressionIrError> {
        for root in self.index.roots() {
            if self.graph.roots.len() >= self.limits.max_roots {
                return Err(ExpressionIrError::Limit("roots"));
            }
            let mut pending = vec![Task::Visit(root)];
            while let Some(task) = pending.pop() {
                match task {
                    Task::Visit(node) => {
                        let syntax = self.index.syntax(node)?;
                        pending.push(Task::Finish(node));
                        for (slot, child) in
                            self.index.children[node.ordinal()].iter().enumerate().rev()
                        {
                            pending.push(Task::Bind {
                                parent: node,
                                slot,
                                child: *child,
                                kind: operand_kind(&syntax.kind),
                            });
                            pending.push(Task::Visit(*child));
                        }
                    }
                    Task::Bind {
                        parent,
                        slot,
                        child,
                        kind,
                    } => {
                        let mut value = *self
                            .intrinsic
                            .get(&child)
                            .ok_or(ExpressionIrError::Invalid("child definition"))?;
                        if let Some(context) = self.index.operand_context(child, kind)? {
                            value = self.push(
                                child,
                                None,
                                Some(context),
                                &self.index.contexts[context].expected,
                                ExpressionOperation::Convert { value, context },
                            )?;
                        }
                        self.inputs.insert((parent, slot), value);
                    }
                    Task::Finish(node) => {
                        let source = self.index.syntax(node)?;
                        let children = self.index.children[node.ordinal()].len();
                        if children != operand_count(&source.kind) {
                            return Err(ExpressionIrError::Invalid("source operand shape"));
                        }
                        self.admit(children)?;
                        let operands: Vec<_> = (0..children)
                            .map(|slot| {
                                self.inputs
                                    .get(&(node, slot))
                                    .copied()
                                    .ok_or(ExpressionIrError::Invalid("operand definition"))
                            })
                            .collect::<Result<_, _>>()?;
                        let operation = match &source.kind {
                            ExpressionKind::Integer(v) => {
                                ExpressionOperation::Literal(ExpressionLiteral::Int(v))
                            }
                            ExpressionKind::Float(v) => {
                                ExpressionOperation::Literal(ExpressionLiteral::Float(v))
                            }
                            ExpressionKind::String(v) => {
                                ExpressionOperation::Literal(ExpressionLiteral::String(v))
                            }
                            ExpressionKind::Boolean(v) => {
                                ExpressionOperation::Literal(ExpressionLiteral::Bool(*v))
                            }
                            ExpressionKind::Identifier(_) => {
                                let (usage, declaration) = *self
                                    .index
                                    .reads
                                    .get(&node)
                                    .ok_or(ExpressionIrError::Invalid("read target"))?;
                                ExpressionOperation::Read { usage, declaration }
                            }
                            ExpressionKind::Binary { operator, .. } => {
                                ExpressionOperation::Binary {
                                    operator,
                                    left: operands[0],
                                    right: operands[1],
                                }
                            }
                            ExpressionKind::Call { .. } => {
                                let call = *self
                                    .index
                                    .calls
                                    .get(&node)
                                    .ok_or(ExpressionIrError::Invalid("call target"))?;
                                ExpressionOperation::Call {
                                    call,
                                    target: self.index.call_table[call].target,
                                    arguments: operands,
                                }
                            }
                            ExpressionKind::ArrayLiteral(_) => ExpressionOperation::Array(operands),
                            ExpressionKind::StructLiteral { fields, .. } => {
                                ExpressionOperation::Record(
                                    fields
                                        .iter()
                                        .map(|(name, _)| name.as_str())
                                        .zip(operands)
                                        .collect(),
                                )
                            }
                            ExpressionKind::FieldAccess { field, .. } => {
                                ExpressionOperation::Field {
                                    receiver: operands[0],
                                    field,
                                }
                            }
                            ExpressionKind::Index { .. } => ExpressionOperation::Index {
                                target: operands[0],
                                index: operands[1],
                            },
                            ExpressionKind::QualifiedName { .. } => {
                                return Err(ExpressionIrError::Invalid(
                                    "qualified value without binding",
                                ))
                            }
                        };
                        let (expression, ty) = self.index.expression(node)?;
                        let value = self.push(node, Some(expression), None, ty, operation)?;
                        self.intrinsic.insert(node, value);
                    }
                }
            }
            self.graph.roots.push(ExpressionRoot {
                source: root,
                result: self.intrinsic[&root],
            });
        }
        for (context, fact) in self.index.contexts.iter().enumerate() {
            if boundary(fact.kind) {
                if self.graph.deferred.len() >= self.limits.max_deferred_contexts {
                    return Err(ExpressionIrError::Limit("deferred contexts"));
                }
                self.graph.deferred.push(context);
            }
        }
        Ok(())
    }
}
impl MappedCallable<'_> {
    /// Emit and validate checked expression instructions, preserving per-operand conversion order.
    /// Statement/control-flow/store/return emission and execution remain separate unfinished work.
    pub fn emit_expression_ir(
        &self,
        limits: ExpressionIrLimits,
    ) -> Result<ExpressionIr<'_>, ExpressionIrError> {
        static NEXT_GRAPH: AtomicU64 = AtomicU64::new(0);
        let unit = super::enum_types::allocate_identity(&NEXT_GRAPH)
            .map_err(|_| ExpressionIrError::IdentityExhausted)?;
        let graph = ExpressionIr {
            mapping: self,
            unit,
            instructions: Vec::new(),
            values: Vec::new(),
            roots: Vec::new(),
            deferred: Vec::new(),
        };
        let mut emitter = Emitter {
            graph,
            index: Index::new(self)?,
            limits,
            operands: 0,
            intrinsic: HashMap::new(),
            inputs: HashMap::new(),
        };
        emitter.emit()?;
        emitter.graph.validate(limits.max_validation_steps)?;
        Ok(emitter.graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_graph(test: impl FnOnce(ExpressionIr<'_>)) {
        let context = crate::compiler::parser::Parser::new()
            .parse_source("")
            .unwrap();
        let env = crate::compiler::v2::check_enum_types(
            "o",
            "c",
            &context,
            &[crate::compiler::v2::EnumSource {
                label: "enum",
                source: "enum E{V(Float)}",
            }],
            crate::compiler::v2::EnumTypeLimits::DEFAULT,
        )
        .unwrap();
        let checked = env
            .check_scoped_callable_function_source(
                "f",
                "fn f()->E{return E::V(1);}",
                crate::compiler::v2::ScopedCallableLimits::DEFAULT,
            )
            .unwrap();
        let mapped = checked
            .map_for_lowering(crate::compiler::v2::MappingLimits::DEFAULT)
            .unwrap();
        test(
            mapped
                .emit_expression_ir(ExpressionIrLimits::DEFAULT)
                .unwrap(),
        );
    }
    #[test]
    fn verifier_refuses_forward_or_self_referential_operands() {
        with_graph(|mut graph| {
            let result = graph.values[2].id;
            let ExpressionOperation::Call { arguments, .. } = &mut graph.instructions[2].operation
            else {
                panic!("constructor")
            };
            arguments[0] = result;
            assert_eq!(
                graph.validate(100),
                Err(ExpressionIrError::Invalid("operand ownership/order"))
            );
        });
    }
    #[test]
    fn verifier_refuses_wrong_context_output_types() {
        static BOOL: Type = Type::Bool;
        with_graph(|mut graph| {
            graph.values[1].ty = &BOOL;
            assert_eq!(
                graph.validate(100),
                Err(ExpressionIrError::Invalid("conversion type/placement"))
            );
        });
    }
    #[test]
    fn verifier_refuses_changed_call_targets_and_missing_boundary_contexts() {
        with_graph(|mut graph| {
            let ExpressionOperation::Call { target, .. } = &mut graph.instructions[2].operation
            else {
                panic!("constructor")
            };
            *target = CallableTarget::SelfFunction;
            assert_eq!(
                graph.validate(100),
                Err(ExpressionIrError::Invalid("source operation"))
            );
        });
        with_graph(|mut graph| {
            graph.deferred.clear();
            assert_eq!(
                graph.validate(100),
                Err(ExpressionIrError::Invalid("context coverage"))
            );
        });
    }
    #[test]
    fn verifier_refuses_bypassed_numeric_conversion_even_if_value_source_matches() {
        with_graph(|mut graph| {
            let intrinsic = graph.values[0].id;
            let ExpressionOperation::Call { arguments, .. } = &mut graph.instructions[2].operation
            else {
                panic!("constructor")
            };
            arguments[0] = intrinsic;
            assert_eq!(
                graph.validate(100),
                Err(ExpressionIrError::Invalid("operand checked context"))
            );
        });
    }
}
