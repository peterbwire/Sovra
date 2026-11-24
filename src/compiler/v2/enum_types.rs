use crate::compiler::{
    ast::{self, Program, StructDeclaration, StructField, TypeDeclaration},
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    resources,
    semantic::{self, SemanticAnalyzer, Type},
    sum_syntax::{parse_enum_declaration, EnumSyntaxError, EnumSyntaxLimits},
};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

/// A separately labelled source fragment containing exactly one enum declaration.
#[derive(Debug, Clone, Copy)]
pub struct EnumSource<'a> {
    /// Opaque provenance label, unique within the request; not filesystem authority.
    pub label: &'a str,
    /// Original declaration source; spans are relative to this fragment.
    pub source: &'a str,
}
/// A declaration attached explicitly to an existing inline module or root scope.
#[derive(Debug, Clone, Copy)]
pub struct ScopedEnumSource<'a> {
    /// None is root; Some must identify an existing context module exactly.
    pub module: Option<&'a str>,
    /// Original single-declaration fragment and its unique label.
    pub declaration: EnumSource<'a>,
}
/// Independent finite quotas for an enum type-environment request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumTypeLimits {
    /// Declaration fragments, admitted before parsing.
    pub max_enums: usize,
    /// Aggregate declaration UTF-8 bytes; independent per-fragment syntax cap remains.
    pub max_source_bytes: usize,
    /// Borrowed legacy context AST before any type-header copying.
    pub max_context_ast_nodes: usize,
    /// Retained/copied type headers, fields, enum/variant records and payload nodes.
    pub max_metadata_nodes: usize,
    /// Retained/copied labels, names and annotation text, including probes and facts.
    pub max_text_bytes: usize,
    /// The previously approved per-fragment syntax policy.
    pub syntax: EnumSyntaxLimits,
}
impl EnumTypeLimits {
    /// Opt-in local enum-check defaults; no existing command profile is changed.
    pub const DEFAULT: Self = Self {
        max_enums: 1024,
        max_source_bytes: 16 * 1024 * 1024,
        max_context_ast_nodes: resources::DEFAULT_AST_NODES,
        max_metadata_nodes: 65_536,
        max_text_bytes: 16 * 1024 * 1024,
        syntax: EnumSyntaxLimits::DEFAULT,
    };
}
/// Type-checker admission dimension, separate from syntax quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumTypeLimit {
    /// Number of enum source fragments.
    Enums,
    /// Aggregate original declaration source bytes.
    SourceBytes,
    /// Borrowed context AST nodes.
    ContextAstNodes,
    /// Cumulative metadata construction nodes.
    MetadataNodes,
    /// Cumulative owned metadata text bytes.
    TextBytes,
}
/// A semantic diagnostic tied to its own source fragment/context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumTypeDiagnostic {
    /// Provenance supplied by the trusted caller, not a validated file path.
    pub source_label: Arc<str>,
    /// Original source span and a bounded E3xxx diagnostic.
    pub diagnostic: Diagnostic,
}
/// Complete failure; no checked environment or partial match plan is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumTypeError {
    /// A refused resource dimension.
    Limit {
        /// Exhausted dimension.
        dimension: EnumTypeLimit,
        /// Selected cap.
        limit: usize,
    },
    /// The approved declaration parser rejected one source fragment.
    Syntax {
        /// Source fragment label.
        source_label: Arc<str>,
        /// Exact existing syntax/refusal outcome.
        error: EnumSyntaxError,
    },
    /// Source labels must be unique, so relative spans are unambiguous.
    DuplicateSourceLabel(Arc<str>),
    /// The requested inline module is absent from the admitted type context.
    UnavailableModule(Arc<str>),
    /// Bounded source-owned semantic failures.
    Diagnostics(Vec<EnumTypeDiagnostic>),
    /// An identity belongs to a different checked environment.
    UnavailableIdentity,
    /// An internal diagnostic did not map to admitted original input metadata.
    UnavailableDiagnosticOrigin,
    /// Environment identity allocation cannot wrap and reuse an old identity.
    IdentityExhausted,
}
impl std::fmt::Display for EnumTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Limit { dimension, limit } => {
                write!(f, "enum type {dimension:?} limit of {limit} exceeded")
            }
            Self::Syntax {
                source_label,
                error,
            } => write!(f, "{source_label}: {error}"),
            Self::UnavailableIdentity => {
                write!(f, "enum identity belongs to a different environment")
            }
            Self::UnavailableDiagnosticOrigin => write!(f, "enum diagnostic origin is unavailable"),
            Self::IdentityExhausted => write!(f, "enum environment identities exhausted"),
            Self::UnavailableModule(name) => write!(f, "enum module is unavailable: {name}"),
            Self::DuplicateSourceLabel(label) => write!(f, "duplicate enum source label: {label}"),
            Self::Diagnostics(errors) => write!(
                f,
                "enum type checking rejected: {}",
                errors
                    .first()
                    .map_or("invalid declarations", |e| e.diagnostic.message.as_str())
            ),
        }
    }
}
impl std::error::Error for EnumTypeError {}

