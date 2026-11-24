//! Sealed generic function signature checking; bodies remain explicitly opaque.
use super::{
    generic_aliases::translate, generic_resolution::primitive, CheckedGenericDeclarations,
    ExpandedGenericType, GenericAliasExpansionError, GenericAliasExpansionLimits,
    GenericFunctionSyntax,
};
use crate::compiler::{diagnostics::Span, stdlib};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
static NEXT_SIGNATURES: AtomicU64 = AtomicU64::new(1);
/// Whole signature-table retention and cumulative expansion admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSignatureLimits {
    /// Function records and duplicate-name index entries.
    pub max_functions: usize,
    /// Total value parameters across all functions.
    pub max_value_parameters: usize,
    /// Referenced function/type-parameter/value-parameter/annotation syntax records.
    pub max_metadata_nodes: usize,
    /// Referenced full/fragment source, including opaque bodies in full source only.
    pub max_source_bytes: usize,
    /// Referenced syntax names and copied function-name wrappers.
    pub max_name_bytes: usize,
    /// Generated type nodes across all value/return annotations.
    pub max_expanded_nodes: usize,
    /// Generated type edges across all value/return annotations.
    pub max_expanded_edges: usize,
    /// Function/scope/member checks and annotation expansion steps.
    pub max_work: usize,
    /// Independent per-annotation expansion caps, reduced by remaining totals.
    pub expansion: GenericAliasExpansionLimits,
}
impl GenericSignatureLimits {
    /// Finite root signature profile; generic function execution remains unsupported.
    pub const DEFAULT: Self = Self {
        max_functions: 128,
        max_value_parameters: 4096,
        max_metadata_nodes: 65_536,
        max_source_bytes: 4 * 1024 * 1024,
        max_name_bytes: 1024 * 1024,
        max_expanded_nodes: 65_536,
        max_expanded_edges: 65_536,
        max_work: 1_000_000,
        expansion: GenericAliasExpansionLimits::DEFAULT,
    };
}
/// Function identity owned by one complete checked signature table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericSignatureId {
    owner: u64,
    ordinal: usize,
}
impl GenericSignatureId {
    /// Source/input-order ordinal within its own table only.
    pub fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Where a failed explicit annotation occurs in its function header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericSignaturePosition {
    /// Source-order value parameter.
    Value(usize),
    /// Explicit return annotation.
    Return,
}
/// Complete signature-table refusal, without a partially checked result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericSignatureError {
    /// Finite logical admission exhausted.
    Limit(&'static str),
    /// Invalid function/scope identity/name convention in original full-source coordinates.
    Definition {
        /// Input function index.
        function: usize,
        /// Original full-source name or type-parameter span.
        span: Span,
        /// Fixed bounded explanation.
        reason: &'static str,
    },
    /// An explicit annotation failed in its own fragment/alias provenance context.
    Annotation {
        /// Input function index.
        function: usize,
        /// Value parameter or return position.
        position: GenericSignaturePosition,
        /// Original full-source input-fragment range.
        annotation_range: Span,
        /// Preserved bounded inner error; alias origins use their own target source.
        error: Box<GenericAliasExpansionError>,
    },
}
impl std::fmt::Display for GenericSignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic signature checking refused: {self:?}")
    }
}
impl std::error::Error for GenericSignatureError {}
/// Preserve the implicit Unit convention without inventing a source annotation.
#[derive(Debug, Clone)]
pub enum GenericSignatureReturn {
    /// No written annotation; semantic return convention is Unit.
    ImplicitUnit,
    /// Complete symbolic expanded explicit annotation.
    Explicit(ExpandedGenericType),
}
/// Sealed signature proof with original opaque body; not a checked function body.
#[derive(Debug, Clone)]
pub struct CheckedGenericSignature {
    id: GenericSignatureId,
    types: CheckedGenericDeclarations,
    syntax: GenericFunctionSyntax,
    values: Arc<[ExpandedGenericType]>,
    return_type: GenericSignatureReturn,
}
impl CheckedGenericSignature {
    /// Table-owned function identity.
    pub fn id(&self) -> GenericSignatureId {
        self.id
    }
    /// Complete checked type-definition context retained by this signature.
    pub fn types(&self) -> &CheckedGenericDeclarations {
        &self.types
    }
    /// Original source/header and explicitly opaque body syntax.
    pub fn syntax(&self) -> &GenericFunctionSyntax {
        &self.syntax
    }
    /// Source-order complete symbolic value-parameter types.
    pub fn parameter_types(&self) -> &[ExpandedGenericType] {
        &self.values
    }
    /// Explicit expanded type or the implicit Unit convention.
    pub fn return_type(&self) -> &GenericSignatureReturn {
        &self.return_type
    }
}
/// Complete immutable function signatures in supplied source order.
#[derive(Debug, Clone)]
pub struct CheckedGenericSignatures {
    owner: u64,
    types: CheckedGenericDeclarations,
    signatures: Arc<[CheckedGenericSignature]>,
    expanded_nodes: usize,
    expanded_edges: usize,
    work: usize,
}
impl CheckedGenericSignatures {
    /// Complete retained type-definition context.
    pub fn types(&self) -> &CheckedGenericDeclarations {
        &self.types
    }
    /// Checked signatures, including functions with no callers.
    pub fn signatures(&self) -> &[CheckedGenericSignature] {
        &self.signatures
    }
    /// Resolve a root function name within this table; linear in admitted function count.
    pub fn find(&self, name: &str) -> Option<GenericSignatureId> {
        if self.types.environment().registry().is_package_scoped() {
            return None;
        }
        self.signatures
            .iter()
            .find(|s| s.syntax.name() == name)
            .map(|s| s.id)
    }
    /// Inspect an ID only when it belongs to this table.
    pub fn signature(&self, id: GenericSignatureId) -> Option<&CheckedGenericSignature> {
        if id.owner != self.owner {
            return None;
        }
        self.signatures.get(id.ordinal)
    }
    /// Cumulative generated annotation nodes.
    pub fn expanded_nodes(&self) -> usize {
        self.expanded_nodes
    }
    /// Cumulative generated annotation edges.
    pub fn expanded_edges(&self) -> usize {
        self.expanded_edges
    }
    /// Cumulative logical checking steps.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    n: usize,
    dimension: &'static str,
) -> Result<(), GenericSignatureError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericSignatureError::Limit(dimension))?;
    Ok(())
}
struct Admission {
    values: usize,
    metadata: usize,
    source: usize,
    names: usize,
    nodes: usize,
    edges: usize,
    work: usize,
}
struct Annotation<'a> {
    function: usize,
    position: GenericSignaturePosition,
    range: Span,
    syntax: &'a super::GenericTypeSyntax,
    parameters: &'a super::GenericParameters,
}
impl Admission {
    fn annotation(
        &mut self,
        types: &CheckedGenericDeclarations,
        input: Annotation<'_>,
        mut expansion: GenericAliasExpansionLimits,
    ) -> Result<ExpandedGenericType, GenericSignatureError> {
        expansion.max_nodes = expansion.max_nodes.min(self.nodes);
        expansion.max_edges = expansion.max_edges.min(self.edges);
        expansion.max_work = expansion.max_work.min(self.work);
        let checked = types
            .environment()
            .expand(input.syntax, input.parameters, expansion)
            .map_err(|error| GenericSignatureError::Annotation {
                function: input.function,
                position: input.position,
                annotation_range: input.range,
                error: Box::new(error),
            })?;
        charge(
            &mut self.nodes,
            checked.nodes().len(),
            "signature expanded nodes",
        )?;
        charge(&mut self.edges, checked.edges(), "signature expanded edges")?;
        charge(&mut self.work, checked.work(), "signature work")?;
        Ok(checked)
    }
}
/// Check every supplied root generic function's explicit annotations against a
/// complete checked type context. This never parses/checks/lowers its body and
/// cannot produce a callable executable function or application entry.
pub fn check_generic_signatures(
    types: &CheckedGenericDeclarations,
    functions: &[&GenericFunctionSyntax],
    limits: GenericSignatureLimits,
) -> Result<CheckedGenericSignatures, GenericSignatureError> {
    if functions.len() > limits.max_functions {
        return Err(GenericSignatureError::Limit("generic functions"));
    }
    let mut admission = Admission {
        values: limits.max_value_parameters,
        metadata: limits.max_metadata_nodes,
        source: limits.max_source_bytes,
        names: limits.max_name_bytes,
        nodes: limits.max_expanded_nodes,
        edges: limits.max_expanded_edges,
        work: limits.max_work,
    };
    let mut signatures = Vec::new();
    let mut names = BTreeSet::new();
    for (function, syntax) in functions.iter().enumerate() {
        charge(&mut admission.work, 1, "signature work")?;
        let definition = |span, reason| GenericSignatureError::Definition {
            function,
            span,
            reason,
        };
        let scope = types
            .environment()
            .registry()
            .function_scope(syntax.parameters())
            .ok_or_else(|| definition(syntax.name_span(), "foreign package function scope"))?;
        if !names.insert((scope, syntax.name())) {
            return Err(definition(syntax.name_span(), "duplicate generic function"));
        }
        if stdlib::lookup(syntax.name()).is_some() {
            return Err(definition(
                syntax.name_span(),
                "function conflicts with a standard callable",
            ));
        }
        if syntax.name() == "main" && !syntax.parameters().parameters().is_empty() {
            return Err(definition(
                syntax.name_span(),
                "application entry cannot require generic type arguments",
            ));
        }
        let metadata = syntax
            .type_nodes()
            .checked_add(syntax.value_parameters().len())
            .and_then(|n| n.checked_add(syntax.parameters().parameters().len()))
            .and_then(|n| n.checked_add(1))
            .ok_or(GenericSignatureError::Limit("signature metadata"))?;
        charge(
            &mut admission.values,
            syntax.value_parameters().len(),
            "signature value parameters",
        )?;
        charge(&mut admission.metadata, metadata, "signature metadata")?;
        charge(
            &mut admission.source,
            syntax.retained_source_bytes(),
            "signature source",
        )?;
        charge(
            &mut admission.names,
            syntax
                .name_bytes()
                .checked_add(syntax.name().len())
                .ok_or(GenericSignatureError::Limit("signature names"))?,
            "signature names",
        )?;
        for parameter in syntax.parameters().parameters() {
            charge(&mut admission.work, 1, "signature work")?;
            if primitive(&parameter.name).is_some() {
                return Err(definition(
                    translate(parameter.name_span, syntax.parameter_range()),
                    "type parameter conflicts with a primitive",
                ));
            }
        }
        let mut values = Vec::new();
        for (index, value) in syntax.value_parameters().iter().enumerate() {
            charge(&mut admission.work, 1, "signature work")?;
            values.push(admission.annotation(
                types,
                Annotation {
                    function,
                    position: GenericSignaturePosition::Value(index),
                    range: value.ty.type_range,
                    syntax: value.ty.annotation.syntax(),
                    parameters: syntax.parameters(),
                },
                limits.expansion,
            )?);
        }
        let return_type = if let Some(ty) = syntax.return_type() {
            charge(&mut admission.work, 1, "signature work")?;
            GenericSignatureReturn::Explicit(admission.annotation(
                types,
                Annotation {
                    function,
                    position: GenericSignaturePosition::Return,
                    range: ty.type_range,
                    syntax: ty.annotation.syntax(),
                    parameters: syntax.parameters(),
                },
                limits.expansion,
            )?)
        } else {
            GenericSignatureReturn::ImplicitUnit
        };
        signatures.push(CheckedGenericSignature {
            id: GenericSignatureId {
                owner: 0,
                ordinal: function,
            },
            types: types.clone(),
            syntax: (*syntax).clone(),
            values: values.into(),
            return_type,
        });
    }
    let owner = loop {
        let current = NEXT_SIGNATURES.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .ok_or(GenericSignatureError::Limit("signature identities"))?;
        if NEXT_SIGNATURES
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break current;
        }
    };
    for signature in &mut signatures {
        signature.id.owner = owner;
    }
    Ok(CheckedGenericSignatures {
        owner,
        types: types.clone(),
        signatures: signatures.into(),
        expanded_nodes: limits.max_expanded_nodes - admission.nodes,
        expanded_edges: limits.max_expanded_edges - admission.edges,
        work: limits.max_work - admission.work,
    })
}
