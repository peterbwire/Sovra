//! Immutable, standalone checked ownership and structural indexing (ADR 0041).
//! Optional checked value/binding types are retained during existing analysis.
//! Scoped function/value uses and numeric coercion edges are separately opt-in.
//! CLI/IDE projection and an explicit standalone IR path consume these facts;
//! standalone executable callers now use bounded compiler::compile_program;
//! ordinary package callers use cumulative checked evidence too.

use std::{collections::HashMap, sync::Arc};

mod evidence;
mod flow;
mod imports;
mod lowering;
pub(crate) use evidence::Recorder;
pub use evidence::{
    EvidenceError, FactKind, RecordedType, SemanticEvidence, SemanticEvidenceLimits, TypeFact,
    TypeId,
};
pub use flow::{
    Coercion, CoercionKind, Declaration, DeclarationKind, FlowEvidence, FlowEvidenceLimits,
    ResolvedUse, UseKind, UseTarget,
};
pub(crate) use imports::ImportBinding;
pub use imports::{FunctionImport, ImportId, ImportedCall, PackageEvidence};
pub use lowering::{FunctionResult, LoweringEvidence, LoweringEvidenceLimits, TypeContext};

use super::ast::{Expression, ExpressionKind, Function, Program, Statement, StructDeclaration};
use super::diagnostics::{Diagnostics, Span};
use super::semantic::SemanticAnalyzer;

/// Explicit snapshot storage allowances; no new ordinary compiler defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceLimits {
    /// Indexed AST nodes, using the existing AST admission node definition.
    pub max_nodes: usize,
    /// Retained original UTF-8 source bytes; irrelevant for AST-only construction.
    pub max_source_bytes: usize,
}

/// Snapshot construction failures, distinct from ordinary semantic diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// Existing parse/semantic/admission diagnostics, unchanged.
    Diagnostics(Diagnostics),
    /// The caller's structural index allowance was exceeded.
    NodeLimit,
    /// The caller's retained source allowance was exceeded.
    SourceLimit,
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostics(errors) => write!(
                formatter,
                "checking failed with {} diagnostics",
                errors.items.len()
            ),
            Self::NodeLimit => formatter.write_str("checked snapshot node allowance exceeded"),
            Self::SourceLimit => formatter.write_str("checked snapshot source allowance exceeded"),
        }
    }
}

impl std::error::Error for CheckError {}

/// Dense structural ordinal. Meaningful only with its owning snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    /// Snapshot-local ordinal, not a persistent or cross-unit identifier.
    pub const fn ordinal(self) -> usize {
        self.0
    }
}

/// Structural node categories, independent of byte-range uniqueness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// Inline source module.
    Module,
    /// Type alias declaration.
    Alias,
    /// Record declaration.
    Record,
    /// Record field declaration.
    Field,
    /// Function declaration.
    Function,
    /// Function parameter.
    Parameter,
    /// Statement, including an expression statement's wrapper.
    Statement,
    /// Expression, including call targets and assignment targets.
    Expression,
}

/// One indexed node. Records are exposed through immutable snapshot borrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRecord {
    /// Unique ordinal within this snapshot, even when spans coincide.
    pub id: NodeId,
    /// Containing indexed node; top-level declarations have no parent.
    pub parent: Option<NodeId>,
    /// Structural category.
    pub kind: NodeKind,
    /// Original byte location; AST-only callers may supply synthetic locations.
    pub span: Span,
}

#[derive(Debug)]
struct Snapshot {
    program: Program,
    source: Option<Arc<str>>,
    nodes: Vec<NodeRecord>,
    evidence: Option<SemanticEvidence>,
}

/// Immutable successfully checked standalone AST and its structural index.
/// Clones share ownership. No mutable AST/fact accessor or unchecked constructor
/// is exposed. Explicit recording constructors can also retain checked types;
/// flow-enabled constructors also retain resolved uses and numeric coercion edges.
///
/// ```compile_fail
/// use sovra::compiler::checked::{CheckedUnit, EvidenceLimits};
/// let unit = CheckedUnit::from_source("fn main() {}", EvidenceLimits {
///     max_nodes: 10, max_source_bytes: 100,
/// }).unwrap();
/// unit.program().functions.clear(); // Cannot mutate a checked snapshot.
/// ```
#[derive(Debug, Clone)]
pub struct CheckedUnit(Arc<Snapshot>);

