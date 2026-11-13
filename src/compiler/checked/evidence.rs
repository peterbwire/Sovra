//! Bounded collection during semantic checking; no independent inference walk.

use std::{collections::HashMap, sync::Arc};

use super::flow::{
    Coercion, CoercionKind, DeclarationKind, FlowEvidence, FlowEvidenceLimits, FlowRecorder,
    UseKind, UseTarget,
};
use super::imports::{ImportBinding, ImportRecorder, ImportedCall, PackageEvidence};
use super::lowering::{
    FunctionResult, LoweringEvidence, LoweringEvidenceLimits, LoweringRecorder, TypeContext,
};
use super::{CheckError, Index, NodeId, NodeKind, NodeRecord};
use crate::compiler::{
    ast::{Expression, Function, Program},
    semantic::Type,
};

/// Opt-in recording failures; the structural-only CheckError API is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    /// Existing parse/check/structural/source failure, with original diagnostics.
    Check(CheckError),
    /// A caller-selected semantic fact/type/name allowance was exhausted.
    Limit,
}

impl From<CheckError> for EvidenceError {
    fn from(error: CheckError) -> Self {
        Self::Check(error)
    }
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Check(error) => error.fmt(formatter),
            Self::Limit => formatter.write_str("checked semantic evidence allowance exceeded"),
        }
    }
}

impl std::error::Error for EvidenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Check(error) => Some(error),
            Self::Limit => None,
        }
    }
}

impl EvidenceError {
    /// Convert opt-in checking failures into compiler diagnostics. Existing source
    /// diagnostics are preserved. Metadata exhaustion uses E3025 with no invented
    /// source range or file; the report target supplies invocation identity.
    /// This does not change ordinary compiler entry points or IDE fallback policy.
    pub fn into_diagnostics(self) -> crate::compiler::diagnostics::Diagnostics {
        use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
        if let Self::Check(CheckError::Diagnostics(diagnostics)) = self {
            return diagnostics;
        }
        let message = match self {
            Self::Check(CheckError::NodeLimit) => "checked structural evidence allowance exceeded",
            Self::Check(CheckError::SourceLimit) => "checked retained-source allowance exceeded",
            _ => "checked semantic evidence allowance exceeded",
        };
        Diagnostics {
            items: vec![Diagnostic {
                severity: Severity::Error,
                code: "E3025",
                message: message.to_owned(),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
                source_file: None,
            }],
        }
    }
}

/// Explicit limits for retained type facts, separate from structural/source limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticEvidenceLimits {
    /// Maximum expression/parameter/local records (each counts once).
    pub max_facts: usize,
    /// Maximum unique interned types, including array layers and inference holes.
    pub max_types: usize,
    /// UTF-8 bytes of unique retained nominal type names.
    pub max_name_bytes: usize,
}

/// Interned type ordinal, meaningful only within its owning semantic evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(usize);

impl TypeId {
    /// Snapshot-local type table ordinal.
    pub const fn ordinal(self) -> usize {
        self.0
    }
}

/// Canonical checked type structure; aliases' original spelling stays in the AST/source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedType {
    /// Unit value.
    Unit,
    /// Boolean value.
    Bool,
    /// Signed integer value.
    Int,
    /// Floating-point value.
    Float,
    /// UTF-8 string value.
    String,
    /// Array element type, including nested inference holes.
    Array(TypeId),
    /// Canonical standalone nominal type identity (package identities are future work).
    Named(Arc<str>),
    /// An existing unresolved inference hole, never a synthesized Any type.
    Unknown,
}

/// Semantic role of a recorded type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactKind {
    /// Value expression actually checked, excluding function-call heads/unchecked targets.
    Expression,
    /// Function parameter declaration.
    Parameter,
    /// Local declaration's binding type, separate from its initializer's type.
    Local {
        /// Whether assignment to this binding is allowed by existing rules.
        mutable: bool,
    },
}

/// Checked type attached to a structural node in the same snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeFact {
    /// Structural expression/parameter/let identity.
    pub node: NodeId,
    /// Interned canonical checked type.
    pub ty: TypeId,
    /// Declaration or value-expression role.
    pub kind: FactKind,
}

/// Read-only successfully collected types. Collection order follows checking;
/// consumers must use node IDs, not assume structural traversal order.
#[derive(Debug)]
pub struct SemanticEvidence {
    types: Vec<RecordedType>,
    facts: Vec<TypeFact>,
    name_bytes: usize,
    flow: Option<FlowEvidence>,
    lowering: Option<LoweringEvidence>,
    package: Option<PackageEvidence>,
}

