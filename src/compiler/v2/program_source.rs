//! Bounded root-program checking and canonical cross-function links; execution is in program_execution.
use super::*;
use crate::compiler::diagnostics::Span;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
static NEXT_PROGRAM: AtomicU64 = AtomicU64::new(1);
/// Identity owned by one successful source-program request, never a source-offset key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramFunctionId {
    unit: u64,
    ordinal: usize,
}
impl ProgramFunctionId {
    /// Source-order function index within this program only.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
#[derive(Debug)]
pub(super) struct ProgramHeaders {
    pub functions: Vec<(ProgramFunctionId, EnumFunctionSyntax)>,
}
/// Program-specific cumulative admission, alongside existing per-function quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramSourceLimits {
    /// Whole source/type projection and per-function checking limits.
    pub source: EntrySourceLimits,
    /// Total source functions, before retaining each body projection.
    pub max_functions: usize,
    /// Header records retained once plus all per-body signature copies, constructors included.
    pub max_signature_copies: usize,
    /// Initial owned headers plus all per-body signature text under existing charging rules.
    pub max_signature_text_bytes: usize,
    /// Aggregate retained call/context/expression/link/declaration/use facts across functions.
    pub max_evidence_items: usize,
    /// Retained canonical program-call links, including self-recursion.
    pub max_call_links: usize,
}
impl ProgramSourceLimits {
    /// Finite opt-in profile; earlier single-entry APIs keep their original scope.
    pub const DEFAULT: Self = Self {
        source: EntrySourceLimits::DEFAULT,
        max_functions: 128,
        max_signature_copies: 65_536,
        max_signature_text_bytes: 64 * 1024 * 1024,
        max_evidence_items: 1_000_000,
        max_call_links: 100_000,
    };
}
/// Source-owned caller/callee identities checked against the same immutable headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramCallLink {
    /// Calling function.
    pub caller: ProgramFunctionId,
    /// Canonical called function, never resolved by spelling at runtime.
    pub callee: ProgramFunctionId,
    /// Original call-head coordinates in this source.
    pub span: Span,
}
/// Entire-program refusal; no unchecked or partially linked functions are returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramSourceError {
    /// Source projection or nominal type validation failed.
    Source(EntrySourceError),
    /// Duplicate root function declaration.
    DuplicateFunction(Span),
    /// One function's signature/body/evidence was invalid, including unused bodies.
    Function {
        /// Original function index.
        function: usize,
        /// Existing checker refusal.
        error: EnumFunctionError,
    },
    /// Program admission failed without resetting a cumulative allowance.
    Limit(&'static str),
    /// Retained identity did not belong to this header table.
    InvalidLink,
}
impl std::fmt::Display for ProgramSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "program source refused: {self:?}")
    }
}
impl std::error::Error for ProgramSourceError {}
/// Checked root functions/call links; opt-in linked execution uses ProgramLoweringLimits.
#[derive(Debug)]
pub struct CheckedProgramSource {
    source: Arc<str>,
    label: Arc<str>,
    declarations: Vec<EntryDeclaration>,
    headers: Arc<ProgramHeaders>,
    functions: Vec<CheckedScopedCallableFunction>,
    links: Vec<ProgramCallLink>,
    signature_copies: usize,
    signature_text_bytes: usize,
    evidence_items: usize,
}
impl CheckedProgramSource {
    /// Original complete admitted source.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Opaque source label, never filesystem authority.
    pub fn source_label(&self) -> &str {
        &self.label
    }
    /// Supported declaration coverage in original source order.
    pub fn declarations(&self) -> &[EntryDeclaration] {
        &self.declarations
    }
    /// Checked bodies, in source order, including functions never called by main.
    pub fn functions(&self) -> &[CheckedScopedCallableFunction] {
        &self.functions
    }
    /// Resolve a root function spelling to its checked identity for inspection/entry selection.
    pub fn function_id(&self, name: &str) -> Option<ProgramFunctionId> {
        self.headers
            .functions
            .iter()
            .find(|(_, f)| f.name == name)
            .map(|(id, _)| *id)
    }
    /// Source-order identity lookup, including qualified module functions.
    pub fn function_id_at(&self, ordinal: usize) -> Option<ProgramFunctionId> {
        self.headers.functions.get(ordinal).map(|(id, _)| *id)
    }
    /// Canonical source-program name for an owned function ID.
    pub fn function_name(&self, id: ProgramFunctionId) -> Option<&str> {
        self.headers
            .functions
            .get(id.ordinal)
            .filter(|(actual, _)| *actual == id)
            .map(|(_, f)| f.name.as_str())
    }
    /// Lookup refuses IDs from another independently checked program.
    pub fn function(&self, id: ProgramFunctionId) -> Option<&CheckedScopedCallableFunction> {
        self.headers
            .functions
            .get(id.ordinal)
            .filter(|(actual, _)| *actual == id)?;
        self.functions.get(id.ordinal)
    }
    /// Canonical retained call links; standard/constructor calls remain in per-function evidence.
    pub fn call_links(&self) -> &[ProgramCallLink] {
        &self.links
    }
    /// Actual cumulatively charged initial/per-body signature records.
    pub fn signature_copies(&self) -> usize {
        self.signature_copies
    }
    /// Actual cumulatively charged header/signature text.
    pub fn signature_text_bytes(&self) -> usize {
        self.signature_text_bytes
    }
    /// Aggregate admitted retained evidence items, independent of type-tree quotas.
    pub fn evidence_items(&self) -> usize {
        self.evidence_items
    }
}
fn charge(remaining: &mut usize, n: usize, kind: &'static str) -> Result<(), ProgramSourceError> {
    *remaining = remaining
        .checked_sub(n)
        .ok_or(ProgramSourceError::Limit(kind))?;
    Ok(())
}
fn header_bytes(f: &EnumFunctionSyntax) -> Option<usize> {
    f.parameters.iter().try_fold(
        f.name
            .len()
            .checked_add(f.return_type.as_deref().unwrap_or("").len())?,
        |n, p| {
            n.checked_add(p.name.len())?
                .checked_add(p.type_name.as_deref().unwrap_or("").len())
        },
    )
}
fn evidence_items(f: &CheckedScopedCallableFunction) -> Option<usize> {
    let typed = f.typed();
    let calls = typed.callable().evidence();
    [
        calls.calls().len(),
        calls.contexts().len(),
        typed.types().expressions().len(),
        typed.types().calls().len(),
        typed.types().contexts().len(),
        f.bindings().declarations().len(),
        f.bindings().uses().len(),
    ]
    .into_iter()
    .try_fold(0usize, usize::checked_add)
}
/// Check every root body against one source-owned signature/nominal environment.
/// Forward/mutual calls retain canonical IDs. Modules/imports remain unavailable.
/// Use checked-program execution for links; standalone IR execution still refuses them.
pub fn check_program_source(
    label: &str,
    source: &str,
    limits: ProgramSourceLimits,
) -> Result<CheckedProgramSource, ProgramSourceError> {
    let parts = super::entry_source::parse_source_parts(
        label,
        source,
        limits.source,
        limits.max_functions,
        true,
        true,
    )
    .map_err(ProgramSourceError::Source)?;
    let mut unit = NEXT_PROGRAM.load(Ordering::Relaxed);
    loop {
        let next = unit
            .checked_add(1)
            .ok_or(ProgramSourceError::Limit("program identities"))?;
        match NEXT_PROGRAM.compare_exchange_weak(unit, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => unit = actual,
        }
    }
    let mut signatures = limits.max_signature_copies;
    let mut text = limits.max_signature_text_bytes;
    charge(&mut signatures, parts.functions.len(), "program signatures")?;
    let mut facts = limits.max_evidence_items;
    let mut scopes = std::collections::HashMap::new();
    for (index, module) in parts.function_modules.iter().enumerate() {
        if let Some(module) = module {
            if !scopes.contains_key(module.as_ref()) {
                let scoped = parts.environment.scoped(
                    module,
                    &mut facts,
                    &mut text,
                    parts.functions[index].span,
                )?;
                scopes.insert(module.to_string(), scoped);
            }
        }
    }
    let has_modules = !scopes.is_empty();
    let mut names = std::collections::HashSet::new();
    let mut headers = Vec::new();
    for (ordinal, f) in parts.functions.iter().enumerate() {
        let module = parts.function_modules[ordinal].as_deref();
        let size = f
            .name
            .len()
            .checked_add(module.map_or(0, |name| name.len() + 2))
            .ok_or(ProgramSourceError::Limit("signature text"))?;
        if module.is_some() {
            charge(&mut text, size, "module function name")?;
        }
        let canonical =
            module.map_or_else(|| f.name.clone(), |module| format!("{module}::{}", f.name));
        if !names.insert((module, f.name.as_str())) {
            return Err(ProgramSourceError::DuplicateFunction(f.span));
        }
        charge(
            &mut text,
            header_bytes(f).ok_or(ProgramSourceError::Limit("signature text"))?,
            "signature text",
        )?;
        headers.push((
            ProgramFunctionId { unit, ordinal },
            EnumFunctionSyntax {
                name: canonical,
                is_exported: f.is_exported,
                parameters: f.parameters.clone(),
                return_type: f.return_type.clone(),
                return_type_span: f.return_type_span,
                body: Vec::new(),
                span: f.span,
            },
        ));
    }
    if has_modules {
        for (index, (_, header)) in headers.iter_mut().enumerate() {
            let env = parts.function_modules[index]
                .as_deref()
                .and_then(|module| scopes.get(module))
                .unwrap_or(&parts.environment);
            let known = env.body_named_types();
            for annotation in header
                .parameters
                .iter_mut()
                .filter_map(|p| p.type_name.as_mut())
                .chain(header.return_type.iter_mut())
            {
                let ty =
                    crate::compiler::semantic::Type::from_name_with_known(annotation, Some(known))
                        .canonicalize(known);
                if ty != crate::compiler::semantic::Type::Unknown {
                    // Bound the canonical copy before materializing its type spelling.
                    let mut leaf = &ty;
                    let mut bytes = 0usize;
                    while let crate::compiler::semantic::Type::Array(inner) = leaf {
                        bytes = bytes
                            .checked_add(2)
                            .ok_or(ProgramSourceError::Limit("signature text"))?;
                        leaf = inner;
                    }
                    bytes = bytes
                        .checked_add(match leaf {
                            crate::compiler::semantic::Type::Named(name) => name.len(),
                            crate::compiler::semantic::Type::String => 6,
                            crate::compiler::semantic::Type::Float => 5,
                            crate::compiler::semantic::Type::Unit
                            | crate::compiler::semantic::Type::Bool => 4,
                            _ => 3,
                        })
                        .ok_or(ProgramSourceError::Limit("signature text"))?;
                    charge(&mut text, bytes, "canonical signature text")?;
                    *annotation = super::callable::spelling(&ty);
                }
            }
        }
    }
    drop(names);
    let headers = Arc::new(ProgramHeaders { functions: headers });
    let mut functions = Vec::new();
    let mut links = Vec::new();
    for (index, syntax) in parts.functions.into_iter().enumerate() {
        let mut local = limits.source.callable;
        local.typed.callable.max_signatures = local.typed.callable.max_signatures.min(signatures);
        local.typed.callable.max_signature_text_bytes =
            local.typed.callable.max_signature_text_bytes.min(text);
        local.typed.callable.max_calls = local.typed.callable.max_calls.min(facts);
        local.typed.callable.max_contexts = local.typed.callable.max_contexts.min(facts);
        local.typed.expressions.max_expressions =
            local.typed.expressions.max_expressions.min(facts);
        local.bindings.max_declarations = local.bindings.max_declarations.min(facts);
        local.bindings.max_uses = local.bindings.max_uses.min(facts);
        let environment = parts.function_modules[index]
            .as_deref()
            .and_then(|module| scopes.get(module))
            .unwrap_or(&parts.environment);
        let (checked, usage) = environment
            .check_program_callable_syntax(label, syntax, local, Arc::clone(&headers))
            .map_err(|error| ProgramSourceError::Function {
                function: index,
                error,
            })?;
        charge(&mut signatures, usage.0, "program signatures")?;
        charge(&mut text, usage.1, "signature text")?;
        charge(
            &mut facts,
            evidence_items(&checked).ok_or(ProgramSourceError::Limit("program evidence"))?,
            "program evidence",
        )?;
        for call in checked.typed().callable().evidence().calls() {
            if let CallableTarget::ProgramFunction(callee) = call.target {
                if headers.functions.get(callee.ordinal).map(|(id, _)| *id) != Some(callee) {
                    return Err(ProgramSourceError::InvalidLink);
                }
                if links.len() >= limits.max_call_links {
                    return Err(ProgramSourceError::Limit("program call links"));
                }
                links.push(ProgramCallLink {
                    caller: headers.functions[index].0,
                    callee,
                    span: call.span,
                });
            }
        }
        functions.push(checked);
    }
    Ok(CheckedProgramSource {
        source: Arc::from(source),
        label: Arc::from(label),
        declarations: parts.declarations,
        headers,
        functions,
        links,
        signature_copies: limits.max_signature_copies - signatures,
        signature_text_bytes: limits.max_signature_text_bytes - text,
        evidence_items: limits.max_evidence_items - facts,
    })
}
