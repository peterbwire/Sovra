//! Complete bounded generic type-definition annotation checking; no function/body lowering.
use super::{
    ExpandedGenericType, GenericAliasEnvironment, GenericAliasExpansionError,
    GenericAliasExpansionLimits, GenericAliasSyntax, GenericEnumSyntax, GenericRecordSyntax,
    GenericTypeDeclaration, GenericTypeDeclarationId, GenericTypeDeclarationKind,
};
use crate::compiler::diagnostics::Span;
use std::sync::Arc;
/// Cumulative complete-type-declaration admission alongside per-annotation expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericDeclarationLimits {
    /// Registry headers and supplied definitions, each bounded before allocation.
    pub max_declarations: usize,
    /// Aggregate record fields plus enum variants.
    pub max_members: usize,
    /// Referenced declaration/parameter/member/type-syntax records.
    pub max_metadata_nodes: usize,
    /// Referenced retained full/fragment source bytes.
    pub max_source_bytes: usize,
    /// Referenced syntax names plus newly cloned declaration-name bytes.
    pub max_name_bytes: usize,
    /// Cumulative generated type nodes across all annotations.
    pub max_expanded_nodes: usize,
    /// Cumulative generated type edges across all annotations.
    pub max_expanded_edges: usize,
    /// Header/scope/coverage checks and annotation expansion steps.
    pub max_work: usize,
    /// Independent per-annotation admission, capped by remaining cumulative budgets.
    pub expansion: GenericAliasExpansionLimits,
}
impl GenericDeclarationLimits {
    /// Finite root definition profile; no executable generic paths are selected.
    pub const DEFAULT: Self = Self {
        max_declarations: 1024,
        max_members: 8192,
        max_metadata_nodes: 65_536,
        max_source_bytes: 4 * 1024 * 1024,
        max_name_bytes: 1024 * 1024,
        max_expanded_nodes: 65_536,
        max_expanded_edges: 65_536,
        max_work: 1_000_000,
        expansion: GenericAliasExpansionLimits::DEFAULT,
    };
}
/// No partial checked definition collection is returned on failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericDeclarationError {
    /// A selected cumulative admission dimension exhausted.
    Limit(&'static str),
    /// A supplied definition does not match its original registered owner.
    Ownership {
        /// Supplied definition index.
        input: usize,
        /// Original full-source name/parameter span.
        span: Span,
        /// Fixed bounded explanation.
        reason: &'static str,
    },
    /// A registered definition was not supplied, including unused types.
    Missing(GenericTypeDeclarationId),
    /// Executable enum type declarations require at least one variant.
    EmptyEnum {
        /// Registered type identity.
        declaration: GenericTypeDeclarationId,
        /// Original full-source enum-name span.
        span: Span,
    },
    /// A field/payload annotation failed in its original fragment context.
    Annotation {
        /// Registered owning declaration.
        declaration: GenericTypeDeclarationId,
        /// Source-order field/variant index.
        member: usize,
        /// Full-source range corresponding to the input annotation fragment.
        annotation_range: Span,
        /// Preserved input/alias provenance; alias-target origins use their own source.
        error: Box<GenericAliasExpansionError>,
    },
}
impl std::fmt::Display for GenericDeclarationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic declaration checking refused: {self:?}")
    }
}
impl std::error::Error for GenericDeclarationError {}
/// Sealed checked alias target ownership; target semantics come from its prepared environment.
#[derive(Debug, Clone)]
pub struct CheckedGenericAlias {
    environment: GenericAliasEnvironment,
    id: GenericTypeDeclarationId,
    syntax: GenericAliasSyntax,
}
impl CheckedGenericAlias {
    /// Registered declaration identity.
    pub fn id(&self) -> GenericTypeDeclarationId {
        self.id
    }
    /// Retained original declaration syntax.
    pub fn syntax(&self) -> &GenericAliasSyntax {
        &self.syntax
    }
    /// Prepared environment containing its validated target.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
}
/// Sealed checked field-type annotations, not a runtime layout or constructor plan.
#[derive(Debug, Clone)]
pub struct CheckedGenericRecord {
    environment: GenericAliasEnvironment,
    id: GenericTypeDeclarationId,
    syntax: GenericRecordSyntax,
    fields: Arc<[ExpandedGenericType]>,
}
impl CheckedGenericRecord {
    /// Registered declaration identity.
    pub fn id(&self) -> GenericTypeDeclarationId {
        self.id
    }
    /// Original immutable record syntax.
    pub fn syntax(&self) -> &GenericRecordSyntax {
        &self.syntax
    }
    /// Prepared environment owning all type identities.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
    /// Complete source-order expanded symbolic field types.
    pub fn field_types(&self) -> &[ExpandedGenericType] {
        &self.fields
    }
}
/// Sealed checked payload annotations and nonempty variant coverage, not match/constructor IR.
#[derive(Debug, Clone)]
pub struct CheckedGenericEnum {
    environment: GenericAliasEnvironment,
    id: GenericTypeDeclarationId,
    syntax: GenericEnumSyntax,
    payloads: Arc<[Option<ExpandedGenericType>]>,
}
impl CheckedGenericEnum {
    /// Registered declaration identity.
    pub fn id(&self) -> GenericTypeDeclarationId {
        self.id
    }
    /// Original immutable enum syntax.
    pub fn syntax(&self) -> &GenericEnumSyntax {
        &self.syntax
    }
    /// Prepared environment owning all type identities.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
    /// Source-order payload types; None retains each nullary variant.
    pub fn payload_types(&self) -> &[Option<ExpandedGenericType>] {
        &self.payloads
    }
}
/// Checked definition category. Inner proof constructors are private.
#[derive(Debug, Clone)]
pub enum CheckedGenericDefinition {
    /// Checked sealed Byte/Bytes header without fields or variants.
    Data(super::CheckedGenericData),
    /// Checked opaque compiler-owned collection header.
    Collection(super::CheckedGenericCollection),
    /// Validated alias target and ownership.
    Alias(CheckedGenericAlias),
    /// Every symbolic field annotation checked.
    Record(CheckedGenericRecord),
    /// Nonempty enum with every symbolic payload checked.
    Enum(CheckedGenericEnum),
}
/// Complete immutable type definitions aligned with registered declaration order.
#[derive(Debug, Clone)]
pub struct CheckedGenericDeclarations {
    environment: GenericAliasEnvironment,
    definitions: Arc<[CheckedGenericDefinition]>,
    expanded_nodes: usize,
    expanded_edges: usize,
    work: usize,
}
impl CheckedGenericDeclarations {
    /// Retained prepared type/alias owner.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
    /// Complete definitions in registry order, independent of supplied ordering.
    pub fn definitions(&self) -> &[CheckedGenericDefinition] {
        &self.definitions
    }
    /// Read an identity only when it belongs to this registry.
    pub fn definition(&self, id: GenericTypeDeclarationId) -> Option<&CheckedGenericDefinition> {
        self.environment.registry().header(id)?;
        self.definitions.get(id.ordinal())
    }
    /// Cumulative generated annotation nodes, without refunds.
    pub fn expanded_nodes(&self) -> usize {
        self.expanded_nodes
    }
    /// Cumulative generated annotation edges, without refunds.
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
) -> Result<(), GenericDeclarationError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericDeclarationError::Limit(dimension))?;
    Ok(())
}
struct Admission {
    members: usize,
    metadata: usize,
    source: usize,
    names: usize,
    nodes: usize,
    edges: usize,
    work: usize,
}
impl Admission {
    fn annotation(
        &mut self,
        environment: &GenericAliasEnvironment,
        input: Annotation<'_>,
        mut expansion: GenericAliasExpansionLimits,
    ) -> Result<ExpandedGenericType, GenericDeclarationError> {
        expansion.max_nodes = expansion.max_nodes.min(self.nodes);
        expansion.max_edges = expansion.max_edges.min(self.edges);
        expansion.max_work = expansion.max_work.min(self.work);
        let checked = environment
            .expand(input.syntax, input.parameters, expansion)
            .map_err(|error| GenericDeclarationError::Annotation {
                declaration: input.declaration,
                member: input.member,
                annotation_range: input.range,
                error: Box::new(error),
            })?;
        charge(
            &mut self.nodes,
            checked.nodes().len(),
            "declaration expanded nodes",
        )?;
        charge(
            &mut self.edges,
            checked.edges(),
            "declaration expanded edges",
        )?;
        charge(&mut self.work, checked.work(), "declaration work")?;
        Ok(checked)
    }
}
struct Annotation<'a> {
    declaration: GenericTypeDeclarationId,
    member: usize,
    range: Span,
    syntax: &'a super::GenericTypeSyntax,
    parameters: &'a super::GenericParameters,
}
/// Check exactly one original definition for every registered root type, including
/// unused definitions. Alias targets are already checked by the prepared owner.
/// Record/enum annotation validity is not runtime layout, constructors or function checking.
pub fn check_generic_declarations(
    environment: &GenericAliasEnvironment,
    declarations: &[GenericTypeDeclaration<'_>],
    limits: GenericDeclarationLimits,
) -> Result<CheckedGenericDeclarations, GenericDeclarationError> {
    let registry = environment.registry();
    if declarations.len() > limits.max_declarations
        || registry.headers().len() > limits.max_declarations
    {
        return Err(GenericDeclarationError::Limit("type definitions"));
    }
    let mut admission = Admission {
        members: limits.max_members,
        metadata: limits.max_metadata_nodes,
        source: limits.max_source_bytes,
        names: limits.max_name_bytes,
        nodes: limits.max_expanded_nodes,
        edges: limits.max_expanded_edges,
        work: limits.max_work,
    };
    let mut definitions: Vec<Option<CheckedGenericDefinition>> =
        (0..registry.headers().len()).map(|_| None).collect();
    for (input, declaration) in declarations.iter().enumerate() {
        charge(&mut admission.work, 1, "declaration work")?;
        let (name, parameters, span, kind) = declaration.parts();
        let ownership = |reason| GenericDeclarationError::Ownership {
            input,
            span,
            reason,
        };
        let header = registry
            .find_header_in(name, parameters)
            .ok_or_else(|| ownership("unregistered type definition"))?;
        if header.kind != kind {
            return Err(ownership("definition kind differs from registered header"));
        }
        if definitions[header.id.ordinal()].is_some() {
            return Err(ownership("duplicate type definition"));
        }
        if matches!(declaration, GenericTypeDeclaration::Data(_))
            && parameters.scope_key() != header.parameters.scope_key()
        {
            return Err(ownership("foreign data parameter scope"));
        }
        if parameters.parameters().len() != header.parameters.parameters().len() {
            return Err(ownership("foreign parameter owner"));
        }
        for (a, b) in parameters
            .parameters()
            .iter()
            .zip(header.parameters.parameters())
        {
            charge(&mut admission.work, 1, "declaration work")?;
            if a.id != b.id {
                return Err(ownership("foreign parameter owner"));
            }
        }
        let (members, nodes, source, names) = match declaration {
            GenericTypeDeclaration::Data(d) => (
                0,
                0,
                d.parameters().source().len(),
                d.parameters().name_bytes(),
            ),
            GenericTypeDeclaration::Collection(d) => (
                0,
                0,
                d.parameters().source().len(),
                d.parameters().name_bytes(),
            ),
            GenericTypeDeclaration::Alias(d) => (
                0,
                d.target().syntax().nodes().len(),
                d.retained_source_bytes(),
                d.name_bytes(),
            ),
            GenericTypeDeclaration::Record(d) => (
                d.fields().len(),
                d.type_nodes(),
                d.retained_source_bytes(),
                d.name_bytes(),
            ),
            GenericTypeDeclaration::Enum(d) => (
                d.variants().len(),
                d.type_nodes(),
                d.retained_source_bytes(),
                d.name_bytes(),
            ),
        };
        charge(&mut admission.members, members, "declaration members")?;
        let metadata = nodes
            .checked_add(members)
            .and_then(|n| n.checked_add(parameters.parameters().len()))
            .and_then(|n| n.checked_add(1))
            .ok_or(GenericDeclarationError::Limit("declaration metadata"))?;
        charge(&mut admission.metadata, metadata, "declaration metadata")?;
        charge(&mut admission.source, source, "declaration source")?;
        charge(
            &mut admission.names,
            names
                .checked_add(name.len())
                .ok_or(GenericDeclarationError::Limit("declaration names"))?,
            "declaration names",
        )?;
        let checked = match declaration {
            GenericTypeDeclaration::Data(d) => CheckedGenericDefinition::Data(
                super::CheckedGenericData::checked(environment.clone(), header.id, (*d).clone()),
            ),
            GenericTypeDeclaration::Collection(d) => {
                CheckedGenericDefinition::Collection(super::CheckedGenericCollection::checked(
                    environment.clone(),
                    header.id,
                    (*d).clone(),
                ))
            }
            GenericTypeDeclaration::Alias(d) => {
                if kind != GenericTypeDeclarationKind::Alias
                    || environment.alias_source(header.id).is_none()
                {
                    return Err(ownership("prepared alias target is missing"));
                }
                CheckedGenericDefinition::Alias(CheckedGenericAlias {
                    environment: environment.clone(),
                    id: header.id,
                    syntax: (*d).clone(),
                })
            }
            GenericTypeDeclaration::Record(d) => {
                let mut fields = Vec::new();
                for (member, field) in d.fields().iter().enumerate() {
                    charge(&mut admission.work, 1, "declaration work")?;
                    fields.push(admission.annotation(
                        environment,
                        Annotation {
                            declaration: header.id,
                            member,
                            range: field.type_range,
                            syntax: field.annotation.syntax(),
                            parameters,
                        },
                        limits.expansion,
                    )?);
                }
                CheckedGenericDefinition::Record(CheckedGenericRecord {
                    environment: environment.clone(),
                    id: header.id,
                    syntax: (*d).clone(),
                    fields: fields.into(),
                })
            }
            GenericTypeDeclaration::Enum(d) => {
                if d.variants().is_empty() {
                    return Err(GenericDeclarationError::EmptyEnum {
                        declaration: header.id,
                        span: d.name_span(),
                    });
                }
                let mut payloads = Vec::new();
                for (member, variant) in d.variants().iter().enumerate() {
                    charge(&mut admission.work, 1, "declaration work")?;
                    let payload = if let Some(payload) = &variant.payload {
                        Some(admission.annotation(
                            environment,
                            Annotation {
                                declaration: header.id,
                                member,
                                range: payload.type_range,
                                syntax: payload.annotation.syntax(),
                                parameters,
                            },
                            limits.expansion,
                        )?)
                    } else {
                        None
                    };
                    payloads.push(payload);
                }
                CheckedGenericDefinition::Enum(CheckedGenericEnum {
                    environment: environment.clone(),
                    id: header.id,
                    syntax: (*d).clone(),
                    payloads: payloads.into(),
                })
            }
        };
        definitions[header.id.ordinal()] = Some(checked);
    }
    let mut complete = Vec::new();
    for (header, definition) in registry.headers().iter().zip(definitions) {
        charge(&mut admission.work, 1, "declaration work")?;
        complete.push(definition.ok_or(GenericDeclarationError::Missing(header.id))?);
    }
    Ok(CheckedGenericDeclarations {
        environment: environment.clone(),
        definitions: complete.into(),
        expanded_nodes: limits.max_expanded_nodes - admission.nodes,
        expanded_edges: limits.max_expanded_edges - admission.edges,
        work: limits.max_work - admission.work,
    })
}
