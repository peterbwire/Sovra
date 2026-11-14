//! Opt-in constructor/self-call checking and bounded source-owned call/context facts.
use super::{
    CheckedEnumFunction, CheckedEnumTypes, EnumFunctionError, EnumFunctionLimits,
    EnumFunctionSyntax, EnumId,
};
use crate::compiler::{
    ast::{Expression, Function, Parameter},
    checked::CoercionKind,
    diagnostics::Span,
    semantic::Type,
};
use std::collections::HashMap;
/// Additional admission for constructor signatures and call/context evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallableLimits {
    /// Existing single-function source admission.
    pub function: EnumFunctionLimits,
    /// All enum variant signatures plus the current function signature.
    pub max_signatures: usize,
    /// Owned signature names, lookup keys, annotations and parameter names.
    pub max_signature_text_bytes: usize,
    /// Retained resolved call heads, including standard calls.
    pub max_calls: usize,
    /// Retained explicit core coercion/type contexts, including identity contexts.
    pub max_contexts: usize,
    /// Cumulative scalar/named/array nodes copied into actual/expected context types.
    pub max_type_nodes: usize,
    /// Cumulative nominal-name bytes copied into context types.
    pub max_type_text_bytes: usize,
}
impl CallableLimits {
    /// Opt-in defaults; existing APIs and command profiles remain unchanged.
    pub const DEFAULT: Self = Self {
        function: EnumFunctionLimits::DEFAULT,
        max_signatures: 16_384,
        max_signature_text_bytes: 4 * 1024 * 1024,
        max_calls: 100_000,
        max_contexts: 250_000,
        max_type_nodes: 1_000_000,
        max_type_text_bytes: 16 * 1024 * 1024,
    };
}
/// Target resolved by core call checking in this retained source/environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallableTarget {
    /// Canonical standard-library identity, including canonicalization of aliases.
    Standard(&'static str),
    /// Current function; no external declaration is implied.
    SelfFunction,
    /// A source-program-owned target; requires linked-program execution support.
    ProgramFunction(super::ProgramFunctionId),
    /// Nominal constructor, with its environment-local declaration/variant identity.
    Constructor {
        /// Checked enum identity.
        enum_id: EnumId,
        /// Variant ordinal in the checked declaration.
        variant: usize,
    },
}
/// One resolved call head; spans belong to this checked function source only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallableCall {
    /// Original call-head range, excluding arguments.
    pub span: Span,
    /// Resolved target, never a source spelling lookup at execution time.
    pub target: CallableTarget,
}
/// Explicit core conversion/type context; identity and empty-array contexts remain useful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableContext {
    /// Original value range, within this immutable source syntax.
    pub value_span: Span,
    /// Existing core context category.
    pub kind: CoercionKind,
    /// Core actual type (including existing empty-array Unknown holes).
    pub actual: Type,
    /// Required core type; numeric widening follows existing compatibility rules.
    pub expected: Type,
}
/// Immutable opt-in call/type-context facts; not complete lowering-ready evidence.
#[derive(Debug, Clone, Default)]
pub struct CallableEvidence {
    calls: Vec<CallableCall>,
    contexts: Vec<CallableContext>,
}
impl CallableEvidence {
    /// Resolved call heads in semantic visitation order, not necessarily source order.
    pub fn calls(&self) -> &[CallableCall] {
        &self.calls
    }
    /// Explicit core coercion sites; array elements are included by the typed opt-in API.
    pub fn contexts(&self) -> &[CallableContext] {
        &self.contexts
    }
}
/// Checked single-function source with nominal constructors and self-call signatures.
#[derive(Debug, Clone)]
pub struct CheckedCallableFunction {
    pub(super) function: CheckedEnumFunction,
    pub(super) evidence: CallableEvidence,
}
impl CheckedCallableFunction {
    /// Source/scopes/return coverage retained by the function checker.
    pub fn function(&self) -> &CheckedEnumFunction {
        &self.function
    }
    /// Bounded call and conversion-context facts.
    pub fn evidence(&self) -> &CallableEvidence {
        &self.evidence
    }
}
pub(crate) struct RecordedCallable {
    pub evidence: CallableEvidence,
    pub types: Option<super::TypedCallableEvidence>,
    pub bindings: Option<super::ScopedCallableEvidence>,
}
#[derive(Debug)]
pub(crate) struct CallRecorder {
    limits: CallableLimits,
    targets: HashMap<String, CallableTarget>,
    evidence: CallableEvidence,
    nodes: usize,
    text: usize,
    exhausted: bool,
    typed: Option<super::typed_callable::TypedRecorder>,
    scoped: Option<super::scoped_callable::ScopedRecorder>,
}
impl CallRecorder {
    pub(crate) fn new(limits: CallableLimits, targets: HashMap<String, CallableTarget>) -> Self {
        Self {
            limits,
            targets,
            evidence: CallableEvidence::default(),
            nodes: 0,
            text: 0,
            exhausted: false,
            typed: None,
            scoped: None,
        }
    }
    pub(super) fn enable_types(
        &mut self,
        limits: super::ExpressionEvidenceLimits,
    ) -> Result<(), EnumFunctionError> {
        self.typed = Some(super::typed_callable::TypedRecorder::new(limits)?);
        Ok(())
    }
    pub(super) fn enable_scopes(&mut self, limits: super::BindingEvidenceLimits) {
        let Some(unit) = self.typed.as_ref().map(|typed| typed.unit()) else {
            self.exhausted = true;
            return;
        };
        self.scoped = super::scoped_callable::ScopedRecorder::new(unit, limits);
        if self.scoped.is_none() {
            self.exhausted = true;
        }
    }
    pub(crate) fn has_scopes(&self) -> bool {
        self.scoped.is_some() && !self.exhausted
    }
    pub(crate) fn enter_scope(&mut self) {
        if !self.exhausted {
            if let Some(scoped) = &mut self.scoped {
                if !scoped.enter() {
                    self.exhausted = true;
                }
            }
        }
    }
    pub(crate) fn exit_scope(&mut self) {
        if !self.exhausted {
            if let Some(scoped) = &mut self.scoped {
                if !scoped.exit() {
                    self.exhausted = true;
                }
            }
        }
    }
    pub(crate) fn declare(
        &mut self,
        name: &str,
        span: Span,
        kind: super::ScopedDeclarationKind,
        ty: &Type,
        named: &HashMap<String, Type>,
    ) {
        if self.exhausted {
            return;
        }
        if let Some(scoped) = &mut self.scoped {
            if !scoped.declare(name, span, kind, ty, named) {
                self.exhausted = true;
            }
        }
    }
    pub(crate) fn variable(
        &mut self,
        value: &Expression,
        name: &str,
        kind: crate::compiler::checked::UseKind,
    ) {
        if self.exhausted {
            return;
        }
        let Some(kind) = super::scoped_callable::ScopedRecorder::variable_kind(kind) else {
            return;
        };
        let id = if kind == super::ScopedUseKind::Read {
            self.typed
                .as_ref()
                .and_then(|typed| typed.expression_id(value))
        } else {
            None
        };
        if let Some(scoped) = &mut self.scoped {
            if !scoped.usage(name, value.span, kind, id) {
                self.exhausted = true;
            }
        }
    }
    pub(crate) fn scrutinee(&mut self, name: &str, span: Span, match_index: usize) {
        if self.exhausted {
            return;
        }
        if let Some(scoped) = &mut self.scoped {
            if !scoped.usage(
                name,
                span,
                super::ScopedUseKind::MatchScrutinee { match_index },
                None,
            ) {
                self.exhausted = true;
            }
        }
    }
    pub(crate) fn expression(
        &mut self,
        value: &Expression,
        ty: &Type,
        names: &HashMap<String, Type>,
    ) {
        if self.exhausted {
            return;
        }
        if let Some(typed) = &mut self.typed {
            if !typed.expression(value, ty, names) {
                self.exhausted = true;
            }
        }
    }
    pub(crate) fn array_items(
        &mut self,
        items: &[Expression],
        element: &Type,
        names: &HashMap<String, Type>,
    ) {
        if self.exhausted || self.typed.is_none() {
            return;
        }
        for value in items {
            let Some(actual) = self
                .typed
                .as_ref()
                .and_then(|typed| typed.ty(value))
                .cloned()
            else {
                self.exhausted = true;
                return;
            };
            self.context(value, CoercionKind::ArrayElement, &actual, element, names);
            if self.exhausted {
                return;
            }
        }
    }
    pub(crate) fn call(&mut self, head: &Expression, name: &str) {
        let Some(target) = self.targets.get(name).copied() else {
            self.exhausted = true;
            return;
        };
        self.push_call(head, target);
    }
    pub(crate) fn builtin(&mut self, head: &Expression, name: &'static str) {
        self.push_call(head, CallableTarget::Standard(name));
    }
    fn push_call(&mut self, head: &Expression, target: CallableTarget) {
        if self.exhausted {
            return;
        }
        if self.evidence.calls.len() >= self.limits.max_calls {
            self.exhausted = true;
            return;
        }
        if let Some(typed) = &mut self.typed {
            if !typed.head(head, self.evidence.calls.len()) {
                self.exhausted = true;
                return;
            }
        }
        self.evidence.calls.push(CallableCall {
            span: head.span,
            target,
        });
    }
    pub(crate) fn context(
        &mut self,
        value: &Expression,
        kind: CoercionKind,
        actual: &Type,
        expected: &Type,
        names: &HashMap<String, Type>,
    ) {
        if self.exhausted {
            return;
        }
        if self.evidence.contexts.len() >= self.limits.max_contexts {
            self.exhausted = true;
            return;
        }
        // Canonicalization is bounded core scratch, before retained-copy admission.
        let actual = actual.canonicalize(names);
        let expected = expected.canonicalize(names);
        for ty in [&actual, &expected] {
            let mut leaf = ty;
            loop {
                let Some(nodes) = self
                    .nodes
                    .checked_add(1)
                    .filter(|n| *n <= self.limits.max_type_nodes)
                else {
                    self.exhausted = true;
                    return;
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
                            self.exhausted = true;
                            return;
                        };
                        self.text = text;
                        break;
                    }
                    _ => break,
                }
            }
        }
        if let Some(typed) = &mut self.typed {
            if !typed.context(value, self.evidence.contexts.len()) {
                self.exhausted = true;
                return;
            }
        }
        self.evidence.contexts.push(CallableContext {
            value_span: value.span,
            kind,
            actual,
            expected,
        });
    }
    pub(crate) fn finish(self) -> Result<RecordedCallable, EnumFunctionError> {
        if self.exhausted {
            Err(EnumFunctionError::Limit("callable evidence"))
        } else {
            let bindings = self
                .scoped
                .map(|scoped| {
                    scoped
                        .finish()
                        .ok_or(EnumFunctionError::Limit("binding scope identity"))
                })
                .transpose()?;
            Ok(RecordedCallable {
                evidence: self.evidence,
                types: self.typed.map(|typed| typed.finish()),
                bindings,
            })
        }
    }
}
pub(super) fn spelling(ty: &Type) -> String {
    let mut depth = 0;
    let mut leaf = ty;
    while let Type::Array(item) = leaf {
        depth += 1;
        leaf = item;
    }
    let name = match leaf {
        Type::Unit => "Unit",
        Type::Bool => "Bool",
        Type::Int => "Int",
        Type::Float => "Float",
        Type::String => "String",
        Type::Named(name) => name,
        _ => unreachable!("validated payload"),
    };
    let mut text = String::new();
    for _ in 0..depth {
        text.push('[');
    }
    text.push_str(name);
    for _ in 0..depth {
        text.push(']');
    }
    text
}
pub(super) struct Signatures {
    pub functions: Vec<Function>,
    pub targets: HashMap<String, CallableTarget>,
    pub text_bytes: usize,
}
pub(super) fn signatures(
    env: &CheckedEnumTypes,
    syntax: &EnumFunctionSyntax,
    limits: CallableLimits,
    program: Option<&super::program_source::ProgramHeaders>,
) -> Result<Signatures, EnumFunctionError> {
    let visible = |header: &EnumFunctionSyntax| {
        header.name.rsplit_once("::").map_or(true, |(module, _)| {
            header.is_exported || env.module() == Some(module)
        })
    };
    let mut count = program.map_or(1, |p| {
        p.functions
            .iter()
            .filter(|(_, header)| visible(header))
            .count()
    });
    for declaration in env.declarations() {
        count = count
            .checked_add(declaration.variants.len())
            .filter(|n| *n <= limits.max_signatures)
            .ok_or(EnumFunctionError::Limit("callable signatures"))?;
    }
    // Alias targets are already flattened by validated type resolution. Count
    // every generated header before retaining/sorting alias spelling records.
    for (_, id) in env.enum_aliases() {
        count = count
            .checked_add(env.declarations()[id.ordinal()].variants.len())
            .filter(|n| *n <= limits.max_signatures)
            .ok_or(EnumFunctionError::Limit("callable signatures"))?;
    }
    if count > limits.max_signatures {
        return Err(EnumFunctionError::Limit("callable signatures"));
    }
    let mut bytes = 0usize;
    let mut charge = |n: usize| -> Result<(), EnumFunctionError> {
        bytes = bytes
            .checked_add(n)
            .filter(|n| *n <= limits.max_signature_text_bytes)
            .ok_or(EnumFunctionError::Limit("signature text"))?;
        Ok(())
    };
    let mut functions = Vec::new();
    let mut targets = HashMap::new();
    let headers: Vec<_> = if let Some(program) = program {
        program
            .functions
            .iter()
            .filter(|(_, header)| visible(header))
            .map(|(id, f)| (CallableTarget::ProgramFunction(*id), f))
            .collect()
    } else {
        vec![(CallableTarget::SelfFunction, syntax)]
    };
    for (target, header) in headers {
        for _ in 0..3 {
            charge(header.name.len())?;
        }
        for parameter in &header.parameters {
            charge(parameter.name.len())?;
            charge(parameter.type_name.as_deref().unwrap_or("").len())?;
        }
        charge(header.return_type.as_deref().unwrap_or("").len())?;
        functions.push(Function {
            name: header.name.clone(),
            is_exported: header.is_exported,
            parameters: header.parameters.clone(),
            return_type: header.return_type.clone(),
            return_type_span: header.return_type_span,
            body: Vec::new(),
            span: header.span,
        });
        targets.insert(header.name.clone(), target);
    }
    let mut spellings: Vec<_> = env
        .declarations()
        .iter()
        .map(|d| (d.name.as_str(), d.id))
        .collect();
    let mut aliases: Vec<_> = env.enum_aliases().collect();
    aliases.sort_unstable_by(|a, b| a.0.cmp(b.0));
    spellings.extend(aliases);
    for (constructor_prefix, id) in spellings {
        let declaration = &env.declarations()[id.ordinal()];
        for (variant_index, variant) in declaration.variants.iter().enumerate() {
            let size = constructor_prefix
                .len()
                .checked_add(2)
                .and_then(|n| n.checked_add(variant.name.len()))
                .ok_or(EnumFunctionError::Limit("signature text"))?;
            for _ in 0..3 {
                charge(size)?;
            }
            let name = format!("{}::{}", constructor_prefix, variant.name);
            let collision = if crate::compiler::stdlib::lookup(&name).is_some() {
                Some("a standard callable")
            } else if targets.contains_key(&name) {
                Some("a declared callable")
            } else {
                None
            };
            if let Some(collision) = collision {
                return Err(EnumFunctionError::Coverage(
                    super::EnumTypeError::Diagnostics(vec![super::EnumTypeDiagnostic {
                        source_label: std::sync::Arc::clone(&declaration.source_label),
                        diagnostic: crate::compiler::diagnostics::Diagnostic {
                            source_file: None,
                            severity: crate::compiler::diagnostics::Severity::Error,
                            code: "E3110",
                            message: crate::compiler::diagnostics::format_message(format_args!(
                                "enum constructor `{name}` collides with {collision}"
                            )),
                            span: variant.span,
                        },
                    }]),
                ));
            }
            charge(declaration.name.len())?;
            let mut parameters = Vec::new();
            if let Some(payload) = variant.payload {
                let annotation = spelling(
                    &env.body_payload_type(payload)
                        .map_err(EnumFunctionError::Coverage)?,
                );
                charge("payload".len())?;
                charge(annotation.len())?;
                parameters.push(Parameter {
                    name: "payload".into(),
                    type_name: Some(annotation),
                    type_span: None,
                    span: variant.span,
                });
            }
            targets.insert(
                name.clone(),
                CallableTarget::Constructor {
                    enum_id: declaration.id,
                    variant: variant_index,
                },
            );
            functions.push(Function {
                name,
                is_exported: declaration.is_exported,
                parameters,
                return_type: Some(declaration.name.clone()),
                return_type_span: None,
                body: Vec::new(),
                span: variant.span,
            });
        }
    }
    Ok(Signatures {
        functions,
        targets,
        text_bytes: bytes,
    })
}
impl CheckedEnumTypes {
    /// Opt into canonical enum constructor calls and self recursion in one source function.
    /// Private signature headers are checking inputs, never executable v1 programs.
    /// Other user functions, package visibility and runtime/lowering remain unavailable.
    pub fn check_callable_function_source(
        &self,
        label: &str,
        source: &str,
        limits: CallableLimits,
    ) -> Result<CheckedCallableFunction, EnumFunctionError> {
        let result = self.check_function_source_mode(
            label,
            source,
            limits.function,
            Some(super::function::FunctionEvidenceMode {
                callable: limits,
                expressions: None,
                bindings: None,
            }),
        )?;
        Ok(CheckedCallableFunction {
            function: result.function,
            evidence: result.evidence.expect("callable mode records evidence"),
        })
    }
}
