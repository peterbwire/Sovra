//! Bounded structural source/fact mapping for subsequent versioned lowering.
use super::{
    CallableTarget, CheckedScopedCallableFunction, EnumFunctionStatement, EnumFunctionSyntax,
    EnumMatchPlan, MatchArmSyntax, MatchStatementSyntax, ScopedDeclarationId,
    ScopedDeclarationKind, ScopedUseId, ScopedUseKind, TypedExpressionId,
};
use crate::compiler::{
    ast::{Expression, ExpressionKind, Parameter, Statement},
    diagnostics::Span,
};
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicU64,
};
/// Admission for retained structural nodes and semantic fact links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingLimits {
    /// Nodes scheduled/retained, including function, statements, patterns and call/write heads.
    pub max_nodes: usize,
    /// Expression/declaration/use/call/context links retained in total.
    pub max_fact_links: usize,
}
impl MappingLimits {
    /// Opt-in finite mapping profile; previous check APIs remain unchanged.
    pub const DEFAULT: Self = Self {
        max_nodes: 1_000_000,
        max_fact_links: 2_000_000,
    };
}
/// Structural identity local to one immutable mapping, never a source-offset key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MappedNodeId {
    unit: u64,
    ordinal: usize,
}
impl MappedNodeId {
    /// Preorder structural position in this map.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Expression position; function names and write targets are not fabricated typed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappedExpressionRole {
    /// Core evaluated value, with a retained expression type.
    Value,
    /// Resolved function-name head of a call.
    CallHead,
    /// Direct assignment target.
    Write,
    /// Indexed assignment target; its base/index are evaluated child values.
    IndexedWrite,
}
/// Borrowed original syntax node; immutable checked syntax outlives its mapping.
#[derive(Debug, Clone, Copy)]
pub enum MappedSyntax<'a> {
    /// Single versioned function root.
    Function(&'a EnumFunctionSyntax),
    /// Source parameter.
    Parameter(&'a Parameter),
    /// Existing core statement, including nested branches/loops.
    Statement(&'a Statement),
    /// Match header and retained coverage; recursive bodies stay in checked syntax.
    Match {
        /// Original syntax.
        syntax: &'a MatchStatementSyntax,
        /// Preorder match ordinal within the checked function.
        index: usize,
        /// Checked nominal coverage.
        plan: &'a EnumMatchPlan,
    },
    /// Separate binding read, not a fabricated expression.
    Scrutinee {
        /// Original match syntax.
        syntax: &'a MatchStatementSyntax,
        /// Preorder match ordinal within the checked function.
        index: usize,
    },
    /// Arm with its declaration variant ordinal.
    Arm {
        /// Original syntax.
        syntax: &'a MatchArmSyntax,
        /// Preorder match ordinal within the checked function.
        match_index: usize,
        /// Checked declaration variant ordinal.
        variant: usize,
    },
    /// Immutable payload declaration, present only for a binding arm.
    Payload {
        /// Original arm syntax.
        syntax: &'a MatchArmSyntax,
        /// Preorder match ordinal within the checked function.
        match_index: usize,
        /// Checked declaration variant ordinal.
        variant: usize,
    },
    /// Original core expression and its structural role.
    Expression {
        /// Original expression.
        syntax: &'a Expression,
        /// Position/category.
        role: MappedExpressionRole,
    },
}
impl MappedSyntax<'_> {
    /// Original source range; local declarations retain the complete statement range.
    pub fn span(self) -> Span {
        match self {
            Self::Function(f) => f.span,
            Self::Parameter(p) => p.span,
            Self::Statement(s) => statement_span(s),
            Self::Match { syntax, .. } => syntax.span,
            Self::Scrutinee { syntax, .. } => syntax.scrutinee_span,
            Self::Arm { syntax, .. } => syntax.span,
            Self::Payload { syntax, .. } => syntax.binding_span.unwrap_or(syntax.span),
            Self::Expression { syntax, .. } => syntax.span,
        }
    }
}
/// One structural node with an explicit parent, in preorder.
#[derive(Debug, Clone, Copy)]
pub struct MappedNode<'a> {
    /// Mapping-local identity.
    pub id: MappedNodeId,
    /// Earlier parent node; only the function root has no parent.
    pub parent: Option<MappedNodeId>,
    /// Immutable source-owned syntax.
    pub syntax: MappedSyntax<'a>,
}
/// A retained checked fact attached to an original structural node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedFact {
    /// Core evaluated-expression identity/type fact.
    Expression(TypedExpressionId),
    /// Source value declaration.
    Declaration(ScopedDeclarationId),
    /// Source-resolved read/write/scrutinee use.
    Use(ScopedUseId),
    /// Index in the retained callable call table; attached to the call-result expression.
    Call(usize),
    /// Index in the retained callable context table; attached to the actual value.
    Context(usize),
}
/// Semantic attachment; all facts must map exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappedFactLink {
    /// Structural owner of the fact.
    pub node: MappedNodeId,
    /// Original checked fact identity/index.
    pub fact: MappedFact,
}
/// Complete structural admission/validation failure; no partial map or legacy fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingError {
    /// Structural or attachment cap was exceeded.
    Limit(&'static str),
    /// A required source location is ambiguous within the same structural role.
    AmbiguousSpan(Span),
    /// Required fact/source ownership, shape, identity or coverage was unavailable.
    Invalid(&'static str),
    /// Mapping identity counter cannot advance without reuse.
    IdentityExhausted,
}
impl std::fmt::Display for MappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "callable mapping refused: {self:?}")
    }
}
impl std::error::Error for MappingError {}
/// Borrowed structural/fact map for future versioned IR lowering; not executable IR.
#[derive(Debug, Clone)]
pub struct MappedCallable<'a> {
    checked: &'a CheckedScopedCallableFunction,
    unit: u64,
    nodes: Vec<MappedNode<'a>>,
    links: Vec<MappedFactLink>,
}
impl<'a> MappedCallable<'a> {
    /// Source/type/scope owner borrowed by this map.
    pub fn checked(&self) -> &'a CheckedScopedCallableFunction {
        self.checked
    }
    /// Structural nodes in preorder, including untyped call heads/write sites.
    pub fn nodes(&self) -> &[MappedNode<'a>] {
        &self.nodes
    }
    /// Checked semantic attachments in table order.
    pub fn links(&self) -> &[MappedFactLink] {
        &self.links
    }
    /// Refuse IDs from another mapping; clones retain their identities.
    pub fn node(&self, id: MappedNodeId) -> Option<&MappedNode<'a>> {
        if id.unit == self.unit {
            self.nodes.get(id.ordinal)
        } else {
            None
        }
    }
}
fn statement_span(s: &Statement) -> Span {
    match s {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::Return { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. } => *span,
        Statement::Expression(e) => e.span,
    }
}
type Key = (usize, usize, usize, usize);
fn key(span: Span) -> Key {
    (span.start, span.end, span.line, span.column)
}
struct Builder<'a> {
    map: MappedCallable<'a>,
    limits: MappingLimits,
    scheduled: usize,
    pending: Vec<(Option<MappedNodeId>, MappedSyntax<'a>)>,
    match_indices: HashMap<*const MatchStatementSyntax, usize>,
    recursive_bodies: HashMap<*const MatchArmSyntax, &'a [EnumFunctionStatement]>,
    controls: HashMap<*const Statement, &'a super::RecursiveControlSyntax>,
}
impl<'a> Builder<'a> {
    fn schedule(
        &mut self,
        parent: Option<MappedNodeId>,
        syntax: MappedSyntax<'a>,
    ) -> Result<(), MappingError> {
        if self.scheduled >= self.limits.max_nodes {
            return Err(MappingError::Limit("nodes"));
        }
        self.scheduled += 1;
        self.pending.push((parent, syntax));
        Ok(())
    }
    fn core(&mut self, parent: MappedNodeId, body: &'a [Statement]) -> Result<(), MappingError> {
        for s in body.iter().rev() {
            self.schedule(Some(parent), MappedSyntax::Statement(s))?;
        }
        Ok(())
    }
    fn index_matches(&mut self, body: &'a [EnumFunctionStatement]) -> Result<(), MappingError> {
        for statement in body {
            let (header, bodies) = match statement {
                EnumFunctionStatement::Core(_) => continue,
                EnumFunctionStatement::Control(s) => {
                    if self.match_indices.len() + self.recursive_bodies.len() + self.controls.len()
                        >= self.limits.max_nodes
                    {
                        return Err(MappingError::Limit("control indexes"));
                    }
                    self.controls.insert(&s.header as *const _, s);
                    for body in &s.bodies {
                        self.index_matches(body)?;
                    }
                    continue;
                }
                EnumFunctionStatement::Match(s) => (s, None),
                EnumFunctionStatement::RecursiveMatch(s) => (&s.header, Some(&s.bodies)),
            };
            if self.match_indices.len() + self.recursive_bodies.len() + self.controls.len()
                >= self.limits.max_nodes
            {
                return Err(MappingError::Limit("match indexes"));
            }
            let index = self.match_indices.len();
            self.match_indices.insert(header as *const _, index);
            if let Some(bodies) = bodies {
                if header.arms.len() != bodies.len() {
                    return Err(MappingError::Invalid("recursive arm coverage"));
                }
                for (arm, body) in header.arms.iter().zip(bodies) {
                    if self.match_indices.len() + self.recursive_bodies.len()
                        >= self.limits.max_nodes
                    {
                        return Err(MappingError::Limit("match indexes"));
                    }
                    self.recursive_bodies.insert(arm as *const _, body);
                    self.index_matches(body)?;
                }
            }
        }
        Ok(())
    }
    fn extended(
        &mut self,
        parent: MappedNodeId,
        body: &'a [EnumFunctionStatement],
    ) -> Result<(), MappingError> {
        for s in body.iter().rev() {
            let header = match s {
                EnumFunctionStatement::Control(s) => {
                    self.schedule(Some(parent), MappedSyntax::Statement(&s.header))?;
                    continue;
                }
                EnumFunctionStatement::Core(s) => {
                    self.schedule(Some(parent), MappedSyntax::Statement(s))?;
                    continue;
                }
                EnumFunctionStatement::Match(s) => s,
                EnumFunctionStatement::RecursiveMatch(s) => &s.header,
            };
            let index = *self
                .match_indices
                .get(&(header as *const _))
                .ok_or(MappingError::Invalid("match index"))?;
            let plan = self
                .map
                .checked
                .typed()
                .callable()
                .function()
                .matches()
                .get(index)
                .ok_or(MappingError::Invalid("match plan"))?;
            self.schedule(
                Some(parent),
                MappedSyntax::Match {
                    syntax: header,
                    index,
                    plan,
                },
            )?;
        }
        Ok(())
    }
    fn expr(
        &mut self,
        parent: MappedNodeId,
        e: &'a Expression,
        role: MappedExpressionRole,
    ) -> Result<(), MappingError> {
        self.schedule(Some(parent), MappedSyntax::Expression { syntax: e, role })
    }
    fn walk(&mut self) -> Result<(), MappingError> {
        use MappedExpressionRole as Role;
        while let Some((parent, syntax)) = self.pending.pop() {
            let id = MappedNodeId {
                unit: self.map.unit,
                ordinal: self.map.nodes.len(),
            };
            self.map.nodes.push(MappedNode { id, parent, syntax });
            match syntax {
                MappedSyntax::Function(f) => {
                    self.index_matches(&f.body)?;
                    if self.match_indices.len()
                        != self
                            .map
                            .checked
                            .typed()
                            .callable()
                            .function()
                            .matches()
                            .len()
                    {
                        return Err(MappingError::Invalid("match plans"));
                    }
                    self.extended(id, &f.body)?;
                    for p in f.parameters.iter().rev() {
                        self.schedule(Some(id), MappedSyntax::Parameter(p))?;
                    }
                }
                MappedSyntax::Statement(s) => {
                    if let Some(control) = self.controls.get(&(s as *const _)).copied() {
                        for body in control.bodies.iter().rev() {
                            self.extended(id, body)?;
                        }
                        let condition = match &control.header {
                            Statement::If { condition, .. }
                            | Statement::While { condition, .. } => condition,
                            _ => return Err(MappingError::Invalid("control header")),
                        };
                        self.expr(id, condition, Role::Value)?;
                        continue;
                    }
                    match s {
                        Statement::Let { value, .. } | Statement::Expression(value) => {
                            self.expr(id, value, Role::Value)?
                        }
                        Statement::Assign { target, value, .. } => {
                            self.expr(id, value, Role::Value)?;
                            let role = match target.kind {
                                ExpressionKind::Identifier(_) => Role::Write,
                                ExpressionKind::Index { .. } => Role::IndexedWrite,
                                _ => return Err(MappingError::Invalid("write shape")),
                            };
                            self.expr(id, target, role)?;
                        }
                        Statement::Return { value, .. } => {
                            if let Some(value) = value {
                                self.expr(id, value, Role::Value)?;
                            }
                        }
                        Statement::If {
                            condition,
                            then_block,
                            else_block,
                            ..
                        } => {
                            if let Some(body) = else_block {
                                self.core(id, body)?;
                            }
                            self.core(id, then_block)?;
                            self.expr(id, condition, Role::Value)?;
                        }
                        Statement::While {
                            condition, body, ..
                        } => {
                            self.core(id, body)?;
                            self.expr(id, condition, Role::Value)?;
                        }
                    }
                }
                MappedSyntax::Match {
                    syntax,
                    index,
                    plan,
                } => {
                    if syntax.arms.len() != plan.variant_order().len() {
                        return Err(MappingError::Invalid("arm coverage"));
                    }
                    for (arm, variant) in syntax.arms.iter().zip(plan.variant_order()).rev() {
                        self.schedule(
                            Some(id),
                            MappedSyntax::Arm {
                                syntax: arm,
                                match_index: index,
                                variant: *variant,
                            },
                        )?;
                    }
                    self.schedule(Some(id), MappedSyntax::Scrutinee { syntax, index })?;
                }
                MappedSyntax::Arm {
                    syntax,
                    match_index,
                    variant,
                } => {
                    if let Some(body) = self.recursive_bodies.get(&(syntax as *const _)).copied() {
                        self.extended(id, body)?;
                    } else {
                        self.core(id, &syntax.body)?;
                    }
                    if syntax.binding.is_some() {
                        self.schedule(
                            Some(id),
                            MappedSyntax::Payload {
                                syntax,
                                match_index,
                                variant,
                            },
                        )?;
                    }
                }
                MappedSyntax::Expression { syntax, role } => match (&syntax.kind, role) {
                    (_, Role::CallHead | Role::Write) => {}
                    (ExpressionKind::Call { callee, arguments }, Role::Value) => {
                        for arg in arguments.iter().rev() {
                            self.expr(id, arg, Role::Value)?;
                        }
                        self.expr(id, callee, Role::CallHead)?;
                    }
                    (ExpressionKind::Binary { left, right, .. }, Role::Value) => {
                        self.expr(id, right, Role::Value)?;
                        self.expr(id, left, Role::Value)?;
                    }
                    (ExpressionKind::Index { target, index }, _) => {
                        self.expr(id, index, Role::Value)?;
                        self.expr(id, target, Role::Value)?;
                    }
                    (ExpressionKind::ArrayLiteral(items), Role::Value) => {
                        for item in items.iter().rev() {
                            self.expr(id, item, Role::Value)?;
                        }
                    }
                    (ExpressionKind::StructLiteral { fields, .. }, Role::Value) => {
                        for (_, value) in fields.iter().rev() {
                            self.expr(id, value, Role::Value)?;
                        }
                    }
                    (ExpressionKind::FieldAccess { receiver, .. }, Role::Value) => {
                        self.expr(id, receiver, Role::Value)?
                    }
                    (_, Role::IndexedWrite) => {
                        return Err(MappingError::Invalid("indexed write shape"))
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }
    fn link(&mut self, node: MappedNodeId, fact: MappedFact) -> Result<(), MappingError> {
        if self.map.links.len() >= self.limits.max_fact_links {
            return Err(MappingError::Limit("fact links"));
        }
        self.map.links.push(MappedFactLink { node, fact });
        Ok(())
    }
    fn validate(&mut self) -> Result<(), MappingError> {
        use MappedExpressionRole as Role;
        let owner = self.map.checked;
        let typed = owner.typed().types();
        let calls = owner.typed().callable().evidence();
        let scope = owner.bindings();
        let mut values = HashMap::new();
        let mut declarations = HashMap::new();
        let mut writes = HashMap::new();
        let mut scrutinees = HashMap::new();
        for node in &self.map.nodes {
            let location = key(node.syntax.span());
            let duplicate = match node.syntax {
                MappedSyntax::Expression {
                    role: Role::Value, ..
                } => values.insert(location, node.id).is_some(),
                MappedSyntax::Expression {
                    role: role @ (Role::Write | Role::IndexedWrite),
                    ..
                } => writes.insert((location, role), node.id).is_some(),
                MappedSyntax::Parameter(_)
                | MappedSyntax::Payload { .. }
                | MappedSyntax::Statement(Statement::Let { .. }) => {
                    declarations.insert(location, node.id).is_some()
                }
                MappedSyntax::Scrutinee { index, .. } => {
                    scrutinees.insert(index, node.id).is_some()
                }
                _ => false,
            };
            if duplicate {
                return Err(MappingError::AmbiguousSpan(node.syntax.span()));
            }
        }
        let mut expression_nodes = HashMap::new();
        let mut used = HashSet::new();
        for fact in typed.expressions() {
            let node = *values
                .get(&key(fact.span))
                .ok_or(MappingError::Invalid("expression source"))?;
            if !used.insert(node) || expression_nodes.insert(fact.id, node).is_some() {
                return Err(MappingError::Invalid("duplicate expression"));
            }
            self.link(node, MappedFact::Expression(fact.id))?;
        }
        if used.len() != values.len() {
            return Err(MappingError::Invalid("untyped value expression"));
        }
        used.clear();
        for decl in scope.declarations() {
            let node = *declarations
                .get(&key(decl.span))
                .ok_or(MappingError::Invalid("declaration source"))?;
            let matches = match (self.map.nodes[node.ordinal].syntax, decl.kind) {
                (MappedSyntax::Parameter(p), ScopedDeclarationKind::Parameter) => {
                    p.name == decl.name.as_ref()
                }
                (
                    MappedSyntax::Statement(Statement::Let {
                        name, is_mutable, ..
                    }),
                    ScopedDeclarationKind::Local { mutable },
                ) => name == decl.name.as_ref() && *is_mutable == mutable,
                (
                    MappedSyntax::Payload {
                        syntax,
                        match_index,
                        variant,
                    },
                    ScopedDeclarationKind::Payload {
                        match_index: m,
                        variant: v,
                    },
                ) => {
                    match_index == m
                        && variant == v
                        && syntax.binding.as_deref() == Some(decl.name.as_ref())
                }
                _ => false,
            };
            if !matches || !used.insert(node) {
                return Err(MappingError::Invalid("declaration shape"));
            }
            self.link(node, MappedFact::Declaration(decl.id))?;
        }
        if used.len() != declarations.len() {
            return Err(MappingError::Invalid("unbound declaration"));
        }
        let mut covered_sites = HashSet::new();
        let mut read_values = HashSet::new();
        for usage in scope.uses() {
            let decl = scope
                .declaration(usage.declaration)
                .ok_or(MappingError::Invalid("foreign use target"))?;
            let node = match usage.kind {
                ScopedUseKind::Read => {
                    let value = usage.value.ok_or(MappingError::Invalid("read identity"))?;
                    let fact = typed
                        .expression(value)
                        .ok_or(MappingError::Invalid("foreign read value"))?;
                    if fact.ty != decl.ty || !read_values.insert(value) {
                        return Err(MappingError::Invalid("read type/coverage"));
                    }
                    *expression_nodes
                        .get(&value)
                        .ok_or(MappingError::Invalid("read source"))?
                }
                ScopedUseKind::Write | ScopedUseKind::IndexedWrite => {
                    if usage.value.is_some()
                        || decl.kind != (ScopedDeclarationKind::Local { mutable: true })
                    {
                        return Err(MappingError::Invalid("write permission"));
                    }
                    let role = if usage.kind == ScopedUseKind::Write {
                        Role::Write
                    } else {
                        Role::IndexedWrite
                    };
                    *writes
                        .get(&(key(usage.span), role))
                        .ok_or(MappingError::Invalid("write source"))?
                }
                ScopedUseKind::MatchScrutinee { match_index } => {
                    if usage.value.is_some() {
                        return Err(MappingError::Invalid("fabricated scrutinee value"));
                    }
                    *scrutinees
                        .get(&match_index)
                        .ok_or(MappingError::Invalid("scrutinee source"))?
                }
            };
            let name = match self.map.nodes[node.ordinal].syntax {
                MappedSyntax::Expression { syntax, .. } => match &syntax.kind {
                    ExpressionKind::Identifier(name) => Some(name.as_str()),
                    ExpressionKind::Index { target, .. } => match &target.kind {
                        ExpressionKind::Identifier(name) => Some(name.as_str()),
                        _ => None,
                    },
                    _ => None,
                },
                MappedSyntax::Scrutinee { syntax, .. } => Some(syntax.scrutinee.as_str()),
                _ => None,
            };
            if name != Some(decl.name.as_ref())
                || self.map.nodes[node.ordinal].syntax.span() != usage.span
                || !covered_sites.insert(node)
            {
                return Err(MappingError::Invalid("use source/coverage"));
            }
            self.link(node, MappedFact::Use(usage.id))?;
        }
        let expected_reads = self
            .map
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.syntax,
                    MappedSyntax::Expression {
                        syntax: Expression {
                            kind: ExpressionKind::Identifier(_),
                            ..
                        },
                        role: Role::Value
                    }
                )
            })
            .count();
        if read_values.len() != expected_reads
            || covered_sites.len() != expected_reads + writes.len() + scrutinees.len()
        {
            return Err(MappingError::Invalid("missing value use"));
        }
        let mut covered_calls = HashSet::new();
        for link in typed.calls() {
            let call = calls
                .calls()
                .get(link.call)
                .ok_or(MappingError::Invalid("call table index"))?;
            let node = *expression_nodes
                .get(&link.result)
                .ok_or(MappingError::Invalid("call result"))?;
            let MappedSyntax::Expression {
                syntax:
                    Expression {
                        kind: ExpressionKind::Call { callee, .. },
                        ..
                    },
                role: Role::Value,
            } = self.map.nodes[node.ordinal].syntax
            else {
                return Err(MappingError::Invalid("call result shape"));
            };
            let name = match &callee.kind {
                ExpressionKind::Identifier(name) => name.clone(),
                ExpressionKind::QualifiedName { path } => path.join("::"),
                _ => return Err(MappingError::Invalid("call head shape")),
            };
            let target_matches = match call.target {
                CallableTarget::Standard(canonical) => {
                    crate::compiler::stdlib::lookup(&name).is_some_and(|f| f.name == canonical)
                }
                CallableTarget::ProgramFunction(id) => {
                    owner.typed().callable().function().program_target_name(id)
                        == Some(name.as_str())
                }
                CallableTarget::SelfFunction => {
                    name == owner.typed().callable().function().syntax().name
                }
                CallableTarget::Constructor { enum_id, variant } => owner
                    .typed()
                    .callable()
                    .function()
                    .environment()
                    .declarations()
                    .get(enum_id.ordinal())
                    .filter(|d| d.id == enum_id)
                    .and_then(|d| {
                        d.variants.get(variant).map(|v| {
                            name.rsplit_once("::").is_some_and(|(prefix, variant)| {
                                variant == v.name
                                    && owner
                                        .typed()
                                        .callable()
                                        .function()
                                        .environment()
                                        .resolve_enum(prefix)
                                        == Some(enum_id)
                            })
                        })
                    })
                    .unwrap_or(false),
            };
            if !target_matches || call.span != callee.span || !covered_calls.insert(link.call) {
                return Err(MappingError::Invalid("call target/coverage"));
            }
            self.link(node, MappedFact::Call(link.call))?;
        }
        let call_nodes = self
            .map
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.syntax,
                    MappedSyntax::Expression {
                        syntax: Expression {
                            kind: ExpressionKind::Call { .. },
                            ..
                        },
                        role: Role::Value
                    }
                )
            })
            .count();
        if covered_calls.len() != calls.calls().len() || call_nodes != covered_calls.len() {
            return Err(MappingError::Invalid("missing call evidence"));
        }
        let mut covered_contexts = HashSet::new();
        for link in typed.contexts() {
            let context = calls
                .contexts()
                .get(link.context)
                .ok_or(MappingError::Invalid("context table index"))?;
            let fact = typed
                .expression(link.value)
                .ok_or(MappingError::Invalid("foreign context value"))?;
            if fact.span != context.value_span
                || fact.ty != context.actual
                || !covered_contexts.insert(link.context)
            {
                return Err(MappingError::Invalid("context type/coverage"));
            }
            let node = *expression_nodes
                .get(&link.value)
                .ok_or(MappingError::Invalid("context source"))?;
            self.link(node, MappedFact::Context(link.context))?;
        }
        if covered_contexts.len() != calls.contexts().len() {
            return Err(MappingError::Invalid("missing context evidence"));
        }
        Ok(())
    }
}
impl CheckedScopedCallableFunction {
    /// Map checked syntax/facts without fresh type inference or lexical name resolution.
    /// Ambiguous/missing locations refuse; this does not emit or execute IR instructions.
    pub fn map_for_lowering(
        &self,
        limits: MappingLimits,
    ) -> Result<MappedCallable<'_>, MappingError> {
        static NEXT_MAP: AtomicU64 = AtomicU64::new(0);
        let unit = super::enum_types::allocate_identity(&NEXT_MAP)
            .map_err(|_| MappingError::IdentityExhausted)?;
        let mut builder = Builder {
            map: MappedCallable {
                checked: self,
                unit,
                nodes: Vec::new(),
                links: Vec::new(),
            },
            limits,
            scheduled: 0,
            pending: Vec::new(),
            match_indices: HashMap::new(),
            recursive_bodies: HashMap::new(),
            controls: HashMap::new(),
        };
        builder.schedule(
            None,
            MappedSyntax::Function(self.typed().callable().function().syntax()),
        )?;
        builder.walk()?;
        builder.validate()?;
        Ok(builder.map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checked() -> CheckedScopedCallableFunction {
        let context = crate::compiler::parser::Parser::new()
            .parse_source("")
            .unwrap();
        let env = crate::compiler::v2::check_enum_types(
            "o",
            "c",
            &context,
            &[],
            crate::compiler::v2::EnumTypeLimits::DEFAULT,
        )
        .unwrap();
        env.check_scoped_callable_function_source(
            "f",
            "fn f(n:Int)->Int{return n;}",
            crate::compiler::v2::ScopedCallableLimits::DEFAULT,
        )
        .unwrap()
    }
    fn builder(checked: &CheckedScopedCallableFunction) -> Builder<'_> {
        let mut builder = Builder {
            map: MappedCallable {
                checked,
                unit: 1,
                nodes: Vec::new(),
                links: Vec::new(),
            },
            limits: MappingLimits::DEFAULT,
            scheduled: 0,
            pending: Vec::new(),
            match_indices: HashMap::new(),
            recursive_bodies: HashMap::new(),
            controls: HashMap::new(),
        };
        builder
            .schedule(
                None,
                MappedSyntax::Function(checked.typed().callable().function().syntax()),
            )
            .unwrap();
        builder.walk().unwrap();
        builder
    }
    #[test]
    fn duplicate_value_locations_are_refused_instead_of_selecting_an_arbitrary_fact() {
        let checked = checked();
        let mut builder = builder(&checked);
        let mut node = *builder.map.nodes.last().unwrap();
        node.id.ordinal = builder.map.nodes.len();
        builder.map.nodes.push(node);
        assert!(matches!(
            builder.validate(),
            Err(MappingError::AmbiguousSpan(_))
        ));
    }
    #[test]
    fn missing_expression_source_never_becomes_a_partial_map() {
        let checked = checked();
        let mut builder = builder(&checked);
        builder.map.nodes.pop();
        assert_eq!(
            builder.validate(),
            Err(MappingError::Invalid("expression source"))
        );
    }
    #[test]
    fn identifier_use_shape_must_match_the_retained_checked_declaration() {
        let checked = checked();
        let mut builder = builder(&checked);
        let span = builder.map.nodes.last().unwrap().syntax.span();
        let literal = Expression {
            kind: ExpressionKind::Integer("1".into()),
            span,
        };
        builder.map.nodes.last_mut().unwrap().syntax = MappedSyntax::Expression {
            syntax: &literal,
            role: MappedExpressionRole::Value,
        };
        assert_eq!(
            builder.validate(),
            Err(MappingError::Invalid("use source/coverage"))
        );
    }
}
