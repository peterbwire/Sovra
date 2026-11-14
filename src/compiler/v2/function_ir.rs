//! Function control flow around sealed checked expression graphs for the opt-in Rust VM.
use super::{
    EnumFunctionStatement, EnumId, ExpressionIr, ExpressionIrError, MappedFact, MappedNodeId,
    MappedSyntax, ScopedDeclarationId, ScopedDeclarationKind, ScopedUseKind,
};
use crate::compiler::{
    ast::{Expression, Statement},
    checked::CoercionKind,
    semantic::Type,
};
use std::collections::{HashMap, HashSet, VecDeque};
/// Limits for function instruction emission and definite-availability validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionIrLimits {
    /// Retained function instructions, including scope/cleanup operations.
    pub max_instructions: usize,
    /// Explicit branch/jump/switch edges retained/emitted.
    pub max_edges: usize,
    /// Simultaneously active scope frames, including the function root.
    pub max_scope_frames: usize,
    /// Static instruction, reachable instruction and propagated-edge checks.
    pub max_validation_steps: usize,
    /// Cumulative state entries charged before flow-state copies.
    pub max_state_items: usize,
    /// Independent validation allowance for the borrowed expression graph.
    pub max_expression_validation_steps: usize,
}
impl FunctionIrLimits {
    /// Opt-in finite profile; existing CLI/runtime paths remain unchanged.
    pub const DEFAULT: Self = Self {
        max_instructions: 250_000,
        max_edges: 500_000,
        max_scope_frames: 256,
        max_validation_steps: 2_000_000,
        max_state_items: 16_000_000,
        max_expression_validation_steps: 5_000_000,
    };
}
/// Function-relative value reference; expression roots and boundary conversions are separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionOperand {
    /// Intrinsic result of an expression root in the borrowed expression graph.
    Root(usize),
    /// Result of a checked boundary context, indexed in the retained context table.
    Converted(usize),
}
/// Function instructions, with numeric targets relative to this sealed function only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionInstruction {
    /// Enter a fresh scope instance; the root also owns parameters.
    EnterScope(usize),
    /// Destroy bindings introduced by this scope and leave it.
    ExitScope(usize),
    /// Initialize an immutable source parameter from an invocation argument.
    Parameter {
        /// Source argument position.
        index: usize,
        /// Checked parameter binding.
        declaration: ScopedDeclarationId,
    },
    /// Evaluate one expression tree and define its intrinsic root result.
    Evaluate {
        /// Expression-graph root index.
        root: usize,
    },
    /// Apply a deferred statement-boundary context.
    Convert {
        /// Context table index/result identity.
        context: usize,
        /// Already evaluated intrinsic root.
        root: usize,
    },
    /// Initialize one local snapshot in the active scope.
    Bind {
        /// Checked local declaration.
        declaration: ScopedDeclarationId,
        /// Converted initializer.
        value: FunctionOperand,
    },
    /// Replace an existing mutable binding.
    Store {
        /// Checked selected binding.
        declaration: ScopedDeclarationId,
        /// Converted replacement.
        value: FunctionOperand,
    },
    /// Replace an array slot using the evaluated base snapshot, index and replacement.
    StoreIndex {
        /// Checked mutable array binding.
        declaration: ScopedDeclarationId,
        /// Evaluated base root.
        array: usize,
        /// Evaluated Int index root.
        index: usize,
        /// Converted replacement.
        value: FunctionOperand,
    },
    /// Release the root's expression temporaries and associated boundary conversion.
    Release {
        /// Expression root index.
        root: usize,
    },
    /// Read a bound enum snapshot once for this direct match.
    SnapshotMatch {
        /// Direct-match ordinal.
        index: usize,
        /// Source-resolved enum binding.
        declaration: ScopedDeclarationId,
    },
    /// Dispatch its tag without a materialized Bool or repeated scrutinee evaluation.
    Switch {
        /// Direct-match ordinal.
        index: usize,
        /// Checked nominal enum identity.
        enum_id: EnumId,
        /// Declaration variant ordinal and target instruction.
        arms: Vec<(usize, usize)>,
    },
    /// Snapshot the selected payload into its immutable arm-local declaration.
    Payload {
        /// Direct-match ordinal.
        index: usize,
        /// Checked declaration variant.
        variant: usize,
        /// Immutable payload declaration.
        declaration: ScopedDeclarationId,
    },
    /// Destroy a completed match snapshot.
    ReleaseMatch(usize),
    /// Test and release a Bool condition root, jumping when false.
    BranchFalse {
        /// Evaluated condition root.
        condition: usize,
        /// False-path instruction index.
        target: usize,
    },
    /// Continue at another instruction.
    Jump(usize),
    /// Return a checked value (or Unit), unwinding all active scopes/snapshots/temporaries.
    Return(Option<FunctionOperand>),
    /// Implicit Unit completion after the root scope exits; non-Unit fallthrough is refused.
    End,
}
/// Validation accounting, useful when choosing explicit work/state budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionValidation {
    /// Charged static/reachable/edge checks.
    pub steps: usize,
    /// Charged entries copied into work states.
    pub state_items: usize,
}
/// Complete refusal; never returns partial function instructions or a legacy fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionIrError {
    /// Borrowed expression graph validation failed.
    Expression(ExpressionIrError),
    /// A selected instruction/edge/scope/work/state dimension was exhausted.
    Limit(&'static str),
    /// Type, ownership, scope, availability, dispatch or target validation failed.
    Invalid(&'static str),
}
impl std::fmt::Display for FunctionIrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "function IR refused: {self:?}")
    }
}
impl std::error::Error for FunctionIrError {}
/// Validated single-function control-flow IR borrowing its checked expression graph.
/// The opt-in Rust VM supports a restricted standard adapter; native/JavaScript
/// serialization and full runtime/host integration remain unavailable.
#[derive(Debug, Clone)]
pub struct FunctionIr<'a> {
    expressions: &'a ExpressionIr<'a>,
    instructions: Vec<FunctionInstruction>,
}
impl<'a> FunctionIr<'a> {
    /// Borrowed expression instructions and checked source owner.
    pub fn expressions(&self) -> &'a ExpressionIr<'a> {
        self.expressions
    }
    /// Function-relative instruction stream.
    pub fn instructions(&self) -> &[FunctionInstruction] {
        &self.instructions
    }
    /// Checked return type, supplied by retained return contexts (or Unit).
    pub fn return_type(&self) -> &'a Type {
        static UNIT: Type = Type::Unit;
        self.contexts()
            .iter()
            .find(|c| c.kind == CoercionKind::Return)
            .map_or(&UNIT, |c| &c.expected)
    }
    fn contexts(&self) -> &'a [super::CallableContext] {
        self.expressions
            .mapping()
            .checked()
            .typed()
            .callable()
            .evidence()
            .contexts()
    }
    fn bindings(&self) -> &'a super::ScopedCallableEvidence {
        self.expressions.mapping().checked().bindings()
    }
    fn root_type(&self, root: usize) -> Result<&'a Type, FunctionIrError> {
        let root = self
            .expressions
            .roots()
            .get(root)
            .ok_or(FunctionIrError::Invalid("root index"))?;
        Ok(self
            .expressions
            .value(root.result)
            .ok_or(FunctionIrError::Invalid("root register"))?
            .ty)
    }
    fn value_type(&self, value: FunctionOperand) -> Result<&'a Type, FunctionIrError> {
        match value {
            FunctionOperand::Root(root) => self.root_type(root),
            FunctionOperand::Converted(context) => self
                .contexts()
                .get(context)
                .map(|c| &c.expected)
                .ok_or(FunctionIrError::Invalid("context index")),
        }
    }
    fn declaration(
        &self,
        id: ScopedDeclarationId,
    ) -> Result<&'a super::ScopedDeclaration, FunctionIrError> {
        self.bindings()
            .declaration(id)
            .ok_or(FunctionIrError::Invalid("foreign declaration"))
    }
    /// Check expression proof, static targets/types and reachable definite availability/scope/tag flow.
    pub fn validate(
        &self,
        limits: FunctionIrLimits,
    ) -> Result<FunctionValidation, FunctionIrError> {
        self.expressions
            .validate(limits.max_expression_validation_steps)
            .map_err(FunctionIrError::Expression)?;
        if self.instructions.is_empty() || self.instructions.len() > limits.max_instructions {
            return Err(FunctionIrError::Limit("instructions"));
        }
        let index = Index::new(self.expressions)?;
        let mut budget = Budget {
            limits,
            steps: 0,
            state_items: 0,
        };
        let mut roots = HashSet::new();
        let mut contexts = HashSet::new();
        let mut declarations = HashSet::new();
        let mut snapshots = HashSet::new();
        let mut scopes = HashSet::new();
        let mut edges = 0usize;
        let params: Vec<_> = self
            .bindings()
            .declarations()
            .iter()
            .filter(|d| d.kind == ScopedDeclarationKind::Parameter)
            .map(|d| d.id)
            .collect();
        for instruction in &self.instructions {
            budget.step()?;
            let target = |target: usize| -> Result<(), FunctionIrError> {
                if target >= self.instructions.len() {
                    Err(FunctionIrError::Invalid("jump target"))
                } else {
                    Ok(())
                }
            };
            match instruction {
                FunctionInstruction::EnterScope(scope) => {
                    if !scopes.insert(*scope) {
                        return Err(FunctionIrError::Invalid("duplicate scope"));
                    }
                }
                FunctionInstruction::Evaluate { root } => {
                    self.root_type(*root)?;
                    if !roots.insert(*root) {
                        return Err(FunctionIrError::Invalid("duplicate root evaluation site"));
                    }
                }
                FunctionInstruction::Convert { context, root } => {
                    let fact = self
                        .contexts()
                        .get(*context)
                        .ok_or(FunctionIrError::Invalid("context index"))?;
                    if !index.context_root.contains_key(context)
                        || index.context_root.get(context) != Some(root)
                        || self.root_type(*root)? != &fact.actual
                        || !contexts.insert(*context)
                    {
                        return Err(FunctionIrError::Invalid("boundary context"));
                    }
                }
                FunctionInstruction::Parameter { index, declaration } => {
                    if params.get(*index) != Some(declaration) || !declarations.insert(*declaration)
                    {
                        return Err(FunctionIrError::Invalid("parameter binding"));
                    }
                }
                FunctionInstruction::Bind { declaration, value } => {
                    if !matches!(
                        self.declaration(*declaration)?.kind,
                        ScopedDeclarationKind::Local { .. }
                    ) || self.declaration(*declaration)?.ty != *self.value_type(*value)?
                        || !declarations.insert(*declaration)
                    {
                        return Err(FunctionIrError::Invalid("local binding type"));
                    }
                }
                FunctionInstruction::Store { declaration, value } => {
                    if self.declaration(*declaration)?.kind
                        != (ScopedDeclarationKind::Local { mutable: true })
                        || self.declaration(*declaration)?.ty != *self.value_type(*value)?
                    {
                        return Err(FunctionIrError::Invalid("store permission/type"));
                    }
                }
                FunctionInstruction::StoreIndex {
                    declaration,
                    array,
                    index,
                    value,
                } => {
                    let decl = self.declaration(*declaration)?;
                    let Type::Array(element) = &decl.ty else {
                        return Err(FunctionIrError::Invalid("array binding"));
                    };
                    if decl.kind != (ScopedDeclarationKind::Local { mutable: true })
                        || self.root_type(*array)? != &decl.ty
                        || self.root_type(*index)? != &Type::Int
                        || element.as_ref() != self.value_type(*value)?
                    {
                        return Err(FunctionIrError::Invalid("indexed store type"));
                    }
                }
                FunctionInstruction::BranchFalse {
                    condition,
                    target: t,
                } => {
                    target(*t)?;
                    if self.root_type(*condition)? != &Type::Bool {
                        return Err(FunctionIrError::Invalid("condition type"));
                    }
                    edges = edges
                        .checked_add(2)
                        .ok_or(FunctionIrError::Limit("edges"))?;
                }
                FunctionInstruction::Jump(t) => {
                    target(*t)?;
                    edges = edges
                        .checked_add(1)
                        .ok_or(FunctionIrError::Limit("edges"))?;
                }
                FunctionInstruction::Switch {
                    index: match_index,
                    enum_id,
                    arms,
                } => {
                    let plans = self
                        .expressions
                        .mapping()
                        .checked()
                        .typed()
                        .callable()
                        .function()
                        .matches();
                    let plan = plans
                        .get(*match_index)
                        .ok_or(FunctionIrError::Invalid("match index"))?;
                    let mut variants = HashSet::new();
                    if plan.enum_id() != *enum_id || arms.len() != plan.variant_order().len() {
                        return Err(FunctionIrError::Invalid("switch enum/coverage"));
                    }
                    for (variant, t) in arms {
                        budget.step()?;
                        target(*t)?;
                        if !plan.variant_order().contains(variant) || !variants.insert(*variant) {
                            return Err(FunctionIrError::Invalid("switch variants"));
                        }
                    }
                    edges = edges
                        .checked_add(arms.len())
                        .ok_or(FunctionIrError::Limit("edges"))?;
                }
                FunctionInstruction::SnapshotMatch {
                    index: match_index,
                    declaration,
                } => {
                    let usage = index
                        .scrutinees
                        .get(match_index)
                        .ok_or(FunctionIrError::Invalid("match binding"))?;
                    if usage != declaration || !snapshots.insert(*match_index) {
                        return Err(FunctionIrError::Invalid("match snapshot source"));
                    }
                }
                FunctionInstruction::Payload {
                    index,
                    variant,
                    declaration,
                } => {
                    if self.declaration(*declaration)?.kind
                        != (ScopedDeclarationKind::Payload {
                            match_index: *index,
                            variant: *variant,
                        })
                        || !declarations.insert(*declaration)
                    {
                        return Err(FunctionIrError::Invalid("payload binding"));
                    }
                }
                FunctionInstruction::Return(value) => {
                    let ty = match value {
                        Some(value) => self.value_type(*value)?,
                        None => {
                            static UNIT: Type = Type::Unit;
                            &UNIT
                        }
                    };
                    if ty != self.return_type() {
                        return Err(FunctionIrError::Invalid("return type"));
                    }
                }
                _ => {}
            }
            if edges > limits.max_edges {
                return Err(FunctionIrError::Limit("edges"));
            }
        }
        if roots.len() != self.expressions.roots().len()
            || contexts.len() != self.expressions.deferred_contexts().len()
            || declarations.len() != self.bindings().declarations().len()
            || snapshots.len()
                != self
                    .expressions
                    .mapping()
                    .checked()
                    .typed()
                    .callable()
                    .function()
                    .matches()
                    .len()
        {
            return Err(FunctionIrError::Invalid("source coverage"));
        }
        let mut states: Vec<Option<State>> = vec![None; self.instructions.len()];
        states[0] = Some(State::default());
        let mut pending = VecDeque::from([0]);
        while let Some(pc) = pending.pop_front() {
            budget.step()?;
            let stored = states[pc]
                .as_ref()
                .ok_or(FunctionIrError::Invalid("flow state"))?;
            budget.copy(stored)?;
            let mut state = stored.clone();
            let mut successors = Vec::new();
            let mut ordinary = true;
            let available = |state: &State, value: FunctionOperand| -> bool {
                match value {
                    FunctionOperand::Root(r) => state.roots.contains(&r),
                    FunctionOperand::Converted(c) => state.converted.contains(&c),
                }
            };
            match &self.instructions[pc] {
                FunctionInstruction::EnterScope(scope) => {
                    if state.scopes.len() >= limits.max_scope_frames {
                        return Err(FunctionIrError::Limit("scope frames"));
                    }
                    state.scopes.push((*scope, HashSet::new()));
                }
                FunctionInstruction::ExitScope(scope) => {
                    if state.scopes.pop().map(|s| s.0) != Some(*scope) {
                        return Err(FunctionIrError::Invalid("scope exit"));
                    }
                }
                FunctionInstruction::Parameter { declaration, .. }
                | FunctionInstruction::Bind { declaration, .. }
                | FunctionInstruction::Payload { declaration, .. } => {
                    if state.bound(*declaration) {
                        return Err(FunctionIrError::Invalid("binding already active"));
                    }
                    if let FunctionInstruction::Bind { value, .. } = &self.instructions[pc] {
                        if !available(&state, *value) {
                            return Err(FunctionIrError::Invalid("initializer availability"));
                        }
                    }
                    if let FunctionInstruction::Payload { index, variant, .. } =
                        &self.instructions[pc]
                    {
                        if state.tags.get(index) != Some(variant)
                            || !state.snapshots.contains(index)
                        {
                            return Err(FunctionIrError::Invalid("unguarded payload"));
                        }
                    }
                    state
                        .scopes
                        .last_mut()
                        .ok_or(FunctionIrError::Invalid("missing scope"))?
                        .1
                        .insert(*declaration);
                }
                FunctionInstruction::Evaluate { root } => {
                    for instruction in &self.expressions.instructions()
                        [index.ranges[*root].0..index.ranges[*root].1]
                    {
                        budget.step()?;
                        if let super::ExpressionOperation::Read { declaration, .. } =
                            instruction.operation
                        {
                            if !state.bound(declaration) {
                                return Err(FunctionIrError::Invalid("read before binding"));
                            }
                        }
                    }
                    state.roots.insert(*root);
                    state.live_roots.insert(*root);
                }
                FunctionInstruction::Convert { context, root } => {
                    if !state.roots.contains(root) {
                        return Err(FunctionIrError::Invalid("conversion before evaluation"));
                    }
                    state.converted.insert(*context);
                    state.live_converted.insert(*context);
                }
                FunctionInstruction::Store { declaration, value } => {
                    if !state.bound(*declaration) || !available(&state, *value) {
                        return Err(FunctionIrError::Invalid("store availability"));
                    }
                }
                FunctionInstruction::StoreIndex {
                    declaration,
                    array,
                    index,
                    value,
                } => {
                    if !state.bound(*declaration)
                        || !state.roots.contains(array)
                        || !state.roots.contains(index)
                        || !available(&state, *value)
                    {
                        return Err(FunctionIrError::Invalid("indexed store availability"));
                    }
                }
                FunctionInstruction::Release { root } => {
                    if !state.roots.remove(root) {
                        return Err(FunctionIrError::Invalid("release before evaluation"));
                    }
                    state
                        .converted
                        .retain(|c| index.context_root.get(c) != Some(root));
                    state.live_roots.remove(root);
                    state
                        .live_converted
                        .retain(|c| index.context_root.get(c) != Some(root));
                }
                FunctionInstruction::SnapshotMatch { index, declaration } => {
                    if !state.bound(*declaration) || !state.snapshots.insert(*index) {
                        return Err(FunctionIrError::Invalid("snapshot availability"));
                    }
                    state.live_snapshots.insert(*index);
                }
                FunctionInstruction::Switch { index, arms, .. } => {
                    if !state.snapshots.contains(index) {
                        return Err(FunctionIrError::Invalid("switch before snapshot"));
                    }
                    ordinary = false;
                    for (variant, target) in arms {
                        successors.push((*target, Some((*index, *variant))));
                    }
                }
                FunctionInstruction::ReleaseMatch(index) => {
                    if !state.snapshots.remove(index) {
                        return Err(FunctionIrError::Invalid("match release"));
                    }
                    state.tags.remove(index);
                    state.live_snapshots.remove(index);
                }
                FunctionInstruction::BranchFalse { condition, target } => {
                    if !state.roots.remove(condition) {
                        return Err(FunctionIrError::Invalid("condition availability"));
                    }
                    state.live_roots.remove(condition);
                    state
                        .converted
                        .retain(|c| index.context_root.get(c) != Some(condition));
                    state
                        .live_converted
                        .retain(|c| index.context_root.get(c) != Some(condition));
                    successors.push((*target, None));
                }
                FunctionInstruction::Jump(target) => {
                    ordinary = false;
                    successors.push((*target, None));
                }
                FunctionInstruction::Return(value) => {
                    if value.is_some_and(|v| !available(&state, v)) {
                        return Err(FunctionIrError::Invalid("return availability"));
                    }
                    ordinary = false;
                }
                FunctionInstruction::End => {
                    if self.return_type() != &Type::Unit
                        || !state.scopes.is_empty()
                        || !state.live_snapshots.is_empty()
                        || !state.live_roots.is_empty()
                        || !state.live_converted.is_empty()
                    {
                        return Err(FunctionIrError::Invalid("fallthrough/cleanup"));
                    }
                    ordinary = false;
                }
            }
            if ordinary {
                let next = pc
                    .checked_add(1)
                    .filter(|n| *n < self.instructions.len())
                    .ok_or(FunctionIrError::Invalid("unterminated function"))?;
                successors.push((next, None));
            }
            for (target, tag) in successors {
                budget.step()?;
                budget.copy(&state)?;
                let mut next = state.clone();
                if let Some((index, variant)) = tag {
                    next.tags.insert(index, variant);
                }
                if let Some(old) = &mut states[target] {
                    if old.merge(&next)? {
                        pending.push_back(target);
                    }
                } else {
                    states[target] = Some(next);
                    pending.push_back(target);
                }
            }
        }
        Ok(FunctionValidation {
            steps: budget.steps,
            state_items: budget.state_items,
        })
    }
}
#[derive(Debug, Clone, Default)]
struct State {
    scopes: Vec<(usize, HashSet<ScopedDeclarationId>)>,
    roots: HashSet<usize>,
    converted: HashSet<usize>,
    snapshots: HashSet<usize>,
    live_roots: HashSet<usize>,
    live_converted: HashSet<usize>,
    live_snapshots: HashSet<usize>,
    tags: HashMap<usize, usize>,
}
impl State {
    fn bound(&self, id: ScopedDeclarationId) -> bool {
        self.scopes.iter().rev().any(|s| s.1.contains(&id))
    }
    fn merge(&mut self, other: &Self) -> Result<bool, FunctionIrError> {
        if self
            .scopes
            .iter()
            .map(|s| s.0)
            .ne(other.scopes.iter().map(|s| s.0))
        {
            return Err(FunctionIrError::Invalid("scope merge"));
        }
        let mut changed = false;
        for (a, b) in self.scopes.iter_mut().zip(&other.scopes) {
            let before = a.1.len();
            a.1.retain(|v| b.1.contains(v));
            changed |= before != a.1.len();
        }
        for (a, b) in [
            (&mut self.roots, &other.roots),
            (&mut self.converted, &other.converted),
            (&mut self.snapshots, &other.snapshots),
        ] {
            let before = a.len();
            a.retain(|v| b.contains(v));
            changed |= before != a.len();
        }
        for (a, b) in [
            (&mut self.live_roots, &other.live_roots),
            (&mut self.live_converted, &other.live_converted),
            (&mut self.live_snapshots, &other.live_snapshots),
        ] {
            let before = a.len();
            a.extend(b.iter().copied());
            changed |= before != a.len();
        }
        let before = self.tags.len();
        self.tags.retain(|k, v| other.tags.get(k) == Some(v));
        changed |= before != self.tags.len();
        Ok(changed)
    }

