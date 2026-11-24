//! Finite concrete call-graph discovery from sealed symbolic bodies, not executable code.
use super::{
    generic_type_key, CheckedGenericCopyBodies, ExpandedGenericType, GenericAliasExpansionError,
    GenericAliasExpansionLimits, GenericCopyExpressionId, GenericCopyExpressionKind,
    GenericSignatureId, GenericTypeEquivalenceError, GenericTypeKey, GenericTypeKeyLimits,
    ResolvedGenericTypeKind,
};
use std::sync::Arc;

/// Cumulative concrete graph admission, independent of subsequent code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSpecializationLimits {
    /// Distinct root/function/type-argument instances.
    pub max_instances: usize,
    /// Retained call-site edges, including repeated and recursive calls.
    pub max_calls: usize,
    /// Concrete type arguments generated across all visited call sites, including duplicates.
    pub max_arguments: usize,
    /// Generated substitution nodes, including unused eager argument nodes.
    pub max_nodes: usize,
    /// Generated substitution edges.
    pub max_edges: usize,
    /// Generated normalized key nodes, including duplicate candidates.
    pub max_key_nodes: usize,
    /// Generated normalized key edges.
    pub max_key_edges: usize,
    /// Graph traversal, substitution, concreteness, normalization and exact cache lookup work.
    pub max_work: usize,
    /// Structural type depth; must be in 1..=128.
    pub max_depth: usize,
    /// Independent per-argument substitution profile, capped by remaining totals.
    pub expansion: GenericAliasExpansionLimits,
    /// Independent per-argument exact key profile, capped by remaining totals.
    pub keys: GenericTypeKeyLimits,
}
impl GenericSpecializationLimits {
    /// Approved standalone instance cap with finite generated metadata/work.
    pub const DEFAULT: Self = Self {
        max_instances: 1024,
        max_calls: 32_768,
        max_arguments: 16_384,
        max_nodes: 262_144,
        max_edges: 524_288,
        max_key_nodes: 262_144,
        max_key_edges: 524_288,
        max_work: 4_000_000,
        max_depth: 128,
        expansion: GenericAliasExpansionLimits::DEFAULT,
        keys: GenericTypeKeyLimits::DEFAULT,
    };
    /// Approved package instance cap; this API itself does not load/link packages.
    pub const PACKAGE: Self = Self {
        max_instances: 4096,
        ..Self::DEFAULT
    };
}
/// Graph discovery refuses completely, never publishing a partial closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericSpecializationError {
    /// Typed postfix bodies require concrete/lowering integration before discovery.
    PropagationUnavailable,
    /// Selected admission dimension exhausted or structural profile invalid.
    Limit(&'static str),
    /// Root is foreign/absent or requires explicit type arguments.
    Root,
    /// Concrete substitution retained an opaque parameter.
    NonConcrete,
    /// Exact caller substitution failed, retaining annotation provenance.
    Substitution(GenericAliasExpansionError),
    /// Exact normalized key construction failed.
    Key(GenericTypeEquivalenceError),
}
impl std::fmt::Display for GenericSpecializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic specialization discovery refused: {self:?}")
    }
}
impl std::error::Error for GenericSpecializationError {}
/// One source call in a concrete instance. Indices are local to the enclosing graph.
#[derive(Debug, Clone)]
pub struct GenericSpecializationCall {
    /// Expression identity in the retained symbolic body arena.
    pub expression: GenericCopyExpressionId,
    /// Concrete target instance index, possibly this instance or an earlier one.
    pub target: usize,
}
/// One concrete argument vector for an original function, without executable body claims.
#[derive(Debug, Clone)]
pub struct GenericSpecializationInstance {
    function: GenericSignatureId,
    arguments: Arc<[ExpandedGenericType]>,
    calls: Vec<GenericSpecializationCall>,
}
impl GenericSpecializationInstance {
    /// Original exact signature-table identity.
    pub fn function(&self) -> GenericSignatureId {
        self.function
    }
    /// Concrete source-order type arguments with original/alias provenance.
    pub fn arguments(&self) -> &[ExpandedGenericType] {
        &self.arguments
    }
    /// All checked source call sites, including statically unreachable sites.
    pub fn calls(&self) -> &[GenericSpecializationCall] {
        &self.calls
    }
}
/// Measured discovery usage; duplicate call candidates are charged before cache reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GenericSpecializationUsage {
    /// Retained instances, including the nongeneric root.
    pub instances: usize,
    /// Retained call-site edges.
    pub calls: usize,
    /// Generated concrete argument records.
    pub arguments: usize,
    /// Generated substitution nodes.
    pub nodes: usize,
    /// Generated substitution edges.
    pub edges: usize,
    /// Generated normalized key nodes.
    pub key_nodes: usize,
    /// Generated normalized key edges.
    pub key_edges: usize,
    /// Consumed logical work.
    pub work: usize,
}
/// Sealed finite reachable call closure, not rechecked concrete bodies or runnable IR.
#[derive(Debug, Clone)]
pub struct CheckedGenericSpecializations {
    bodies: CheckedGenericCopyBodies,
    instances: Arc<[GenericSpecializationInstance]>,
    usage: GenericSpecializationUsage,
}
impl CheckedGenericSpecializations {
    /// Retained complete symbolic body proofs and their exact source/table owners.
    pub fn bodies(&self) -> &CheckedGenericCopyBodies {
        &self.bodies
    }
    /// Deterministic discovery order; the supplied nongeneric root is index zero.
    pub fn instances(&self) -> &[GenericSpecializationInstance] {
        &self.instances
    }
    /// Cumulative discovery usage.
    pub fn usage(&self) -> GenericSpecializationUsage {
        self.usage
    }
}
fn charge(
    left: &mut usize,
    n: usize,
    dimension: &'static str,
) -> Result<(), GenericSpecializationError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericSpecializationError::Limit(dimension))?;
    Ok(())
}
// All generated nodes are admitted before allocating this flat depth memo. Checking
// eager nodes too prevents hidden symbolic owners from entering the concrete cache.
pub(super) fn concrete(
    ty: &ExpandedGenericType,
    depth: usize,
    work: &mut usize,
) -> Result<(), GenericSpecializationError> {
    let mut depths = Vec::with_capacity(ty.nodes().len());
    for node in ty.nodes() {
        charge(work, 1, "specialization work")?;
        let children: &[usize] = match &node.kind {
            ResolvedGenericTypeKind::Parameter(_) => {
                return Err(GenericSpecializationError::NonConcrete)
            }
            ResolvedGenericTypeKind::Primitive(_) => &[],
            ResolvedGenericTypeKind::Array { element } => std::slice::from_ref(element),
            ResolvedGenericTypeKind::Declared { arguments, .. } => arguments,
        };
        charge(work, children.len(), "specialization work")?;
        let current = children
            .iter()
            .map(|child| depths[*child])
            .max()
            .unwrap_or(0usize)
            + 1;
        if current > depth {
            return Err(GenericSpecializationError::Limit(
                "specialization type depth",
            ));
        }
        depths.push(current);
    }
    Ok(())
}
/// Discover the complete concrete call closure from a checked nongeneric root.
/// Every supplied function was checked symbolically already. All source calls in
/// each discovered instance are traversed; branch reachability is not optimized.
/// Identical normalized argument vectors reuse an instance before processing it,
/// allowing direct/mutual recursion while bounding expanding recursion. No legacy
/// fallback, concrete operation recheck, code emission or execution occurs here.
pub fn discover_generic_specializations(
    bodies: &CheckedGenericCopyBodies,
    root: GenericSignatureId,
    limits: GenericSpecializationLimits,
) -> Result<CheckedGenericSpecializations, GenericSpecializationError> {
    discover_mode(bodies, root, limits, false)
}
/// Discover a finite closure containing opt-in symbolic postfix facts. This
/// authorizes concrete rechecking only, not executable lowering.
pub fn discover_generic_propagation_specializations(
    bodies: &CheckedGenericCopyBodies,
    root: GenericSignatureId,
    limits: GenericSpecializationLimits,
) -> Result<CheckedGenericSpecializations, GenericSpecializationError> {
    discover_mode(bodies, root, limits, true)
}
fn discover_mode(
    bodies: &CheckedGenericCopyBodies,
    root: GenericSignatureId,
    limits: GenericSpecializationLimits,
    propagation: bool,
) -> Result<CheckedGenericSpecializations, GenericSpecializationError> {
    if !propagation
        && bodies
            .propagation_usage()
            .is_some_and(|usage| usage.sites > 0)
    {
        return Err(GenericSpecializationError::PropagationUnavailable);
    }
    let root_body = bodies.body(root).ok_or(GenericSpecializationError::Root)?;
    let scope = root_body.signature().syntax().parameters();
    if !scope.parameters().is_empty() {
        return Err(GenericSpecializationError::Root);
    }
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(GenericSpecializationError::Limit(
            "specialization depth profile",
        ));
    }
    let mut left = limits;
    charge(&mut left.max_instances, 1, "specialization instances")?;
    charge(&mut left.max_work, 1, "specialization work")?;
    let mut instances = vec![GenericSpecializationInstance {
        function: root,
        arguments: Arc::from([]),
        calls: Vec::new(),
    }];
    // Exact flat keys with measured worst-case linear lookup, avoiding hash-only
    // identity and charging equality work even on duplicate/colliding candidates.
    let mut cache: Vec<(Vec<GenericTypeKey>, usize)> = vec![(Vec::new(), 0)];
    let mut cursor = 0;
    while cursor < instances.len() {
        charge(&mut left.max_work, 1, "specialization work")?;
        let body = bodies
            .body(instances[cursor].function)
            .expect("sealed instance owner");
        for expression in body.expressions() {
            charge(&mut left.max_work, 1, "specialization work")?;
            let GenericCopyExpressionKind::Call { signature, .. } = &expression.kind else {
                continue;
            };
            charge(&mut left.max_calls, 1, "specialization calls")?;
            let mut arguments = Vec::new();
            let mut keys = Vec::new();
            let mut weight = 0;
            for template in signature.binding().arguments().arguments() {
                charge(&mut left.max_arguments, 1, "specialization arguments")?;
                let expansion = GenericAliasExpansionLimits {
                    max_nodes: limits.expansion.max_nodes.min(left.max_nodes),
                    max_edges: limits.expansion.max_edges.min(left.max_edges),
                    max_work: limits.expansion.max_work.min(left.max_work),
                    max_depth: limits.expansion.max_depth.min(limits.max_depth),
                    ..limits.expansion
                };
                let ty = template
                    .substitute_arguments(
                        body.signature().syntax().parameters(),
                        &instances[cursor].arguments,
                        scope,
                        expansion,
                    )
                    .map_err(GenericSpecializationError::Substitution)?;
                charge(
                    &mut left.max_nodes,
                    ty.nodes().len(),
                    "specialization nodes",
                )?;
                charge(&mut left.max_edges, ty.edges(), "specialization edges")?;
                charge(&mut left.max_work, ty.work(), "specialization work")?;
                concrete(&ty, limits.max_depth, &mut left.max_work)?;
                let key = generic_type_key(
                    &ty,
                    GenericTypeKeyLimits {
                        max_nodes: limits.keys.max_nodes.min(left.max_key_nodes),
                        max_edges: limits.keys.max_edges.min(left.max_key_edges),
                        max_work: limits.keys.max_work.min(left.max_work),
                        ..limits.keys
                    },
                )
                .map_err(GenericSpecializationError::Key)?;
                charge(
                    &mut left.max_key_nodes,
                    key.nodes(),
                    "specialization key nodes",
                )?;
                charge(
                    &mut left.max_key_edges,
                    key.edges(),
                    "specialization key edges",
                )?;
                charge(&mut left.max_work, key.work(), "specialization work")?;
                weight = key
                    .nodes()
                    .checked_add(key.edges())
                    .and_then(|n| n.checked_add(weight))
                    .ok_or(GenericSpecializationError::Limit(
                        "specialization key weight",
                    ))?;
                keys.push(key.key().clone());
                arguments.push(ty);
            }
            let callee = signature.binding().callee();
            let mut target = None;
            for (index, (existing, existing_weight)) in cache.iter().enumerate() {
                charge(&mut left.max_work, 1, "specialization work")?;
                if instances[index].function != callee {
                    continue;
                }
                charge(&mut left.max_work, weight, "specialization work")?;
                charge(&mut left.max_work, *existing_weight, "specialization work")?;
                if existing == &keys {
                    target = Some(index);
                    break;
                }
            }
            let target = if let Some(target) = target {
                target
            } else {
                charge(&mut left.max_instances, 1, "specialization instances")?;
                let index = instances.len();
                instances.push(GenericSpecializationInstance {
                    function: callee,
                    arguments: arguments.into(),
                    calls: Vec::new(),
                });
                cache.push((keys, weight));
                index
            };
            instances[cursor].calls.push(GenericSpecializationCall {
                expression: expression.id,
                target,
            });
        }
        cursor += 1;
    }
    Ok(CheckedGenericSpecializations {
        bodies: bodies.clone(),
        instances: instances.into(),
        usage: GenericSpecializationUsage {
            instances: limits.max_instances - left.max_instances,
            calls: limits.max_calls - left.max_calls,
            arguments: limits.max_arguments - left.max_arguments,
            nodes: limits.max_nodes - left.max_nodes,
            edges: limits.max_edges - left.max_edges,
            key_nodes: limits.max_key_nodes - left.max_key_nodes,
            key_edges: limits.max_key_edges - left.max_key_edges,
            work: limits.max_work - left.max_work,
        },
    })
}
