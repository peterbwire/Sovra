//! Opt-in value declarations and scope-resolved read/write/scrutinee identities.
use super::{
    CheckedEnumTypes, CheckedTypedCallableFunction, EnumFunctionError, TypedCallableLimits,
    TypedExpressionId,
};
use crate::compiler::{checked::UseKind, diagnostics::Span, semantic::Type};
use std::{collections::HashMap, sync::Arc};
/// Independent retained binding and active-scope admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingEvidenceLimits {
    /// Parameter, local and payload declarations.
    pub max_declarations: usize,
    /// Read, write, indexed-write and match-scrutinee sites.
    pub max_uses: usize,
    /// Owned declaration-name bytes, charged once per declaration.
    pub max_name_bytes: usize,
    /// Cumulative copied declaration type leaves and array wrappers.
    pub max_type_nodes: usize,
    /// Cumulative copied nominal-name bytes in declaration types.
    pub max_type_text_bytes: usize,
    /// Simultaneously active lexical frames, including the function root.
    pub max_scope_frames: usize,
}
impl BindingEvidenceLimits {
    /// Opt-in finite profile; inherited source/depth quotas still apply.
    pub const DEFAULT: Self = Self {
        max_declarations: 250_000,
        max_uses: 500_000,
        max_name_bytes: 16 * 1024 * 1024,
        max_type_nodes: 1_000_000,
        max_type_text_bytes: 16 * 1024 * 1024,
        max_scope_frames: 256,
    };
}
/// Full existing typed admission plus separate binding evidence caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopedCallableLimits {
    /// Existing source, call, context and expression budgets.
    pub typed: TypedCallableLimits,
    /// Additional declarations, uses, name/type copies and scope frames.
    pub bindings: BindingEvidenceLimits,
}
impl ScopedCallableLimits {
    /// Opt-in defaults; older APIs retain previous behavior.
    pub const DEFAULT: Self = Self {
        typed: TypedCallableLimits::DEFAULT,
        bindings: BindingEvidenceLimits::DEFAULT,
    };
}
/// A source value declaration scoped to one immutable checked unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopedDeclarationId {
    unit: u64,
    ordinal: usize,
}
impl ScopedDeclarationId {
    /// Position in this unit's declaration table, not a global symbol number.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// A source value-use site scoped to the same checked unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopedUseId {
    unit: u64,
    ordinal: usize,
}
impl ScopedUseId {
    /// Position in the unit's semantic use order.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Role and assignment permission established by source checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopedDeclarationKind {
    /// Immutable annotated function parameter.
    Parameter,
    /// Ordinary local declaration with core shadowing behavior.
    Local {
        /// Source assignment permission.
        mutable: bool,
    },
    /// Immutable arm-local snapshot binding.
    Payload {
        /// Direct-match ordinal in the checked function.
        match_index: usize,
        /// Declaration variant ordinal, independent of arm order.
        variant: usize,
    },
}
/// One retained value declaration; function call targets remain a separate namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedDeclaration {
    /// Unit-owned declaration identity.
    pub id: ScopedDeclarationId,
    /// Retained source spelling; scopes share this allocation.
    pub name: Arc<str>,
    /// Original parameter/local/payload declaration location.
    pub span: Span,
    /// Checked source role and permission.
    pub kind: ScopedDeclarationKind,
    /// Checked canonical core type, retaining existing empty-array holes.
    pub ty: Type,
}
/// Use categories; match scrutinees are resolved binding reads without fabricated expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopedUseKind {
    /// Value-expression identifier read.
    Read,
    /// Direct binding write.
    Write,
    /// Indexed write to an array binding.
    IndexedWrite,
    /// Direct match's once-evaluated binding read.
    MatchScrutinee {
        /// Match ordinal in the checked function.
        match_index: usize,
    },
}
/// One use collected during core checking, without a later source-inference pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopedUse {
    /// Unit-owned use-site identity.
    pub id: ScopedUseId,
    /// Declaration selected by the active checked scope.
    pub declaration: ScopedDeclarationId,
    /// Original use location (indexed writes include the complete target).
    pub span: Span,
    /// Checked read/write/scrutinee category.
    pub kind: ScopedUseKind,
    /// Read-expression identity; writes/scrutinees are not value expressions.
    pub value: Option<TypedExpressionId>,
}
/// Immutable source value-binding facts; IDs from other units are refused.
#[derive(Debug, Clone)]
pub struct ScopedCallableEvidence {
    unit: u64,
    declarations: Vec<ScopedDeclaration>,
    uses: Vec<ScopedUse>,
}
impl ScopedCallableEvidence {
    /// Declarations in semantic introduction order.
    pub fn declarations(&self) -> &[ScopedDeclaration] {
        &self.declarations
    }
    /// Uses in semantic visitation order, which can differ from source order.
    pub fn uses(&self) -> &[ScopedUse] {
        &self.uses
    }
    /// Refuse foreign declaration IDs, even for identical source.
    pub fn declaration(&self, id: ScopedDeclarationId) -> Option<&ScopedDeclaration> {
        if id.unit == self.unit {
            self.declarations.get(id.ordinal)
        } else {
            None
        }
    }
    /// Refuse foreign use-site IDs.
    pub fn usage(&self, id: ScopedUseId) -> Option<&ScopedUse> {
        if id.unit == self.unit {
            self.uses.get(id.ordinal)
        } else {
            None
        }
    }
}
/// Checked single-function typed/call evidence plus value binding identities.
#[derive(Debug, Clone)]
pub struct CheckedScopedCallableFunction {
    typed: CheckedTypedCallableFunction,
    bindings: ScopedCallableEvidence,
}
impl CheckedScopedCallableFunction {
    /// Existing source, call, conversion and expression facts.
    pub fn typed(&self) -> &CheckedTypedCallableFunction {
        &self.typed
    }
    /// Scope-resolved declarations and value-use identities.
    pub fn bindings(&self) -> &ScopedCallableEvidence {
        &self.bindings
    }
}
#[derive(Debug)]
pub(super) struct ScopedRecorder {
    limits: BindingEvidenceLimits,
    evidence: ScopedCallableEvidence,
    frames: Vec<HashMap<Arc<str>, ScopedDeclarationId>>,
    names: usize,
    nodes: usize,
    text: usize,
}
impl ScopedRecorder {
    pub(super) fn new(unit: u64, limits: BindingEvidenceLimits) -> Option<Self> {
        if limits.max_scope_frames == 0 {
            return None;
        }
        Some(Self {
            limits,
            evidence: ScopedCallableEvidence {
                unit,
                declarations: Vec::new(),
                uses: Vec::new(),
            },
            frames: vec![HashMap::new()],
            names: 0,
            nodes: 0,
            text: 0,
        })
    }
    pub(super) fn enter(&mut self) -> bool {
        if self.frames.len() >= self.limits.max_scope_frames {
            return false;
        }
        self.frames.push(HashMap::new());
        true
    }
    pub(super) fn exit(&mut self) -> bool {
        if self.frames.len() <= 1 {
            return false;
        }
        self.frames.pop();
        true
    }
    pub(super) fn declare(
        &mut self,
        name: &str,
        span: Span,
        kind: ScopedDeclarationKind,
        ty: &Type,
        named: &HashMap<String, Type>,
    ) -> bool {
        if self.evidence.declarations.len() >= self.limits.max_declarations {
            return false;
        }
        let Some(names) = self
            .names
            .checked_add(name.len())
            .filter(|n| *n <= self.limits.max_name_bytes)
        else {
            return false;
        };
        self.names = names;
        let ty = ty.canonicalize(named);
        let mut leaf = &ty;
        loop {
            let Some(nodes) = self
                .nodes
                .checked_add(1)
                .filter(|n| *n <= self.limits.max_type_nodes)
            else {
                return false;
            };
            self.nodes = nodes;
            match leaf {
                Type::Array(item) => leaf = item,
                Type::Named(name) => {
                    let Some(text) = self
                        .text
                        .checked_add(name.len())
                        .filter(|n| *n <= self.limits.max_type_text_bytes)
                    else {
                        return false;
                    };
                    self.text = text;
                    break;
                }
                _ => break,
            }
        }
        let id = ScopedDeclarationId {
            unit: self.evidence.unit,
            ordinal: self.evidence.declarations.len(),
        };
        let name: Arc<str> = Arc::from(name);
        self.frames
            .last_mut()
            .expect("function root remains active")
            .insert(Arc::clone(&name), id);
        self.evidence.declarations.push(ScopedDeclaration {
            id,
            name,
            span,
            kind,
            ty,
        });
        true
    }
    pub(super) fn usage(
        &mut self,
        name: &str,
        span: Span,
        kind: ScopedUseKind,
        value: Option<TypedExpressionId>,
    ) -> bool {
        if self.evidence.uses.len() >= self.limits.max_uses {
            return false;
        }
        let Some(declaration) = self
            .frames
            .iter()
            .rev()
            .find_map(|frame| frame.get(name))
            .copied()
        else {
            return false;
        };
        if kind == ScopedUseKind::Read && value.is_none() {
            return false;
        }
        let id = ScopedUseId {
            unit: self.evidence.unit,
            ordinal: self.evidence.uses.len(),
        };
        self.evidence.uses.push(ScopedUse {
            id,
            declaration,
            span,
            kind,
            value,
        });
        true
    }
    pub(super) fn variable_kind(kind: UseKind) -> Option<ScopedUseKind> {
        match kind {
            UseKind::Read => Some(ScopedUseKind::Read),
            UseKind::Write => Some(ScopedUseKind::Write),
            UseKind::IndexedWrite => Some(ScopedUseKind::IndexedWrite),
            UseKind::Call => None,
        }
    }
    pub(super) fn finish(self) -> Option<ScopedCallableEvidence> {
        if self.frames.len() == 1 {
            Some(self.evidence)
        } else {
            None
        }
    }
}
impl CheckedEnumTypes {
    /// Record scope-resolved value declarations/reads/writes and match scrutinee uses.
    /// This does not implement executable lowering or expand the single-function grammar.
    pub fn check_scoped_callable_function_source(
        &self,
        label: &str,
        source: &str,
        limits: ScopedCallableLimits,
    ) -> Result<CheckedScopedCallableFunction, EnumFunctionError> {
        let result = self.check_function_source_mode(
            label,
            source,
            limits.typed.callable.function,
            Some(super::function::FunctionEvidenceMode {
                callable: limits.typed.callable,
                expressions: Some(limits.typed.expressions),
                bindings: Some(limits.bindings),
            }),
        )?;
        Self::scoped_result(result)
    }
    pub(super) fn check_scoped_callable_function_syntax(
        &self,
        label: &str,
        syntax: super::EnumFunctionSyntax,
        limits: ScopedCallableLimits,
    ) -> Result<CheckedScopedCallableFunction, EnumFunctionError> {
        let result = self.check_function_syntax_mode(
            label,
            syntax,
            limits.typed.callable.function,
            Some(super::function::FunctionEvidenceMode {
                callable: limits.typed.callable,
                expressions: Some(limits.typed.expressions),
                bindings: Some(limits.bindings),
            }),
        )?;
        Self::scoped_result(result)
    }
    pub(super) fn check_program_callable_syntax(
        &self,
        label: &str,
        syntax: super::EnumFunctionSyntax,
        limits: ScopedCallableLimits,
        program: std::sync::Arc<super::program_source::ProgramHeaders>,
    ) -> Result<(CheckedScopedCallableFunction, (usize, usize)), EnumFunctionError> {
        let result = self.check_function_syntax_program_mode(
            label,
            syntax,
            limits.typed.callable.function,
            Some(super::function::FunctionEvidenceMode {
                callable: limits.typed.callable,
                expressions: Some(limits.typed.expressions),
                bindings: Some(limits.bindings),
            }),
            Some(program),
        )?;
        let usage = result.signature_usage;
        Ok((Self::scoped_result(result)?, usage))
    }
    fn scoped_result(
        result: super::function::FunctionCheckResult,
    ) -> Result<CheckedScopedCallableFunction, EnumFunctionError> {
        Ok(CheckedScopedCallableFunction {
            typed: CheckedTypedCallableFunction {
                callable: super::CheckedCallableFunction {
                    function: result.function,
                    evidence: result.evidence.expect("scoped mode records calls"),
                },
                types: result.types.expect("scoped mode records types"),
            },
            bindings: result.bindings.expect("scoped mode records bindings"),
        })
    }
}