/// An enum identity scoped to one immutable checked environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnumId {
    environment: u64,
    ordinal: usize,
}
impl EnumId {
    /// Position in this checked environment, not a global symbol ID.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Sealed record identity scoped to one immutable checked environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeRecordId {
    environment: u64,
    ordinal: usize,
}
/// A flat payload-type node scoped to one immutable checked environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumTypeId {
    environment: u64,
    ordinal: usize,
}
impl EnumTypeId {
    /// Position in the environment's payload type arena.
    pub const fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Resolved payload facts; enum and record identity are deliberately distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumPayloadType {
    /// Unit payload, distinct from a nullary variant.
    Unit,
    /// Boolean payload.
    Bool,
    /// Signed 64-bit integer payload.
    Int,
    /// Float payload.
    Float,
    /// Valid UTF-8 String payload.
    String,
    /// Existing record identity in the caller's type context.
    Record(String),
    /// Nominal enum reference; cycles do not structurally unfold.
    Enum(EnumId),
    /// Typed array edge into the same flat arena.
    Array(EnumTypeId),
}
/// Checked variant header; no constructor executable is implied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantFact {
    /// Declared variant name.
    pub name: String,
    /// Original variant/name span.
    pub span: Span,
    /// None is nullary; Some(Unit) still requires a payload.
    pub payload: Option<EnumTypeId>,
}
/// One nominal enum in a checked local environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDeclarationFact {
    /// Environment-local identity, not a record identity.
    pub id: EnumId,
    /// Declared name; aliases resolve to this identity.
    pub name: String,
    /// Export intent; package/interface visibility remains a later integration gate.
    pub is_exported: bool,
    /// Source fragment label.
    pub source_label: Arc<str>,
    /// Original declaration/name span.
    pub span: Span,
    /// Ordered checked variants.
    pub variants: Vec<EnumVariantFact>,
}
#[derive(Debug)]
struct Facts {
    environment: u64,
    limits: EnumTypeLimits,
    owner: Arc<str>,
    declarations: Vec<EnumDeclarationFact>,
    enum_names: HashMap<String, EnumId>,
    types: Vec<EnumPayloadType>,
    known: HashMap<String, Type>,
    record_fields: HashMap<String, HashMap<String, Type>>,
}
/// Immutable local enum-type evidence. It cannot be constructed from unchecked facts.
#[derive(Debug, Clone)]
pub struct CheckedEnumTypes(Arc<Facts>, Option<Arc<ModuleTypes>>);
#[derive(Debug)]
struct ModuleTypes {
    name: Arc<str>,
    known: HashMap<String, Type>,
}
impl CheckedEnumTypes {
    /// Resolve a canonical checked record name; caller text alone cannot forge an ID.
    pub fn find_record(&self, name: &str) -> Option<RuntimeRecordId> {
        // HashMap iteration order stays fixed for the lifetime of these immutable facts.
        // IDs are environment-local and never serialized or shared across fresh checks.
        self.0
            .record_fields
            .keys()
            .position(|n| n == name)
            .map(|ordinal| RuntimeRecordId {
                environment: self.0.environment,
                ordinal,
            })
    }
    pub(super) fn record_schema(
        &self,
        id: RuntimeRecordId,
    ) -> Option<(&str, &HashMap<String, Type>)> {
        if id.environment != self.0.environment {
            return None;
        }
        self.0
            .record_fields
            .iter()
            .nth(id.ordinal)
            .map(|(name, fields)| (name.as_str(), fields))
    }
    pub(super) fn body_named_types(&self) -> &HashMap<String, Type> {
        self.1.as_ref().map_or(&self.0.known, |scope| &scope.known)
    }
    /// Inline module owning this checked function context; not package authority.
    pub fn module(&self) -> Option<&str> {
        self.1.as_ref().map(|scope| scope.name.as_ref())
    }
    pub(super) fn scoped(
        &self,
        module: &str,
        nodes: &mut usize,
        text: &mut usize,
        span: Span,
    ) -> Result<Self, super::ProgramSourceError> {
        fn charge(left: &mut usize, amount: usize) -> Result<(), super::ProgramSourceError> {
            *left = left
                .checked_sub(amount)
                .ok_or(super::ProgramSourceError::Limit("module scope metadata"))?;
            Ok(())
        }
        let mut known = HashMap::new();
        charge(nodes, 1)?;
        charge(
            text,
            module
                .len()
                .checked_mul(3)
                .and_then(|n| n.checked_add(2))
                .ok_or(super::ProgramSourceError::Limit("module scope metadata"))?,
        )?;
        let prefix = format!("{module}::");
        for (name, ty) in &self.0.known {
            charge(nodes, 1)?;
            charge(text, name.len())?;
            let mut leaf = ty;
            loop {
                charge(nodes, 1)?;
                match leaf {
                    Type::Array(inner) => leaf = inner,
                    Type::Named(name) => {
                        charge(text, name.len())?;
                        break;
                    }
                    _ => break,
                }
            }
            known.insert(name.clone(), ty.clone());
        }
        for (name, ty) in &self.0.known {
            if let Some(short) = name
                .strip_prefix(&prefix)
                .filter(|name| !name.contains("::"))
            {
                if self.0.known.get(short).is_some_and(|global| global != ty) {
                    return Err(super::ProgramSourceError::Source(
                        super::EntrySourceError::Unsupported {
                            kind: "module function scope with a shadowed root type",
                            span,
                        },
                    ));
                }
                charge(nodes, 1)?;
                charge(text, short.len())?;
                let mut leaf = ty;
                loop {
                    charge(nodes, 1)?;
                    match leaf {
                        Type::Array(inner) => leaf = inner,
                        Type::Named(name) => {
                            charge(text, name.len())?;
                            break;
                        }
                        _ => break,
                    }
                }
                known.insert(short.to_owned(), ty.clone());
            }
        }
        Ok(Self(
            Arc::clone(&self.0),
            Some(Arc::new(ModuleTypes {
                name: Arc::from(module),
                known,
            })),
        ))
    }
    pub(super) fn body_record_fields(&self) -> &HashMap<String, HashMap<String, Type>> {
        &self.0.record_fields
    }
    pub(super) fn body_payload_type(&self, mut id: EnumTypeId) -> Result<Type, EnumTypeError> {
        let mut depth = 0;
        let leaf = loop {
            match self
                .payload_type(id)
                .ok_or(EnumTypeError::UnavailableIdentity)?
            {
                EnumPayloadType::Array(element) => {
                    depth += 1;
                    id = *element;
                }
                leaf => break leaf,
            }
        };
        let mut ty = match leaf {
            EnumPayloadType::Unit => Type::Unit,
            EnumPayloadType::Bool => Type::Bool,
            EnumPayloadType::Int => Type::Int,
            EnumPayloadType::Float => Type::Float,
            EnumPayloadType::String => Type::String,
            EnumPayloadType::Record(name) => Type::Named(name.clone()),
            EnumPayloadType::Enum(id) => {
                Type::Named(self.declarations()[id.ordinal()].name.clone())
            }
            EnumPayloadType::Array(_) => unreachable!(),
        };
        for _ in 0..depth {
            ty = Type::Array(Box::new(ty));
        }
        Ok(ty)
    }
    /// Trusted caller's owner label; not a package authorization token.
    pub fn owner(&self) -> &str {
        &self.0.owner
    }
    /// Checked declarations, with environment-local identities.
    pub fn declarations(&self) -> &[EnumDeclarationFact] {
        &self.0.declarations
    }
    /// Flat payload-type graph, including explicit nominal enum edges.
    pub fn types(&self) -> &[EnumPayloadType] {
        &self.0.types
    }
    /// Read a payload node only if its identity belongs to this environment.
    pub fn payload_type(&self, id: EnumTypeId) -> Option<&EnumPayloadType> {
        if id.environment == self.0.environment {
            self.0.types.get(id.ordinal)
        } else {
            None
        }
    }
    /// Resolve a declaration in this environment, without global/package lookup.
    pub fn find(&self, name: &str) -> Option<EnumId> {
        self.0
            .declarations
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.id)
    }

    /// Resolve a canonical enum name or validated transparent alias in this context.
    /// Array/record/scalar aliases are not enum constructor namespaces. This does
    /// not establish module/package visibility or introduce another nominal owner.
    pub fn resolve_enum(&self, name: &str) -> Option<EnumId> {
        match self.body_named_types().get(name)? {
            Type::Named(canonical) => self.0.enum_names.get(canonical).copied(),
            _ => None,
        }
    }
    pub(super) fn enum_aliases(&self) -> impl Iterator<Item = (&str, EnumId)> {
        self.body_named_types()
            .iter()
            .filter_map(|(name, ty)| match ty {
                Type::Named(canonical) if name != canonical => self
                    .0
                    .enum_names
                    .get(canonical)
                    .map(|id| (name.as_str(), *id)),
                _ => None,
            })
    }

    /// Check a constructor's closed argument type metadata against its nominal payload.
    /// This plans type/coercion behavior only, not expression bodies or execution.
    pub fn check_constructor(
        &self,
        enum_id: EnumId,
        source_label: &str,
        span: Span,
        variant_name: &str,
        arguments: &[EnumConstructorArgument<'_>],
    ) -> Result<EnumConstructorPlan, EnumTypeError> {
        if enum_id.environment != self.0.environment {
            return Err(EnumTypeError::UnavailableIdentity);
        }
        let Some(declaration) = self.0.declarations.get(enum_id.ordinal) else {
            return Err(EnumTypeError::UnavailableIdentity);
        };
        if source_label.len() > self.0.limits.max_text_bytes {
            return Err(failure(
                EnumTypeLimit::TextBytes,
                self.0.limits.max_text_bytes,
            ));
        }
        let label: Arc<str> = Arc::from(source_label);
        let Some(variant_index) = declaration
            .variants
            .iter()
            .position(|variant| variant.name == variant_name)
        else {
            return Err(semantic_error(
                label,
                "E3105",
                "unknown enum constructor variant",
                span,
            ));
        };
        let expected = declaration.variants[variant_index].payload;
        if arguments.len() != usize::from(expected.is_some()) {
            return Err(semantic_error(
                label,
                "E3105",
                "enum constructor payload arity mismatch",
                span,
            ));
        }
        let coercion = if let Some(expected) = expected {
            let argument = &arguments[0];
            if argument.type_name.len() > self.0.limits.max_text_bytes {
                return Err(failure(
                    EnumTypeLimit::TextBytes,
                    self.0.limits.max_text_bytes,
                ));
            }
            let actual =
                Type::from_name_with_known(argument.type_name, Some(self.body_named_types()))
                    .canonicalize(self.body_named_types());
            let mut expected = self
                .payload_type(expected)
                .ok_or(EnumTypeError::UnavailableIdentity)?;
            let mut actual = &actual;
            let mut depth = 0;
            while let (EnumPayloadType::Array(element), Type::Array(actual_element)) =
                (expected, actual)
            {
                depth += 1;
                expected = self
                    .payload_type(*element)
                    .ok_or(EnumTypeError::UnavailableIdentity)?;
                actual = actual_element;
            }
            let compatible = match (expected, actual) {
                (EnumPayloadType::Unit, Type::Unit)
                | (EnumPayloadType::Bool, Type::Bool)
                | (EnumPayloadType::Int, Type::Int)
                | (EnumPayloadType::Float, Type::Float)
                | (EnumPayloadType::String, Type::String) => true,
                (EnumPayloadType::Record(name), Type::Named(actual)) => name == actual,
                (EnumPayloadType::Enum(id), Type::Named(actual)) => self.find(actual) == Some(*id),
                _ => false,
            };
            if matches!((expected, actual), (EnumPayloadType::Float, Type::Int)) {
                Some(EnumPayloadCoercion::IntToFloat { array_depth: depth })
            } else if compatible {
                Some(EnumPayloadCoercion::Identity)
            } else {
                return Err(semantic_error(
                    label,
                    "E3106",
                    "enum constructor payload type mismatch or unresolved argument type",
                    argument.span,
                ));
            }
        } else {
            None
        };
        Ok(EnumConstructorPlan {
            environment: self.clone(),
            enum_id,
            variant_index,
            expected,
            coercion,
        })
    }

    /// Validate exhaustive variant coverage and payload binding arity only.
    /// This does not parse/check arm bodies, immutability, return flow or execution.
    pub fn check_match(
        &self,
        enum_id: EnumId,
        source_label: &str,
        span: Span,
        arms: &[EnumMatchArm<'_>],
    ) -> Result<EnumMatchPlan, EnumTypeError> {
        if enum_id.environment != self.0.environment {
            return Err(EnumTypeError::UnavailableIdentity);
        }
        let Some(declaration) = self.0.declarations.get(enum_id.ordinal) else {
            return Err(EnumTypeError::UnavailableIdentity);
        };
        if source_label.len() > self.0.limits.max_text_bytes {
            return Err(failure(
                EnumTypeLimit::TextBytes,
                self.0.limits.max_text_bytes,
            ));
        }
        let source_label: Arc<str> = Arc::from(source_label);
        if arms.len() > declaration.variants.len() {
            return Err(semantic_error(
                Arc::clone(&source_label),
                "E3103",
                "match has more arms than enum variants",
                span,
            ));
        }
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        for arm in arms {
            if self.resolve_enum(arm.enum_name) != Some(enum_id) {
                return Err(semantic_error(
                    Arc::clone(&source_label),
                    "E3103",
                    "match arm names a different enum",
                    arm.span,
                ));
            }
            let Some(index) = declaration
                .variants
                .iter()
                .position(|v| v.name == arm.variant_name)
            else {
                return Err(semantic_error(
                    Arc::clone(&source_label),
                    "E3103",
                    "unknown match variant",
                    arm.span,
                ));
            };
            if !seen.insert(index) {
                return Err(semantic_error(
                    Arc::clone(&source_label),
                    "E3103",
                    "duplicate match variant",
                    arm.span,
                ));
            }
            if arm.has_payload_binding != declaration.variants[index].payload.is_some() {
                return Err(semantic_error(
                    Arc::clone(&source_label),
                    "E3103",
                    "match payload binding arity does not match variant",
                    arm.span,
                ));
            }
            order.push(index);
        }
        if seen.len() != declaration.variants.len() {
            return Err(semantic_error(
                Arc::clone(&source_label),
                "E3103",
                "match is not exhaustive",
                span,
            ));
        }
        Ok(EnumMatchPlan {
            environment: self.clone(),
            enum_id,
            variant_order: order,
        })
    }
}
/// Closed caller-supplied argument type metadata, not a checked source expression.
#[derive(Debug, Clone, Copy)]
pub struct EnumConstructorArgument<'a> {
    /// Existing known annotation spelling; no inference from arbitrary values occurs.
    pub type_name: &'a str,
    /// Original argument span in the caller's input.
    pub span: Span,
}
/// Existing numeric conversion needed for one constructor payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumPayloadCoercion {
    /// Same primitive/nominal type, including array shape.
    Identity,
    /// Int-to-Float, through this many array wrappers (zero means scalar).
    IntToFloat {
        /// Number of enclosing array wrappers.
        array_depth: usize,
    },
}
/// Nominal constructor type/coercion plan; not an executable enum value.
#[derive(Debug, Clone)]
pub struct EnumConstructorPlan {
    environment: CheckedEnumTypes,
    enum_id: EnumId,
    variant_index: usize,
    expected: Option<EnumTypeId>,
    coercion: Option<EnumPayloadCoercion>,
}
impl EnumConstructorPlan {
    /// Enum identity in the retained environment.
    pub fn enum_id(&self) -> EnumId {
        self.enum_id
    }
    /// Ordered declaration variant selected by this constructor.
    pub fn variant_index(&self) -> usize {
        self.variant_index
    }
    /// None is nullary, not a Unit payload.
    pub fn expected_payload(&self) -> Option<EnumTypeId> {
        self.expected
    }
    /// Exact numeric conversion, if a payload exists.
    pub fn payload_coercion(&self) -> Option<EnumPayloadCoercion> {
        self.coercion
    }
    /// Environment owning this plan's nominal identities.
    pub fn environment(&self) -> &CheckedEnumTypes {
        &self.environment
    }
}

