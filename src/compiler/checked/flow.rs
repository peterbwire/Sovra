//! Bounded declaration/use and numeric-conversion evidence; no new resolution rules.

use std::{collections::HashMap, sync::Arc};

use super::{NodeId, TypeId};

/// Caller-selected retained declaration/use/conversion allowances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowEvidenceLimits {
    /// Function, parameter and local declarations.
    pub max_declarations: usize,
    /// Value reads, writes and function-call heads.
    pub max_uses: usize,
    /// Non-identity numeric conversion edges.
    pub max_coercions: usize,
    /// Retained declaration-name UTF-8 payload, charged per declaration.
    pub max_name_bytes: usize,
}

/// Kind of declaration in the independent value/function namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationKind {
    /// Source function, including inline module functions.
    Function,
    /// Function parameter.
    Parameter,
    /// Local declaration.
    Local {
        /// Existing assignment permission.
        mutable: bool,
    },
}

/// A declaration identity is its structural node in the same checked snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// Snapshot-local declaration identity, never a byte-offset key.
    pub node: NodeId,
    /// Original unqualified declaration spelling.
    pub name: Arc<str>,
    /// Function/parameter/local role.
    pub kind: DeclarationKind,
}

/// Resolved source declaration or Rust registry builtin (never an invented file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseTarget {
    /// Function, parameter or local node in this snapshot.
    Declaration(NodeId),
    /// Canonical Rust-owned standard-library function name.
    Builtin(&'static str),
}

/// Semantic use category; indexed writes identify the outer index expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    /// Value-expression read, including array/field receivers.
    Read,
    /// Direct mutable binding assignment target.
    Write,
    /// Mutable array indexed write, separate from its receiver read.
    IndexedWrite,
    /// Function-call head, independent of same-spelled local bindings.
    Call,
}

/// A use resolved by existing semantic checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedUse {
    /// Identifier, qualified call head or indexed target node.
    pub node: NodeId,
    /// Checked target identity.
    pub target: UseTarget,
    /// Read/write/call role.
    pub kind: UseKind,
}

/// Context requiring an existing numeric conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoercionKind {
    /// Initializer to local binding type.
    Initialization,
    /// Assignment to an existing binding type.
    Assignment,
    /// Assignment to an array element type.
    IndexedAssignment,
    /// Returned expression to declared result type.
    Return,
    /// Call argument to parameter type.
    Argument,
    /// Record-literal field value to declared field type.
    RecordField,
    /// Array-literal item to its final inferred element type.
    ArrayElement,
    /// Mixed numeric arithmetic/comparison operand; not a new IR instruction.
    BinaryOperand,
}

/// Non-identity Int-to-Float conversion, possibly through equal array nesting.
/// Records the existing semantic requirement, not where a backend implements it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coercion {
    /// Containing statement, call, record, array or binary expression.
    pub owner: NodeId,
    /// Expression whose value needs conversion.
    pub value: NodeId,
    /// Intrinsic checked type before conversion.
    pub from: TypeId,
    /// Contextual checked type after conversion.
    pub to: TypeId,
    /// Zero for scalar widening, positive for nested array widening.
    pub array_depth: usize,
    /// Context establishing this requirement.
    pub kind: CoercionKind,
}

/// Immutable source function/value links and conversion edges for one snapshot.
#[derive(Debug, Default)]
pub struct FlowEvidence {
    pub(super) declarations: Vec<Declaration>,
    pub(super) uses: Vec<ResolvedUse>,
    pub(super) coercions: Vec<Coercion>,
    pub(super) name_bytes: usize,
}

impl FlowEvidence {
    /// All checked function/parameter/local declarations.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }
    /// Resolved value reads/writes and function-call heads.
    pub fn uses(&self) -> &[ResolvedUse] {
        &self.uses
    }
    /// Existing non-identity numeric requirements; identity/Unknown edges omitted.
    pub fn coercions(&self) -> &[Coercion] {
        &self.coercions
    }
    /// Retained name payload; map/Vec capacity is not included.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
}

pub(super) struct FlowRecorder {
    pub limits: FlowEvidenceLimits,
    pub evidence: FlowEvidence,
    pub scopes: Vec<HashMap<Arc<str>, NodeId>>,
}

impl FlowRecorder {
    pub fn new(limits: FlowEvidenceLimits) -> Self {
        Self {
            limits,
            evidence: FlowEvidence::default(),
            scopes: Vec::new(),
        }
    }

    pub fn declare(&mut self, node: NodeId, name: &str, kind: DeclarationKind) -> bool {
        if self.evidence.declarations.len() >= self.limits.max_declarations {
            return false;
        }
        let Some(total) = self.evidence.name_bytes.checked_add(name.len()) else {
            return false;
        };
        if total > self.limits.max_name_bytes {
            return false;
        }
        let name: Arc<str> = Arc::from(name);
        self.evidence.name_bytes = total;
        if kind != DeclarationKind::Function {
            self.scopes
                .last_mut()
                .expect("binding belongs to active function scope")
                .insert(Arc::clone(&name), node);
        }
        self.evidence
            .declarations
            .push(Declaration { node, name, kind });
        true
    }

    pub fn resolve(&self, name: &str) -> Option<NodeId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    pub fn usage(&mut self, node: NodeId, target: UseTarget, kind: UseKind) -> bool {
        if self.evidence.uses.len() >= self.limits.max_uses {
            return false;
        }
        self.evidence.uses.push(ResolvedUse { node, target, kind });
        true
    }
}
