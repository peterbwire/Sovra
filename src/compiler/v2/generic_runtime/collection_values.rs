//! Flat nonempty collection snapshots and metered primitive-key matching.
use super::*;
pub(super) fn key_cost(a: &Node, b: &Node) -> Option<u64> {
    let text = match (&a.kind, &b.kind) {
        (Kind::Byte(_), Kind::Byte(_))
        | (Kind::Int(_), Kind::Int(_))
        | (Kind::Bool(_), Kind::Bool(_)) => 0,
        (Kind::String(a), Kind::String(b)) => (a.len() as u64).checked_add(b.len() as u64)?,
        _ => return None,
    };
    text.checked_add(1)
}
pub(super) fn key_equal(a: &Node, b: &Node) -> bool {
    match (&a.kind, &b.kind) {
        (Kind::Int(a), Kind::Int(b)) => a == b,
        (Kind::Byte(a), Kind::Byte(b)) => a == b,
        (Kind::Bool(a), Kind::Bool(b)) => a == b,
        (Kind::String(a), Kind::String(b)) => a == b,
        _ => false,
    }
}
pub(super) fn entry(nodes: &[Node], index: usize) -> Option<(usize, usize)> {
    match &nodes.get(index)?.kind {
        Kind::Entry { key, value } => Some((*key, *value)),
        _ => None,
    }
}
fn matches(left: &Node, right: &Node, b: &mut Budget) -> Result<bool, GenericVmError> {
    let cost = key_cost(left, right).ok_or_else(|| b.invalid("collection key kind"))?;
    b.work(cost)?;
    Ok(key_equal(left, right))
}
fn add(a: usize, c: usize, b: &Budget) -> Result<usize, GenericVmError> {
    a.checked_add(c)
        .ok_or_else(|| b.limit("collection snapshot nodes"))
}
fn text(a: u64, c: u64, b: &Budget) -> Result<u64, GenericVmError> {
    a.checked_add(c)
        .ok_or_else(|| b.limit("collection snapshot text"))
}
fn copy(
    source: &Value,
    root: usize,
    out: &mut Vec<Node>,
    b: &mut Budget,
) -> Result<usize, GenericVmError> {
    let node = source
        .nodes
        .get(root)
        .ok_or_else(|| b.invalid("collection child"))?;
    let start = (root + 1)
        .checked_sub(node.count)
        .ok_or_else(|| b.invalid("collection interval"))?;
    let dest = out.len();
    for node in &source.nodes[start..=root] {
        b.work(0)?;
        let mut node = node.clone();
        node.kind.indices(|index| index - start + dest);
        out.push(node);
    }
    out.len()
        .checked_sub(1)
        .ok_or_else(|| b.invalid("collection copy root"))
}
/// Plan every copied descendant before reserving output ownership or allocating result nodes.
pub(super) fn insert(
    ir: &CheckedGenericIr,
    builtin: GenericBuiltin,
    layout: usize,
    args: &[Value],
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    if args[0].root().layout != layout {
        return Err(b.invalid("collection insertion result type"));
    }
    let key = &args[1];
    let key_root = key.nodes.len() - 1;
    let (key_layout, value_layout) = match (
        builtin,
        ir.plans().layouts().layout(layout).map(|l| l.kind()),
    ) {
        (GenericBuiltin::MapInsert, Some(GenericRuntimeLayoutKind::Map { key, value, .. })) => {
            (*key, Some(*value))
        }
        (GenericBuiltin::SetInsert, Some(GenericRuntimeLayoutKind::Set { key, .. })) => {
            (*key, None)
        }
        _ => return Err(b.invalid("collection insertion shape")),
    };
    if key.root().layout != key_layout || value_layout.is_some_and(|id| args[2].root().layout != id)
    {
        return Err(b.invalid("collection insertion operand type"));
    }
    let roots = match &args[0].root().kind {
        Kind::Map(v) if value_layout.is_some() => v,
        Kind::Set(v) if value_layout.is_none() => v,
        _ => return Err(b.invalid("collection insertion receiver")),
    };
    let mut found = None;
    for (i, root) in roots.iter().enumerate() {
        let stored = if value_layout.is_some() {
            entry(&args[0].nodes, *root)
                .ok_or_else(|| b.invalid("map entry"))?
                .0
        } else {
            *root
        };
        if matches(&args[0].nodes[stored], key.root(), b)? {
            found = Some(i);
            break;
        }
    }
    let length =
        crate::compiler::stdlib::array_growth_len(roots.len() as u64, u64::from(found.is_none()))
            .map_err(|_| b.limit("collection representation length"))?;
    let (mut count, mut bytes) = (1usize, 0u64);
    for (i, root) in roots.iter().enumerate() {
        b.work(1)?;
        if value_layout.is_some() {
            let (k, v) = entry(&args[0].nodes, *root).ok_or_else(|| b.invalid("map entry"))?;
            let val = if found == Some(i) {
                args[2].root()
            } else {
                &args[0].nodes[v]
            };
            count = add(
                count,
                add(1, add(args[0].nodes[k].count, val.count, b)?, b)?,
                b,
            )?;
            bytes = text(bytes, text(args[0].nodes[k].text, val.text, b)?, b)?;
        } else {
            count = add(count, args[0].nodes[*root].count, b)?;
            bytes = text(bytes, args[0].nodes[*root].text, b)?;
        }
    }
    if found.is_none() {
        count = add(count, key.root().count, b)?;
        bytes = text(bytes, key.root().text, b)?;
        if value_layout.is_some() {
            count = add(count, add(1, args[2].root().count, b)?, b)?;
            bytes = text(bytes, args[2].root().text, b)?;
        }
    }
    b.fuel
        .preflight(count as u64, bytes)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .and_then(|n| n.checked_add(bytes))
            .ok_or_else(|| b.limit("collection copy work"))?,
    )?;
    b.admit(count, bytes)?;
    let mut nodes = Vec::with_capacity(count);
    let mut result = Vec::with_capacity(length);
    for (i, root) in roots.iter().enumerate() {
        b.work(0)?;
        if value_layout.is_some() {
            let (k, v) = entry(&args[0].nodes, *root).ok_or_else(|| b.invalid("map entry"))?;
            let key = copy(&args[0], k, &mut nodes, b)?;
            let value = if found == Some(i) {
                copy(&args[2], args[2].nodes.len() - 1, &mut nodes, b)?
            } else {
                copy(&args[0], v, &mut nodes, b)?
            };
            push_entry(&mut nodes, layout, key, value, b)?;
            result.push(nodes.len() - 1);
        } else {
            result.push(copy(&args[0], *root, &mut nodes, b)?);
        }
    }
    if found.is_none() {
        let key = copy(key, key_root, &mut nodes, b)?;
        if value_layout.is_some() {
            let value = copy(&args[2], args[2].nodes.len() - 1, &mut nodes, b)?;
            push_entry(&mut nodes, layout, key, value, b)?;
            result.push(nodes.len() - 1);
        } else {
            result.push(key)
        }
    }
    nodes.push(Node {
        layout,
        kind: if value_layout.is_some() {
            Kind::Map(result)
        } else {
            Kind::Set(result)
        },
        count,
        text: bytes,
    });
    Ok(Value { nodes })
}
fn push_entry(
    nodes: &mut Vec<Node>,
    layout: usize,
    key: usize,
    value: usize,
    b: &Budget,
) -> Result<(), GenericVmError> {
    let count = add(1, add(nodes[key].count, nodes[value].count, b)?, b)?;
    let bytes = text(nodes[key].text, nodes[value].text, b)?;
    nodes.push(Node {
        layout,
        kind: Kind::Entry { key, value },
        count,
        text: bytes,
    });
    Ok(())
}

