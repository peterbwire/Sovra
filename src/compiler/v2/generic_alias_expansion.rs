//! Transparent alias expansion into a bounded immutable type DAG; no runtime lowering.
use super::generic_data_types::DataResultKind;
use super::{
    GenericAliasSyntax, GenericParameterBound, GenericParameters, GenericPrimitive,
    GenericResolutionError, GenericResolutionLimits, GenericTypeDeclarationId,
    GenericTypeDeclarationKind, GenericTypeRegistry, GenericTypeSyntax, ResolvedGenericTypeKind,
};
use crate::compiler::diagnostics::Span;
use std::{collections::BTreeMap, sync::Arc};
/// Per-expansion generated type and logical work limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericAliasExpansionLimits {
    /// Generated immutable type nodes, including subsequently unused arguments.
    pub max_nodes: usize,
    /// Retained array/application child edges.
    pub max_edges: usize,
    /// Input-head resolution plus expansion/checking steps.
    pub max_work: usize,
    /// Structural result depth and recursive expansion walk; fixed ceiling 128.
    pub max_depth: usize,
    /// Independent input annotation head admission.
    pub input: GenericResolutionLimits,
}
impl GenericAliasExpansionLimits {
    /// Finite opt-in profile; no specialization/execution fallback.
    pub const DEFAULT: Self = Self {
        max_nodes: 4096,
        max_edges: 4096,
        max_work: 65_536,
        max_depth: 128,
        input: GenericResolutionLimits::DEFAULT,
    };
}
/// Aggregate retention and validation limits for a complete alias environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericAliasEnvironmentLimits {
    /// Alias definitions and lookup entries.
    pub max_aliases: usize,
    /// Referenced syntax plus retained resolved-head nodes, counted separately.
    pub max_nodes: usize,
    /// Referenced syntax plus retained resolved-head edges, counted separately.
    pub max_edges: usize,
    /// Referenced alias target source-fragment bytes.
    pub max_source_bytes: usize,
    /// Referenced target qualified-name bytes.
    pub max_name_bytes: usize,
    /// Generated nodes across all symbolic alias-target validations.
    pub max_validation_nodes: usize,
    /// Generated edges across all symbolic alias-target validations.
    pub max_validation_edges: usize,
    /// Header matching, head resolution, cycle scan and symbolic validation steps.
    pub max_work: usize,
    /// Alias dependency walk depth, fixed ceiling 128.
    pub max_depth: usize,
    /// Independent per-target symbolic expansion limits.
    pub expansion: GenericAliasExpansionLimits,
}
impl GenericAliasEnvironmentLimits {
    /// Finite aggregate profile; source/header admission remains independent.
    pub const DEFAULT: Self = Self {
        max_aliases: 1024,
        max_nodes: 8192,
        max_edges: 8192,
        max_source_bytes: 1024 * 1024,
        max_name_bytes: 64 * 1024,
        max_validation_nodes: 65_536,
        max_validation_edges: 65_536,
        max_work: 1_000_000,
        max_depth: 128,
        expansion: GenericAliasExpansionLimits::DEFAULT,
    };
}
/// Span owner: a node-specific input annotation or a retained alias target.
/// For expanded nodes, use `ExpandedGenericType::node_source` to select the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericExpansionOrigin {
    /// None is the input annotation; Some is that alias's retained target fragment.
    pub alias: Option<GenericTypeDeclarationId>,
    /// Original range in the selected source fragment.
    pub span: Span,
}
/// Complete environment/expansion refusal, without partial successful output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericAliasExpansionError {
    /// Finite logical allowance exhausted.
    Limit(&'static str),
    /// Missing, duplicate or foreign declaration ownership.
    Environment(&'static str),
    /// Cyclic alias dependencies, including dependencies nested in type arguments.
    Cycle(GenericTypeDeclarationId),
    /// Name/arity/scope failure in an input or alias-target fragment.
    Resolution {
        /// Source owner; ParameterScope errors use its parameter fragment instead.
        alias: Option<GenericTypeDeclarationId>,
        /// Existing bounded head error.
        error: GenericResolutionError,
    },
    /// Expanded argument does not satisfy std::Key.
    Bound(GenericExpansionOrigin),
}
impl std::fmt::Display for GenericAliasExpansionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic alias expansion refused: {self:?}")
    }
}
impl std::error::Error for GenericAliasExpansionError {}
#[derive(Debug, Clone)]
struct Definition {
    parameters: GenericParameters,
    syntax: GenericTypeSyntax,
    nodes: Arc<[ResolvedGenericTypeKind]>,
}
#[derive(Debug)]
struct Environment {
    registry: GenericTypeRegistry,
    definitions: BTreeMap<usize, Definition>,
    validation_work: usize,
    validation_nodes: usize,
    validation_edges: usize,
}
/// Complete, acyclic alias targets with symbolic bound checks; not a checked program.
#[derive(Debug, Clone)]
pub struct GenericAliasEnvironment(Arc<Environment>);
/// One immutable expanded node. Aliases are absent from declared heads.
#[derive(Debug, Clone)]
pub struct ExpandedGenericTypeNode {
    /// Primitive/parameter/non-alias declaration/array with earlier child indices.
    pub kind: ResolvedGenericTypeKind,
    /// Original input/alias-target provenance.
    pub origin: GenericExpansionOrigin,
}
/// A transparent expanded type DAG with an explicit root, not a specialization.
#[derive(Debug, Clone)]
pub struct ExpandedGenericType {
    environment: GenericAliasEnvironment,
    syntax: GenericTypeSyntax,
    parameters: GenericParameters,
    nodes: Arc<[ExpandedGenericTypeNode]>,
    root: usize,
    edges: usize,
    work: usize,
    input_origins: Option<Arc<[GenericTypeSyntax]>>,
}
impl ExpandedGenericType {
    fn input_origin(&self, index: usize) -> &GenericTypeSyntax {
        self.input_origins
            .as_ref()
            .map_or(&self.syntax, |origins| &origins[index])
    }
    /// Source fragment owning a node's origin, including composed field inputs.
    /// Use with that node's origin span; the primary syntax alone may differ.
    pub fn node_source(&self, index: usize) -> Option<&str> {
        let node = self.nodes.get(index)?;
        match node.origin.alias {
            Some(alias) => self.environment.alias_source(alias),
            None => Some(self.input_origin(index).source()),
        }
    }
    // Derive a compiler-owned Option around a checked payload without reparsing
    // names or losing the payload's original environment/parameter/source owners.
    pub(super) fn wrap_standard_option(
        &self,
        option: &super::CheckedGenericEnum,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        if !self.environment.same_environment(option.environment())
            || option.syntax().standard_identity() != Some(super::GenericStandardEnum::Option)
        {
            return Err(GenericAliasExpansionError::Environment(
                "foreign standard Option",
            ));
        }
        self.wrap_derived_root(
            ResolvedGenericTypeKind::Declared {
                declaration: option.id(),
                arguments: vec![self.root],
            },
            limits,
        )
    }
    // Retain exact checked element ownership when deriving collection arrays.
    pub(super) fn wrap_array(
        &self,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        self.wrap_derived_root(
            ResolvedGenericTypeKind::Array { element: self.root },
            limits,
        )
    }
    // Derive MapEntry<K,V> plus its array from a checked opaque Map receiver.
    pub(super) fn map_entry_array(
        &self,
        entry: &super::CheckedGenericRecord,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        if !self.environment.same_environment(entry.environment())
            || !entry.syntax().is_standard_map_entry()
        {
            return Err(GenericAliasExpansionError::Environment(
                "foreign standard MapEntry",
            ));
        }
        let ResolvedGenericTypeKind::Declared {
            declaration,
            arguments,
        } = &self.nodes[self.root].kind
        else {
            return Err(GenericAliasExpansionError::Environment(
                "entry receiver is not nominal",
            ));
        };
        if arguments.len() != 2
            || !self
                .environment
                .registry()
                .header(*declaration)
                .is_some_and(|h| h.kind == GenericTypeDeclarationKind::Collection)
        {
            return Err(GenericAliasExpansionError::Environment(
                "entry receiver is not Map",
            ));
        }
        // One bounded worker admits both generated wrappers and all retained proof nodes.
        self.wrap_derived_roots(
            &[
                ResolvedGenericTypeKind::Declared {
                    declaration: entry.id(),
                    arguments: arguments.clone(),
                },
                ResolvedGenericTypeKind::Array {
                    element: self.nodes.len(),
                },
            ],
            limits,
        )
    }
    // Derive the sealed Byte edge of Bytes without textual names or foreign owners.
    pub(super) fn bytes_element_type(
        &self,
        byte: &super::CheckedGenericData,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        if !self.environment.same_environment(byte.environment()) {
            return Err(GenericAliasExpansionError::Environment(
                "foreign Bytes element schema",
            ));
        }
        let ResolvedGenericTypeKind::Declared {
            declaration,
            arguments,
        } = &self.nodes[self.root].kind
        else {
            return Err(GenericAliasExpansionError::Environment(
                "Bytes receiver is not nominal",
            ));
        };
        if !arguments.is_empty()
            || !self
                .environment
                .registry()
                .header(*declaration)
                .is_some_and(|h| {
                    h.kind == GenericTypeDeclarationKind::Data(super::GenericDataTypeKind::Bytes)
                })
        {
            return Err(GenericAliasExpansionError::Environment(
                "Bytes receiver identity",
            ));
        }
        self.wrap_derived_root(
            ResolvedGenericTypeKind::Declared {
                declaration: byte.id(),
                arguments: Vec::new(),
            },
            limits,
        )
    }
    pub(super) fn data_result_type(
        &self,
        byte: &super::CheckedGenericData,
        error: &super::CheckedGenericRecord,
        result: &super::CheckedGenericEnum,
        limits: GenericAliasExpansionLimits,
        success_kind: DataResultKind,
        json: Option<&super::CheckedGenericEnum>,
    ) -> Result<Self, GenericAliasExpansionError> {
        if !self.environment.same_environment(byte.environment())
            || !self.environment.same_environment(error.environment())
            || !self.environment.same_environment(result.environment())
            || byte.syntax().kind()
                != if success_kind == DataResultKind::Bytes {
                    super::GenericDataTypeKind::Bytes
                } else {
                    super::GenericDataTypeKind::Byte
                }
            || !error.syntax().is_standard_error()
            || result.syntax().standard_identity() != Some(super::GenericStandardEnum::Result)
            || !matches!(&self.nodes[self.root].kind,ResolvedGenericTypeKind::Declared{declaration,arguments} if *declaration==error.id()&&arguments.is_empty())
        {
            return Err(GenericAliasExpansionError::Environment(
                "data Result schema ownership",
            ));
        }
        let root = if success_kind == DataResultKind::JsonValue {
            let json = json.ok_or(GenericAliasExpansionError::Environment(
                "JSON schema absent",
            ))?;
            if !self.environment.same_environment(json.environment())
                || json.syntax().standard_identity() != Some(super::GenericStandardEnum::JsonValue)
            {
                return Err(GenericAliasExpansionError::Environment(
                    "JSON schema ownership",
                ));
            }
            ResolvedGenericTypeKind::Declared {
                declaration: json.id(),
                arguments: Vec::new(),
            }
        } else if success_kind == DataResultKind::String {
            ResolvedGenericTypeKind::Primitive(super::GenericPrimitive::String)
        } else {
            ResolvedGenericTypeKind::Declared {
                declaration: byte.id(),
                arguments: Vec::new(),
            }
        };
        let mut roots = vec![root];
        let mut success = self.nodes.len();
        if success_kind == DataResultKind::ByteArray {
            roots.push(ResolvedGenericTypeKind::Array { element: success });
            success += 1;
        }
        roots.push(ResolvedGenericTypeKind::Declared {
            declaration: result.id(),
            arguments: vec![success, self.root],
        });
        self.wrap_derived_roots(&roots, limits)
    }
    fn wrap_derived_root(
        &self,
        kind: ResolvedGenericTypeKind,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        self.wrap_derived_roots(&[kind], limits)
    }
    fn wrap_derived_roots(
        &self,
        kinds: &[ResolvedGenericTypeKind],
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        let mut worker = Worker {
            environment: &self.environment,
            scope: &self.parameters,
            limits,
            work: limits.max_work,
            nodes: Vec::new(),
            depths: Vec::new(),
            edges: 0,
        };
        let mut origins = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            worker.step()?;
            worker.push(node.kind.clone(), node.origin)?;
            origins.push(self.input_origin(index).clone());
        }
        let mut root = self.root;
        for kind in kinds {
            worker.step()?;
            root = worker.push(kind.clone(), self.nodes[self.root].origin)?;
            origins.push(self.input_origin(self.root).clone());
        }
        Ok(Self {
            environment: self.environment.clone(),
            syntax: self.syntax.clone(),
            parameters: self.parameters.clone(),
            nodes: worker.nodes.into(),
            root,
            edges: worker.edges,
            work: limits.max_work - worker.work,
            input_origins: Some(origins.into()),
        })
    }
    // Compose an already-checked field with its validated nominal receiver.
    pub(super) fn substitute_field(
        &self,
        template: &Self,
        declaration: GenericTypeDeclarationId,
        parameters: &GenericParameters,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        if !Arc::ptr_eq(&self.environment.0, &template.environment.0) {
            return Err(GenericAliasExpansionError::Environment(
                "foreign field environment",
            ));
        }
        let ResolvedGenericTypeKind::Declared {
            declaration: actual,
            arguments,
        } = &self.nodes[self.root].kind
        else {
            return Err(GenericAliasExpansionError::Environment(
                "field receiver is not nominal",
            ));
        };
        if *actual != declaration || arguments.len() != parameters.parameters().len() {
            return Err(GenericAliasExpansionError::Environment(
                "field receiver identity or arity",
            ));
        }
        let mut worker = Worker {
            environment: &self.environment,
            scope: &self.parameters,
            limits,
            work: limits.max_work,
            nodes: Vec::new(),
            depths: Vec::new(),
            edges: 0,
        };
        let mut origins = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            worker.step()?;
            worker.push(node.kind.clone(), node.origin)?;
            origins.push(self.input_origin(index).clone());
        }
        // Charge temporary postorder mapping entries before reserving storage.
        charge(&mut worker.work, template.nodes.len(), "field mapping work")?;
        let mut mapping = Vec::with_capacity(template.nodes.len());
        for (index, node) in template.nodes.iter().enumerate() {
            worker.step()?;
            if let ResolvedGenericTypeKind::Parameter(parameter) = node.kind {
                if parameters.parameter(parameter).is_none() {
                    return Err(GenericAliasExpansionError::Environment(
                        "foreign field parameter",
                    ));
                }
                mapping.push(arguments[parameter.ordinal()]);
                continue;
            }
            let kind = match &node.kind {
                ResolvedGenericTypeKind::Primitive(p) => ResolvedGenericTypeKind::Primitive(*p),
                ResolvedGenericTypeKind::Array { element } => ResolvedGenericTypeKind::Array {
                    element: mapping[*element],
                },
                ResolvedGenericTypeKind::Declared {
                    declaration,
                    arguments,
                } => {
                    charge(
                        &mut worker.work,
                        arguments.len(),
                        "field argument mapping work",
                    )?;
                    ResolvedGenericTypeKind::Declared {
                        declaration: *declaration,
                        arguments: arguments.iter().map(|index| mapping[*index]).collect(),
                    }
                }
                ResolvedGenericTypeKind::Parameter(_) => unreachable!(),
            };
            let root = worker.push(kind, node.origin)?;
            origins.push(template.input_origin(index).clone());
            mapping.push(root);
        }
        Ok(Self {
            environment: self.environment.clone(),
            syntax: self.syntax.clone(),
            parameters: self.parameters.clone(),
            nodes: worker.nodes.into(),
            root: mapping[template.root],
            edges: worker.edges,
            work: limits.max_work - worker.work,
            input_origins: Some(origins.into()),
        })
    }
    // Only used after sealed callee type/bound matching. Copy all eager argument
    // arenas and replace exact callee-owned parameter nodes in this annotation.
    pub(super) fn substitute_arguments(
        &self,
        parameters: &GenericParameters,
        arguments: &[Self],
        caller: &GenericParameters,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        if parameters.parameters().len() != arguments.len() {
            return Err(GenericAliasExpansionError::Environment(
                "signature substitution arity",
            ));
        }
        let mut worker = Worker {
            environment: &self.environment,
            scope: caller,
            limits,
            work: limits.max_work,
            nodes: Vec::new(),
            depths: Vec::new(),
            edges: 0,
        };
        let mut origins = Vec::new();
        let mut roots = Vec::new();
        for argument in arguments {
            worker.step()?;
            if !self.environment.same_environment(&argument.environment) {
                return Err(GenericAliasExpansionError::Environment(
                    "foreign signature argument environment",
                ));
            }
            let offset = worker.nodes.len();
            for (index, node) in argument.nodes.iter().enumerate() {
                worker.step()?;
                let kind = match &node.kind {
                    ResolvedGenericTypeKind::Primitive(p) => ResolvedGenericTypeKind::Primitive(*p),
                    ResolvedGenericTypeKind::Parameter(id) => {
                        if caller.parameter(*id).is_none() {
                            return Err(GenericAliasExpansionError::Environment(
                                "foreign caller parameter",
                            ));
                        }
                        ResolvedGenericTypeKind::Parameter(*id)
                    }
                    ResolvedGenericTypeKind::Array { element } => ResolvedGenericTypeKind::Array {
                        element: offset + element,
                    },
                    ResolvedGenericTypeKind::Declared {
                        declaration,
                        arguments,
                    } => {
                        charge(
                            &mut worker.work,
                            arguments.len(),
                            "signature argument mapping work",
                        )?;
                        ResolvedGenericTypeKind::Declared {
                            declaration: *declaration,
                            arguments: arguments.iter().map(|child| offset + child).collect(),
                        }
                    }
                };
                worker.push(kind, node.origin)?;
                origins.push(argument.input_origin(index).clone());
            }
            roots.push(offset + argument.root);
        }
        charge(&mut worker.work, self.nodes.len(), "signature mapping work")?;
        let mut mapping = Vec::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            worker.step()?;
            if let ResolvedGenericTypeKind::Parameter(parameter) = node.kind {
                if parameters.parameter(parameter).is_none() {
                    return Err(GenericAliasExpansionError::Environment(
                        "foreign callee parameter",
                    ));
                }
                mapping.push(roots[parameter.ordinal()]);
                continue;
            }
            let kind = match &node.kind {
                ResolvedGenericTypeKind::Primitive(p) => ResolvedGenericTypeKind::Primitive(*p),
                ResolvedGenericTypeKind::Array { element } => ResolvedGenericTypeKind::Array {
                    element: mapping[*element],
                },
                ResolvedGenericTypeKind::Declared {
                    declaration,
                    arguments,
                } => {
                    charge(
                        &mut worker.work,
                        arguments.len(),
                        "signature annotation mapping work",
                    )?;
                    ResolvedGenericTypeKind::Declared {
                        declaration: *declaration,
                        arguments: arguments.iter().map(|child| mapping[*child]).collect(),
                    }
                }
                ResolvedGenericTypeKind::Parameter(_) => unreachable!(),
            };
            let root = worker.push(kind, node.origin)?;
            origins.push(self.input_origin(index).clone());
            mapping.push(root);
        }
        Ok(Self {
            environment: self.environment.clone(),
            syntax: self.syntax.clone(),
            parameters: caller.clone(),
            nodes: worker.nodes.into(),
            root: mapping[self.root],
            edges: worker.edges,
            work: limits.max_work - worker.work,
            input_origins: Some(origins.into()),
        })
    }
    // Copy only the explicit root's semantic closure. Used by concrete body
    // rechecking when a parameter-independent field retains an unused owner from
    // its declaration arena. Reachable foreign parameters remain visible and
    // must still fail the following exact-owner substitution.
    pub(super) fn reachable_projection(
        &self,
        limits: GenericAliasExpansionLimits,
    ) -> Result<Self, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        let mut worker = Worker {
            environment: &self.environment,
            scope: &self.parameters,
            limits,
            work: limits.max_work,
            nodes: Vec::new(),
            depths: Vec::new(),
            edges: 0,
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut pending = vec![self.root];
        let mut visit_edges = limits.max_edges;
        while let Some(index) = pending.pop() {
            worker.step()?;
            if seen.contains(&index) {
                continue;
            }
            if seen.len() >= limits.max_nodes {
                return Err(GenericAliasExpansionError::Limit("projection nodes"));
            }
            seen.insert(index);
            let children: &[usize] = match &self.nodes[index].kind {
                ResolvedGenericTypeKind::Array { element } => std::slice::from_ref(element),
                ResolvedGenericTypeKind::Declared { arguments, .. } => arguments,
                _ => &[],
            };
            charge(&mut visit_edges, children.len(), "projection edges")?;
            charge(&mut worker.work, children.len(), "projection work")?;
            pending.extend(children.iter().copied());
        }
        let mut mapping = BTreeMap::new();
        let mut origins = Vec::new();
        for index in seen {
            worker.step()?;
            let node = &self.nodes[index];
            let kind = match &node.kind {
                ResolvedGenericTypeKind::Primitive(p) => ResolvedGenericTypeKind::Primitive(*p),
                ResolvedGenericTypeKind::Parameter(p) => ResolvedGenericTypeKind::Parameter(*p),
                ResolvedGenericTypeKind::Array { element } => ResolvedGenericTypeKind::Array {
                    element: mapping[element],
                },
                ResolvedGenericTypeKind::Declared {
                    declaration,
                    arguments,
                } => {
                    charge(&mut worker.work, arguments.len(), "projection mapping work")?;
                    ResolvedGenericTypeKind::Declared {
                        declaration: *declaration,
                        arguments: arguments.iter().map(|child| mapping[child]).collect(),
                    }
                }
            };
            let target = worker.push(kind, node.origin)?;
            mapping.insert(index, target);
            origins.push(self.input_origin(index).clone());
        }
        Ok(Self {
            environment: self.environment.clone(),
            syntax: self.syntax.clone(),
            parameters: self.parameters.clone(),
            root: mapping[&self.root],
            nodes: worker.nodes.into(),
            edges: worker.edges,
            work: limits.max_work - worker.work,
            input_origins: Some(origins.into()),
        })
    }
    // Project only an already-validated array edge. Retain the complete arena,
    // provenance and admission history; no public arbitrary-root constructor.
    pub(super) fn array_element(&self) -> Option<Self> {
        let ResolvedGenericTypeKind::Array { element } = self.nodes[self.root].kind else {
            return None;
        };
        let mut projected = self.clone();
        projected.root = element;
        Some(projected)
    }
    // Project only a validated nominal argument of the expected declaration.
    // The original caller's arena, parameter owner and provenance remain intact.
    pub(super) fn declaration_argument(
        &self,
        expected: GenericTypeDeclarationId,
        ordinal: usize,
    ) -> Option<Self> {
        let ResolvedGenericTypeKind::Declared {
            declaration,
            arguments,
        } = &self.nodes[self.root].kind
        else {
            return None;
        };
        if *declaration != expected {
            return None;
        }
        let root = *arguments.get(ordinal)?;
        let mut projected = self.clone();
        projected.root = root;
        Some(projected)
    }
    /// Retained alias/header owner.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
    /// Primary input annotation owner; composed node origins use `node_source`.
    pub fn syntax(&self) -> &GenericTypeSyntax {
        &self.syntax
    }
    /// Original input parameter owner.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    /// Generated arena; unused eager argument nodes can precede the root.
    pub fn nodes(&self) -> &[ExpandedGenericTypeNode] {
        &self.nodes
    }
    /// Explicit arena root, which need not be the final generated node.
    pub fn root(&self) -> usize {
        self.root
    }
    /// Generated retained child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Consumed logical input/expansion steps.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    n: usize,
    dimension: &'static str,
) -> Result<(), GenericAliasExpansionError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericAliasExpansionError::Limit(dimension))?;
    Ok(())
}
fn check_depth(depth: usize) -> Result<(), GenericAliasExpansionError> {
    if depth > 128 {
        return Err(GenericAliasExpansionError::Limit(
            "fixed alias depth ceiling",
        ));
    }
    Ok(())
}
/// Attach every alias target to its exact registered parameter owner, reject cycles
/// and validate symbolic alias targets under cumulative generated/work limits.
pub fn prepare_generic_aliases(
    registry: &GenericTypeRegistry,
    aliases: &[&GenericAliasSyntax],
    limits: GenericAliasEnvironmentLimits,
) -> Result<GenericAliasEnvironment, GenericAliasExpansionError> {
    check_depth(limits.max_depth)?;
    check_depth(limits.expansion.max_depth)?;
    if aliases.len() > limits.max_aliases {
        return Err(GenericAliasExpansionError::Limit("alias definitions"));
    }
    let mut work = limits.max_work;
    let mut nodes_left = limits.max_nodes;
    let mut edges_left = limits.max_edges;
    let mut source_left = limits.max_source_bytes;
    let mut names_left = limits.max_name_bytes;
    let mut definitions = BTreeMap::new();
    for alias in aliases {
        let mut found = None;
        for header in registry.headers() {
            charge(&mut work, 1, "alias environment work")?;
            if header.name.as_ref() == alias.name()
                && (!registry.is_package_scoped()
                    || header.parameters.scope_key() == alias.parameters().scope_key())
            {
                found = Some(header);
                break;
            }
        }
        let header = found.ok_or(GenericAliasExpansionError::Environment(
            "alias header is absent",
        ))?;
        if header.kind != GenericTypeDeclarationKind::Alias {
            return Err(GenericAliasExpansionError::Environment(
                "alias target belongs to a non-alias header",
            ));
        }
        if definitions.contains_key(&header.id.ordinal()) {
            return Err(GenericAliasExpansionError::Environment(
                "duplicate alias target",
            ));
        }
        if header.parameters.parameters().len() != alias.parameters().parameters().len() {
            return Err(GenericAliasExpansionError::Environment(
                "foreign alias parameter owner",
            ));
        }
        for (a, b) in header
            .parameters
            .parameters()
            .iter()
            .zip(alias.parameters().parameters())
        {
            charge(&mut work, 1, "alias environment work")?;
            if a.id != b.id {
                return Err(GenericAliasExpansionError::Environment(
                    "foreign alias parameter owner",
                ));
            }
        }
        let syntax = alias.target().syntax();
        let doubled = syntax
            .nodes()
            .len()
            .checked_mul(2)
            .ok_or(GenericAliasExpansionError::Limit("alias metadata nodes"))?;
        charge(&mut nodes_left, doubled, "alias metadata nodes")?;
        charge(
            &mut edges_left,
            syntax
                .edges()
                .checked_mul(2)
                .ok_or(GenericAliasExpansionError::Limit("alias metadata edges"))?,
            "alias metadata edges",
        )?;
        charge(
            &mut source_left,
            syntax.source().len(),
            "alias target source",
        )?;
        charge(&mut names_left, syntax.name_bytes(), "alias target names")?;
        let mut input = limits.expansion.input;
        input.max_work = input.max_work.min(work);
        let (nodes, used) = registry
            .resolve_nodes(syntax, alias.parameters(), input, false)
            .map_err(|error| GenericAliasExpansionError::Resolution {
                alias: Some(header.id),
                error,
            })?;
        charge(&mut work, used, "alias environment work")?;
        definitions.insert(
            header.id.ordinal(),
            Definition {
                parameters: alias.parameters().clone(),
                syntax: syntax.clone(),
                nodes: nodes.into(),
            },
        );
    }
    for header in registry.headers() {
        charge(&mut work, 1, "alias environment work")?;
        if header.kind == GenericTypeDeclarationKind::Alias
            && !definitions.contains_key(&header.id.ordinal())
        {
            return Err(GenericAliasExpansionError::Environment(
                "registered alias target is missing",
            ));
        }
    }
    let mut states = BTreeMap::new();
    for &root in definitions.keys() {
        charge(&mut work, 1, "alias cycle work")?;
        if states.get(&root) == Some(&2) {
            continue;
        }
        if limits.max_depth == 0 {
            return Err(GenericAliasExpansionError::Limit("alias dependency depth"));
        }
        states.insert(root, 1);
        let mut stack = vec![(root, 0usize)];
        while let Some((alias, next)) = stack.last_mut() {
            charge(&mut work, 1, "alias cycle work")?;
            let definition = &definitions[alias];
            if *next == definition.nodes.len() {
                states.insert(*alias, 2);
                stack.pop();
                continue;
            }
            let node = &definition.nodes[*next];
            *next += 1;
            if let ResolvedGenericTypeKind::Declared { declaration, .. } = node {
                if registry.header(*declaration).expect("resolved header").kind
                    == GenericTypeDeclarationKind::Alias
                {
                    let target = declaration.ordinal();
                    match states.get(&target) {
                        Some(1) => return Err(GenericAliasExpansionError::Cycle(*declaration)),
                        Some(2) => {}
                        _ => {
                            if stack.len() >= limits.max_depth {
                                return Err(GenericAliasExpansionError::Limit(
                                    "alias dependency depth",
                                ));
                            }
                            states.insert(target, 1);
                            stack.push((target, 0));
                        }
                    }
                }
            }
        }
    }
    let environment = GenericAliasEnvironment(Arc::new(Environment {
        registry: registry.clone(),
        definitions,
        validation_work: 0,
        validation_nodes: 0,
        validation_edges: 0,
    }));
    let mut generated_nodes = limits.max_validation_nodes;
    let mut generated_edges = limits.max_validation_edges;
    for (&ordinal, definition) in &environment.0.definitions {
        let mut expansion = limits.expansion;
        expansion.max_work = expansion.max_work.min(work);
        expansion.max_nodes = expansion.max_nodes.min(generated_nodes);
        expansion.max_edges = expansion.max_edges.min(generated_edges);
        let alias = environment.0.registry.headers()[ordinal].id;
        let run = run(
            &environment,
            &definition.syntax,
            &definition.parameters,
            &definition.nodes,
            Some(alias),
            expansion,
        )?;
        charge(&mut work, run.work, "alias environment work")?;
        charge(
            &mut generated_nodes,
            run.nodes.len(),
            "alias validation nodes",
        )?;
        charge(&mut generated_edges, run.edges, "alias validation edges")?;
    }
    let mut environment = environment;
    let data = Arc::get_mut(&mut environment.0).expect("unpublished environment");
    data.validation_work = limits.max_work - work;
    data.validation_nodes = limits.max_validation_nodes - generated_nodes;
    data.validation_edges = limits.max_validation_edges - generated_edges;
    Ok(environment)
}
impl GenericAliasEnvironment {
    pub(super) fn same_environment(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// Registered root header owner.
    pub fn registry(&self) -> &GenericTypeRegistry {
        &self.0.registry
    }
    /// Exact target-fragment source selected by an owned alias identity.
    pub fn alias_source(&self, id: GenericTypeDeclarationId) -> Option<&str> {
        let header = self.0.registry.header(id)?;
        self.0
            .definitions
            .get(&header.id.ordinal())
            .map(|d| d.syntax.source())
    }
    /// Logical preparation, cycle and symbolic-validation work.
    pub fn validation_work(&self) -> usize {
        self.0.validation_work
    }
    /// Generated nodes across symbolic target validation.
    pub fn validation_nodes(&self) -> usize {
        self.0.validation_nodes
    }
    /// Generated edges across symbolic target validation.
    pub fn validation_edges(&self) -> usize {
        self.0.validation_edges
    }
    /// Expand transparent aliases, substituting argument roots by identity and
    /// proving Key constraints against their expanded forms. No source rewriting.
    pub fn expand(
        &self,
        syntax: &GenericTypeSyntax,
        parameters: &GenericParameters,
        limits: GenericAliasExpansionLimits,
    ) -> Result<ExpandedGenericType, GenericAliasExpansionError> {
        check_depth(limits.max_depth)?;
        let mut input = limits.input;
        input.max_work = input.max_work.min(limits.max_work);
        let (heads, used) = self
            .0
            .registry
            .resolve_nodes(syntax, parameters, input, false)
            .map_err(|error| GenericAliasExpansionError::Resolution { alias: None, error })?;
        let mut remaining = limits;
        charge(&mut remaining.max_work, used, "expansion work")?;
        let run = run(self, syntax, parameters, &heads, None, remaining)?;
        Ok(ExpandedGenericType {
            environment: self.clone(),
            syntax: syntax.clone(),
            parameters: parameters.clone(),
            nodes: run.nodes.into(),
            root: run.root,
            edges: run.edges,
            work: used + run.work,
            input_origins: None,
        })
    }
}
struct Run {
    nodes: Vec<ExpandedGenericTypeNode>,
    root: usize,
    edges: usize,
    work: usize,
}
#[derive(Clone, Copy)]
struct Frame<'a> {
    syntax: &'a GenericTypeSyntax,
    heads: &'a [ResolvedGenericTypeKind],
    parameters: &'a GenericParameters,
    bindings: Option<&'a [usize]>,
    alias: Option<GenericTypeDeclarationId>,
}
struct Worker<'a> {
    environment: &'a GenericAliasEnvironment,
    scope: &'a GenericParameters,
    limits: GenericAliasExpansionLimits,
    work: usize,
    nodes: Vec<ExpandedGenericTypeNode>,
    depths: Vec<usize>,
    edges: usize,
}
impl Worker<'_> {
    fn step(&mut self) -> Result<(), GenericAliasExpansionError> {
        charge(&mut self.work, 1, "expansion work")
    }
    fn push(
        &mut self,
        kind: ResolvedGenericTypeKind,
        origin: GenericExpansionOrigin,
    ) -> Result<usize, GenericAliasExpansionError> {
        if self.nodes.len() >= self.limits.max_nodes {
            return Err(GenericAliasExpansionError::Limit("expanded nodes"));
        }
        let children: &[usize] = match &kind {
            ResolvedGenericTypeKind::Declared { arguments, .. } => arguments,
            ResolvedGenericTypeKind::Array { element } => std::slice::from_ref(element),
            _ => &[],
        };
        let mut depth = 1;
        for &child in children {
            self.step()?;
            depth = depth.max(self.depths[child] + 1);
        }
        if depth > self.limits.max_depth {
            return Err(GenericAliasExpansionError::Limit(
                "expanded structural depth",
            ));
        }
        let edges = self
            .edges
            .checked_add(children.len())
            .filter(|n| *n <= self.limits.max_edges)
            .ok_or(GenericAliasExpansionError::Limit("expanded edges"))?;
        self.edges = edges;
        let index = self.nodes.len();
        self.nodes.push(ExpandedGenericTypeNode { kind, origin });
        self.depths.push(depth);
        Ok(index)
    }
    fn walk(
        &mut self,
        frame: Frame<'_>,
        index: usize,
        depth: usize,
    ) -> Result<usize, GenericAliasExpansionError> {
        let Frame {
            syntax,
            heads,
            parameters,
            bindings,
            alias,
        } = frame;
        self.step()?;
        if depth > self.limits.max_depth {
            return Err(GenericAliasExpansionError::Limit("alias expansion depth"));
        }
        let origin = GenericExpansionOrigin {
            alias,
            span: syntax.nodes()[index].span,
        };
        match &heads[index] {
            ResolvedGenericTypeKind::Primitive(p) => {
                self.push(ResolvedGenericTypeKind::Primitive(*p), origin)
            }
            ResolvedGenericTypeKind::Parameter(id) => {
                if let Some(bindings) = bindings {
                    if parameters.parameter(*id).is_none() {
                        return Err(GenericAliasExpansionError::Environment(
                            "unavailable substitution owner",
                        ));
                    }
                    bindings.get(id.ordinal()).copied().ok_or(
                        GenericAliasExpansionError::Environment(
                            "unavailable substitution argument",
                        ),
                    )
                } else {
                    self.push(ResolvedGenericTypeKind::Parameter(*id), origin)
                }
            }
            ResolvedGenericTypeKind::Array { element } => {
                let element = self.walk(frame, *element, depth + 1)?;
                self.push(ResolvedGenericTypeKind::Array { element }, origin)
            }
            ResolvedGenericTypeKind::Declared {
                declaration,
                arguments,
            } => {
                let header = self
                    .environment
                    .0
                    .registry
                    .header(*declaration)
                    .ok_or(GenericAliasExpansionError::Environment(
                        "foreign declaration head",
                    ))?
                    .clone();
                let mut expanded = Vec::new();
                for &argument in arguments {
                    self.step()?;
                    expanded.push(self.walk(frame, argument, depth + 1)?);
                }
                for (&argument, parameter) in expanded.iter().zip(header.parameters.parameters()) {
                    self.step()?;
                    if parameter.bound == Some(GenericParameterBound::Key) {
                        let allowed = match self.nodes[argument].kind {
                            ResolvedGenericTypeKind::Primitive(
                                GenericPrimitive::Int
                                | GenericPrimitive::Bool
                                | GenericPrimitive::String,
                            ) => true,
                            ResolvedGenericTypeKind::Declared { declaration, .. }
                                if super::generic_data_types::is_byte_header(
                                    self.environment.registry(),
                                    declaration,
                                ) =>
                            {
                                true
                            }
                            ResolvedGenericTypeKind::Parameter(id) => self
                                .scope
                                .parameter(id)
                                .is_some_and(|p| p.bound == Some(GenericParameterBound::Key)),
                            _ => false,
                        };
                        if !allowed {
                            return Err(GenericAliasExpansionError::Bound(
                                self.nodes[argument].origin,
                            ));
                        }
                    }
                }
                if header.kind == GenericTypeDeclarationKind::Alias {
                    let definition = self
                        .environment
                        .0
                        .definitions
                        .get(&declaration.ordinal())
                        .ok_or(GenericAliasExpansionError::Environment(
                            "alias definition is absent",
                        ))?
                        .clone();
                    self.walk(
                        Frame {
                            syntax: &definition.syntax,
                            heads: &definition.nodes,
                            parameters: &definition.parameters,
                            bindings: Some(&expanded),
                            alias: Some(*declaration),
                        },
                        definition.nodes.len() - 1,
                        depth + 1,
                    )
                } else {
                    self.push(
                        ResolvedGenericTypeKind::Declared {
                            declaration: *declaration,
                            arguments: expanded,
                        },
                        origin,
                    )
                }
            }
        }
    }
}
fn run(
    environment: &GenericAliasEnvironment,
    syntax: &GenericTypeSyntax,
    parameters: &GenericParameters,
    heads: &[ResolvedGenericTypeKind],
    alias: Option<GenericTypeDeclarationId>,
    limits: GenericAliasExpansionLimits,
) -> Result<Run, GenericAliasExpansionError> {
    check_depth(limits.max_depth)?;
    let mut worker = Worker {
        environment,
        scope: parameters,
        limits,
        work: limits.max_work,
        nodes: Vec::new(),
        depths: Vec::new(),
        edges: 0,
    };
    let root = worker.walk(
        Frame {
            syntax,
            heads,
            parameters,
            bindings: None,
            alias,
        },
        heads.len() - 1,
        1,
    )?;
    Ok(Run {
        nodes: worker.nodes,
        root,
        edges: worker.edges,
        work: limits.max_work - worker.work,
    })
}
