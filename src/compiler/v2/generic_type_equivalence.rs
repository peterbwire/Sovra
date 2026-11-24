//! Bounded semantic comparison and normalized in-process keys for expanded type DAGs.
use super::{
    ExpandedGenericType, GenericParameterId, GenericPrimitive, GenericTypeDeclarationId,
    ResolvedGenericTypeKind,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};
/// Independent logical comparison admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericTypeEquivalenceLimits {
    /// Distinct source-node pairs retained in the comparison memo.
    pub max_pairs: usize,
    /// Queued child-pair edges, including repeated source pairs.
    pub max_edges: usize,
    /// Pair visits and edge-queue operations; not CPU time.
    pub max_work: usize,
}
impl GenericTypeEquivalenceLimits {
    /// Finite default comparison profile.
    pub const DEFAULT: Self = Self {
        max_pairs: 4096,
        max_edges: 8192,
        max_work: 32_768,
    };
}
/// Logical exhaustion returns no equality decision or partial key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericTypeEquivalenceError {
    /// Selected admission dimension exhausted.
    Limit(&'static str),
}
impl std::fmt::Display for GenericTypeEquivalenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic type equivalence refused: {self:?}")
    }
}
impl std::error::Error for GenericTypeEquivalenceError {}
/// Complete exact-type decision with measured logical admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericTypeComparison {
    equal: bool,
    pairs: usize,
    edges: usize,
    work: usize,
}
impl GenericTypeComparison {
    /// Exact semantic equality, without implicit numeric or nominal conversions.
    pub fn equivalent(self) -> bool {
        self.equal
    }
    /// Distinct visited source-node pairs.
    pub fn pairs(self) -> usize {
        self.pairs
    }
    /// Queued child-pair edges.
    pub fn edges(self) -> usize {
        self.edges
    }
    /// Logical comparison operations.
    pub fn work(self) -> usize {
        self.work
    }
}
fn charge(
    left: &mut usize,
    n: usize,
    dimension: &'static str,
) -> Result<(), GenericTypeEquivalenceError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericTypeEquivalenceError::Limit(dimension))?;
    Ok(())
}
/// Compare explicit roots of alias-expanded immutable graphs. Shared versus
/// duplicated subgraphs and unused eager argument nodes do not affect equality.
/// Nominal declaration/parameter owners remain significant across contexts.
pub fn compare_generic_types(
    left: &ExpandedGenericType,
    right: &ExpandedGenericType,
    limits: GenericTypeEquivalenceLimits,
) -> Result<GenericTypeComparison, GenericTypeEquivalenceError> {
    if limits.max_pairs == 0 {
        return Err(GenericTypeEquivalenceError::Limit("comparison pairs"));
    }
    let mut work = limits.max_work;
    let mut edges_left = limits.max_edges;
    let mut pairs = BTreeSet::new();
    let mut pending = vec![(left.root(), right.root())];
    let mut equal = true;
    while let Some((a, b)) = pending.pop() {
        charge(&mut work, 1, "comparison work")?;
        if pairs.contains(&(a, b)) {
            continue;
        }
        if pairs.len() >= limits.max_pairs {
            return Err(GenericTypeEquivalenceError::Limit("comparison pairs"));
        }
        pairs.insert((a, b));
        match (&left.nodes()[a].kind, &right.nodes()[b].kind) {
            (ResolvedGenericTypeKind::Primitive(a), ResolvedGenericTypeKind::Primitive(b)) => {
                equal = a == b
            }
            (ResolvedGenericTypeKind::Parameter(a), ResolvedGenericTypeKind::Parameter(b)) => {
                equal = a == b
            }
            (
                ResolvedGenericTypeKind::Array { element: a },
                ResolvedGenericTypeKind::Array { element: b },
            ) => {
                charge(&mut edges_left, 1, "comparison edges")?;
                charge(&mut work, 1, "comparison work")?;
                pending.push((*a, *b));
            }
            (
                ResolvedGenericTypeKind::Declared {
                    declaration: a,
                    arguments: aa,
                },
                ResolvedGenericTypeKind::Declared {
                    declaration: b,
                    arguments: ba,
                },
            ) => {
                if a != b || aa.len() != ba.len() {
                    equal = false;
                } else {
                    charge(&mut edges_left, aa.len(), "comparison edges")?;
                    charge(&mut work, aa.len(), "comparison work")?;
                    pending.extend(aa.iter().zip(ba).rev().map(|(a, b)| (*a, *b)));
                }
            }
            _ => equal = false,
        }
        if !equal {
            break;
        }
    }
    Ok(GenericTypeComparison {
        equal,
        pairs: pairs.len(),
        edges: limits.max_edges - edges_left,
        work: limits.max_work - work,
    })
}
/// Independent normalization/cache-key construction admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericTypeKeyLimits {
    /// Distinct visited source nodes retained in the memo.
    pub max_visited_nodes: usize,
    /// Distinct retained canonical nodes and interning entries.
    pub max_nodes: usize,
    /// Retained canonical child edges.
    pub max_edges: usize,
    /// Pending visit/finish frames, also bounding candidate argument width.
    pub max_pending: usize,
    /// Visit/finish, enqueue and child-normalization steps.
    pub max_work: usize,
}
impl GenericTypeKeyLimits {
    /// Finite default normalization profile.
    pub const DEFAULT: Self = Self {
        max_visited_nodes: 4096,
        max_nodes: 4096,
        max_edges: 8192,
        max_pending: 8192,
        max_work: 65_536,
    };
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum KeyNode {
    Primitive(GenericPrimitive),
    Parameter(GenericParameterId),
    Declared(GenericTypeDeclarationId, Vec<usize>),
    Array(usize),
}
/// Exact structural type key for in-process compiler maps, not a persistent codec.
/// Opaque context IDs are retained; this is not alpha-equivalence or runtime type evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericTypeKey {
    nodes: Arc<[Arc<KeyNode>]>,
    root: usize,
}
/// Complete key with separate construction statistics; statistics do not affect equality.
#[derive(Debug, Clone)]
pub struct GenericTypeKeyResult {
    key: GenericTypeKey,
    visited: usize,
    edges: usize,
    work: usize,
    peak_pending: usize,
}
impl GenericTypeKeyResult {
    /// Exact normalized key, cheap to clone through immutable shared storage.
    pub fn key(&self) -> &GenericTypeKey {
        &self.key
    }
    /// Distinct source nodes used by the explicit root.
    pub fn visited_nodes(&self) -> usize {
        self.visited
    }
    /// Canonical retained nodes, with repeated structural subtypes interned.
    pub fn nodes(&self) -> usize {
        self.key.nodes.len()
    }
    /// Canonical child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Logical normalization operations.
    pub fn work(&self) -> usize {
        self.work
    }
    /// Maximum simultaneous visit/finish frames.
    pub fn peak_pending(&self) -> usize {
        self.peak_pending
    }
}
/// Normalize only the explicit root's reachable structure in deterministic
/// left-to-right postorder. Interning ignores graph sharing/provenance, while
/// preserving nominal/parameter owner IDs and ordered type arguments.
/// No hash-only equality, textual type spelling, alias expansion or source rewrite is used.
pub fn generic_type_key(
    ty: &ExpandedGenericType,
    limits: GenericTypeKeyLimits,
) -> Result<GenericTypeKeyResult, GenericTypeEquivalenceError> {
    if limits.max_visited_nodes == 0 {
        return Err(GenericTypeEquivalenceError::Limit("key visited nodes"));
    }
    if limits.max_pending == 0 {
        return Err(GenericTypeEquivalenceError::Limit("key pending frames"));
    }
    let mut work = limits.max_work;
    let mut edges_left = limits.max_edges;
    let mut memo = BTreeMap::new();
    let mut intern: HashMap<Arc<KeyNode>, usize> = HashMap::new();
    let mut nodes = Vec::new();
    let mut pending = vec![(ty.root(), false)];
    let mut peak_pending = 1;
    while let Some((index, finish)) = pending.pop() {
        charge(&mut work, 1, "key work")?;
        if memo.contains_key(&index) {
            continue;
        }
        let kind = &ty.nodes()[index].kind;
        let children: &[usize] = match kind {
            ResolvedGenericTypeKind::Array { element } => std::slice::from_ref(element),
            ResolvedGenericTypeKind::Declared { arguments, .. } => arguments,
            _ => &[],
        };
        if !finish {
            let needed = children
                .len()
                .checked_add(1)
                .and_then(|n| n.checked_add(pending.len()))
                .filter(|n| *n <= limits.max_pending)
                .ok_or(GenericTypeEquivalenceError::Limit("key pending frames"))?;
            charge(
                &mut work,
                children
                    .len()
                    .checked_add(1)
                    .ok_or(GenericTypeEquivalenceError::Limit("key work"))?,
                "key work",
            )?;
            pending.push((index, true));
            pending.extend(children.iter().rev().map(|child| (*child, false)));
            peak_pending = peak_pending.max(needed);
            continue;
        }
        if memo.len() >= limits.max_visited_nodes {
            return Err(GenericTypeEquivalenceError::Limit("key visited nodes"));
        }
        charge(&mut work, children.len(), "key work")?;
        let candidate = match kind {
            ResolvedGenericTypeKind::Primitive(p) => KeyNode::Primitive(*p),
            ResolvedGenericTypeKind::Parameter(id) => KeyNode::Parameter(*id),
            ResolvedGenericTypeKind::Array { element } => KeyNode::Array(memo[element]),
            ResolvedGenericTypeKind::Declared {
                declaration,
                arguments,
            } => KeyNode::Declared(
                *declaration,
                arguments.iter().map(|child| memo[child]).collect(),
            ),
        };
        let canonical = if let Some(index) = intern.get(&candidate) {
            *index
        } else {
            if nodes.len() >= limits.max_nodes {
                return Err(GenericTypeEquivalenceError::Limit("key canonical nodes"));
            }
            charge(&mut edges_left, children.len(), "key canonical edges")?;
            let canonical = nodes.len();
            let node = Arc::new(candidate);
            intern.insert(Arc::clone(&node), canonical);
            nodes.push(node);
            canonical
        };
        memo.insert(index, canonical);
    }
    let root = memo[&ty.root()];
    Ok(GenericTypeKeyResult {
        key: GenericTypeKey {
            nodes: nodes.into(),
            root,
        },
        visited: memo.len(),
        edges: limits.max_edges - edges_left,
        work: limits.max_work - work,
        peak_pending,
    })
}
