//! Sealed opaque collection layout plans beside the unchanged executable layout table.
use super::*;
/// Compiler-owned collection constructor, distinct from a same-spelled user record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericCollectionLayoutKind {
    /// Ordered snapshot map with key and value arguments.
    Map,
    /// Ordered snapshot set with one key argument.
    Set,
}
/// Refusal before a usable plan or partially materialized storage is returned.
#[derive(Debug, PartialEq, Eq)]
pub enum GenericCollectionLayoutError {
    /// Byte key storage requires separately qualified data operations.
    DataOperationsUnavailable,
    /// Key must satisfy the checked Key constraint.
    KeyBound,
    /// A map plan cannot instantiate a set, or vice versa.
    Kind,
    /// Independent exact type admission refused.
    Type(GenericCollectionError),
    /// Generic map storage refused.
    Map(GenericCollectionMapError),
    /// Primitive set storage refused.
    Set(RuntimeSetError),
}
impl std::fmt::Display for GenericCollectionLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "collection layout refused: {self:?}")
    }
}
impl std::error::Error for GenericCollectionLayoutError {}
impl From<GenericCollectionError> for GenericCollectionLayoutError {
    fn from(e: GenericCollectionError) -> Self {
        Self::Type(e)
    }
}
/// Closed opaque plan retaining checked arguments, not fields or an executable ABI.
/// Private fields prevent unchecked names/indices or inconsistent arities.
#[derive(Debug, Clone)]
pub struct GenericCollectionLayout {
    key: GenericCollectionType,
    value: Option<GenericCollectionType>,
    key_domain: RuntimeKeyType,
    types: GenericTypeEquivalenceLimits,
}
impl GenericCollectionLayout {
    /// Bind an admitted opaque executable-layout shape to its typed storage plan.
    /// The index is local to this retained table; ordinary record shapes are refused.
    pub fn from_checked_layout(
        layouts: &CheckedGenericLayouts,
        index: usize,
        types: GenericTypeEquivalenceLimits,
    ) -> Result<Self, GenericCollectionLayoutError> {
        let layout = layouts
            .layout(index)
            .ok_or(GenericCollectionError::Invalid)?;
        match layout.kind() {
            GenericRuntimeLayoutKind::Map { key, value, .. } => Self::map(
                GenericCollectionType::from_layout(layouts, *key)?,
                GenericCollectionType::from_layout(layouts, *value)?,
                types,
            ),
            GenericRuntimeLayoutKind::Set { key, .. } => {
                Self::set(GenericCollectionType::from_layout(layouts, *key)?, types)
            }
            _ => Err(GenericCollectionLayoutError::Kind),
        }
    }
    /// Check Map<K,V> with exact retained V ownership and the primitive Key bound.
    pub fn map(
        key: GenericCollectionType,
        value: GenericCollectionType,
        types: GenericTypeEquivalenceLimits,
    ) -> Result<Self, GenericCollectionLayoutError> {
        Self::checked(key, Some(value), types)
    }
    /// Check Set<K> with the primitive Key bound; no value/default argument exists.
    pub fn set(
        key: GenericCollectionType,
        types: GenericTypeEquivalenceLimits,
    ) -> Result<Self, GenericCollectionLayoutError> {
        Self::checked(key, None, types)
    }
    fn checked(
        key: GenericCollectionType,
        value: Option<GenericCollectionType>,
        types: GenericTypeEquivalenceLimits,
    ) -> Result<Self, GenericCollectionLayoutError> {
        if matches!(key.layout().kind(), GenericRuntimeLayoutKind::Byte { .. }) {
            return Err(GenericCollectionLayoutError::DataOperationsUnavailable);
        }
        let key_domain = match key.layout().ty().primitive() {
            Some(GenericPrimitive::Int) => RuntimeKeyType::Int,
            Some(GenericPrimitive::Bool) => RuntimeKeyType::Bool,
            Some(GenericPrimitive::String) => RuntimeKeyType::String,
            _ => return Err(GenericCollectionLayoutError::KeyBound),
        };
        if !key.equivalent(&key, types)? {
            return Err(GenericCollectionLayoutError::KeyBound);
        }
        if let Some(value) = &value {
            if !value.equivalent(value, types)? {
                return Err(GenericCollectionError::Invalid.into());
            }
        }
        Ok(Self {
            key,
            value,
            key_domain,
            types,
        })
    }
    /// Compiler-owned constructor identity, never inferred from a declaration name.
    pub fn kind(&self) -> GenericCollectionLayoutKind {
        if self.value.is_some() {
            GenericCollectionLayoutKind::Map
        } else {
            GenericCollectionLayoutKind::Set
        }
    }
    /// Reserved descriptive name; this API does not register it for source lookup.
    pub fn qualified_name(&self) -> &'static str {
        match self.kind() {
            GenericCollectionLayoutKind::Map => "std::Map",
            GenericCollectionLayoutKind::Set => "std::Set",
        }
    }
    /// Constructor arity; Map has two arguments and Set has one.
    pub fn argument_count(&self) -> usize {
        1 + usize::from(self.value.is_some())
    }
    /// Borrow a checked argument in key/value order.
    pub fn argument(&self, index: usize) -> Option<&GenericCollectionType> {
        match index {
            0 => Some(&self.key),
            1 => self.value.as_ref(),
            _ => None,
        }
    }
    /// Primitive storage domain derived solely from the checked key argument.
    pub fn key_domain(&self) -> RuntimeKeyType {
        self.key_domain
    }
    /// Retained finite per-argument comparison profile, not a new command default.
    pub fn type_limits(&self) -> GenericTypeEquivalenceLimits {
        self.types
    }
    /// Exact constructor and argument identity under both plans' finite profiles.
    pub fn equivalent(&self, other: &Self) -> Result<bool, GenericCollectionLayoutError> {
        if self.kind() != other.kind() {
            return Ok(false);
        }
        let types = GenericTypeEquivalenceLimits {
            max_pairs: self.types.max_pairs.min(other.types.max_pairs),
            max_edges: self.types.max_edges.min(other.types.max_edges),
            max_work: self.types.max_work.min(other.types.max_work),
        };
        if !self.key.equivalent(&other.key, types)? {
            return Ok(false);
        }
        match (&self.value, &other.value) {
            (Some(a), Some(b)) => Ok(a.equivalent(b, types)?),
            (None, None) => Ok(true),
            _ => Err(GenericCollectionLayoutError::Kind),
        }
    }
    /// Instantiate map storage from this checked key/value binding; no forged defaults.
    pub fn instantiate_map(
        &self,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<GenericCollectionMap, GenericCollectionLayoutError> {
        let value = self
            .value
            .as_ref()
            .ok_or(GenericCollectionLayoutError::Kind)?;
        GenericCollectionMap::new(self.key_domain, value.clone(), self.types, fuel)
            .map_err(GenericCollectionLayoutError::Map)
    }
    /// Instantiate primitive set storage from this checked key binding.
    pub fn instantiate_set(
        &self,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<RuntimeSetV2, GenericCollectionLayoutError> {
        if self.value.is_some() {
            return Err(GenericCollectionLayoutError::Kind);
        }
        RuntimeSetV2::new(self.key_domain, fuel).map_err(GenericCollectionLayoutError::Set)
    }
}