/// Caller-supplied arm shape; spans remain caller-owned input, not parsed statements.
#[derive(Debug, Clone, Copy)]
pub struct EnumMatchArm<'a> {
    /// Nominal enum name resolved by the caller to this local environment.
    pub enum_name: &'a str,
    /// Variant spelling.
    pub variant_name: &'a str,
    /// Whether one payload binding is supplied.
    pub has_payload_binding: bool,
    /// Original pattern span, relative to its caller-owned input.
    pub span: Span,
}
/// Exhaustive arm-shape evidence tied to its immutable environment.
#[derive(Debug, Clone)]
pub struct EnumMatchPlan {
    environment: CheckedEnumTypes,
    enum_id: EnumId,
    variant_order: Vec<usize>,
}
impl EnumMatchPlan {
    /// Enum identity in the retained environment.
    pub fn enum_id(&self) -> EnumId {
        self.enum_id
    }
    /// Declaration-variant positions, in arm order.
    pub fn variant_order(&self) -> &[usize] {
        &self.variant_order
    }
    /// Environment retaining the payload types used by this plan.
    pub fn environment(&self) -> &CheckedEnumTypes {
        &self.environment
    }
}

pub(super) fn allocate_identity(counter: &AtomicU64) -> Result<u64, EnumTypeError> {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .ok_or(EnumTypeError::IdentityExhausted)?;
        match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(value) => return Ok(value),
            Err(value) => current = value,
        }
    }
}

