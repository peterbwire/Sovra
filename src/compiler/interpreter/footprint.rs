//! Borrowed logical measurements. No allocation policy or execution hook.
use super::Value;

/// Logical size of one owned runtime value tree, independent of allocator layout.
/// Counts are observations, not heap/RSS measurements or enforced limits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ValueFootprint {
    /// One per scalar, Unit, array or record, including compound descendants.
    pub value_nodes: u64,
    /// UTF-8 bytes of String payloads, record type names and field labels.
    pub owned_text_bytes: u64,
    /// UTF-8 bytes of existing runtime display, excluding any printed LF.
    pub display_bytes: u64,
}
impl ValueFootprint {
    /// Sum independent roots without adding an enclosing container or separators.
    /// Any overflow rejects the whole aggregate rather than returning partial data.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            value_nodes: self.value_nodes.checked_add(other.value_nodes)?,
            owned_text_bytes: self.owned_text_bytes.checked_add(other.owned_text_bytes)?,
            display_bytes: self.display_bytes.checked_add(other.display_bytes)?,
        })
    }
}

impl Value {
    /// Measure an existing value without cloning payloads or rendering compound text.
    /// Traversal borrows iterator frames with scratch space proportional to depth.
    /// None means counter overflow. Scalar number formatting may use small temporary
    /// strings; this does not measure those temporaries, allocation capacity or RSS.
    pub fn logical_footprint(&self) -> Option<ValueFootprint> {
        enum Work<'a> {
            Value(&'a Value),
            Values(std::slice::Iter<'a, Value>),
            Fields(std::slice::Iter<'a, (String, Value)>),
        }
        fn bytes(text: &str) -> Option<u64> {
            u64::try_from(text.len()).ok()
        }
        fn separators(count: usize) -> Option<u64> {
            u64::try_from(count).ok()?.saturating_sub(1).checked_mul(2)
        }
        let mut result = ValueFootprint::default();
        let mut pending = vec![Work::Value(self)];
        while let Some(work) = pending.pop() {
            match work {
                Work::Values(mut items) => {
                    if let Some(value) = items.next() {
                        pending.push(Work::Values(items));
                        pending.push(Work::Value(value));
                    }
                }
                Work::Fields(mut fields) => {
                    if let Some((name, value)) = fields.next() {
                        let length = bytes(name)?;
                        result = result.checked_add(ValueFootprint {
                            owned_text_bytes: length,
                            display_bytes: length.checked_add(2)?,
                            ..ValueFootprint::default()
                        })?;
                        pending.push(Work::Fields(fields));
                        pending.push(Work::Value(value));
                    }
                }
                Work::Value(value) => {
                    let mut observed = ValueFootprint {
                        value_nodes: 1,
                        ..ValueFootprint::default()
                    };
                    match value {
                        Value::Unit => {}
                        Value::Int(value) => observed.display_bytes = bytes(&value.to_string())?,
                        Value::Float(value) => observed.display_bytes = bytes(&value.to_string())?,
                        Value::Bool(value) => observed.display_bytes = if *value { 4 } else { 5 },
                        Value::String(value) => {
                            observed.owned_text_bytes = bytes(value)?;
                            observed.display_bytes = observed.owned_text_bytes;
                        }
                        Value::Array(items) => {
                            observed.display_bytes = 2u64.checked_add(separators(items.len())?)?;
                            pending.push(Work::Values(items.iter()));
                        }
                        Value::Struct(record) => {
                            observed.owned_text_bytes = bytes(&record.type_name)?;
                            observed.display_bytes = observed
                                .owned_text_bytes
                                .checked_add(5)?
                                .checked_add(separators(record.fields.len())?)?;
                            pending.push(Work::Fields(record.fields.iter()));
                        }
                    }
                    result = result.checked_add(observed)?;
                }
            }
        }
        Some(result)
    }
}
