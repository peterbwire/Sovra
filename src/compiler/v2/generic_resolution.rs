//! Bounded generic type name/arity/bound resolution, separate from body checking.
use super::{
    GenericAliasSyntax, GenericEnumSyntax, GenericParameterBound, GenericParameterId,
    GenericParameters, GenericRecordSyntax, GenericTypeKind, GenericTypeSyntax,
};
use crate::compiler::diagnostics::Span;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
static NEXT_REGISTRY: AtomicU64 = AtomicU64::new(1);
/// Supported source-owned generic type declaration, excluding functions.
#[derive(Debug, Clone, Copy)]
pub enum GenericTypeDeclaration<'a> {
    /// Transparent alias header; expansion remains a separate stage.
    Alias(&'a GenericAliasSyntax),
    /// Record header, without claiming field/layout validity.
    Record(&'a GenericRecordSyntax),
    /// Enum header, without claiming payload/constructor validity.
    Enum(&'a GenericEnumSyntax),
    /// Opaque compiler-owned Map/Set header.
    Collection(&'a super::GenericCollectionSyntax),
    /// Opaque compiler-owned Byte/Bytes header.
    Data(&'a super::GenericDataSyntax),
}
/// Category retained with a registered type identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericTypeDeclarationKind {
    /// Alias, requiring expansion before concrete underlying-type proofs.
    Alias,
    /// Nominal record.
    Record,
    /// Nominal enum.
    Enum,
    /// Opaque intrinsic collection, never a user record.
    Collection,
    /// Sealed standard data type, retaining its exact category.
    Data(super::GenericDataTypeKind),
}
impl<'a> GenericTypeDeclaration<'a> {
    pub(super) fn parts(
        self,
    ) -> (
        &'a str,
        &'a GenericParameters,
        Span,
        GenericTypeDeclarationKind,
    ) {
        match self {
            Self::Data(d) => (
                d.qualified_name(),
                d.parameters(),
                d.span(),
                GenericTypeDeclarationKind::Data(d.kind()),
            ),
            Self::Collection(d) => (
                d.qualified_name(),
                d.parameters(),
                d.span(),
                GenericTypeDeclarationKind::Collection,
            ),
            Self::Alias(d) => (
                d.name(),
                d.parameters(),
                d.name_span(),
                GenericTypeDeclarationKind::Alias,
            ),
            Self::Record(d) => (
                d.qualified_name(),
                d.parameters(),
                d.name_span(),
                GenericTypeDeclarationKind::Record,
            ),
            Self::Enum(d) => (
                d.qualified_name(),
                d.parameters(),
                d.name_span(),
                GenericTypeDeclarationKind::Enum,
            ),
        }
    }
}
/// Finite retention limits for immutable type headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericRegistryLimits {
    /// Distinct declaration headers and lookup entries.
    pub max_declarations: usize,
    /// Aggregate referenced parameter records.
    pub max_parameters: usize,
    /// Copied declaration-name bytes; index/header share each name allocation.
    pub max_name_bytes: usize,
    /// Aggregate referenced parameter-fragment source bytes.
    pub max_parameter_source_bytes: usize,
}
impl GenericRegistryLimits {
    /// Finite root-header profile, independent of specialization counts.
    pub const DEFAULT: Self = Self {
        max_declarations: 1024,
        max_parameters: 4096,
        max_name_bytes: 64 * 1024,
        max_parameter_source_bytes: 1024 * 1024,
    };
}
/// Type declaration identity owned by one immutable registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericTypeDeclarationId {
    owner: u64,
    ordinal: usize,
}
impl GenericTypeDeclarationId {
    /// Declaration input position within its registry only.
    pub fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Source-owned header, not proof that a declaration's target/fields/payloads check.
#[derive(Debug, Clone)]
pub struct GenericTypeHeader {
    /// Registry-local identity.
    pub id: GenericTypeDeclarationId,
    /// Shared immutable root name.
    pub name: Arc<str>,
    /// Declaration category.
    pub kind: GenericTypeDeclarationKind,
    /// Shared declaration-local parameter owner and bound spelling.
    pub parameters: GenericParameters,
}
#[derive(Debug)]
pub(super) struct PackageTypeContext {
    pub table: Arc<super::GenericPackageDeclarations>,
    pub scopes: BTreeMap<u64, Option<super::GenericPackageSourceId>>,
    pub types: BTreeMap<super::GenericPackageDeclarationId, usize>,
}
#[derive(Debug)]
struct Registry {
    package: Option<PackageTypeContext>,
    owner: u64,
    headers: Vec<GenericTypeHeader>,
    names: BTreeMap<Arc<str>, usize>,
}
/// Immutable bounded root type-header registry; recursive declarations may be indexed.
#[derive(Debug, Clone)]
pub struct GenericTypeRegistry(Arc<Registry>);
/// Existing primitive types available to this first semantic profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenericPrimitive {
    /// Unit.
    Unit,
    /// Boolean.
    Bool,
    /// Signed integer.
    Int,
    /// Floating point.
    Float,
    /// UTF-8 string.
    String,
}
pub(super) fn primitive(name: &str) -> Option<GenericPrimitive> {
    Some(match name {
        "Unit" => GenericPrimitive::Unit,
        "Bool" => GenericPrimitive::Bool,
        "Int" => GenericPrimitive::Int,
        "Float" => GenericPrimitive::Float,
        "String" => GenericPrimitive::String,
        _ => return None,
    })
}
/// Complete registry/annotation refusal; no partially resolved table is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericResolutionError {
    /// Selected logical allowance exhausted.
    Limit(&'static str),
    /// Invalid header; index addresses the input declarations and span their full source.
    Declaration {
        /// Input declaration index.
        index: usize,
        /// Original full-source declaration/parameter range.
        span: Span,
        /// Fixed bounded explanation.
        reason: &'static str,
    },
    /// Invalid annotation, in the retained type-expression source coordinates.
    Annotation {
        /// Original annotation range.
        span: Span,
        /// Fixed bounded explanation.
        reason: &'static str,
    },
    /// A parameter scope conflicts with an existing primitive identity.
    ParameterScope {
        /// Original name range in the parameter-list fragment source.
        span: Span,
    },
    /// Type application arity does not match its resolved head.
    Arity {
        /// Original application range.
        span: Span,
        /// Required argument count.
        expected: usize,
        /// Supplied argument count.
        actual: usize,
    },
    /// A concrete or symbolic argument does not prove the required std::Key bound.
    Bound {
        /// Original argument range.
        span: Span,
    },
    /// An alias argument needs transparent expansion before a Key proof is possible.
    AliasBoundNeedsExpansion {
        /// Original alias application argument range.
        span: Span,
    },
}
impl std::fmt::Display for GenericResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic type resolution refused: {self:?}")
    }
}
impl std::error::Error for GenericResolutionError {}
fn charge(
    left: &mut usize,
    n: usize,
    dimension: &'static str,
) -> Result<(), GenericResolutionError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericResolutionError::Limit(dimension))?;
    Ok(())
}
/// Index admitted root generic declarations, checking duplicate/reserved names.
/// Only headers are indexed; this does not validate declaration bodies or cycles.
pub fn register_generic_types(
    declarations: &[GenericTypeDeclaration<'_>],
    limits: GenericRegistryLimits,
) -> Result<GenericTypeRegistry, GenericResolutionError> {
    register_types(declarations, limits, None)
}
pub(super) fn register_package_types(
    declarations: &[GenericTypeDeclaration<'_>],
    limits: GenericRegistryLimits,
    context: PackageTypeContext,
) -> Result<GenericTypeRegistry, GenericResolutionError> {
    register_types(declarations, limits, Some(context))
}
fn register_types(
    declarations: &[GenericTypeDeclaration<'_>],
    limits: GenericRegistryLimits,
    package: Option<PackageTypeContext>,
) -> Result<GenericTypeRegistry, GenericResolutionError> {
    if declarations.len() > limits.max_declarations {
        return Err(GenericResolutionError::Limit("type declarations"));
    }
    let mut parameters_left = limits.max_parameters;
    let mut names_left = limits.max_name_bytes;
    let mut source_left = limits.max_parameter_source_bytes;
    let mut headers = Vec::new();
    let mut names = BTreeMap::new();
    for (index, declaration) in declarations.iter().enumerate() {
        let (name, parameters, span, kind) = declaration.parts();
        if primitive(name).is_some() {
            return Err(GenericResolutionError::Declaration {
                index,
                span,
                reason: "type declaration conflicts with a primitive",
            });
        }
        if names.contains_key(name) && (package.is_none() || name.starts_with("std::")) {
            return Err(GenericResolutionError::Declaration {
                index,
                span,
                reason: "duplicate type declaration",
            });
        }
        charge(
            &mut parameters_left,
            parameters.parameters().len(),
            "registry parameters",
        )?;
        charge(&mut names_left, name.len(), "registry names")?;
        charge(
            &mut source_left,
            parameters.source().len(),
            "registry parameter source",
        )?;
        for parameter in parameters.parameters() {
            if primitive(&parameter.name).is_some() {
                let span = match declaration {
                    GenericTypeDeclaration::Alias(d) => d.parameter_span(parameter.id),
                    GenericTypeDeclaration::Record(d) => d.parameter_span(parameter.id),
                    GenericTypeDeclaration::Enum(d) => d.parameter_span(parameter.id),
                    GenericTypeDeclaration::Data(d) => {
                        d.parameters().parameter(parameter.id).map(|p| p.name_span)
                    }
                    GenericTypeDeclaration::Collection(d) => {
                        d.parameters().parameter(parameter.id).map(|p| p.name_span)
                    }
                }
                .expect("owned parameter");
                return Err(GenericResolutionError::Declaration {
                    index,
                    span,
                    reason: "type parameter conflicts with a primitive",
                });
            }
        }
        let name: Arc<str> = Arc::from(name);
        if package.is_none() || name.starts_with("std::") {
            names.insert(Arc::clone(&name), index);
        }
        headers.push(GenericTypeHeader {
            id: GenericTypeDeclarationId {
                owner: 0,
                ordinal: index,
            },
            name,
            kind,
            parameters: parameters.clone(),
        });
    }
    let owner = loop {
        let current = NEXT_REGISTRY.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .ok_or(GenericResolutionError::Limit("registry identities"))?;
        if NEXT_REGISTRY
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break current;
        }
    };
    for header in &mut headers {
        header.id.owner = owner;
    }
    Ok(GenericTypeRegistry(Arc::new(Registry {
        package,
        owner,
        headers,
        names,
    })))
}
/// Resolved structure. Child indices address this result's annotation-local arena.
/// No cross-result type equivalence/alias normalization is implied by indices.
#[derive(Debug, Clone)]
pub enum ResolvedGenericTypeKind {
    /// Existing primitive identity.
    Primitive(GenericPrimitive),
    /// Parameter in the retained annotation's parameter owner.
    Parameter(GenericParameterId),
    /// Resolved type header with arity-checked ordered arguments.
    Declared {
        /// Owning registry identity.
        declaration: GenericTypeDeclarationId,
        /// Earlier resolved node indices.
        arguments: Vec<usize>,
    },
    /// Structural array with one earlier element node.
    Array {
        /// Earlier resolved node index.
        element: usize,
    },
}
/// Logical construction/checking limits, separate from parsed syntax quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericResolutionLimits {
    /// Retained resolved type nodes.
    pub max_nodes: usize,
    /// Retained argument/array edges.
    pub max_edges: usize,
    /// Logical node, parameter-lookup and bound-check steps; not CPU time.
    pub max_work: usize,
}
impl GenericResolutionLimits {
    /// Finite opt-in defaults; no unchecked fallback.
    pub const DEFAULT: Self = Self {
        max_nodes: 4096,
        max_edges: 4096,
        max_work: 65_536,
    };
}
/// Complete annotation name/arity/bound proof, not a checked declaration/body.
#[derive(Debug, Clone)]
pub struct ResolvedGenericType {
    registry: GenericTypeRegistry,
    parameters: GenericParameters,
    syntax: GenericTypeSyntax,
    nodes: Arc<[ResolvedGenericTypeKind]>,
    work: usize,
}
impl ResolvedGenericType {
    /// Registry owning all declared head identities.
    pub fn registry(&self) -> &GenericTypeRegistry {
        &self.registry
    }
    /// Scope owning all parameter identities.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    /// Original source and postorder spans, aligned with resolved nodes.
    pub fn syntax(&self) -> &GenericTypeSyntax {
        &self.syntax
    }
    /// Immutable postorder resolved structures.
    pub fn nodes(&self) -> &[ResolvedGenericTypeKind] {
        &self.nodes
    }
    /// Logical checking steps consumed.
    pub fn work(&self) -> usize {
        self.work
    }
}
impl GenericTypeRegistry {
    pub(super) fn is_package_scoped(&self) -> bool {
        self.0.package.is_some()
    }
    pub(super) fn function_scope(&self, parameters: &GenericParameters) -> Option<usize> {
        match &self.0.package {
            None => Some(0),
            Some(context) => context
                .scopes
                .get(&parameters.scope_key())?
                .map(|source| source.ordinal() + 1)
                .or(Some(0)),
        }
    }
    pub(super) fn package_function_owner(
        &self,
        name: &str,
        parameters: &GenericParameters,
    ) -> Option<u64> {
        let context = self.0.package.as_ref()?;
        let source = (*context.scopes.get(&parameters.scope_key())?)?;
        let mut parts = name.split("::");
        let first = parts.next()?;
        let second = parts.next();
        let third = parts.next();
        if parts.next().is_some() {
            return None;
        }
        let declaration = match (second, third) {
            (None, None) => {
                context
                    .table
                    .resolve(source, &[], super::GenericPackageNamespace::Function, first)
            }
            (Some(name), None) => context.table.resolve(
                source,
                &[first],
                super::GenericPackageNamespace::Function,
                name,
            ),
            (Some(module), Some(name)) => context.table.resolve(
                source,
                &[first, module],
                super::GenericPackageNamespace::Function,
                name,
            ),
            _ => None,
        }?;
        match context.table.declaration(declaration)?.syntax().kind() {
            super::GenericSourceDeclarationKind::Function(function) => {
                Some(function.parameters().scope_key())
            }
            _ => None,
        }
    }
    pub(super) fn find_header_in(
        &self,
        name: &str,
        parameters: &GenericParameters,
    ) -> Option<&GenericTypeHeader> {
        if let Some(context) = &self.0.package {
            let scope = context.scopes.get(&parameters.scope_key())?;
            if name.starts_with("std::") {
                return self.0.names.get(name).and_then(|i| self.0.headers.get(*i));
            }
            let source = (*scope)?;
            let mut parts = name.split("::");
            let first = parts.next()?;
            let second = parts.next();
            let third = parts.next();
            if parts.next().is_some() {
                return None;
            }
            let id = match (second, third) {
                (None, None) => {
                    context
                        .table
                        .resolve(source, &[], super::GenericPackageNamespace::Type, first)
                }
                (Some(name), None) => context.table.resolve(
                    source,
                    &[first],
                    super::GenericPackageNamespace::Type,
                    name,
                ),
                (Some(module), Some(name)) => context.table.resolve(
                    source,
                    &[first, module],
                    super::GenericPackageNamespace::Type,
                    name,
                ),
                _ => None,
            }?;
            return self.0.headers.get(*context.types.get(&id)?);
        }
        self.0
            .names
            .get(name)
            .and_then(|index| self.0.headers.get(*index))
    }
    /// Headers in input order.
    pub fn headers(&self) -> &[GenericTypeHeader] {
        &self.0.headers
    }
    /// Inspect an identity only when it belongs to this registry.
    pub fn header(&self, id: GenericTypeDeclarationId) -> Option<&GenericTypeHeader> {
        if id.owner != self.0.owner {
            return None;
        }
        self.0.headers.get(id.ordinal)
    }
    /// Resolve one annotation under its declaration-local parameter scope.
    /// Reject unknown names, wrong arities and unproven bounds; alias Key proofs
    /// explicitly refuse pending expansion rather than treating an alias as a key.
    pub fn resolve(
        &self,
        syntax: &GenericTypeSyntax,
        parameters: &GenericParameters,
        limits: GenericResolutionLimits,
    ) -> Result<ResolvedGenericType, GenericResolutionError> {
        let (nodes, work) = self.resolve_nodes(syntax, parameters, limits, true)?;
        Ok(ResolvedGenericType {
            registry: self.clone(),
            parameters: parameters.clone(),
            syntax: syntax.clone(),
            nodes: nodes.into(),
            work,
        })
    }
    // Internal head resolution deliberately returns no public checked proof.
    // Alias expansion checks bounds after transparent arguments are substituted.
    pub(super) fn resolve_nodes(
        &self,
        syntax: &GenericTypeSyntax,
        parameters: &GenericParameters,
        limits: GenericResolutionLimits,
        check_bounds: bool,
    ) -> Result<(Vec<ResolvedGenericTypeKind>, usize), GenericResolutionError> {
        if syntax.nodes().len() > limits.max_nodes {
            return Err(GenericResolutionError::Limit("resolved nodes"));
        }
        if syntax.edges() > limits.max_edges {
            return Err(GenericResolutionError::Limit("resolved edges"));
        }
        if self.function_scope(parameters).is_none() {
            return Err(GenericResolutionError::Limit(
                "foreign package parameter scope",
            ));
        }
        let mut work = limits.max_work;
        for parameter in parameters.parameters() {
            charge(&mut work, 1, "resolution work")?;
            if primitive(&parameter.name).is_some() {
                return Err(GenericResolutionError::ParameterScope {
                    span: parameter.name_span,
                });
            }
        }
        let mut nodes = Vec::new();
        for node in syntax.nodes() {
            charge(&mut work, 1, "resolution work")?;
            let kind = match &node.kind {
                GenericTypeKind::Array { element } => {
                    ResolvedGenericTypeKind::Array { element: *element }
                }
                GenericTypeKind::Named { name, arguments } => {
                    let mut parameter = None;
                    for candidate in parameters.parameters() {
                        charge(&mut work, 1, "resolution work")?;
                        if candidate.name == *name {
                            parameter = Some(candidate.id);
                            break;
                        }
                    }
                    let (expected, head) = if let Some(parameter) = parameter {
                        (0, ResolvedGenericTypeKind::Parameter(parameter))
                    } else if let Some(kind) = primitive(name) {
                        (0, ResolvedGenericTypeKind::Primitive(kind))
                    } else {
                        charge(&mut work, 1, "resolution work")?;
                        let Some(header) = self.find_header_in(name, parameters) else {
                            return Err(GenericResolutionError::Annotation {
                                span: node.span,
                                reason: "unknown type name",
                            });
                        };
                        (
                            header.parameters.parameters().len(),
                            ResolvedGenericTypeKind::Declared {
                                declaration: header.id,
                                arguments: Vec::new(),
                            },
                        )
                    };
                    if arguments.len() != expected {
                        return Err(GenericResolutionError::Arity {
                            span: node.span,
                            expected,
                            actual: arguments.len(),
                        });
                    }
                    if let ResolvedGenericTypeKind::Declared { declaration, .. } = head {
                        let header = &self.0.headers[declaration.ordinal];
                        for (argument, bound) in
                            arguments.iter().zip(header.parameters.parameters())
                        {
                            charge(&mut work, 1, "resolution work")?;
                            if check_bounds && bound.bound == Some(GenericParameterBound::Key) {
                                let allowed = match &nodes[*argument] {
                                    ResolvedGenericTypeKind::Primitive(
                                        GenericPrimitive::Int
                                        | GenericPrimitive::Bool
                                        | GenericPrimitive::String,
                                    ) => true,
                                    ResolvedGenericTypeKind::Declared { declaration, .. }
                                        if super::generic_data_types::is_byte_header(
                                            self,
                                            *declaration,
                                        ) =>
                                    {
                                        true
                                    }
                                    ResolvedGenericTypeKind::Parameter(id) => {
                                        parameters.parameter(*id).is_some_and(|p| {
                                            p.bound == Some(GenericParameterBound::Key)
                                        })
                                    }
                                    ResolvedGenericTypeKind::Declared { declaration, .. }
                                        if self.0.headers[declaration.ordinal].kind
                                            == GenericTypeDeclarationKind::Alias =>
                                    {
                                        return Err(
                                            GenericResolutionError::AliasBoundNeedsExpansion {
                                                span: syntax.nodes()[*argument].span,
                                            },
                                        )
                                    }
                                    _ => false,
                                };
                                if !allowed {
                                    return Err(GenericResolutionError::Bound {
                                        span: syntax.nodes()[*argument].span,
                                    });
                                }
                            }
                        }
                        ResolvedGenericTypeKind::Declared {
                            declaration,
                            arguments: arguments.clone(),
                        }
                    } else {
                        head
                    }
                }
            };
            nodes.push(kind);
        }
        Ok((nodes, limits.max_work - work))
    }
}