fn failure(dimension: EnumTypeLimit, limit: usize) -> EnumTypeError {
    EnumTypeError::Limit { dimension, limit }
}
fn semantic_error(
    label: Arc<str>,
    code: &'static str,
    message: &'static str,
    span: Span,
) -> EnumTypeError {
    EnumTypeError::Diagnostics(vec![EnumTypeDiagnostic {
        source_label: label,
        diagnostic: Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
            source_file: None,
        },
    }])
}
struct Budget {
    limits: EnumTypeLimits,
    nodes: usize,
    text: usize,
}
impl Budget {
    fn node(&mut self, count: usize) -> Result<(), EnumTypeError> {
        self.nodes = self
            .nodes
            .checked_add(count)
            .filter(|n| *n <= self.limits.max_metadata_nodes)
            .ok_or_else(|| failure(EnumTypeLimit::MetadataNodes, self.limits.max_metadata_nodes))?;
        Ok(())
    }
    fn text(&mut self, count: usize) -> Result<(), EnumTypeError> {
        self.text = self
            .text
            .checked_add(count)
            .filter(|n| *n <= self.limits.max_text_bytes)
            .ok_or_else(|| failure(EnumTypeLimit::TextBytes, self.limits.max_text_bytes))?;
        Ok(())
    }
    fn owned(&mut self, text: &str) -> Result<String, EnumTypeError> {
        self.text(text.len())?;
        Ok(text.into())
    }
    fn label(&mut self, text: &str) -> Result<Arc<str>, EnumTypeError> {
        self.text(text.len())?;
        Ok(Arc::from(text))
    }
}
struct Origins(Vec<(Arc<str>, Span)>);
impl Origins {
    fn add(&mut self, label: &Arc<str>, span: Span) -> Span {
        let start = self.0.len();
        self.0.push((Arc::clone(label), span));
        Span {
            start,
            end: start + 1,
            line: 0,
            column: start,
        }
    }
    fn errors(&self, errors: Diagnostics) -> EnumTypeError {
        let mut result = Vec::new();
        for mut diagnostic in errors.items {
            let Some((label, span)) = self.0.get(diagnostic.span.start) else {
                return EnumTypeError::UnavailableDiagnosticOrigin;
            };
            diagnostic.span = *span;
            result.push(EnumTypeDiagnostic {
                source_label: Arc::clone(label),
                diagnostic,
            });
        }
        EnumTypeError::Diagnostics(result)
    }
}
fn alias(
    input: &TypeDeclaration,
    label: &Arc<str>,
    budget: &mut Budget,
    origins: &mut Origins,
) -> Result<TypeDeclaration, EnumTypeError> {
    budget.node(1)?;
    Ok(TypeDeclaration {
        name: budget.owned(&input.name)?,
        target: budget.owned(&input.target)?,
        span: origins.add(label, input.span),
    })
}
fn record(
    input: &StructDeclaration,
    label: &Arc<str>,
    budget: &mut Budget,
    origins: &mut Origins,
) -> Result<StructDeclaration, EnumTypeError> {
    budget.node(1)?;
    let name = budget.owned(&input.name)?;
    let span = origins.add(label, input.span);
    let mut fields = Vec::new();
    for field in &input.fields {
        budget.node(1)?;
        fields.push(StructField {
            name: budget.owned(&field.name)?,
            type_name: budget.owned(&field.type_name)?,
            span: origins.add(label, field.span),
            type_span: Some(origins.add(label, field.type_span.unwrap_or(field.span))),
        });
    }
    Ok(StructDeclaration {
        is_exported: input.is_exported,
        name,
        fields,
        span,
    })
}

