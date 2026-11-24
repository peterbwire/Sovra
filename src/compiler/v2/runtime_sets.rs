//! Opaque primitive-key set snapshots. Explicit library API, not source/VM admission.
use super::{RuntimeValueError, RuntimeValueFuel};

/// Admitted homogeneous key domain; Float and compound values are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKeyType {
    /// Signed integer keys.
    Int,
    /// Boolean keys.
    Bool,
    /// Exact UTF-8 string keys without normalization.
    String,
}
/// Primitive key input. Set storage remains private and cannot contain duplicates.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeKey {
    /// Signed integer key.
    Int(i64),
    /// Boolean key.
    Bool(bool),
    /// Exact string key.
    String(String),
}
impl RuntimeKey {
    pub(super) fn kind(&self) -> RuntimeKeyType {
        match self {
            Self::Int(_) => RuntimeKeyType::Int,
            Self::Bool(_) => RuntimeKeyType::Bool,
            Self::String(_) => RuntimeKeyType::String,
        }
    }
    pub(super) fn text(&self) -> u64 {
        match self {
            Self::String(s) => s.len() as u64,
            _ => 0,
        }
    }
    pub(super) fn copy(&self) -> Self {
        match self {
            Self::Int(v) => Self::Int(*v),
            Self::Bool(v) => Self::Bool(*v),
            Self::String(s) => Self::String(s.clone()),
        }
    }
}
/// Refusal without an updated collection or partial snapshot.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeSetError {
    /// The caller supplied a different key domain.
    KeyType,
    /// Shared logical materialization admission refused.
    Materialization(RuntimeValueError),
    /// Finite helper-work allowance exhausted.
    WorkLimit,
    /// Representation or checked aggregate arithmetic exceeded its ceiling.
    Capacity,
}
impl std::fmt::Display for RuntimeSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime set refused: {self:?}")
    }
}
impl std::error::Error for RuntimeSetError {}
impl From<RuntimeValueError> for RuntimeSetError {
    fn from(e: RuntimeValueError) -> Self {
        Self::Materialization(e)
    }
}
/// Cumulative explicit helper work. Refusal does not refund already performed work.
#[derive(Debug)]
pub struct RuntimeSetWork {
    remaining: u64,
}
impl RuntimeSetWork {
    /// Create a finite shared allowance.
    pub fn new(units: u64) -> Self {
        Self { remaining: units }
    }
    /// Unspent units.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    pub(super) fn charge(&mut self, units: u64) -> Result<(), RuntimeSetError> {
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or(RuntimeSetError::WorkLimit)?;
        Ok(())
    }
}
/// Ordered, unique, homogeneous primitive keys. No unchecked mutation or Clone API.
#[derive(Debug)]
pub struct RuntimeSetV2 {
    kind: RuntimeKeyType,
    keys: Vec<RuntimeKey>,
}
impl RuntimeSetV2 {
    /// Create an empty set, charging one logical container node.
    pub fn new(kind: RuntimeKeyType, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeSetError> {
        fuel.admit(1, 0)?;
        Ok(Self {
            kind,
            keys: Vec::new(),
        })
    }
    /// Key domain retained even for an empty set.
    pub fn key_type(&self) -> RuntimeKeyType {
        self.kind
    }
    /// Borrowed metadata; this does not materialize a language Int.
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    /// Whether this set has no keys.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    fn position(
        &self,
        key: &RuntimeKey,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<usize>, RuntimeSetError> {
        if key.kind() != self.kind {
            return Err(RuntimeSetError::KeyType);
        }
        for (i, existing) in self.keys.iter().enumerate() {
            work.charge(
                1u64.checked_add(existing.text())
                    .and_then(|n| n.checked_add(key.text()))
                    .ok_or(RuntimeSetError::Capacity)?,
            )?;
            if existing == key {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    /// Membership; returned Rust bool has no hidden language materialization.
    pub fn contains(
        &self,
        key: &RuntimeKey,
        work: &mut RuntimeSetWork,
    ) -> Result<bool, RuntimeSetError> {
        Ok(self.position(key, work)?.is_some())
    }
    /// Copy a present key; Rust Option adds no language wrapper node.
    pub fn get(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<RuntimeKey>, RuntimeSetError> {
        let Some(index) = self.position(key, work)? else {
            return Ok(None);
        };
        let found = &self.keys[index];
        fuel.preflight(1, found.text())?;
        work.charge(
            1u64.checked_add(found.text())
                .ok_or(RuntimeSetError::Capacity)?,
        )?;
        fuel.admit(1, found.text())?;
        Ok(Some(found.copy()))
    }
    // Admit the entire result and copy work before cloning a single String.
    fn copied_keys(
        &self,
        skip: Option<usize>,
        append: Option<&RuntimeKey>,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<RuntimeKey>, RuntimeSetError> {
        let length = result_length(self.keys.len(), skip.is_some(), append.is_some())?;
        let mut text = 0u64;
        for (i, key) in self.keys.iter().enumerate() {
            work.charge(1)?;
            if skip != Some(i) {
                text = text
                    .checked_add(key.text())
                    .ok_or(RuntimeSetError::Capacity)?;
            }
        }
        if let Some(key) = append {
            text = text
                .checked_add(key.text())
                .ok_or(RuntimeSetError::Capacity)?;
        }
        let nodes = (length as u64)
            .checked_add(1)
            .ok_or(RuntimeSetError::Capacity)?;
        fuel.preflight(nodes, text)?;
        work.charge(nodes.checked_add(text).ok_or(RuntimeSetError::Capacity)?)?;
        fuel.admit(nodes, text)?;
        let mut keys = Vec::with_capacity(length);
        for (i, key) in self.keys.iter().enumerate() {
            if skip != Some(i) {
                keys.push(key.copy());
            }
        }
        if let Some(key) = append {
            keys.push(key.copy());
        }
        Ok(keys)
    }
    /// Independent set snapshot, one container plus every copied key and UTF-8 byte.
    pub fn snapshot(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, RuntimeSetError> {
        Ok(Self {
            kind: self.kind,
            keys: self.copied_keys(None, None, fuel, work)?,
        })
    }
    /// Insert at the end; duplicate insertion preserves the original position.
    pub fn insert(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Self, RuntimeSetError> {
        let found = self.position(key, work)?.is_some();
        Ok(Self {
            kind: self.kind,
            keys: self.copied_keys(None, if found { None } else { Some(key) }, fuel, work)?,
        })
    }
    /// Remove a member. Absence returns Rust None and materializes no language value.
    pub fn remove(
        &self,
        key: &RuntimeKey,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Option<Self>, RuntimeSetError> {
        let Some(index) = self.position(key, work)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            kind: self.kind,
            keys: self.copied_keys(Some(index), None, fuel, work)?,
        }))
    }
    /// Copy keys into an insertion-ordered array snapshot with the same node costs.
    pub fn keys(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeSetWork,
    ) -> Result<Vec<RuntimeKey>, RuntimeSetError> {
        self.copied_keys(None, None, fuel, work)
    }
    /// Membership equality independent of insertion order; key domains must agree.
    pub fn equal(&self, other: &Self, work: &mut RuntimeSetWork) -> Result<bool, RuntimeSetError> {
        if self.kind != other.kind {
            return Err(RuntimeSetError::KeyType);
        }
        work.charge(1)?;
        if self.len() != other.len() {
            return Ok(false);
        }
        for key in &self.keys {
            if other.position(key, work)?.is_none() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

pub(super) fn result_length(
    current: usize,
    removed: bool,
    added: bool,
) -> Result<usize, RuntimeSetError> {
    let length = current
        .checked_sub(usize::from(removed))
        .and_then(|n| n.checked_add(usize::from(added)))
        .ok_or(RuntimeSetError::Capacity)?;
    if length as u64 > u64::from(u32::MAX) {
        return Err(RuntimeSetError::Capacity);
    }
    Ok(length)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn representation_ceiling_and_arithmetic_refuse_without_allocating() {
        let max = u32::MAX as usize;
        assert_eq!(result_length(max, false, false), Ok(max));
        assert_eq!(
            result_length(max, false, true),
            Err(RuntimeSetError::Capacity)
        );
        assert_eq!(result_length(max, true, true), Ok(max));
        assert_eq!(
            result_length(0, true, false),
            Err(RuntimeSetError::Capacity)
        );
        assert_eq!(
            result_length(usize::MAX, false, true),
            Err(RuntimeSetError::Capacity)
        );
    }
}
