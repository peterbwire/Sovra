//! Ordered maps retaining actual generic value/type proofs; no source/IR admission.
use super::*;
/// Refusal without an updated map or partial owned result.
#[derive(Debug, PartialEq, Eq)]
pub enum GenericCollectionMapError {
    /// Key domain differs from the map's declared primitive domain.
    KeyType,
    /// Exact checked value owners/type arguments do not agree.
    ValueType,
    /// Checked arithmetic or representation ceiling exceeded.
    Capacity,
    /// Snapshot/type/work/materialization admission refused.
    Value(GenericCollectionError),
}
impl std::fmt::Display for GenericCollectionMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic collection map refused: {self:?}")
    }
}
impl std::error::Error for GenericCollectionMapError {}
impl From<GenericCollectionError> for GenericCollectionMapError {
    fn from(e: GenericCollectionError) -> Self {
        Self::Value(e)
    }
}
impl From<RuntimeValueError> for GenericCollectionMapError {
    fn from(e: RuntimeValueError) -> Self {
        Self::Value(e.into())
    }
}
impl From<RuntimeSetError> for GenericCollectionMapError {
    fn from(e: RuntimeSetError) -> Self {
        match e {
            RuntimeSetError::KeyType => Self::KeyType,
            RuntimeSetError::Capacity => Self::Capacity,
            e => Self::Value(e.into()),
        }
    }
}
/// Independent entry snapshot; public fields cannot mutate internal storage.
#[derive(Debug)]
pub struct GenericCollectionMapEntry {
    /// Owned primitive key.
    pub key: RuntimeKey,
    /// Owned value retaining its original generic proof.
    pub value: GenericCollectionSnapshot,
}
/// Successful removal result; the Rust carrier adds no language wrapper.
#[derive(Debug)]
pub struct GenericCollectionMapRemoval {
    /// Updated map retaining its declared value type even when empty.
    pub map: GenericCollectionMap,
    /// Independent removed value snapshot.
    pub value: GenericCollectionSnapshot,
}
/// Private unique ordered keys and exactly typed proof-carrying generic values.
#[derive(Debug)]
pub struct GenericCollectionMap {
    key_type: RuntimeKeyType,
    value_type: GenericCollectionType,
    types: GenericTypeEquivalenceLimits,
    entries: Vec<GenericCollectionMapEntry>,
}
fn plus(a: u64, b: u64) -> Result<u64, GenericCollectionMapError> {
    a.checked_add(b).ok_or(GenericCollectionMapError::Capacity)
}
struct Shape {
    nodes: u64,
    text: u64,
    values: u64,
}
impl Shape {
    fn new() -> Self {
        Self {
            nodes: 1,
            text: 0,
            values: 0,
        }
    }
    fn value(&mut self, v: &GenericCollectionSnapshot) -> Result<(), GenericCollectionMapError> {
        let (n, t) = v.value().footprint();
        self.nodes = plus(self.nodes, n as u64)?;
        self.text = plus(self.text, t)?;
        self.values = plus(self.values, n as u64)?;
        Ok(())
    }
    fn entry(
        &mut self,
        k: &RuntimeKey,
        v: &GenericCollectionSnapshot,
    ) -> Result<(), GenericCollectionMapError> {
        self.nodes = plus(self.nodes, 2)?;
        self.text = plus(self.text, k.text())?;
        self.value(v)
    }
    fn admit(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<(), GenericCollectionMapError> {
        fuel.preflight(self.nodes, self.text)?;
        work.charge(plus(plus(self.nodes, self.text)?, self.values)?)?;
        fuel.admit(self.nodes, self.text)?;
        Ok(())
    }
}
impl GenericCollectionMapEntry {
    fn copy_admitted(&self) -> Self {
        Self {
            key: self.key.copy(),
            value: self.value.copy_admitted(),
        }
    }
}
impl GenericCollectionMap {
    /// Create an empty map with explicit finite per-comparison type admission.
    pub fn new(
        key_type: RuntimeKeyType,
        value_type: GenericCollectionType,
        types: GenericTypeEquivalenceLimits,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, GenericCollectionMapError> {
        if !value_type.equivalent(&value_type, types)? {
            return Err(GenericCollectionMapError::ValueType);
        }
        fuel.admit(1, 0)?;
        Ok(Self {
            key_type,
            value_type,
            types,
            entries: Vec::new(),
        })
    }
    /// Borrow the exact declared type proof without materializing a language value.
    pub fn value_type(&self) -> &GenericCollectionType {
        &self.value_type
    }
    /// Declared primitive key domain.
    pub fn key_type(&self) -> RuntimeKeyType {
        self.key_type
    }
    /// Borrowed length metadata, not a language Int result.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn result(&self, entries: Vec<GenericCollectionMapEntry>) -> Self {
        Self {
            key_type: self.key_type,
            value_type: self.value_type.clone(),
            types: self.types,
            entries,
        }
    }
    fn position(
        &self,
        key: &RuntimeKey,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<usize>, GenericCollectionMapError> {
        if key.kind() != self.key_type {
            return Err(GenericCollectionMapError::KeyType);
        }
        for (i, e) in self.entries.iter().enumerate() {
            work.charge(plus(plus(1, e.key.text())?, key.text())?)?;
            if &e.key == key {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    fn plan(
        &self,
        skip: Option<usize>,
        replace: Option<(usize, &GenericCollectionSnapshot)>,
        append: Option<(&RuntimeKey, &GenericCollectionSnapshot)>,
        work: &mut RuntimeSetWork,
    ) -> Result<Shape, GenericCollectionMapError> {
        let mut shape = Shape::new();
        for (i, e) in self.entries.iter().enumerate() {
            work.charge(1)?;
            if skip == Some(i) {
                continue;
            }
            let value = match replace {
                Some((at, value)) if at == i => value,
                _ => &e.value,
            };
            shape.entry(&e.key, value)?;
        }
        if let Some((key, value)) = append {
            shape.entry(key, value)?;
        }
        Ok(shape)
    }
    /// Membership lookup; no hidden language Bool is materialized.
    pub fn contains(
        &self,
        key: &RuntimeKey,
        work: &mut RuntimeSetWork,
    ) -> Result<bool, GenericCollectionMapError> {
        Ok(self.position(key, work)?.is_some())
    }
    /// Missing keys return Rust None; present values copy their entire subtree.
    pub fn get(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<GenericCollectionSnapshot>, GenericCollectionMapError> {
        let Some(index) = self.position(key, work)? else {
            return Ok(None);
        };
        let value = &self.entries[index].value;
        let (n, t) = value.value().footprint();
        Shape {
            nodes: n as u64,
            text: t,
            values: n as u64,
        }
        .admit(fuel, work)?;
        Ok(Some(value.copy_admitted()))
    }
    /// Copy a complete map atomically, retaining checked value ownership.
    pub fn snapshot(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, GenericCollectionMapError> {
        self.plan(None, None, None, work)?.admit(fuel, work)?;
        Ok(self.result(
            self.entries
                .iter()
                .map(GenericCollectionMapEntry::copy_admitted)
                .collect(),
        ))
    }
    /// Replace without moving position, or append a new key; inputs remain unchanged.
    pub fn insert(
        &self,
        key: &RuntimeKey,
        value: &GenericCollectionSnapshot,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, GenericCollectionMapError> {
        let found = self.position(key, work)?;
        if !self.value_type.equivalent(&value.ty(), self.types)? {
            return Err(GenericCollectionMapError::ValueType);
        }
        let length = super::super::runtime_sets::result_length(self.len(), false, found.is_none())?;
        self.plan(
            None,
            found.map(|i| (i, value)),
            if found.is_none() {
                Some((key, value))
            } else {
                None
            },
            work,
        )?
        .admit(fuel, work)?;
        let mut entries = Vec::with_capacity(length);
        for (i, e) in self.entries.iter().enumerate() {
            entries.push(if found == Some(i) {
                GenericCollectionMapEntry {
                    key: e.key.copy(),
                    value: value.copy_admitted(),
                }
            } else {
                e.copy_admitted()
            });
        }
        if found.is_none() {
            entries.push(GenericCollectionMapEntry {
                key: key.copy(),
                value: value.copy_admitted(),
            });
        }
        Ok(self.result(entries))
    }
    /// Admit the updated map and removed value together. Absence copies nothing.
    pub fn remove(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<GenericCollectionMapRemoval>, GenericCollectionMapError> {
        let Some(index) = self.position(key, work)? else {
            return Ok(None);
        };
        let removed = &self.entries[index].value;
        let mut shape = self.plan(Some(index), None, None, work)?;
        shape.value(removed)?;
        shape.admit(fuel, work)?;
        let entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, e)| e.copy_admitted())
            .collect();
        Ok(Some(GenericCollectionMapRemoval {
            map: self.result(entries),
            value: removed.copy_admitted(),
        }))
    }
    /// Independent insertion-ordered key array snapshot.
    pub fn keys(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<RuntimeKey>, GenericCollectionMapError> {
        let mut shape = Shape::new();
        for e in &self.entries {
            work.charge(1)?;
            shape.nodes = plus(shape.nodes, 1)?;
            shape.text = plus(shape.text, e.key.text())?;
        }
        shape.admit(fuel, work)?;
        Ok(self.entries.iter().map(|e| e.key.copy()).collect())
    }
    /// Independent insertion-ordered value array snapshot including all descendants.
    pub fn values(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<GenericCollectionSnapshot>, GenericCollectionMapError> {
        let mut shape = Shape::new();
        for e in &self.entries {
            work.charge(1)?;
            shape.value(&e.value)?;
        }
        shape.admit(fuel, work)?;
        Ok(self
            .entries
            .iter()
            .map(|e| e.value.copy_admitted())
            .collect())
    }
    /// Independent insertion-ordered anonymous entries array snapshot.
    pub fn entries(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<GenericCollectionMapEntry>, GenericCollectionMapError> {
        self.plan(None, None, None, work)?.admit(fuel, work)?;
        Ok(self
            .entries
            .iter()
            .map(GenericCollectionMapEntry::copy_admitted)
            .collect())
    }
    /// Association equality independent of order; declared key/value domains must agree.
    /// Both maps' finite type profiles constrain the root comparison.
    pub fn equal(
        &self,
        other: &Self,
        work: &mut RuntimeSetWork,
    ) -> Result<bool, GenericCollectionMapError> {
        if self.key_type != other.key_type {
            return Err(GenericCollectionMapError::KeyType);
        }
        let types = GenericTypeEquivalenceLimits {
            max_pairs: self.types.max_pairs.min(other.types.max_pairs),
            max_edges: self.types.max_edges.min(other.types.max_edges),
            max_work: self.types.max_work.min(other.types.max_work),
        };
        if !self.value_type.equivalent(&other.value_type, types)? {
            return Err(GenericCollectionMapError::ValueType);
        }
        work.charge(1)?;
        if self.len() != other.len() {
            return Ok(false);
        }
        for e in &self.entries {
            let Some(i) = other.position(&e.key, work)? else {
                return Ok(false);
            };
            if !e.value.equal_typed(&other.entries[i].value, work)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