impl SemanticEvidence {
    /// Package provenance only for the resolver-owned import-aware constructor.
    pub fn package(&self) -> Option<&PackageEvidence> {
        self.package.as_ref()
    }
    /// Contexts/results only for explicit lowering-ready constructors.
    pub fn lowering(&self) -> Option<&LoweringEvidence> {
        self.lowering.as_ref()
    }
    /// None for type-only collection; Some for explicit flow-enabled collection.
    pub fn flow(&self) -> Option<&FlowEvidence> {
        self.flow.as_ref()
    }
    /// Interned types; IDs are indices into this table only.
    pub fn types(&self) -> &[RecordedType] {
        &self.types
    }
    /// Types recorded where the checker establishes a value/binding type.
    pub fn facts(&self) -> &[TypeFact] {
        &self.facts
    }
    /// Charged unique nominal-name payload, excluding allocation/map overhead.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
}

#[derive(Default)]
pub(crate) struct Recorder {
    limits: Option<SemanticEvidenceLimits>,
    node_limit: usize,
    index: Option<Index>,
    types: Vec<RecordedType>,
    facts: Vec<TypeFact>,
    scalars: [Option<TypeId>; 6],
    names: HashMap<Arc<str>, TypeId>,
    arrays: HashMap<TypeId, TypeId>,
    name_bytes: usize,
    exhausted: bool,
    flow: Option<FlowRecorder>,
    value_types: HashMap<NodeId, TypeId>,
    lowering: Option<LoweringRecorder>,
    package: Option<ImportRecorder>,
}

