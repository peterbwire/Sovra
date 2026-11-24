//! Opaque typed map snapshots; explicit Rust API, not compiler/VM admission.
use super::{
    runtime_sets::result_length, RuntimeKey, RuntimeKeyType, RuntimeSetError, RuntimeSetWork,
    RuntimeValueFuel, RuntimeValueV2,
};
/// Admitted primitive or checked-nominal map value leaves. Keys remain restricted independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMapPrimitive {
    /// Unit value.
    Unit,
    /// Boolean value.
    Bool,
    /// Signed integer value.
    Int,
    /// Float value, without implicit widening.
    Float,
    /// UTF-8 string value.
    String,
    /// Nominal enum identity from an immutable checked environment.
    /// Records in its payload tree must carry sealed owners.
    Enum(super::EnumId),
    /// Record identity sealed by checked record construction.
    Record(super::RuntimeRecordId),
}
/// Flat type descriptor; nominal values use sealed identities, never caller names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeMapValueType {
    leaf: RuntimeMapPrimitive,
    depth: u8,
}
impl RuntimeMapValueType {
    /// Primitive/checked-nominal leaf or homogeneous arrays, at most 128 array levels.
    pub fn new(leaf: RuntimeMapPrimitive, array_depth: u8) -> Result<Self, RuntimeMapError> {
        if array_depth > 128 {
            return Err(RuntimeMapError::ValueType);
        }
        Ok(Self {
            leaf,
            depth: array_depth,
        })
    }
    /// Leaf primitive or sealed enum identity.
    pub fn leaf(self) -> RuntimeMapPrimitive {
        self.leaf
    }
    /// Number of array levels around the leaf.
    pub fn array_depth(self) -> u8 {
        self.depth
    }
}
/// Map refusal without a partially updated result.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeMapError {
    /// Key-domain, capacity, work or materialization refusal shared with sets.
    Collection(RuntimeSetError),
    /// Value shape or map value domain does not match exactly.
    ValueType,
}
impl From<RuntimeSetError> for RuntimeMapError {
    fn from(e: RuntimeSetError) -> Self {
        Self::Collection(e)
    }
}
impl From<super::RuntimeValueError> for RuntimeMapError {
    fn from(e: super::RuntimeValueError) -> Self {
        Self::Collection(RuntimeSetError::Materialization(e))
    }
}
impl std::fmt::Display for RuntimeMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime map refused: {self:?}")
    }
}
impl std::error::Error for RuntimeMapError {}
#[derive(Debug)]
struct Entry {
    key: RuntimeKey,
    value: RuntimeValueV2,
    nodes: u64,
    text: u64,
}
/// Independent entry snapshot. Mutating/dropping it cannot change a map.
#[derive(Debug)]
pub struct RuntimeMapEntry {
    /// Owned primitive key snapshot.
    pub key: RuntimeKey,
    /// Owned value snapshot including all descendants.
    pub value: RuntimeValueV2,
}
/// Successful remove result; Rust wrapper adds no language materialization.
#[derive(Debug)]
pub struct RuntimeMapRemoval {
    /// Map without the selected entry.
    pub map: RuntimeMapV2,
    /// Removed value snapshot.
    pub value: RuntimeValueV2,
}
/// Private homogeneous ordered storage; callers cannot forge duplicate entries.
#[derive(Debug)]
pub struct RuntimeMapV2 {
    key_type: RuntimeKeyType,
    value_type: RuntimeMapValueType,
    entries: Vec<Entry>,
}
fn plus(a: u64, b: u64) -> Result<u64, RuntimeMapError> {
    a.checked_add(b).ok_or(RuntimeSetError::Capacity.into())
}
impl RuntimeMapV2 {
    /// Empty map, charging one logical container node.
    pub fn new(
        key_type: RuntimeKeyType,
        value_type: RuntimeMapValueType,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeMapError> {
        fuel.admit(1, 0)?;
        Ok(Self {
            key_type,
            value_type,
            entries: Vec::new(),
        })
    }
    /// Key domain, including for empty maps.
    pub fn key_type(&self) -> RuntimeKeyType {
        self.key_type
    }
    /// Exact value shape, including for empty maps.
    pub fn value_type(&self) -> RuntimeMapValueType {
        self.value_type
    }
    /// Borrowed metadata, without materializing a language Int.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether no entries exist.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn position(
        &self,
        key: &RuntimeKey,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<usize>, RuntimeMapError> {
        if key.kind() != self.key_type {
            return Err(RuntimeSetError::KeyType.into());
        }
        for (i, entry) in self.entries.iter().enumerate() {
            work.charge(plus(plus(1, entry.key.text())?, key.text())?)?;
            if &entry.key == key {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    /// Membership; Rust bool adds no language result node.
    pub fn contains(
        &self,
        key: &RuntimeKey,
        work: &mut RuntimeSetWork,
    ) -> Result<bool, RuntimeMapError> {
        Ok(self.position(key, work)?.is_some())
    }
    /// Missing keys return None; present values copy their complete subtree.
    pub fn get(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<RuntimeValueV2>, RuntimeMapError> {
        let Some(index) = self.position(key, work)? else {
            return Ok(None);
        };
        let e = &self.entries[index];
        admit(e.nodes, e.text, fuel, work)?;
        Ok(Some(e.value.collection_copy_admitted()))
    }
    fn copy_entry(e: &Entry) -> Entry {
        Entry {
            key: e.key.copy(),
            value: e.value.collection_copy_admitted(),
            nodes: e.nodes,
            text: e.text,
        }
    }
    fn measure<'a>(
        entries: impl Iterator<Item = (&'a RuntimeKey, u64, u64)>,
        work: &mut RuntimeSetWork,
    ) -> Result<(u64, u64), RuntimeMapError> {
        let (mut nodes, mut text) = (1, 0);
        for (key, value_nodes, value_text) in entries {
            work.charge(1)?;
            nodes = plus(nodes, plus(2, value_nodes)?)?;
            text = plus(text, plus(key.text(), value_text)?)?;
        }
        Ok((nodes, text))
    }
    /// Independent complete map snapshot.
    pub fn snapshot(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, RuntimeMapError> {
        let (nodes, text) =
            Self::measure(self.entries.iter().map(|e| (&e.key, e.nodes, e.text)), work)?;
        admit(nodes, text, fuel, work)?;
        Ok(Self {
            key_type: self.key_type,
            value_type: self.value_type,
            entries: self.entries.iter().map(Self::copy_entry).collect(),
        })
    }
    /// Insert or replace in place in the ordering, preserving the input map.
    pub fn insert(
        &self,
        key: &RuntimeKey,
        value: &RuntimeValueV2,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, RuntimeMapError> {
        let found = self.position(key, work)?;
        result_length(self.len(), false, found.is_none())?;
        let (value_nodes, value_text) = value.collection_shape(self.value_type, work)?;
        let (mut nodes, mut text) = (1, 0);
        for (i, e) in self.entries.iter().enumerate() {
            work.charge(1)?;
            let (vn, vt) = if found == Some(i) {
                (value_nodes, value_text)
            } else {
                (e.nodes, e.text)
            };
            nodes = plus(nodes, plus(2, vn)?)?;
            text = plus(text, plus(e.key.text(), vt)?)?;
        }
        if found.is_none() {
            nodes = plus(nodes, plus(2, value_nodes)?)?;
            text = plus(text, plus(key.text(), value_text)?)?;
        }
        admit(nodes, text, fuel, work)?;
        let mut entries = Vec::with_capacity(self.len() + usize::from(found.is_none()));
        for (i, e) in self.entries.iter().enumerate() {
            entries.push(if found == Some(i) {
                Entry {
                    key: e.key.copy(),
                    value: value.collection_copy_admitted(),
                    nodes: value_nodes,
                    text: value_text,
                }
            } else {
                Self::copy_entry(e)
            });
        }
        if found.is_none() {
            entries.push(Entry {
                key: key.copy(),
                value: value.collection_copy_admitted(),
                nodes: value_nodes,
                text: value_text,
            });
        }
        Ok(Self {
            key_type: self.key_type,
            value_type: self.value_type,
            entries,
        })
    }
    /// Missing removal returns None. Present removal admits map and removed value together.
    pub fn remove(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<RuntimeMapRemoval>, RuntimeMapError> {
        let Some(index) = self.position(key, work)? else {
            return Ok(None);
        };
        let (nodes, text) = Self::measure(
            self.entries
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, e)| (&e.key, e.nodes, e.text)),
            work,
        )?;
        let removed = &self.entries[index];
        admit(
            plus(nodes, removed.nodes)?,
            plus(text, removed.text)?,
            fuel,
            work,
        )?;
        let entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, e)| Self::copy_entry(e))
            .collect();
        Ok(Some(RuntimeMapRemoval {
            map: Self {
                key_type: self.key_type,
                value_type: self.value_type,
                entries,
            },
            value: removed.value.collection_copy_admitted(),
        }))
    }
    /// Insertion-ordered key array snapshot; one array and each copied key node.
    pub fn keys(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<RuntimeKey>, RuntimeMapError> {
        let mut text = 0;
        for e in &self.entries {
            work.charge(1)?;
            text = plus(text, e.key.text())?;
        }
        admit(plus(1, self.len() as u64)?, text, fuel, work)?;
        Ok(self.entries.iter().map(|e| e.key.copy()).collect())
    }
    /// Insertion-ordered value array snapshot with full descendant ownership costs.
    pub fn values(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<RuntimeValueV2>, RuntimeMapError> {
        let (mut nodes, mut text) = (1, 0);
        for e in &self.entries {
            work.charge(1)?;
            nodes = plus(nodes, e.nodes)?;
            text = plus(text, e.text)?;
        }
        admit(nodes, text, fuel, work)?;
        Ok(self
            .entries
            .iter()
            .map(|e| e.value.collection_copy_admitted())
            .collect())
    }
    /// Insertion-ordered anonymous entry array; entry nodes own no text labels.
    pub fn entries(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<RuntimeMapEntry>, RuntimeMapError> {
        let (nodes, text) =
            Self::measure(self.entries.iter().map(|e| (&e.key, e.nodes, e.text)), work)?;
        admit(nodes, text, fuel, work)?;
        Ok(self
            .entries
            .iter()
            .map(|e| RuntimeMapEntry {
                key: e.key.copy(),
                value: e.value.collection_copy_admitted(),
            })
            .collect())
    }
    /// Association equality independent of order; exact key and value domains must match.
    pub fn equal(&self, other: &Self, work: &mut RuntimeSetWork) -> Result<bool, RuntimeMapError> {
        if self.key_type != other.key_type {
            return Err(RuntimeSetError::KeyType.into());
        }
        if self.value_type != other.value_type {
            return Err(RuntimeMapError::ValueType);
        }
        work.charge(1)?;
        if self.len() != other.len() {
            return Ok(false);
        }
        for e in &self.entries {
            let Some(i) = other.position(&e.key, work)? else {
                return Ok(false);
            };
            let r = &other.entries[i];
            work.charge(plus(plus(e.nodes, r.nodes)?, plus(e.text, r.text)?)?)?;
            if !e.value.collection_equal_admitted(&r.value, work)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
fn admit(
    nodes: u64,
    text: u64,
    fuel: &mut RuntimeValueFuel,
    work: &mut RuntimeSetWork,
) -> Result<(), RuntimeMapError> {
    fuel.preflight(nodes, text)?;
    work.charge(plus(nodes, text)?)?;
    fuel.admit(nodes, text)?;
    Ok(())
}
