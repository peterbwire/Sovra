//! Proof-carrying generic value snapshots for future collection adapters.
use super::*;
/// Bounded selection within an outcome or an already owned snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericCollectionSelector {
    /// Declaration-order record field.
    Field(usize),
    /// Array or immutable byte-buffer element.
    Item(usize),
    /// Selected enum payload; nullary variants have none.
    Payload,
    /// Select a map key without exposing its internal entry wrapper.
    MapKey(usize),
    /// Select a map value with its original checked type proof.
    MapValue(usize),
    /// Select an insertion-ordered set member.
    SetItem(usize),
}
/// Refusal without a partial snapshot or a fabricated type/equality decision.
#[derive(Debug, PartialEq, Eq)]
pub enum GenericCollectionError {
    /// Missing field/item/payload or wrong selector kind.
    Missing,
    /// Whole-result node/text admission refused.
    Materialization(RuntimeValueError),
    /// Finite shared helper work refused.
    Work(RuntimeSetError),
    /// Independent exact-type comparison admission refused.
    Type(GenericTypeEquivalenceError),
    /// Sealed value/layout interval could not be retained.
    Invalid,
}
impl std::fmt::Display for GenericCollectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic collection bridge refused: {self:?}")
    }
}
impl std::error::Error for GenericCollectionError {}
impl From<RuntimeValueError> for GenericCollectionError {
    fn from(e: RuntimeValueError) -> Self {
        Self::Materialization(e)
    }
}
impl From<RuntimeSetError> for GenericCollectionError {
    fn from(e: RuntimeSetError) -> Self {
        Self::Work(e)
    }
}
impl From<GenericTypeEquivalenceError> for GenericCollectionError {
    fn from(e: GenericTypeEquivalenceError) -> Self {
        Self::Type(e)
    }
}
/// Checked concrete type retaining its enclosing table; raw layout indices are not identities.
#[derive(Debug, Clone)]
pub struct GenericCollectionType {
    layouts: CheckedGenericLayouts,
    index: usize,
}
impl GenericCollectionType {
    /// Retain a type from an admitted layout table. This never attaches it to a value.
    pub fn from_layout(
        layouts: &CheckedGenericLayouts,
        index: usize,
    ) -> Result<Self, GenericCollectionError> {
        layouts
            .layout(index)
            .ok_or(GenericCollectionError::Invalid)?;
        Ok(Self {
            layouts: layouts.clone(),
            index,
        })
    }
    /// Complete checked layout; child indices remain local to this retained table.
    pub fn layout(&self) -> &GenericRuntimeLayout {
        &self.layouts.layouts()[self.index]
    }
    /// Original source-owned nominal declaration, when this type is nominal.
    pub fn declaration(&self) -> Option<GenericTypeDeclarationId> {
        match self.layout().kind() {
            GenericRuntimeLayoutKind::Byte { declaration }
            | GenericRuntimeLayoutKind::Bytes { declaration, .. }
            | GenericRuntimeLayoutKind::Record { declaration, .. }
            | GenericRuntimeLayoutKind::Map { declaration, .. }
            | GenericRuntimeLayoutKind::Set { declaration, .. }
            | GenericRuntimeLayoutKind::Enum { declaration, .. } => Some(*declaration),
            _ => None,
        }
    }
    /// Whether this exact type satisfies Key; Byte storage admission remains separate.
    pub fn is_key(&self) -> bool {
        matches!(
            self.layout().ty().primitive(),
            Some(GenericPrimitive::Int | GenericPrimitive::Bool | GenericPrimitive::String)
        ) || matches!(self.layout().kind(), GenericRuntimeLayoutKind::Byte { .. })
    }
    /// Exact semantic identity, including nominal owners and all concrete arguments.
    /// Independent finite type-comparison limits apply even for matching primitives.
    pub fn equivalent(
        &self,
        other: &Self,
        limits: GenericTypeEquivalenceLimits,
    ) -> Result<bool, GenericCollectionError> {
        if limits.max_pairs == 0 {
            return Err(GenericTypeEquivalenceError::Limit("comparison pairs").into());
        }
        if limits.max_work == 0 {
            return Err(GenericTypeEquivalenceError::Limit("comparison work").into());
        }
        let (a, b) = (self.layout().ty(), other.layout().ty());
        match (a.primitive(), b.primitive()) {
            (Some(a), Some(b)) => Ok(a == b),
            (Some(_), None) | (None, Some(_)) => Ok(false),
            (None, None) => Ok(compare_generic_types(
                a.expanded().ok_or(GenericCollectionError::Invalid)?,
                b.expanded().ok_or(GenericCollectionError::Invalid)?,
                limits,
            )?
            .equivalent()),
        }
    }
}
/// Owned flat generic snapshot paired with the checked layout proof that created it.
/// No public raw-value/foreign-table constructor or unchecked Clone is provided.
#[derive(Debug)]
pub struct GenericCollectionSnapshot {
    layouts: CheckedGenericLayouts,
    value: Value,
}
impl GenericVmOutcome {
    /// Capture a result subtree, preserving its package/declaration/type-argument owner.
    pub fn collection_snapshot(
        &self,
        path: &[GenericCollectionSelector],
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<GenericCollectionSnapshot, GenericCollectionError> {
        capture(self.ir.plans().layouts(), &self.value, path, fuel, work)
    }
}
impl GenericCollectionSnapshot {
    /// Inspect the immutable value without copying or detaching its type proof.
    pub fn value(&self) -> GenericValueRef<'_> {
        GenericValueRef {
            nodes: &self.value.nodes,
            root: self.value.nodes.len() - 1,
        }
    }
    /// Retain a checked concrete type token; metadata sharing is not a value copy.
    pub fn ty(&self) -> GenericCollectionType {
        GenericCollectionType {
            layouts: self.layouts.clone(),
            index: self.value.root().layout,
        }
    }
    /// Copy a selected subtree with the original proof, never a caller-supplied table.
    pub fn select(
        &self,
        path: &[GenericCollectionSelector],
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, GenericCollectionError> {
        capture(&self.layouts, &self.value, path, fuel, work)
    }
    /// Copy the complete snapshot with full subtree ownership admission.
    pub fn snapshot(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, GenericCollectionError> {
        self.select(&[], fuel, work)
    }
    // Whole-result admission belongs to the trusted storage caller.
    pub(super) fn copy_admitted(&self) -> Self {
        Self {
            layouts: self.layouts.clone(),
            value: Value {
                nodes: self.value.nodes.clone(),
            },
        }
    }
    /// Structural equality after an exact checked root-type comparison.
    /// Compiler-sealed nominal field order permits linear field pairing across tables.
    pub fn equal(
        &self,
        other: &Self,
        limits: GenericTypeEquivalenceLimits,
        work: &mut RuntimeSetWork,
    ) -> Result<bool, GenericCollectionError> {
        work.charge(1)?;
        if !self.ty().equivalent(&other.ty(), limits)? {
            return Ok(false);
        }
        self.equal_typed(other, work)
    }
    // Only storage with a proven matching declared value type may skip root comparison.
    pub(super) fn equal_typed(
        &self,
        other: &Self,
        work: &mut RuntimeSetWork,
    ) -> Result<bool, GenericCollectionError> {
        let (a, b) = (self.value.root(), other.value.root());
        let cost = (a.count as u64)
            .checked_add(b.count as u64)
            .and_then(|n| n.checked_add(a.text))
            .and_then(|n| n.checked_add(b.text))
            .ok_or(GenericCollectionError::Invalid)?;
        work.charge(cost)?;
        let mut pending = vec![(self.value.nodes.len() - 1, other.value.nodes.len() - 1)];
        while let Some((a, b)) = pending.pop() {
            match (&self.value.nodes[a].kind, &other.value.nodes[b].kind) {
                (Kind::Unit, Kind::Unit) => {}
                (Kind::Map(x), Kind::Map(y)) if x.len() == y.len() => {
                    for entry in x {
                        let (k, v) = collection_values::entry(&self.value.nodes, *entry)
                            .ok_or(GenericCollectionError::Invalid)?;
                        let mut found = None;
                        for other_entry in y {
                            let (ok, ov) =
                                collection_values::entry(&other.value.nodes, *other_entry)
                                    .ok_or(GenericCollectionError::Invalid)?;
                            let cost = collection_values::key_cost(
                                &self.value.nodes[k],
                                &other.value.nodes[ok],
                            )
                            .ok_or(GenericCollectionError::Invalid)?;
                            work.charge(cost)?;
                            if collection_values::key_equal(
                                &self.value.nodes[k],
                                &other.value.nodes[ok],
                            ) {
                                found = Some(ov);
                                break;
                            }
                        }
                        let Some(r) = found else { return Ok(false) };
                        pending.push((v, r));
                    }
                }
                (Kind::Set(x), Kind::Set(y)) if x.len() == y.len() => {
                    for key in x {
                        let mut found = false;
                        for other_key in y {
                            let cost = collection_values::key_cost(
                                &self.value.nodes[*key],
                                &other.value.nodes[*other_key],
                            )
                            .ok_or(GenericCollectionError::Invalid)?;
                            work.charge(cost)?;
                            if collection_values::key_equal(
                                &self.value.nodes[*key],
                                &other.value.nodes[*other_key],
                            ) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return Ok(false);
                        }
                    }
                }
                (Kind::Int(a), Kind::Int(b)) if a == b => {}
                (Kind::Byte(a), Kind::Byte(b)) if a == b => {}
                (Kind::Float(a), Kind::Float(b)) if a == b => {}
                (Kind::Bool(a), Kind::Bool(b)) if a == b => {}
                (Kind::String(a), Kind::String(b)) if a == b => {}
                (Kind::Array(a), Kind::Array(b))
                | (Kind::Bytes(a), Kind::Bytes(b))
                | (Kind::Record(a), Kind::Record(b))
                    if a.len() == b.len() =>
                {
                    pending.extend(a.iter().copied().zip(b.iter().copied()));
                }
                (Kind::Enum { tag: a, payload: x }, Kind::Enum { tag: b, payload: y })
                    if a == b =>
                {
                    match (x, y) {
                        (None, None) => {}
                        (Some(x), Some(y)) => pending.push((*x, *y)),
                        _ => return Ok(false),
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}
fn capture(
    layouts: &CheckedGenericLayouts,
    value: &Value,
    path: &[GenericCollectionSelector],
    fuel: &mut RuntimeValueFuel,
    work: &mut RuntimeSetWork,
) -> Result<GenericCollectionSnapshot, GenericCollectionError> {
    let mut root = value
        .nodes
        .len()
        .checked_sub(1)
        .ok_or(GenericCollectionError::Invalid)?;
    for selector in path {
        work.charge(1)?;
        root = match (selector, &value.nodes[root].kind) {
            (GenericCollectionSelector::MapKey(index), Kind::Map(entries))
            | (GenericCollectionSelector::MapValue(index), Kind::Map(entries)) => {
                let entry = *entries.get(*index).ok_or(GenericCollectionError::Missing)?;
                let (key, value) = collection_values::entry(&value.nodes, entry)
                    .ok_or(GenericCollectionError::Invalid)?;
                if matches!(selector, GenericCollectionSelector::MapKey(_)) {
                    key
                } else {
                    value
                }
            }
            (GenericCollectionSelector::SetItem(index), Kind::Set(keys)) => {
                *keys.get(*index).ok_or(GenericCollectionError::Missing)?
            }
            (GenericCollectionSelector::Field(i), Kind::Record(fields)) => {
                *fields.get(*i).ok_or(GenericCollectionError::Missing)?
            }
            (GenericCollectionSelector::Item(i), Kind::Array(items) | Kind::Bytes(items)) => {
                *items.get(*i).ok_or(GenericCollectionError::Missing)?
            }
            (
                GenericCollectionSelector::Payload,
                Kind::Enum {
                    payload: Some(i), ..
                },
            ) => *i,
            _ => return Err(GenericCollectionError::Missing),
        };
    }
    let node = &value.nodes[root];
    if layouts.layout(node.layout).is_none() {
        return Err(GenericCollectionError::Invalid);
    }
    let start = (root + 1)
        .checked_sub(node.count)
        .ok_or(GenericCollectionError::Invalid)?;
    if node.count == 0 {
        return Err(GenericCollectionError::Invalid);
    }
    let cost = (node.count as u64)
        .checked_mul(2)
        .and_then(|n| n.checked_add(node.text))
        .ok_or(GenericCollectionError::Invalid)?;
    fuel.preflight(node.count as u64, node.text)?;
    work.charge(cost)?;
    fuel.admit(node.count as u64, node.text)?;
    let mut nodes = Vec::with_capacity(node.count);
    for node in &value.nodes[start..=root] {
        let mut copy = node.clone();
        copy.kind.indices(|index| index - start);
        nodes.push(copy);
    }
    Ok(GenericCollectionSnapshot {
        layouts: layouts.clone(),
        value: Value { nodes },
    })
}