pub(super) fn contains(
    ir: &CheckedGenericIr,
    builtin: GenericBuiltin,
    layout: usize,
    args: &[Value],
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let receiver = &args[0];
    let (roots, key_layout, map) = match (
        builtin,
        ir.plans()
            .layouts()
            .layout(receiver.root().layout)
            .map(|l| l.kind()),
        &receiver.root().kind,
    ) {
        (
            GenericBuiltin::MapContains,
            Some(GenericRuntimeLayoutKind::Map { key, .. }),
            Kind::Map(roots),
        ) => (roots, *key, true),
        (
            GenericBuiltin::SetContains,
            Some(GenericRuntimeLayoutKind::Set { key, .. }),
            Kind::Set(roots),
        ) => (roots, *key, false),
        _ => return Err(b.invalid("collection membership receiver")),
    };
    if args[1].root().layout != key_layout {
        return Err(b.invalid("collection membership key"));
    }
    let mut found = false;
    for root in roots {
        let key = if map {
            entry(&receiver.nodes, *root)
                .ok_or_else(|| b.invalid("map entry"))?
                .0
        } else {
            *root
        };
        if matches(&receiver.nodes[key], args[1].root(), b)? {
            found = true;
            break;
        }
    }
    b.scalar(layout, Kind::Bool(found), 0)
}

