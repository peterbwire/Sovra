//! Bounded symbolic generic expressions, locals, direct assignment and returns.
use super::generic_data_types::DataResultKind;
use super::{
    bind_generic_type_arguments, check_generic_type_arguments, compare_generic_types,
    instantiate_generic_signature, parse_generic_type, parse_generic_type_arguments,
    CheckedGenericDefinition, CheckedGenericSignature, CheckedGenericSignatures,
    ExpandedGenericType, GenericAliasExpansionError, GenericAliasExpansionLimits,
    GenericArgumentCheckError, GenericArgumentCheckLimits, GenericCallBindingError,
    GenericCallBindingLimits, GenericCallValueArgument, GenericPrimitive, GenericSignatureId,
    GenericSignatureInstantiationError, GenericSignatureInstantiationLimits,
    GenericSignatureReturn, GenericTypeArgumentLimits, GenericTypeDeclarationId,
    GenericTypeEquivalenceError, GenericTypeEquivalenceLimits, GenericTypeError, GenericTypeLimits,
    InstantiatedGenericSignature, ResolvedGenericTypeKind,
};
use crate::compiler::{
    ast::{Expression, ExpressionKind, Statement},
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, Token, TokenKind},
    parser::Parser,
    resources::FrontendLimits,
};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
static NEXT_BODY_ARENA: AtomicU64 = AtomicU64::new(1);
fn allocate_body_arena(counter: &AtomicU64) -> Result<u64, GenericCopyBodyError> {
    loop {
        let current = counter.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .ok_or(GenericCopyBodyError::Limit("body arena identities"))?;
        if counter
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(current);
        }
    }
}
/// Independent per-parse/expansion profiles and cumulative local type admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericLocalAnnotationLimits {
    /// Explicit local annotation records.
    pub max_annotations: usize,
    /// Copied original local type fragments.
    pub max_source_bytes: usize,
    /// Relexed type tokens, including each fragment EOF.
    pub max_tokens: usize,
    /// Retained parsed type nodes.
    pub max_nodes: usize,
    /// Retained parsed type edges.
    pub max_edges: usize,
    /// Retained copied type-name bytes.
    pub max_name_bytes: usize,
    /// Generated expanded type nodes.
    pub max_expanded_nodes: usize,
    /// Generated expanded type edges.
    pub max_expanded_edges: usize,
    /// Independent type fragment syntax profile.
    pub syntax: GenericTypeLimits,
    /// Independent annotation expansion profile, also capped by body work.
    pub expansion: GenericAliasExpansionLimits,
}
impl GenericLocalAnnotationLimits {
    /// Finite cumulative annotation profile across the whole body table.
    pub const DEFAULT: Self = Self {
        max_annotations: 4096,
        max_source_bytes: 1024 * 1024,
        max_tokens: 65_536,
        max_nodes: 65_536,
        max_edges: 65_536,
        max_name_bytes: 1024 * 1024,
        max_expanded_nodes: 65_536,
        max_expanded_edges: 65_536,
        syntax: GenericTypeLimits::DEFAULT,
        expansion: GenericAliasExpansionLimits::DEFAULT,
    };
    fn remaining(self) -> GenericLocalAnnotationUsage {
        GenericLocalAnnotationUsage {
            annotations: self.max_annotations,
            source_bytes: self.max_source_bytes,
            tokens: self.max_tokens,
            nodes: self.max_nodes,
            edges: self.max_edges,
            name_bytes: self.max_name_bytes,
            expanded_nodes: self.max_expanded_nodes,
            expanded_edges: self.max_expanded_edges,
        }
    }
}
/// Cumulative retained/checked local annotation measurements, not a type proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericLocalAnnotationUsage {
    /// Explicit annotations.
    pub annotations: usize,
    /// Retained original fragment bytes.
    pub source_bytes: usize,
    /// Parsed fragment tokens, including EOFs.
    pub tokens: usize,
    /// Parsed type nodes.
    pub nodes: usize,
    /// Parsed type edges.
    pub edges: usize,
    /// Copied type-name bytes.
    pub name_bytes: usize,
    /// Expanded type nodes.
    pub expanded_nodes: usize,
    /// Expanded type edges.
    pub expanded_edges: usize,
}
/// Source-origin failure within a local annotation fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericLocalAnnotationError {
    /// Bounded type grammar/fragment lexer refusal.
    Syntax(GenericTypeError),
    /// Bounded name/arity/alias/bound expansion refusal.
    Expansion(GenericAliasExpansionError),
}
/// A retained complete explicit local type and original full-source type range.
#[derive(Debug, Clone)]
pub struct GenericCopyLocalAnnotation {
    /// Exact original function-source fragment range.
    pub range: Span,
    /// Resolved, alias-expanded symbolic type under the function parameter owner.
    pub ty: ExpandedGenericType,
}
/// Independent output/work/depth limits for resolved field/payload contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericFieldSubstitutionLimits {
    /// Generated nodes, including the retained receiver arena.
    pub max_nodes: usize,
    /// Generated child edges.
    pub max_edges: usize,
    /// Mapping, copy and child-processing work.
    pub max_work: usize,
    /// Maximum generated structural depth, at most 128.
    pub max_depth: usize,
}
impl GenericFieldSubstitutionLimits {
    /// Finite per-field profile, also reduced by cumulative body limits.
    pub const DEFAULT: Self = Self {
        max_nodes: 16_384,
        max_edges: 32_768,
        max_work: 262_144,
        max_depth: 128,
    };
    fn expansion(self) -> GenericAliasExpansionLimits {
        GenericAliasExpansionLimits {
            max_nodes: self.max_nodes,
            max_edges: self.max_edges,
            max_work: self.max_work,
            max_depth: self.max_depth,
            ..GenericAliasExpansionLimits::DEFAULT
        }
    }
}
/// Shared explicit call/record input admission plus independent pipeline profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericBodyCallLimits {
    /// Total explicit generic and ordinary nongeneric call facts across every body.
    pub max_calls: usize,
    /// Total call argument, record initializer and enum payload edges.
    pub max_values: usize,
    /// Total generated call argument/signature and nominal constructor type nodes.
    pub max_nodes: usize,
    /// Total generated argument/signature child edges.
    pub max_edges: usize,
    /// Per-list syntax caps; also reduced by cumulative input argument ledgers.
    pub syntax: GenericTypeArgumentLimits,
    /// Cumulative source/token/input/name/type-argument admission; generated and
    /// work caps are independent per call and also reduced by shared body budgets.
    pub arguments: GenericArgumentCheckLimits,
    /// Independent per-call callee matching profile, capped by remaining body work.
    pub binding: GenericCallBindingLimits,
    /// Independent per-call signature profile, capped by shared node/edge/work.
    pub signature: GenericSignatureInstantiationLimits,
}
impl GenericBodyCallLimits {
    /// Finite symbolic source-call profile; no generic execution.
    pub const DEFAULT: Self = Self {
        max_calls: 4096,
        max_values: 16_384,
        max_nodes: 65_536,
        max_edges: 131_072,
        syntax: GenericTypeArgumentLimits::DEFAULT,
        arguments: GenericArgumentCheckLimits::DEFAULT,
        binding: GenericCallBindingLimits::DEFAULT,
        signature: GenericSignatureInstantiationLimits::DEFAULT,
    };
}
/// Retained/generated source-call usage across the complete body table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericBodyCallUsage {
    /// Source call facts.
    pub calls: usize,
    /// Source call argument, record initializer and enum payload edges.
    pub values: usize,
    /// Copied explicit type arguments.
    pub type_arguments: usize,
    /// Explicit call-list and complete constructor-head source bytes.
    pub source_bytes: usize,
    /// Explicit-list tokens plus constructor prefix tokens, including each fragment EOF.
    pub tokens: usize,
    /// Copied type syntax nodes, including nominal constructor heads.
    pub input_nodes: usize,
    /// Copied type syntax edges including list roots.
    pub input_edges: usize,
    /// Copied qualified type-name bytes, including constructor heads.
    pub name_bytes: usize,
    /// Generated expanded call and constructor type nodes.
    pub nodes: usize,
    /// Generated expanded argument/signature edges.
    pub edges: usize,
}
impl GenericBodyCallLimits {
    fn remaining(self) -> GenericBodyCallUsage {
        GenericBodyCallUsage {
            calls: self.max_calls,
            values: self.max_values,
            type_arguments: self.arguments.max_arguments,
            source_bytes: self.arguments.max_source_bytes,
            tokens: self.arguments.max_tokens,
            input_nodes: self.arguments.max_input_nodes,
            input_edges: self.arguments.max_input_edges,
            name_bytes: self.arguments.max_name_bytes,
            nodes: self.max_nodes,
            edges: self.max_edges,
        }
    }
}
/// Original source-call stage refusal; nested type-list diagnostics retain their
/// fragment/alias sources and are not relabeled as full-function coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericBodyCallError {
    /// Explicit-list syntax failure.
    Syntax(GenericTypeError),
    /// Caller-scope type arguments failed.
    Arguments(GenericArgumentCheckError),
    /// Callee parameter/bound matching failed.
    Binding(GenericCallBindingError),
    /// Bounded signature substitution failed.
    Signature(GenericSignatureInstantiationError),
}
/// Cumulative admission across every supplied generic body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCopyBodyLimits {
    /// Functions, all of which must pass this body profile.
    pub max_functions: usize,
    /// Scanned complete original function source bytes.
    pub max_source_bytes: usize,
    /// Scanned original tokens including each EOF.
    pub max_tokens: usize,
    /// Postparse statement/expression AST admission before semantic fact retention.
    pub max_ast_nodes: usize,
    /// Retained flat statement facts.
    pub max_statements: usize,
    /// Retained postorder expression facts.
    pub max_expressions: usize,
    /// Retained parameter/local binding records.
    pub max_bindings: usize,
    /// Copied binding-name bytes; scope/fact labels share allocations.
    pub max_binding_text_bytes: usize,
    /// Cumulative type-comparison pairs, including primitive comparisons.
    pub max_type_pairs: usize,
    /// Cumulative queued comparison edges.
    pub max_type_edges: usize,
    /// Semantic visits, binding lookups and type-comparison operations.
    pub max_work: usize,
    /// Cumulative generated field-substitution nodes and provenance slots.
    pub max_field_nodes: usize,
    /// Cumulative generated field-substitution edges.
    pub max_field_edges: usize,
    /// Independent per-field caps, reduced by cumulative admission.
    pub field_expansion: GenericFieldSubstitutionLimits,
    /// Existing bounded lexer/parser diagnostics; zero retains their minimum one.
    pub max_diagnostics: usize,
    /// Independent per-comparison caps, reduced by cumulative remaining allowances.
    pub comparison: GenericTypeEquivalenceLimits,
    /// Cumulative explicit enum constructor facts.
    pub max_enum_constructions: usize,
    /// Cumulative explicit record constructor facts.
    pub max_record_constructions: usize,
    /// Cumulative source calls and independent call pipeline profiles.
    pub calls: GenericBodyCallLimits,
    /// Cumulative explicit local type admission.
    pub local_types: GenericLocalAnnotationLimits,
}
impl GenericCopyBodyLimits {
    /// Package-wide type-argument admission; per-list syntax stays at its existing cap.
    pub const PACKAGE: Self = Self {
        calls: GenericBodyCallLimits {
            arguments: GenericArgumentCheckLimits {
                max_arguments: 4096,
                ..GenericArgumentCheckLimits::DEFAULT
            },
            ..GenericBodyCallLimits::DEFAULT
        },
        ..Self::DEFAULT
    };
    /// Finite opt-in copy/return/equality profile; no executable lowering is enabled.
    pub const DEFAULT: Self = Self {
        max_functions: 128,
        max_source_bytes: 4 * 1024 * 1024,
        max_tokens: 262_144,
        max_ast_nodes: 65_536,
        max_statements: 16_384,
        max_expressions: 65_536,
        max_bindings: 16_384,
        max_binding_text_bytes: 64 * 1024,
        max_type_pairs: 65_536,
        max_type_edges: 131_072,
        max_work: 1_000_000,
        max_field_nodes: 65_536,
        max_field_edges: 131_072,
        field_expansion: GenericFieldSubstitutionLimits::DEFAULT,
        max_diagnostics: 64,
        comparison: GenericTypeEquivalenceLimits::DEFAULT,
        max_enum_constructions: 1024,
        max_record_constructions: 1024,
        calls: GenericBodyCallLimits::DEFAULT,
        local_types: GenericLocalAnnotationLimits::DEFAULT,
    };
}
/// Complete body-profile refusal; no unchecked statements or partial proof escape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericCopyBodyError {
    /// Opt-in postfix compatibility refusal with original question provenance.
    Propagation {
        /// Owning function identity.
        function: GenericSignatureId,
        /// Original question token.
        span: Span,
        /// Exact bounded symbolic compatibility refusal.
        error: Box<super::GenericPropagationError>,
    },
    /// Selected cumulative admission exhausted.
    Limit(&'static str),
    /// Existing core block parser or lexer rejected original source tokens.
    Parse {
        /// Owning checked signature identity.
        function: GenericSignatureId,
        /// Original full-function-source diagnostic coordinates.
        diagnostics: Diagnostics,
    },
    /// Unsupported syntax, invalid name/type/literal or missing return coverage.
    Semantic {
        /// Owning checked signature identity.
        function: GenericSignatureId,
        /// Original full-source expression/statement/closing-body range.
        span: Span,
        /// Fixed bounded explanation.
        reason: &'static str,
    },
    /// Local type fragment failure, preserving input and alias provenance.
    Annotation {
        /// Owning checked signature identity.
        function: GenericSignatureId,
        /// Full original function-source annotation range.
        annotation_range: Span,
        /// Fragment-local grammar/expansion error; alias origins stay separate.
        error: Box<GenericLocalAnnotationError>,
    },
    /// Bounded field substitution failed without a partial type proof.
    FieldType {
        /// Owning signature.
        function: GenericSignatureId,
        /// Original field expression range.
        span: Span,
        /// Exact substitution refusal.
        error: Box<GenericAliasExpansionError>,
    },
    /// Explicit call/constructor list failure with an original function range and distinct
    /// nested explicit-list/alias diagnostic provenance.
    Call {
        /// Original caller identity.
        function: GenericSignatureId,
        /// Original full-function explicit type list, or empty range at an ordinary callee end.
        type_range: Span,
        /// Exact stage failure.
        error: Box<GenericBodyCallError>,
    },
    /// Constructor nominal type failed with input ranges in the original
    /// function source; alias errors retain their separate alias source.
    RecordType {
        /// Original caller.
        function: GenericSignatureId,
        /// Original full constructor type-head range.
        type_range: Span,
        /// Exact nominal type expansion refusal.
        error: Box<GenericAliasExpansionError>,
    },
    /// Enum nominal type failed with input ranges in the original function;
    /// alias origins retain their independent source.
    EnumType {
        /// Original caller.
        function: GenericSignatureId,
        /// Original full enum constructor type-head range.
        type_range: Span,
        /// Exact nominal expansion refusal.
        error: Box<GenericAliasExpansionError>,
    },
    /// Exhaustive nominal pattern or payload context refusal.
    Pattern {
        /// Owning caller.
        function: GenericSignatureId,
        /// Exact coverage/substitution refusal.
        error: Box<super::GenericEnumPatternError>,
    },
    /// Type comparison exhausted, never converted into an equality decision.
    Comparison {
        /// Owning signature identity.
        function: GenericSignatureId,
        /// Existing bounded comparison refusal.
        error: GenericTypeEquivalenceError,
    },
}
impl std::fmt::Display for GenericCopyBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic copy-body checking refused: {self:?}")
    }
}
impl std::error::Error for GenericCopyBodyError {}
#[derive(Debug, Clone)]
enum TypeKind {
    Primitive(GenericPrimitive),
    Expanded(ExpandedGenericType),
}
/// Source-owned symbolic value type, without introducing a runtime reference/value.
#[derive(Debug, Clone)]
pub struct GenericCopyType(TypeKind);
impl GenericCopyType {
    pub(super) fn concrete_primitive(value: GenericPrimitive) -> Self {
        Self(TypeKind::Primitive(value))
    }
    pub(super) fn concrete_expanded(value: ExpandedGenericType) -> Self {
        Self(TypeKind::Expanded(value))
    }