impl CheckedUnit {
    /// Package-only constructor: normalized signatures and canonical symbols come
    /// from the same resolver/normalizer invocation. No original-source claim.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_package_for_lowering<'a>(
        program: &Program,
        signatures: &[(String, &Function)],
        owner: usize,
        imports: impl IntoIterator<Item = ImportBinding<'a>>,
        max_imports: usize,
        max_import_bytes: usize,
        limits: EvidenceLimits,
        semantic: SemanticEvidenceLimits,
        flow: FlowEvidenceLimits,
        lowering: LoweringEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        super::resources::check_ast_budget(program, limits.max_nodes)
            .map_err(|_| CheckError::NodeLimit)?;
        let mut recorder = Recorder::new(limits.max_nodes, semantic);
        recorder.enable_flow(Some(flow));
        recorder.enable_lowering(Some(lowering));
        recorder.enable_imports(owner, imports, max_imports, max_import_bytes);
        let typed = SemanticAnalyzer::new()
            .analyze_recorded_imports(program, signatures, &mut recorder)
            .map_err(CheckError::Diagnostics)?;
        let (nodes, evidence) = recorder.finish()?;
        Ok(Self(Arc::new(Snapshot {
            program: typed.program,
            source: None,
            nodes,
            evidence: Some(evidence),
        })))
    }
    /// Parse/check the exact source later retained for source-backed consumers.
    /// Existing frontend/AST/depth limits still apply independently. Source and
    /// index quotas do not bound total semantic work, AST strings or process RSS.
    pub fn from_source(source: &str, limits: EvidenceLimits) -> Result<Self, CheckError> {
        if source.len() > limits.max_source_bytes {
            return Err(CheckError::SourceLimit);
        }
        let parsed = super::parser::Parser::new()
            .parse_source(source)
            .map_err(CheckError::Diagnostics)?;
        Self::build(&parsed, Some(source), limits)
    }

    /// Check a borrowed AST without claiming it matches any original source text.
    /// Caller-built excess depth is rejected by existing semantic admission.
    /// Caller-owned AST destruction is unchanged; see Program::drop_iterative.
    pub fn from_program(program: &Program, limits: EvidenceLimits) -> Result<Self, CheckError> {
        Self::build(program, None, limits)
    }

    fn build(
        program: &Program,
        source: Option<&str>,
        limits: EvidenceLimits,
    ) -> Result<Self, CheckError> {
        // Borrowed counting precedes semantic cloning and index allocation.
        let count = super::resources::check_ast_budget(program, limits.max_nodes)
            .map_err(|_| CheckError::NodeLimit)?;
        let checked = SemanticAnalyzer::new()
            .analyze(program)
            .map_err(CheckError::Diagnostics)?;
        let mut index = Index {
            nodes: Vec::new(),
            limit: limits.max_nodes,
            links: None,
        };
        index.unit(&checked.program, None)?;
        debug_assert_eq!(index.nodes.len(), count);
        Ok(Self(Arc::new(Snapshot {
            program: checked.program,
            source: source.map(Arc::from),
            nodes: index.nodes,
            evidence: None,
        })))
    }

    /// Check standalone source while retaining types established by the checker.
    /// Facts describe value expressions, parameters and locals. This type-only
    /// constructor omits flow records; use from_source_with_flow_evidence for
    /// resolved uses/conversions. No production defaults are selected.
    pub fn from_source_with_evidence(
        source: &str,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        if source.len() > limits.max_source_bytes {
            return Err(CheckError::SourceLimit.into());
        }
        let program = super::parser::Parser::new()
            .parse_source(source)
            .map_err(CheckError::Diagnostics)?;
        Self::build_with_evidence(&program, Some(source), limits, semantic_limits, None, None)
    }

    /// Retain semantic types for caller-built syntax without source provenance.
    pub fn from_program_with_evidence(
        program: &Program,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        Self::build_with_evidence(program, None, limits, semantic_limits, None, None)
    }

    /// Retain checked types plus scoped source uses and numeric conversion edges.
    /// Existing type-only APIs remain available with their original allowances.
    pub fn from_source_with_flow_evidence(
        source: &str,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
        flow_limits: FlowEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        if source.len() > limits.max_source_bytes {
            return Err(CheckError::SourceLimit.into());
        }
        let program = super::parser::Parser::new()
            .parse_source(source)
            .map_err(CheckError::Diagnostics)?;
        Self::build_with_evidence(
            &program,
            Some(source),
            limits,
            semantic_limits,
            Some(flow_limits),
            None,
        )
    }

    /// Retain full supported evidence for caller ASTs without original-source claims.
    pub fn from_program_with_flow_evidence(
        program: &Program,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
        flow_limits: FlowEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        Self::build_with_evidence(
            program,
            None,
            limits,
            semantic_limits,
            Some(flow_limits),
            None,
        )
    }

    /// Collect complete supported standalone lowering facts with explicit quotas.
    pub fn from_source_for_lowering(
        source: &str,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
        flow_limits: FlowEvidenceLimits,
        lowering_limits: LoweringEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        if source.len() > limits.max_source_bytes {
            return Err(CheckError::SourceLimit.into());
        }
        let parsed = super::parser::Parser::new()
            .parse_source(source)
            .map_err(CheckError::Diagnostics)?;
        Self::build_with_evidence(
            &parsed,
            Some(source),
            limits,
            semantic_limits,
            Some(flow_limits),
            Some(lowering_limits),
        )
    }

    /// Collect lowering facts for a caller AST, without original-source claims.
    pub fn from_program_for_lowering(
        program: &Program,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
        flow_limits: FlowEvidenceLimits,
        lowering_limits: LoweringEvidenceLimits,
    ) -> Result<Self, EvidenceError> {
        Self::build_with_evidence(
            program,
            None,
            limits,
            semantic_limits,
            Some(flow_limits),
            Some(lowering_limits),
        )
    }

    fn build_with_evidence(
        program: &Program,
        source: Option<&str>,
        limits: EvidenceLimits,
        semantic_limits: SemanticEvidenceLimits,
        flow_limits: Option<FlowEvidenceLimits>,
        lowering_limits: Option<LoweringEvidenceLimits>,
    ) -> Result<Self, EvidenceError> {
        super::resources::check_ast_budget(program, limits.max_nodes)
            .map_err(|_| CheckError::NodeLimit)?;
        super::resources::check_ast_budget(program, super::resources::DEFAULT_AST_NODES)
            .map_err(CheckError::Diagnostics)?;
        let mut recorder = Recorder::new(limits.max_nodes, semantic_limits);
        recorder.enable_flow(flow_limits);
        recorder.enable_lowering(lowering_limits);
        let typed = SemanticAnalyzer::new()
            .analyze_recorded(program, &mut recorder)
            .map_err(CheckError::Diagnostics)?;
        let (nodes, evidence) = recorder.finish()?;
        Ok(Self(Arc::new(Snapshot {
            program: typed.program,
            source: source.map(Arc::from),
            nodes,
            evidence: Some(evidence),
        })))
    }

    /// None for structural-only snapshots; Some only after complete collection
    /// of the currently supported fact categories, with no semantic failures.
    pub fn semantic_evidence(&self) -> Option<&SemanticEvidence> {
        self.0.evidence.as_ref()
    }

    /// Borrow the normalized AST. Clone it explicitly for independent mutation.
    pub fn program(&self) -> &Program {
        &self.0.program
    }

    /// Original text only when this snapshot parsed and checked that exact text.
    pub fn source(&self) -> Option<&str> {
        self.0.source.as_deref()
    }

    /// Deterministic preorder index: aliases, records, functions, then modules;
    /// members use the same category order, children retain AST vector order.
    /// Program/collection containers are not indexed (matching AST admission).
    pub fn nodes(&self) -> &[NodeRecord] {
        &self.0.nodes
    }

    // Rebuild temporary borrowed-address lookup against this immutable AST.
    // Deterministic ordinals, never addresses, remain the persistent identities.
    pub(crate) fn node_links(&self) -> HashMap<(NodeKind, usize), NodeId> {
        let mut index = Index {
            nodes: Vec::new(),
            limit: self.nodes().len(),
            links: Some(HashMap::new()),
        };
        index
            .unit(self.program(), None)
            .expect("snapshot AST matches its admitted index");
        debug_assert_eq!(index.nodes, self.nodes());
        index.links.unwrap()
    }
}