/// A complete Option snapshot is preflighted before allocating/copying payload nodes.
pub(super) fn get(
    ir: &CheckedGenericIr,
    builtin: GenericBuiltin,
    layout: usize,
    args: &[Value],
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let receiver = &args[0];
    let (roots, key_layout, payload_layout, map) = match (
        builtin,
        ir.plans()
            .layouts()
            .layout(receiver.root().layout)
            .map(|l| l.kind()),
        &receiver.root().kind,
    ) {
        (
            GenericBuiltin::MapGet,
            Some(GenericRuntimeLayoutKind::Map { key, value, .. }),
            Kind::Map(roots),
        ) => (roots, *key, *value, true),
        (
            GenericBuiltin::SetGet,
            Some(GenericRuntimeLayoutKind::Set { key, .. }),
            Kind::Set(roots),
        ) => (roots, *key, *key, false),
        _ => return Err(b.invalid("collection lookup receiver")),
    };
    if args[1].root().layout != key_layout {
        return Err(b.invalid("collection lookup key"));
    }
    let Some(GenericRuntimeLayoutKind::Enum {
        declaration,
        arguments,
        variants,
    }) = ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("lookup Option layout"));
    };
    let types = ir
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let Some(CheckedGenericDefinition::Enum(def)) = types.definition(*declaration) else {
        return Err(b.invalid("lookup Option definition"));
    };
    if def.syntax().standard_identity() != Some(GenericStandardEnum::Option)
        || arguments.as_slice() != [payload_layout]
        || variants.as_slice() != [Some(payload_layout), None]
    {
        return Err(b.invalid("lookup standard Option shape"));
    }
    let mut found = None;
    for root in roots {
        let (key, payload) = if map {
            entry(&receiver.nodes, *root).ok_or_else(|| b.invalid("map entry"))?
        } else {
            (*root, *root)
        };
        if matches(&receiver.nodes[key], args[1].root(), b)? {
            found = Some(payload);
            break;
        }
    }
    let tag = usize::from(found.is_none());
    let label_bytes = labels(ir, layout, Some(tag), b)?;
    let (count, bytes) = if let Some(root) = found {
        (
            add(1, receiver.nodes[root].count, b)?,
            text(label_bytes, receiver.nodes[root].text, b)?,
        )
    } else {
        (1, label_bytes)
    };
    b.fuel
        .preflight(count as u64, bytes)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .and_then(|n| n.checked_add(bytes))
            .ok_or_else(|| b.limit("lookup copy work"))?,
    )?;
    b.admit(count, bytes)?;
    let mut nodes = Vec::with_capacity(count);
    let payload = match found {
        Some(root) => Some(copy(receiver, root, &mut nodes, b)?),
        None => None,
    };
    nodes.push(Node {
        layout,
        kind: Kind::Enum { tag, payload },
        count,
        text: bytes,
    });
    Ok(Value { nodes })
}