    fn items(&self) -> usize {
        1 + self.scopes.len()
            + self.scopes.iter().map(|s| s.1.len()).sum::<usize>()
            + self.roots.len()
            + self.converted.len()
            + self.snapshots.len()
            + self.tags.len()
            + self.live_roots.len()
            + self.live_converted.len()
            + self.live_snapshots.len()
    }
}
struct Budget {
    limits: FunctionIrLimits,
    steps: usize,
    state_items: usize,
}
impl Budget {
    fn step(&mut self) -> Result<(), FunctionIrError> {
        if self.steps >= self.limits.max_validation_steps {
            return Err(FunctionIrError::Limit("validation steps"));
        }
        self.steps += 1;
        Ok(())
    }
    fn copy(&mut self, state: &State) -> Result<(), FunctionIrError> {
        self.state_items = self
            .state_items
            .checked_add(state.items())
            .filter(|n| *n <= self.limits.max_state_items)
            .ok_or(FunctionIrError::Limit("validation state items"))?;
        Ok(())
    }
}
struct Index<'a> {
    expressions: &'a ExpressionIr<'a>,
    roots: HashMap<usize, usize>,
    statements: HashMap<usize, MappedNodeId>,
    expression_nodes: HashMap<usize, MappedNodeId>,
    declarations: HashMap<MappedNodeId, ScopedDeclarationId>,
    uses: HashMap<MappedNodeId, ScopedDeclarationId>,
    contexts: HashMap<(usize, CoercionKind), usize>,
    context_root: HashMap<usize, usize>,
    scrutinees: HashMap<usize, ScopedDeclarationId>,
    payloads: HashMap<(usize, usize), ScopedDeclarationId>,
    ranges: Vec<(usize, usize)>,
}
impl<'a> Index<'a> {
    fn new(expressions: &'a ExpressionIr<'a>) -> Result<Self, FunctionIrError> {
        let mut index = Self {
            expressions,
            roots: HashMap::new(),
            statements: HashMap::new(),
            expression_nodes: HashMap::new(),
            declarations: HashMap::new(),
            uses: HashMap::new(),
            contexts: HashMap::new(),
            context_root: HashMap::new(),
            scrutinees: HashMap::new(),
            payloads: HashMap::new(),
            ranges: Vec::new(),
        };
        let map = expressions.mapping();
        let root_sources: HashMap<_, _> = expressions
            .roots()
            .iter()
            .enumerate()
            .map(|(i, r)| (r.source, i))
            .collect();
        let mut ownership = HashMap::new();
        for node in map.nodes() {
            if matches!(
                node.syntax,
                MappedSyntax::Expression {
                    role: super::MappedExpressionRole::Value,
                    ..
                }
            ) {
                let owner = root_sources
                    .get(&node.id)
                    .copied()
                    .or_else(|| node.parent.and_then(|p| ownership.get(&p).copied()))
                    .ok_or(FunctionIrError::Invalid("expression root ownership"))?;
                ownership.insert(node.id, owner);
            }
        }
        let mut start = 0;
        for (root, r) in expressions.roots().iter().enumerate() {
            let node = map
                .node(r.source)
                .ok_or(FunctionIrError::Invalid("root source"))?;
            let MappedSyntax::Expression { syntax, .. } = node.syntax else {
                return Err(FunctionIrError::Invalid("root syntax"));
            };
            index
                .roots
                .insert(syntax as *const Expression as usize, root);
            let end = r
                .result
                .ordinal()
                .checked_add(1)
                .ok_or(FunctionIrError::Invalid("root range"))?;
            if end <= start || end > expressions.instructions().len() {
                return Err(FunctionIrError::Invalid("root range"));
            }
            if expressions.values()[start..end]
                .iter()
                .any(|v| ownership.get(&v.source) != Some(&root))
            {
                return Err(FunctionIrError::Invalid("interleaved expression roots"));
            }
            index.ranges.push((start, end));
            start = end;
        }
        if start != expressions.instructions().len() {
            return Err(FunctionIrError::Invalid("expression segment coverage"));
        }
        for node in map.nodes() {
            match node.syntax {
                MappedSyntax::Statement(s) => {
                    index
                        .statements
                        .insert(s as *const Statement as usize, node.id);
                }
                MappedSyntax::Expression { syntax, .. } => {
                    index
                        .expression_nodes
                        .insert(syntax as *const Expression as usize, node.id);
                }
                _ => {}
            }
        }
        let bindings = map.checked().bindings();
        for link in map.links() {
            match link.fact {
                MappedFact::Declaration(id) => {
                    index.declarations.insert(link.node, id);
                    let decl = bindings
                        .declaration(id)
                        .ok_or(FunctionIrError::Invalid("declaration"))?;
                    if let ScopedDeclarationKind::Payload {
                        match_index,
                        variant,
                    } = decl.kind
                    {
                        index.payloads.insert((match_index, variant), id);
                    }
                }
                MappedFact::Use(id) => {
                    let usage = bindings.usage(id).ok_or(FunctionIrError::Invalid("use"))?;
                    if let ScopedUseKind::MatchScrutinee { match_index } = usage.kind {
                        index.scrutinees.insert(match_index, usage.declaration);
                    } else if matches!(
                        usage.kind,
                        ScopedUseKind::Write | ScopedUseKind::IndexedWrite
                    ) {
                        index.uses.insert(link.node, usage.declaration);
                    }
                }
                _ => {}
            }
        }
        let root_nodes: HashMap<_, _> = expressions
            .roots()
            .iter()
            .enumerate()
            .map(|(i, r)| (r.source, i))
            .collect();
        let deferred: HashSet<_> = expressions.deferred_contexts().iter().copied().collect();
        for link in map.links() {
            if let MappedFact::Context(context) = link.fact {
                if deferred.contains(&context) {
                    let c = expressions
                        .mapping()
                        .checked()
                        .typed()
                        .callable()
                        .evidence()
                        .contexts()
                        .get(context)
                        .ok_or(FunctionIrError::Invalid("context"))?;
                    let root = *root_nodes
                        .get(&link.node)
                        .ok_or(FunctionIrError::Invalid("boundary root"))?;
                    if index.contexts.insert((root, c.kind), context).is_some() {
                        return Err(FunctionIrError::Invalid("ambiguous boundary context"));
                    }
                    index.context_root.insert(context, root);
                }
            }
        }
        Ok(index)
    }
    fn root(&self, value: &Expression) -> Result<usize, FunctionIrError> {
        self.roots
            .get(&(value as *const Expression as usize))
            .copied()
            .ok_or(FunctionIrError::Invalid("expression root"))
    }
}
struct Emitter<'a> {
    ir: FunctionIr<'a>,
    index: Index<'a>,
    limits: FunctionIrLimits,
    edges: usize,
    scope: usize,
    depth: usize,
}
impl<'a> Emitter<'a> {
    fn push(&mut self, op: FunctionInstruction) -> Result<usize, FunctionIrError> {
        if self.ir.instructions.len() >= self.limits.max_instructions {
            return Err(FunctionIrError::Limit("instructions"));
        }
        let n = match &op {
            FunctionInstruction::Jump(_) => 1,
            FunctionInstruction::BranchFalse { .. } => 2,
            FunctionInstruction::Switch { arms, .. } => arms.len(),
            _ => 0,
        };
        self.edges = self
            .edges
            .checked_add(n)
            .filter(|n| *n <= self.limits.max_edges)
            .ok_or(FunctionIrError::Limit("edges"))?;
        let pc = self.ir.instructions.len();
        self.ir.instructions.push(op);
        Ok(pc)
    }
    fn enter(&mut self) -> Result<usize, FunctionIrError> {
        if self.depth >= self.limits.max_scope_frames {
            return Err(FunctionIrError::Limit("scope frames"));
        }
        let scope = self.scope;
        self.scope += 1;
        self.depth += 1;
        self.push(FunctionInstruction::EnterScope(scope))?;
        Ok(scope)
    }
    fn exit(&mut self, scope: usize) -> Result<(), FunctionIrError> {
        self.push(FunctionInstruction::ExitScope(scope))?;
        self.depth -= 1;
        Ok(())
    }
    fn evaluate(&mut self, value: &Expression) -> Result<usize, FunctionIrError> {
        let root = self.index.root(value)?;
        self.push(FunctionInstruction::Evaluate { root })?;
        Ok(root)
    }
    fn convert(
        &mut self,
        root: usize,
        kind: CoercionKind,
    ) -> Result<FunctionOperand, FunctionIrError> {
        let context = *self
            .index
            .contexts
            .get(&(root, kind))
            .ok_or(FunctionIrError::Invalid("required boundary context"))?;
        self.push(FunctionInstruction::Convert { context, root })?;
        Ok(FunctionOperand::Converted(context))
    }
    fn release(&mut self, root: usize) -> Result<(), FunctionIrError> {
        self.push(FunctionInstruction::Release { root })?;
        Ok(())
    }
    fn patch(&mut self, pc: usize, target: usize) -> Result<(), FunctionIrError> {
        match self.ir.instructions.get_mut(pc) {
            Some(FunctionInstruction::Jump(t))
            | Some(FunctionInstruction::BranchFalse { target: t, .. }) => *t = target,
            _ => return Err(FunctionIrError::Invalid("patch site")),
        }
        Ok(())
    }
    fn block(&mut self, body: &[Statement]) -> Result<(), FunctionIrError> {
        let scope = self.enter()?;
        for s in body {
            self.statement(s)?;
        }
        self.exit(scope)
    }
    fn statement(&mut self, s: &Statement) -> Result<(), FunctionIrError> {
        match s {
            Statement::Let { value, .. } => {
                let root = self.evaluate(value)?;
                let value = self.convert(root, CoercionKind::Initialization)?;
                let node = self.index.statements[&(s as *const Statement as usize)];
                let declaration = self.index.declarations[&node];
                self.push(FunctionInstruction::Bind { declaration, value })?;
                self.release(root)?;
            }
            Statement::Assign { target, value, .. } => {
                let node = self.index.expression_nodes[&(target as *const Expression as usize)];
                let declaration = self.index.uses[&node];
                if let crate::compiler::ast::ExpressionKind::Index { target, index } = &target.kind
                {
                    let array = self.evaluate(target)?;
                    let index = self.evaluate(index)?;
                    let root = self.evaluate(value)?;
                    let value = self.convert(root, CoercionKind::IndexedAssignment)?;
                    self.push(FunctionInstruction::StoreIndex {
                        declaration,
                        array,
                        index,
                        value,
                    })?;
                    self.release(array)?;
                    self.release(index)?;
                    self.release(root)?;
                } else {
                    let root = self.evaluate(value)?;
                    let value = self.convert(root, CoercionKind::Assignment)?;
                    self.push(FunctionInstruction::Store { declaration, value })?;
                    self.release(root)?;
                }
            }
            Statement::Return { value, .. } => {
                let value = if let Some(value) = value {
                    let root = self.evaluate(value)?;
                    Some(self.convert(root, CoercionKind::Return)?)
                } else {
                    None
                };
                self.push(FunctionInstruction::Return(value))?;
            }
            Statement::Expression(value) => {
                let root = self.evaluate(value)?;
                self.release(root)?;
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let condition = self.evaluate(condition)?;
                let branch = self.push(FunctionInstruction::BranchFalse {
                    condition,
                    target: usize::MAX,
                })?;
                self.block(then_block)?;
                if let Some(body) = else_block {
                    let jump = self.push(FunctionInstruction::Jump(usize::MAX))?;
                    self.patch(branch, self.ir.instructions.len())?;
                    self.block(body)?;
                    self.patch(jump, self.ir.instructions.len())?;
                } else {
                    self.patch(branch, self.ir.instructions.len())?;
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                let start = self.ir.instructions.len();
                let condition = self.evaluate(condition)?;
                let branch = self.push(FunctionInstruction::BranchFalse {
                    condition,
                    target: usize::MAX,
                })?;
                self.block(body)?;
                self.push(FunctionInstruction::Jump(start))?;
                self.patch(branch, self.ir.instructions.len())?;
            }
        }
        Ok(())
    }
    fn extended(
        &mut self,
        body: &[EnumFunctionStatement],
        match_index: &mut usize,
    ) -> Result<(), FunctionIrError> {
        for s in body {
            match s {
                EnumFunctionStatement::Core(s) => self.statement(s)?,
                EnumFunctionStatement::Control(s) => self.control(s, match_index)?,
                EnumFunctionStatement::Match(s) => self.match_body(s, None, match_index)?,
                EnumFunctionStatement::RecursiveMatch(s) => {
                    self.match_body(&s.header, Some(&s.bodies), match_index)?
                }
            }
        }
        Ok(())
    }
    fn extended_block(
        &mut self,
        body: &[EnumFunctionStatement],
        index: &mut usize,
    ) -> Result<(), FunctionIrError> {
        let scope = self.enter()?;
        self.extended(body, index)?;
        self.exit(scope)
    }
    fn control(
        &mut self,
        s: &super::RecursiveControlSyntax,
        index: &mut usize,
    ) -> Result<(), FunctionIrError> {
        match &s.header {
            Statement::If {
                condition,
                else_block,
                ..
            } => {
                if s.bodies.len() != if else_block.is_some() { 2 } else { 1 } {
                    return Err(FunctionIrError::Invalid("control bodies"));
                }
                let condition = self.evaluate(condition)?;
                let branch = self.push(FunctionInstruction::BranchFalse {
                    condition,
                    target: usize::MAX,
                })?;
                self.extended_block(&s.bodies[0], index)?;
                if else_block.is_some() {
                    let jump = self.push(FunctionInstruction::Jump(usize::MAX))?;
                    self.patch(branch, self.ir.instructions.len())?;
                    self.extended_block(&s.bodies[1], index)?;
                    self.patch(jump, self.ir.instructions.len())?;
                } else {
                    self.patch(branch, self.ir.instructions.len())?;
                }
            }
            Statement::While { condition, .. } => {
                if s.bodies.len() != 1 {
                    return Err(FunctionIrError::Invalid("control bodies"));
                }
                let start = self.ir.instructions.len();
                let condition = self.evaluate(condition)?;
                let branch = self.push(FunctionInstruction::BranchFalse {
                    condition,
                    target: usize::MAX,
                })?;
                self.extended_block(&s.bodies[0], index)?;
                self.push(FunctionInstruction::Jump(start))?;
                self.patch(branch, self.ir.instructions.len())?;
            }
            _ => return Err(FunctionIrError::Invalid("control header")),
        }
        Ok(())
    }
    fn match_body(
        &mut self,
        s: &super::MatchStatementSyntax,
        bodies: Option<&[Vec<EnumFunctionStatement>]>,
        match_index: &mut usize,
    ) -> Result<(), FunctionIrError> {
        let index = *match_index;
        *match_index += 1;
        let declaration = self.index.scrutinees[&index];
        self.push(FunctionInstruction::SnapshotMatch { index, declaration })?;
        let plan = &self
            .index
            .expressions
            .mapping()
            .checked()
            .typed()
            .callable()
            .function()
            .matches()[index];
        if self
            .edges
            .checked_add(s.arms.len())
            .filter(|n| *n <= self.limits.max_edges)
            .is_none()
        {
            return Err(FunctionIrError::Limit("edges"));
        }
        let pc = self.push(FunctionInstruction::Switch {
            index,
            enum_id: plan.enum_id(),
            arms: vec![(0, usize::MAX); s.arms.len()],
        })?;
        let mut jumps = Vec::new();
        for (arm_index, (arm, variant)) in s.arms.iter().zip(plan.variant_order()).enumerate() {
            let target = self.ir.instructions.len();
            let scope = self.enter()?;
            if arm.binding.is_some() {
                let declaration = self.index.payloads[&(index, *variant)];
                self.push(FunctionInstruction::Payload {
                    index,
                    variant: *variant,
                    declaration,
                })?;
            }
            if let Some(bodies) = bodies {
                self.extended(&bodies[arm_index], match_index)?;
            } else {
                for statement in &arm.body {
                    self.statement(statement)?;
                }
            }
            self.exit(scope)?;
            self.push(FunctionInstruction::ReleaseMatch(index))?;
            jumps.push((
                (*variant, target),
                self.push(FunctionInstruction::Jump(usize::MAX))?,
            ));
        }
        let end = self.ir.instructions.len();
        let arms: Vec<_> = jumps.iter().map(|(arm, _)| *arm).collect();
        for (_, jump) in jumps {
            self.patch(jump, end)?;
        }
        let FunctionInstruction::Switch { arms: targets, .. } = &mut self.ir.instructions[pc]
        else {
            return Err(FunctionIrError::Invalid("switch patch"));
        };
        *targets = arms;
        Ok(())
    }
    fn emit(&mut self) -> Result<(), FunctionIrError> {
        let root = self.enter()?;
        let checked = self.index.expressions.mapping().checked();
        for (index, d) in checked
            .bindings()
            .declarations()
            .iter()
            .filter(|d| d.kind == ScopedDeclarationKind::Parameter)
            .enumerate()
        {
            self.push(FunctionInstruction::Parameter {
                index,
                declaration: d.id,
            })?;
        }
        let function = checked.typed().callable().function();
        let mut match_index = 0;
        self.extended(&function.syntax().body, &mut match_index)?;
        if match_index != function.matches().len() {
            return Err(FunctionIrError::Invalid("unused match plans"));
        }
        self.exit(root)?;
        self.push(FunctionInstruction::End)?;
        Ok(())
    }
}
impl ExpressionIr<'_> {
    /// Emit validated stores/scopes/control flow/match dispatch/returns around expression trees.
    /// No interpreter, JavaScript/native backend or runtime resource policy is enabled here.
    pub fn emit_function_ir(
        &self,
        limits: FunctionIrLimits,
    ) -> Result<FunctionIr<'_>, FunctionIrError> {
        self.validate(limits.max_expression_validation_steps)
            .map_err(FunctionIrError::Expression)?;
        let ir = FunctionIr {
            expressions: self,
            instructions: Vec::new(),
        };
        let mut emitter = Emitter {
            ir,
            index: Index::new(self)?,
            limits,
            edges: 0,
            scope: 0,
            depth: 0,
        };
        emitter.emit()?;
        emitter.ir.validate(limits)?;
        Ok(emitter.ir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_ir(source: &str, test: impl FnOnce(FunctionIr<'_>)) {
        let context = crate::compiler::parser::Parser::new()
            .parse_source("")
            .unwrap();
        let env = crate::compiler::v2::check_enum_types(
            "o",
            "c",
            &context,
            &[crate::compiler::v2::EnumSource {
                label: "e",
                source: "enum E{V(Int),Empty}",
            }],
            crate::compiler::v2::EnumTypeLimits::DEFAULT,
        )
        .unwrap();
        let checked = env
            .check_scoped_callable_function_source(
                "f",
                source,
                crate::compiler::v2::ScopedCallableLimits::DEFAULT,
            )
            .unwrap();
        let map = checked
            .map_for_lowering(crate::compiler::v2::MappingLimits::DEFAULT)
            .unwrap();
        let expr = map
            .emit_expression_ir(crate::compiler::v2::ExpressionIrLimits::DEFAULT)
            .unwrap();
        test(expr.emit_function_ir(FunctionIrLimits::DEFAULT).unwrap());
    }
    #[test]
    fn validator_refuses_bad_targets_and_unguarded_payload_extraction() {
        with_ir(
            "fn f(input:E)->Int{match input{E::V(n)=>{return n;},E::Empty()=>{return 0;}}}",
            |mut ir| {
                let pc = ir
                    .instructions
                    .iter()
                    .position(|i| matches!(i, FunctionInstruction::Switch { .. }))
                    .unwrap();
                let FunctionInstruction::Switch { arms, .. } = &mut ir.instructions[pc] else {
                    unreachable!()
                };
                arms[0].1 = usize::MAX;
                assert!(matches!(
                    ir.validate(FunctionIrLimits::DEFAULT),
                    Err(FunctionIrError::Invalid("jump target"))
                ));
            },
        );
        with_ir(
            "fn f(input:E)->Int{match input{E::V(n)=>{return n;},E::Empty()=>{return 0;}}}",
            |mut ir| {
                let payload = ir
                    .instructions
                    .iter()
                    .position(|i| matches!(i, FunctionInstruction::Payload { .. }))
                    .unwrap();
                let switch = ir
                    .instructions
                    .iter()
                    .position(|i| matches!(i, FunctionInstruction::Switch { .. }))
                    .unwrap();
                let FunctionInstruction::Switch { arms, .. } = &mut ir.instructions[switch] else {
                    unreachable!()
                };
                let target = payload - 1;
                arms[1].1 = target;
                assert!(ir.validate(FunctionIrLimits::DEFAULT).is_err());
            },
        );
    }
    #[test]
    fn validator_refuses_conversion_before_evaluation_and_missing_active_bindings() {
        with_ir("fn f(n:Int)->Float{return n;}", |mut ir| {
            let eval = ir
                .instructions
                .iter()
                .position(|i| matches!(i, FunctionInstruction::Evaluate { .. }))
                .unwrap();
            ir.instructions.swap(eval, eval + 1);
            assert!(matches!(
                ir.validate(FunctionIrLimits::DEFAULT),
                Err(FunctionIrError::Invalid("conversion before evaluation"))
            ));
        });
        with_ir("fn f(n:Int)->Int{return n;}", |mut ir| {
            // Keep static coverage but make the parameter unreachable until after its read.
            let parameter = ir
                .instructions
                .iter()
                .position(|i| matches!(i, FunctionInstruction::Parameter { .. }))
                .unwrap();
            ir.instructions.swap(parameter, parameter + 1);
            assert!(matches!(
                ir.validate(FunctionIrLimits::DEFAULT),
                Err(FunctionIrError::Invalid("read before binding"))
            ));
        });
    }
    #[test]
    fn validator_refuses_nonunit_fallthrough_and_cleanup_disagreement() {
        with_ir("fn f()->Int{return 1;}", |mut ir| {
            let pc = ir
                .instructions
                .iter()
                .position(|i| matches!(i, FunctionInstruction::Return(_)))
                .unwrap();
            ir.instructions[pc] = FunctionInstruction::End;
            assert!(matches!(
                ir.validate(FunctionIrLimits::DEFAULT),
                Err(FunctionIrError::Invalid("fallthrough/cleanup"))
            ));
        });
        with_ir("fn f(){}", |mut ir| {
            ir.instructions[1] = FunctionInstruction::ExitScope(7);
            assert!(matches!(
                ir.validate(FunctionIrLimits::DEFAULT),
                Err(FunctionIrError::Invalid("scope exit"))
            ));
        });
    }
    #[test]
    fn validator_preserves_may_live_resources_across_branch_joins() {
        with_ir("fn f(){if true{print(1);}else{print(2);}}", |mut ir| {
            let pc = ir
                .instructions
                .iter()
                .position(|i| matches!(i, FunctionInstruction::Release { .. }))
                .unwrap();
            ir.instructions[pc] = FunctionInstruction::Jump(pc + 1);
            assert_eq!(
                ir.validate(FunctionIrLimits::DEFAULT),
                Err(FunctionIrError::Invalid("fallthrough/cleanup"))
            );
        });
    }
}
