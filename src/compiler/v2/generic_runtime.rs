//! Iterative execution of sealed generic IR with flat owned values and shared fuel.
use super::*;
mod arrays;
mod codecs;
mod collection_maps;
mod collection_values;
mod json;
pub use collection_maps::{
    GenericCollectionMap, GenericCollectionMapEntry, GenericCollectionMapError,
    GenericCollectionMapRemoval,
};
mod collections;
pub use collections::{
    GenericCollectionError, GenericCollectionSelector, GenericCollectionSnapshot,
    GenericCollectionType,
};
mod display;
use crate::compiler::{
    diagnostics::Span,
    execution::{ExecutionControl, ExecutionLimits, MaterializationLimits, RuntimePolicy},
};
/// Explicit generic VM policy; no existing command/runtime defaults change.
#[derive(Debug, Clone)]
pub struct GenericVmPolicy {
    /// Existing shared instruction/output and logical materialization allowances.
    pub runtime: RuntimePolicy,
    /// Existing cooperative cancellation/elapsed control.
    pub control: ExecutionControl,
    /// Active frames including the root, in 1..=256.
    pub max_call_depth: usize,
    /// Total active local slots, admitted before frame allocation.
    pub max_active_slots: usize,
    /// Total active operand values across all frames.
    pub max_active_operands: usize,
    /// Shared instruction/value traversal and byte processing work.
    pub max_work: u64,
    /// Explicit data-profile capacity, in 0..=16 MiB; can lower the default ceiling.
    pub max_byte_buffer_bytes: usize,
    /// Explicit bounded JSON staging/parser/serializer policy.
    pub json: JsonLimits,
}
impl GenericVmPolicy {
    /// Finite, host-free library profile; not a CLI policy or process sandbox.
    pub const DEFAULT: Self = Self {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(1_000_000),
                max_output_bytes: Some(0),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(1_000_000),
                max_text_bytes: Some(16 * 1024 * 1024),
            },
        },
        control: ExecutionControl::UNLIMITED,
        max_call_depth: 256,
        max_active_slots: 65_536,
        max_active_operands: 65_536,
        max_work: 16_000_000,
        max_byte_buffer_bytes: super::RuntimeBytesLimits::DEFAULT.max_bytes,
        json: JsonLimits::DEFAULT,
    };
}
/// Execution failure; no partial success value or backend fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericVmError {
    /// Concrete function instance at refusal.
    pub function: usize,
    /// Instruction address at refusal.
    pub instruction: usize,
    /// Original function-source operation range.
    pub span: Span,
    /// Specific runtime refusal.
    pub kind: GenericVmErrorKind,
}
/// Runtime refusal categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericVmErrorKind {
    /// The ordinary VM entry point does not authorize propagation.
    PropagationUnavailable,
    /// Instruction/cancellation/elapsed refusal from existing execution policy.
    Execution(String),
    /// Existing logical materialization refusal.
    Materialization(RuntimeValueError),
    /// Explicit active frame/slot/operand/work allowance.
    Limit(&'static str),
    /// The initial host-free entry API requires no value arguments.
    RootArguments,
    /// Owned host child cleanup failed; the original host reason is retained.
    HostCleanup(String),
    /// Checked arithmetic failure.
    Arithmetic(&'static str),
    /// Negative or unavailable array index.
    Bounds,
    /// Sealed instruction/value agreement is unavailable.
    Invalid(&'static str),
}
impl std::fmt::Display for GenericVmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic execution refused: {self:?}")
    }
}
impl std::error::Error for GenericVmError {}
#[derive(Debug, Clone)]
enum Kind {
    Map(Vec<usize>),
    Set(Vec<usize>),
    // Internal entry layout is its enclosing map context, never a language value type.
    Entry { key: usize, value: usize },
    Unit,
    Int(i64),
    Byte(u8),
    // One sealed Byte child per byte keeps physical and logical subtree counts equal.
    Bytes(Vec<usize>),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<usize>),
    Record(Vec<usize>),
    Enum { tag: usize, payload: Option<usize> },
}
impl Kind {
    fn indices(&mut self, mut map: impl FnMut(usize) -> usize) {
        match self {
            Self::Array(v) | Self::Bytes(v) | Self::Record(v) | Self::Map(v) | Self::Set(v) => {
                for index in v {
                    *index = map(*index);
                }
            }
            Self::Entry { key, value } => {
                *key = map(*key);
                *value = map(*value);
            }
            Self::Enum {
                payload: Some(index),
                ..
            } => *index = map(*index),
            _ => {}
        }
    }
}
#[derive(Debug, Clone)]
struct Node {
    layout: usize,
    kind: Kind,
    count: usize,
    text: u64,
}
// Every value is a postorder tree. Each subtree occupies a contiguous interval;
// record slots may point into those intervals in declaration order.
#[derive(Debug)]
struct Value {
    nodes: Vec<Node>,
}
impl Value {
    fn root(&self) -> &Node {
        self.nodes.last().expect("owned value root")
    }
}
/// Allocation-free borrowed view into an owned flat execution result.
#[derive(Debug, Clone, Copy)]
pub struct GenericValueRef<'a> {
    nodes: &'a [Node],
    root: usize,
}
impl<'a> GenericValueRef<'a> {
    /// Layout index in the enclosing execution's checked layout table.
    pub fn layout(&self) -> usize {
        self.nodes[self.root].layout
    }
    /// Inspect an Int.
    pub fn as_int(&self) -> Option<i64> {
        if let Kind::Int(v) = self.nodes[self.root].kind {
            Some(v)
        } else {
            None
        }
    }
    /// Inspect a checked Byte without implicit Int conversion.
    pub fn as_byte(&self) -> Option<u8> {
        if let Kind::Byte(v) = self.nodes[self.root].kind {
            Some(v)
        } else {
            None
        }
    }
    /// Inspect a Float, preserving existing runtime IEEE arithmetic behavior.
    pub fn as_float(&self) -> Option<f64> {
        if let Kind::Float(v) = self.nodes[self.root].kind {
            Some(v)
        } else {
            None
        }
    }
    /// Inspect a Bool.
    pub fn as_bool(&self) -> Option<bool> {
        if let Kind::Bool(v) = self.nodes[self.root].kind {
            Some(v)
        } else {
            None
        }
    }
    /// Inspect UTF-8 text without copying.
    pub fn as_str(&self) -> Option<&'a str> {
        if let Kind::String(v) = &self.nodes[self.root].kind {
            Some(v)
        } else {
            None
        }
    }
    /// Whether the value is Unit.
    pub fn is_unit(&self) -> bool {
        matches!(self.nodes[self.root].kind, Kind::Unit)
    }
    /// Array length, when applicable.
    pub fn array_len(&self) -> Option<usize> {
        if let Kind::Array(v) = &self.nodes[self.root].kind {
            Some(v.len())
        } else {
            None
        }
    }
    /// Borrow one array element.
    pub fn array_item(&self, index: usize) -> Option<Self> {
        let Kind::Array(v) = &self.nodes[self.root].kind else {
            return None;
        };
        Some(Self {
            nodes: self.nodes,
            root: *v.get(index)?,
        })
    }
    /// Inspect the length of an immutable byte buffer.
    pub fn bytes_len(&self) -> Option<usize> {
        if let Kind::Bytes(v) = &self.nodes[self.root].kind {
            Some(v.len())
        } else {
            None
        }
    }
    /// Borrow a Byte in a buffer without allocating a snapshot.
    pub fn byte_item(&self, index: usize) -> Option<Self> {
        let Kind::Bytes(v) = &self.nodes[self.root].kind else {
            return None;
        };
        Some(Self {
            nodes: self.nodes,
            root: *v.get(index)?,
        })
    }
    /// Borrow one declaration-order record field.
    pub fn field(&self, index: usize) -> Option<Self> {
        let Kind::Record(v) = &self.nodes[self.root].kind else {
            return None;
        };
        Some(Self {
            nodes: self.nodes,
            root: *v.get(index)?,
        })
    }
    /// Declaration-order enum tag.
    pub fn enum_tag(&self) -> Option<usize> {
        if let Kind::Enum { tag, .. } = self.nodes[self.root].kind {
            Some(tag)
        } else {
            None
        }
    }
    /// Borrow the selected payload; nullary variants return None and Unit remains Some.
    pub fn payload(&self) -> Option<Self> {
        let Kind::Enum { payload, .. } = &self.nodes[self.root].kind else {
            return None;
        };
        Some(Self {
            nodes: self.nodes,
            root: (*payload)?,
        })
    }
    /// Borrowed map length without materializing a language Int.
    pub fn map_len(&self) -> Option<usize> {
        if let Kind::Map(v) = &self.nodes[self.root].kind {
            Some(v.len())
        } else {
            None
        }
    }
    /// Borrowed set length without materializing a language Int.
    pub fn set_len(&self) -> Option<usize> {
        if let Kind::Set(v) = &self.nodes[self.root].kind {
            Some(v.len())
        } else {
            None
        }
    }
    /// Borrow an insertion-ordered key/value pair without exposing its internal entry wrapper.
    pub fn map_entry(&self, index: usize) -> Option<(Self, Self)> {
        let Kind::Map(entries) = &self.nodes[self.root].kind else {
            return None;
        };
        let (k, v) = collection_values::entry(self.nodes, *entries.get(index)?)?;
        Some((
            Self {
                nodes: self.nodes,
                root: k,
            },
            Self {
                nodes: self.nodes,
                root: v,
            },
        ))
    }
    /// Borrow one insertion-ordered set member.
    pub fn set_item(&self, index: usize) -> Option<Self> {
        let Kind::Set(keys) = &self.nodes[self.root].kind else {
            return None;
        };
        Some(Self {
            nodes: self.nodes,
            root: *keys.get(index)?,
        })
    }
    /// Cached logical subtree nodes and UTF-8 String/nominal-label bytes.
    pub fn footprint(&self) -> (usize, u64) {
        let n = &self.nodes[self.root];
        (n.count, n.text)
    }
}
/// Successful host-free execution. Owned result destruction is flat, including deep compounds.
#[derive(Debug)]
pub struct GenericVmOutcome {
    ir: CheckedGenericIr,
    value: Value,
    remaining: RuntimePolicy,
    work: u64,
    instructions: u64,
    peak_frames: usize,
    peak_slots: usize,
    peak_operands: usize,
}
impl GenericVmOutcome {
    /// Inspect the returned value without allocating or creating an unmetered clone.
    pub fn value(&self) -> GenericValueRef<'_> {
        GenericValueRef {
            nodes: &self.value.nodes,
            root: self.value.nodes.len() - 1,
        }
    }
    /// Retained source/type/layout/instruction proof.
    pub fn ir(&self) -> &CheckedGenericIr {
        &self.ir
    }
    /// Shared materialization/instruction allowances remaining after execution.
    pub fn remaining(&self) -> RuntimePolicy {
        self.remaining
    }
    /// Logical VM work consumed, separate from dispatched instruction count.
    pub fn work(&self) -> u64 {
        self.work
    }
    /// Dispatched instructions across all calls.
    pub fn instructions(&self) -> u64 {
        self.instructions
    }
    /// Maximum simultaneous invocation frames.
    pub fn peak_frames(&self) -> usize {
        self.peak_frames
    }
    /// Maximum simultaneously active local slots.
    pub fn peak_slots(&self) -> usize {
        self.peak_slots
    }
    /// Maximum simultaneously active operand values across frames.
    pub fn peak_operands(&self) -> usize {
        self.peak_operands
    }
}
struct Budget<'a> {
    host: &'a HostV3,
    json: JsonLimits,
    max_byte_buffer_bytes: usize,
    fuel: RuntimeValueFuel,
    work: u64,
    function: usize,
    pc: usize,
    span: Span,
}
impl Budget<'_> {
    fn error(&self, kind: GenericVmErrorKind) -> GenericVmError {
        GenericVmError {
            function: self.function,
            instruction: self.pc,
            span: self.span,
            kind,
        }
    }
    fn invalid(&self, reason: &'static str) -> GenericVmError {
        self.error(GenericVmErrorKind::Invalid(reason))
    }
    fn limit(&self, name: &'static str) -> GenericVmError {
        self.error(GenericVmErrorKind::Limit(name))
    }
    fn work(&mut self, n: u64) -> Result<(), GenericVmError> {
        self.fuel
            .checkpoint()
            .map_err(|e| self.error(GenericVmErrorKind::Execution(e)))?;
        self.work = self
            .work
            .checked_sub(n)
            .ok_or_else(|| self.limit("generic runtime work"))?;
        Ok(())
    }
    fn admit(&mut self, n: usize, text: u64) -> Result<(), GenericVmError> {
        self.fuel
            .admit(n as u64, text)
            .map_err(|e| self.error(GenericVmErrorKind::Materialization(e)))
    }
    fn scalar(&mut self, layout: usize, kind: Kind, text: u64) -> Result<Value, GenericVmError> {
        self.work(
            1u64.checked_add(text)
                .ok_or_else(|| self.limit("value size"))?,
        )?;
        self.admit(1, text)?;
        Ok(Value {
            nodes: vec![Node {
                layout,
                kind,
                count: 1,
                text,
            }],
        })
    }
    fn string(&mut self, layout: usize, text: &str) -> Result<Value, GenericVmError> {
        self.work(
            (text.len() as u64)
                .checked_add(1)
                .ok_or_else(|| self.limit("string work"))?,
        )?;
        self.admit(1, text.len() as u64)?;
        let value = Value {
            nodes: vec![Node {
                layout,
                kind: Kind::String(text.to_owned()),
                count: 1,
                text: text.len() as u64,
            }],
        };
        self.work(0)?;
        Ok(value)
    }
    fn snapshot(&mut self, value: &Value, root: usize) -> Result<Value, GenericVmError> {
        let node = value
            .nodes
            .get(root)
            .ok_or_else(|| self.invalid("snapshot root"))?;
        self.work(
            (node.count as u64)
                .checked_mul(2)
                .and_then(|n| n.checked_add(node.text))
                .ok_or_else(|| self.limit("snapshot size"))?,
        )?;
        self.admit(node.count, node.text)?;
        let start = (root + 1)
            .checked_sub(node.count)
            .ok_or_else(|| self.invalid("snapshot interval"))?;
        let mut nodes = Vec::with_capacity(node.count);
        for node in &value.nodes[start..=root] {
            self.fuel
                .checkpoint()
                .map_err(|e| self.error(GenericVmErrorKind::Execution(e)))?;
            let mut node = node.clone();
            node.kind.indices(|index| index - start);
            nodes.push(node);
        }
        Ok(Value { nodes })
    }
    fn wrapper(
        &mut self,
        layout: usize,
        values: Vec<Value>,
        text: u64,
        kind: impl FnOnce(Vec<usize>) -> Kind,
    ) -> Result<Value, GenericVmError> {
        let count = values.iter().try_fold(1usize, |n, v| {
            n.checked_add(v.nodes.len())
                .ok_or_else(|| self.limit("value size"))
        })?;
        let bytes = values.iter().try_fold(text, |n, v| {
            n.checked_add(v.root().text)
                .ok_or_else(|| self.limit("value text size"))
        })?;
        self.work(
            (count as u64)
                .checked_mul(2)
                .ok_or_else(|| self.limit("value size"))?,
        )?;
        self.admit(1, text)?;
        let mut nodes = Vec::with_capacity(count);
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            let offset = nodes.len();
            for mut node in value.nodes {
                self.fuel
                    .checkpoint()
                    .map_err(|e| self.error(GenericVmErrorKind::Execution(e)))?;
                node.kind.indices(|index| index + offset);
                nodes.push(node);
            }
            roots.push(nodes.len() - 1);
        }
        nodes.push(Node {
            layout,
            kind: kind(roots),
            count,
            text: bytes,
        });
        Ok(Value { nodes })
    }
    fn replace(
        &mut self,
        array: &mut Value,
        index: usize,
        value: Value,
    ) -> Result<(), GenericVmError> {
        let Kind::Array(children) = &array.root().kind else {
            return Err(self.invalid("indexed local is not an array"));
        };
        let removed = *children
            .get(index)
            .ok_or_else(|| self.error(GenericVmErrorKind::Bounds))?;
        let old = &array.nodes[removed];
        let start = removed + 1 - old.count;
        let removed_count = old.count;
        let removed_text = old.text;
        let new_count = array
            .nodes
            .len()
            .checked_sub(removed_count)
            .and_then(|n| n.checked_add(value.nodes.len()))
            .ok_or_else(|| self.limit("array replacement size"))?;
        let new_text = array
            .root()
            .text
            .checked_sub(removed_text)
            .and_then(|n| n.checked_add(value.root().text))
            .ok_or_else(|| self.limit("array replacement text"))?;
        self.work(
            (array.nodes.len() as u64)
                .checked_add(value.nodes.len() as u64)
                .and_then(|n| n.checked_mul(2))
                .ok_or_else(|| self.limit("array replacement work"))?,
        )?;
        // All fallible policy/bounds/size checks precede taking the old local.
        let mut old = std::mem::take(&mut array.nodes);
        let mut root = old.pop().expect("array root");
        let mut nodes = Vec::with_capacity(new_count);
        let rebase = |id| if id > removed { id - removed_count } else { id };
        for (at, mut node) in old.into_iter().enumerate() {
            if (start..=removed).contains(&at) {
                continue;
            }
            node.kind.indices(rebase);
            nodes.push(node);
        }
        let offset = nodes.len();
        for mut node in value.nodes {
            node.kind.indices(|id| id + offset);
            nodes.push(node);
        }
        root.kind.indices(rebase);
        let Kind::Array(children) = &mut root.kind else {
            unreachable!()
        };
        children[index] = nodes.len() - 1;
        root.count = new_count;
        root.text = new_text;
        nodes.push(root);
        array.nodes = nodes;
        Ok(())
    }
    fn equal(&mut self, left: &Value, right: &Value) -> Result<bool, GenericVmError> {
        let work = (left.root().count as u64)
            .checked_add(right.root().count as u64)
            .and_then(|n| n.checked_add(left.root().text))
            .and_then(|n| n.checked_add(right.root().text))
            .ok_or_else(|| self.limit("equality work"))?;
        self.work(work)?;
        let mut pending = vec![(left.nodes.len() - 1, right.nodes.len() - 1)];
        while let Some((a, b)) = pending.pop() {
            self.fuel
                .checkpoint()
                .map_err(|e| self.error(GenericVmErrorKind::Execution(e)))?;
            let a = &left.nodes[a];
            let b = &right.nodes[b];
            if a.layout != b.layout {
                return Ok(false);
            }
            match (&a.kind, &b.kind) {
                (Kind::Unit, Kind::Unit) => {}
                (Kind::Map(x), Kind::Map(y)) if x.len() == y.len() => {
                    for entry in x {
                        let (k, v) = collection_values::entry(&left.nodes, *entry)
                            .ok_or_else(|| self.invalid("map equality entry"))?;
                        let mut found = None;
                        for other in y {
                            let (ok, ov) = collection_values::entry(&right.nodes, *other)
                                .ok_or_else(|| self.invalid("map equality entry"))?;
                            let cost =
                                collection_values::key_cost(&left.nodes[k], &right.nodes[ok])
                                    .ok_or_else(|| self.invalid("map equality key"))?;
                            self.work(cost)?;
                            if collection_values::key_equal(&left.nodes[k], &right.nodes[ok]) {
                                found = Some(ov);
                                break;
                            }
                        }
                        let Some(other) = found else { return Ok(false) };
                        pending.push((v, other));
                    }
                }
                (Kind::Set(x), Kind::Set(y)) if x.len() == y.len() => {
                    for key in x {
                        let mut found = false;
                        for other in y {
                            let cost = collection_values::key_cost(
                                &left.nodes[*key],
                                &right.nodes[*other],
                            )
                            .ok_or_else(|| self.invalid("set equality key"))?;
                            self.work(cost)?;
                            if collection_values::key_equal(&left.nodes[*key], &right.nodes[*other])
                            {
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
                    pending.extend(a.iter().zip(b).map(|(a, b)| (*a, *b)))
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
struct Frame {
    function: usize,
    pc: usize,
    locals: Vec<Option<Value>>,
    stack: Vec<Value>,
}
fn labels(
    ir: &CheckedGenericIr,
    layout: usize,
    tag: Option<usize>,
    b: &mut Budget,
) -> Result<u64, GenericVmError> {
    let definitions = ir
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let shape = ir
        .plans()
        .layouts()
        .layout(layout)
        .ok_or_else(|| b.invalid("value layout"))?
        .kind();
    let text = match shape {
        GenericRuntimeLayoutKind::Record { declaration, .. } => {
            let Some(CheckedGenericDefinition::Record(record)) =
                definitions.definition(*declaration)
            else {
                return Err(b.invalid("record labels"));
            };
            let mut bytes = record.syntax().qualified_name().len() as u64;
            for field in record.syntax().fields() {
                b.work(1)?;
                bytes = bytes
                    .checked_add(field.name.len() as u64)
                    .ok_or_else(|| b.limit("label bytes"))?;
            }
            bytes
        }
        GenericRuntimeLayoutKind::Enum { declaration, .. } => {
            let Some(CheckedGenericDefinition::Enum(definition)) =
                definitions.definition(*declaration)
            else {
                return Err(b.invalid("enum labels"));
            };
            let variant = definition
                .syntax()
                .variants()
                .get(tag.ok_or_else(|| b.invalid("missing tag"))?)
                .ok_or_else(|| b.invalid("enum tag"))?;
            (definition.syntax().qualified_name().len() as u64)
                .checked_add(variant.name.len() as u64)
                .ok_or_else(|| b.limit("label bytes"))?
        }
        _ => 0,
    };
    Ok(text)
}
fn numeric(
    op: GenericNumericOperator,
    left: &Node,
    right: &Node,
    b: &Budget,
) -> Result<Kind, GenericVmError> {
    use GenericNumericOperator::*;
    let arithmetic = |reason| b.error(GenericVmErrorKind::Arithmetic(reason));
    match (&left.kind, &right.kind) {
        (Kind::Int(a), Kind::Int(c)) => Ok(match op {
            Less => Kind::Bool(a < c),
            LessEqual => Kind::Bool(a <= c),
            Greater => Kind::Bool(a > c),
            GreaterEqual => Kind::Bool(a >= c),
            _ => {
                if op == Divide && *c == 0 {
                    return Err(arithmetic("division by zero"));
                }
                Kind::Int(
                    match op {
                        Add => a.checked_add(*c),
                        Subtract => a.checked_sub(*c),
                        Multiply => a.checked_mul(*c),
                        Divide => a.checked_div(*c),
                        _ => unreachable!(),
                    }
                    .ok_or_else(|| arithmetic("integer overflow"))?,
                )
            }
        }),
        (Kind::Float(a), Kind::Float(c)) => Ok(match op {
            Less => Kind::Bool(a < c),
            LessEqual => Kind::Bool(a <= c),
            Greater => Kind::Bool(a > c),
            GreaterEqual => Kind::Bool(a >= c),
            Add => Kind::Float(a + c),
            Subtract => Kind::Float(a - c),
            Multiply => Kind::Float(a * c),
            Divide => {
                if *c == 0.0 {
                    return Err(arithmetic("division by zero"));
                }
                Kind::Float(a / c)
            }
        }),
        _ => Err(b.invalid("numeric values")),
    }
}
/// Execute the sealed root instance with no host services or external arguments.
/// All calls share instruction/materialization/work/control policy. Frames and
/// compound values are iterative/flat; no instruction or error path falls back.
pub fn execute_generic_ir(
    ir: &CheckedGenericIr,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, GenericVmError> {
    execute_mode(ir, policy, false, false, false, None)
}
/// Explicit host-free execution of checked propagation IR with shared snapshot fuel.
/// Ordinary IR execution and source/CLI/native profiles retain their existing gates.
pub fn execute_generic_propagation_ir(
    ir: &CheckedGenericPropagationIr,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, GenericVmError> {
    execute_mode(ir.checked_ir(), policy, true, false, false, None)
}
/// Execute only the explicit collection IR profile under shared VM controls.
pub fn execute_generic_collection_ir(
    ir: &CheckedGenericPropagationIr,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, GenericVmError> {
    execute_mode(ir.checked_ir(), policy, true, true, false, None)
}
/// Execute only the explicit data IR profile under existing VM limits/control.
pub fn execute_generic_data_ir(
    ir: &CheckedGenericPropagationIr,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, GenericVmError> {
    execute_mode(ir.checked_ir(), policy, true, true, true, None)
}
/// Execute data IR with explicit trusted host grants; no authority is retained in the IR.
pub fn execute_generic_data_ir_with_host(
    ir: &CheckedGenericPropagationIr,
    policy: GenericVmPolicy,
    host: &HostV3,
) -> Result<GenericVmOutcome, GenericVmError> {
    execute_mode(ir.checked_ir(), policy, true, true, true, Some(host))
}
fn execute_mode(
    ir: &CheckedGenericIr,
    policy: GenericVmPolicy,
    propagation: bool,
    collections: bool,
    data: bool,
    host: Option<&HostV3>,
) -> Result<GenericVmOutcome, GenericVmError> {
    let denied_host = HostV3::default();
    let host = host.unwrap_or(&denied_host);
    let first = ir.functions()[0].instructions()[0].span;
    let mut b = Budget {
        host,
        json: policy.json,
        max_byte_buffer_bytes: policy.max_byte_buffer_bytes,
        fuel: RuntimeValueFuel::for_runtime(policy.runtime, policy.control),
        work: policy.max_work,
        function: 0,
        pc: 0,
        span: first,
    };
    if data && policy.max_byte_buffer_bytes > super::RuntimeBytesLimits::DEFAULT.max_bytes {
        return Err(b.invalid("byte buffer capacity policy"));
    }
    if data && policy.json.validate().is_err() {
        return Err(b.invalid("JSON policy"));
    }
    if ir.has_data_operations() && !data {
        return Err(b.invalid("data execution requires explicit admission"));
    }
    if ir.has_collection_operations() && !collections {
        return Err(b.invalid("collection execution requires explicit admission"));
    }
    if !propagation
        && ir
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph()
            .bodies()
            .propagation_usage()
            .is_some_and(|usage| usage.sites > 0)
    {
        return Err(b.error(GenericVmErrorKind::PropagationUnavailable));
    }
    b.work(0)?;
    if !ir.functions()[0].parameter_slots().is_empty() {
        return Err(b.error(GenericVmErrorKind::RootArguments));
    }
    if policy.max_call_depth == 0 || policy.max_call_depth > 256 {
        return Err(b.limit("call depth profile"));
    }
    let mut frames = Vec::new();
    let mut slots = 0usize;
    let mut operands = 0usize;
    let mut dispatched = 0u64;
    let mut peak = 0;
    let mut peak_slots = 0;
    let mut peak_operands = 0;
    let enter = |target: usize,
                 args: Vec<Value>,
                 frames: &mut Vec<Frame>,
                 slots: &mut usize,
                 b: &mut Budget|
     -> Result<(), GenericVmError> {
        let function = ir
            .functions()
            .get(target)
            .ok_or_else(|| b.invalid("callee"))?;
        if frames.len() >= policy.max_call_depth {
            return Err(b.limit("call depth"));
        }
        let total = slots
            .checked_add(function.slot_layouts().len())
            .ok_or_else(|| b.limit("active local slots"))?;
        if total > policy.max_active_slots {
            return Err(b.limit("active local slots"));
        }
        if args.len() != function.parameter_slots().len() {
            return Err(b.invalid("callee arity"));
        }
        b.work(function.slot_layouts().len() as u64)?;
        let mut locals: Vec<_> = (0..function.slot_layouts().len()).map(|_| None).collect();
        for (slot, value) in function.parameter_slots().iter().zip(args) {
            if value.root().layout != function.slot_layouts()[*slot] {
                return Err(b.invalid("argument value layout"));
            }
            locals[*slot] = Some(value);
        }
        frames.push(Frame {
            function: target,
            pc: 0,
            locals,
            stack: Vec::new(),
        });
        *slots = total;
        Ok(())
    };
    enter(0, Vec::new(), &mut frames, &mut slots, &mut b)?;
    loop {
        peak = peak.max(frames.len());
        peak_slots = peak_slots.max(slots);
        peak_operands = peak_operands.max(operands);
        let frame = frames.last_mut().expect("active invocation");
        b.function = frame.function;
        b.pc = frame.pc;
        let instruction = ir.functions()[frame.function]
            .instructions()
            .get(frame.pc)
            .ok_or_else(|| b.invalid("instruction address"))?;
        b.span = instruction.span;
        b.work(1)?;
        b.fuel
            .instruction()
            .map_err(|e| b.error(GenericVmErrorKind::Execution(e)))?;
        dispatched = dispatched
            .checked_add(1)
            .ok_or_else(|| b.limit("instruction count"))?;
        frame.pc += 1;
        let pop = |frame: &mut Frame,
                   operands: &mut usize,
                   b: &Budget|
         -> Result<Value, GenericVmError> {
            let value = frame
                .stack
                .pop()
                .ok_or_else(|| b.invalid("operand stack"))?;
            *operands -= 1;
            Ok(value)
        };
        let mut result = None;
        let mut returning = None;
        match &instruction.operation {
            GenericIrOperation::Builtin {
                builtin,
                layout,
                right_layout,
            } => {
                let mut args = Vec::with_capacity(builtin.arity());
                for _ in 0..builtin.arity() {
                    args.push(pop(frame, &mut operands, &b)?);
                }
                args.reverse();
                if matches!(
                    builtin,
                    GenericBuiltin::JsonParse
                        | GenericBuiltin::JsonStringify
                        | GenericBuiltin::HostCall
                ) {
                    if !data || !ir.has_data_operations() {
                        return Err(b.invalid("JSON data profile"));
                    }
                    result = Some(json::execute(*builtin, *layout, &args[0], ir, &mut b)?);
                } else if matches!(
                    builtin,
                    GenericBuiltin::BytesFromArray
                        | GenericBuiltin::BytesFromText
                        | GenericBuiltin::BytesLen
                        | GenericBuiltin::BytesGet
                        | GenericBuiltin::BytesConcat
                        | GenericBuiltin::BytesSlice
                        | GenericBuiltin::BytesToArray
                        | GenericBuiltin::BytesToHex
                        | GenericBuiltin::BytesFromHex
                        | GenericBuiltin::BytesToBase64
                        | GenericBuiltin::BytesFromBase64
                        | GenericBuiltin::BytesToUtf8
                ) {
                    if !data || !ir.has_data_operations() {
                        return Err(b.invalid("data operation profile"));
                    }
                    result = Some(bytes_operation(*builtin, *layout, &args, ir, &mut b)?);
                } else if matches!(
                    builtin,
                    GenericBuiltin::ByteFromInt | GenericBuiltin::ByteToInt
                ) {
                    if !data || !ir.has_data_operations() {
                        return Err(b.invalid("data operation profile"));
                    }
                    result = Some(byte_conversion(*builtin, *layout, &args[0], ir, &mut b)?);
                } else if let Some(kind) = builtin.collection_kind() {
                    if !collections || !ir.has_collection_operations() {
                        return Err(b.invalid("collection operation profile"));
                    }
                    result = Some(if builtin.collection_new() {
                        let valid = matches!(
                            (kind, ir.plans().layouts().layout(*layout).map(|l| l.kind())),
                            (
                                GenericCollectionLayoutKind::Map,
                                Some(GenericRuntimeLayoutKind::Map { .. })
                            ) | (
                                GenericCollectionLayoutKind::Set,
                                Some(GenericRuntimeLayoutKind::Set { .. })
                            )
                        );
                        if !valid {
                            return Err(b.invalid("collection_new result layout"));
                        }
                        b.scalar(
                            *layout,
                            if kind == GenericCollectionLayoutKind::Map {
                                Kind::Map(Vec::new())
                            } else {
                                Kind::Set(Vec::new())
                            },
                            0,
                        )?
                    } else if *builtin == GenericBuiltin::MapEntries {
                        collection_values::entries(ir, *layout, &args[0], &mut b)?
                    } else if builtin.collection_iteration() {
                        collection_values::iterate(ir, *builtin, *layout, &args[0], &mut b)?
                    } else if builtin.collection_remove() {
                        collection_values::remove(ir, *builtin, *layout, &args, &mut b)?
                    } else if builtin.collection_get() {
                        collection_values::get(ir, *builtin, *layout, &args, &mut b)?
                    } else if builtin.collection_contains() {
                        collection_values::contains(ir, *builtin, *layout, &args, &mut b)?
                    } else if builtin.collection_insert() {
                        collection_values::insert(ir, *builtin, *layout, &args, &mut b)?
                    } else {
                        let length = match (kind, &args[0].root().kind) {
                            (GenericCollectionLayoutKind::Map, Kind::Map(v))
                            | (GenericCollectionLayoutKind::Set, Kind::Set(v)) => v.len(),
                            _ => return Err(b.invalid("collection_len operand")),
                        };
                        b.scalar(
                            *layout,
                            Kind::Int(
                                i64::try_from(length).map_err(|_| b.limit("collection length"))?,
                            ),
                            0,
                        )?
                    });
                } else if builtin.array_operation() {
                    result = Some(arrays::execute(
                        ir,
                        *builtin,
                        *layout,
                        *right_layout,
                        &mut args,
                        &mut b,
                    )?);
                } else if *builtin == GenericBuiltin::ToString {
                    result = Some(display::format(ir, &args[0], *layout, &mut b)?);
                } else if *builtin == GenericBuiltin::ArrayLen {
                    let Kind::Array(items) = &args[0].root().kind else {
                        return Err(b.invalid("array_len argument"));
                    };
                    let length = i64::try_from(items.len()).map_err(|_| b.limit("array length"))?;
                    result = Some(b.scalar(*layout, Kind::Int(length), 0)?);
                } else {
                    let Kind::String(text) = &args[0].root().kind else {
                        return Err(b.invalid("builtin String argument"));
                    };
                    if builtin.result_primitive().is_some() {
                        result = Some(match builtin {
                            GenericBuiltin::StringLen => b.scalar(
                                *layout,
                                Kind::Int(
                                    i64::try_from(text.len())
                                        .map_err(|_| b.limit("String byte length"))?,
                                ),
                                0,
                            )?,
                            GenericBuiltin::LinesUnique => {
                                b.work(
                                    (text.len() as u64)
                                        .checked_add(1)
                                        .ok_or_else(|| b.limit("line scan work"))?,
                                )?;
                                let mut seen = std::collections::BTreeSet::new();
                                let mut unique = true;
                                for line in text.split_terminator('\n') {
                                    let work = (seen.len() as u64)
                                        .checked_add(1)
                                        .and_then(|n| {
                                            n.checked_mul((line.len() as u64).checked_add(1)?)
                                        })
                                        .and_then(|n| n.checked_mul(2))
                                        .ok_or_else(|| b.limit("line comparison work"))?;
                                    b.work(work)?;
                                    if seen.contains(line) {
                                        unique = false;
                                        break;
                                    }
                                    b.admit(1, 0)?;
                                    seen.insert(line);
                                    b.work(0)?;
                                }
                                b.scalar(*layout, Kind::Bool(unique), 0)?
                            }
                            GenericBuiltin::EscapeTerminal => {
                                b.work(
                                    (text.len() as u64)
                                        .checked_add(1)
                                        .ok_or_else(|| b.limit("terminal escape work"))?,
                                )?;
                                let bytes = text
                                    .chars()
                                    .try_fold(0u64, |n, c| {
                                        n.checked_add(
                                            if c as u32 <= 0x1f
                                                || (0x7f..=0x9f).contains(&(c as u32))
                                            {
                                                8
                                            } else {
                                                c.len_utf8() as u64
                                            },
                                        )
                                    })
                                    .ok_or_else(|| b.limit("terminal escape size"))?;
                                b.work(bytes)?;
                                b.admit(1, bytes)?;
                                let escaped = crate::compiler::stdlib::escape_terminal(text);
                                b.work(0)?;
                                Value {
                                    nodes: vec![Node {
                                        layout: *layout,
                                        kind: Kind::String(escaped),
                                        count: 1,
                                        text: bytes,
                                    }],
                                }
                            }
                            _ => return Err(b.invalid("primitive String builtin")),
                        });
                    } else {
                        let GenericRuntimeLayoutKind::Record { fields, .. } = ir
                            .plans()
                            .layouts()
                            .layout(*layout)
                            .ok_or_else(|| b.invalid("builtin layout"))?
                            .kind()
                        else {
                            return Err(b.invalid("builtin result record"));
                        };
                        let values = match builtin {
                            GenericBuiltin::ParseInt => {
                                if fields.len() != 2 {
                                    return Err(b.invalid("parse result fields"));
                                }
                                b.work(text.len() as u64)?;
                                let parsed = crate::compiler::stdlib::parse_int(text);
                                b.work(0)?;
                                vec![
                                    b.scalar(fields[0], Kind::Bool(parsed.is_some()), 0)?,
                                    b.scalar(fields[1], Kind::Int(parsed.unwrap_or_default()), 0)?,
                                ]
                            }
                            GenericBuiltin::SplitOnce => {
                                if fields.len() != 3 {
                                    return Err(b.invalid("split result fields"));
                                }
                                let Kind::String(delimiter) = &args[1].root().kind else {
                                    return Err(b.invalid("split delimiter"));
                                };
                                let scan = (text.len() as u64)
                                    .checked_add(1)
                                    .and_then(|n| {
                                        n.checked_mul((delimiter.len() as u64).checked_add(1)?)
                                    })
                                    .ok_or_else(|| b.limit("split scan work"))?;
                                b.work(scan)?;
                                let (found, before, after) =
                                    match text.split_once(delimiter.as_str()) {
                                        Some((left, right)) => (true, left, right),
                                        None => (false, text.as_str(), ""),
                                    };
                                b.work(0)?;
                                vec![
                                    b.scalar(fields[0], Kind::Bool(found), 0)?,
                                    b.string(fields[1], before)?,
                                    b.string(fields[2], after)?,
                                ]
                            }
                            _ => return Err(b.invalid("record builtin kind")),
                        };
                        let text = labels(ir, *layout, None, &mut b)?;
                        result = Some(b.wrapper(*layout, values, text, Kind::Record)?);
                    }
                }
            }
            GenericIrOperation::Constant { value, layout } => {
                let (kind, text) = match value {
                    GenericLiteralValue::Int(v) => (Kind::Int(*v), 0),
                    GenericLiteralValue::FloatBits(v) => (Kind::Float(f64::from_bits(*v)), 0),
                    GenericLiteralValue::Bool(v) => (Kind::Bool(*v), 0),
                    GenericLiteralValue::String(v) => {
                        b.work(v.len() as u64)?;
                        b.admit(1, v.len() as u64)?;
                        result = Some(Value {
                            nodes: vec![Node {
                                layout: *layout,
                                kind: Kind::String(v.to_string()),
                                count: 1,
                                text: v.len() as u64,
                            }],
                        });
                        (Kind::Unit, 0)
                    }
                };
                if result.is_none() {
                    result = Some(b.scalar(*layout, kind, text)?);
                }
            }
            GenericIrOperation::Unit { layout } => {
                result = Some(b.scalar(*layout, Kind::Unit, 0)?)
            }
            GenericIrOperation::Load { slot } => {
                let value = frame
                    .locals
                    .get(*slot)
                    .and_then(|v| v.as_ref())
                    .ok_or_else(|| b.invalid("uninitialized local"))?;
                result = Some(b.snapshot(value, value.nodes.len() - 1)?);
            }
            GenericIrOperation::Store { slot, .. } => {
                let value = pop(frame, &mut operands, &b)?;
                if value.root().layout != ir.functions()[frame.function].slot_layouts()[*slot] {
                    return Err(b.invalid("stored value layout"));
                }
                frame.locals[*slot] = Some(value);
            }
            GenericIrOperation::IndexStore { slot } => {
                let value = pop(frame, &mut operands, &b)?;
                let index = pop(frame, &mut operands, &b)?;
                let Kind::Int(index) = index.root().kind else {
                    return Err(b.invalid("index value"));
                };
                let index =
                    usize::try_from(index).map_err(|_| b.error(GenericVmErrorKind::Bounds))?;
                let array = frame
                    .locals
                    .get_mut(*slot)
                    .and_then(Option::as_mut)
                    .ok_or_else(|| b.invalid("indexed local"))?;
                let GenericRuntimeLayoutKind::Array { element } = ir
                    .plans()
                    .layouts()
                    .layout(array.root().layout)
                    .ok_or_else(|| b.invalid("array layout"))?
                    .kind()
                else {
                    return Err(b.invalid("array layout"));
                };
                if value.root().layout != *element {
                    return Err(b.invalid("replacement layout"));
                }
                b.replace(array, index, value)?;
            }
            GenericIrOperation::Drop => {
                pop(frame, &mut operands, &b)?;
            }
            GenericIrOperation::IntToFloat { layout } => {
                let mut value = pop(frame, &mut operands, &b)?;
                let root = value.nodes.last_mut().expect("scalar value");
                let Kind::Int(n) = root.kind else {
                    return Err(b.invalid("numeric widening"));
                };
                root.kind = Kind::Float(n as f64);
                root.layout = *layout;
                result = Some(value);
            }
            GenericIrOperation::Binary { operator, layout } => {
                let right = pop(frame, &mut operands, &b)?;
                let left = pop(frame, &mut operands, &b)?;
                let kind = match operator {
                    GenericIrBinary::Numeric(op) => numeric(*op, left.root(), right.root(), &b)?,
                    GenericIrBinary::Equality { negated } => {
                        Kind::Bool(b.equal(&left, &right)? != *negated)
                    }
                    GenericIrBinary::String(op) => {
                        let (Kind::String(a), Kind::String(c)) =
                            (&left.root().kind, &right.root().kind)
                        else {
                            return Err(b.invalid("String operands"));
                        };
                        let len = a
                            .len()
                            .checked_add(c.len())
                            .ok_or_else(|| b.limit("String size"))?;
                        b.work(len as u64)?;
                        if *op == GenericStringOperator::Concatenate {
                            b.admit(1, len as u64)?;
                            let mut text = String::with_capacity(len);
                            text.push_str(a);
                            text.push_str(c);
                            result = Some(Value {
                                nodes: vec![Node {
                                    layout: *layout,
                                    kind: Kind::String(text),
                                    count: 1,
                                    text: len as u64,
                                }],
                            });
                            Kind::Unit
                        } else {
                            Kind::Bool(match op {
                                GenericStringOperator::Less => a < c,
                                GenericStringOperator::LessEqual => a <= c,
                                GenericStringOperator::Greater => a > c,
                                GenericStringOperator::GreaterEqual => a >= c,
                                _ => unreachable!(),
                            })
                        }
                    }
                };
                if result.is_none() {
                    result = Some(b.scalar(*layout, kind, 0)?);
                }
            }
            GenericIrOperation::Array { layout, .. }
            | GenericIrOperation::Record { layout, .. } => {
                let (count, order) = match &instruction.operation {
                    GenericIrOperation::Array { count, .. } => (*count, None),
                    GenericIrOperation::Record { slots, .. } => {
                        (slots.len(), Some(slots.as_slice()))
                    }
                    _ => unreachable!(),
                };
                if matches!(instruction.operation, GenericIrOperation::Array { .. })
                    && count as u64 > crate::compiler::stdlib::MAX_ARRAY_GROWTH_LENGTH
                {
                    return Err(b.limit("array length"));
                }
                b.work(count as u64)?;
                let start = frame
                    .stack
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| b.invalid("constructor operands"))?;
                let values = frame.stack.split_off(start);
                operands -= count;
                let text = labels(ir, *layout, None, &mut b)?;
                result = Some(b.wrapper(*layout, values, text, |roots| {
                    if let Some(order) = order {
                        let mut fields = vec![0; roots.len()];
                        for (slot, root) in order.iter().zip(roots) {
                            fields[*slot] = root;
                        }
                        Kind::Record(fields)
                    } else {
                        Kind::Array(roots)
                    }
                })?);
            }
            GenericIrOperation::Enum {
                layout,
                tag,
                payload,
            } => {
                let values = if *payload {
                    vec![pop(frame, &mut operands, &b)?]
                } else {
                    Vec::new()
                };
                let text = labels(ir, *layout, Some(*tag), &mut b)?;
                result = Some(b.wrapper(*layout, values, text, |roots| Kind::Enum {
                    tag: *tag,
                    payload: roots.first().copied(),
                })?);
            }
            GenericIrOperation::Field { .. } | GenericIrOperation::Index { .. } => {
                let index = if matches!(instruction.operation, GenericIrOperation::Index { .. }) {
                    let index = pop(frame, &mut operands, &b)?;
                    let Kind::Int(n) = index.root().kind else {
                        return Err(b.invalid("array index"));
                    };
                    usize::try_from(n).map_err(|_| b.error(GenericVmErrorKind::Bounds))?
                } else {
                    match instruction.operation {
                        GenericIrOperation::Field { slot, .. } => slot,
                        _ => unreachable!(),
                    }
                };
                let value = pop(frame, &mut operands, &b)?;
                let roots = match &value.root().kind {
                    Kind::Array(v) | Kind::Record(v) => v,
                    _ => return Err(b.invalid("member receiver")),
                };
                let root = *roots
                    .get(index)
                    .ok_or_else(|| b.error(GenericVmErrorKind::Bounds))?;
                result = Some(b.snapshot(&value, root)?);
            }
            GenericIrOperation::Call { target, arguments } => {
                b.work(*arguments as u64)?;
                let start = frame
                    .stack
                    .len()
                    .checked_sub(*arguments)
                    .ok_or_else(|| b.invalid("call operands"))?;
                let args = frame.stack.split_off(start);
                operands -= arguments;
                enter(*target, args, &mut frames, &mut slots, &mut b)?;
                continue;
            }
            GenericIrOperation::Jump { target } => frame.pc = *target,
            GenericIrOperation::Branch { yes, no } => {
                let value = pop(frame, &mut operands, &b)?;
                let Kind::Bool(condition) = value.root().kind else {
                    return Err(b.invalid("branch value"));
                };
                frame.pc = if condition { *yes } else { *no };
            }
            GenericIrOperation::Switch { layout, targets } => {
                let value = frame
                    .stack
                    .last()
                    .ok_or_else(|| b.invalid("switch value"))?;
                let Kind::Enum { tag, .. } = value.root().kind else {
                    return Err(b.invalid("switch tag"));
                };
                if value.root().layout != *layout {
                    return Err(b.invalid("switch nominal layout"));
                }
                frame.pc = *targets.get(tag).ok_or_else(|| b.invalid("switch tag"))?;
            }
            GenericIrOperation::MatchBind {
                layout,
                tag,
                binding,
            } => {
                let value = pop(frame, &mut operands, &b)?;
                let Kind::Enum {
                    tag: actual,
                    payload,
                } = value.root().kind
                else {
                    return Err(b.invalid("match binding value"));
                };
                if value.root().layout != *layout || actual != *tag {
                    return Err(b.invalid("match binding tag"));
                }
                match (binding, payload) {
                    (Some(slot), Some(root)) => {
                        let payload = b.snapshot(&value, root)?;
                        if payload.root().layout
                            != ir.functions()[frame.function].slot_layouts()[*slot]
                        {
                            return Err(b.invalid("payload layout"));
                        }
                        frame.locals[*slot] = Some(payload);
                    }
                    (None, None) => {}
                    _ => return Err(b.invalid("match payload arity")),
                }
            }
            GenericIrOperation::Propagate {
                input,
                success,
                result: target,
                success_tag,
                failure_tag,
                failure,
            } => {
                if !propagation {
                    return Err(b.error(GenericVmErrorKind::PropagationUnavailable));
                }
                let value = pop(frame, &mut operands, &b)?;
                if value.root().layout != *input {
                    return Err(b.invalid("propagation input layout"));
                }
                let Kind::Enum { tag, payload } = value.root().kind else {
                    return Err(b.invalid("propagation wrapper"));
                };
                if tag == *success_tag {
                    let root = payload.ok_or_else(|| b.invalid("propagation success payload"))?;
                    let extracted = b.snapshot(&value, root)?;
                    if extracted.root().layout != *success {
                        return Err(b.invalid("propagation success layout"));
                    }
                    result = Some(extracted);
                } else if tag == *failure_tag {
                    let values = match (failure, payload) {
                        (None, None) => Vec::new(),
                        (Some(expected), Some(root)) => {
                            let error = b.snapshot(&value, root)?;
                            if error.root().layout != *expected {
                                return Err(b.invalid("propagation failure layout"));
                            }
                            vec![error]
                        }
                        _ => return Err(b.invalid("propagation failure payload")),
                    };
                    let text = labels(ir, *target, Some(*failure_tag), &mut b)?;
                    let returned = b.wrapper(*target, values, text, |roots| Kind::Enum {
                        tag: *failure_tag,
                        payload: roots.first().copied(),
                    })?;
                    if returned.root().layout != ir.functions()[frame.function].return_layout() {
                        return Err(b.invalid("propagation caller result"));
                    }
                    // Admit the flat destruction work before abandoning this frame.
                    // Materialization remains cumulative; cleanup never refunds fuel.
                    b.work(
                        (frame.locals.len() as u64)
                            .checked_add(frame.stack.len() as u64)
                            .ok_or_else(|| b.limit("frame cleanup size"))?,
                    )?;
                    for owned in frame
                        .locals
                        .iter()
                        .filter_map(Option::as_ref)
                        .chain(frame.stack.iter())
                        .chain(std::iter::once(&value))
                    {
                        b.work(owned.nodes.len() as u64)?;
                    }
                    returning = Some(returned);
                } else {
                    return Err(b.invalid("propagation tag"));
                }
            }
            GenericIrOperation::Return => {
                let value = pop(frame, &mut operands, &b)?;
                if value.root().layout != ir.functions()[frame.function].return_layout()
                    || !frame.stack.is_empty()
                {
                    return Err(b.invalid("return value"));
                }
                returning = Some(value);
            }
            GenericIrOperation::Unreachable => return Err(b.invalid("terminal sentinel reached")),
        }
        if let Some(value) = returning {
            let removed = frames.pop().expect("return frame");
            slots -= removed.locals.len();
            operands = operands
                .checked_sub(removed.stack.len())
                .ok_or_else(|| b.invalid("frame operand accounting"))?;
            drop(removed);
            if let Some(caller) = frames.last_mut() {
                if operands >= policy.max_active_operands
                    || caller.stack.len() >= ir.functions()[caller.function].max_stack()
                {
                    return Err(b.limit("active operand slots"));
                }
                caller.stack.push(value);
                operands += 1;
                continue;
            }
            return Ok(GenericVmOutcome {
                ir: ir.clone(),
                value,
                remaining: b.fuel.policy_remaining(),
                work: policy.max_work - b.work,
                instructions: dispatched,
                peak_frames: peak,
                peak_slots,
                peak_operands,
            });
        }
        if let Some(value) = result {
            if operands >= policy.max_active_operands
                || frame.stack.len() >= ir.functions()[frame.function].max_stack()
            {
                return Err(b.limit("active operand slots"));
            }
            frame.stack.push(value);
            operands += 1;
        }
    }
}

fn byte_conversion(
    builtin: GenericBuiltin,
    layout: usize,
    input: &Value,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    if builtin == GenericBuiltin::ByteToInt {
        let Kind::Byte(value) = input.root().kind else {
            return Err(b.invalid("Byte operand"));
        };
        return b.scalar(layout, Kind::Int(i64::from(value)), 0);
    }
    let Kind::Int(value) = input.root().kind else {
        return Err(b.invalid("Int operand"));
    };
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("Byte result"));
    };
    if arguments.len() != 2 {
        return Err(b.invalid("Byte result arguments"));
    }
    if let Ok(value) = u8::try_from(value) {
        let text = labels(ir, layout, Some(0), b)?;
        b.fuel
            .preflight(2, text)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        let byte = b.scalar(arguments[0], Kind::Byte(value), 0)?;
        return b.wrapper(layout, vec![byte], text, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    data_error_result(
        layout,
        "byte_range",
        "byte value must be in 0..255",
        None,
        ir,
        b,
    )
}

fn data_error_result(
    layout: usize,
    code: &str,
    message: &str,
    offset: Option<i64>,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("data error Result"));
    };
    let error_layout = *arguments
        .get(1)
        .ok_or_else(|| b.invalid("data error Result arguments"))?;
    let Some(GenericRuntimeLayoutKind::Record { fields, .. }) =
        ir.plans().layouts().layout(error_layout).map(|l| l.kind())
    else {
        return Err(b.invalid("data Error record"));
    };
    if fields.len() != 3 {
        return Err(b.invalid("data Error fields"));
    }
    let result_text = labels(ir, layout, Some(1), b)?;
    let error_text = labels(ir, error_layout, None, b)?;
    let offset_tag = usize::from(offset.is_none());
    let offset_text = labels(ir, fields[2], Some(offset_tag), b)?;
    let text = result_text + error_text + offset_text + code.len() as u64 + message.len() as u64;
    b.fuel
        .preflight(if offset.is_some() { 6 } else { 5 }, text)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    let code = b.string(fields[0], code)?;
    let message = b.string(fields[1], message)?;
    let offset_values = if let Some(offset) = offset {
        let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
            ir.plans().layouts().layout(fields[2]).map(|l| l.kind())
        else {
            return Err(b.invalid("data Error offset"));
        };
        vec![b.scalar(arguments[0], Kind::Int(offset), 0)?]
    } else {
        vec![]
    };
    let offset = b.wrapper(fields[2], offset_values, offset_text, |v| Kind::Enum {
        tag: offset_tag,
        payload: v.first().copied(),
    })?;
    let error = b.wrapper(
        error_layout,
        vec![code, message, offset],
        error_text,
        Kind::Record,
    )?;
    b.wrapper(layout, vec![error], result_text, |v| Kind::Enum {
        tag: 1,
        payload: Some(v[0]),
    })
}

fn bytes_operation(
    builtin: GenericBuiltin,
    layout: usize,
    args: &[Value],
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let input = &args[0];
    if builtin == GenericBuiltin::BytesToUtf8 {
        return codecs::utf8(layout, input, ir, b);
    }
    if matches!(
        builtin,
        GenericBuiltin::BytesToBase64 | GenericBuiltin::BytesFromBase64
    ) {
        return codecs::base64(builtin, layout, input, ir, b);
    }
    if matches!(
        builtin,
        GenericBuiltin::BytesToHex | GenericBuiltin::BytesFromHex
    ) {
        return codecs::hex(builtin, layout, input, ir, b);
    }
    if builtin == GenericBuiltin::BytesGet {
        let Kind::Bytes(items) = &input.root().kind else {
            return Err(b.invalid("bytes_get receiver"));
        };
        let Kind::Int(index) = args[1].root().kind else {
            return Err(b.invalid("bytes_get index"));
        };
        let Some(child) = usize::try_from(index).ok().and_then(|i| items.get(i)) else {
            return data_error_result(
                layout,
                "bounds",
                "byte index out of bounds",
                Some(index),
                ir,
                b,
            );
        };
        let text = labels(ir, layout, Some(0), b)?;
        b.fuel
            .preflight(2, text)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        let node = &input.nodes[*child];
        let Kind::Byte(value) = node.kind else {
            return Err(b.invalid("bytes_get Byte"));
        };
        let byte = b.scalar(node.layout, Kind::Byte(value), 0)?;
        return b.wrapper(layout, vec![byte], text, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    if builtin == GenericBuiltin::BytesToArray {
        let Kind::Bytes(items) = &input.root().kind else {
            return Err(b.invalid("bytes_to_array receiver"));
        };
        if items.len() > b.max_byte_buffer_bytes {
            return data_error_result(
                layout,
                "capacity",
                "byte buffer capacity exceeded",
                None,
                ir,
                b,
            );
        }
        let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
            ir.plans().layouts().layout(layout).map(|l| l.kind())
        else {
            return Err(b.invalid("bytes_to_array Result"));
        };
        let array_layout = arguments[0];
        let Some(GenericRuntimeLayoutKind::Array { element }) =
            ir.plans().layouts().layout(array_layout).map(|l| l.kind())
        else {
            return Err(b.invalid("bytes_to_array result array"));
        };
        b.work(items.len() as u64)?;
        for child in items {
            b.work(0)?;
            let node = &input.nodes[*child];
            if node.layout != *element || !matches!(node.kind, Kind::Byte(_)) {
                return Err(b.invalid("bytes_to_array Byte identity"));
            }
        }
        let text = labels(ir, layout, Some(0), b)?;
        b.fuel
            .preflight((input.root().count as u64) + 1, text)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        let mut out = b.snapshot(input, input.nodes.len() - 1)?;
        let root = out.nodes.last_mut().expect("buffer snapshot root");
        let Kind::Bytes(items) = std::mem::replace(&mut root.kind, Kind::Unit) else {
            return Err(b.invalid("bytes_to_array snapshot"));
        };
        root.kind = Kind::Array(items);
        root.layout = array_layout;
        return b.wrapper(layout, vec![out], text, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    if builtin == GenericBuiltin::BytesSlice {
        let Kind::Bytes(children) = &input.root().kind else {
            return Err(b.invalid("bytes_slice receiver"));
        };
        let Kind::Int(start) = args[1].root().kind else {
            return Err(b.invalid("bytes_slice start"));
        };
        let Kind::Int(end) = args[2].root().kind else {
            return Err(b.invalid("bytes_slice end"));
        };
        let bad = if start < 0 {
            Some(start)
        } else if end < 0 {
            Some(end)
        } else if start > end {
            Some(start)
        } else if end as u64 > children.len() as u64 {
            Some(end)
        } else {
            None
        };
        if let Some(index) = bad {
            return data_error_result(
                layout,
                "bounds",
                "byte slice out of bounds",
                Some(index),
                ir,
                b,
            );
        }
        let start = usize::try_from(start).map_err(|_| b.invalid("slice start representation"))?;
        let end = usize::try_from(end).map_err(|_| b.invalid("slice end representation"))?;
        let len = end - start;
        if len > b.max_byte_buffer_bytes {
            return data_error_result(
                layout,
                "capacity",
                "byte buffer capacity exceeded",
                None,
                ir,
                b,
            );
        }
        let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
            ir.plans().layouts().layout(layout).map(|l| l.kind())
        else {
            return Err(b.invalid("bytes_slice Result"));
        };
        let buffer_layout = arguments[0];
        let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) =
            ir.plans().layouts().layout(buffer_layout).map(|l| l.kind())
        else {
            return Err(b.invalid("bytes_slice buffer layout"));
        };
        let count = len
            .checked_add(1)
            .ok_or_else(|| b.limit("byte slice size"))?;
        let text = labels(ir, layout, Some(0), b)?;
        b.work(
            (count as u64)
                .checked_mul(2)
                .ok_or_else(|| b.limit("byte slice work"))?,
        )?;
        b.fuel
            .preflight((count as u64) + 1, text)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        b.admit(count, 0)?;
        let mut nodes = Vec::with_capacity(count);
        let mut items = Vec::with_capacity(len);
        for child in &children[start..end] {
            b.work(0)?;
            let node = &input.nodes[*child];
            let Kind::Byte(value) = node.kind else {
                return Err(b.invalid("bytes_slice Byte"));
            };
            if node.layout != *byte {
                return Err(b.invalid("bytes_slice Byte identity"));
            }
            items.push(nodes.len());
            nodes.push(Node {
                layout: *byte,
                kind: Kind::Byte(value),
                count: 1,
                text: 0,
            });
        }
        nodes.push(Node {
            layout: buffer_layout,
            kind: Kind::Bytes(items),
            count,
            text: 0,
        });
        return b.wrapper(layout, vec![Value { nodes }], text, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    if builtin == GenericBuiltin::BytesConcat {
        let Kind::Bytes(left) = &input.root().kind else {
            return Err(b.invalid("bytes_concat left"));
        };
        let Kind::Bytes(right) = &args[1].root().kind else {
            return Err(b.invalid("bytes_concat right"));
        };
        let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
            ir.plans().layouts().layout(layout).map(|l| l.kind())
        else {
            return Err(b.invalid("bytes_concat Result"));
        };
        let buffer_layout = arguments[0];
        let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) =
            ir.plans().layouts().layout(buffer_layout).map(|l| l.kind())
        else {
            return Err(b.invalid("bytes_concat result"));
        };
        let len = left
            .len()
            .checked_add(right.len())
            .ok_or_else(|| b.limit("byte buffer size"))?;
        if len > b.max_byte_buffer_bytes {
            return data_error_result(
                layout,
                "capacity",
                "byte buffer capacity exceeded",
                None,
                ir,
                b,
            );
        }
        let count = len
            .checked_add(1)
            .ok_or_else(|| b.limit("byte buffer size"))?;
        b.work(
            (count as u64)
                .checked_mul(2)
                .ok_or_else(|| b.limit("byte buffer work"))?,
        )?;
        let result_text = labels(ir, layout, Some(0), b)?;
        b.fuel
            .preflight((count as u64) + 1, result_text)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        b.admit(count, 0)?;
        let mut nodes = Vec::with_capacity(count);
        let mut items = Vec::with_capacity(len);
        for (source, children) in [(input, left), (&args[1], right)] {
            for child in children {
                b.work(0)?;
                let node = &source.nodes[*child];
                let Kind::Byte(value) = node.kind else {
                    return Err(b.invalid("bytes_concat Byte"));
                };
                if node.layout != *byte {
                    return Err(b.invalid("bytes_concat Byte identity"));
                }
                items.push(nodes.len());
                nodes.push(Node {
                    layout: *byte,
                    kind: Kind::Byte(value),
                    count: 1,
                    text: 0,
                });
            }
        }
        nodes.push(Node {
            layout: buffer_layout,
            kind: Kind::Bytes(items),
            count,
            text: 0,
        });
        return b.wrapper(layout, vec![Value { nodes }], result_text, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    if builtin == GenericBuiltin::BytesLen {
        let Kind::Bytes(bytes) = &input.root().kind else {
            return Err(b.invalid("Bytes operand"));
        };
        let n = i64::try_from(bytes.len()).map_err(|_| b.limit("buffer length"))?;
        return b.scalar(layout, Kind::Int(n), 0);
    }
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("buffer constructor Result"));
    };
    let buffer_layout = arguments[0];
    let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) =
        ir.plans().layouts().layout(buffer_layout).map(|l| l.kind())
    else {
        return Err(b.invalid("Bytes result layout"));
    };
    if builtin == GenericBuiltin::BytesFromArray {
        let Kind::Array(items) = &input.root().kind else {
            return Err(b.invalid("Byte array operand"));
        };
        if items.len() > b.max_byte_buffer_bytes {
            return data_error_result(
                layout,
                "capacity",
                "byte buffer capacity exceeded",
                None,
                ir,
                b,
            );
        }
        b.work(items.len() as u64)?;
        for item in items {
            b.work(0)?;
            let node = &input.nodes[*item];
            if node.layout != *byte || !matches!(node.kind, Kind::Byte(_)) {
                return Err(b.invalid("Byte array element"));
            }
        }
        let result_text = labels(ir, layout, Some(0), b)?;
        b.fuel
            .preflight((input.root().count as u64) + 1, result_text)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        let mut out = b.snapshot(input, input.nodes.len() - 1)?;
        let root = out.nodes.last_mut().expect("array root");
        let Kind::Array(items) = std::mem::replace(&mut root.kind, Kind::Unit) else {
            return Err(b.invalid("Byte array snapshot"));
        };
        root.kind = Kind::Bytes(items);
        root.layout = buffer_layout;
        return b.wrapper(layout, vec![out], result_text, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    let Kind::String(text) = &input.root().kind else {
        return Err(b.invalid("String operand"));
    };
    if text.len() > b.max_byte_buffer_bytes {
        return data_error_result(
            layout,
            "capacity",
            "byte buffer capacity exceeded",
            None,
            ir,
            b,
        );
    }
    let count = text
        .len()
        .checked_add(1)
        .ok_or_else(|| b.limit("byte buffer size"))?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .ok_or_else(|| b.limit("byte buffer work"))?,
    )?;
    let result_text = labels(ir, layout, Some(0), b)?;
    b.fuel
        .preflight((count as u64) + 1, result_text)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.admit(count, 0)?;
    let mut nodes = Vec::with_capacity(count);
    let mut items = Vec::with_capacity(text.len());
    for value in text.bytes() {
        b.work(0)?;
        items.push(nodes.len());
        nodes.push(Node {
            layout: *byte,
            kind: Kind::Byte(value),
            count: 1,
            text: 0,
        });
    }
    nodes.push(Node {
        layout: buffer_layout,
        kind: Kind::Bytes(items),
        count,
        text: 0,
    });
    b.wrapper(layout, vec![Value { nodes }], result_text, |v| Kind::Enum {
        tag: 0,
        payload: Some(v[0]),
    })
}