/// Admit the complete Option/updated collection before copying surviving subtrees.
pub(super) fn remove(
    ir: &CheckedGenericIr,
    builtin: GenericBuiltin,
    layout: usize,
    args: &[Value],
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let receiver = &args[0];
    let receiver_layout = receiver.root().layout;
    let (roots, key_layout, map) = match (
        builtin,
        ir.plans()
            .layouts()
            .layout(receiver_layout)
            .map(|l| l.kind()),
        &receiver.root().kind,
    ) {
        (
            GenericBuiltin::MapRemove,
            Some(GenericRuntimeLayoutKind::Map { key, .. }),
            Kind::Map(roots),
        ) => (roots, *key, true),
        (
            GenericBuiltin::SetRemove,
            Some(GenericRuntimeLayoutKind::Set { key, .. }),
            Kind::Set(roots),
        ) => (roots, *key, false),
        _ => return Err(b.invalid("collection removal receiver")),
    };
    if args[1].root().layout != key_layout {
        return Err(b.invalid("collection removal key"));
    }
    let Some(GenericRuntimeLayoutKind::Enum {
        declaration,
        arguments,
        variants,
    }) = ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("removal Option layout"));
    };
    let types = ir
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let Some(CheckedGenericDefinition::Enum(def)) = types.definition(*declaration) else {
        return Err(b.invalid("removal Option definition"));
    };
    if def.syntax().standard_identity() != Some(GenericStandardEnum::Option)
        || arguments.as_slice() != [receiver_layout]
        || variants.as_slice() != [Some(receiver_layout), None]
    {
        return Err(b.invalid("removal standard Option shape"));
    }
    let mut found = None;
    for (index, root) in roots.iter().enumerate() {
        let key = if map {
            entry(&receiver.nodes, *root)
                .ok_or_else(|| b.invalid("map entry"))?
                .0
        } else {
            *root
        };
        if matches(&receiver.nodes[key], args[1].root(), b)? {
            found = Some(index);
            break;
        }
    }
    let tag = usize::from(found.is_none());
    let label_bytes = labels(ir, layout, Some(tag), b)?;
    let (collection_count, collection_text) = match found {
        Some(index) => {
            let removed = &receiver.nodes[roots[index]];
            (
                receiver
                    .root()
                    .count
                    .checked_sub(removed.count)
                    .filter(|n| *n >= 1)
                    .ok_or_else(|| b.invalid("removal subtree count"))?,
                receiver
                    .root()
                    .text
                    .checked_sub(removed.text)
                    .ok_or_else(|| b.invalid("removal subtree text"))?,
            )
        }
        None => (0, 0),
    };
    let count = add(1, collection_count, b)?;
    let bytes = text(label_bytes, collection_text, b)?;
    b.fuel
        .preflight(count as u64, bytes)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .and_then(|n| n.checked_add(bytes))
            .ok_or_else(|| b.limit("removal copy work"))?,
    )?;
    b.admit(count, bytes)?;
    let mut nodes = Vec::with_capacity(count);
    let payload = if let Some(removed) = found {
        let mut remaining = Vec::with_capacity(roots.len() - 1);
        for (index, root) in roots.iter().enumerate() {
            b.work(0)?;
            if index != removed {
                remaining.push(copy(receiver, *root, &mut nodes, b)?);
            }
        }
        nodes.push(Node {
            layout: receiver_layout,
            kind: if map {
                Kind::Map(remaining)
            } else {
                Kind::Set(remaining)
            },
            count: collection_count,
            text: collection_text,
        });
        Some(nodes.len() - 1)
    } else {
        None
    };
    nodes.push(Node {
        layout,
        kind: Kind::Enum { tag, payload },
        count,
        text: bytes,
    });
    Ok(Value { nodes })
}

/// Snapshot selected children into one typed array, admitting the entire result first.
pub(super) fn iterate(
    ir: &CheckedGenericIr,
    builtin: GenericBuiltin,
    layout: usize,
    receiver: &Value,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let (roots, element, map) = match (
        builtin,
        ir.plans()
            .layouts()
            .layout(receiver.root().layout)
            .map(|l| l.kind()),
        &receiver.root().kind,
    ) {
        (
            GenericBuiltin::MapKeys,
            Some(GenericRuntimeLayoutKind::Map { key, .. }),
            Kind::Map(roots),
        ) => (roots, *key, true),
        (
            GenericBuiltin::MapValues,
            Some(GenericRuntimeLayoutKind::Map { value, .. }),
            Kind::Map(roots),
        ) => (roots, *value, true),
        (
            GenericBuiltin::SetValues,
            Some(GenericRuntimeLayoutKind::Set { key, .. }),
            Kind::Set(roots),
        ) => (roots, *key, false),
        _ => return Err(b.invalid("collection iteration receiver")),
    };
    if !matches!(ir.plans().layouts().layout(layout).map(|l|l.kind()),Some(GenericRuntimeLayoutKind::Array{element:actual}) if *actual==element)
    {
        return Err(b.invalid("collection iteration element type"));
    }
    let length = crate::compiler::stdlib::array_growth_len(roots.len() as u64, 0)
        .map_err(|_| b.limit("collection iteration length"))?;
    let selected = |root, b: &Budget| -> Result<usize, GenericVmError> {
        if map {
            let (key, value) =
                entry(&receiver.nodes, root).ok_or_else(|| b.invalid("map entry"))?;
            Ok(if builtin == GenericBuiltin::MapKeys {
                key
            } else {
                value
            })
        } else {
            Ok(root)
        }
    };
    let (mut count, mut bytes) = (1usize, 0u64);
    for root in roots {
        b.work(1)?;
        let child = selected(*root, b)?;
        let node = &receiver.nodes[child];
        count = add(count, node.count, b)?;
        bytes = text(bytes, node.text, b)?;
    }
    b.fuel
        .preflight(count as u64, bytes)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .and_then(|n| n.checked_add(bytes))
            .ok_or_else(|| b.limit("collection iteration copy work"))?,
    )?;
    b.admit(count, bytes)?;
    let mut nodes = Vec::with_capacity(count);
    let mut items = Vec::with_capacity(length);
    for root in roots {
        let child = selected(*root, b)?;
        items.push(copy(receiver, child, &mut nodes, b)?);
    }
    nodes.push(Node {
        layout,
        kind: Kind::Array(items),
        count,
        text: bytes,
    });
    Ok(Value { nodes })
}