// Recursive descent is used only after existing semantic depth admission.
struct Index {
    nodes: Vec<NodeRecord>,
    limit: usize,
    // Temporary addresses locate structural IDs during this borrowed traversal.
    // Addresses never become identities or survive in a returned snapshot.
    links: Option<HashMap<(NodeKind, usize), NodeId>>,
}

impl Index {
    fn add(
        &mut self,
        kind: NodeKind,
        span: Span,
        parent: Option<NodeId>,
    ) -> Result<NodeId, CheckError> {
        if self.nodes.len() >= self.limit {
            return Err(CheckError::NodeLimit);
        }
        let id = NodeId(self.nodes.len());
        self.nodes.push(NodeRecord {
            id,
            parent,
            kind,
            span,
        });
        Ok(id)
    }

    fn unit(&mut self, program: &Program, parent: Option<NodeId>) -> Result<(), CheckError> {
        for alias in &program.type_declarations {
            self.add(NodeKind::Alias, alias.span, parent)?;
        }
        for record in &program.struct_declarations {
            self.record(record, parent)?;
        }
        for function in &program.functions {
            self.function(function, parent)?;
        }
        for module in &program.modules {
            let id = Some(self.add(NodeKind::Module, module.span, parent)?);
            for alias in &module.type_declarations {
                self.add(NodeKind::Alias, alias.span, id)?;
            }
            for record in &module.struct_declarations {
                self.record(record, id)?;
            }
            for function in &module.functions {
                self.function(function, id)?;
            }
        }
        Ok(())
    }

