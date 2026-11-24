//! Two-pass iterative display over the VM's flat owned values.
use super::*;
fn record<'a>(
    ir: &'a CheckedGenericIr,
    layout: usize,
    b: &Budget,
) -> Result<(&'a str, &'a [GenericRecordField]), GenericVmError> {
    let Some(GenericRuntimeLayoutKind::Record { declaration, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("display record layout"));
    };
    let Some(CheckedGenericDefinition::Record(r)) = ir
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types()
        .definition(*declaration)
    else {
        return Err(b.invalid("display record identity"));
    };
    Ok((r.syntax().qualified_name(), r.syntax().fields()))
}
fn enumeration<'a>(
    ir: &'a CheckedGenericIr,
    layout: usize,
    tag: usize,
    b: &Budget,
) -> Result<(&'a str, &'a str), GenericVmError> {
    let Some(GenericRuntimeLayoutKind::Enum { declaration, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("display enum layout"));
    };
    let Some(CheckedGenericDefinition::Enum(e)) = ir
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies()
        .signatures()
        .types()
        .definition(*declaration)
    else {
        return Err(b.invalid("display enum identity"));
    };
    let variant = e
        .syntax()
        .variants()
        .get(tag)
        .ok_or_else(|| b.invalid("display enum tag"))?;
    Ok((e.syntax().qualified_name(), &variant.name))
}
fn collection_name(
    ir: &CheckedGenericIr,
    node: &Node,
    b: &Budget,
) -> Result<&'static str, GenericVmError> {
    match (
        &node.kind,
        ir.plans().layouts().layout(node.layout).map(|l| l.kind()),
    ) {
        (Kind::Map(_), Some(GenericRuntimeLayoutKind::Map { .. })) => Ok("std::Map"),
        (Kind::Set(_), Some(GenericRuntimeLayoutKind::Set { .. })) => Ok("std::Set"),
        _ => Err(b.invalid("display collection layout")),
    }
}
fn entry_children(
    value: &Value,
    node: &Node,
    ir: &CheckedGenericIr,
    b: &Budget,
) -> Result<(usize, usize), GenericVmError> {
    let Kind::Entry { key, value: child } = node.kind else {
        return Err(b.invalid("display private entry"));
    };
    let Some(GenericRuntimeLayoutKind::Map {
        key: expected_key,
        value: expected_value,
        ..
    }) = ir.plans().layouts().layout(node.layout).map(|l| l.kind())
    else {
        return Err(b.invalid("display entry layout"));
    };
    if value.nodes.get(key).map(|n| n.layout) != Some(*expected_key)
        || value.nodes.get(child).map(|n| n.layout) != Some(*expected_value)
    {
        return Err(b.invalid("display entry children"));
    }
    Ok((key, child))
}
fn separators(n: usize, b: &Budget) -> Result<u64, GenericVmError> {
    (n.saturating_sub(1) as u64)
        .checked_mul(2)
        .ok_or_else(|| b.limit("display separators"))
}
pub(super) fn format(
    ir: &CheckedGenericIr,
    value: &Value,
    layout: usize,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    b.work(value.nodes.len() as u64)?;
    let mut bytes = 0u64;
    for node in &value.nodes {
        b.work(1)?;
        let size = match &node.kind {
            Kind::Map(items) | Kind::Set(items) => {
                let name = collection_name(ir, node, b)?;
                for child in items {
                    b.work(1)?;
                    let child = value
                        .nodes
                        .get(*child)
                        .ok_or_else(|| b.invalid("display collection child"))?;
                    if matches!(node.kind, Kind::Map(_)) {
                        if child.layout != node.layout || !matches!(child.kind, Kind::Entry { .. })
                        {
                            return Err(b.invalid("display map entry owner"));
                        }
                    } else {
                        let Some(GenericRuntimeLayoutKind::Set { key, .. }) =
                            ir.plans().layouts().layout(node.layout).map(|l| l.kind())
                        else {
                            unreachable!()
                        };
                        if child.layout != *key {
                            return Err(b.invalid("display set key"));
                        }
                    }
                }
                (name.len() as u64)
                    .checked_add(if items.is_empty() { 4 } else { 5 })
                    .and_then(|n| n.checked_add(separators(items.len(), b).ok()?))
                    .ok_or_else(|| b.limit("display size"))?
            }
            Kind::Entry { .. } => {
                entry_children(value, node, ir, b)?;
                2
            }
            Kind::Unit => 0,
            Kind::Int(n) => {
                b.work(400)?;
                n.to_string().len() as u64
            }
            Kind::Byte(n) => {
                b.work(400)?;
                n.to_string().len() as u64
            }
            Kind::Float(n) => {
                b.work(400)?;
                n.to_string().len() as u64
            }
            Kind::Bool(n) => {
                if *n {
                    4
                } else {
                    5
                }
            }
            Kind::String(s) => s.len() as u64,
            Kind::Bytes(items) => 13u64
                .checked_add(separators(items.len(), b)?)
                .ok_or_else(|| b.limit("display size"))?,
            Kind::Array(items) => 2u64
                .checked_add(separators(items.len(), b)?)
                .ok_or_else(|| b.limit("display size"))?,
            Kind::Record(items) => {
                let (name, fields) = record(ir, node.layout, b)?;
                if fields.len() != items.len() {
                    return Err(b.invalid("display record fields"));
                }
                let mut n = (name.len() as u64)
                    .checked_add(5)
                    .and_then(|n| n.checked_add(separators(items.len(), b).ok()?))
                    .ok_or_else(|| b.limit("display size"))?;
                for field in fields {
                    b.work(1)?;
                    n = n
                        .checked_add(field.name.len() as u64)
                        .and_then(|n| n.checked_add(2))
                        .ok_or_else(|| b.limit("display size"))?;
                }
                n
            }
            Kind::Enum { tag, .. } => {
                let (name, label) = enumeration(ir, node.layout, *tag, b)?;
                (name.len() as u64)
                    .checked_add(label.len() as u64)
                    .and_then(|n| n.checked_add(4))
                    .ok_or_else(|| b.limit("display size"))?
            }
        };
        bytes = bytes
            .checked_add(size)
            .ok_or_else(|| b.limit("display size"))?;
    }
    let capacity = usize::try_from(bytes).map_err(|_| b.limit("display size"))?;
    let temporary = value
        .nodes
        .len()
        .checked_mul(4)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| b.limit("display pending size"))?;
    b.work(bytes)?;
    b.admit(
        temporary
            .checked_add(1)
            .ok_or_else(|| b.limit("display node size"))?,
        bytes,
    )?;
    enum Part<'a> {
        Node(usize),
        Text(&'a str),
    }
    let mut pending = vec![Part::Node(value.nodes.len() - 1)];
    let mut text = String::with_capacity(capacity);
    let append = |text: &mut String, part: &str, b: &mut Budget| -> Result<(), GenericVmError> {
        b.work(part.len() as u64)?;
        if text.len() > capacity || part.len() > capacity - text.len() {
            return Err(b.invalid("display preflight mismatch"));
        }
        text.push_str(part);
        Ok(())
    };
    while let Some(part) = pending.pop() {
        b.work(1)?;
        match part {
            Part::Text(s) => append(&mut text, s, b)?,
            Part::Node(index) => {
                let node = value
                    .nodes
                    .get(index)
                    .ok_or_else(|| b.invalid("display child"))?;
                match &node.kind {
                    Kind::Map(items) | Kind::Set(items) => {
                        let name = collection_name(ir, node, b)?;
                        append(&mut text, name, b)?;
                        append(&mut text, " {", b)?;
                        if !items.is_empty() {
                            append(&mut text, " ", b)?;
                        }
                        pending.push(Part::Text(" }"));
                        for (i, child) in items.iter().enumerate().rev() {
                            b.work(1)?;
                            pending.push(Part::Node(*child));
                            if i > 0 {
                                pending.push(Part::Text(", "));
                            }
                        }
                    }
                    Kind::Entry { .. } => {
                        let (key, value) = entry_children(value, node, ir, b)?;
                        pending.push(Part::Node(value));
                        pending.push(Part::Text(": "));
                        pending.push(Part::Node(key));
                    }
                    Kind::Unit => {}
                    Kind::Int(n) => {
                        b.work(400)?;
                        append(&mut text, &n.to_string(), b)?;
                    }
                    Kind::Byte(n) => {
                        b.work(400)?;
                        append(&mut text, &n.to_string(), b)?;
                    }
                    Kind::Float(n) => {
                        b.work(400)?;
                        append(&mut text, &n.to_string(), b)?;
                    }
                    Kind::Bool(n) => append(&mut text, if *n { "true" } else { "false" }, b)?,
                    Kind::String(s) => append(&mut text, s, b)?,
                    Kind::Array(items) | Kind::Bytes(items) => {
                        append(
                            &mut text,
                            if matches!(node.kind, Kind::Bytes(_)) {
                                "std::Bytes ["
                            } else {
                                "["
                            },
                            b,
                        )?;
                        pending.push(Part::Text("]"));
                        for (i, child) in items.iter().enumerate().rev() {
                            pending.push(Part::Node(*child));
                            if i > 0 {
                                pending.push(Part::Text(", "));
                            }
                        }
                    }
                    Kind::Record(items) => {
                        let (name, fields) = record(ir, node.layout, b)?;
                        append(&mut text, name, b)?;
                        append(&mut text, " { ", b)?;
                        pending.push(Part::Text(" }"));
                        for (i, (field, child)) in fields.iter().zip(items).enumerate().rev() {
                            pending.push(Part::Node(*child));
                            pending.push(Part::Text(": "));
                            pending.push(Part::Text(&field.name));
                            if i > 0 {
                                pending.push(Part::Text(", "));
                            }
                        }
                    }
                    Kind::Enum { tag, payload } => {
                        let (name, label) = enumeration(ir, node.layout, *tag, b)?;
                        append(&mut text, name, b)?;
                        append(&mut text, "::", b)?;
                        append(&mut text, label, b)?;
                        append(&mut text, "(", b)?;
                        pending.push(Part::Text(")"));
                        if let Some(child) = payload {
                            pending.push(Part::Node(*child));
                        }
                    }
                }
            }
        }
        if pending.len() > temporary {
            return Err(b.invalid("display pending bound"));
        }
    }
    if text.len() != capacity {
        return Err(b.invalid("display final size"));
    }
    Ok(Value {
        nodes: vec![Node {
            layout,
            kind: Kind::String(text),
            count: 1,
            text: bytes,
        }],
    })
}