/// Public MapEntry records are new wrappers, never exposed private map entry nodes.
pub(super) fn entries(
    ir: &CheckedGenericIr,
    layout: usize,
    receiver: &Value,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let (roots, key_layout, value_layout) = match (
        ir.plans()
            .layouts()
            .layout(receiver.root().layout)
            .map(|l| l.kind()),
        &receiver.root().kind,
    ) {
        (Some(GenericRuntimeLayoutKind::Map { key, value, .. }), Kind::Map(roots)) => {
            (roots, *key, *value)
        }
        _ => return Err(b.invalid("map entries receiver")),
    };
    let Some(GenericRuntimeLayoutKind::Array { element }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("entries array layout"));
    };
    let entry_layout = *element;
    let Some(GenericRuntimeLayoutKind::Record {
        declaration,
        arguments,
        fields,
    }) = ir.plans().layouts().layout(entry_layout).map(|l| l.kind())
    else {
        return Err(b.invalid("entries record layout"));
    };
    let types = ir
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types();
    let Some(CheckedGenericDefinition::Record(def)) = types.definition(*declaration) else {
        return Err(b.invalid("entries record definition"));
    };
    if !def.syntax().is_standard_map_entry()
        || arguments.as_slice() != [key_layout, value_layout]
        || fields.as_slice() != arguments.as_slice()
    {
        return Err(b.invalid("entries standard MapEntry shape"));
    }
    let length = crate::compiler::stdlib::array_growth_len(roots.len() as u64, 0)
        .map_err(|_| b.limit("map entries length"))?;
    let label_bytes = labels(ir, entry_layout, None, b)?;
    let (mut count, mut bytes) = (1usize, 0u64);
    for root in roots {
        b.work(1)?;
        let (key, value) = entry(&receiver.nodes, *root).ok_or_else(|| b.invalid("map entry"))?;
        let k = &receiver.nodes[key];
        let v = &receiver.nodes[value];
        count = add(count, add(1, add(k.count, v.count, b)?, b)?, b)?;
        bytes = text(bytes, text(label_bytes, text(k.text, v.text, b)?, b)?, b)?;
    }
    b.fuel
        .preflight(count as u64, bytes)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .and_then(|n| n.checked_add(bytes))
            .ok_or_else(|| b.limit("map entries copy work"))?,
    )?;
    b.admit(count, bytes)?;
    let mut nodes = Vec::with_capacity(count);
    let mut items = Vec::with_capacity(length);
    for root in roots {
        b.work(0)?;
        let (key, value) = entry(&receiver.nodes, *root).ok_or_else(|| b.invalid("map entry"))?;
        let key = copy(receiver, key, &mut nodes, b)?;
        let value = copy(receiver, value, &mut nodes, b)?;
        let entry_count = add(1, add(nodes[key].count, nodes[value].count, b)?, b)?;
        let entry_text = text(label_bytes, text(nodes[key].text, nodes[value].text, b)?, b)?;
        nodes.push(Node {
            layout: entry_layout,
            kind: Kind::Record(vec![key, value]),
            count: entry_count,
            text: entry_text,
        });
        items.push(nodes.len() - 1);
    }
    nodes.push(Node {
        layout,
        kind: Kind::Array(items),
        count,
        text: bytes,
    });
    Ok(Value { nodes })
}