/// Resolve local enum payload types against legacy type headers, without changing
/// or validating context function bodies. Caller-supplied labels are provenance,
/// not package visibility/access evidence. No executable old/new program is returned.
pub fn check_enum_types(
    owner: &str,
    context_label: &str,
    context: &Program,
    sources: &[EnumSource<'_>],
    limits: EnumTypeLimits,
) -> Result<CheckedEnumTypes, EnumTypeError> {
    if sources.len() > limits.max_enums {
        return Err(failure(EnumTypeLimit::Enums, limits.max_enums));
    }
    let sources: Vec<_> = sources
        .iter()
        .map(|source| ScopedEnumSource {
            module: None,
            declaration: *source,
        })
        .collect();
    check_scoped_enum_types(owner, context_label, context, &sources, limits)
}

/// Check root/module enum payload declarations against admitted type headers.
/// Module visibility uses the existing record/interface rules, but no package
/// graph, function body, source constructor or runtime execution is established.
pub fn check_scoped_enum_types(
    owner: &str,
    context_label: &str,
    context: &Program,
    sources: &[ScopedEnumSource<'_>],
    limits: EnumTypeLimits,
) -> Result<CheckedEnumTypes, EnumTypeError> {
    check_scoped_enum_types_parsed(owner, context_label, context, sources, limits, None)
}
// Only the bounded source-unit parser supplies pre-parsed syntax with original spans.
pub(super) fn check_scoped_enum_types_parsed(
    owner: &str,
    context_label: &str,
    context: &Program,
    sources: &[ScopedEnumSource<'_>],
    limits: EnumTypeLimits,
    parsed: Option<Vec<crate::compiler::sum_syntax::EnumDeclarationSyntax>>,
) -> Result<CheckedEnumTypes, EnumTypeError> {
    if parsed.as_ref().is_some_and(|v| v.len() != sources.len()) {
        return Err(failure(EnumTypeLimit::Enums, limits.max_enums));
    }
    let mut supplied = parsed.map(Vec::into_iter);
    if sources.len() > limits.max_enums {
        return Err(failure(EnumTypeLimit::Enums, limits.max_enums));
    }
    resources::check_ast_budget(context, limits.max_context_ast_nodes)
        .map_err(|_| failure(EnumTypeLimit::ContextAstNodes, limits.max_context_ast_nodes))?;
    let mut bytes = 0usize;
    for source in sources {
        bytes = bytes
            .checked_add(source.declaration.source.len())
            .filter(|n| *n <= limits.max_source_bytes)
            .ok_or_else(|| failure(EnumTypeLimit::SourceBytes, limits.max_source_bytes))?;
    }
    let mut budget = Budget {
        limits,
        nodes: 0,
        text: 0,
    };
    let owner = budget.label(owner)?;
    let context_label = budget.label(context_label)?;
    let mut parsed = Vec::new();
    let mut labels = BTreeSet::new();
    let mut names = BTreeSet::new();
    for source in sources {
        let label = budget.label(source.declaration.label)?;
        if !labels.insert(Arc::clone(&label)) {
            return Err(EnumTypeError::DuplicateSourceLabel(label));
        }
        let syntax = if let Some(supplied) = &mut supplied {
            supplied
                .next()
                .ok_or_else(|| failure(EnumTypeLimit::Enums, limits.max_enums))?
        } else {
            parse_enum_declaration(source.declaration.source, limits.syntax).map_err(|error| {
                EnumTypeError::Syntax {
                    source_label: Arc::clone(&label),
                    error,
                }
            })?
        };
        budget.node(1)?;
        budget.text(syntax.name.len())?;
        let module = if let Some(name) = source.module {
            let Some(index) = context
                .modules
                .iter()
                .position(|module| module.name == name)
            else {
                return Err(EnumTypeError::UnavailableModule(budget.label(name)?));
            };
            Some(index)
        } else {
            None
        };
        let canonical = if let Some(index) = module {
            let name = &context.modules[index].name;
            let bytes = name
                .len()
                .checked_add(2)
                .and_then(|bytes| bytes.checked_add(syntax.name.len()))
                .ok_or_else(|| failure(EnumTypeLimit::TextBytes, limits.max_text_bytes))?;
            budget.text(bytes)?;
            format!("{name}::{}", syntax.name)
        } else {
            budget.owned(&syntax.name)?
        };
        if !names.insert(canonical.clone()) {
            return Err(semantic_error(
                label,
                "E3100",
                "duplicate enum name",
                syntax.name_span,
            ));
        }
        if syntax.variants.is_empty() {
            return Err(semantic_error(
                label,
                "E3101",
                "executable enum requires at least one variant",
                syntax.name_span,
            ));
        }
        let mut variants = BTreeSet::new();
        for variant in &syntax.variants {
            budget.node(1)?;
            budget.text(variant.name.len())?;
            if !variants.insert(variant.name.clone()) {
                return Err(semantic_error(
                    label,
                    "E3102",
                    "duplicate enum variant",
                    variant.name_span,
                ));
            }
            if let Some(payload) = &variant.payload_type {
                budget.text(payload.len())?;
            }
        }
        parsed.push((module, label, syntax, canonical));
    }
    let mut origins = Origins(Vec::new());
    let mut probe = Program {
        functions: vec![],
        modules: vec![],
        type_declarations: vec![],
        struct_declarations: vec![],
    };
    for input in &context.type_declarations {
        probe
            .type_declarations
            .push(alias(input, &context_label, &mut budget, &mut origins)?);
    }
    for input in &context.struct_declarations {
        probe
            .struct_declarations
            .push(record(input, &context_label, &mut budget, &mut origins)?);
    }
    for input in &context.modules {
        budget.node(1)?;
        let name = budget.owned(&input.name)?;
        let span = origins.add(&context_label, input.span);
        let mut declarations = Vec::new();
        for ty in &input.type_declarations {
            declarations.push(alias(ty, &context_label, &mut budget, &mut origins)?);
        }
        let mut records = Vec::new();
        for ty in &input.struct_declarations {
            records.push(record(ty, &context_label, &mut budget, &mut origins)?);
        }
        probe.modules.push(ast::Module {
            name,
            type_declarations: declarations,
            struct_declarations: records,
            functions: vec![],
            span,
        });
    }
    // Private type-only probes reuse existing alias/record resolution; these are
    // never exposed as successful source programs or enum runtime representations.
    let mut probe_locations = Vec::new();
    for (module, label, syntax, _) in &parsed {
        budget.node(1)?;
        let name = budget.owned(&syntax.name)?;
        let mut fields = Vec::new();
        for variant in &syntax.variants {
            budget.node(1)?;
            fields.push(StructField {
                name: budget.owned(&variant.name)?,
                type_name: budget.owned(variant.payload_type.as_deref().unwrap_or("Unit"))?,
                span: origins.add(
                    label,
                    variant.payload_type_span.unwrap_or(variant.name_span),
                ),
                type_span: Some(origins.add(
                    label,
                    variant.payload_type_span.unwrap_or(variant.name_span),
                )),
            });
        }
        let declaration = StructDeclaration {
            is_exported: syntax.is_exported,
            name,
            fields,
            span: origins.add(label, syntax.name_span),
        };
        if let Some(module) = module {
            let index = probe.modules[*module].struct_declarations.len();
            probe.modules[*module].struct_declarations.push(declaration);
            probe_locations.push((Some(*module), index));
        } else {
            let index = probe.struct_declarations.len();
            probe.struct_declarations.push(declaration);
            probe_locations.push((None, index));
        }
    }
    let typed = SemanticAnalyzer::new()
        .analyze_with_node_limit(&probe, limits.max_context_ast_nodes)
        .map_err(|errors| {
            if errors.items.iter().any(|e| e.code == "E3020") {
                failure(EnumTypeLimit::ContextAstNodes, limits.max_context_ast_nodes)
            } else {
                origins.errors(errors)
            }
        })?;
    let mut diagnostics = Diagnostics::new();
    let mut known = semantic::collect_named_types(&typed.program, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(origins.errors(diagnostics));
    }
    // Builtin record identities are resolved directly by Type::from_name_with_known;
    // do not retain duplicated standard bindings in every local environment.
    known.retain(|name, _| !crate::compiler::stdlib::is_standard_record_type(name));
    static NEXT_ENVIRONMENT: AtomicU64 = AtomicU64::new(0);
    let environment = allocate_identity(&NEXT_ENVIRONMENT)?;
    let enum_names: HashMap<_, _> = parsed
        .iter()
        .enumerate()
        .map(|(i, (_, _, _, name))| {
            (
                name.as_str(),
                EnumId {
                    environment,
                    ordinal: i,
                },
            )
        })
        .collect();
    let mut arena = Vec::new();
    let mut declarations = Vec::new();
    for (index, (_, label, syntax, canonical)) in parsed.iter().enumerate() {
        let (module, record) = probe_locations[index];
        let probe_record = if let Some(module) = module {
            &typed.program.modules[module].struct_declarations[record]
        } else {
            &typed.program.struct_declarations[record]
        };
        let mut variants = Vec::new();
        for (variant_index, variant) in syntax.variants.iter().enumerate() {
            let payload = if variant.payload_type.is_some() {
                let annotation = &probe_record.fields[variant_index].type_name;
                let resolved =
                    Type::from_name_with_known(annotation, Some(&known)).canonicalize(&known);
                let mut leaf = &resolved;
                let mut depth = 0;
                while let Type::Array(element) = leaf {
                    leaf = element;
                    depth += 1;
                }
                let kind = match leaf {
                    Type::Unit => EnumPayloadType::Unit,
                    Type::Bool => EnumPayloadType::Bool,
                    Type::Int => EnumPayloadType::Int,
                    Type::Float => EnumPayloadType::Float,
                    Type::String => EnumPayloadType::String,
                    Type::Named(name) => {
                        if let Some(id) = enum_names.get(name.as_str()) {
                            EnumPayloadType::Enum(*id)
                        } else {
                            EnumPayloadType::Record(budget.owned(name)?)
                        }
                    }
                    _ => {
                        return Err(semantic_error(
                            Arc::clone(label),
                            "E3104",
                            "unresolved enum payload type",
                            variant.payload_type_span.unwrap_or(variant.span),
                        ))
                    }
                };
                budget.node(1)?;
                let mut id = EnumTypeId {
                    environment,
                    ordinal: arena.len(),
                };
                arena.push(kind);
                for _ in 0..depth {
                    budget.node(1)?;
                    let next = EnumTypeId {
                        environment,
                        ordinal: arena.len(),
                    };
                    arena.push(EnumPayloadType::Array(id));
                    id = next;
                }
                Some(id)
            } else {
                None
            };
            variants.push(EnumVariantFact {
                name: budget.owned(&variant.name)?,
                span: variant.name_span,
                payload,
            });
        }
        declarations.push(EnumDeclarationFact {
            id: EnumId {
                environment,
                ordinal: index,
            },
            name: budget.owned(canonical)?,
            is_exported: syntax.is_exported,
            source_label: Arc::clone(label),
            span: syntax.name_span,
            variants,
        });
    }
    // Retain the validated alias/nominal context for closed constructor arguments.
    // Count every retained map binding and type node before publishing the environment.
    for (name, ty) in &known {
        budget.node(1)?;
        budget.text(name.len())?;
        let mut leaf = ty;
        while let Type::Array(element) = leaf {
            budget.node(1)?;
            leaf = element;
        }
        budget.node(1)?;
        if let Type::Named(name) = leaf {
            budget.text(name.len())?;
        }
    }
    // Retain real record fields only. Internal enum probes must never authorize
    // record construction or field projection on a nominal enum value.
    let mut record_fields =
        semantic::collect_struct_fields(&typed.program, &known, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(origins.errors(diagnostics));
    }
    record_fields.retain(|name, _| !enum_names.contains_key(name.as_str()));
    for (name, fields) in &record_fields {
        budget.node(1)?;
        budget.text(name.len())?;
        for (name, ty) in fields {
            budget.node(1)?;
            budget.text(name.len())?;
            let mut leaf = ty;
            while let Type::Array(element) = leaf {
                budget.node(1)?;
                leaf = element;
            }
            budget.node(1)?;
            if let Type::Named(name) = leaf {
                budget.text(name.len())?;
            }
        }
    }
    let mut retained_enum_names = HashMap::new();
    for declaration in &declarations {
        budget.node(1)?;
        budget.text(declaration.name.len())?;
        retained_enum_names.insert(declaration.name.clone(), declaration.id);
    }
    Ok(CheckedEnumTypes(
        Arc::new(Facts {
            environment,
            limits,
            owner,
            declarations,
            enum_names: retained_enum_names,
            types: arena,
            known,
            record_fields,
        }),
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_id_allocation_never_wraps_or_reuses_an_id() {
        let counter = AtomicU64::new(u64::MAX - 1);
        assert_eq!(allocate_identity(&counter), Ok(u64::MAX - 1));
        assert_eq!(
            allocate_identity(&counter),
            Err(EnumTypeError::IdentityExhausted)
        );
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    }
}
