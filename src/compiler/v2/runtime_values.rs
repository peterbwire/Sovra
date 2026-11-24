//! Model-v2 owned value arenas and enum materialization used by the opt-in Rust VM.
use super::{CheckedEnumTypes, EnumId};
use crate::compiler::{
    execution::{self, Budget, MaterializationLimits, RuntimePolicy},
    interpreter::{Value, ValueFootprint},
    semantic::Type,
};
use std::collections::{HashMap, HashSet};
/// Additive logical materialization model; the legacy model marker remains 1.
pub const VALUE_MODEL_V2: u32 = 2;
/// Value-layer errors, separate from future instruction/output/control/host errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValueError {
    /// Existing logical node allowance refused the whole result.
    NodeLimit,
    /// Existing logical text allowance refused the whole result.
    TextLimit,
    /// Logical size or representation index overflow.
    SizeOverflow,
    /// Wrong primitive/record/enum/array type or invalid record shape.
    TypeMismatch,
    /// Enum identity belongs to another checked environment or variant is unavailable.
    UnavailableEnum,
    /// Nullary versus single-payload constructor arity mismatch.
    PayloadArity,
    /// Extraction requires the specified nominal enum and selected variant.
    WrongVariant,
    /// An observed display allowance was exceeded before fuel/materialization.
    DisplayLimit,
    /// Index or field is unavailable.
    MissingMember,
}
impl std::fmt::Display for RuntimeValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "versioned runtime value refused: {self:?}")
    }
}
impl std::error::Error for RuntimeValueError {}
/// Opt-in model-v2 cumulative materialization budget, reusing v1 atomic admission.
/// This does not enforce instructions, host/output policy, interruption or physical heap.
pub struct RuntimeValueFuel {
    budget: Budget,
}
impl std::fmt::Debug for RuntimeValueFuel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeValueFuel")
            .field("remaining", &self.remaining())
            .finish()
    }
}
impl RuntimeValueFuel {
    /// Select explicit node/text allowances; no existing runtime defaults change.
    pub fn new(limits: MaterializationLimits) -> Self {
        Self {
            budget: Budget::new(RuntimePolicy {
                materialization: limits,
                ..RuntimePolicy::UNLIMITED
            }),
        }
    }
    /// Remaining optional allowances; releases never refund either counter.
    pub fn remaining(&self) -> MaterializationLimits {
        self.budget.materialization_remaining()
    }
    pub(super) fn for_runtime(policy: RuntimePolicy, control: execution::ExecutionControl) -> Self {
        Self {
            budget: Budget::with_control(policy, control),
        }
    }
    pub(super) fn checkpoint(&self) -> Result<(), String> {
        self.budget.checkpoint()
    }
    pub(super) fn instruction(&mut self) -> Result<(), String> {
        self.budget.instruction()
    }
    pub(super) fn preflight_output(&self, bytes: u64) -> Result<(), String> {
        self.budget.preflight_output(bytes)
    }
    pub(super) fn commit_output(&mut self, bytes: u64) -> Result<(), String> {
        self.budget.output_size(bytes)
    }
    pub(super) fn policy_remaining(&self) -> RuntimePolicy {
        RuntimePolicy {
            execution: self.budget.execution_remaining(),
            materialization: self.remaining(),
        }
    }
    pub(super) fn preflight(&self, nodes: u64, text: u64) -> Result<(), RuntimeValueError> {
        self.budget
            .preflight(nodes, text)
            .map_err(|e| match e.as_str() {
                execution::NODE_LIMIT_ERROR => RuntimeValueError::NodeLimit,
                execution::TEXT_LIMIT_ERROR => RuntimeValueError::TextLimit,
                _ => RuntimeValueError::SizeOverflow,
            })
    }
    pub(super) fn admit(&mut self, nodes: u64, text: u64) -> Result<(), RuntimeValueError> {
        self.budget
            .materialize(nodes, text)
            .map_err(|e| match e.as_str() {
                execution::NODE_LIMIT_ERROR => RuntimeValueError::NodeLimit,
                execution::TEXT_LIMIT_ERROR => RuntimeValueError::TextLimit,
                _ => RuntimeValueError::SizeOverflow,
            })
    }
}
#[derive(Debug, Clone)]
enum Node {
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<usize>),
    Record {
        owner: Option<super::RuntimeRecordId>,
        name: String,
        fields: Vec<(String, usize)>,
    },
    Enum {
        id: EnumId,
        name: String,
        variant: usize,
        label: String,
        payload: Option<usize>,
    },
}
impl Node {
    fn children(&self) -> Vec<usize> {
        match self {
            Self::Array(items) => items.clone(),
            Self::Record { fields, .. } => fields.iter().map(|(_, v)| *v).collect(),
            Self::Enum { payload, .. } => payload.iter().copied().collect(),
            _ => Vec::new(),
        }
    }
    fn rebase(&mut self, offset: usize) {
        match self {
            Self::Array(items) => {
                for v in items {
                    *v += offset;
                }
            }
            Self::Record { fields, .. } => {
                for (_, v) in fields {
                    *v += offset;
                }
            }
            Self::Enum {
                payload: Some(v), ..
            } => *v += offset,
            _ => {}
        }
    }
}
/// Opaque acyclic owned snapshot. Flat child indexes make ordinary drop/debug and
/// snapshots safe for deeply nested values without recursive enum/array destruction.
#[derive(Debug)]
pub struct RuntimeValueV2 {
    nodes: Vec<Node>,
    root: usize,
}
fn bytes(s: &str) -> Result<u64, RuntimeValueError> {
    u64::try_from(s.len()).map_err(|_| RuntimeValueError::SizeOverflow)
}
fn add(a: u64, b: u64) -> Result<u64, RuntimeValueError> {
    a.checked_add(b).ok_or(RuntimeValueError::SizeOverflow)
}
impl RuntimeValueV2 {
    // Trusted standard-record builders admit the whole record, labels and payloads
    // before creating any owned output text. Children below are already admitted.
    fn standard_record_admitted(name: &str, fields: Vec<(&str, Node)>) -> Self {
        let mut nodes = Vec::with_capacity(fields.len() + 1);
        let mut labels = Vec::with_capacity(fields.len());
        for (label, node) in fields {
            labels.push((label.to_owned(), nodes.len()));
            nodes.push(node);
        }
        let root = nodes.len();
        nodes.push(Node::Record {
            owner: None,
            name: name.to_owned(),
            fields: labels,
        });
        Self { nodes, root }
    }
    fn standard_labels(name: &str, labels: &[&str]) -> Result<u64, RuntimeValueError> {
        labels
            .iter()
            .try_fold(bytes(name)?, |n, label| add(n, bytes(label)?))
    }
    pub(super) fn preflight_text_read(fuel: &RuntimeValueFuel) -> Result<(), RuntimeValueError> {
        fuel.preflight(
            4,
            Self::standard_labels(
                crate::compiler::stdlib::TEXT_READ_TYPE,
                &["ok", "text", "error"],
            )?,
        )
    }
    pub(super) fn preflight_text_write(fuel: &RuntimeValueFuel) -> Result<(), RuntimeValueError> {
        fuel.preflight(
            3,
            Self::standard_labels(crate::compiler::stdlib::TEXT_WRITE_TYPE, &["ok", "error"])?,
        )
    }
    pub(super) fn text_read(
        result: crate::compiler::text_files::TextRead,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let name = crate::compiler::stdlib::TEXT_READ_TYPE;
        let labels = Self::standard_labels(name, &["ok", "text", "error"])?;
        fuel.admit(
            4,
            add(labels, add(bytes(&result.text)?, bytes(&result.error)?)?)?,
        )?;
        Ok(Self::standard_record_admitted(
            name,
            vec![
                ("ok", Node::Bool(result.ok)),
                ("text", Node::String(result.text)),
                ("error", Node::String(result.error)),
            ],
        ))
    }
    pub(super) fn text_write(
        result: crate::compiler::text_files::TextWrite,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let name = crate::compiler::stdlib::TEXT_WRITE_TYPE;
        let labels = Self::standard_labels(name, &["ok", "error"])?;
        fuel.admit(3, add(labels, bytes(&result.error)?)?)?;
        Ok(Self::standard_record_admitted(
            name,
            vec![
                ("ok", Node::Bool(result.ok)),
                ("error", Node::String(result.error)),
            ],
        ))
    }
    pub(super) fn preflight_input_line(fuel: &RuntimeValueFuel) -> Result<(), RuntimeValueError> {
        let name = crate::compiler::stdlib::INPUT_LINE_TYPE;
        fuel.preflight(3, Self::standard_labels(name, &["eof", "text"])?)
    }
    pub(super) fn input_line(
        text: Option<String>,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let name = crate::compiler::stdlib::INPUT_LINE_TYPE;
        let labels = Self::standard_labels(name, &["eof", "text"])?;
        fuel.admit(3, add(labels, bytes(text.as_deref().unwrap_or(""))?)?)?;
        Ok(Self::standard_record_admitted(
            name,
            vec![
                ("eof", Node::Bool(text.is_none())),
                ("text", Node::String(text.unwrap_or_default())),
            ],
        ))
    }
    pub(super) fn split_once(
        text: &str,
        delimiter: &str,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let (found, before, after) = match text.split_once(delimiter) {
            Some((before, after)) => (true, before, after),
            None => (false, text, ""),
        };
        let name = crate::compiler::stdlib::SPLIT_ONCE_TYPE;
        let labels = Self::standard_labels(name, &["found", "before", "after"])?;
        fuel.admit(4, add(labels, add(bytes(before)?, bytes(after)?)?)?)?;
        Ok(Self::standard_record_admitted(
            name,
            vec![
                ("found", Node::Bool(found)),
                ("before", Node::String(before.to_owned())),
                ("after", Node::String(after.to_owned())),
            ],
        ))
    }
    pub(super) fn parse_int(
        text: &str,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let value = crate::compiler::stdlib::parse_int(text);
        let name = crate::compiler::stdlib::PARSED_INT_TYPE;
        fuel.admit(3, Self::standard_labels(name, &["ok", "value"])?)?;
        Ok(Self::standard_record_admitted(
            name,
            vec![
                ("ok", Node::Bool(value.is_some())),
                ("value", Node::Int(value.unwrap_or_default())),
            ],
        ))
    }
    pub(super) fn escape_terminal(
        text: &str,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let size = text.chars().try_fold(0u64, |n, c| {
            let point = c as u32;
            add(
                n,
                if point <= 0x1f || (0x7f..=0x9f).contains(&point) {
                    8
                } else {
                    c.len_utf8() as u64
                },
            )
        })?;
        usize::try_from(size).map_err(|_| RuntimeValueError::SizeOverflow)?;
        fuel.admit(1, size)?;
        Ok(Self::string_admitted(
            crate::compiler::stdlib::escape_terminal(text),
        ))
    }
    /// Borrow a scalar Int without materializing a new language value.
    pub fn as_int(&self) -> Option<i64> {
        if let Node::Int(n) = self.nodes[self.root] {
            Some(n)
        } else {
            None
        }
    }
    /// Borrow a scalar Float without materialization.
    pub fn as_float(&self) -> Option<f64> {
        if let Node::Float(n) = self.nodes[self.root] {
            Some(n)
        } else {
            None
        }
    }
    /// Borrow a scalar Bool for trusted host/control inspection.
    pub fn as_bool(&self) -> Option<bool> {
        if let Node::Bool(n) = self.nodes[self.root] {
            Some(n)
        } else {
            None
        }
    }
    /// Borrow String contents; this does not grant a mutable reference.
    pub fn as_str(&self) -> Option<&str> {
        if let Node::String(s) = &self.nodes[self.root] {
            Some(s)
        } else {
            None
        }
    }
    /// Inspect array length without materializing a language Int.
    pub fn array_len(&self) -> Option<usize> {
        if let Node::Array(items) = &self.nodes[self.root] {
            Some(items.len())
        } else {
            None
        }
    }
    pub(super) fn bool_admitted(value: bool) -> Self {
        Self {
            nodes: vec![Node::Bool(value)],
            root: 0,
        }
    }
    pub(super) fn unit_admitted() -> Self {
        Self {
            nodes: vec![Node::Unit],
            root: 0,
        }
    }
    pub(super) fn string_admitted(value: String) -> Self {
        Self {
            nodes: vec![Node::String(value)],
            root: 0,
        }
    }
    pub(super) fn coerce(
        mut self,
        env: &CheckedEnumTypes,
        expected: &Type,
    ) -> Result<Self, RuntimeValueError> {
        let widen = self.validate_type(env, expected)?;
        for index in widen {
            let Node::Int(n) = self.nodes[index] else {
                unreachable!("validated widening")
            };
            self.nodes[index] = Node::Float(n as f64);
        }
        Ok(self)
    }
    pub(super) fn validate_as(
        &self,
        env: &CheckedEnumTypes,
        expected: &Type,
    ) -> Result<(), RuntimeValueError> {
        self.validate_type(env, expected).map(|_| ())
    }
    pub(super) fn store_index(
        mut self,
        index: usize,
        value: Self,
    ) -> Result<Self, RuntimeValueError> {
        let Node::Array(items) = &self.nodes[self.root] else {
            return Err(RuntimeValueError::TypeMismatch);
        };
        if index >= items.len() {
            return Err(RuntimeValueError::MissingMember);
        }
        let Some(Node::Array(roots)) = self.nodes.pop() else {
            return Err(RuntimeValueError::TypeMismatch);
        };
        let mut input = self.nodes.into_iter();
        let mut values = Vec::with_capacity(roots.len());
        let mut offset = 0usize;
        for root in roots {
            let len = root - offset + 1;
            let mut nodes: Vec<_> = input.by_ref().take(len).collect();
            for node in &mut nodes {
                match node {
                    Node::Array(items) => {
                        for id in items {
                            *id -= offset;
                        }
                    }
                    Node::Record { fields, .. } => {
                        for (_, id) in fields {
                            *id -= offset;
                        }
                    }
                    Node::Enum {
                        payload: Some(id), ..
                    } => *id -= offset,
                    _ => {}
                }
            }
            values.push(Self {
                nodes,
                root: len - 1,
            });
            offset = root + 1;
        }
        values[index] = value;
        let (mut nodes, roots) = Self::merge(values)?;
        let root = nodes.len();
        nodes.push(Node::Array(roots));
        Ok(Self { nodes, root })
    }
    fn scalar(
        node: Node,
        text: u64,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        fuel.admit(1, text)?;
        Ok(Self {
            nodes: vec![node],
            root: 0,
        })
    }
    /// Materialize a signed integer.
    pub fn int(value: i64, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeValueError> {
        Self::scalar(Node::Int(value), 0, fuel)
    }
    /// Materialize a Float with the legacy numeric representation contract.
    pub fn float(value: f64, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeValueError> {
        Self::scalar(Node::Float(value), 0, fuel)
    }
    /// Materialize a Bool (control-only enum tag inspection uses no Bool value).
    pub fn boolean(value: bool, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeValueError> {
        Self::scalar(Node::Bool(value), 0, fuel)
    }
    /// Materialize Unit, distinct from a nullary variant.
    pub fn unit(fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeValueError> {
        Self::scalar(Node::Unit, 0, fuel)
    }
    /// Admit String text before copying it.
    pub fn string(value: &str, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeValueError> {
        fuel.admit(1, bytes(value)?)?;
        Ok(Self {
            nodes: vec![Node::String(value.into())],
            root: 0,
        })
    }
    /// Import a snapshot of a legacy value after complete shape/size validation.
    /// Legacy public Value variants and their ordinary caller-owned drop remain unchanged.
    pub fn from_legacy(
        value: &Value,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let mut scan = vec![value];
        while let Some(value) = scan.pop() {
            match value {
                Value::Array(items) => {
                    length(items.len())?;
                    scan.extend(items);
                }
                Value::Struct(record) => {
                    let mut names = HashSet::new();
                    for (name, value) in &record.fields {
                        if !names.insert(name) {
                            return Err(RuntimeValueError::TypeMismatch);
                        }
                        scan.push(value);
                    }
                }
                _ => {}
            }
        }
        let size = value
            .logical_footprint()
            .ok_or(RuntimeValueError::SizeOverflow)?;
        fuel.admit(size.value_nodes, size.owned_text_bytes)?;
        enum Work<'a> {
            Visit(&'a Value),
            Array(usize),
            Record(&'a crate::compiler::interpreter::StructValue),
        }
        let mut work = vec![Work::Visit(value)];
        let mut nodes = Vec::new();
        let mut results = Vec::new();
        while let Some(task) = work.pop() {
            let node = match task {
                Work::Visit(value) => match value {
                    Value::Unit => Node::Unit,
                    Value::Int(n) => Node::Int(*n),
                    Value::Float(n) => Node::Float(*n),
                    Value::Bool(n) => Node::Bool(*n),
                    Value::String(s) => Node::String(s.clone()),
                    Value::Array(items) => {
                        work.push(Work::Array(items.len()));
                        work.extend(items.iter().rev().map(Work::Visit));
                        continue;
                    }
                    Value::Struct(r) => {
                        work.push(Work::Record(r));
                        work.extend(r.fields.iter().rev().map(|(_, v)| Work::Visit(v)));
                        continue;
                    }
                },
                Work::Array(n) => Node::Array(results.split_off(results.len() - n)),
                Work::Record(r) => {
                    let children = results.split_off(results.len() - r.fields.len());
                    Node::Record {
                        owner: None,
                        name: r.type_name.clone(),
                        fields: r
                            .fields
                            .iter()
                            .map(|(n, _)| n.clone())
                            .zip(children)
                            .collect(),
                    }
                }
            };
            results.push(nodes.len());
            nodes.push(node);
        }
        Ok(Self {
            nodes,
            root: results[0],
        })
    }
    fn merge(values: Vec<Self>) -> Result<(Vec<Node>, Vec<usize>), RuntimeValueError> {
        let total = values
            .iter()
            .try_fold(0usize, |n, v| n.checked_add(v.nodes.len()))
            .ok_or(RuntimeValueError::SizeOverflow)?;
        total
            .checked_add(1)
            .ok_or(RuntimeValueError::SizeOverflow)?;
        let mut nodes = Vec::with_capacity(total);
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            let offset = nodes.len();
            roots.push(value.root + offset);
            for mut node in value.nodes {
                node.rebase(offset);
                nodes.push(node);
            }
        }
        Ok((nodes, roots))
    }
    /// Transfer existing children and materialize only one array wrapper.
    /// Rust callers may build heterogeneous arrays; typed enum ingress validates every element.
    pub fn array(
        values: Vec<Self>,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        length(values.len())?;
        values
            .iter()
            .try_fold(1usize, |n, v| n.checked_add(v.nodes.len()))
            .ok_or(RuntimeValueError::SizeOverflow)?;
        fuel.admit(1, 0)?;
        let (mut nodes, roots) = Self::merge(values)?;
        let root = nodes.len();
        nodes.push(Node::Array(roots));
        Ok(Self { nodes, root })
    }
    /// Transfer record children and admit its wrapper/type/field labels without double charging.
    pub fn record(
        name: &str,
        fields: Vec<(&str, Self)>,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let mut seen = HashSet::new();
        let mut text = bytes(name)?;
        let mut total = 1usize;
        for (label, value) in &fields {
            if !seen.insert(*label) {
                return Err(RuntimeValueError::TypeMismatch);
            }
            text = add(text, bytes(label)?)?;
            total = total
                .checked_add(value.nodes.len())
                .ok_or(RuntimeValueError::SizeOverflow)?;
        }
        fuel.admit(1, text)?;
        let (labels, values): (Vec<_>, Vec<_>) = fields.into_iter().unzip();
        let (mut nodes, roots) = Self::merge(values)?;
        let root = nodes.len();
        nodes.push(Node::Record {
            owner: None,
            name: name.into(),
            fields: labels.into_iter().map(str::to_owned).zip(roots).collect(),
        });
        Ok(Self { nodes, root })
    }
    /// Construct a sealed record from checked schema evidence with exact field types.
    /// Children transfer; only wrapper/type/field labels consume new materialization.
    /// Validation and transfer work use the caller's finite shared collection allowance.
    pub fn checked_record(
        env: &CheckedEnumTypes,
        id: super::RuntimeRecordId,
        fields: Vec<(&str, Self)>,
        fuel: &mut RuntimeValueFuel,
        work: &mut super::RuntimeSetWork,
    ) -> Result<Self, super::RuntimeMapError> {
        use super::{RuntimeMapError, RuntimeSetError};
        let (name, schema) = env.record_schema(id).ok_or(RuntimeMapError::ValueType)?;
        if fields.len() != schema.len() {
            return Err(RuntimeMapError::ValueType);
        }
        let mut seen = HashSet::new();
        let mut text = name.len() as u64;
        let mut total = 1usize;
        for (label, value) in &fields {
            work.charge(
                1u64.checked_add(label.len() as u64)
                    .ok_or(RuntimeSetError::Capacity)?,
            )?;
            if !seen.insert(*label) {
                return Err(RuntimeMapError::ValueType);
            }
            let ty = schema.get(*label).ok_or(RuntimeMapError::ValueType)?;
            value.collection_check_type(env, ty, work)?;
            // Validate transitive ownership, including legacy enum payloads.
            work.charge(1)?;
            value.collection_enum_size(value.root, work)?;
            total = total
                .checked_add(value.nodes.len())
                .ok_or(RuntimeSetError::Capacity)?;
            text = text
                .checked_add(label.len() as u64)
                .ok_or(RuntimeSetError::Capacity)?;
        }
        fuel.preflight(1, text)?;
        work.charge(
            (total as u64)
                .checked_add(text)
                .ok_or(RuntimeSetError::Capacity)?,
        )?;
        fuel.admit(1, text)?;
        let (labels, values): (Vec<_>, Vec<_>) = fields.into_iter().unzip();
        let (mut nodes, roots) = Self::merge(values)?;
        let root = nodes.len();
        nodes.push(Node::Record {
            owner: Some(id),
            name: name.to_owned(),
            fields: labels.into_iter().map(str::to_owned).zip(roots).collect(),
        });
        Ok(Self { nodes, root })
    }
    fn collection_check_type(
        &self,
        env: &CheckedEnumTypes,
        expected: &Type,
        work: &mut super::RuntimeSetWork,
    ) -> Result<(), super::RuntimeMapError> {
        use super::RuntimeMapError;
        let mut pending = vec![(self.root, expected)];
        while let Some((index, ty)) = pending.pop() {
            work.charge(1)?;
            match (ty, &self.nodes[index]) {
                (Type::Unit, Node::Unit)
                | (Type::Int, Node::Int(_))
                | (Type::Float, Node::Float(_))
                | (Type::Bool, Node::Bool(_))
                | (Type::String, Node::String(_)) => {}
                (Type::Array(item), Node::Array(values)) => {
                    work.charge(values.len() as u64)?;
                    pending.extend(values.iter().map(|index| (*index, item.as_ref())));
                }
                (Type::Named(name), Node::Enum { id, .. }) if env.find(name) == Some(*id) => {}
                (
                    Type::Named(name),
                    Node::Record {
                        owner: Some(id), ..
                    },
                ) if env.find_record(name) == Some(*id) => {}
                _ => return Err(RuntimeMapError::ValueType),
            }
        }
        Ok(())
    }
    /// Materialize one nominal enum wrapper and owned labels, transferring the existing payload.
    /// Complete payload type validation precedes admission; numeric widening costs zero as in v1.
    pub fn enum_value(
        env: &CheckedEnumTypes,
        id: EnumId,
        variant: usize,
        mut payload: Option<Self>,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let decl = env
            .declarations()
            .get(id.ordinal())
            .filter(|d| d.id == id)
            .ok_or(RuntimeValueError::UnavailableEnum)?;
        let fact = decl
            .variants
            .get(variant)
            .ok_or(RuntimeValueError::UnavailableEnum)?;
        if payload.is_some() != fact.payload.is_some() {
            return Err(RuntimeValueError::PayloadArity);
        }
        let mut widen = Vec::new();
        if let (Some(value), Some(expected)) = (&payload, fact.payload) {
            let expected = env
                .body_payload_type(expected)
                .map_err(|_| RuntimeValueError::TypeMismatch)?;
            widen = value.validate_type(env, &expected)?;
        }
        payload
            .as_ref()
            .map_or(Some(1), |v| v.nodes.len().checked_add(1))
            .ok_or(RuntimeValueError::SizeOverflow)?;
        let text = add(bytes(&decl.name)?, bytes(&fact.name)?)?;
        fuel.admit(1, text)?;
        if let Some(value) = &mut payload {
            for index in widen {
                let Node::Int(n) = value.nodes[index] else {
                    unreachable!("validated widening")
                };
                value.nodes[index] = Node::Float(n as f64);
            }
        }
        let (mut nodes, child) = if let Some(value) = payload {
            (value.nodes, Some(value.root))
        } else {
            (Vec::new(), None)
        };
        let root = nodes.len();
        nodes.push(Node::Enum {
            id,
            name: decl.name.clone(),
            variant,
            label: fact.name.clone(),
            payload: child,
        });
        Ok(Self { nodes, root })
    }
    fn validate_type(
        &self,
        env: &CheckedEnumTypes,
        expected: &Type,
    ) -> Result<Vec<usize>, RuntimeValueError> {
        let mut leaf = expected;
        while let Type::Array(item) = leaf {
            leaf = item;
        }
        let mut standard = HashMap::new();
        if matches!(leaf,Type::Named(name) if env.find(name).is_none()) {
            crate::compiler::semantic::add_standard_record_fields(&mut standard);
        }
        let mut pending = vec![(self.root, expected, true)];
        let mut widening = Vec::new();
        while let Some((index, expected, widen)) = pending.pop() {
            match (expected, &self.nodes[index]) {
                (Type::Unit, Node::Unit)
                | (Type::Bool, Node::Bool(_))
                | (Type::Int, Node::Int(_))
                | (Type::Float, Node::Float(_))
                | (Type::String, Node::String(_)) => {}
                (Type::Float, Node::Int(_)) if widen => widening.push(index),
                (Type::Array(item), Node::Array(values)) => {
                    pending.extend(values.iter().map(|v| (*v, item.as_ref(), widen)))
                }
                (Type::Named(name), node) => {
                    if let Some(id) = env.find(name) {
                        if !matches!(node,Node::Enum {id:actual,..} if *actual==id) {
                            return Err(RuntimeValueError::TypeMismatch);
                        }
                    } else {
                        let Node::Record {
                            name: actual,
                            fields,
                            owner,
                        } = node
                        else {
                            return Err(RuntimeValueError::TypeMismatch);
                        };
                        if actual != name
                            || owner.is_some_and(|id| Some(id) != env.find_record(name))
                        {
                            return Err(RuntimeValueError::TypeMismatch);
                        }
                        let schema = env
                            .body_record_fields()
                            .get(name)
                            .or_else(|| standard.get(name))
                            .ok_or(RuntimeValueError::TypeMismatch)?;
                        if fields.len() != schema.len() {
                            return Err(RuntimeValueError::TypeMismatch);
                        }
                        for (name, value) in fields {
                            let ty = schema.get(name).ok_or(RuntimeValueError::TypeMismatch)?;
                            pending.push((*value, ty, false));
                        }
                    }
                }
                _ => return Err(RuntimeValueError::TypeMismatch),
            }
        }
        Ok(widening)
    }
    /// Inspect a control-only tag without allocating or charging a VM Bool.
    pub fn enum_tag(&self) -> Option<(EnumId, usize)> {
        match self.nodes[self.root] {
            Node::Enum { id, variant, .. } => Some((id, variant)),
            _ => None,
        }
    }
    /// Copy a guarded variant payload, admitting its entire footprint before copying.
    pub fn payload_snapshot(
        &self,
        id: EnumId,
        variant: usize,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let Node::Enum {
            id: actual,
            variant: tag,
            payload,
            ..
        } = &self.nodes[self.root]
        else {
            return Err(RuntimeValueError::WrongVariant);
        };
        if *actual != id || *tag != variant {
            return Err(RuntimeValueError::WrongVariant);
        }
        self.snapshot_at(payload.ok_or(RuntimeValueError::PayloadArity)?, fuel)
    }
    // Collection ingress is exact and bounded; nominal records require sealed owners.
    pub(super) fn collection_shape(
        &self,
        ty: super::RuntimeMapValueType,
        work: &mut super::RuntimeSetWork,
    ) -> Result<(u64, u64), super::RuntimeMapError> {
        use super::{RuntimeMapError, RuntimeMapPrimitive as P, RuntimeSetError};
        let mut pending = vec![(self.root, ty.array_depth())];
        let (mut nodes, mut text) = (0u64, 0u64);
        while let Some((index, depth)) = pending.pop() {
            work.charge(1)?;
            nodes = nodes.checked_add(1).ok_or(RuntimeSetError::Capacity)?;
            let node = &self.nodes[index];
            if depth > 0 {
                let Node::Array(items) = node else {
                    return Err(RuntimeMapError::ValueType);
                };
                work.charge(items.len() as u64)?;
                pending.extend(items.iter().map(|i| (*i, depth - 1)));
            } else {
                match (ty.leaf(), node) {
                    (P::Unit, Node::Unit)
                    | (P::Int, Node::Int(_))
                    | (P::Float, Node::Float(_))
                    | (P::Bool, Node::Bool(_)) => {}
                    (P::String, Node::String(value)) => {
                        text = text
                            .checked_add(value.len() as u64)
                            .ok_or(RuntimeSetError::Capacity)?;
                    }
                    (
                        P::Record(expected),
                        Node::Record {
                            owner: Some(actual),
                            ..
                        },
                    ) if expected == *actual => {
                        let (owned_nodes, owned_text) = self.collection_enum_size(index, work)?;
                        nodes = nodes
                            .checked_add(owned_nodes - 1)
                            .ok_or(RuntimeSetError::Capacity)?;
                        text = text
                            .checked_add(owned_text)
                            .ok_or(RuntimeSetError::Capacity)?;
                    }
                    (P::Enum(expected), Node::Enum { id, .. }) if expected == *id => {
                        // Root constructor already sealed enum shape/identity. Recount every
                        // descendant under work before copying; reject legacy named records.
                        let (payload_nodes, payload_text) =
                            self.collection_enum_size(index, work)?;
                        nodes = nodes
                            .checked_add(payload_nodes - 1)
                            .ok_or(RuntimeSetError::Capacity)?;
                        text = text
                            .checked_add(payload_text)
                            .ok_or(RuntimeSetError::Capacity)?;
                    }
                    _ => return Err(RuntimeMapError::ValueType),
                }
            }
        }
        Ok((nodes, text))
    }
    fn collection_enum_size(
        &self,
        root: usize,
        work: &mut super::RuntimeSetWork,
    ) -> Result<(u64, u64), super::RuntimeMapError> {
        use super::{RuntimeMapError, RuntimeSetError};
        let mut pending = vec![root];
        let (mut nodes, mut text) = (0u64, 0u64);
        while let Some(index) = pending.pop() {
            // The enclosing shape check already charged and checked the root.
            if index != root {
                work.charge(1)?;
            }
            nodes = nodes.checked_add(1).ok_or(RuntimeSetError::Capacity)?;
            let owned = match &self.nodes[index] {
                Node::Unit | Node::Bool(_) | Node::Int(_) | Node::Float(_) => 0,
                Node::String(value) => value.len() as u64,
                Node::Array(items) => {
                    work.charge(items.len() as u64)?;
                    pending.extend(items);
                    0
                }
                Node::Enum {
                    name,
                    label,
                    payload,
                    ..
                } => {
                    if let Some(child) = payload {
                        work.charge(1)?;
                        pending.push(*child);
                    }
                    (name.len() as u64)
                        .checked_add(label.len() as u64)
                        .ok_or(RuntimeSetError::Capacity)?
                }
                Node::Record {
                    owner: Some(_),
                    name,
                    fields,
                } => {
                    let mut bytes = name.len() as u64;
                    for (label, child) in fields {
                        work.charge(
                            1u64.checked_add(label.len() as u64)
                                .ok_or(RuntimeSetError::Capacity)?,
                        )?;
                        bytes = bytes
                            .checked_add(label.len() as u64)
                            .ok_or(RuntimeSetError::Capacity)?;
                        pending.push(*child);
                    }
                    bytes
                }
                Node::Record { owner: None, .. } => return Err(RuntimeMapError::ValueType),
            };
            text = text.checked_add(owned).ok_or(RuntimeSetError::Capacity)?;
        }
        Ok((nodes, text))
    }
    // Callers must atomically admit the complete collection result before using this.
    pub(super) fn collection_copy_admitted(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            root: self.root,
        }
    }
    /// Full owned snapshot, admitted atomically before label/payload copying.
    pub fn snapshot(&self, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeValueError> {
        let size = self.logical_footprint()?;
        fuel.admit(size.value_nodes, size.owned_text_bytes)?;
        Ok(Self {
            nodes: self.nodes.clone(),
            root: self.root,
        })
    }
    /// Snapshot one array element after validating the index.
    pub fn array_item_snapshot(
        &self,
        index: usize,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let Node::Array(items) = &self.nodes[self.root] else {
            return Err(RuntimeValueError::TypeMismatch);
        };
        self.snapshot_at(
            *items.get(index).ok_or(RuntimeValueError::MissingMember)?,
            fuel,
        )
    }
    /// Root record owner from checked construction; legacy/name-only records return None.
    pub fn record_owner(&self) -> Option<super::RuntimeRecordId> {
        match &self.nodes[self.root] {
            Node::Record { owner, .. } => *owner,
            _ => None,
        }
    }
    /// Snapshot one record field after validating its label.
    pub fn field_snapshot(
        &self,
        name: &str,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let Node::Record { fields, .. } = &self.nodes[self.root] else {
            return Err(RuntimeValueError::TypeMismatch);
        };
        self.snapshot_at(
            fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| *v)
                .ok_or(RuntimeValueError::MissingMember)?,
            fuel,
        )
    }
    // Premeasure and atomically admit the whole result before cloning any selected payload.
    fn array_snapshot_parts(
        parts: &[(&Self, &[usize])],
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let count = parts.iter().try_fold(0usize, |n, (_, roots)| {
            n.checked_add(roots.len())
                .ok_or(RuntimeValueError::SizeOverflow)
        })?;
        length(count)?;
        let (mut nodes, mut text) = (1u64, 0u64);
        for (value, roots) in parts {
            for root in *roots {
                let size = value.footprint_at(*root)?;
                nodes = add(nodes, size.value_nodes)?;
                text = add(text, size.owned_text_bytes)?;
            }
        }
        usize::try_from(nodes).map_err(|_| RuntimeValueError::SizeOverflow)?;
        fuel.admit(nodes, text)?;
        let mut children = Vec::with_capacity(count);
        for (value, roots) in parts {
            children.extend(roots.iter().map(|root| value.snapshot_admitted(*root)));
        }
        let (mut nodes, roots) = Self::merge(children)?;
        let root = nodes.len();
        nodes.push(Node::Array(roots));
        Ok(Self { nodes, root })
    }
    pub(super) fn slice_snapshot(
        &self,
        start: usize,
        end: usize,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let Node::Array(items) = &self.nodes[self.root] else {
            return Err(RuntimeValueError::TypeMismatch);
        };
        let selected = items
            .get(start..end)
            .ok_or(RuntimeValueError::MissingMember)?;
        Self::array_snapshot_parts(&[(self, selected)], fuel)
    }
    pub(super) fn growth_snapshot(
        &self,
        right: &Self,
        concat: bool,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let Node::Array(left) = &self.nodes[self.root] else {
            return Err(RuntimeValueError::TypeMismatch);
        };
        let single = [right.root];
        let roots = if concat {
            let Node::Array(items) = &right.nodes[right.root] else {
                return Err(RuntimeValueError::TypeMismatch);
            };
            items.as_slice()
        } else {
            &single
        };
        Self::array_snapshot_parts(&[(self, left), (right, roots)], fuel)
    }
    fn snapshot_at(
        &self,
        root: usize,
        fuel: &mut RuntimeValueFuel,
    ) -> Result<Self, RuntimeValueError> {
        let size = self.footprint_at(root)?;
        fuel.admit(size.value_nodes, size.owned_text_bytes)?;
        Ok(self.snapshot_admitted(root))
    }
    fn snapshot_admitted(&self, root: usize) -> Self {
        let mut pending = vec![(root, false)];
        let mut results = Vec::new();
        let mut nodes = Vec::new();
        while let Some((index, finish)) = pending.pop() {
            let source = &self.nodes[index];
            let children = source.children();
            if !finish && !children.is_empty() {
                pending.push((index, true));
                pending.extend(children.into_iter().rev().map(|c| (c, false)));
                continue;
            }
            let ids = results.split_off(results.len() - children.len());
            let node = match source {
                Node::Array(_) => Node::Array(ids),
                Node::Record {
                    owner,
                    name,
                    fields,
                } => Node::Record {
                    owner: *owner,
                    name: name.clone(),
                    fields: fields.iter().map(|(n, _)| n.clone()).zip(ids).collect(),
                },
                Node::Enum {
                    id,
                    name,
                    variant,
                    label,
                    payload,
                } => Node::Enum {
                    id: *id,
                    name: name.clone(),
                    variant: *variant,
                    label: label.clone(),
                    payload: payload.map(|_| ids[0]),
                },
                _ => source.clone(),
            };
            results.push(nodes.len());
            nodes.push(node);
        }
        Self {
            nodes,
            root: results[0],
        }
    }
    /// Logical v2 nodes, owned text and display bytes; no full compound formatting/copying.
    pub fn logical_footprint(&self) -> Result<ValueFootprint, RuntimeValueError> {
        self.footprint_at(self.root)
    }
    fn footprint_at(&self, root: usize) -> Result<ValueFootprint, RuntimeValueError> {
        let mut size = ValueFootprint::default();
        let mut pending = vec![root];
        while let Some(index) = pending.pop() {
            size.value_nodes = add(size.value_nodes, 1)?;
            let (text, display) = match &self.nodes[index] {
                Node::Unit => (0, 0),
                Node::Int(n) => (0, bytes(&n.to_string())?),
                Node::Float(n) => (0, bytes(&n.to_string())?),
                Node::Bool(n) => (0, if *n { 4 } else { 5 }),
                Node::String(s) => (bytes(s)?, bytes(s)?),
                Node::Array(items) => {
                    pending.extend(items);
                    (0, add(2, separators(items.len())?)?)
                }
                Node::Record { name, fields, .. } => {
                    let mut text = bytes(name)?;
                    let mut display = add(add(text, 5)?, separators(fields.len())?)?;
                    for (label, value) in fields {
                        text = add(text, bytes(label)?)?;
                        display = add(display, add(bytes(label)?, 2)?)?;
                        pending.push(*value);
                    }
                    (text, display)
                }
                Node::Enum {
                    name,
                    label,
                    payload,
                    ..
                } => {
                    pending.extend(payload);
                    let text = add(bytes(name)?, bytes(label)?)?;
                    (text, add(text, 4)?)
                }
            };
            size.owned_text_bytes = add(size.owned_text_bytes, text)?;
            size.display_bytes = add(size.display_bytes, display)?;
        }
        Ok(size)
    }
    /// Materialize one display String under fuel and an independent optional byte cap.
    /// This does not publish output or consume an output/LF allowance.
    pub fn render(
        &self,
        fuel: &mut RuntimeValueFuel,
        max_bytes: Option<u64>,
    ) -> Result<String, RuntimeValueError> {
        let size = self.logical_footprint()?;
        if max_bytes.is_some_and(|n| size.display_bytes > n) {
            return Err(RuntimeValueError::DisplayLimit);
        }
        usize::try_from(size.display_bytes).map_err(|_| RuntimeValueError::SizeOverflow)?;
        fuel.admit(1, size.display_bytes)?;
        Ok(self.render_admitted())
    }
    pub(super) fn render_admitted(&self) -> String {
        enum Work<'a> {
            Node(usize),
            Text(&'a str),
        }
        let mut pending = vec![Work::Node(self.root)];
        let mut text = String::new();
        while let Some(work) = pending.pop() {
            match work {
                Work::Text(s) => text.push_str(s),
                Work::Node(index) => match &self.nodes[index] {
                    Node::Unit => {}
                    Node::Int(n) => text.push_str(&n.to_string()),
                    Node::Float(n) => text.push_str(&n.to_string()),
                    Node::Bool(n) => text.push_str(if *n { "true" } else { "false" }),
                    Node::String(s) => text.push_str(s),
                    Node::Array(items) => {
                        text.push('[');
                        pending.push(Work::Text("]"));
                        for (i, v) in items.iter().enumerate().rev() {
                            pending.push(Work::Node(*v));
                            if i > 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    Node::Record { name, fields, .. } => {
                        text.push_str(name);
                        text.push_str(" { ");
                        pending.push(Work::Text(" }"));
                        for (i, (label, v)) in fields.iter().enumerate().rev() {
                            pending.push(Work::Node(*v));
                            pending.push(Work::Text(": "));
                            pending.push(Work::Text(label));
                            if i > 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    Node::Enum {
                        name,
                        label,
                        payload,
                        ..
                    } => {
                        text.push_str(name);
                        text.push_str("::");
                        text.push_str(label);
                        text.push('(');
                        pending.push(Work::Text(")"));
                        if let Some(value) = payload {
                            pending.push(Work::Node(*value));
                        }
                    }
                },
            }
        }
        text
    }
    /// Structural snapshot equality, with exact enum identity/variant and legacy numeric rules.
    pub fn structurally_equal(&self, other: &Self) -> bool {
        self.compare_values(other, None).unwrap_or(false)
    }
    // Caller admits the linear node/text comparison; charge record field searches additionally.
    pub(super) fn collection_equal_admitted(
        &self,
        other: &Self,
        work: &mut super::RuntimeSetWork,
    ) -> Result<bool, super::RuntimeSetError> {
        self.compare_values(other, Some(work))
    }
    fn compare_values(
        &self,
        other: &Self,
        mut work: Option<&mut super::RuntimeSetWork>,
    ) -> Result<bool, super::RuntimeSetError> {
        let mut pending = vec![(self.root, other.root)];
        while let Some((a, b)) = pending.pop() {
            match (&self.nodes[a], &other.nodes[b]) {
                (Node::Unit, Node::Unit) => {}
                (Node::Int(a), Node::Int(b)) if a == b => {}
                (Node::Float(a), Node::Float(b)) if a == b => {}
                (Node::Int(a), Node::Float(b)) if (*a as f64) == *b => {}
                (Node::Float(a), Node::Int(b)) if *a == (*b as f64) => {}
                (Node::Bool(a), Node::Bool(b)) if a == b => {}
                (Node::String(a), Node::String(b)) if a == b => {}
                (Node::Array(a), Node::Array(b)) if a.len() == b.len() => {
                    pending.extend(a.iter().copied().zip(b.iter().copied()))
                }
                (
                    Node::Record {
                        owner: oa,
                        name: a,
                        fields: x,
                    },
                    Node::Record {
                        owner: ob,
                        name: b,
                        fields: y,
                    },
                ) if oa == ob && a == b && x.len() == y.len() => {
                    for (name, value) in x {
                        let mut found = None;
                        for (label, child) in y {
                            if let Some(w) = work.as_deref_mut() {
                                let cost = 1u64
                                    .checked_add(name.len() as u64)
                                    .and_then(|n| n.checked_add(label.len() as u64))
                                    .ok_or(super::RuntimeSetError::Capacity)?;
                                w.charge(cost)?;
                            }
                            if name == label {
                                found = Some(*child);
                                break;
                            }
                        }
                        let Some(other) = found else { return Ok(false) };
                        pending.push((*value, other));
                    }
                }
                (
                    Node::Enum {
                        id: a,
                        variant: x,
                        payload: p,
                        ..
                    },
                    Node::Enum {
                        id: b,
                        variant: y,
                        payload: q,
                        ..
                    },
                ) if a == b && x == y => match (p, q) {
                    (Some(a), Some(b)) => pending.push((*a, *b)),
                    (None, None) => {}
                    _ => return Ok(false),
                },
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}
fn length(n: usize) -> Result<(), RuntimeValueError> {
    if u64::try_from(n).map_err(|_| RuntimeValueError::SizeOverflow)?
        > crate::compiler::stdlib::MAX_ARRAY_GROWTH_LENGTH
    {
        Err(RuntimeValueError::SizeOverflow)
    } else {
        Ok(())
    }
}
fn separators(n: usize) -> Result<u64, RuntimeValueError> {
    u64::try_from(n.saturating_sub(1))
        .map_err(|_| RuntimeValueError::SizeOverflow)?
        .checked_mul(2)
        .ok_or(RuntimeValueError::SizeOverflow)
}

#[cfg(test)]
mod array_adapter_tests {
    use super::*;
    #[test]
    fn result_admission_is_atomic_for_unicode_and_does_not_copy_unselected_values() {
        let mut unlimited = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
        let a = RuntimeValueV2::array(
            vec![
                RuntimeValueV2::string("λ", &mut unlimited).unwrap(),
                RuntimeValueV2::string("discard", &mut unlimited).unwrap(),
            ],
            &mut unlimited,
        )
        .unwrap();
        let limits = MaterializationLimits {
            max_value_nodes: Some(2),
            max_text_bytes: Some(1),
        };
        let mut fuel = RuntimeValueFuel::new(limits);
        assert_eq!(
            a.slice_snapshot(0, 1, &mut fuel).unwrap_err(),
            RuntimeValueError::TextLimit
        );
        assert_eq!(fuel.remaining(), limits);
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits {
            max_value_nodes: Some(2),
            max_text_bytes: Some(2),
        });
        let result = a.slice_snapshot(0, 1, &mut fuel).unwrap();
        assert_eq!(result.render(&mut unlimited, None).unwrap(), "[λ]");
        assert_eq!(fuel.remaining().max_text_bytes, Some(0));
        assert_eq!(fuel.remaining().max_value_nodes, Some(0));
        let limits = MaterializationLimits {
            max_value_nodes: Some(3),
            max_text_bytes: Some(100),
        };
        let mut fuel = RuntimeValueFuel::new(limits);
        assert_eq!(
            a.growth_snapshot(&a, true, &mut fuel).unwrap_err(),
            RuntimeValueError::NodeLimit
        );
        assert_eq!(fuel.remaining(), limits);
    }
    #[test]
    fn deep_array_adapter_copies_and_error_cleanup_are_iterative() {
        let mut legacy = Value::String("λ".into());
        for _ in 0..16_000 {
            legacy = Value::Array(vec![legacy]);
        }
        let mut unlimited = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
        let a = RuntimeValueV2::from_legacy(&legacy, &mut unlimited).unwrap();
        legacy.drop_iterative();
        let b = a.slice_snapshot(0, 1, &mut unlimited).unwrap();
        assert!(a.structurally_equal(&b));
        let c = a.growth_snapshot(&b, true, &mut unlimited).unwrap();
        assert_eq!(c.array_len(), Some(2));
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits {
            max_value_nodes: Some(16_000),
            max_text_bytes: Some(2),
        });
        assert_eq!(
            a.slice_snapshot(0, 1, &mut fuel).unwrap_err(),
            RuntimeValueError::NodeLimit
        );
        drop((a, b, c));
    }
}

#[cfg(test)]
mod string_adapter_tests {
    use super::*;

    #[test]
    fn standard_records_admit_complete_labels_and_payloads_atomically() {
        let labels = RuntimeValueV2::standard_labels(
            crate::compiler::stdlib::SPLIT_ONCE_TYPE,
            &["found", "before", "after"],
        )
        .unwrap();
        let limits = MaterializationLimits {
            max_value_nodes: Some(4),
            max_text_bytes: Some(labels + 5),
        };
        let mut fuel = RuntimeValueFuel::new(limits);
        assert_eq!(
            RuntimeValueV2::split_once("λ::😀", "::", &mut fuel).unwrap_err(),
            RuntimeValueError::TextLimit
        );
        assert_eq!(fuel.remaining(), limits);
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits {
            max_value_nodes: Some(4),
            max_text_bytes: Some(labels + 6),
        });
        let result = RuntimeValueV2::split_once("λ::😀", "::", &mut fuel).unwrap();
        assert_eq!(
            result.logical_footprint().unwrap().owned_text_bytes,
            labels + 6
        );
        assert_eq!(fuel.remaining().max_value_nodes, Some(0));
        assert_eq!(fuel.remaining().max_text_bytes, Some(0));
        let limits = MaterializationLimits {
            max_value_nodes: Some(2),
            max_text_bytes: Some(100),
        };
        let mut fuel = RuntimeValueFuel::new(limits);
        assert_eq!(
            RuntimeValueV2::parse_int("7", &mut fuel).unwrap_err(),
            RuntimeValueError::NodeLimit
        );
        assert_eq!(fuel.remaining(), limits);
    }

    #[test]
    fn terminal_escape_measures_complete_output_before_admission() {
        let limits = MaterializationLimits {
            max_value_nodes: Some(1),
            max_text_bytes: Some(9),
        };
        let mut fuel = RuntimeValueFuel::new(limits);
        assert_eq!(
            RuntimeValueV2::escape_terminal("λ\n", &mut fuel).unwrap_err(),
            RuntimeValueError::TextLimit
        );
        assert_eq!(fuel.remaining(), limits);
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits {
            max_value_nodes: Some(1),
            max_text_bytes: Some(10),
        });
        assert_eq!(
            RuntimeValueV2::escape_terminal("λ\n", &mut fuel)
                .unwrap()
                .as_str(),
            Some("λ\\u{000A}")
        );
        assert_eq!(fuel.remaining().max_value_nodes, Some(0));
        assert_eq!(fuel.remaining().max_text_bytes, Some(0));
    }
}