impl Recorder {
    pub(super) fn enable_imports<'a>(
        &mut self,
        owner: usize,
        imports: impl IntoIterator<Item = ImportBinding<'a>>,
        max_imports: usize,
        max_bytes: usize,
    ) {
        match ImportRecorder::new(owner, imports, max_imports, max_bytes) {
            Ok(package) => self.package = Some(package),
            Err(_) => self.exhausted = true, // Keep checking so ordinary diagnostics win.
        }
    }
    pub(super) fn enable_lowering(&mut self, limits: Option<LoweringEvidenceLimits>) {
        self.lowering = limits.map(|limits| LoweringRecorder {
            limits,
            evidence: LoweringEvidence::default(),
        });
    }

    pub(crate) fn function_result(
        &mut self,
        function: &Function,
        ty: &Type,
        names: &HashMap<String, Type>,
    ) {
        let Some(lowering) = &self.lowering else {
            return;
        };
        if self.exhausted {
            return;
        }
        if lowering.evidence.results.len() >= lowering.limits.max_function_results {
            self.exhausted = true;
            return;
        }
        let function = self.id(function, NodeKind::Function);
        let Some(ty) = self.intern(ty, names) else {
            self.exhausted = true;
            return;
        };
        self.lowering
            .as_mut()
            .unwrap()
            .evidence
            .results
            .push(FunctionResult { function, ty });
    }
    pub(super) fn enable_flow(&mut self, limits: Option<FlowEvidenceLimits>) {
        self.flow = limits.map(FlowRecorder::new);
    }
    pub(super) fn new(node_limit: usize, limits: SemanticEvidenceLimits) -> Self {
        Self {
            limits: Some(limits),
            node_limit,
            ..Self::default()
        }
    }

    pub(crate) fn begin(&mut self, program: &Program) {
        if self.limits.is_none() {
            return;
        }
        let mut index = Index {
            nodes: Vec::new(),
            limit: self.node_limit,
            links: Some(HashMap::new()),
        };
        if index.unit(program, None).is_err() {
            self.exhausted = true;
        }
        self.index = Some(index);
        for function in program
            .functions
            .iter()
            .chain(program.modules.iter().flat_map(|module| &module.functions))
        {
            self.declare(
                function,
                NodeKind::Function,
                &function.name,
                DeclarationKind::Function,
            );
        }
    }

    pub(super) fn finish(self) -> Result<(Vec<NodeRecord>, SemanticEvidence), EvidenceError> {
        if self.exhausted {
            return Err(EvidenceError::Limit);
        }
        let nodes = self
            .index
            .expect("successful recorded analysis indexes its unit")
            .nodes;
        Ok((
            nodes,
            SemanticEvidence {
                types: self.types,
                facts: self.facts,
                name_bytes: self.name_bytes,
                flow: self.flow.map(|flow| flow.evidence),
                lowering: self.lowering.map(|lowering| lowering.evidence),
                package: self.package.map(|package| package.evidence),
            },
        ))
    }

    pub(crate) fn record<T>(
        &mut self,
        node: &T,
        kind: NodeKind,
        role: FactKind,
        ty: &Type,
        named_types: &HashMap<String, Type>,
    ) {
        let Some(limits) = self.limits else {
            return;
        };
        if self.exhausted {
            return;
        }
        if self.facts.len() >= limits.max_facts {
            self.exhausted = true;
            return;
        }
        let id = self
            .index
            .as_ref()
            .and_then(|index| index.links.as_ref())
            .and_then(|links| links.get(&(kind, node as *const T as usize)))
            .copied()
            .expect("checked node belongs to indexed normalized AST");
        if let Some(ty) = self.intern(ty, named_types) {
            if self.flow.is_some() && role == FactKind::Expression {
                self.value_types.insert(id, ty);
            }
            self.facts.push(TypeFact {
                node: id,
                ty,
                kind: role,
            });
        } else {
            self.exhausted = true;
        }
    }

    fn id<T>(&self, node: &T, kind: NodeKind) -> NodeId {
        *self
            .index
            .as_ref()
            .and_then(|index| index.links.as_ref())
            .and_then(|links| links.get(&(kind, node as *const T as usize)))
            .expect("flow node belongs to indexed normalized AST")
    }

    pub(crate) fn start_function(&mut self) {
        if self.exhausted {
            return;
        }
        if let Some(flow) = &mut self.flow {
            flow.scopes.clear();
            flow.scopes.push(HashMap::new());
        }
    }

    pub(crate) fn has_flow(&self) -> bool {
        self.flow.is_some() && !self.exhausted
    }

    pub(crate) fn enter_scope(&mut self) {
        if self.exhausted {
            return;
        }
        if let Some(flow) = &mut self.flow {
            flow.scopes.push(HashMap::new());
        }
    }

    pub(crate) fn exit_scope(&mut self) {
        if self.exhausted {
            return;
        }
        if let Some(flow) = &mut self.flow {
            flow.scopes.pop().expect("entered scope");
        }
    }

    pub(crate) fn declare<T>(
        &mut self,
        node: &T,
        kind: NodeKind,
        name: &str,
        role: DeclarationKind,
    ) {
        if self.flow.is_none() || self.exhausted {
            return;
        }
        let id = self.id(node, kind);
        if !self.flow.as_mut().unwrap().declare(id, name, role) {
            self.exhausted = true;
        }
    }

    fn allow_use(&mut self) -> bool {
        if self.flow.is_none() || self.exhausted {
            return false;
        }
        let uses = self.flow.as_ref().unwrap().evidence.uses.len()
            + self
                .package
                .as_ref()
                .map_or(0, |package| package.evidence.calls.len());
        if uses >= self.flow.as_ref().unwrap().limits.max_uses {
            self.exhausted = true;
            return false;
        }
        true
    }

    pub(crate) fn variable(&mut self, node: &Expression, name: &str, kind: UseKind) {
        if !self.allow_use() {
            return;
        }
        let id = self.id(node, NodeKind::Expression);
        let target = self
            .flow
            .as_ref()
            .unwrap()
            .resolve(name)
            .expect("a checker-resolved value has an active declaration identity");
        if !self
            .flow
            .as_mut()
            .unwrap()
            .usage(id, UseTarget::Declaration(target), kind)
        {
            self.exhausted = true;
        }
    }

    pub(crate) fn function_call(&mut self, head: &Expression, function: &Function, spelling: &str) {
        if !self.allow_use() {
            return;
        }
        if let Some(package) = &self.package {
            if let Some((import, pointer)) = package.lookup.get(spelling) {
                assert_eq!(
                    *pointer, function as *const Function as usize,
                    "selected imported signature matches its binding"
                );
                let import = *import;
                let node = self.id(head, NodeKind::Expression);
                self.package
                    .as_mut()
                    .unwrap()
                    .evidence
                    .calls
                    .push(ImportedCall { node, import });
                return;
            }
        }
        let target = self.id(function, NodeKind::Function);
        let node = self.id(head, NodeKind::Expression);
        if !self
            .flow
            .as_mut()
            .unwrap()
            .usage(node, UseTarget::Declaration(target), UseKind::Call)
        {
            self.exhausted = true;
        }
    }

    pub(crate) fn builtin_call(&mut self, head: &Expression, name: &'static str) {
        if !self.allow_use() {
            return;
        }
        let node = self.id(head, NodeKind::Expression);
        if !self
            .flow
            .as_mut()
            .unwrap()
            .usage(node, UseTarget::Builtin(name), UseKind::Call)
        {
            self.exhausted = true;
        }
    }

    #[allow(clippy::too_many_arguments)] // Keep explicit intrinsic/context types at checker edges.
    pub(crate) fn coerce<T>(
        &mut self,
        owner: &T,
        owner_kind: NodeKind,
        value: &Expression,
        kind: CoercionKind,
        actual: &Type,
        expected: &Type,
        names: &HashMap<String, Type>,
    ) {
        if self.flow.is_none() || self.exhausted {
            return;
        }
        let Some(from) = self.intern(actual, names) else {
            self.exhausted = true;
            return;
        };
        let Some(to) = self.intern(expected, names) else {
            self.exhausted = true;
            return;
        };
        self.coerce_ids(owner, owner_kind, value, kind, from, to);
    }

    fn coerce_ids<T>(
        &mut self,
        owner: &T,
        owner_kind: NodeKind,
        value: &Expression,
        kind: CoercionKind,
        from: TypeId,
        to: TypeId,
    ) {
        if let Some(lowering) = &self.lowering {
            if lowering.evidence.contexts.len() >= lowering.limits.max_contexts {
                self.exhausted = true;
                return;
            }
            let owner = self.id(owner, owner_kind);
            let value = self.id(value, NodeKind::Expression);
            self.lowering
                .as_mut()
                .unwrap()
                .evidence
                .contexts
                .push(TypeContext {
                    owner,
                    value,
                    expected: to,
                    kind,
                });
        }
        let (mut left, mut right, mut depth) = (from, to, 0);
        loop {
            match (&self.types[left.0], &self.types[right.0]) {
                (RecordedType::Array(a), RecordedType::Array(b)) => {
                    left = *a;
                    right = *b;
                    depth += 1;
                }
                (RecordedType::Int, RecordedType::Float) => break,
                _ => return, // Identity, holes and non-numeric types are not widening.
            }
        }
        let flow = self.flow.as_ref().expect("flow-enabled conversion");
        if flow.evidence.coercions.len() >= flow.limits.max_coercions {
            self.exhausted = true;
            return;
        }
        let owner = self.id(owner, owner_kind);
        let value = self.id(value, NodeKind::Expression);
        self.flow
            .as_mut()
            .unwrap()
            .evidence
            .coercions
            .push(Coercion {
                owner,
                value,
                from,
                to,
                array_depth: depth,
                kind,
            });
    }

    pub(crate) fn array_items(
        &mut self,
        owner: &Expression,
        items: &[Expression],
        element: &Type,
        names: &HashMap<String, Type>,
    ) {
        if self.flow.is_none() || self.exhausted {
            return;
        }
        let Some(to) = self.intern(element, names) else {
            self.exhausted = true;
            return;
        };
        for value in items {
            let node = self.id(value, NodeKind::Expression);
            if let Some(from) = self.value_types.get(&node).copied() {
                self.coerce_ids(
                    owner,
                    NodeKind::Expression,
                    value,
                    CoercionKind::ArrayElement,
                    from,
                    to,
                );
                if self.exhausted {
                    return;
                }
            }
        }
    }

    fn intern(&mut self, ty: &Type, named_types: &HashMap<String, Type>) -> Option<TypeId> {
        // Canonicalization follows existing checker rules, including explicit holes.
        // Borrow arrays directly rather than repeatedly cloning nested types.
        let canonical;
        let ty = if matches!(ty, Type::Named(_)) {
            canonical = ty.canonicalize(named_types);
            &canonical
        } else {
            ty
        };
        let scalar = match ty {
            Type::Unit => Some(0),
            Type::Bool => Some(1),
            Type::Int => Some(2),
            Type::Float => Some(3),
            Type::String => Some(4),
            Type::Unknown => Some(5),
            _ => None,
        };
        if let Some(index) = scalar {
            if let Some(id) = self.scalars[index] {
                return Some(id);
            }
        }
        let record = match ty {
            Type::Unit => RecordedType::Unit,
            Type::Bool => RecordedType::Bool,
            Type::Int => RecordedType::Int,
            Type::Float => RecordedType::Float,
            Type::String => RecordedType::String,
            Type::Unknown => RecordedType::Unknown,
            Type::Named(name) => {
                if let Some(id) = self.names.get(name.as_str()) {
                    return Some(*id);
                }
                let total = self.name_bytes.checked_add(name.len())?;
                if total > self.limits?.max_name_bytes || self.types.len() >= self.limits?.max_types
                {
                    return None;
                }
                self.name_bytes = total;
                RecordedType::Named(Arc::from(name.as_str()))
            }
            Type::Array(element) => {
                let element = self.intern(element, named_types)?;
                if let Some(id) = self.arrays.get(&element) {
                    return Some(*id);
                }
                RecordedType::Array(element)
            }
        };
        if self.types.len() >= self.limits?.max_types {
            return None;
        }
        let id = TypeId(self.types.len());
        match &record {
            RecordedType::Named(name) => {
                self.names.insert(Arc::clone(name), id);
            }
            RecordedType::Array(element) => {
                self.arrays.insert(*element, id);
            }
            _ => {
                self.scalars[scalar.expect("primitive type has scalar slot")] = Some(id);
            }
        }
        self.types.push(record);
        Some(id)
    }
}
