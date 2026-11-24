//! Bounded backend-neutral layouts for closed generic types; no physical ABI or execution.
use super::{
    generic_type_key, CheckedGenericDefinition, CheckedGenericLiterals, ConcreteGenericBody,
    ExpandedGenericType, GenericAliasExpansionError, GenericAliasExpansionLimits,
    GenericCopyBindingId, GenericCopyExpressionId, GenericCopyType, GenericParameterBound,
    GenericPrimitive, GenericSpecializationError, GenericTypeDeclarationId,
    GenericTypeEquivalenceError, GenericTypeKey, GenericTypeKeyLimits, ResolvedGenericTypeKind,
};
use std::sync::Arc;
/// Cumulative logical layout discovery admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericLayoutLimits {
    /// Combined function instances already discovered and new nominal type instances.
    pub max_instantiations: usize,
    /// Unique primitive, array and nominal layout records.
    pub max_layouts: usize,
    /// Record fields and enum variants, including nullary variants.
    pub max_members: usize,
    /// Array-element, nominal-argument, field and payload layout edges.
    pub max_edges: usize,
    /// All source type mappings and child interning requests, including cache hits.
    pub max_references: usize,
    /// Generated field/payload substitution nodes, including eager unused nodes.
    pub max_generated_nodes: usize,
    /// Generated field/payload substitution edges.
    pub max_generated_edges: usize,
    /// Generated canonical key nodes, including duplicate candidates.
    pub max_key_nodes: usize,
    /// Generated canonical key edges.
    pub max_key_edges: usize,
    /// Input visits, substitution, key construction, exact lookup and layout work.
    pub max_work: usize,
    /// Structural type depth in 1..=128; layout graph cycles do not expand this depth.
    pub max_depth: usize,
    /// Independent admitted input/generated type arena node cap, before depth memo allocation.
    pub max_nodes_per_type: usize,
    /// Independent admitted input/generated type arena edge cap.
    pub max_edges_per_type: usize,
    /// Independent member substitution profile, reduced by remaining totals.
    pub expansion: GenericAliasExpansionLimits,
    /// Independent exact key profile, reduced by remaining totals.
    pub keys: GenericTypeKeyLimits,
}
impl GenericLayoutLimits {
    /// Approved standalone combined instantiation cap with finite layout metadata.
    pub const DEFAULT: Self = Self {
        max_instantiations: 1024,
        max_layouts: 16_384,
        max_members: 65_536,
        max_edges: 131_072,
        max_references: 262_144,
        max_generated_nodes: 262_144,
        max_generated_edges: 524_288,
        max_key_nodes: 262_144,
        max_key_edges: 524_288,
        max_work: 8_000_000,
        max_depth: 128,
        max_nodes_per_type: 65_536,
        max_edges_per_type: 131_072,
        expansion: GenericAliasExpansionLimits::DEFAULT,
        keys: GenericTypeKeyLimits::DEFAULT,
    };
    /// Approved combined package cap; this API does not load/link packages.
    pub const PACKAGE: Self = Self {
        max_instantiations: 4096,
        ..Self::DEFAULT
    };
}
/// Complete refusal without a partial or unresolved layout table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericLayoutError {
    /// Selected allowance exhausted or depth profile invalid.
    Limit(&'static str),
    /// Concrete type/depth validation refused.
    Concrete(GenericSpecializationError),
    /// Exact normalized type key construction refused.
    Key(GenericTypeEquivalenceError),
    /// A concrete field/payload substitution failed with original provenance.
    Member {
        /// Original nominal declaration.
        declaration: GenericTypeDeclarationId,
        /// Original field/variant index.
        member: usize,
        /// Exact source/alias substitution refusal.
        error: Box<GenericAliasExpansionError>,
    },
    /// Retained type identity, arity or constraint is inconsistent.
    Type(&'static str),
}
impl std::fmt::Display for GenericLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic layout discovery refused: {self:?}")
    }
}
impl std::error::Error for GenericLayoutError {}
/// Backend-neutral logical storage shape. Every index is local to its enclosing table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericRuntimeLayoutKind {
    /// Existing primitive value contract, with no selected machine representation.
    Primitive(GenericPrimitive),
    /// Opaque checked byte scalar, distinct from Int and user records.
    Byte {
        /// Compiler-owned standard declaration identity.
        declaration: GenericTypeDeclarationId,
    },
    /// Opaque immutable binary buffer with one exact Byte layout edge.
    Bytes {
        /// Compiler-owned standard declaration identity.
        declaration: GenericTypeDeclarationId,
        /// Checked element layout, never Int or a same-named record.
        byte: usize,
    },
    /// Ordered owned sequence of values of the referenced element layout.
    Array {
        /// Element layout index, possibly participating in nominal recursion.
        element: usize,
    },
    /// Opaque intrinsic map; no source-visible record fields or enum variants.
    Map {
        /// Compiler-owned standard collection identity.
        declaration: GenericTypeDeclarationId,
        /// Checked primitive key layout.
        key: usize,
        /// Exact value layout, possibly part of a recursive nominal graph.
        value: usize,
    },
    /// Opaque intrinsic set over an admitted primitive key layout.
    Set {
        /// Compiler-owned standard collection identity.
        declaration: GenericTypeDeclarationId,
        /// Checked primitive key layout.
        key: usize,
    },
    /// Nominal record with declaration-order field layouts.
    Record {
        /// Exact original declaration identity.
        declaration: GenericTypeDeclarationId,
        /// Source-order concrete type-argument layout indices.
        arguments: Vec<usize>,
        /// Original field index maps directly to this layout vector.
        fields: Vec<usize>,
    },
    /// Nominal enum with declaration-order tags and distinct nullary/Unit payloads.
    Enum {
        /// Exact original declaration identity.
        declaration: GenericTypeDeclarationId,
        /// Source-order concrete type-argument layout indices.
        arguments: Vec<usize>,
        /// Original variant index is the tag; None is nullary, Some is a payload layout.
        variants: Vec<Option<usize>>,
    },
}
/// Complete closed logical layout and its first-discovered concrete type/provenance.
#[derive(Debug, Clone)]
pub struct GenericRuntimeLayout {
    ty: GenericCopyType,
    kind: GenericRuntimeLayoutKind,
}
impl GenericRuntimeLayout {
    /// First concrete representative; exact type identity ignores unused eager nodes.
    pub fn ty(&self) -> &GenericCopyType {
        &self.ty
    }
    /// Resolved shape with graph-local child layout indices.
    pub fn kind(&self) -> &GenericRuntimeLayoutKind {
        &self.kind
    }
}
/// Concrete source type facts mapped into one shared layout table.
#[derive(Debug, Clone)]
pub struct GenericInstanceLayouts {
    body: ConcreteGenericBody,
    type_arguments: Arc<[usize]>,
    parameters: Arc<[usize]>,
    result: usize,
    bindings: Arc<[usize]>,
    expressions: Arc<[usize]>,
}
impl GenericInstanceLayouts {
    /// Retained original concrete body/identity proof.
    pub fn body(&self) -> &ConcreteGenericBody {
        &self.body
    }
    /// Concrete function type-argument layouts, including phantom arguments.
    pub fn type_arguments(&self) -> &[usize] {
        &self.type_arguments
    }
    /// Value parameter layouts in source order.
    pub fn parameter_layouts(&self) -> &[usize] {
        &self.parameters
    }
    /// Return value layout, including implicit Unit.
    pub fn return_layout(&self) -> usize {
        self.result
    }
    /// Binding layout only for this original source body arena.
    pub fn binding_layout(&self, id: GenericCopyBindingId) -> Option<usize> {
        self.body.body().binding(id)?;
        self.bindings.get(id.ordinal()).copied()
    }
    /// Expression layout only for this original source body arena.
    pub fn expression_layout(&self, id: GenericCopyExpressionId) -> Option<usize> {
        self.body.body().expression(id)?;
        self.expressions.get(id.ordinal()).copied()
    }
}
/// Exact logical discovery usage, including duplicate lookup candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericLayoutUsage {
    /// Existing function instances plus discovered nominal type instances.
    pub instantiations: usize,
    /// Unique layout records.
    pub layouts: usize,
    /// Record fields and enum variants.
    pub members: usize,
    /// Retained child-layout references.
    pub edges: usize,
    /// Type lookup requests, including cache hits.
    pub references: usize,
    /// Generated member substitution nodes.
    pub generated_nodes: usize,
    /// Generated member substitution edges.
    pub generated_edges: usize,
    /// Generated canonical key nodes.
    pub key_nodes: usize,
    /// Generated canonical key edges.
    pub key_edges: usize,
    /// Consumed logical work.
    pub work: usize,
}
/// Complete finite logical layouts with retained source/value/type proofs, not runnable IR.
#[derive(Debug, Clone)]
pub struct CheckedGenericLayouts {
    collections: bool,
    data: bool,
    literals: CheckedGenericLiterals,
    layouts: Arc<[GenericRuntimeLayout]>,
    bodies: Arc<[GenericInstanceLayouts]>,
    usage: GenericLayoutUsage,
}
impl CheckedGenericLayouts {
    /// Whether discovered layouts contain Byte/Bytes, including phantom arguments.
    /// Construction/IR/VM admission remains independently gated.
    pub fn has_data(&self) -> bool {
        self.data
    }
    /// Whether the complete discovered graph contains opaque collection layouts.
    /// This requires separate collection operation/IR/VM admission before execution.
    pub fn has_collections(&self) -> bool {
        self.collections
    }
    /// Original source literal/concrete/body/specialization proofs.
    pub fn literals(&self) -> &CheckedGenericLiterals {
        &self.literals
    }
    /// Complete layout table; child indices refer only to this table.
    pub fn layouts(&self) -> &[GenericRuntimeLayout] {
        &self.layouts
    }
    /// Original instance-order source fact mappings.
    pub fn bodies(&self) -> &[GenericInstanceLayouts] {
        &self.bodies
    }
    /// Inspect an enclosing-table layout index; indices have no cross-table identity contract.
    pub fn layout(&self, index: usize) -> Option<&GenericRuntimeLayout> {
        self.layouts.get(index)
    }
    /// Exact cumulative usage.
    pub fn usage(&self) -> GenericLayoutUsage {
        self.usage
    }
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<(), GenericLayoutError> {
    *left = left.checked_sub(n).ok_or(GenericLayoutError::Limit(name))?;
    Ok(())
}
#[derive(PartialEq, Eq)]
enum Key {
    Primitive(GenericPrimitive),
    Compound(GenericTypeKey),
}
struct Builder {
    limits: GenericLayoutLimits,
    left: GenericLayoutLimits,
    types: Vec<GenericCopyType>,
    keys: Vec<(Key, usize)>,
    shapes: Vec<Option<GenericRuntimeLayoutKind>>,
}
impl Builder {
    fn step(&mut self) -> Result<(), GenericLayoutError> {
        charge(&mut self.left.max_work, 1, "layout work")
    }
    fn intern(&mut self, ty: GenericCopyType) -> Result<usize, GenericLayoutError> {
        charge(&mut self.left.max_references, 1, "layout references")?;
        self.step()?;
        if let Some(expanded) = ty.expanded() {
            if expanded.nodes().len() > self.limits.max_nodes_per_type
                || expanded.edges() > self.limits.max_edges_per_type
            {
                return Err(GenericLayoutError::Limit("layout type arena"));
            }
            super::generic_specialization::concrete(
                expanded,
                self.limits.max_depth,
                &mut self.left.max_work,
            )
            .map_err(GenericLayoutError::Concrete)?;
        }
        let (key, weight) = if let Some(primitive) = ty.primitive() {
            (Key::Primitive(primitive), 1)
        } else {
            let key = generic_type_key(
                ty.expanded()
                    .ok_or(GenericLayoutError::Type("missing expanded type"))?,
                GenericTypeKeyLimits {
                    max_nodes: self.limits.keys.max_nodes.min(self.left.max_key_nodes),
                    max_edges: self.limits.keys.max_edges.min(self.left.max_key_edges),
                    max_work: self.limits.keys.max_work.min(self.left.max_work),
                    ..self.limits.keys
                },
            )
            .map_err(GenericLayoutError::Key)?;
            charge(
                &mut self.left.max_key_nodes,
                key.nodes(),
                "layout key nodes",
            )?;
            charge(
                &mut self.left.max_key_edges,
                key.edges(),
                "layout key edges",
            )?;
            charge(&mut self.left.max_work, key.work(), "layout work")?;
            let weight = key
                .nodes()
                .checked_add(key.edges())
                .ok_or(GenericLayoutError::Limit("layout key weight"))?;
            (Key::Compound(key.key().clone()), weight)
        };
        for (index, (existing, existing_weight)) in self.keys.iter().enumerate() {
            charge(&mut self.left.max_work, 1, "layout work")?;
            charge(&mut self.left.max_work, weight, "layout work")?;
            charge(&mut self.left.max_work, *existing_weight, "layout work")?;
            if *existing == key {
                return Ok(index);
            }
        }
        charge(&mut self.left.max_layouts, 1, "layout records")?;
        if let Some(expanded) = ty.expanded() {
            if matches!(
                expanded.nodes()[expanded.root()].kind,
                ResolvedGenericTypeKind::Declared { .. }
            ) {
                charge(
                    &mut self.left.max_instantiations,
                    1,
                    "layout instantiations",
                )?;
            }
        }
        let index = self.types.len();
        self.types.push(ty);
        self.keys.push((key, weight));
        self.shapes.push(None);
        Ok(index)
    }
    fn edge(&mut self, ty: GenericCopyType) -> Result<usize, GenericLayoutError> {
        charge(&mut self.left.max_edges, 1, "layout edges")?;
        self.intern(ty)
    }
    fn byte_edge(
        &mut self,
        receiver: &ExpandedGenericType,
        byte: &super::CheckedGenericData,
    ) -> Result<usize, GenericLayoutError> {
        self.step()?;
        let limits = GenericAliasExpansionLimits {
            max_nodes: self
                .limits
                .expansion
                .max_nodes
                .min(self.left.max_generated_nodes)
                .min(self.limits.max_nodes_per_type),
            max_edges: self
                .limits
                .expansion
                .max_edges
                .min(self.left.max_generated_edges)
                .min(self.limits.max_edges_per_type),
            max_work: self.limits.expansion.max_work.min(self.left.max_work),
            max_depth: self.limits.expansion.max_depth.min(self.limits.max_depth),
            ..self.limits.expansion
        };
        let ty = receiver.bytes_element_type(byte, limits).map_err(|error| {
            GenericLayoutError::Member {
                declaration: byte.id(),
                member: 0,
                error: Box::new(error),
            }
        })?;
        charge(
            &mut self.left.max_generated_nodes,
            ty.nodes().len(),
            "layout generated nodes",
        )?;
        charge(
            &mut self.left.max_generated_edges,
            ty.edges(),
            "layout generated edges",
        )?;
        charge(&mut self.left.max_work, ty.work(), "layout work")?;
        self.edge(GenericCopyType::concrete_expanded(ty))
    }
    fn member(
        &mut self,
        receiver: &ExpandedGenericType,
        template: &ExpandedGenericType,
        declaration: GenericTypeDeclarationId,
        member: usize,
        parameters: &super::GenericParameters,
    ) -> Result<usize, GenericLayoutError> {
        self.step()?;
        if template.nodes().len() > self.limits.max_nodes_per_type
            || template.edges() > self.limits.max_edges_per_type
        {
            return Err(GenericLayoutError::Limit("layout member type arena"));
        }
        let expansion = GenericAliasExpansionLimits {
            max_nodes: self
                .limits
                .expansion
                .max_nodes
                .min(self.left.max_generated_nodes)
                .min(self.limits.max_nodes_per_type),
            max_edges: self
                .limits
                .expansion
                .max_edges
                .min(self.left.max_generated_edges)
                .min(self.limits.max_edges_per_type),
            max_work: self.limits.expansion.max_work.min(self.left.max_work),
            max_depth: self.limits.expansion.max_depth.min(self.limits.max_depth),
            ..self.limits.expansion
        };
        let ty = receiver
            .substitute_field(template, declaration, parameters, expansion)
            .map_err(|error| GenericLayoutError::Member {
                declaration,
                member,
                error: Box::new(error),
            })?;
        charge(
            &mut self.left.max_generated_nodes,
            ty.nodes().len(),
            "layout generated nodes",
        )?;
        charge(
            &mut self.left.max_generated_edges,
            ty.edges(),
            "layout generated edges",
        )?;
        charge(&mut self.left.max_work, ty.work(), "layout work")?;
        self.edge(GenericCopyType::concrete_expanded(ty))
    }
}
/// Build a complete finite logical layout closure, registering each type before
/// expanding children so nominal recursion forms indexed edges. This does not
/// select byte sizes, alignment, pointer representation, allocation or a native ABI.
/// Expanding recursion, resource exhaustion or inconsistent types refuse without
/// partial tables, unchecked layouts or an execution fallback.
pub fn plan_generic_layouts(
    literals: &CheckedGenericLiterals,
    limits: GenericLayoutLimits,
) -> Result<CheckedGenericLayouts, GenericLayoutError> {
    plan_layouts(literals, limits, false, false)
}
/// Opt-in complete opaque collection layout discovery under the existing cumulative budgets.
/// This authorizes layout metadata only; construction/IR/VM/native admission stays gated.
pub fn plan_generic_collection_layouts(
    literals: &CheckedGenericLiterals,
    limits: GenericLayoutLimits,
) -> Result<CheckedGenericLayouts, GenericLayoutError> {
    plan_layouts(literals, limits, true, false)
}
/// Discover closed Byte/Bytes and collection graphs under cumulative layout budgets.
/// Metadata only: existing construction/IR/VM/native paths refuse data-bearing tables.
pub fn plan_generic_data_layouts(
    literals: &CheckedGenericLiterals,
    limits: GenericLayoutLimits,
) -> Result<CheckedGenericLayouts, GenericLayoutError> {
    plan_layouts(literals, limits, true, true)
}
fn plan_layouts(
    literals: &CheckedGenericLiterals,
    limits: GenericLayoutLimits,
    collections: bool,
    data: bool,
) -> Result<CheckedGenericLayouts, GenericLayoutError> {
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(GenericLayoutError::Limit("layout depth profile"));
    }
    let concrete = literals.concrete();
    let graph = concrete.graph();
    let definitions = graph.bodies().signatures().types();
    let mut b = Builder {
        limits,
        left: limits,
        types: Vec::new(),
        keys: Vec::new(),
        shapes: Vec::new(),
    };
    charge(
        &mut b.left.max_instantiations,
        graph.instances().len(),
        "layout instantiations",
    )?;
    let mut bodies = Vec::new();
    for (index, body) in concrete.bodies().iter().enumerate() {
        b.step()?;
        let type_arguments = graph.instances()[index]
            .arguments()
            .iter()
            .map(|ty| b.intern(GenericCopyType::concrete_expanded(ty.clone())))
            .collect::<Result<Vec<_>, _>>()?;
        let parameters = body
            .parameter_types()
            .iter()
            .map(|ty| b.intern(ty.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let result = b.intern(body.return_type().clone())?;
        let bindings = body
            .body()
            .bindings()
            .iter()
            .map(|binding| {
                b.intern(
                    body.binding_type(binding.id)
                        .expect("sealed binding type")
                        .clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expressions = body
            .body()
            .expressions()
            .iter()
            .map(|expression| {
                b.intern(
                    body.expression_type(expression.id)
                        .expect("sealed expression type")
                        .clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        bodies.push(GenericInstanceLayouts {
            body: body.clone(),
            type_arguments: type_arguments.into(),
            parameters: parameters.into(),
            result,
            bindings: bindings.into(),
            expressions: expressions.into(),
        });
    }
    let mut collection_shapes = false;
    let mut data_shapes = false;
    let mut cursor = 0;
    while cursor < b.types.len() {
        b.step()?;
        let ty = b.types[cursor].clone();
        let kind = if let Some(primitive) = ty.primitive() {
            GenericRuntimeLayoutKind::Primitive(primitive)
        } else {
            let receiver = ty
                .expanded()
                .ok_or(GenericLayoutError::Type("missing layout receiver"))?;
            match &receiver.nodes()[receiver.root()].kind {
                ResolvedGenericTypeKind::Array { .. } => GenericRuntimeLayoutKind::Array {
                    element: b.edge(GenericCopyType::concrete_expanded(
                        receiver.array_element().expect("checked array edge"),
                    ))?,
                },
                ResolvedGenericTypeKind::Declared {
                    declaration,
                    arguments,
                } => {
                    let definition = definitions
                        .definition(*declaration)
                        .ok_or(GenericLayoutError::Type("foreign layout declaration"))?;
                    let parameters = match definition {
                        CheckedGenericDefinition::Record(r) => r.syntax().parameters(),
                        CheckedGenericDefinition::Enum(e) => e.syntax().parameters(),
                        CheckedGenericDefinition::Data(d) if data => d.syntax().parameters(),
                        CheckedGenericDefinition::Data(_) => {
                            return Err(GenericLayoutError::Type(
                                "intrinsic data execution is not enabled",
                            ))
                        }
                        CheckedGenericDefinition::Collection(c) if collections => {
                            c.syntax().parameters()
                        }
                        CheckedGenericDefinition::Collection(_) => {
                            return Err(GenericLayoutError::Type(
                                "intrinsic collection execution is not enabled",
                            ))
                        }
                        _ => return Err(GenericLayoutError::Type("unexpanded layout alias")),
                    };
                    if parameters.parameters().len() != arguments.len() {
                        return Err(GenericLayoutError::Type("layout type arity"));
                    }
                    let mut argument_layouts = Vec::new();
                    for (ordinal, (parameter, argument)) in
                        parameters.parameters().iter().zip(arguments).enumerate()
                    {
                        b.step()?;
                        if parameter.bound == Some(GenericParameterBound::Key)
                            && !matches!(
                                receiver.nodes()[*argument].kind,
                                ResolvedGenericTypeKind::Primitive(
                                    GenericPrimitive::Int
                                        | GenericPrimitive::Bool
                                        | GenericPrimitive::String
                                )
                            )
                            && !matches!(receiver.nodes()[*argument].kind,ResolvedGenericTypeKind::Declared{declaration,..} if super::generic_data_types::is_byte_header(definitions.environment().registry(),declaration))
                        {
                            return Err(GenericLayoutError::Type("layout Key bound"));
                        }
                        argument_layouts.push(
                            b.edge(GenericCopyType::concrete_expanded(
                                receiver
                                    .declaration_argument(*declaration, ordinal)
                                    .expect("sealed nominal argument"),
                            ))?,
                        );
                    }
                    match definition {
                        CheckedGenericDefinition::Data(d) => {
                            data_shapes = true;
                            match d.syntax().kind() {
                                super::GenericDataTypeKind::Byte => {
                                    GenericRuntimeLayoutKind::Byte {
                                        declaration: *declaration,
                                    }
                                }
                                super::GenericDataTypeKind::Bytes => {
                                    let mut byte = None;
                                    for definition in definitions.definitions() {
                                        b.step()?;
                                        if let CheckedGenericDefinition::Data(d) = definition {
                                            if d.syntax().kind() == super::GenericDataTypeKind::Byte
                                            {
                                                byte = Some(d);
                                                break;
                                            }
                                        }
                                    }
                                    let byte = byte.ok_or(GenericLayoutError::Type(
                                        "Bytes element schema unavailable",
                                    ))?;
                                    GenericRuntimeLayoutKind::Bytes {
                                        declaration: *declaration,
                                        byte: b.byte_edge(receiver, byte)?,
                                    }
                                }
                            }
                        }
                        CheckedGenericDefinition::Collection(c) => {
                            collection_shapes = true;
                            match (c.syntax().kind(), argument_layouts.as_slice()) {
                                (super::GenericCollectionLayoutKind::Map, [key, value]) => {
                                    GenericRuntimeLayoutKind::Map {
                                        declaration: *declaration,
                                        key: *key,
                                        value: *value,
                                    }
                                }
                                (super::GenericCollectionLayoutKind::Set, [key]) => {
                                    GenericRuntimeLayoutKind::Set {
                                        declaration: *declaration,
                                        key: *key,
                                    }
                                }
                                _ => {
                                    return Err(GenericLayoutError::Type("collection layout arity"))
                                }
                            }
                        }
                        CheckedGenericDefinition::Record(record) => {
                            charge(
                                &mut b.left.max_members,
                                record.field_types().len(),
                                "layout members",
                            )?;
                            let mut fields = Vec::new();
                            for (index, template) in record.field_types().iter().enumerate() {
                                fields.push(b.member(
                                    receiver,
                                    template,
                                    *declaration,
                                    index,
                                    parameters,
                                )?);
                            }
                            GenericRuntimeLayoutKind::Record {
                                declaration: *declaration,
                                arguments: argument_layouts,
                                fields,
                            }
                        }
                        CheckedGenericDefinition::Enum(definition) => {
                            if definition.syntax().standard_identity()
                                == Some(super::GenericStandardEnum::JsonValue)
                            {
                                if !data {
                                    return Err(GenericLayoutError::Type(
                                        "JSON requires the explicit data layout profile",
                                    ));
                                }
                                data_shapes = true;
                            }
                            charge(
                                &mut b.left.max_members,
                                definition.payload_types().len(),
                                "layout members",
                            )?;
                            let mut variants = Vec::new();
                            for (index, template) in definition.payload_types().iter().enumerate() {
                                b.step()?;
                                variants.push(if let Some(template) = template {
                                    Some(b.member(
                                        receiver,
                                        template,
                                        *declaration,
                                        index,
                                        parameters,
                                    )?)
                                } else {
                                    None
                                });
                            }
                            GenericRuntimeLayoutKind::Enum {
                                declaration: *declaration,
                                arguments: argument_layouts,
                                variants,
                            }
                        }
                        _ => unreachable!("checked nominal category"),
                    }
                }
                _ => return Err(GenericLayoutError::Type("nonconcrete layout root")),
            }
        };
        b.shapes[cursor] = Some(kind);
        cursor += 1;
    }
    let layouts = b
        .types
        .into_iter()
        .zip(b.shapes)
        .map(|(ty, kind)| GenericRuntimeLayout {
            ty,
            kind: kind.expect("complete sealed layout closure"),
        })
        .collect::<Vec<_>>();
    Ok(CheckedGenericLayouts {
        collections: collection_shapes,
        data: data_shapes,
        literals: literals.clone(),
        layouts: layouts.into(),
        bodies: bodies.into(),
        usage: GenericLayoutUsage {
            instantiations: limits.max_instantiations - b.left.max_instantiations,
            layouts: limits.max_layouts - b.left.max_layouts,
            members: limits.max_members - b.left.max_members,
            edges: limits.max_edges - b.left.max_edges,
            references: limits.max_references - b.left.max_references,
            generated_nodes: limits.max_generated_nodes - b.left.max_generated_nodes,
            generated_edges: limits.max_generated_edges - b.left.max_generated_edges,
            key_nodes: limits.max_key_nodes - b.left.max_key_nodes,
            key_edges: limits.max_key_edges - b.left.max_key_edges,
            work: limits.max_work - b.left.max_work,
        },
    })
}

#[cfg(test)]
mod propagation_mapping_tests {
    use super::*;
    use crate::compiler::v2::*;
    fn layouts() -> CheckedGenericLayouts {
        let base =
            compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
        let types = base
            .ir()
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .types();
        let syntax = parse_generic_propagation_function("fn root(x:std::Option<Int>,y:std::Option<String>)->std::Option<Bool>{let a=x?;let b=y?;return std::Option::<Bool>::Some(true);}",GenericFunctionLimits::DEFAULT).unwrap();
        let sig =
            check_generic_signatures(types, &[&syntax], GenericSignatureLimits::DEFAULT).unwrap();
        let bodies = check_generic_propagation_bodies(
            &sig,
            GenericCopyBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
        let graph = discover_generic_propagation_specializations(
            &bodies,
            sig.find("root").unwrap(),
            GenericSpecializationLimits::DEFAULT,
        )
        .unwrap();
        let concrete = recheck_generic_propagation_concrete_bodies(
            &graph,
            GenericConcreteBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
        let literals = decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap();
        plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap()
    }
    #[test]
    fn coherent_operand_and_success_mapping_swap_is_rejected() {
        let mut l = layouts();
        let mut bodies = l.bodies.to_vec();
        let b = &mut bodies[0];
        let p = b.body.propagations();
        let mut expressions = b.expressions.to_vec();
        expressions[p[0].operand().ordinal()] = expressions[p[1].operand().ordinal()];
        expressions[p[0].expression().ordinal()] = expressions[p[1].expression().ordinal()];
        b.expressions = expressions.into();
        l.bodies = bodies.into();
        assert!(matches!(
            plan_generic_propagation_layouts(&l, GenericPropagationLayoutLimits::DEFAULT),
            Err(GenericPropagationLayoutError::Context)
        ));
    }
    #[test]
    fn option_failure_with_injected_payload_is_rejected() {
        let mut l = layouts();
        let p = &l.bodies[0].body.propagations()[0];
        let input = l.bodies[0].expression_layout(p.operand()).unwrap();
        let payload = l.bodies[0].expression_layout(p.expression()).unwrap();
        let mut table = l.layouts.to_vec();
        let GenericRuntimeLayoutKind::Enum { variants, .. } = &mut table[input].kind else {
            panic!("enum")
        };
        variants[1] = Some(payload);
        l.layouts = table.into();
        assert!(matches!(
            plan_generic_propagation_layouts(&l, GenericPropagationLayoutLimits::DEFAULT),
            Err(GenericPropagationLayoutError::Context)
        ));
    }
}