    fn record(
        &mut self,
        record: &StructDeclaration,
        parent: Option<NodeId>,
    ) -> Result<(), CheckError> {
        let id = Some(self.add(NodeKind::Record, record.span, parent)?);
        for field in &record.fields {
            self.add(NodeKind::Field, field.span, id)?;
        }
        Ok(())
    }

    fn function(&mut self, function: &Function, parent: Option<NodeId>) -> Result<(), CheckError> {
        let function_id = self.add(NodeKind::Function, function.span, parent)?;
        if let Some(links) = &mut self.links {
            links.insert(
                (NodeKind::Function, function as *const _ as usize),
                function_id,
            );
        }
        let id = Some(function_id);
        for parameter in &function.parameters {
            let node = self.add(NodeKind::Parameter, parameter.span, id)?;
            if let Some(links) = &mut self.links {
                links.insert((NodeKind::Parameter, parameter as *const _ as usize), node);
            }
        }
        self.block(&function.body, id)
    }

    fn block(
        &mut self,
        statements: &[Statement],
        parent: Option<NodeId>,
    ) -> Result<(), CheckError> {
        for statement in statements {
            let span = match statement {
                Statement::Let { span, .. }
                | Statement::Assign { span, .. }
                | Statement::Return { span, .. }
                | Statement::If { span, .. }
                | Statement::While { span, .. } => *span,
                Statement::Expression(value) => value.span,
            };
            let node = self.add(NodeKind::Statement, span, parent)?;
            if let Some(links) = &mut self.links {
                links.insert((NodeKind::Statement, statement as *const _ as usize), node);
            }
            let id = Some(node);
            match statement {
                Statement::Let { value, .. }
                | Statement::Expression(value)
                | Statement::Return {
                    value: Some(value), ..
                } => self.expression(value, id)?,
                Statement::Assign { target, value, .. } => {
                    self.expression(target, id)?;
                    self.expression(value, id)?;
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    self.expression(condition, id)?;
                    self.block(then_block, id)?;
                    if let Some(body) = else_block {
                        self.block(body, id)?;
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    self.expression(condition, id)?;
                    self.block(body, id)?;
                }
            }
        }
        Ok(())
    }

    fn expression(&mut self, value: &Expression, parent: Option<NodeId>) -> Result<(), CheckError> {
        let node = self.add(NodeKind::Expression, value.span, parent)?;
        if let Some(links) = &mut self.links {
            links.insert((NodeKind::Expression, value as *const _ as usize), node);
        }
        let id = Some(node);
        match &value.kind {
            ExpressionKind::Binary { left, right, .. } => {
                self.expression(left, id)?;
                self.expression(right, id)?;
            }
            ExpressionKind::Call { callee, arguments } => {
                self.expression(callee, id)?;
                for value in arguments {
                    self.expression(value, id)?;
                }
            }
            ExpressionKind::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.expression(value, id)?;
                }
            }
            ExpressionKind::ArrayLiteral(items) => {
                for value in items {
                    self.expression(value, id)?;
                }
            }
            ExpressionKind::FieldAccess { receiver, .. } => self.expression(receiver, id)?,
            ExpressionKind::Index { target, index } => {
                self.expression(target, id)?;
                self.expression(index, id)?;
            }
            ExpressionKind::String(_)
            | ExpressionKind::Integer(_)
            | ExpressionKind::Float(_)
            | ExpressionKind::Boolean(_)
            | ExpressionKind::Identifier(_)
            | ExpressionKind::QualifiedName { .. } => {}
        }
        Ok(())
    }
}
