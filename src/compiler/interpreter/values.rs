//! Iterative value operations and private ownership guards. Public values retain
//! their movable enum fields; cleanup safety belongs to runtime-owned values.
use super::{StructValue, Value};
use std::ops::{Deref, DerefMut};

impl Clone for Value {
    fn clone(&self) -> Self {
        // Scalar loads are common; avoid work-stack allocation on this path.
        match self {
            Value::Int(value) => return Value::Int(*value),
            Value::Float(value) => return Value::Float(*value),
            Value::Bool(value) => return Value::Bool(*value),
            Value::String(value) => return Value::String(value.clone()),
            Value::Unit => return Value::Unit,
            _ => {}
        }
        enum Work<'a> {
            Value(&'a Value),
            Array(usize),
            Record(&'a StructValue),
        }
        let mut pending = vec![Work::Value(self)];
        let mut results = Vec::new();
        while let Some(work) = pending.pop() {
            match work {
                Work::Value(value) => match value {
                    Value::Int(value) => results.push(Value::Int(*value)),
                    Value::Float(value) => results.push(Value::Float(*value)),
                    Value::Bool(value) => results.push(Value::Bool(*value)),
                    Value::String(value) => results.push(Value::String(value.clone())),
                    Value::Unit => results.push(Value::Unit),
                    Value::Array(items) => {
                        pending.push(Work::Array(items.len()));
                        pending.extend(items.iter().rev().map(Work::Value));
                    }
                    Value::Struct(record) => {
                        pending.push(Work::Record(record));
                        pending.extend(
                            record
                                .fields
                                .iter()
                                .rev()
                                .map(|(_, value)| Work::Value(value)),
                        );
                    }
                },
                Work::Array(count) => {
                    let items = results.split_off(results.len() - count);
                    results.push(Value::Array(items));
                }
                Work::Record(record) => {
                    let values = results.split_off(results.len() - record.fields.len());
                    results.push(Value::Struct(Box::new(StructValue {
                        type_name: record.type_name.clone(),
                        fields: record
                            .fields
                            .iter()
                            .map(|(name, _)| name.clone())
                            .zip(values)
                            .collect(),
                    })));
                }
            }
        }
        results.pop().expect("one cloned root")
    }
}

impl Value {
    /// Consume a caller-owned value without recursively dropping compound children.
    /// Runtime-owned values use this traversal automatically. Ordinary Rust drop,
    /// Debug and PartialEq on caller-owned values retain their existing behavior;
    /// this method does not impose an allocation or execution budget.
    pub fn drop_iterative(self) {
        dispose(self);
    }

    pub(super) fn display(&self) -> String {
        self.display_with_limit(None)
            .expect("unlimited value formatting")
    }

    pub(super) fn display_with_limit(&self, limit: Option<usize>) -> Result<String, String> {
        fn append(text: &mut String, part: &str, limit: Option<usize>) -> Result<(), String> {
            if let Some(limit) = limit {
                if text.len() > limit || part.len() > limit - text.len() {
                    return Err(crate::compiler::execution::OUTPUT_LIMIT_ERROR.into());
                }
            }
            text.push_str(part);
            Ok(())
        }
        enum Work<'a> {
            Value(&'a Value),
            Text(&'a str),
        }
        let mut pending = vec![Work::Value(self)];
        let mut text = String::new();
        while let Some(work) = pending.pop() {
            match work {
                Work::Text(part) => append(&mut text, part, limit)?,
                Work::Value(value) => match value {
                    Self::Int(value) => append(&mut text, &value.to_string(), limit)?,
                    Self::Float(value) => append(&mut text, &value.to_string(), limit)?,
                    Self::Bool(value) => append(&mut text, &value.to_string(), limit)?,
                    Self::String(value) => append(&mut text, value, limit)?,
                    Self::Unit => {}
                    Self::Array(items) => {
                        append(&mut text, "[", limit)?;
                        pending.push(Work::Text("]"));
                        for (index, item) in items.iter().enumerate().rev() {
                            pending.push(Work::Value(item));
                            if index > 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    Self::Struct(record) => {
                        append(&mut text, &record.type_name, limit)?;
                        append(&mut text, " { ", limit)?;
                        pending.push(Work::Text(" }"));
                        for (index, (name, value)) in record.fields.iter().enumerate().rev() {
                            pending.push(Work::Value(value));
                            pending.push(Work::Text(": "));
                            pending.push(Work::Text(name));
                            if index > 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                },
            }
        }
        Ok(text)
    }
}

pub(super) fn dispose(value: Value) {
    if !matches!(value, Value::Array(_) | Value::Struct(_)) {
        return;
    }
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(items) => pending.extend(items),
            Value::Struct(record) => {
                pending.extend(record.fields.into_iter().map(|(_, value)| value))
            }
            _ => {}
        }
    }
}

pub(super) struct OwnedValue(Value);
impl OwnedValue {
    pub fn new(value: Value) -> Self {
        Self(value)
    }
    pub fn into_inner(mut self) -> Value {
        std::mem::replace(&mut self.0, Value::Unit)
    }
}
impl Deref for OwnedValue {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.0
    }
}
impl DerefMut for OwnedValue {
    fn deref_mut(&mut self) -> &mut Value {
        &mut self.0
    }
}
impl Drop for OwnedValue {
    fn drop(&mut self) {
        dispose(std::mem::replace(&mut self.0, Value::Unit));
    }
}

pub(super) struct OwnedValues(pub Vec<Value>);
impl Deref for OwnedValues {
    type Target = Vec<Value>;
    fn deref(&self) -> &Vec<Value> {
        &self.0
    }
}
impl DerefMut for OwnedValues {
    fn deref_mut(&mut self) -> &mut Vec<Value> {
        &mut self.0
    }
}
impl Drop for OwnedValues {
    fn drop(&mut self) {
        for value in self.0.drain(..) {
            dispose(value);
        }
    }
}