    /// Existing concrete primitive, if this value type is primitive.
    pub fn primitive(&self) -> Option<GenericPrimitive> {
        match &self.0 {
            TypeKind::Primitive(p) => Some(*p),
            TypeKind::Expanded(ty) => match ty.nodes()[ty.root()].kind {
                ResolvedGenericTypeKind::Primitive(p) => Some(p),
                _ => None,
            },
        }
    }
    /// Existing expanded annotation owner, if the type came from a signature.
    pub fn expanded(&self) -> Option<&ExpandedGenericType> {
        match &self.0 {
            TypeKind::Expanded(ty) => Some(ty),
            _ => None,
        }
    }
}
pub(super) fn collection_type_kind(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> Option<super::GenericCollectionLayoutKind> {
    let expanded = ty.expanded()?;
    let ResolvedGenericTypeKind::Declared { declaration, .. } =
        expanded.nodes()[expanded.root()].kind
    else {
        return None;
    };
    match types.definition(declaration)? {
        super::CheckedGenericDefinition::Collection(c) => Some(c.syntax().kind()),
        _ => None,
    }
}
pub(super) fn standard_map_entry_arguments(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> Option<(GenericCopyType, GenericCopyType)> {
    let e = ty.expanded()?;
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &e.nodes()[e.root()].kind
    else {
        return None;
    };
    let CheckedGenericDefinition::Record(def) = types.definition(*declaration)? else {
        return None;
    };
    if !def.syntax().is_standard_map_entry() || arguments.len() != 2 {
        return None;
    }
    Some((
        GenericCopyType::concrete_expanded(e.declaration_argument(*declaration, 0)?),
        GenericCopyType::concrete_expanded(e.declaration_argument(*declaration, 1)?),
    ))
}
pub(super) fn is_byte_type(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    ty.expanded().is_some_and(|e|matches!(e.nodes()[e.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..} if super::generic_data_types::is_byte_header(types.environment().registry(),declaration)))
}
pub(super) fn is_bytes_type(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    ty.expanded().is_some_and(|e|matches!(e.nodes()[e.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..} if matches!(types.definition(declaration),Some(CheckedGenericDefinition::Data(d)) if d.syntax().kind()==super::GenericDataTypeKind::Bytes)))
}
pub(super) fn is_byte_array(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    ty.expanded().is_some_and(|e|matches!(e.nodes()[e.root()].kind,ResolvedGenericTypeKind::Array{element} if matches!(e.nodes()[element].kind,ResolvedGenericTypeKind::Declared{declaration,..} if super::generic_data_types::is_byte_header(types.environment().registry(),declaration))))
}
pub(super) fn is_json_type(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    ty.expanded().is_some_and(|e|matches!(e.nodes()[e.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..} if matches!(types.definition(declaration),Some(CheckedGenericDefinition::Enum(d)) if d.syntax().standard_identity()==Some(super::GenericStandardEnum::JsonValue))))
}
pub(super) fn is_json_result(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    is_data_result(ty, types, DataResultKind::JsonValue)
}
pub(super) fn is_byte_result(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    is_data_result(ty, types, DataResultKind::Byte)
}
pub(super) fn is_bytes_result(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    is_data_result(ty, types, DataResultKind::Bytes)
}
pub(super) fn is_byte_array_result(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    is_data_result(ty, types, DataResultKind::ByteArray)
}
pub(super) fn is_string_result(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> bool {
    is_data_result(ty, types, DataResultKind::String)
}
fn is_data_result(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
    success: DataResultKind,
) -> bool {
    let Some(e) = ty.expanded() else {
        return false;
    };
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &e.nodes()[e.root()].kind
    else {
        return false;
    };
    let Some(CheckedGenericDefinition::Enum(def)) = types.definition(*declaration) else {
        return false;
    };
    if def.syntax().standard_identity() != Some(super::GenericStandardEnum::Result)
        || arguments.len() != 2
    {
        return false;
    }
    let Some(byte) = e.declaration_argument(*declaration, 0) else {
        return false;
    };
    let Some(error) = e.declaration_argument(*declaration, 1) else {
        return false;
    };
    (match success {
        DataResultKind::Byte => is_byte_type(&GenericCopyType::concrete_expanded(byte), types),
        DataResultKind::Bytes => is_bytes_type(&GenericCopyType::concrete_expanded(byte), types),
        DataResultKind::ByteArray => {
            is_byte_array(&GenericCopyType::concrete_expanded(byte), types)
        }
        DataResultKind::JsonValue => is_json_type(&GenericCopyType::concrete_expanded(byte), types),
        DataResultKind::String => matches!(
            byte.nodes()[byte.root()].kind,
            ResolvedGenericTypeKind::Primitive(GenericPrimitive::String)
        ),
    }) && matches!(error.nodes()[error.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..} if matches!(types.definition(declaration),Some(CheckedGenericDefinition::Record(r)) if r.syntax().is_standard_error()))
}
pub(super) fn standard_option_argument(
    ty: &GenericCopyType,
    types: &super::CheckedGenericDeclarations,
) -> Option<GenericCopyType> {
    let expanded = ty.expanded()?;
    let ResolvedGenericTypeKind::Declared {
        declaration,
        arguments,
    } = &expanded.nodes()[expanded.root()].kind
    else {
        return None;
    };
    let CheckedGenericDefinition::Enum(def) = types.definition(*declaration)? else {
        return None;
    };
    if def.syntax().standard_identity() != Some(super::GenericStandardEnum::Option)
        || arguments.len() != 1
    {
        return None;
    }
    expanded
        .declaration_argument(*declaration, 0)
        .map(GenericCopyType::concrete_expanded)
}
pub(super) fn collection_argument(ty: &GenericCopyType, index: usize) -> Option<GenericCopyType> {
    let e = ty.expanded()?;
    let ResolvedGenericTypeKind::Declared { declaration, .. } = e.nodes()[e.root()].kind else {
        return None;
    };
    e.declaration_argument(declaration, index)
        .map(GenericCopyType::concrete_expanded)
}
/// Binding identity scoped to one checked signature and one immutable body arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericCopyBindingId {
    function: GenericSignatureId,
    arena: u64,
    ordinal: usize,
}
impl GenericCopyBindingId {
    /// Source-order parameter/local ordinal within this arena only.
    pub fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Expression identity scoped to one checked signature and one immutable body arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericCopyExpressionId {
    function: GenericSignatureId,
    arena: u64,
    ordinal: usize,
}
impl GenericCopyExpressionId {
    /// Postorder expression ordinal within this arena only.
    pub fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Binding origin and local mutability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericCopyBindingKind {
    /// Existing explicit value parameter index.
    Parameter(usize),
    /// Inferred immutable local declaration.
    Local,
    /// Immutable snapshot of one selected enum payload.
    MatchPayload {
        /// Single checked scrutinee read in this body arena.
        scrutinee: GenericCopyExpressionId,
        /// Original enum variant index.
        variant_index: usize,
    },
    /// Mutable local with a fixed symbolic type.
    MutableLocal,
}
/// One fixed parameter/local type and original name range.
#[derive(Debug, Clone)]
pub struct GenericCopyBinding {
    /// Source-owned binding identity.
    pub id: GenericCopyBindingId,
    /// Shared copied name label.
    pub name: Arc<str>,
    /// Original short-name range.
    pub name_span: Span,
    /// Parameter or local origin.
    pub kind: GenericCopyBindingKind,
    /// Complete symbolic value type.
    pub ty: GenericCopyType,
}
/// An original value-name token bound to a checked body-owned declaration.
#[derive(Debug, Clone, Copy)]
pub struct GenericCopyNameUse {
    /// Exact declaration identity, including function and arena.
    pub binding: GenericCopyBindingId,
    /// Original short token, excluding grouping/indexing.
    pub span: Span,
}
/// Existing concrete numeric context conversion; never converts an opaque parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericCopyConversion {
    /// Exact type; no conversion.
    Identity,
    /// Existing Int-to-Float widening on a concrete scalar edge.
    IntToFloat,
}
/// Boolean short-circuit operation; this records typing, not executable control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericLogicalOperator {
    /// Evaluate the right operand at runtime only when the left is true.
    And,
    /// Evaluate the right operand at runtime only when the left is false.
    Or,
}
/// Concrete String operation, with existing lexical ordering semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericStringOperator {
    /// Concatenation; future execution must admit output bytes before allocation.
    Concatenate,
    /// Less-than.
    Less,
    /// Less-than-or-equal.
    LessEqual,
    /// Greater-than.
    Greater,
    /// Greater-than-or-equal.
    GreaterEqual,
}
/// Existing concrete numeric arithmetic/ordering operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericNumericOperator {
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division; runtime zero/overflow checks remain required.
    Divide,
    /// Numeric less-than.
    Less,
    /// Numeric less-than-or-equal.
    LessEqual,
    /// Numeric greater-than.
    Greater,
    /// Numeric greater-than-or-equal.
    GreaterEqual,
}
impl GenericNumericOperator {
    fn from_spelling(operator: &str) -> Option<Self> {
        Some(match operator {
            "+" => Self::Add,
            "-" => Self::Subtract,
            "*" => Self::Multiply,
            "/" => Self::Divide,
            "<" => Self::Less,
            "<=" => Self::LessEqual,
            ">" => Self::Greater,
            ">=" => Self::GreaterEqual,
            _ => return None,
        })
    }
    fn ordering(self) -> bool {
        matches!(
            self,
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual
        )
    }
}
/// One context-checked array item and its explicit scalar conversion.
#[derive(Debug, Clone)]
pub struct GenericCopyArrayElement {
    /// Earlier checked item expression.
    pub value: GenericCopyExpressionId,
    /// Conversion from the item's type to the declared element type.
    pub conversion: GenericCopyConversion,
}
/// One source-order record initializer matched to a nominal field.
#[derive(Debug, Clone)]
pub struct GenericRecordFieldValue {
    /// Source-order field index in the checked record declaration.
    pub field_index: usize,
    /// Original arena-owned initializer expression.
    pub value: GenericCopyExpressionId,
    /// Initializer-to-field scalar conversion.
    pub conversion: GenericCopyConversion,
}
/// One source enum payload value; distinct from a nullary variant.
#[derive(Debug, Clone)]
pub struct GenericEnumPayloadValue {
    /// Original arena-owned payload expression.
    pub value: GenericCopyExpressionId,
    /// Exact identity or existing scalar widening.
    pub conversion: GenericCopyConversion,
}
/// Source role for a checked expression, not executable IR or literal payload encoding.
#[derive(Debug, Clone)]
pub enum GenericCopyExpressionKind {
    /// Opt-in symbolic extraction/early-return contract, not executable lowering.
    Propagate {
        /// Earlier body-owned wrapped operand.
        operand: GenericCopyExpressionId,
        /// Original question-mark token.
        question: Span,
        /// Exact symbolic wrapper/payload/return compatibility.
        types: Box<super::GenericPropagationTypes>,
    },
    /// Host-free standard operation with an immutable checked result schema.
    Builtin {
        /// Exact compiler-owned operation.
        builtin: super::GenericBuiltin,
        /// Source-order arguments checked against the operation contract.
        arguments: Vec<GenericCopyExpressionId>,
    },
    /// Source literal of the indicated existing primitive type.
    Literal(GenericPrimitive),
    /// Exact source-scope binding selected before a later shadowing declaration.
    Read(GenericCopyBindingId),
    /// Original explicit root call with checked caller/callee/signature/value types.
    /// This is symbolic evidence, not executable IR or a specialized callee body.
    Call {
        /// Retained sealed type argument/callee/substituted signature evidence.
        signature: Box<InstantiatedGenericSignature>,
        /// Original callee identifier/path range, excluding grouping and type arguments.
        callee_span: Span,
        /// Original full-function explicit type-list range.
        type_range: Span,
        /// Source-order expression IDs and explicit parameter conversions.
        arguments: Vec<GenericCallValueArgument>,
    },
    /// Explicit nullary/single-payload nominal enum construction; no runtime tag plan.
    Enum {
        /// Exact checked enum declaration.
        declaration: GenericTypeDeclarationId,
        /// Source-order variant index in the original enum.
        variant_index: usize,
        /// Original full enum type-head range.
        type_range: Span,
        /// Original short variant-name range.
        variant_range: Span,
        /// None is nullary; Some(Unit-valued expression) remains distinct.
        payload: Option<GenericEnumPayloadValue>,
    },
    /// Explicit checked nominal record construction, not a runtime layout plan.
    Record {
        /// Exact checked nominal declaration.
        declaration: GenericTypeDeclarationId,
        /// Original full constructor type-head range.
        type_range: Span,
        /// Field initializers in source order, each with its declaration index.
        fields: Vec<GenericRecordFieldValue>,
    },
    /// Read a checked record field with bounded composite substitution if needed.
    Field {
        /// Earlier checked record receiver.
        receiver: GenericCopyExpressionId,
        /// Owning nominal record declaration.
        declaration: GenericTypeDeclarationId,
        /// Source-order field index in the checked record definition.
        field_index: usize,
        /// Receiver argument ordinal for a direct parameter field; otherwise None.
        parameter_argument: Option<usize>,
    },
    /// Context-typed array literal; items remain in original source order.
    Array(Vec<GenericCopyArrayElement>),
    /// Read one array element; future execution must check index bounds.
    Index {
        /// Earlier array expression.
        target: GenericCopyExpressionId,
        /// Earlier concrete Int index expression.
        index: GenericCopyExpressionId,
    },
    /// Boolean operation. Both operands are checked; runtime lowering must short-circuit.
    Logical {
        /// Boolean operator.
        operator: GenericLogicalOperator,
        /// Earlier left operand.
        left: GenericCopyExpressionId,
        /// Earlier right operand.
        right: GenericCopyExpressionId,
    },
    /// Concrete String operation with no implicit formatting or coercion.
    String {
        /// String operator.
        operator: GenericStringOperator,
        /// Earlier left operand.
        left: GenericCopyExpressionId,
        /// Earlier right operand.
        right: GenericCopyExpressionId,
    },
    /// Concrete numeric operation with checked operand types and explicit widening.
    Numeric {
        /// Source numeric operation.
        operator: GenericNumericOperator,
        /// Earlier left operand.
        left: GenericCopyExpressionId,
        /// Earlier right operand.
        right: GenericCopyExpressionId,
        /// Left numeric-context conversion.
        left_conversion: GenericCopyConversion,
        /// Right numeric-context conversion.
        right_conversion: GenericCopyConversion,
    },
    /// Structural equality/inequality, with explicit existing numeric conversions.
    Equality {
        /// Earlier left expression.
        left: GenericCopyExpressionId,
        /// Earlier right expression.
        right: GenericCopyExpressionId,
        /// True is !=; false is ==.
        negated: bool,
        /// Left comparison-context conversion.
        left_conversion: GenericCopyConversion,
        /// Right comparison-context conversion.
        right_conversion: GenericCopyConversion,
    },
}
/// Immutable checked expression role, type and original range.
#[derive(Debug, Clone)]
pub struct GenericCopyExpression {
    /// Body-owned identity.
    pub id: GenericCopyExpressionId,
    /// Original full-source expression range, including grouping parentheses.
    pub span: Span,
    /// Source role and retained operand/binding identities.
    pub kind: GenericCopyExpressionKind,
    /// Complete symbolic result type.
    pub ty: GenericCopyType,
}
/// Flat checked statement role; source remains owned by the retained signature table.
#[derive(Debug, Clone)]
pub enum GenericCopyStatement {
    /// Exhaustive enum matching; execution/lowering is still unavailable.
    Match {
        /// Single original-source binding read.
        scrutinee: GenericCopyExpressionId,
        /// Exact checked nominal enum.
        declaration: GenericTypeDeclarationId,
        /// Source-order independently checked arms.
        arms: Vec<GenericCopyMatchArm>,
        /// Original whole statement range.
        span: Span,
    },
    /// A local with fixed symbolic type; binding origin records mutability.
    Let {
        /// Newly introduced local.
        binding: GenericCopyBindingId,
        /// Earlier checked initializer expression.
        value: GenericCopyExpressionId,
        /// Explicit annotation proof, if written.
        annotation: Option<GenericCopyLocalAnnotation>,
        /// Initializer-context scalar conversion.
        conversion: GenericCopyConversion,
        /// Original full-source statement range.
        span: Span,
    },
    /// Direct replacement of a mutable local; no field/index mutation implied.
    Assign {
        /// Existing mutable binding.
        binding: GenericCopyBindingId,
        /// Original target name range.
        target_span: Span,
        /// Earlier checked replacement expression.
        value: GenericCopyExpressionId,
        /// Replacement-context scalar conversion.
        conversion: GenericCopyConversion,
        /// Original full-source statement range.
        span: Span,
    },
    /// Replace one element of a direct mutable array local, not a reference write.
    IndexAssign {
        /// Existing mutable array binding.
        binding: GenericCopyBindingId,
        /// Checked concrete Int index.
        index: GenericCopyExpressionId,
        /// Checked replacement value.
        value: GenericCopyExpressionId,
        /// Replacement-context scalar conversion.
        conversion: GenericCopyConversion,
        /// Original full indexed target range.
        target_span: Span,
        /// Original full-source statement range.
        span: Span,
    },
    /// Conditional with flat source-order branch ranges, not executable jumps.
    If {
        /// Checked concrete Bool condition.
        condition: GenericCopyExpressionId,
        /// Then statements, including nested branch facts.
        then_statements: std::ops::Range<usize>,
        /// Else statements when present; an empty branch remains distinguishable.
        else_statements: Option<std::ops::Range<usize>>,
        /// Original conditional range.
        span: Span,
    },
    /// Loop typing facts, not executable backedges or a termination proof.
    While {
        /// Concrete Bool condition, re-evaluated by future execution per iteration.
        condition: GenericCopyExpressionId,
        /// Flat loop-body range, including nested facts.
        body_statements: std::ops::Range<usize>,
        /// Original full-source loop range.
        span: Span,
    },
    /// A supported expression whose result is discarded, without implicit conversion.
    Expression {
        /// Checked expression, including its original source range.
        value: GenericCopyExpressionId,
    },
    /// An explicit return, including a void return.
    Return {
        /// Earlier checked returned expression, if present.
        value: Option<GenericCopyExpressionId>,
        /// Existing return-context conversion.
        conversion: GenericCopyConversion,
        /// Original full-source statement range.
        span: Span,
    },
}
/// One checked match arm with an isolated lexical scope.
#[derive(Debug, Clone)]
pub struct GenericCopyMatchArm {
    /// Original nominal variant index.
    pub variant_index: usize,
    /// Immutable arm-local binding, when the variant has a payload.
    pub payload_binding: Option<GenericCopyBindingId>,
    /// Flat body statement range including nested facts.
    pub statements: std::ops::Range<usize>,
    /// Original arm source range.
    pub span: Span,
}
/// Sealed proof for this restricted symbolic body profile, not executable generic IR.
#[derive(Debug, Clone)]
pub struct CheckedGenericCopyBody {
    signatures: CheckedGenericSignatures,
    function: GenericSignatureId,
    arena: u64,
    bindings: Arc<[GenericCopyBinding]>,
    name_uses: Arc<[GenericCopyNameUse]>,
    expressions: Arc<[GenericCopyExpression]>,
    statements: Arc<[GenericCopyStatement]>,
    implicit_unit_return: bool,
}
impl CheckedGenericCopyBody {
    /// Retained original checked signature; body checking is additional to it.
    pub fn signature(&self) -> &CheckedGenericSignature {
        self.signatures
            .signature(self.function)
            .expect("owned signature")
    }
    /// All source-order parameter/local declarations.
    pub fn bindings(&self) -> &[GenericCopyBinding] {
        &self.bindings
    }
    /// Checked original read and write name tokens with resolved binding identities.
    pub fn name_uses(&self) -> &[GenericCopyNameUse] {
        &self.name_uses
    }
    /// All checked postorder expression facts.
    pub fn expressions(&self) -> &[GenericCopyExpression] {
        &self.expressions
    }
    /// All source-order statement facts, including checked unreachable statements.
    pub fn statements(&self) -> &[GenericCopyStatement] {
        &self.statements
    }
    /// True only when a Unit body falls through without an explicit return.
    pub fn implicit_unit_return(&self) -> bool {
        self.implicit_unit_return
    }
    /// Read a binding only when its function and arena identities match.
    pub fn binding(&self, id: GenericCopyBindingId) -> Option<&GenericCopyBinding> {
        if id.function != self.function || id.arena != self.arena {
            return None;
        }
        self.bindings.get(id.ordinal)
    }
    /// Read an expression only when its function and arena identities match.
    pub fn expression(&self, id: GenericCopyExpressionId) -> Option<&GenericCopyExpression> {
        if id.function != self.function || id.arena != self.arena {
            return None;
        }
        self.expressions.get(id.ordinal)
    }
}
/// Complete restricted-profile body proofs for every supplied checked signature.
#[derive(Debug, Clone)]
pub struct CheckedGenericCopyBodies {
    propagation: Option<super::GenericPropagationUsage>,
    signatures: CheckedGenericSignatures,
    bodies: Arc<[CheckedGenericCopyBody]>,
    source_bytes: usize,
    tokens: usize,
    ast_nodes: usize,
    statements: usize,
    expressions: usize,
    bindings: usize,
    binding_text: usize,
    type_pairs: usize,
    type_edges: usize,
    work: usize,
    local_annotations: GenericLocalAnnotationUsage,
    records: usize,
    enums: usize,
    calls: GenericBodyCallUsage,
    field_nodes: usize,
    field_edges: usize,
}
impl CheckedGenericCopyBodies {
    /// Opt-in postfix admission; None for the ordinary strict checker.
    pub fn propagation_usage(&self) -> Option<super::GenericPropagationUsage> {
        self.propagation
    }

    /// Retained explicit enum constructor facts.
    pub fn enum_constructions(&self) -> usize {
        self.enums
    }

    /// Retained explicit record constructor facts.
    pub fn record_constructions(&self) -> usize {
        self.records
    }

    /// Cumulative original call/input/generated metadata usage.
    pub fn call_usage(&self) -> GenericBodyCallUsage {
        self.calls
    }

    /// Generated field nodes, each with one source-provenance slot.
    pub fn field_nodes(&self) -> usize {
        self.field_nodes
    }
    /// Generated field edges.
    pub fn field_edges(&self) -> usize {
        self.field_edges
    }

