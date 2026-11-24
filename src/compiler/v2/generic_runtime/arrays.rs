//! Bounded snapshot array operations and explicit recursive numeric widening.
use super::*;
fn convert(
    ir: &CheckedGenericIr,
    value: &Value,
    target: usize,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let (mut source, mut expected) = (value.root().layout, target);
    let mut pairs = Vec::new();
    loop {
        b.work(pairs.len() as u64 + 1)?;
        if source == expected {
            break;
        }
        if pairs.iter().any(|(s, _, _)| *s == source) {
            return Err(b.invalid("array conversion cycle"));
        }
        let shape = |id| {
            ir.plans()
                .layouts()
                .layout(id)
                .map(|l| l.kind())
                .ok_or_else(|| b.invalid("array conversion layout"))
        };
        let next = match (shape(source)?, shape(expected)?) {
            (
                GenericRuntimeLayoutKind::Array { element: a },
                GenericRuntimeLayoutKind::Array { element: e },
            ) => Some((*a, *e)),
            (
                GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int),
                GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Float),
            ) => None,
            _ => return Err(b.invalid("array conversion type")),
        };
        b.admit(1, 0)?;
        pairs.push((source, expected, next.is_none()));
        if let Some((s, e)) = next {
            source = s;
            expected = e;
        } else {
            break;
        }
    }
    let mut copied = b.snapshot(value, value.nodes.len() - 1)?;
    let work = (copied.nodes.len() as u64)
        .checked_mul(pairs.len() as u64 + 1)
        .ok_or_else(|| b.limit("array conversion work"))?;
    b.work(work)?;
    for node in &mut copied.nodes {
        b.work(0)?;
        if let Some((_, expected, numeric)) =
            pairs.iter().find(|(source, _, _)| *source == node.layout)
        {
            if *numeric {
                let Kind::Int(n) = node.kind else {
                    return Err(b.invalid("numeric array conversion value"));
                };
                node.kind = Kind::Float(n as f64);
            } else if !matches!(node.kind, Kind::Array(_)) {
                return Err(b.invalid("array conversion value"));
            }
            node.layout = *expected;
        }
    }
    if copied.root().layout != target {
        return Err(b.invalid("array conversion result"));
    }
    Ok(copied)
}
pub(super) fn execute(
    ir: &CheckedGenericIr,
    builtin: GenericBuiltin,
    layout: usize,
    right_layout: Option<usize>,
    args: &mut [Value],
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    if let Some(target) = right_layout {
        if args[1].root().layout != target {
            args[1] = convert(ir, &args[1], target, b)?;
        }
    }
    let Kind::Array(left) = &args[0].root().kind else {
        return Err(b.invalid("array builtin first argument"));
    };
    if args[0].root().layout != layout {
        return Err(b.invalid("array builtin result layout"));
    }
    let root = [args[1].nodes.len() - 1];
    let (selected, right, other) = match builtin {
        GenericBuiltin::ArrayAppend => (left.as_slice(), root.as_slice(), &args[1]),
        GenericBuiltin::ArrayConcat => {
            let Kind::Array(right) = &args[1].root().kind else {
                return Err(b.invalid("concat array argument"));
            };
            (left.as_slice(), right.as_slice(), &args[1])
        }
        GenericBuiltin::ArraySlice => {
            let (Kind::Int(start), Kind::Int(end)) = (&args[1].root().kind, &args[2].root().kind)
            else {
                return Err(b.invalid("slice bounds type"));
            };
            let start = usize::try_from(*start).map_err(|_| b.error(GenericVmErrorKind::Bounds))?;
            let end = usize::try_from(*end).map_err(|_| b.error(GenericVmErrorKind::Bounds))?;
            (
                left.get(start..end)
                    .ok_or_else(|| b.error(GenericVmErrorKind::Bounds))?,
                &[][..],
                &args[0],
            )
        }
        _ => return Err(b.invalid("array operation kind")),
    };
    let length =
        crate::compiler::stdlib::array_growth_len(selected.len() as u64, right.len() as u64)
            .map_err(|_| b.limit("array result length"))?;
    b.work(
        (length as u64)
            .checked_mul(3)
            .ok_or_else(|| b.limit("array scan work"))?,
    )?;
    let items = || {
        selected
            .iter()
            .map(|i| (&args[0], *i))
            .chain(right.iter().map(|i| (other, *i)))
    };
    let (mut nodes, mut text) = (1usize, 0u64);
    for (value, root) in items() {
        let child = value
            .nodes
            .get(root)
            .ok_or_else(|| b.invalid("array child"))?;
        nodes = nodes
            .checked_add(child.count)
            .ok_or_else(|| b.limit("array snapshot nodes"))?;
        text = text
            .checked_add(child.text)
            .ok_or_else(|| b.limit("array snapshot text"))?;
    }
    let total = nodes
        .checked_add(length)
        .ok_or_else(|| b.limit("array snapshot temporary nodes"))?;
    b.fuel
        .preflight(total as u64, text)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.admit(length, 0)?;
    let mut values = Vec::with_capacity(length);
    for (value, root) in items() {
        values.push(b.snapshot(value, root)?);
    }
    b.wrapper(layout, values, 0, Kind::Array)
}