    /// Cumulative explicit local type metadata and expansion usage.
    pub fn local_annotation_usage(&self) -> GenericLocalAnnotationUsage {
        self.local_annotations
    }
    /// Retained complete signature/type context.
    pub fn signatures(&self) -> &CheckedGenericSignatures {
        &self.signatures
    }
    /// Source-order body proofs; no body is skipped for having no callers.
    pub fn bodies(&self) -> &[CheckedGenericCopyBody] {
        &self.bodies
    }
    /// Validate a signature's owner before retrieving its body proof.
    pub fn body(&self, id: GenericSignatureId) -> Option<&CheckedGenericCopyBody> {
        self.signatures.signature(id)?;
        self.bodies.get(id.ordinal())
    }
    /// Cumulative scanned original source bytes.
    pub fn source_bytes(&self) -> usize {
        self.source_bytes
    }
    /// Cumulative scanned tokens including EOFs.
    pub fn tokens(&self) -> usize {
        self.tokens
    }
    /// Postparse admitted body statement/expression nodes.
    pub fn ast_nodes(&self) -> usize {
        self.ast_nodes
    }
    /// Retained statement facts.
    pub fn statement_count(&self) -> usize {
        self.statements
    }
    /// Retained expression facts.
    pub fn expression_count(&self) -> usize {
        self.expressions
    }
    /// Retained parameter/local binding records.
    pub fn binding_count(&self) -> usize {
        self.bindings
    }
    /// Copied binding-name text bytes.
    pub fn binding_text_bytes(&self) -> usize {
        self.binding_text
    }
    /// Cumulative exact-comparison node pairs.
    pub fn type_pairs(&self) -> usize {
        self.type_pairs
    }
    /// Cumulative exact-comparison child edges.
    pub fn type_edges(&self) -> usize {
        self.type_edges
    }
    /// Cumulative logical semantic/comparison steps.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(left: &mut usize, n: usize, dimension: &'static str) -> Result<(), GenericCopyBodyError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericCopyBodyError::Limit(dimension))?;
    Ok(())
}
struct Admission {
    propagation: Option<super::GenericPropagationLimits>,
    source: usize,
    tokens: usize,
    ast: usize,
    statements: usize,
    expressions: usize,
    name_uses: usize,
    bindings: usize,
    text: usize,
    pairs: usize,
    edges: usize,
    work: usize,
    locals: GenericLocalAnnotationUsage,
    records: usize,
    enums: usize,
    calls: GenericBodyCallUsage,
    field_nodes: usize,
    field_edges: usize,
}
impl Admission {
    fn step(&mut self) -> Result<(), GenericCopyBodyError> {
        charge(&mut self.work, 1, "body work")
    }
    fn same(
        &mut self,
        function: GenericSignatureId,
        a: &GenericCopyType,
        b: &GenericCopyType,
        mut limits: GenericTypeEquivalenceLimits,
    ) -> Result<bool, GenericCopyBodyError> {
        limits.max_pairs = limits.max_pairs.min(self.pairs);
        limits.max_edges = limits.max_edges.min(self.edges);
        limits.max_work = limits.max_work.min(self.work);
        if let (TypeKind::Expanded(a), TypeKind::Expanded(b)) = (&a.0, &b.0) {
            let result = compare_generic_types(a, b, limits)
                .map_err(|error| GenericCopyBodyError::Comparison { function, error })?;
            charge(&mut self.pairs, result.pairs(), "body type pairs")?;
            charge(&mut self.edges, result.edges(), "body type edges")?;
            charge(&mut self.work, result.work(), "body work")?;
            Ok(result.equivalent())
        } else {
            if limits.max_pairs == 0 || limits.max_work == 0 {
                return Err(GenericCopyBodyError::Comparison {
                    function,
                    error: GenericTypeEquivalenceError::Limit("primitive comparison admission"),
                });
            }
            charge(&mut self.pairs, 1, "body type pairs")?;
            self.step()?;
            Ok(matches!((a.primitive(),b.primitive()),(Some(a),Some(b))if a==b))
        }
    }
}
// Parsed trees stay private and are always disposed of iteratively, including on refusal.
struct AstGuard(Vec<Statement>);
impl Drop for AstGuard {
    fn drop(&mut self) {
        drop_body(std::mem::take(&mut self.0));
    }
}
pub(crate) fn drop_body(mut pending: Vec<Statement>) {
    while let Some(statement) = pending.pop() {
        match statement {
            Statement::Let { value, .. }
            | Statement::Return {
                value: Some(value), ..
            }
            | Statement::Expression(value) => value.drop_iterative(),
            Statement::Assign { target, value, .. } => {
                target.drop_iterative();
                value.drop_iterative();
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                condition.drop_iterative();
                pending.extend(then_block);
                if let Some(body) = else_block {
                    pending.extend(body);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                condition.drop_iterative();
                pending.extend(body);
            }
            Statement::Return { value: None, .. } => {}
        }
    }
}

fn admit_ast(body: &[Statement], admission: &mut Admission) -> Result<(), GenericCopyBodyError> {
    enum Work<'a> {
        Statements(std::slice::Iter<'a, Statement>),
        Statement(&'a Statement),
        Expressions(std::slice::Iter<'a, Expression>),
        Expression(&'a Expression),
        Fields(std::slice::Iter<'a, (String, Expression)>),
    }
    let mut pending = vec![Work::Statements(body.iter())];
    while let Some(work) = pending.pop() {
        match work {
            Work::Statements(mut iter) => {
                if let Some(s) = iter.next() {
                    pending.push(Work::Statements(iter));
                    pending.push(Work::Statement(s));
                }
            }
            Work::Expressions(mut iter) => {
                if let Some(e) = iter.next() {
                    pending.push(Work::Expressions(iter));
                    pending.push(Work::Expression(e));
                }
            }
            Work::Fields(mut iter) => {
                if let Some((_, e)) = iter.next() {
                    pending.push(Work::Fields(iter));
                    pending.push(Work::Expression(e));
                }
            }
            Work::Statement(s) => {
                charge(&mut admission.ast, 1, "body AST nodes")?;
                match s {
                    Statement::Let { value, .. }
                    | Statement::Return {
                        value: Some(value), ..
                    }
                    | Statement::Expression(value) => pending.push(Work::Expression(value)),
                    Statement::Assign { target, value, .. } => {
                        pending.push(Work::Expression(value));
                        pending.push(Work::Expression(target));
                    }
                    Statement::If {
                        condition,
                        then_block,
                        else_block,
                        ..
                    } => {
                        if let Some(body) = else_block {
                            pending.push(Work::Statements(body.iter()));
                        }
                        pending.push(Work::Statements(then_block.iter()));
                        pending.push(Work::Expression(condition));
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        pending.push(Work::Statements(body.iter()));
                        pending.push(Work::Expression(condition));
                    }
                    Statement::Return { value: None, .. } => {}
                }
            }
            Work::Expression(e) => {
                charge(&mut admission.ast, 1, "body AST nodes")?;
                match &e.kind {
                    ExpressionKind::Binary { left, right, .. }
                    | ExpressionKind::Index {
                        target: left,
                        index: right,
                    } => {
                        pending.push(Work::Expression(right));
                        pending.push(Work::Expression(left));
                    }
                    ExpressionKind::Call { callee, arguments } => {
                        pending.push(Work::Expressions(arguments.iter()));
                        pending.push(Work::Expression(callee));
                    }
                    ExpressionKind::ArrayLiteral(items) => {
                        pending.push(Work::Expressions(items.iter()))
                    }
                    ExpressionKind::StructLiteral { fields, .. } => {
                        pending.push(Work::Fields(fields.iter()))
                    }
                    ExpressionKind::FieldAccess { receiver, .. } => {
                        pending.push(Work::Expression(receiver))
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
struct RecordPlan {
    ty: ExpandedGenericType,
    declaration: GenericTypeDeclarationId,
    range: Span,
    fields: Vec<(usize, GenericCopyType)>,
}
struct EnumPlan {
    ty: ExpandedGenericType,
    declaration: GenericTypeDeclarationId,
    type_range: Span,
    variant_index: usize,
    variant_range: Span,
    payload: Option<GenericCopyType>,
}
struct Checker<'a> {
    builtins: u8,
    expression_depths: Option<Vec<usize>>,
    propagation_sites: BTreeMap<(usize, usize), super::GenericPropagationSyntaxSite>,
    signatures: &'a CheckedGenericSignatures,
    call_ranges: BTreeMap<usize, Span>,
    matches: BTreeMap<usize, &'a super::MatchStatementSyntax>,
    call_limits: GenericBodyCallLimits,
    function: GenericSignatureId,
    arena: u64,
    signature: &'a CheckedGenericSignature,
    tokens: &'a [Token],
    admission: &'a mut Admission,
    comparison: GenericTypeEquivalenceLimits,
    scope: BTreeMap<Arc<str>, GenericCopyBindingId>,
    bindings: Vec<GenericCopyBinding>,
    name_uses: Vec<GenericCopyNameUse>,
    expressions: Vec<GenericCopyExpression>,
    statements: Vec<GenericCopyStatement>,
    local_limits: GenericLocalAnnotationLimits,
    field_limits: GenericFieldSubstitutionLimits,
}
impl Checker<'_> {
    fn error(&self, span: Span, reason: &'static str) -> GenericCopyBodyError {
        GenericCopyBodyError::Semantic {
            function: self.function,
            span,
            reason,
        }
    }
    fn bind(
        &mut self,
        name: &str,
        name_span: Span,
        kind: GenericCopyBindingKind,
        ty: GenericCopyType,
    ) -> Result<GenericCopyBindingId, GenericCopyBodyError> {
        charge(&mut self.admission.bindings, 1, "body bindings")?;
        charge(&mut self.admission.text, name.len(), "body binding text")?;
        let id = GenericCopyBindingId {
            function: self.function,
            arena: self.arena,
            ordinal: self.bindings.len(),
        };
        let name: Arc<str> = Arc::from(name);
        self.scope.insert(Arc::clone(&name), id);
        self.bindings.push(GenericCopyBinding {
            id,
            name,
            name_span,
            kind,
            ty,
        });
        Ok(id)
    }
    fn push(
        &mut self,
        span: Span,
        kind: GenericCopyExpressionKind,
        ty: GenericCopyType,
    ) -> Result<GenericCopyExpressionId, GenericCopyBodyError> {
        charge(&mut self.admission.expressions, 1, "body expressions")?;
        if let Some(depths) = &mut self.expression_depths {
            let mut depth = 1usize;
            let mut child = |id: GenericCopyExpressionId| -> Result<(), GenericCopyBodyError> {
                charge(
                    &mut self.admission.work,
                    1,
                    "propagation expression depth work",
                )?;
                if id.function != self.function || id.arena != self.arena {
                    return Err(GenericCopyBodyError::Limit("propagation child ownership"));
                }
                depth = depth.max(
                    1 + *depths
                        .get(id.ordinal)
                        .ok_or(GenericCopyBodyError::Limit("propagation child ownership"))?,
                );
                Ok(())
            };
            match &kind {
                GenericCopyExpressionKind::Literal(_) | GenericCopyExpressionKind::Read(_) => {}
                GenericCopyExpressionKind::Propagate { operand, .. } => child(*operand)?,
                GenericCopyExpressionKind::Builtin { arguments, .. } => {
                    for value in arguments {
                        child(*value)?;
                    }
                }
                GenericCopyExpressionKind::Call { arguments, .. } => {
                    for value in arguments {
                        child(value.expression)?;
                    }
                }
                GenericCopyExpressionKind::Enum { payload, .. } => {
                    if let Some(value) = payload {
                        child(value.value)?;
                    }
                }
                GenericCopyExpressionKind::Record { fields, .. } => {
                    for value in fields {
                        child(value.value)?;
                    }
                }
                GenericCopyExpressionKind::Field { receiver, .. } => child(*receiver)?,
                GenericCopyExpressionKind::Array(values) => {
                    for value in values {
                        child(value.value)?;
                    }
                }
                GenericCopyExpressionKind::Index { target, index } => {
                    child(*target)?;
                    child(*index)?;
                }
                GenericCopyExpressionKind::Logical { left, right, .. }
                | GenericCopyExpressionKind::String { left, right, .. }
                | GenericCopyExpressionKind::Numeric { left, right, .. }
                | GenericCopyExpressionKind::Equality { left, right, .. } => {
                    child(*left)?;
                    child(*right)?;
                }
            }
            if depth > 128 {
                return Err(GenericCopyBodyError::Limit("propagation expression depth"));
            }
            depths.push(depth);
        }

        if let GenericCopyExpressionKind::Read(binding) = &kind {
            self.record_name_use(*binding, span)?;
        }
        let id = GenericCopyExpressionId {
            function: self.function,
            arena: self.arena,
            ordinal: self.expressions.len(),
        };
        self.expressions
            .push(GenericCopyExpression { id, span, kind, ty });
        Ok(id)
    }
    fn prepare_constructor_type(
        &mut self,
        name: &str,
        list: Span,
        span: Span,
        expected: GenericTypeDeclarationId,
        is_enum: bool,
    ) -> Result<(ExpandedGenericType, Span), GenericCopyBodyError> {
        let list_token = self
            .tokens
            .binary_search_by_key(&list.start, |token| token.span.start)
            .map_err(|_| self.error(list, "constructor list token ownership is unavailable"))?;
        let head = list_token
            .checked_sub(1)
            .and_then(|at| self.tokens.get(at))
            .ok_or_else(|| self.error(list, "constructor head ownership is unavailable"))?;
        let head = if self
            .signature
            .types()
            .environment()
            .registry()
            .is_package_scoped()
        {
            let mut at = list_token
                .checked_sub(1)
                .ok_or_else(|| self.error(span, "package nominal head"))?;
            for (part_index, part) in name.rsplit("::").enumerate() {
                self.admission.step()?;
                if part_index > 0 {
                    if at < 2 || self.tokens[at - 1].kind != TokenKind::Operator("::") {
                        return Err(self.error(span, "package nominal head separator"));
                    }
                    at -= 2;
                }
                if !matches!(&self.tokens[at].kind,TokenKind::Identifier(written) if written==part)
                {
                    return Err(self.error(span, "package nominal head provenance"));
                }
            }
            &self.tokens[at]
        } else if let Some((namespace, short)) = name.split_once("::") {
            if !is_enum
                || namespace != "std"
                || !matches!(short, "Option" | "Result")
                || !matches!(&head.kind,TokenKind::Identifier(written) if written==short)
                || list_token < 3
                || self.tokens[list_token - 2].kind != TokenKind::Operator("::")
                || !matches!(&self.tokens[list_token-3].kind,TokenKind::Identifier(written) if written=="std")
            {
                return Err(
                    self.error(span, "constructor requires a registered standard enum head")
                );
            }
            &self.tokens[list_token - 3]
        } else {
            if !matches!(&head.kind,TokenKind::Identifier(written) if written==name) {
                return Err(self.error(span, "constructor requires a direct root nominal head"));
            }
            head
        };
        let range = Span {
            end: list.end,
            ..head.span
        };
        let fragment = self
            .signature
            .syntax()
            .source()
            .get(list.start..list.end)
            .ok_or_else(|| self.error(list, "constructor type source ownership is unavailable"))?;
        let prefix_bytes = list.start - range.start;
        let start_token = self
            .tokens
            .binary_search_by_key(&range.start, |token| token.span.start)
            .map_err(|_| self.error(range, "constructor head token ownership is unavailable"))?;
        let prefix_tokens = list_token - start_token;
        let mut syntax_limits = self.call_limits.syntax;
        syntax_limits.max_arguments = syntax_limits
            .max_arguments
            .min(self.admission.calls.type_arguments);
        let reserve = |amount: usize, extra: usize, label| {
            amount
                .checked_sub(extra)
                .ok_or(GenericCopyBodyError::Limit(label))
        };
        syntax_limits.types.max_source_bytes = reserve(
            syntax_limits
                .types
                .max_source_bytes
                .min(self.admission.calls.source_bytes),
            prefix_bytes,
            "constructor source",
        )?;
        syntax_limits.types.max_tokens = reserve(
            syntax_limits
                .types
                .max_tokens
                .min(self.admission.calls.tokens),
            prefix_tokens,
            "constructor tokens",
        )?;
        syntax_limits.types.max_nodes = reserve(
            syntax_limits
                .types
                .max_nodes
                .min(self.admission.calls.input_nodes),
            1,
            "constructor input nodes",
        )?;
        syntax_limits.types.max_name_bytes = reserve(
            syntax_limits
                .types
                .max_name_bytes
                .min(self.admission.calls.name_bytes),
            name.len(),
            "constructor input names",
        )?;
        syntax_limits.types.max_edges = syntax_limits
            .types
            .max_edges
            .min(self.admission.calls.input_edges);
        let parsed = parse_generic_type_arguments(fragment, syntax_limits).map_err(|error| {
            GenericCopyBodyError::Call {
                function: self.function,
                type_range: list,
                error: Box::new(GenericBodyCallError::Syntax(error)),
            }
        })?;
        let count = parsed.roots().len();
        let syntax = parsed.constructor_type(
            self.signature.syntax().source_owner(),
            list,
            name,
            range,
            prefix_tokens,
        );
        charge(
            &mut self.admission.calls.type_arguments,
            count,
            "constructor type arguments",
        )?;
        self.finish_constructor_type(syntax, range, expected, is_enum)
    }
    fn finish_constructor_type(
        &mut self,
        syntax: super::GenericTypeSyntax,
        range: Span,
        expected: GenericTypeDeclarationId,
        is_enum: bool,
    ) -> Result<(ExpandedGenericType, Span), GenericCopyBodyError> {
        charge(
            &mut self.admission.calls.source_bytes,
            range.end - range.start,
            "constructor source",
        )?;
        charge(
            &mut self.admission.calls.tokens,
            syntax.token_count(),
            "constructor tokens",
        )?;
        charge(
            &mut self.admission.calls.input_nodes,
            syntax.nodes().len(),
            "constructor input nodes",
        )?;
        charge(
            &mut self.admission.calls.input_edges,
            syntax.edges(),
            "constructor input edges",
        )?;
        charge(
            &mut self.admission.calls.name_bytes,
            syntax.name_bytes(),
            "constructor input names",
        )?;
        let mut work = self.call_limits.arguments.max_work.min(self.admission.work);
        for amount in [syntax.nodes().len(), syntax.edges(), syntax.name_bytes()] {
            charge(&mut work, amount, "constructor copy work")?;
            charge(&mut self.admission.work, amount, "body work")?;
        }
        let mut expansion = self.call_limits.arguments.expansion;
        expansion.max_nodes = expansion
            .max_nodes
            .min(self.call_limits.arguments.max_nodes)
            .min(self.admission.calls.nodes);
        expansion.max_edges = expansion
            .max_edges
            .min(self.call_limits.arguments.max_edges)
            .min(self.admission.calls.edges);
        expansion.max_work = expansion.max_work.min(work).min(self.admission.work);
        let ty = self
            .signatures
            .types()
            .environment()
            .expand(&syntax, self.signature.syntax().parameters(), expansion)
            .map_err(|error| {
                if is_enum {
                    GenericCopyBodyError::EnumType {
                        function: self.function,
                        type_range: range,
                        error: Box::new(error),
                    }
                } else {
                    GenericCopyBodyError::RecordType {
                        function: self.function,
                        type_range: range,
                        error: Box::new(error),
                    }
                }
            })?;
        charge(
            &mut self.admission.calls.nodes,
            ty.nodes().len(),
            "constructor generated nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            ty.edges(),
            "constructor generated edges",
        )?;
        charge(&mut self.admission.work, ty.work(), "body work")?;
        if !matches!(ty.nodes()[ty.root()].kind, ResolvedGenericTypeKind::Declared{declaration,..} if declaration==expected)
        {
            return Err(self.error(
                range,
                "constructor type does not identify its nominal declaration",
            ));
        }
        Ok((ty, range))
    }
    fn array_compatible(
        &mut self,
        actual: &GenericCopyType,
        expected: &GenericCopyType,
        span: Span,
    ) -> Result<(), GenericCopyBodyError> {
        let (mut a, mut e) = (actual.clone(), expected.clone());
        loop {
            self.admission.step()?;
            match (
                a.expanded().and_then(ExpandedGenericType::array_element),
                e.expanded().and_then(ExpandedGenericType::array_element),
            ) {
                (Some(x), Some(y)) => {
                    a = GenericCopyType(TypeKind::Expanded(x));
                    e = GenericCopyType(TypeKind::Expanded(y));
                }
                _ => {
                    if a.primitive() == Some(GenericPrimitive::Int)
                        && e.primitive() == Some(GenericPrimitive::Float)
                    {
                        return Ok(());
                    }
                    if self
                        .admission
                        .same(self.function, &a, &e, self.comparison)?
                    {
                        return Ok(());
                    }
                    return Err(self.error(span, "array operation has incompatible element types"));
                }
            }
        }
    }
    fn collection_option_type(
        &mut self,
        ty: &GenericCopyType,
        span: Span,
    ) -> Result<GenericCopyType, GenericCopyBodyError> {
        let mut option = None;
        for definition in self.signatures.types().definitions() {
            self.admission.step()?;
            if let CheckedGenericDefinition::Enum(def) = definition {
                if def.syntax().standard_identity() == Some(super::GenericStandardEnum::Option) {
                    option = Some(def);
                    break;
                }
            }
        }
        let option = option.ok_or_else(|| self.error(span, "standard Option unavailable"))?;
        let mut limits = self.call_limits.arguments.expansion;
        limits.max_nodes = limits
            .max_nodes
            .min(self.call_limits.arguments.max_nodes)
            .min(self.admission.calls.nodes);
        limits.max_edges = limits
            .max_edges
            .min(self.call_limits.arguments.max_edges)
            .min(self.admission.calls.edges);
        limits.max_work = limits
            .max_work
            .min(self.call_limits.arguments.max_work)
            .min(self.admission.work);
        let expanded = ty
            .expanded()
            .ok_or_else(|| self.error(span, "collection payload type"))?
            .wrap_standard_option(option, limits)
            .map_err(|error| GenericCopyBodyError::EnumType {
                function: self.function,
                type_range: span,
                error: Box::new(error),
            })?;
        charge(
            &mut self.admission.calls.nodes,
            expanded.nodes().len(),
            "lookup generated nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            expanded.edges(),
            "lookup generated edges",
        )?;
        charge(
            &mut self.admission.work,
            expanded.work(),
            "lookup type work",
        )?;
        Ok(GenericCopyType::concrete_expanded(expanded))
    }
    fn collection_array_type(
        &mut self,
        ty: &GenericCopyType,
        span: Span,
    ) -> Result<GenericCopyType, GenericCopyBodyError> {
        let mut limits = self.call_limits.arguments.expansion;
        limits.max_nodes = limits
            .max_nodes
            .min(self.call_limits.arguments.max_nodes)
            .min(self.admission.calls.nodes);
        limits.max_edges = limits
            .max_edges
            .min(self.call_limits.arguments.max_edges)
            .min(self.admission.calls.edges);
        limits.max_work = limits
            .max_work
            .min(self.call_limits.arguments.max_work)
            .min(self.admission.work);
        let expanded = ty
            .expanded()
            .ok_or_else(|| self.error(span, "collection payload type"))?
            .wrap_array(limits)
            .map_err(|error| GenericCopyBodyError::FieldType {
                function: self.function,
                span,
                error: Box::new(error),
            })?;
        charge(
            &mut self.admission.calls.nodes,
            expanded.nodes().len(),
            "iteration generated nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            expanded.edges(),
            "iteration generated edges",
        )?;
        charge(
            &mut self.admission.work,
            expanded.work(),
            "iteration type work",
        )?;
        Ok(GenericCopyType::concrete_expanded(expanded))
    }
    fn collection_entries_type(
        &mut self,
        ty: &GenericCopyType,
        span: Span,
    ) -> Result<GenericCopyType, GenericCopyBodyError> {
        let mut entry = None;
        for definition in self.signatures.types().definitions() {
            self.admission.step()?;
            if let CheckedGenericDefinition::Record(def) = definition {
                if def.syntax().is_standard_map_entry() {
                    entry = Some(def);
                    break;
                }
            }
        }
        let entry = entry.ok_or_else(|| self.error(span, "standard MapEntry unavailable"))?;
        let mut limits = self.call_limits.arguments.expansion;
        limits.max_nodes = limits
            .max_nodes
            .min(self.call_limits.arguments.max_nodes)
            .min(self.admission.calls.nodes);
        limits.max_edges = limits
            .max_edges
            .min(self.call_limits.arguments.max_edges)
            .min(self.admission.calls.edges);
        limits.max_work = limits
            .max_work
            .min(self.call_limits.arguments.max_work)
            .min(self.admission.work);
        let expanded = ty
            .expanded()
            .ok_or_else(|| self.error(span, "collection payload type"))?
            .map_entry_array(entry, limits)
            .map_err(|error| GenericCopyBodyError::FieldType {
                function: self.function,
                span,
                error: Box::new(error),
            })?;
        charge(
            &mut self.admission.calls.nodes,
            expanded.nodes().len(),
            "iteration generated nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            expanded.edges(),
            "iteration generated edges",
        )?;
        charge(
            &mut self.admission.work,
            expanded.work(),
            "iteration type work",
        )?;
        Ok(GenericCopyType::concrete_expanded(expanded))
    }
    fn prepare_data_result(
        &mut self,
        callee: &Expression,
        success: DataResultKind,
    ) -> Result<GenericCopyType, GenericCopyBodyError> {
        let kind = if success == DataResultKind::Bytes {
            super::GenericDataTypeKind::Bytes
        } else {
            super::GenericDataTypeKind::Byte
        };
        let (mut byte, mut error, mut result, mut json) = (None, None, None, None);
        for definition in self.signatures.types().definitions() {
            self.admission.step()?;
            match definition {
                CheckedGenericDefinition::Data(d) if d.syntax().kind() == kind => byte = Some(d),
                CheckedGenericDefinition::Record(r) if r.syntax().is_standard_error() => {
                    error = Some(r)
                }
                CheckedGenericDefinition::Enum(e)
                    if e.syntax().standard_identity()
                        == Some(super::GenericStandardEnum::Result) =>
                {
                    result = Some(e)
                }
                CheckedGenericDefinition::Enum(e)
                    if e.syntax().standard_identity()
                        == Some(super::GenericStandardEnum::JsonValue) =>
                {
                    json = Some(e)
                }
                _ => {}
            }
        }
        let (byte, error, result) = (
            byte.ok_or_else(|| self.error(callee.span, "Byte schema unavailable"))?,
            error.ok_or_else(|| self.error(callee.span, "Error schema unavailable"))?,
            result.ok_or_else(|| self.error(callee.span, "Result schema unavailable"))?,
        );
        let syntax = super::GenericTypeArgumentsSyntax::absent().constructor_type(
            self.signature.syntax().source_owner(),
            callee.span,
            "std::Error",
            callee.span,
            4,
        );
        let (anchor, _) = self.finish_constructor_type(syntax, callee.span, error.id(), false)?;
        let mut limits = self.call_limits.arguments.expansion;
        limits.max_nodes = limits
            .max_nodes
            .min(self.call_limits.arguments.max_nodes)
            .min(self.admission.calls.nodes);
        limits.max_edges = limits
            .max_edges
            .min(self.call_limits.arguments.max_edges)
            .min(self.admission.calls.edges);
        limits.max_work = limits
            .max_work
            .min(self.call_limits.arguments.max_work)
            .min(self.admission.work);
        let ty = anchor
            .data_result_type(byte, error, result, limits, success, json)
            .map_err(|error| GenericCopyBodyError::EnumType {
                function: self.function,
                type_range: callee.span,
                error: Box::new(error),
            })?;
        charge(
            &mut self.admission.calls.nodes,
            ty.nodes().len(),
            "byte result nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            ty.edges(),
            "byte result edges",
        )?;
        charge(&mut self.admission.work, ty.work(), "byte result work")?;
        Ok(GenericCopyType::concrete_expanded(ty))
    }
    fn prepare_builtin_type(
        &mut self,
        callee: &Expression,
        builtin: super::GenericBuiltin,
    ) -> Result<GenericCopyType, GenericCopyBodyError> {
        if matches!(
            builtin,
            super::GenericBuiltin::JsonParse | super::GenericBuiltin::HostCall
        ) {
            return self.prepare_data_result(callee, DataResultKind::JsonValue);
        }
        if builtin == super::GenericBuiltin::JsonStringify {
            return self.prepare_data_result(callee, DataResultKind::String);
        }
        if matches!(
            builtin,
            super::GenericBuiltin::ByteFromInt | super::GenericBuiltin::BytesGet
        ) {
            return self.prepare_data_result(callee, DataResultKind::Byte);
        }
        if matches!(
            builtin,
            super::GenericBuiltin::BytesConcat
                | super::GenericBuiltin::BytesSlice
                | super::GenericBuiltin::BytesFromArray
                | super::GenericBuiltin::BytesFromText
                | super::GenericBuiltin::BytesFromHex
                | super::GenericBuiltin::BytesFromBase64
        ) {
            return self.prepare_data_result(callee, DataResultKind::Bytes);
        }
        if matches!(
            builtin,
            super::GenericBuiltin::BytesToHex
                | super::GenericBuiltin::BytesToBase64
                | super::GenericBuiltin::BytesToUtf8
        ) {
            return self.prepare_data_result(callee, DataResultKind::String);
        }
        if builtin == super::GenericBuiltin::BytesToArray {
            return self.prepare_data_result(callee, DataResultKind::ByteArray);
        }
        if let Some(kind) = builtin.result_primitive() {
            self.admission.step()?;
            return Ok(GenericCopyType(TypeKind::Primitive(kind)));
        }
        let mut declaration = None;
        for definition in self.signatures.types().definitions() {
            self.admission.step()?;
            if let CheckedGenericDefinition::Record(record) = definition {
                if record.syntax().is_standard_result()
                    && record.syntax().qualified_name() == builtin.result_name()
                {
                    declaration = Some(record.id());
                    break;
                }
            }
        }
        let declaration = declaration
            .ok_or_else(|| self.error(callee.span, "standard builtin result schema unavailable"))?;
        let limits = self.call_limits.syntax.types;
        if limits.max_depth == 0
            || limits.max_depth > 128
            || limits.max_nodes < 1
            || limits.max_tokens < 4
            || limits.max_name_bytes < builtin.result_name().len()
            || limits.max_source_bytes < callee.span.end - callee.span.start
        {
            return Err(GenericCopyBodyError::Limit("builtin result type syntax"));
        }
        let syntax = super::GenericTypeArgumentsSyntax::absent().constructor_type(
            self.signature.syntax().source_owner(),
            callee.span,
            builtin.result_name(),
            callee.span,
            4,
        );
        let (ty, _) = self.finish_constructor_type(syntax, callee.span, declaration, false)?;
        Ok(GenericCopyType(TypeKind::Expanded(ty)))
    }
    fn prepare_standard_record_type(
        &mut self,
        span: Span,
        expected: GenericTypeDeclarationId,
        canonical: &str,
    ) -> Result<(ExpandedGenericType, Span), GenericCopyBodyError> {
        let leaf = canonical
            .strip_prefix("std::")
            .ok_or_else(|| self.error(span, "standard record identity"))?;
        let mut at = self
            .tokens
            .binary_search_by_key(&span.start, |token| token.span.start)
            .map_err(|_| self.error(span, "standard record source ownership is unavailable"))?;
        while self.tokens.get(at).map(|t| &t.kind) == Some(&TokenKind::Punctuation('(')) {
            self.admission.step()?;
            at += 1;
        }
        self.admission.step()?;
        if !matches!(self.tokens.get(at).map(|t|&t.kind),Some(TokenKind::Identifier(name)) if name=="std")
            || self.tokens.get(at + 1).map(|t| &t.kind) != Some(&TokenKind::Operator("::"))
            || !matches!(self.tokens.get(at+2).map(|t|&t.kind),Some(TokenKind::Identifier(name)) if name==leaf)
            || self.tokens.get(at + 3).map(|t| &t.kind) != Some(&TokenKind::Punctuation('{'))
        {
            return Err(self.error(
                span,
                "expected an original qualified standard record constructor",
            ));
        }
        let range = Span {
            end: self.tokens[at + 2].span.end,
            ..self.tokens[at].span
        };
        let limits = self.call_limits.syntax.types;
        if limits.max_depth == 0
            || limits.max_depth > 128
            || limits.max_nodes < 1
            || limits.max_name_bytes < canonical.len()
            || limits.max_tokens < 4
            || limits.max_source_bytes < range.end - range.start
        {
            return Err(GenericCopyBodyError::Limit("standard record type syntax"));
        }
        let origin = Span {
            start: range.end,
            end: range.end,
            line: self.tokens[at + 2].span.line,
            column: self.tokens[at + 2].span.column + leaf.len(),
        };
        // The exact three original head tokens were checked above; no written
        // type-argument list is synthesized. Include one EOF in type-token cost.
        let syntax = super::GenericTypeArgumentsSyntax::absent().constructor_type(
            self.signature.syntax().source_owner(),
            origin,
            canonical,
            range,
            4,
        );
        self.finish_constructor_type(syntax, range, expected, false)
    }
    fn prepare_record(
        &mut self,
        expression: &Expression,
        name: &str,
        fields: &[(String, Expression)],
    ) -> Result<RecordPlan, GenericCopyBodyError> {
        charge(&mut self.admission.records, 1, "record constructions")?;
        charge(
            &mut self.admission.calls.values,
            fields.len(),
            "constructor field edges",
        )?;
        let signatures = self.signatures;
        let registry = signatures.types().environment().registry();
        let scoped = registry.is_package_scoped();
        let nominal = registry
            .find_header_in(name, self.signature.syntax().parameters())
            .map(|header| header.id);
        let mut record = None;
        for definition in signatures.types().definitions() {
            self.admission.step()?;
            if let CheckedGenericDefinition::Record(candidate) = definition {
                if if scoped {
                    nominal == Some(candidate.id())
                } else {
                    candidate.syntax().qualified_name() == name
                } {
                    record = Some(candidate);
                    break;
                }
            }
        }
        let record = record.ok_or_else(|| {
            self.error(
                expression.span,
                "constructor requires a registered root record",
            )
        })?;
        if fields.len() != record.field_types().len() {
            return Err(self.error(
                expression.span,
                "constructor fields do not cover the record",
            ));
        }
        let (ty, range) =
            if record.syntax().is_standard_error() || record.syntax().is_standard_result() {
                if self.call_ranges.contains_key(&expression.span.start) {
                    return Err(self.error(
                        expression.span,
                        if record.syntax().is_standard_error() {
                            "std::Error has no type parameters"
                        } else {
                            "standard result records have no type parameters"
                        },
                    ));
                }
                self.prepare_standard_record_type(
                    expression.span,
                    record.id(),
                    record.syntax().qualified_name(),
                )?
            } else {
                let list = *self
                    .call_ranges
                    .get(&expression.span.start)
                    .ok_or_else(|| {
                        self.error(
                            expression.span,
                            "generic record construction requires explicit type arguments",
                        )
                    })?;
                self.prepare_constructor_type(name, list, expression.span, record.id(), false)?
            };
        let mut seen = std::collections::BTreeSet::new();
        let mut expected = Vec::new();
        for (name, _) in fields {
            let mut field_index = None;
            for (index, field) in record.syntax().fields().iter().enumerate() {
                self.admission.step()?;
                if field.name == *name {
                    field_index = Some(index);
                    break;
                }
            }
            let index = field_index
                .ok_or_else(|| self.error(expression.span, "unknown constructor field"))?;
            if !seen.insert(index) {
                return Err(self.error(expression.span, "duplicate constructor field"));
            }
            let template = &record.field_types()[index];
            let mut limits = self.field_limits.expansion();
            limits.max_nodes = limits.max_nodes.min(self.admission.field_nodes);
            limits.max_edges = limits.max_edges.min(self.admission.field_edges);
            limits.max_work = limits.max_work.min(self.admission.work);
            let field = ty
                .substitute_field(template, record.id(), record.syntax().parameters(), limits)
                .map_err(|error| GenericCopyBodyError::FieldType {
                    function: self.function,
                    span: expression.span,
                    error: Box::new(error),
                })?;
            charge(
                &mut self.admission.field_nodes,
                field.nodes().len(),
                "constructor field nodes",
            )?;
            charge(
                &mut self.admission.field_edges,
                field.edges(),
                "constructor field edges",
            )?;
            charge(&mut self.admission.work, field.work(), "body work")?;
            expected.push((index, GenericCopyType(TypeKind::Expanded(field))));
        }
        Ok(RecordPlan {
            ty,
            declaration: record.id(),
            range,
            fields: expected,
        })
    }
    fn prepare_enum(
        &mut self,
        callee: &Expression,
        path: &[String],
        values: usize,
    ) -> Result<EnumPlan, GenericCopyBodyError> {
        charge(&mut self.admission.enums, 1, "enum constructions")?;
        charge(
            &mut self.admission.calls.values,
            values,
            "enum payload edges",
        )?;
        let scoped = self
            .signature
            .types()
            .environment()
            .registry()
            .is_package_scoped();
        let (name, variant_name) = if scoped && path.len() >= 2 && path.len() <= 4 {
            let bytes = path
                .iter()
                .try_fold(0usize, |n, p| n.checked_add(p.len() + 2))
                .ok_or(GenericCopyBodyError::Limit("package enum path"))?;
            charge(&mut self.admission.work, bytes, "package enum path")?;
            if bytes > self.call_limits.arguments.max_name_bytes {
                return Err(GenericCopyBodyError::Limit("package enum path"));
            }
            (
                Cow::Owned(path[..path.len() - 1].join("::")),
                path.last().unwrap().as_str(),
            )
        } else {
            let (name, variant_name) = match path {
                [name, variant] => (name.as_str(), variant.as_str()),
                [namespace, name, variant] if namespace == "std" && name == "Option" => {
                    ("std::Option", variant.as_str())
                }
                [namespace, name, variant] if namespace == "std" && name == "Result" => {
                    ("std::Result", variant.as_str())
                }
                _ => {
                    return Err(self.error(
                        callee.span,
                        "only registered root or standard enum constructors are supported",
                    ))
                }
            };
            (Cow::Borrowed(name), variant_name)
        };
        let list = self.call_ranges.get(&callee.span.start).copied();
        let signatures = self.signatures;
        let nominal = signatures
            .types()
            .environment()
            .registry()
            .find_header_in(&name, self.signature.syntax().parameters())
            .map(|h| h.id);
        let mut definition = None;
        for candidate in signatures.types().definitions() {
            self.admission.step()?;
            if let CheckedGenericDefinition::Enum(candidate) = candidate {
                if if scoped {
                    nominal == Some(candidate.id())
                } else {
                    candidate.syntax().qualified_name() == name
                } {
                    definition = Some(candidate);
                    break;
                }
            }
        }
        let definition = definition.ok_or_else(|| {
            self.error(callee.span, "constructor requires a registered root enum")
        })?;
        let mut variant_index = None;
        for (index, variant) in definition.syntax().variants().iter().enumerate() {
            self.admission.step()?;
            if variant.name == variant_name {
                variant_index = Some(index);
                break;
            }
        }
        let variant_index =
            variant_index.ok_or_else(|| self.error(callee.span, "unknown enum variant"))?;
        let template = definition.payload_types()[variant_index].as_ref();
        if values != usize::from(template.is_some()) {
            return Err(self.error(
                callee.span,
                "enum constructor payload count differs from its variant",
            ));
        }
        let (ty, type_range, variant_range) = if let Some(list) = list {
            let result =
                self.prepare_constructor_type(&name, list, callee.span, definition.id(), true)?;
            let after = self
                .tokens
                .partition_point(|token| token.span.start < list.end);
            let variant = self.tokens.get(after + 1).ok_or_else(|| {
                self.error(callee.span, "enum variant token ownership is unavailable")
            })?;
            if self.tokens.get(after).map(|t| &t.kind) != Some(&TokenKind::Operator("::"))
                || !matches!(&variant.kind,TokenKind::Identifier(name)if name==variant_name)
            {
                return Err(self.error(
                    callee.span,
                    "explicit enum arguments must follow the enum head",
                ));
            }
            (result.0, result.1, variant.span)
        } else {
            if definition.syntax().standard_identity()
                != Some(super::GenericStandardEnum::JsonValue)
                || !definition.syntax().parameters().parameters().is_empty()
            {
                return Err(self.error(
                    callee.span,
                    "enum constructors require explicit type arguments",
                ));
            }
            let syntax = super::GenericTypeArgumentsSyntax::absent().constructor_type(
                self.signature.syntax().source_owner(),
                callee.span,
                "std::JsonValue",
                callee.span,
                4,
            );
            let (ty, range) =
                self.finish_constructor_type(syntax, callee.span, definition.id(), true)?;
            let end = self
                .tokens
                .partition_point(|t| t.span.start < callee.span.end);
            let variant = self
                .tokens
                .get(end.saturating_sub(1))
                .ok_or_else(|| self.error(callee.span, "JSON variant token"))?;
            if !matches!(&variant.kind,TokenKind::Identifier(name)if name==variant_name) {
                return Err(self.error(callee.span, "JSON variant ownership"));
            }
            (ty, range, variant.span)
        };
        let payload = if let Some(template) = template {
            let mut limits = self.field_limits.expansion();
            limits.max_nodes = limits.max_nodes.min(self.admission.field_nodes);
            limits.max_edges = limits.max_edges.min(self.admission.field_edges);
            limits.max_work = limits.max_work.min(self.admission.work);
            let payload = ty
                .substitute_field(
                    template,
                    definition.id(),
                    definition.syntax().parameters(),
                    limits,
                )
                .map_err(|error| GenericCopyBodyError::FieldType {
                    function: self.function,
                    span: callee.span,
                    error: Box::new(error),
                })?;
            charge(
                &mut self.admission.field_nodes,
                payload.nodes().len(),
                "enum payload nodes",
            )?;
            charge(
                &mut self.admission.field_edges,
                payload.edges(),
                "enum payload edges",
            )?;
            charge(&mut self.admission.work, payload.work(), "body work")?;
            Some(GenericCopyType(TypeKind::Expanded(payload)))
        } else {
            None
        };
        Ok(EnumPlan {
            ty,
            declaration: definition.id(),
            type_range,
            variant_index,
            variant_range,
            payload,
        })
    }
    fn name_token_span(&mut self, span: Span, name: &str) -> Result<Span, GenericCopyBodyError> {
        self.admission.step()?;
        let mut at = self
            .tokens
            .binary_search_by_key(&span.start, |t| t.span.start)
            .map_err(|_| self.error(span, "name token start"))?;
        while self.tokens.get(at).map(|t| &t.kind) == Some(&TokenKind::Punctuation('(')) {
            self.admission.step()?;
            at += 1;
        }
        self.admission.step()?;
        let token = self
            .tokens
            .get(at)
            .ok_or_else(|| self.error(span, "name identifier token"))?;
        if !matches!(&token.kind,TokenKind::Identifier(actual) if actual==name)
            || token.span.end > span.end
        {
            return Err(self.error(span, "name identifier provenance"));
        }
        Ok(token.span)
    }
    fn record_name_use(
        &mut self,
        binding: GenericCopyBindingId,
        span: Span,
    ) -> Result<(), GenericCopyBodyError> {
        if binding.function != self.function || binding.arena != self.arena {
            return Err(self.error(span, "name use binding owner"));
        }
        let name = Arc::clone(
            &self
                .bindings
                .get(binding.ordinal)
                .ok_or_else(|| self.error(span, "name use binding"))?
                .name,
        );
        let span = self.name_token_span(span, &name)?;
        charge(&mut self.admission.name_uses, 1, "body name uses")?;
        self.name_uses.push(GenericCopyNameUse { binding, span });
        Ok(())
    }
    fn callee_name<'a>(
        &mut self,
        callee: &'a Expression,
    ) -> Result<Cow<'a, str>, GenericCopyBodyError> {
        match &callee.kind {
            ExpressionKind::Identifier(name) => Ok(Cow::Borrowed(name)),
            ExpressionKind::QualifiedName { path }
                if self
                    .signature
                    .types()
                    .environment()
                    .registry()
                    .is_package_scoped() =>
            {
                let bytes = path
                    .iter()
                    .try_fold(0usize, |n, p| {
                        n.checked_add(p.len()).and_then(|n| n.checked_add(2))
                    })
                    .ok_or(GenericCopyBodyError::Limit("package callee name"))?;
                charge(&mut self.admission.work, bytes, "package callee names")?;
                if bytes > self.call_limits.arguments.max_name_bytes {
                    return Err(GenericCopyBodyError::Limit("package callee name"));
                }
                Ok(Cow::Owned(path.join("::")))
            }
            _ => Err(self.error(
                callee.span,
                "only explicit root or linked package calls are supported",
            )),
        }
    }
    fn call_name_span(&mut self, callee: &Expression) -> Result<Span, GenericCopyBodyError> {
        if let ExpressionKind::Identifier(name) = &callee.kind {
            return self.name_token_span(callee.span, name);
        }
        let ExpressionKind::QualifiedName { path } = &callee.kind else {
            return Err(self.error(callee.span, "callee token identity"));
        };
        let mut at = self
            .tokens
            .binary_search_by_key(&callee.span.start, |t| t.span.start)
            .map_err(|_| self.error(callee.span, "callee path start"))?;
        while self.tokens.get(at).map(|t| &t.kind) == Some(&TokenKind::Punctuation('(')) {
            self.admission.step()?;
            at += 1;
        }
        let start = self
            .tokens
            .get(at)
            .ok_or_else(|| self.error(callee.span, "callee path token"))?
            .span;
        let mut end = start.end;
        for (index, name) in path.iter().enumerate() {
            self.admission.step()?;
            if index > 0 {
                if self.tokens.get(at).map(|t| &t.kind) != Some(&TokenKind::Operator("::")) {
                    return Err(self.error(callee.span, "callee path separator"));
                }
                at += 1;
            }
            let token = self
                .tokens
                .get(at)
                .ok_or_else(|| self.error(callee.span, "callee path token"))?;
            if !matches!(&token.kind,TokenKind::Identifier(actual) if actual==name)
                || token.span.end > callee.span.end
            {
                return Err(self.error(callee.span, "callee path provenance"));
            }
            end = token.span.end;
            at += 1;
        }
        Ok(Span { end, ..start })
    }
    fn prepare_call(
        &mut self,
        callee: &Expression,
        values: usize,
    ) -> Result<(InstantiatedGenericSignature, Span), GenericCopyBodyError> {
        let name = self.callee_name(callee)?;
        self.admission.step()?;
        if self
            .scope
            .contains_key(name.split("::").next().unwrap_or(&name))
        {
            return Err(self.error(
                callee.span,
                "root callee is shadowed by an active value binding",
            ));
        }
        let written = self.call_ranges.get(&callee.span.start).copied();
        let range = if let Some(range) = written {
            range
        } else {
            self.admission.step()?;
            let end = self
                .tokens
                .binary_search_by_key(&callee.span.end, |token| token.span.end)
                .ok()
                .and_then(|at| self.tokens.get(at))
                .ok_or_else(|| {
                    self.error(callee.span, "ordinary callee source end is unavailable")
                })?;
            Span {
                start: end.span.end,
                end: end.span.end,
                line: end.span.line,
                column: end.span.column + end.span.end - end.span.start,
            }
        };
        charge(&mut self.admission.calls.calls, 1, "source calls")?;
        charge(&mut self.admission.calls.values, values, "call value edges")?;
        let registry = self.signature.types().environment().registry();
        let package_owner =
            registry.package_function_owner(&name, self.signature.syntax().parameters());
        let mut found = None;
        for signature in self.signatures.signatures() {
            self.admission.step()?;
            if if registry.is_package_scoped() {
                package_owner == Some(signature.syntax().parameters().scope_key())
            } else {
                signature.syntax().name() == name
            } {
                found = Some(signature);
                break;
            }
        }
        let target = found.ok_or_else(|| self.error(callee.span, "unknown root generic callee"))?;
        if values != target.parameter_types().len() {
            return Err(self.error(
                callee.span,
                "generic call value count differs from callee signature",
            ));
        }
        if written.is_none() {
            if !target.syntax().parameters().parameters().is_empty() {
                return Err(
                    self.error(callee.span, "generic calls require explicit type arguments")
                );
            }
            let target = target.id();
            let checked = super::CheckedGenericTypeArguments::absent(
                self.signatures.types(),
                self.signature.syntax().parameters(),
                self.call_limits.arguments.max_work.min(self.admission.work),
            )
            .map_err(|error| GenericCopyBodyError::Call {
                function: self.function,
                type_range: range,
                error: Box::new(GenericBodyCallError::Arguments(error)),
            })?;
            charge(&mut self.admission.work, checked.work(), "body work")?;
            return self.finish_call(&checked, target, range);
        }
        let fragment = self
            .signature
            .syntax()
            .source()
            .get(range.start..range.end)
            .ok_or_else(|| self.error(range, "call type-list source ownership is unavailable"))?;
        let wrap = |error| GenericCopyBodyError::Call {
            function: self.function,
            type_range: range,
            error: Box::new(error),
        };
        let mut syntax_limits = self.call_limits.syntax;
        syntax_limits.max_arguments = syntax_limits
            .max_arguments
            .min(self.admission.calls.type_arguments);
        syntax_limits.types.max_source_bytes = syntax_limits
            .types
            .max_source_bytes
            .min(self.admission.calls.source_bytes);
        syntax_limits.types.max_tokens = syntax_limits
            .types
            .max_tokens
            .min(self.admission.calls.tokens);
        syntax_limits.types.max_nodes = syntax_limits
            .types
            .max_nodes
            .min(self.admission.calls.input_nodes);
        syntax_limits.types.max_edges = syntax_limits
            .types
            .max_edges
            .min(self.admission.calls.input_edges);
        syntax_limits.types.max_name_bytes = syntax_limits
            .types
            .max_name_bytes
            .min(self.admission.calls.name_bytes);
        let syntax = parse_generic_type_arguments(fragment, syntax_limits)
            .map_err(|e| wrap(GenericBodyCallError::Syntax(e)))?;
        let mut arguments = self.call_limits.arguments;
        arguments.max_arguments = arguments
            .max_arguments
            .min(self.admission.calls.type_arguments);
        arguments.max_source_bytes = arguments
            .max_source_bytes
            .min(self.admission.calls.source_bytes);
        arguments.max_tokens = arguments.max_tokens.min(self.admission.calls.tokens);
        arguments.max_input_nodes = arguments
            .max_input_nodes
            .min(self.admission.calls.input_nodes);
        arguments.max_input_edges = arguments
            .max_input_edges
            .min(self.admission.calls.input_edges);
        arguments.max_name_bytes = arguments
            .max_name_bytes
            .min(self.admission.calls.name_bytes);
        arguments.max_nodes = arguments.max_nodes.min(self.admission.calls.nodes);
        arguments.max_edges = arguments.max_edges.min(self.admission.calls.edges);
        arguments.max_work = arguments.max_work.min(self.admission.work);
        let checked = check_generic_type_arguments(
            self.signatures.types(),
            &syntax,
            self.signature.syntax().parameters(),
            arguments,
        )
        .map_err(|e| wrap(GenericBodyCallError::Arguments(e)))?;
        charge(
            &mut self.admission.calls.type_arguments,
            syntax.roots().len(),
            "call type arguments",
        )?;
        charge(
            &mut self.admission.calls.source_bytes,
            fragment.len(),
            "call type-list source",
        )?;
        charge(
            &mut self.admission.calls.tokens,
            syntax.token_count(),
            "call type-list tokens",
        )?;
        charge(
            &mut self.admission.calls.input_nodes,
            syntax.nodes().len(),
            "call input nodes",
        )?;
        charge(
            &mut self.admission.calls.input_edges,
            syntax.edges(),
            "call input edges",
        )?;
        charge(
            &mut self.admission.calls.name_bytes,
            syntax.name_bytes(),
            "call input names",
        )?;
        charge(
            &mut self.admission.calls.nodes,
            checked.nodes(),
            "call generated nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            checked.edges(),
            "call generated edges",
        )?;
        charge(&mut self.admission.work, checked.work(), "body work")?;
        self.finish_call(&checked, target.id(), range)
    }
    fn finish_call(
        &mut self,
        checked: &super::CheckedGenericTypeArguments,
        target: GenericSignatureId,
        range: Span,
    ) -> Result<(InstantiatedGenericSignature, Span), GenericCopyBodyError> {
        let wrap = |error| GenericCopyBodyError::Call {
            function: self.function,
            type_range: range,
            error: Box::new(error),
        };
        let mut matching = self.call_limits.binding;
        matching.max_work = matching.max_work.min(self.admission.work);
        let bound =
            bind_generic_type_arguments(self.signatures, self.function, target, checked, matching)
                .map_err(|e| wrap(GenericBodyCallError::Binding(e)))?;
        charge(&mut self.admission.work, bound.work(), "body work")?;
        let mut substitution = self.call_limits.signature;
        substitution.max_nodes = substitution.max_nodes.min(self.admission.calls.nodes);
        substitution.max_edges = substitution.max_edges.min(self.admission.calls.edges);
        substitution.max_work = substitution.max_work.min(self.admission.work);
        let instantiated = instantiate_generic_signature(&bound, substitution)
            .map_err(|e| wrap(GenericBodyCallError::Signature(e)))?;
        charge(
            &mut self.admission.calls.nodes,
            instantiated.nodes(),
            "call generated nodes",
        )?;
        charge(
            &mut self.admission.calls.edges,
            instantiated.edges(),
            "call generated edges",
        )?;
        charge(&mut self.admission.work, instantiated.work(), "body work")?;
        Ok((instantiated, range))
    }
    fn propagation_site(
        &mut self,
        span: Span,
    ) -> Result<Option<super::GenericPropagationSyntaxSite>, GenericCopyBodyError> {
        let mut current = span;
        loop {
            self.admission.step()?;
            if let Some(site) = self.propagation_sites.remove(&(current.start, current.end)) {
                return Ok(Some(site));
            }
            let Ok(start) = self
                .tokens
                .binary_search_by_key(&current.start, |token| token.span.start)
            else {
                return Ok(None);
            };
            let end = self
                .tokens
                .partition_point(|token| token.span.start < current.end);
            if end <= start + 2
                || self.tokens[start].kind != TokenKind::Punctuation('(')
                || self.tokens[end - 1].kind != TokenKind::Punctuation(')')
            {
                return Ok(None);
            }
            let mut depth = 0usize;
            let mut enclosed = true;
            for (index, token) in self.tokens[start..end].iter().enumerate() {
                self.admission.step()?;
                match token.kind {
                    TokenKind::Punctuation('(') => depth += 1,
                    TokenKind::Punctuation(')') => {
                        depth = depth
                            .checked_sub(1)
                            .ok_or(GenericCopyBodyError::Limit("propagation group ownership"))?;
                        if depth == 0 && start + index + 1 != end {
                            enclosed = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if !enclosed || depth != 0 {
                return Ok(None);
            }
            current = Span {
                end: self.tokens[end - 2].span.end,
                ..self.tokens[start + 1].span
            };
        }
    }
    fn expr(&mut self, root: &Expression) -> Result<GenericCopyExpressionId, GenericCopyBodyError> {
        self.expr_expected(root, None)
    }
    fn expr_expected(
        &mut self,
        root: &Expression,
        expected: Option<GenericCopyType>,
    ) -> Result<GenericCopyExpressionId, GenericCopyBodyError> {
        enum Work<'a> {
            Visit(&'a Expression, Option<GenericCopyType>, Option<Span>),
            Propagate(super::GenericPropagationSyntaxSite, Span),
            Array(Span, GenericCopyType, GenericCopyType, usize),
            Index(Span),
            Call(Span, Box<InstantiatedGenericSignature>, Span, Span, usize),
            Builtin(Span, super::GenericBuiltin, GenericCopyType),
            ArrayArguments(Span, super::GenericBuiltin, &'a [Expression]),
            CollectionArguments(Span, super::GenericBuiltin, &'a [Expression]),
            Field(Span, &'a str),
            Record(Span, Box<RecordPlan>),
            Enum(Span, Box<EnumPlan>),
            Finish(Span, bool),
            Numeric(Span, GenericNumericOperator),
            Logical(Span, GenericLogicalOperator),
        }
        let mut pending = vec![Work::Visit(root, expected, None)];
        let mut completed = Vec::new();
        while let Some(work) = pending.pop() {
            self.admission.step()?;
            match work {
                Work::Visit(e, expected, override_span) => {
                    let span = override_span.unwrap_or(e.span);
                    if self.admission.propagation.is_some() {
                        if let Some(site) = self.propagation_site(span)? {
                            pending.push(Work::Propagate(site, span));
                            pending.push(Work::Visit(e, None, Some(site.operand)));
                            continue;
                        }
                    }
                    let primitive = match &e.kind {
                        ExpressionKind::Identifier(name) => {
                            self.admission.step()?;
                            let binding = *self
                                .scope
                                .get(name.as_str())
                                .ok_or_else(|| self.error(span, "unknown value name"))?;
                            let ty = self.bindings[binding.ordinal].ty.clone();
                            completed.push(self.push(
                                span,
                                GenericCopyExpressionKind::Read(binding),
                                ty,
                            )?);
                            continue;
                        }
                        ExpressionKind::Integer(value) => {
                            if value.parse::<i64>().is_err() {
                                return Err(
                                    self.error(span, "integer literal is outside Int range")
                                );
                            }
                            GenericPrimitive::Int
                        }
                        ExpressionKind::Float(value) => {
                            if !value.parse::<f64>().is_ok_and(|v| v.is_finite()) {
                                return Err(self.error(span, "floating literal is not finite"));
                            }
                            GenericPrimitive::Float
                        }
                        ExpressionKind::String(_) => GenericPrimitive::String,
                        ExpressionKind::Boolean(_) => GenericPrimitive::Bool,
                        ExpressionKind::StructLiteral { type_name, fields } => {
                            let plan = self.prepare_record(e, type_name, fields)?;
                            let contexts: Vec<_> =
                                plan.fields.iter().map(|(_, ty)| Some(ty.clone())).collect();
                            pending.push(Work::Record(span, Box::new(plan)));
                            pending.extend(
                                fields
                                    .iter()
                                    .zip(contexts)
                                    .rev()
                                    .map(|((_, value), context)| Work::Visit(value, context, None)),
                            );
                            continue;
                        }
                        ExpressionKind::Call { callee, arguments } => {
                            if self
                                .propagation_sites
                                .contains_key(&(callee.span.start, callee.span.end))
                            {
                                return Err(self.error(
                                    callee.span,
                                    "propagated callable heads are unsupported",
                                ));
                            }
                            if let ExpressionKind::QualifiedName { path } = &callee.kind {
                                if self.builtins != 0 {
                                    let builtin = match path.as_slice() {
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "host_call" =>
                                        {
                                            Some(super::GenericBuiltin::HostCall)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "json_parse" =>
                                        {
                                            Some(super::GenericBuiltin::JsonParse)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "json_stringify" =>
                                        {
                                            Some(super::GenericBuiltin::JsonStringify)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_to_utf8" =>
                                        {
                                            Some(super::GenericBuiltin::BytesToUtf8)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_to_base64" =>
                                        {
                                            Some(super::GenericBuiltin::BytesToBase64)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_from_base64" =>
                                        {
                                            Some(super::GenericBuiltin::BytesFromBase64)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_to_hex" =>
                                        {
                                            Some(super::GenericBuiltin::BytesToHex)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_from_hex" =>
                                        {
                                            Some(super::GenericBuiltin::BytesFromHex)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_to_array" =>
                                        {
                                            Some(super::GenericBuiltin::BytesToArray)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_slice" =>
                                        {
                                            Some(super::GenericBuiltin::BytesSlice)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_get" =>
                                        {
                                            Some(super::GenericBuiltin::BytesGet)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_concat" =>
                                        {
                                            Some(super::GenericBuiltin::BytesConcat)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_from_array" =>
                                        {
                                            Some(super::GenericBuiltin::BytesFromArray)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_from_text" =>
                                        {
                                            Some(super::GenericBuiltin::BytesFromText)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "bytes_len" =>
                                        {
                                            Some(super::GenericBuiltin::BytesLen)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "byte_from_int" =>
                                        {
                                            Some(super::GenericBuiltin::ByteFromInt)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 6
                                                && root == "std"
                                                && leaf == "byte_to_int" =>
                                        {
                                            Some(super::GenericBuiltin::ByteToInt)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_entries" =>
                                        {
                                            Some(super::GenericBuiltin::MapEntries)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_keys" =>
                                        {
                                            Some(super::GenericBuiltin::MapKeys)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_values" =>
                                        {
                                            Some(super::GenericBuiltin::MapValues)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_values" =>
                                        {
                                            Some(super::GenericBuiltin::SetValues)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_remove" =>
                                        {
                                            Some(super::GenericBuiltin::MapRemove)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_remove" =>
                                        {
                                            Some(super::GenericBuiltin::SetRemove)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_get" =>
                                        {
                                            Some(super::GenericBuiltin::MapGet)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_get" =>
                                        {
                                            Some(super::GenericBuiltin::SetGet)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_contains" =>
                                        {
                                            Some(super::GenericBuiltin::MapContains)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_contains" =>
                                        {
                                            Some(super::GenericBuiltin::SetContains)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_insert" =>
                                        {
                                            Some(super::GenericBuiltin::MapInsert)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_insert" =>
                                        {
                                            Some(super::GenericBuiltin::SetInsert)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_new" =>
                                        {
                                            Some(super::GenericBuiltin::MapNew)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_new" =>
                                        {
                                            Some(super::GenericBuiltin::SetNew)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "map_len" =>
                                        {
                                            Some(super::GenericBuiltin::MapLen)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 5
                                                && root == "std"
                                                && leaf == "set_len" =>
                                        {
                                            Some(super::GenericBuiltin::SetLen)
                                        }
                                        [root, leaf] if root == "std" && leaf == "parse_int" => {
                                            Some(super::GenericBuiltin::ParseInt)
                                        }
                                        [root, leaf] if root == "std" && leaf == "split_once" => {
                                            Some(super::GenericBuiltin::SplitOnce)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 2
                                                && root == "std"
                                                && leaf == "len" =>
                                        {
                                            Some(super::GenericBuiltin::StringLen)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 2
                                                && root == "std"
                                                && leaf == "lines_unique" =>
                                        {
                                            Some(super::GenericBuiltin::LinesUnique)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 2
                                                && root == "std"
                                                && leaf == "escape_terminal" =>
                                        {
                                            Some(super::GenericBuiltin::EscapeTerminal)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 3
                                                && root == "std"
                                                && leaf == "to_string" =>
                                        {
                                            Some(super::GenericBuiltin::ToString)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 3
                                                && root == "std"
                                                && leaf == "array_len" =>
                                        {
                                            Some(super::GenericBuiltin::ArrayLen)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 4
                                                && root == "std"
                                                && leaf == "array_append" =>
                                        {
                                            Some(super::GenericBuiltin::ArrayAppend)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 4
                                                && root == "std"
                                                && leaf == "array_concat" =>
                                        {
                                            Some(super::GenericBuiltin::ArrayConcat)
                                        }
                                        [root, leaf]
                                            if self.builtins >= 4
                                                && root == "std"
                                                && leaf == "array_slice" =>
                                        {
                                            Some(super::GenericBuiltin::ArraySlice)
                                        }
                                        _ => None,
                                    };
                                    if let Some(builtin) = builtin {
                                        if arguments.len() != builtin.arity()
                                            || self.call_ranges.contains_key(&callee.span.start)
                                        {
                                            return Err(self.error(
                                                span,
                                                "standard builtin arity/type arguments",
                                            ));
                                        }
                                        charge(
                                            &mut self.admission.calls.calls,
                                            1,
                                            "source builtin calls",
                                        )?;
                                        charge(
                                            &mut self.admission.calls.values,
                                            arguments.len(),
                                            "builtin argument edges",
                                        )?;
                                        if let Some(kind) = builtin.collection_kind() {
                                            if builtin.collection_insert()
                                                || builtin.collection_contains()
                                                || builtin.collection_get()
                                                || builtin.collection_remove()
                                                || builtin.collection_iteration()
                                            {
                                                pending.push(Work::CollectionArguments(
                                                    span, builtin, arguments,
                                                ));
                                                pending.push(Work::Visit(
                                                    &arguments[0],
                                                    if builtin.collection_insert() {
                                                        expected.clone()
                                                    } else {
                                                        None
                                                    },
                                                    None,
                                                ));
                                                continue;
                                            }
                                            if builtin.collection_new() {
                                                let ty=expected.clone().ok_or_else(||self.error(span,"collection_new requires an explicit result type context"))?;
                                                if collection_type_kind(&ty, self.signature.types())
                                                    != Some(kind)
                                                {
                                                    return Err(self.error(span,"collection_new result constructor mismatch"));
                                                }
                                                completed.push(self.push(
                                                    span,
                                                    GenericCopyExpressionKind::Builtin {
                                                        builtin,
                                                        arguments: Vec::new(),
                                                    },
                                                    ty,
                                                )?);
                                            } else {
                                                pending.push(Work::Builtin(
                                                    span,
                                                    builtin,
                                                    GenericCopyType::concrete_primitive(
                                                        GenericPrimitive::Int,
                                                    ),
                                                ));
                                                pending.push(Work::Visit(
                                                    &arguments[0],
                                                    None,
                                                    None,
                                                ));
                                            }
                                            continue;
                                        }
                                        if builtin.array_operation() {
                                            pending.push(Work::ArrayArguments(
                                                span, builtin, arguments,
                                            ));
                                            pending.push(Work::Visit(
                                                &arguments[0],
                                                expected.clone(),
                                                None,
                                            ));
                                            continue;
                                        }
                                        let ty = self.prepare_builtin_type(callee, builtin)?;
                                        pending.push(Work::Builtin(span, builtin, ty));
                                        pending.extend(arguments.iter().rev().map(|arg| {
                                            Work::Visit(
                                                arg,
                                                if matches!(
                                                    builtin,
                                                    super::GenericBuiltin::ToString
                                                        | super::GenericBuiltin::ArrayLen
                                                ) {
                                                    None
                                                } else {
                                                    Some(GenericCopyType(TypeKind::Primitive(
                                                        GenericPrimitive::String,
                                                    )))
                                                },
                                                None,
                                            )
                                        }));
                                        continue;
                                    }
                                }
                                let package_function = if self
                                    .signature
                                    .types()
                                    .environment()
                                    .registry()
                                    .is_package_scoped()
                                {
                                    let name = self.callee_name(callee)?;
                                    self.signature
                                        .types()
                                        .environment()
                                        .registry()
                                        .package_function_owner(
                                            &name,
                                            self.signature.syntax().parameters(),
                                        )
                                        .is_some()
                                } else {
                                    false
                                };
                                if !package_function {
                                    let plan = self.prepare_enum(callee, path, arguments.len())?;
                                    let context = plan.payload.clone();
                                    pending.push(Work::Enum(span, Box::new(plan)));
                                    if let Some(context) = context {
                                        pending.push(Work::Visit(
                                            &arguments[0],
                                            Some(context),
                                            None,
                                        ));
                                    }
                                    continue;
                                }
                            }
                            let (signature, range) = self.prepare_call(callee, arguments.len())?;
                            let callee_span = self.call_name_span(callee)?;
                            let contexts: Vec<_> = signature
                                .parameter_types()
                                .iter()
                                .cloned()
                                .map(|ty| Some(GenericCopyType(TypeKind::Expanded(ty))))
                                .collect();
                            pending.push(Work::Call(
                                span,
                                Box::new(signature),
                                range,
                                callee_span,
                                arguments.len(),
                            ));
                            pending.extend(
                                arguments
                                    .iter()
                                    .zip(contexts)
                                    .rev()
                                    .map(|(argument, context)| {
                                        Work::Visit(argument, context, None)
                                    }),
                            );
                            continue;
                        }
                        ExpressionKind::FieldAccess { receiver, field } => {
                            pending.push(Work::Field(span, field));
                            pending.push(Work::Visit(receiver, None, None));
                            continue;
                        }
                        ExpressionKind::ArrayLiteral(items) => {
                            let expected = expected.ok_or_else(|| {
                                self.error(
                                    span,
                                    "array literal requires an explicit expected array type",
                                )
                            })?;
                            self.admission.step()?;
                            let element = expected
                                .expanded()
                                .and_then(ExpandedGenericType::array_element)
                                .ok_or_else(|| {
                                    self.error(
                                        span,
                                        "array literal requires an expected array type",
                                    )
                                })?;
                            let element = GenericCopyType(TypeKind::Expanded(element));
                            pending.push(Work::Array(span, expected, element.clone(), items.len()));
                            pending.extend(
                                items
                                    .iter()
                                    .rev()
                                    .map(|item| Work::Visit(item, Some(element.clone()), None)),
                            );
                            continue;
                        }
                        ExpressionKind::Index { target, index } => {
                            pending.push(Work::Index(span));
                            pending.push(Work::Visit(index, None, None));
                            pending.push(Work::Visit(target, None, None));
                            continue;
                        }
                        ExpressionKind::Binary {
                            left,
                            operator,
                            right,
                        } if operator == "==" || operator == "!=" => {
                            pending.push(Work::Finish(span, operator == "!="));
                            pending.push(Work::Visit(right, None, None));
                            pending.push(Work::Visit(left, None, None));
                            continue;
                        }
                        ExpressionKind::Binary {
                            left,
                            operator,
                            right,
                        } if operator == "&&" || operator == "||" => {
                            let operator = if operator == "&&" {
                                GenericLogicalOperator::And
                            } else {
                                GenericLogicalOperator::Or
                            };
                            pending.push(Work::Logical(span, operator));
                            pending.push(Work::Visit(right, None, None));
                            pending.push(Work::Visit(left, None, None));
                            continue;
                        }
                        ExpressionKind::Binary {
                            left,
                            operator,
                            right,
                        } => {
                            let Some(operator) = GenericNumericOperator::from_spelling(operator)
                            else {
                                return Err(self
                                    .error(span, "operator is outside the symbolic body profile"));
                            };
                            pending.push(Work::Numeric(span, operator));
                            pending.push(Work::Visit(right, None, None));
                            pending.push(Work::Visit(left, None, None));
                            continue;
                        }
                        _ => {
                            return Err(self.error(
                                span,
                                "expression is outside the generic copy/equality body profile",
                            ))
                        }
                    };
                    completed.push(self.push(
                        span,
                        GenericCopyExpressionKind::Literal(primitive),
                        GenericCopyType(TypeKind::Primitive(primitive)),
                    )?);
                }
                Work::Propagate(site, result_span) => {
                    let operand: GenericCopyExpressionId =
                        completed.pop().expect("checked propagation operand");
                    let original = self
                        .admission
                        .propagation
                        .ok_or(GenericCopyBodyError::Limit("propagation profile"))?;
                    let mut local = original;
                    local.max_work = local.max_work.min(self.admission.work);
                    local.max_pairs = local
                        .max_pairs
                        .min(self.admission.pairs)
                        .min(self.comparison.max_pairs);
                    local.max_edges = local
                        .max_edges
                        .min(self.admission.edges)
                        .min(self.comparison.max_edges);
                    let before = local;
                    let types = super::generic_propagation::check_propagation_type(
                        self.signature,
                        &self.expressions[operand.ordinal].ty,
                        site.operand,
                        &mut local,
                        self.comparison,
                    )
                    .map_err(|error| GenericCopyBodyError::Propagation {
                        function: self.function,
                        span: site.question,
                        error: Box::new(error),
                    })?;
                    let used = before.max_work - local.max_work;
                    let pairs = before.max_pairs - local.max_pairs;
                    let edges = before.max_edges - local.max_edges;
                    charge(
                        &mut self.admission.pairs,
                        pairs,
                        "propagation body type pairs",
                    )?;
                    charge(
                        &mut self.admission.edges,
                        edges,
                        "propagation body type edges",
                    )?;
                    local.max_work = original.max_work - used;
                    local.max_pairs = original.max_pairs - pairs;
                    local.max_edges = original.max_edges - edges;
                    self.admission.propagation = Some(local);
                    charge(&mut self.admission.work, used, "body work")?;
                    let ty = GenericCopyType::concrete_expanded(types.success_type().clone());
                    completed.push(self.push(
                        result_span,
                        GenericCopyExpressionKind::Propagate {
                            operand,
                            question: site.question,
                            types: Box::new(types),
                        },
                        ty,
                    )?);
                }
                Work::Enum(e, plan) => {
                    let EnumPlan {
                        ty,
                        declaration,
                        type_range,
                        variant_index,
                        variant_range,
                        payload,
                    } = *plan;
                    let payload = if let Some(expected) = payload {
                        let value: GenericCopyExpressionId =
                            completed.pop().expect("checked enum payload");
                        self.admission.step()?;
                        let actual = self.expressions[value.ordinal].ty.clone();
                        let conversion = if self.admission.same(
                            self.function,
                            &actual,
                            &expected,
                            self.comparison,
                        )? {
                            GenericCopyConversion::Identity
                        } else if actual.primitive() == Some(GenericPrimitive::Int)
                            && expected.primitive() == Some(GenericPrimitive::Float)
                        {
                            GenericCopyConversion::IntToFloat
                        } else {
                            return Err(self.error(
                                self.expressions[value.ordinal].span,
                                "enum payload has an incompatible symbolic type",
                            ));
                        };
                        Some(GenericEnumPayloadValue { value, conversion })
                    } else {
                        None
                    };
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Enum {
                            declaration,
                            type_range,
                            variant_index,
                            variant_range,
                            payload,
                        },
                        GenericCopyType(TypeKind::Expanded(ty)),
                    )?);
                }
                Work::Record(e, plan) => {
                    let RecordPlan {
                        ty,
                        declaration,
                        range,
                        fields: expected,
                    } = *plan;
                    let values = completed.split_off(completed.len() - expected.len());
                    let mut fields = Vec::with_capacity(expected.len());
                    for (value, (field_index, expected)) in values.into_iter().zip(expected) {
                        self.admission.step()?;
                        let actual = self.expressions[value.ordinal].ty.clone();
                        let conversion = if self.admission.same(
                            self.function,
                            &actual,
                            &expected,
                            self.comparison,
                        )? {
                            GenericCopyConversion::Identity
                        } else if actual.primitive() == Some(GenericPrimitive::Int)
                            && expected.primitive() == Some(GenericPrimitive::Float)
                        {
                            GenericCopyConversion::IntToFloat
                        } else {
                            return Err(self.error(
                                self.expressions[value.ordinal].span,
                                "constructor field value has an incompatible symbolic type",
                            ));
                        };
                        fields.push(GenericRecordFieldValue {
                            field_index,
                            value,
                            conversion,
                        });
                    }
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Record {
                            declaration,
                            type_range: range,
                            fields,
                        },
                        GenericCopyType(TypeKind::Expanded(ty)),
                    )?);
                }
                Work::CollectionArguments(span, builtin, args) => {
                    let first = *completed
                        .last()
                        .ok_or_else(|| self.error(span, "collection first argument"))?;
                    let ty = self.expressions[first.ordinal()].ty.clone();
                    if collection_type_kind(&ty, self.signature.types())
                        != builtin.collection_kind()
                    {
                        return Err(self.error(span, "collection operation receiver constructor"));
                    }
                    let mut contexts = Vec::new();
                    for index in 0..args.len() - 1 {
                        contexts.push(
                            collection_argument(&ty, index)
                                .ok_or_else(|| self.error(span, "collection argument type"))?,
                        );
                    }
                    let result = if builtin == super::GenericBuiltin::MapEntries {
                        self.collection_entries_type(&ty, span)?
                    } else if builtin.collection_iteration() {
                        let index = usize::from(builtin == super::GenericBuiltin::MapValues);
                        let element = collection_argument(&ty, index)
                            .ok_or_else(|| self.error(span, "iteration element type"))?;
                        self.collection_array_type(&element, span)?
                    } else if builtin.collection_remove() {
                        self.collection_option_type(&ty, span)?
                    } else if builtin.collection_get() {
                        let index = usize::from(builtin == super::GenericBuiltin::MapGet);
                        let payload = collection_argument(&ty, index)
                            .ok_or_else(|| self.error(span, "lookup payload type"))?;
                        self.collection_option_type(&payload, span)?
                    } else if builtin.collection_contains() {
                        GenericCopyType(TypeKind::Primitive(GenericPrimitive::Bool))
                    } else {
                        ty
                    };
                    pending.push(Work::Builtin(span, builtin, result));
                    pending.extend(
                        args[1..]
                            .iter()
                            .zip(contexts)
                            .rev()
                            .map(|(arg, ty)| Work::Visit(arg, Some(ty), None)),
                    );
                }
                Work::ArrayArguments(span, builtin, args) => {
                    let first = *completed
                        .last()
                        .ok_or_else(|| self.error(span, "array argument fact"))?;
                    let array = self.expressions[first.ordinal()].ty.clone();
                    let element = array
                        .expanded()
                        .and_then(ExpandedGenericType::array_element)
                        .ok_or_else(|| {
                            self.error(span, "array operation requires a typed first array")
                        })?;
                    let right = if builtin == super::GenericBuiltin::ArrayAppend {
                        GenericCopyType(TypeKind::Expanded(element))
                    } else if builtin == super::GenericBuiltin::ArrayConcat {
                        array.clone()
                    } else {
                        GenericCopyType(TypeKind::Primitive(GenericPrimitive::Int))
                    };
                    pending.push(Work::Builtin(span, builtin, array));
                    for arg in args[1..].iter().rev() {
                        pending.push(Work::Visit(arg, Some(right.clone()), None));
                    }
                }
                Work::Builtin(span, builtin, ty) => {
                    let start = completed
                        .len()
                        .checked_sub(builtin.arity())
                        .ok_or_else(|| self.error(span, "builtin argument facts unavailable"))?;
                    let arguments = completed.split_off(start);
                    if builtin.collection_insert()
                        || builtin.collection_contains()
                        || builtin.collection_get()
                        || builtin.collection_remove()
                        || builtin.collection_iteration()
                    {
                        for (index, id) in arguments[1..].iter().enumerate() {
                            let receiver = self.expressions[arguments[0].ordinal()].ty.clone();
                            let expected = collection_argument(&receiver, index)
                                .ok_or_else(|| self.error(span, "collection argument type"))?;
                            let actual = self.expressions[id.ordinal()].ty.clone();
                            if !self.admission.same(
                                self.function,
                                &actual,
                                &expected,
                                self.comparison,
                            )? {
                                return Err(self.error(
                                    span,
                                    "collection operation requires exact key/value types",
                                ));
                            }
                        }
                        completed.push(self.push(
                            span,
                            GenericCopyExpressionKind::Builtin { builtin, arguments },
                            ty,
                        )?);
                        continue;
                    }
                    if builtin.array_operation() {
                        if builtin == super::GenericBuiltin::ArraySlice {
                            for id in &arguments[1..] {
                                self.admission.step()?;
                                if self.expressions[id.ordinal()].ty.primitive()
                                    != Some(GenericPrimitive::Int)
                                {
                                    return Err(self.error(span, "array_slice requires Int bounds"));
                                }
                            }
                        } else {
                            let expected = if builtin == super::GenericBuiltin::ArrayAppend {
                                GenericCopyType(TypeKind::Expanded(
                                    ty.expanded()
                                        .and_then(ExpandedGenericType::array_element)
                                        .ok_or_else(|| self.error(span, "append element type"))?,
                                ))
                            } else {
                                ty.clone()
                            };
                            let actual = self.expressions[arguments[1].ordinal()].ty.clone();
                            self.array_compatible(&actual, &expected, span)?;
                        }
                        completed.push(self.push(
                            span,
                            GenericCopyExpressionKind::Builtin { builtin, arguments },
                            ty,
                        )?);
                        continue;
                    }
                    for (argument_index, id) in arguments.iter().enumerate() {
                        self.admission.step()?;
                        let actual = &self.expressions[id.ordinal()].ty;
                        if matches!(
                            builtin,
                            super::GenericBuiltin::JsonParse
                                | super::GenericBuiltin::JsonStringify
                                | super::GenericBuiltin::HostCall
                        ) {
                            let valid = if builtin == super::GenericBuiltin::JsonParse {
                                actual.primitive() == Some(GenericPrimitive::String)
                            } else {
                                is_json_type(actual, self.signature.types())
                            };
                            if !valid {
                                return Err(self.error(span, "JSON operand type"));
                            }
                            continue;
                        }
                        if let Some(kind) = builtin.collection_kind() {
                            if collection_type_kind(actual, self.signature.types()) != Some(kind) {
                                return Err(
                                    self.error(span, "collection_len operand constructor mismatch")
                                );
                            }
                            continue;
                        }
                        if matches!(
                            builtin,
                            super::GenericBuiltin::BytesFromArray
                                | super::GenericBuiltin::BytesFromText
                                | super::GenericBuiltin::BytesLen
                                | super::GenericBuiltin::BytesGet
                                | super::GenericBuiltin::BytesConcat
                                | super::GenericBuiltin::BytesSlice
                                | super::GenericBuiltin::BytesToArray
                                | super::GenericBuiltin::BytesToHex
                                | super::GenericBuiltin::BytesFromHex
                                | super::GenericBuiltin::BytesToBase64
                                | super::GenericBuiltin::BytesFromBase64
                                | super::GenericBuiltin::BytesToUtf8
                        ) {
                            let valid = match builtin {
                                super::GenericBuiltin::BytesFromArray => {
                                    is_byte_array(actual, self.signature.types())
                                }
                                super::GenericBuiltin::BytesFromText
                                | super::GenericBuiltin::BytesFromHex
                                | super::GenericBuiltin::BytesFromBase64 => {
                                    actual.primitive() == Some(GenericPrimitive::String)
                                }
                                super::GenericBuiltin::BytesSlice if argument_index > 0 => {
                                    actual.primitive() == Some(GenericPrimitive::Int)
                                }
                                super::GenericBuiltin::BytesGet if argument_index == 1 => {
                                    actual.primitive() == Some(GenericPrimitive::Int)
                                }
                                _ => is_bytes_type(actual, self.signature.types()),
                            };
                            if !valid {
                                return Err(self.error(span, "byte buffer operand type"));
                            }
                            continue;
                        }
                        if builtin == super::GenericBuiltin::ByteFromInt {
                            if actual.primitive() != Some(GenericPrimitive::Int) {
                                return Err(self.error(span, "byte_from_int requires Int"));
                            }
                            continue;
                        }
                        if builtin == super::GenericBuiltin::ByteToInt {
                            if !actual.expanded().is_some_and(|e|matches!(e.nodes()[e.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..} if super::generic_data_types::is_byte_header(self.signature.types().environment().registry(),declaration))){return Err(self.error(span,"byte_to_int requires Byte"));}
                            continue;
                        }
                        if builtin == super::GenericBuiltin::ToString {
                            continue;
                        }
                        if builtin == super::GenericBuiltin::ArrayLen {
                            if !matches!(
                                actual.expanded().map(|ty| &ty.nodes()[ty.root()].kind),
                                Some(ResolvedGenericTypeKind::Array { .. })
                            ) {
                                return Err(self.error(span, "array_len requires an array"));
                            }
                            continue;
                        }
                        if actual.primitive() != Some(GenericPrimitive::String) {
                            return Err(
                                self.error(span, "standard builtin requires String arguments")
                            );
                        }
                    }
                    completed.push(self.push(
                        span,
                        GenericCopyExpressionKind::Builtin { builtin, arguments },
                        ty,
                    )?);
                }
                Work::Call(e, signature, type_range, callee_span, count) => {
                    let values = completed.split_off(completed.len() - count);
                    let mut arguments = Vec::with_capacity(count);
                    for (value, expected) in values.into_iter().zip(signature.parameter_types()) {
                        self.admission.step()?;
                        let actual = self.expressions[value.ordinal].ty.clone();
                        let expected = GenericCopyType(TypeKind::Expanded(expected.clone()));
                        let conversion = if self.admission.same(
                            self.function,
                            &actual,
                            &expected,
                            self.comparison,
                        )? {
                            GenericCopyConversion::Identity
                        } else if actual.primitive() == Some(GenericPrimitive::Int)
                            && expected.primitive() == Some(GenericPrimitive::Float)
                        {
                            GenericCopyConversion::IntToFloat
                        } else {
                            return Err(self.error(
                                self.expressions[value.ordinal].span,
                                "generic call argument has an incompatible symbolic type",
                            ));
                        };
                        arguments.push(GenericCallValueArgument {
                            expression: value,
                            conversion,
                        });
                    }
                    let result = match signature.return_type() {
                        GenericSignatureReturn::ImplicitUnit => {
                            GenericCopyType(TypeKind::Primitive(GenericPrimitive::Unit))
                        }
                        GenericSignatureReturn::Explicit(ty) => {
                            GenericCopyType(TypeKind::Expanded(ty.clone()))
                        }
                    };
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Call {
                            signature,
                            callee_span,
                            type_range,
                            arguments,
                        },
                        result,
                    )?);
                }
                Work::Field(e, field) => {
                    let receiver: GenericCopyExpressionId =
                        completed.pop().expect("checked receiver");
                    let ty = self.expressions[receiver.ordinal]
                        .ty
                        .expanded()
                        .ok_or_else(|| self.error(e, "field access requires a record"))?;
                    let ResolvedGenericTypeKind::Declared { declaration, .. } =
                        ty.nodes()[ty.root()].kind
                    else {
                        return Err(self.error(e, "field access requires a record"));
                    };
                    self.admission.step()?;
                    let Some(CheckedGenericDefinition::Record(record)) =
                        self.signature.types().definition(declaration)
                    else {
                        return Err(self.error(e, "field access requires a checked record"));
                    };
                    let mut found = None;
                    for (index, candidate) in record.syntax().fields().iter().enumerate() {
                        self.admission.step()?;
                        if candidate.name == field {
                            found = Some(index);
                            break;
                        }
                    }
                    let field_index = found.ok_or_else(|| self.error(e, "unknown record field"))?;
                    let field_type = record.field_types()[field_index].clone();
                    if let ResolvedGenericTypeKind::Parameter(parameter) =
                        field_type.nodes()[field_type.root()].kind
                    {
                        self.admission.step()?;
                        if record.syntax().parameters().parameter(parameter).is_none() {
                            return Err(
                                self.error(e, "field parameter does not belong to its record")
                            );
                        }
                        let ordinal = parameter.ordinal();
                        let projected =
                            ty.declaration_argument(declaration, ordinal)
                                .ok_or_else(|| {
                                    self.error(e, "field argument projection is unavailable")
                                })?;
                        completed.push(self.push(
                            e,
                            GenericCopyExpressionKind::Field {
                                receiver,
                                declaration,
                                field_index,
                                parameter_argument: Some(ordinal),
                            },
                            GenericCopyType(TypeKind::Expanded(projected)),
                        )?);
                        continue;
                    }
                    let mut pending = vec![field_type.root()];
                    let mut visited = std::collections::BTreeSet::new();
                    let mut dependent = false;
                    while let Some(index) = pending.pop() {
                        self.admission.step()?;
                        if !visited.insert(index) {
                            continue;
                        }
                        match &field_type.nodes()[index].kind {
                            ResolvedGenericTypeKind::Parameter(_) => {
                                dependent = true;
                            }
                            ResolvedGenericTypeKind::Array { element } => {
                                self.admission.step()?;
                                pending.push(*element);
                            }
                            ResolvedGenericTypeKind::Declared { arguments, .. } => {
                                charge(&mut self.admission.work, arguments.len(), "body work")?;
                                pending.extend(arguments.iter().copied());
                            }
                            ResolvedGenericTypeKind::Primitive(_) => {}
                        }
                    }
                    let field_type = if dependent {
                        let mut limits = self.field_limits.expansion();
                        limits.max_nodes = limits.max_nodes.min(self.admission.field_nodes);
                        limits.max_edges = limits.max_edges.min(self.admission.field_edges);
                        limits.max_work = limits.max_work.min(self.admission.work);
                        let ty = ty
                            .substitute_field(
                                &field_type,
                                declaration,
                                record.syntax().parameters(),
                                limits,
                            )
                            .map_err(|error| GenericCopyBodyError::FieldType {
                                function: self.function,
                                span: e,
                                error: Box::new(error),
                            })?;
                        charge(
                            &mut self.admission.field_nodes,
                            ty.nodes().len(),
                            "field nodes",
                        )?;
                        charge(&mut self.admission.field_edges, ty.edges(), "field edges")?;
                        charge(&mut self.admission.work, ty.work(), "body work")?;
                        ty
                    } else {
                        field_type
                    };
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Field {
                            receiver,
                            declaration,
                            field_index,
                            parameter_argument: None,
                        },
                        GenericCopyType(TypeKind::Expanded(field_type)),
                    )?);
                }
                Work::Array(e, expected, element, count) => {
                    let values = completed.split_off(completed.len() - count);
                    let mut items = Vec::with_capacity(count);
                    for value in values {
                        self.admission.step()?;
                        let actual = self.expressions[value.ordinal].ty.clone();
                        let conversion = if self.admission.same(
                            self.function,
                            &actual,
                            &element,
                            self.comparison,
                        )? {
                            GenericCopyConversion::Identity
                        } else if actual.primitive() == Some(GenericPrimitive::Int)
                            && element.primitive() == Some(GenericPrimitive::Float)
                        {
                            GenericCopyConversion::IntToFloat
                        } else {
                            return Err(self.error(
                                self.expressions[value.ordinal].span,
                                "array item has an incompatible symbolic type",
                            ));
                        };
                        items.push(GenericCopyArrayElement { value, conversion });
                    }
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Array(items),
                        expected,
                    )?);
                }
                Work::Index(e) => {
                    let index: GenericCopyExpressionId = completed.pop().expect("checked index");
                    let target: GenericCopyExpressionId = completed.pop().expect("checked array");
                    if self.expressions[index.ordinal].ty.primitive() != Some(GenericPrimitive::Int)
                    {
                        return Err(self.error(
                            self.expressions[index.ordinal].span,
                            "array index requires concrete Int",
                        ));
                    }
                    self.admission.step()?;
                    let element = self.expressions[target.ordinal]
                        .ty
                        .expanded()
                        .and_then(ExpandedGenericType::array_element)
                        .ok_or_else(|| {
                            self.error(
                                self.expressions[target.ordinal].span,
                                "index target requires an array",
                            )
                        })?;
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Index { target, index },
                        GenericCopyType(TypeKind::Expanded(element)),
                    )?);
                }
                Work::Logical(e, operator) => {
                    let right: GenericCopyExpressionId =
                        completed.pop().expect("checked right operand");
                    let left: GenericCopyExpressionId =
                        completed.pop().expect("checked left operand");
                    if self.expressions[left.ordinal].ty.primitive() != Some(GenericPrimitive::Bool)
                        || self.expressions[right.ordinal].ty.primitive()
                            != Some(GenericPrimitive::Bool)
                    {
                        return Err(
                            self.error(e, "logical operators require concrete Bool operands")
                        );
                    }
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Logical {
                            operator,
                            left,
                            right,
                        },
                        GenericCopyType(TypeKind::Primitive(GenericPrimitive::Bool)),
                    )?);
                }
                Work::Numeric(e, operator) => {
                    let right: GenericCopyExpressionId =
                        completed.pop().expect("checked right operand");
                    let left: GenericCopyExpressionId =
                        completed.pop().expect("checked left operand");
                    let a = self.expressions[left.ordinal].ty.primitive();
                    let b = self.expressions[right.ordinal].ty.primitive();
                    if a == Some(GenericPrimitive::String) && b == Some(GenericPrimitive::String) {
                        let string_operator = match operator {
                            GenericNumericOperator::Add => GenericStringOperator::Concatenate,
                            GenericNumericOperator::Less => GenericStringOperator::Less,
                            GenericNumericOperator::LessEqual => GenericStringOperator::LessEqual,
                            GenericNumericOperator::Greater => GenericStringOperator::Greater,
                            GenericNumericOperator::GreaterEqual => {
                                GenericStringOperator::GreaterEqual
                            }
                            _ => {
                                return Err(
                                    self.error(e, "operator is not supported for String operands")
                                )
                            }
                        };
                        let result = if operator.ordering() {
                            GenericPrimitive::Bool
                        } else {
                            GenericPrimitive::String
                        };
                        completed.push(self.push(
                            e,
                            GenericCopyExpressionKind::String {
                                operator: string_operator,
                                left,
                                right,
                            },
                            GenericCopyType(TypeKind::Primitive(result)),
                        )?);
                        continue;
                    }
                    let (numeric, left_conversion, right_conversion) = match (a, b) {
                        (Some(GenericPrimitive::Int), Some(GenericPrimitive::Int)) => (
                            GenericPrimitive::Int,
                            GenericCopyConversion::Identity,
                            GenericCopyConversion::Identity,
                        ),
                        (Some(GenericPrimitive::Float), Some(GenericPrimitive::Float)) => (
                            GenericPrimitive::Float,
                            GenericCopyConversion::Identity,
                            GenericCopyConversion::Identity,
                        ),
                        (Some(GenericPrimitive::Int), Some(GenericPrimitive::Float)) => (
                            GenericPrimitive::Float,
                            GenericCopyConversion::IntToFloat,
                            GenericCopyConversion::Identity,
                        ),
                        (Some(GenericPrimitive::Float), Some(GenericPrimitive::Int)) => (
                            GenericPrimitive::Float,
                            GenericCopyConversion::Identity,
                            GenericCopyConversion::IntToFloat,
                        ),
                        _ => {
                            return Err(self.error(
                                e,
                                "numeric operators require concrete Int or Float operands",
                            ))
                        }
                    };
                    let result = if operator.ordering() {
                        GenericPrimitive::Bool
                    } else {
                        numeric
                    };
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Numeric {
                            operator,
                            left,
                            right,
                            left_conversion,
                            right_conversion,
                        },
                        GenericCopyType(TypeKind::Primitive(result)),
                    )?);
                }
                Work::Finish(e, negated) => {
                    let right: GenericCopyExpressionId =
                        completed.pop().expect("checked right operand");
                    let left: GenericCopyExpressionId =
                        completed.pop().expect("checked left operand");
                    let a = self.expressions[left.ordinal].ty.clone();
                    let b = self.expressions[right.ordinal].ty.clone();
                    let conversions =
                        if self
                            .admission
                            .same(self.function, &a, &b, self.comparison)?
                        {
                            (
                                GenericCopyConversion::Identity,
                                GenericCopyConversion::Identity,
                            )
                        } else {
                            match (a.primitive(), b.primitive()) {
                                (Some(GenericPrimitive::Int), Some(GenericPrimitive::Float)) => (
                                    GenericCopyConversion::IntToFloat,
                                    GenericCopyConversion::Identity,
                                ),
                                (Some(GenericPrimitive::Float), Some(GenericPrimitive::Int)) => (
                                    GenericCopyConversion::Identity,
                                    GenericCopyConversion::IntToFloat,
                                ),
                                _ => {
                                    return Err(self.error(
                                        e,
                                        "equality operands have incompatible symbolic types",
                                    ))
                                }
                            }
                        };
                    completed.push(self.push(
                        e,
                        GenericCopyExpressionKind::Equality {
                            left,
                            right,
                            negated,
                            left_conversion: conversions.0,
                            right_conversion: conversions.1,
                        },
                        GenericCopyType(TypeKind::Primitive(GenericPrimitive::Bool)),
                    )?);
                }
            }
        }
        Ok(completed.pop().expect("checked root expression"))
    }
    fn local_annotation(
        &mut self,
        range: Span,
    ) -> Result<GenericCopyLocalAnnotation, GenericCopyBodyError> {
        charge(
            &mut self.admission.locals.annotations,
            1,
            "local annotations",
        )?;
        let source = self
            .signature
            .syntax()
            .source()
            .get(range.start..range.end)
            .ok_or_else(|| self.error(range, "local type source ownership is unavailable"))?;
        charge(
            &mut self.admission.locals.source_bytes,
            source.len(),
            "local annotation source",
        )?;
        let wrap = |error| GenericCopyBodyError::Annotation {
            function: self.function,
            annotation_range: range,
            error: Box::new(error),
        };
        let mut syntax_limits = self.local_limits.syntax;
        syntax_limits.max_tokens = syntax_limits.max_tokens.min(self.admission.locals.tokens);
        syntax_limits.max_nodes = syntax_limits.max_nodes.min(self.admission.locals.nodes);
        syntax_limits.max_edges = syntax_limits.max_edges.min(self.admission.locals.edges);
        syntax_limits.max_name_bytes = syntax_limits
            .max_name_bytes
            .min(self.admission.locals.name_bytes);
        let syntax = parse_generic_type(source, syntax_limits)
            .map_err(|error| wrap(GenericLocalAnnotationError::Syntax(error)))?;
        charge(
            &mut self.admission.locals.tokens,
            syntax.token_count(),
            "local annotation tokens",
        )?;
        charge(
            &mut self.admission.locals.nodes,
            syntax.nodes().len(),
            "local annotation nodes",
        )?;
        charge(
            &mut self.admission.locals.edges,
            syntax.edges(),
            "local annotation edges",
        )?;
        charge(
            &mut self.admission.locals.name_bytes,
            syntax.name_bytes(),
            "local annotation names",
        )?;
        let mut expansion = self.local_limits.expansion;
        expansion.max_nodes = expansion
            .max_nodes
            .min(self.admission.locals.expanded_nodes);
        expansion.max_edges = expansion
            .max_edges
            .min(self.admission.locals.expanded_edges);
        expansion.max_work = expansion.max_work.min(self.admission.work);
        let ty = self
            .signature
            .types()
            .environment()
            .expand(&syntax, self.signature.syntax().parameters(), expansion)
            .map_err(|error| wrap(GenericLocalAnnotationError::Expansion(error)))?;
        charge(
            &mut self.admission.locals.expanded_nodes,
            ty.nodes().len(),
            "local expanded nodes",
        )?;
        charge(
            &mut self.admission.locals.expanded_edges,
            ty.edges(),
            "local expanded edges",
        )?;
        charge(&mut self.admission.work, ty.work(), "body work")?;
        Ok(GenericCopyLocalAnnotation { range, ty })
    }
    fn expected(&self) -> GenericCopyType {
        match self.signature.return_type() {
            GenericSignatureReturn::ImplicitUnit => {
                GenericCopyType(TypeKind::Primitive(GenericPrimitive::Unit))
            }
            GenericSignatureReturn::Explicit(ty) => GenericCopyType(TypeKind::Expanded(ty.clone())),
        }
    }
    fn check(&mut self, body: &[Statement], closing: Span) -> Result<bool, GenericCopyBodyError> {
        for (index, (value, ty)) in self
            .signature
            .syntax()
            .value_parameters()
            .iter()
            .zip(self.signature.parameter_types())
            .enumerate()
        {
            self.admission.step()?;
            self.bind(
                &value.name,
                value.name_span,
                GenericCopyBindingKind::Parameter(index),
                GenericCopyType(TypeKind::Expanded(ty.clone())),
            )?;
        }
        struct MatchFrame<'a> {
            index: usize,
            syntax: &'a super::MatchStatementSyntax,
            plans: Vec<super::GenericEnumPatternArm>,
            scope: BTreeMap<Arc<str>, GenericCopyBindingId>,
            parent_returned: bool,
            all_returned: bool,
            next: usize,
            scrutinee: GenericCopyExpressionId,
        }
        enum Work<'a> {
            MatchNext(Box<MatchFrame<'a>>),
            MatchEnd(Box<MatchFrame<'a>>),
            Statement(&'a Statement),
            EndWhile {
                index: usize,
                scope: BTreeMap<Arc<str>, GenericCopyBindingId>,
                parent_returned: bool,
            },
            EndThen {
                index: usize,
                scope: BTreeMap<Arc<str>, GenericCopyBindingId>,
                parent_returned: bool,
                otherwise: Option<&'a [Statement]>,
            },
            EndIf {
                index: usize,
                scope: BTreeMap<Arc<str>, GenericCopyBindingId>,
                parent_returned: bool,
                then_returned: bool,
                has_else: bool,
            },
        }
        let mut returned = false;
        let mut pending: Vec<_> = body.iter().rev().map(Work::Statement).collect();
        while let Some(work) = pending.pop() {
            self.admission.step()?;
            let statement = match work {
                Work::MatchEnd(mut frame) => {
                    let end = self.statements.len();
                    let GenericCopyStatement::Match { arms, .. } =
                        &mut self.statements[frame.index]
                    else {
                        unreachable!()
                    };
                    arms.last_mut().expect("active match arm").statements.end = end;
                    frame.all_returned &= returned;
                    frame.next += 1;
                    pending.push(Work::MatchNext(frame));
                    continue;
                }
                Work::MatchNext(mut frame) => {
                    if frame.next == frame.plans.len() {
                        self.scope = frame.scope;
                        returned = frame.parent_returned || frame.all_returned;
                        continue;
                    }
                    charge(&mut self.admission.work, frame.scope.len(), "body work")?;
                    self.scope = frame.scope.clone();
                    let plan = &mut frame.plans[frame.next];
                    let payload_binding = if let Some(payload) = plan.payload.take() {
                        Some(self.bind(
                            &payload.name,
                            payload.span,
                            GenericCopyBindingKind::MatchPayload {
                                scrutinee: frame.scrutinee,
                                variant_index: plan.variant_index,
                            },
                            GenericCopyType(TypeKind::Expanded(payload.ty)),
                        )?)
                    } else {
                        None
                    };
                    let start = self.statements.len();
                    let GenericCopyStatement::Match { arms, .. } =
                        &mut self.statements[frame.index]
                    else {
                        unreachable!()
                    };
                    arms.push(GenericCopyMatchArm {
                        variant_index: plan.variant_index,
                        payload_binding,
                        statements: start..start,
                        span: plan.span,
                    });
                    let body = &frame.syntax.arms[frame.next].body;
                    pending.push(Work::MatchEnd(frame));
                    returned = false;
                    pending.extend(body.iter().rev().map(Work::Statement));
                    continue;
                }
                Work::Statement(statement) => statement,
                Work::EndWhile {
                    index,
                    scope,
                    parent_returned,
                } => {
                    let end = self.statements.len();
                    let GenericCopyStatement::While {
                        body_statements, ..
                    } = &mut self.statements[index]
                    else {
                        unreachable!()
                    };
                    body_statements.end = end;
                    self.scope = scope;
                    // A loop may execute zero times, even when its body returns.
                    returned = parent_returned;
                    continue;
                }
                Work::EndThen {
                    index,
                    scope,
                    parent_returned,
                    otherwise,
                } => {
                    let end = self.statements.len();
                    let GenericCopyStatement::If {
                        then_statements,
                        else_statements,
                        ..
                    } = &mut self.statements[index]
                    else {
                        unreachable!()
                    };
                    then_statements.end = end;
                    *else_statements = otherwise.map(|_| end..end);
                    charge(&mut self.admission.work, scope.len(), "body work")?;
                    self.scope = scope.clone();
                    pending.push(Work::EndIf {
                        index,
                        scope,
                        parent_returned,
                        then_returned: returned,
                        has_else: otherwise.is_some(),
                    });
                    returned = false;
                    if let Some(otherwise) = otherwise {
                        pending.extend(otherwise.iter().rev().map(Work::Statement));
                    }
                    continue;
                }
                Work::EndIf {
                    index,
                    scope,
                    parent_returned,
                    then_returned,
                    has_else,
                } => {
                    let end = self.statements.len();
                    let GenericCopyStatement::If {
                        else_statements, ..
                    } = &mut self.statements[index]
                    else {
                        unreachable!()
                    };
                    if let Some(range) = else_statements {
                        range.end = end;
                    }
                    self.scope = scope;
                    returned = parent_returned || (then_returned && has_else && returned);
                    continue;
                }
            };

            charge(&mut self.admission.statements, 1, "body statements")?;
            if let Statement::Expression(marker) = statement {
                if let Some(syntax) = self.matches.get(&marker.span.start).copied() {
                    if marker.span != syntax.span
                        || !matches!(&marker.kind, ExpressionKind::Identifier(name) if name == "match")
                    {
                        return Err(
                            self.error(marker.span, "match source ownership is unavailable")
                        );
                    }
                    let binding = *self.scope.get(syntax.scrutinee.as_str()).ok_or_else(|| {
                        self.error(syntax.scrutinee_span, "unknown match scrutinee")
                    })?;
                    let mut limits = super::GenericEnumPatternLimits::DEFAULT;
                    limits.max_nodes = limits.max_nodes.min(self.admission.field_nodes);
                    limits.max_edges = limits.max_edges.min(self.admission.field_edges);
                    limits.max_work = limits.max_work.min(self.admission.work);
                    limits.payload = self.field_limits;
                    let coverage = super::generic_enum_patterns::check_pattern_headers(
                        &self.bindings[binding.ordinal],
                        self.signature.types(),
                        self.signature.syntax().parameters(),
                        syntax,
                        limits,
                    )
                    .map_err(|error| GenericCopyBodyError::Pattern {
                        function: self.function,
                        error: Box::new(error),
                    })?;
                    charge(
                        &mut self.admission.field_nodes,
                        coverage.nodes,
                        "body field nodes",
                    )?;
                    charge(
                        &mut self.admission.field_edges,
                        coverage.edges,
                        "body field edges",
                    )?;
                    charge(&mut self.admission.work, coverage.work, "body work")?;
                    // The identifier/range are retained directly from the original parsed header.
                    let scrutinee = self.expr(&Expression {
                        kind: ExpressionKind::Identifier(syntax.scrutinee.clone()),
                        span: syntax.scrutinee_span,
                    })?;
                    charge(&mut self.admission.work, self.scope.len(), "body work")?;
                    let index = self.statements.len();
                    self.statements.push(GenericCopyStatement::Match {
                        scrutinee,
                        declaration: coverage.declaration,
                        arms: Vec::new(),
                        span: syntax.span,
                    });
                    pending.push(Work::MatchNext(Box::new(MatchFrame {
                        index,
                        syntax,
                        plans: coverage.arms,
                        scope: self.scope.clone(),
                        parent_returned: returned,
                        all_returned: true,
                        next: 0,
                        scrutinee,
                    })));
                    returned = false;
                    continue;
                }
            }
            match statement {
                Statement::Let {
                    name,
                    is_mutable,
                    type_name,
                    type_span,
                    value,
                    span,
                    ..
                } => {
                    let annotation = if type_name.is_some() {
                        Some(self.local_annotation(type_span.ok_or_else(|| {
                            self.error(*span, "local annotation range is unavailable")
                        })?)?)
                    } else {
                        None
                    };
                    let context = annotation.as_ref().map(|annotation| {
                        GenericCopyType(TypeKind::Expanded(annotation.ty.clone()))
                    });
                    let value = self.expr_expected(value, context)?;
                    let actual = self.expressions[value.ordinal].ty.clone();
                    let (ty, conversion) = if let Some(annotation) = &annotation {
                        let expected = GenericCopyType(TypeKind::Expanded(annotation.ty.clone()));
                        let conversion = if self.admission.same(
                            self.function,
                            &actual,
                            &expected,
                            self.comparison,
                        )? {
                            GenericCopyConversion::Identity
                        } else if actual.primitive() == Some(GenericPrimitive::Int)
                            && expected.primitive() == Some(GenericPrimitive::Float)
                        {
                            GenericCopyConversion::IntToFloat
                        } else {
                            return Err(self.error(
                                annotation.range,
                                "local initializer has an incompatible symbolic type",
                            ));
                        };
                        (expected, conversion)
                    } else {
                        (actual, GenericCopyConversion::Identity)
                    };
                    let token = self
                        .tokens
                        .binary_search_by_key(&span.start, |token| token.span.start)
                        .ok()
                        .and_then(|at| self.tokens.get(at + if *is_mutable { 2 } else { 1 }))
                        .ok_or_else(|| {
                            self.error(*span, "local name token ownership is unavailable")
                        })?;
                    if !matches!(&token.kind,TokenKind::Identifier(written)if written==name) {
                        return Err(self.error(*span, "local name token ownership is unavailable"));
                    }
                    let kind = if *is_mutable {
                        GenericCopyBindingKind::MutableLocal
                    } else {
                        GenericCopyBindingKind::Local
                    };
                    let binding = self.bind(name, token.span, kind, ty)?;
                    self.statements.push(GenericCopyStatement::Let {
                        binding,
                        value,
                        annotation,
                        conversion,
                        span: *span,
                    });
                }
                Statement::Assign {
                    target,
                    value,
                    span,
                } => {
                    if let ExpressionKind::Index {
                        target: base,
                        index,
                    } = &target.kind
                    {
                        let ExpressionKind::Identifier(name) = &base.kind else {
                            return Err(self.error(
                                base.span,
                                "indexed assignment requires a direct mutable array local",
                            ));
                        };
                        self.admission.step()?;
                        let binding = *self.scope.get(name.as_str()).ok_or_else(|| {
                            self.error(base.span, "unknown indexed assignment target")
                        })?;
                        if self.bindings[binding.ordinal].kind
                            != GenericCopyBindingKind::MutableLocal
                        {
                            return Err(self.error(
                                base.span,
                                "indexed assignment target is not a mutable local",
                            ));
                        }
                        self.admission.step()?;
                        let element = self.bindings[binding.ordinal]
                            .ty
                            .expanded()
                            .and_then(ExpandedGenericType::array_element)
                            .ok_or_else(|| {
                                self.error(base.span, "indexed assignment target requires an array")
                            })?;
                        let index = self.expr(index)?;
                        if self.expressions[index.ordinal].ty.primitive()
                            != Some(GenericPrimitive::Int)
                        {
                            return Err(self.error(
                                self.expressions[index.ordinal].span,
                                "array index requires concrete Int",
                            ));
                        }
                        let expected = GenericCopyType(TypeKind::Expanded(element));
                        let value = self.expr_expected(value, Some(expected.clone()))?;
                        let actual = self.expressions[value.ordinal].ty.clone();
                        let conversion = if self.admission.same(
                            self.function,
                            &actual,
                            &expected,
                            self.comparison,
                        )? {
                            GenericCopyConversion::Identity
                        } else if actual.primitive() == Some(GenericPrimitive::Int)
                            && expected.primitive() == Some(GenericPrimitive::Float)
                        {
                            GenericCopyConversion::IntToFloat
                        } else {
                            return Err(self.error(
                                *span,
                                "array replacement has an incompatible symbolic type",
                            ));
                        };
                        self.record_name_use(binding, base.span)?;
                        self.statements.push(GenericCopyStatement::IndexAssign {
                            binding,
                            index,
                            value,
                            conversion,
                            target_span: target.span,
                            span: *span,
                        });
                        continue;
                    }
                    let ExpressionKind::Identifier(name) = &target.kind else {
                        return Err(self.error(
                            target.span,
                            "only direct mutable local assignment is supported",
                        ));
                    };
                    self.admission.step()?;
                    let binding = *self
                        .scope
                        .get(name.as_str())
                        .ok_or_else(|| self.error(target.span, "unknown assignment target"))?;
                    if self.bindings[binding.ordinal].kind != GenericCopyBindingKind::MutableLocal {
                        return Err(
                            self.error(target.span, "assignment target is not a mutable local")
                        );
                    }
                    let expected = self.bindings[binding.ordinal].ty.clone();
                    let value = self.expr_expected(value, Some(expected.clone()))?;
                    let actual = self.expressions[value.ordinal].ty.clone();
                    let conversion = if self.admission.same(
                        self.function,
                        &actual,
                        &expected,
                        self.comparison,
                    )? {
                        GenericCopyConversion::Identity
                    } else if actual.primitive() == Some(GenericPrimitive::Int)
                        && expected.primitive() == Some(GenericPrimitive::Float)
                    {
                        GenericCopyConversion::IntToFloat
                    } else {
                        return Err(
                            self.error(*span, "assignment value has an incompatible symbolic type")
                        );
                    };
                    self.record_name_use(binding, target.span)?;
                    self.statements.push(GenericCopyStatement::Assign {
                        binding,
                        target_span: target.span,
                        value,
                        conversion,
                        span: *span,
                    });
                }
                Statement::While {
                    condition,
                    body,
                    span,
                } => {
                    let condition = self.expr(condition)?;
                    if self.expressions[condition.ordinal].ty.primitive()
                        != Some(GenericPrimitive::Bool)
                    {
                        return Err(self.error(
                            self.expressions[condition.ordinal].span,
                            "while condition requires concrete Bool",
                        ));
                    }
                    charge(&mut self.admission.work, self.scope.len(), "body work")?;
                    let scope = self.scope.clone();
                    let index = self.statements.len();
                    self.statements.push(GenericCopyStatement::While {
                        condition,
                        body_statements: index + 1..index + 1,
                        span: *span,
                    });
                    pending.push(Work::EndWhile {
                        index,
                        scope,
                        parent_returned: returned,
                    });
                    returned = false;
                    pending.extend(body.iter().rev().map(Work::Statement));
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    span,
                } => {
                    let condition = self.expr(condition)?;
                    if self.expressions[condition.ordinal].ty.primitive()
                        != Some(GenericPrimitive::Bool)
                    {
                        return Err(self.error(
                            self.expressions[condition.ordinal].span,
                            "if condition requires concrete Bool",
                        ));
                    }
                    charge(&mut self.admission.work, self.scope.len(), "body work")?;
                    let scope = self.scope.clone();
                    let index = self.statements.len();
                    self.statements.push(GenericCopyStatement::If {
                        condition,
                        then_statements: index + 1..index + 1,
                        else_statements: None,
                        span: *span,
                    });
                    pending.push(Work::EndThen {
                        index,
                        scope,
                        parent_returned: returned,
                        otherwise: else_block.as_deref(),
                    });
                    returned = false;
                    pending.extend(then_block.iter().rev().map(Work::Statement));
                }
                Statement::Expression(expression) => {
                    let value = self.expr(expression)?;
                    self.statements
                        .push(GenericCopyStatement::Expression { value });
                }
                Statement::Return { value, span } => {
                    let expected = self.expected();
                    let value = value
                        .as_ref()
                        .map(|e| self.expr_expected(e, Some(expected.clone())))
                        .transpose()?;
                    let actual = value
                        .map(|id| self.expressions[id.ordinal].ty.clone())
                        .unwrap_or(GenericCopyType(TypeKind::Primitive(GenericPrimitive::Unit)));
                    let conversion = if self.admission.same(
                        self.function,
                        &actual,
                        &expected,
                        self.comparison,
                    )? {
                        GenericCopyConversion::Identity
                    } else if actual.primitive() == Some(GenericPrimitive::Int)
                        && expected.primitive() == Some(GenericPrimitive::Float)
                    {
                        GenericCopyConversion::IntToFloat
                    } else {
                        return Err(
                            self.error(*span, "return value has an incompatible symbolic type")
                        );
                    };
                    self.statements.push(GenericCopyStatement::Return {
                        value,
                        conversion,
                        span: *span,
                    });
                    returned = true;
                }
            }
        }
        if !returned {
            let unit = GenericCopyType(TypeKind::Primitive(GenericPrimitive::Unit));
            if !self
                .admission
                .same(self.function, &unit, &self.expected(), self.comparison)?
            {
                return Err(self.error(closing, "non-Unit function does not return a value"));
            }
        }
        Ok(!returned)
    }
}
/// Check every signature body in the bounded symbolic local/assignment/return
/// profile. Unsupported constructs fail; no body is skipped and no executable IR
/// or generic specialization is produced by this API.
pub fn check_generic_copy_bodies(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, None, 0, false)
}
/// Check complete symbolic bodies with original postfix syntax and exact standard
/// failure compatibility. Concrete specialization/lowering still refuse these bodies.
pub fn check_generic_propagation_bodies(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 0, false)
}
pub(super) fn check_generic_builtin_bodies(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 1, false)
}
pub(super) fn check_generic_string_bodies(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 2, false)
}
pub(super) fn check_generic_value_bodies(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 3, false)
}
pub(super) fn check_generic_array_bodies(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 4, false)
}
pub(super) fn check_generic_package_bodies_inner(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 4, true)
}
pub(super) fn check_generic_collection_package_bodies_inner(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 5, true)
}
pub(super) fn check_generic_data_package_bodies_inner(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: super::GenericPropagationLimits,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_bodies_mode(signatures, limits, Some(propagation), 6, true)
}
fn check_bodies_mode(
    signatures: &CheckedGenericSignatures,
    limits: GenericCopyBodyLimits,
    propagation: Option<super::GenericPropagationLimits>,
    builtins: u8,
    package: bool,
) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    if !package
        && signatures
            .types()
            .environment()
            .registry()
            .is_package_scoped()
    {
        return Err(GenericCopyBodyError::Limit(
            "package body contexts are not linked",
        ));
    }

    if signatures.signatures().len() > limits.max_functions {
        return Err(GenericCopyBodyError::Limit("generic body functions"));
    }
    let mut admission = Admission {
        propagation,
        source: limits.max_source_bytes,
        tokens: limits.max_tokens,
        ast: limits.max_ast_nodes,
        statements: limits.max_statements,
        expressions: limits.max_expressions,
        name_uses: limits
            .max_expressions
            .checked_add(limits.max_statements)
            .ok_or(GenericCopyBodyError::Limit("name use profile"))?,
        bindings: limits.max_bindings,
        text: limits.max_binding_text_bytes,
        pairs: limits.max_type_pairs,
        edges: limits.max_type_edges,
        work: limits.max_work,
        locals: limits.local_types.remaining(),
        records: limits.max_record_constructions,
        enums: limits.max_enum_constructions,
        calls: limits.calls.remaining(),
        field_nodes: limits.max_field_nodes,
        field_edges: limits.max_field_edges,
    };
    let mut bodies = Vec::new();
    for signature in signatures.signatures() {
        admission.step()?;
        let source = signature.syntax().source();
        charge(&mut admission.source, source.len(), "body source bytes")?;
        if admission.tokens == 0 {
            return Err(GenericCopyBodyError::Limit("body tokens"));
        }
        let frontend = FrontendLimits {
            max_source_bytes: source.len(),
            max_tokens: admission.tokens - 1,
            max_diagnostics: limits.max_diagnostics,
        };
        let tokens = if propagation.is_some() {
            Lexer::new().tokenize_propagation(source, frontend)
        } else {
            Lexer::new().tokenize_with_limits(source, frontend)
        }
        .map_err(|diagnostics| GenericCopyBodyError::Parse {
            function: signature.id(),
            diagnostics,
        })?;
        charge(&mut admission.tokens, tokens.len(), "body tokens")?;
        let open = tokens
            .binary_search_by_key(&signature.syntax().body_span().start, |token| {
                token.span.start
            })
            .map_err(|_| GenericCopyBodyError::Semantic {
                function: signature.id(),
                span: signature.syntax().body_span(),
                reason: "body token ownership is unavailable",
            })?;
        let (body, consumed, call_ranges, matches, sites) = if let Some(left) =
            admission.propagation
        {
            let count = tokens
                .iter()
                .filter(|token| token.kind == TokenKind::Punctuation('?'))
                .count();
            if count > left.max_sites {
                return Err(GenericCopyBodyError::Limit("propagation sites"));
            }
            charge(&mut admission.work, tokens.len(), "propagation source work")?;
            charge(
                &mut admission.work,
                count
                    .checked_mul(3)
                    .ok_or(GenericCopyBodyError::Limit("propagation source work"))?,
                "propagation source work",
            )?;
            let parsed = Parser::new()
                .parse_propagation_block_prefix(
                    &tokens[open..],
                    limits.max_diagnostics,
                    left.max_sites,
                    128,
                )
                .map_err(|diagnostics| GenericCopyBodyError::Parse {
                    function: signature.id(),
                    diagnostics,
                })?;
            if let Err(span) = super::generic_propagation_syntax::validate_sites(&tokens, &parsed.4)
            {
                drop(AstGuard(parsed.0));
                return Err(GenericCopyBodyError::Semantic {
                    function: signature.id(),
                    span,
                    reason: "unbound original propagation token",
                });
            }
            parsed
        } else {
            let (body, used, calls, matches) = Parser::new()
                .parse_generic_block_prefix(&tokens[open..], limits.max_diagnostics)
                .map_err(|diagnostics| GenericCopyBodyError::Parse {
                    function: signature.id(),
                    diagnostics,
                })?;
            (body, used, calls, matches, Vec::new())
        };
        let guard = AstGuard(body);
        charge(
            &mut admission.ast,
            sites.len(),
            "propagation virtual AST nodes",
        )?;
        let closing = tokens
            .get(open + consumed - 1)
            .expect("parsed block closing token")
            .span;
        if closing.end != signature.syntax().body_span().end
            || tokens.get(open + consumed).map(|t| &t.kind) != Some(&TokenKind::Eof)
        {
            return Err(GenericCopyBodyError::Semantic {
                function: signature.id(),
                span: closing,
                reason: "parsed body ownership differs from original source",
            });
        }
        admit_ast(&guard.0, &mut admission)?;
        for syntax in &matches.0 {
            // The private marker accounts for the header read; all actual arm ASTs
            // are admitted once from this flat overlay, including nested matches.
            for arm in &syntax.arms {
                admit_ast(&arm.body, &mut admission)?;
            }
        }
        let mut checker = Checker {
            builtins,
            expression_depths: propagation.map(|_| Vec::new()),
            propagation_sites: sites
                .into_iter()
                .map(|site| ((site.expression.start, site.expression.end), site))
                .collect(),
            signatures,
            call_ranges: call_ranges.into_iter().collect(),
            matches: matches
                .0
                .iter()
                .map(|syntax| (syntax.span.start, syntax))
                .collect(),
            call_limits: limits.calls,
            function: signature.id(),
            arena: allocate_body_arena(&NEXT_BODY_ARENA)?,
            signature,
            tokens: &tokens,
            admission: &mut admission,
            comparison: limits.comparison,
            scope: BTreeMap::new(),
            bindings: Vec::new(),
            name_uses: Vec::new(),
            expressions: Vec::new(),
            statements: Vec::new(),
            local_limits: limits.local_types,
            field_limits: limits.field_expansion,
        };
        let implicit_unit_return = checker.check(&guard.0, closing)?;
        if let Some((_, site)) = checker.propagation_sites.first_key_value() {
            return Err(checker.error(
                site.question,
                "propagation site was not bound to a value expression",
            ));
        }
        bodies.push(CheckedGenericCopyBody {
            signatures: signatures.clone(),
            function: signature.id(),
            arena: checker.arena,
            bindings: checker.bindings.into(),
            name_uses: checker.name_uses.into(),
            expressions: checker.expressions.into(),
            statements: checker.statements.into(),
            implicit_unit_return,
        });
    }
    Ok(CheckedGenericCopyBodies {
        propagation: propagation.map(|initial| {
            let left = admission.propagation.expect("selected propagation profile");
            super::GenericPropagationUsage {
                sites: initial.max_sites - left.max_sites,
                contexts: initial.max_contexts - left.max_contexts,
                pairs: initial.max_pairs - left.max_pairs,
                edges: initial.max_edges - left.max_edges,
                work: initial.max_work - left.max_work,
            }
        }),
        signatures: signatures.clone(),
        bodies: bodies.into(),
        source_bytes: limits.max_source_bytes - admission.source,
        tokens: limits.max_tokens - admission.tokens,
        ast_nodes: limits.max_ast_nodes - admission.ast,
        statements: limits.max_statements - admission.statements,
        expressions: limits.max_expressions - admission.expressions,
        bindings: limits.max_bindings - admission.bindings,
        binding_text: limits.max_binding_text_bytes - admission.text,
        type_pairs: limits.max_type_pairs - admission.pairs,
        type_edges: limits.max_type_edges - admission.edges,
        work: limits.max_work - admission.work,
        field_nodes: limits.max_field_nodes - admission.field_nodes,
        field_edges: limits.max_field_edges - admission.field_edges,
        calls: {
            let initial = limits.calls.remaining();
            GenericBodyCallUsage {
                calls: initial.calls - admission.calls.calls,
                values: initial.values - admission.calls.values,
                type_arguments: initial.type_arguments - admission.calls.type_arguments,
                source_bytes: initial.source_bytes - admission.calls.source_bytes,
                tokens: initial.tokens - admission.calls.tokens,
                input_nodes: initial.input_nodes - admission.calls.input_nodes,
                input_edges: initial.input_edges - admission.calls.input_edges,
                name_bytes: initial.name_bytes - admission.calls.name_bytes,
                nodes: initial.nodes - admission.calls.nodes,
                edges: initial.edges - admission.calls.edges,
            }
        },
        records: limits.max_record_constructions - admission.records,
        enums: limits.max_enum_constructions - admission.enums,
        local_annotations: GenericLocalAnnotationUsage {
            annotations: limits.local_types.max_annotations - admission.locals.annotations,
            source_bytes: limits.local_types.max_source_bytes - admission.locals.source_bytes,
            tokens: limits.local_types.max_tokens - admission.locals.tokens,
            nodes: limits.local_types.max_nodes - admission.locals.nodes,
            edges: limits.local_types.max_edges - admission.locals.edges,
            name_bytes: limits.local_types.max_name_bytes - admission.locals.name_bytes,
            expanded_nodes: limits.local_types.max_expanded_nodes - admission.locals.expanded_nodes,
            expanded_edges: limits.local_types.max_expanded_edges - admission.locals.expanded_edges,
        },
    })
}

// Fault injection stays test-only; public checked bodies remain immutable.
#[cfg(test)]
pub(super) fn corrupt_propagation_for_test(bodies: &mut CheckedGenericCopyBodies, operand: bool) {
    let donor = bodies
        .body(bodies.signatures.find("donor").unwrap())
        .unwrap()
        .expressions()
        .iter()
        .find_map(|e| {
            if let GenericCopyExpressionKind::Propagate { types, .. } = &e.kind {
                Some(types.clone())
            } else {
                None
            }
        })
        .unwrap();
    let target = bodies.signatures.find("test").unwrap().ordinal();
    let mut all = bodies.bodies.to_vec();
    let mut expressions = all[target].expressions.to_vec();
    let at = expressions
        .iter()
        .position(|e| matches!(e.kind, GenericCopyExpressionKind::Propagate { .. }))
        .unwrap();
    let own = expressions[at].id;
    if let GenericCopyExpressionKind::Propagate {
        operand: input,
        types,
        ..
    } = &mut expressions[at].kind
    {
        if operand {
            *input = own;
        } else {
            *types = donor;
        }
    }
    all[target].expressions = expressions.into();
    bodies.bodies = all.into();
}

#[cfg(test)]
mod arena_tests {
    use super::*;
    #[test]
    fn arena_identity_exhaustion_never_wraps_or_reuses_an_owner() {
        let counter = AtomicU64::new(u64::MAX - 1);
        assert_eq!(allocate_body_arena(&counter).unwrap(), u64::MAX - 1);
        assert_eq!(
            allocate_body_arena(&counter).unwrap_err(),
            GenericCopyBodyError::Limit("body arena identities")
        );
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert!(allocate_body_arena(&counter).is_err());
    }
}
