//! Explicit JSON bridge; staging has separate parser limits, output ownership uses shared VM fuel.
use super::*;
use crate::compiler::v2::JsonValue as J;
fn payload(
    ir: &CheckedGenericIr,
    layout: usize,
    tag: usize,
    b: &Budget,
) -> Result<Option<usize>, GenericVmError> {
    let Some(GenericRuntimeLayoutKind::Enum { variants, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("JSON enum layout"));
    };
    variants
        .get(tag)
        .copied()
        .ok_or_else(|| b.invalid("JSON tag"))
}
fn result(ir: &CheckedGenericIr, layout: usize, b: &Budget) -> Result<usize, GenericVmError> {
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("JSON Result"));
    };
    arguments
        .first()
        .copied()
        .ok_or_else(|| b.invalid("JSON Result arguments"))
}
fn tag(v: &J) -> usize {
    match v {
        J::Null => 0,
        J::Bool(_) => 1,
        J::Int(_) => 2,
        J::Float(_) => 3,
        J::String(_) => 4,
        J::Array(_) => 5,
        J::Object(_) => 6,
    }
}
fn cost(
    v: &J,
    l: usize,
    ir: &CheckedGenericIr,
    b: &mut Budget,
    d: usize,
) -> Result<(usize, u64), GenericVmError> {
    if d > 128 {
        return Err(b.limit("JSON depth"));
    }
    b.work(1)?;
    let mut n = if matches!(v, J::Null) { 1 } else { 2 };
    let mut t = labels(ir, l, Some(tag(v)), b)?;
    match v {
        J::String(s) => {
            b.work(s.len() as u64)?;
            t = t
                .checked_add(s.len() as u64)
                .ok_or_else(|| b.limit("JSON text"))?;
        }
        J::Array(items) => {
            for item in items {
                let (c, s) = cost(item, l, ir, b, d + 1)?;
                n = usize::checked_add(n, c).ok_or_else(|| b.limit("JSON nodes"))?;
                t = t.checked_add(s).ok_or_else(|| b.limit("JSON text"))?;
            }
        }
        J::Object(items) => {
            for (k, v) in items {
                b.work(k.len() as u64)?;
                let (c, s) = cost(v, l, ir, b, d + 1)?;
                n = usize::checked_add(n, c + 2).ok_or_else(|| b.limit("JSON nodes"))?;
                t = t
                    .checked_add(s)
                    .and_then(|t| t.checked_add(k.len() as u64))
                    .ok_or_else(|| b.limit("JSON text"))?;
            }
        }
        _ => {}
    }
    Ok((n, t))
}
fn build(
    v: &J,
    l: usize,
    ir: &CheckedGenericIr,
    b: &mut Budget,
    d: usize,
) -> Result<Value, GenericVmError> {
    if d > 128 {
        return Err(b.limit("JSON depth"));
    }
    let tag = tag(v);
    let p = payload(ir, l, tag, b)?;
    let mut values = Vec::new();
    if let Some(p) = p {
        let out = match v {
            J::Bool(n) => b.scalar(p, Kind::Bool(*n), 0)?,
            J::Int(n) => b.scalar(p, Kind::Int(*n), 0)?,
            J::Float(n) => b.scalar(p, Kind::Float(*n), 0)?,
            J::String(s) => b.string(p, s)?,
            J::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.push(build(item, l, ir, b, d + 1)?);
                }
                b.wrapper(p, out, 0, Kind::Array)?
            }
            J::Object(items) => {
                let Some(GenericRuntimeLayoutKind::Map { key, .. }) =
                    ir.plans().layouts().layout(p).map(|l| l.kind())
                else {
                    return Err(b.invalid("JSON object map"));
                };
                let mut out = Vec::new();
                for (k, v) in items {
                    let k = b.string(*key, k)?;
                    let v = build(v, l, ir, b, d + 1)?;
                    out.push(b.wrapper(p, vec![k, v], 0, |r| Kind::Entry {
                        key: r[0],
                        value: r[1],
                    })?);
                }
                b.wrapper(p, out, 0, Kind::Map)?
            }
            J::Null => return Err(b.invalid("JSON Null payload")),
        };
        values.push(out);
    }
    let text = labels(ir, l, Some(tag), b)?;
    b.wrapper(l, values, text, |v| Kind::Enum {
        tag,
        payload: v.first().copied(),
    })
}
fn extract(
    input: &Value,
    root: usize,
    b: &mut Budget,
    d: usize,
    left: &mut usize,
) -> Result<J, GenericVmError> {
    if d > 128 || *left == 0 {
        return Err(b.limit("JSON staging depth/values"));
    }
    *left -= 1;
    b.work(1)?;
    let Kind::Enum { tag, payload } = input.nodes[root].kind else {
        return Err(b.invalid("JSON value"));
    };
    if tag == 0 {
        return Ok(J::Null);
    }
    let p = payload.ok_or_else(|| b.invalid("JSON payload"))?;
    Ok(match (&input.nodes[p].kind, tag) {
        (Kind::Bool(n), 1) => J::Bool(*n),
        (Kind::Int(n), 2) => J::Int(*n),
        (Kind::Float(n), 3) => J::Float(*n),
        (Kind::String(s), 4) => {
            b.work(s.len() as u64)?;
            J::String(s.clone())
        }
        (Kind::Array(items), 5) => {
            let mut out = Vec::new();
            for child in items {
                out.push(extract(input, *child, b, d + 1, left)?);
            }
            J::Array(out)
        }
        (Kind::Map(items), 6) => {
            let mut out = Vec::new();
            for entry in items {
                if *left == 0 {
                    return Err(b.limit("JSON staging members"));
                }
                *left -= 1;
                let (k, v) = collection_values::entry(&input.nodes, *entry)
                    .ok_or_else(|| b.invalid("JSON map entry"))?;
                let Kind::String(k) = &input.nodes[k].kind else {
                    return Err(b.invalid("JSON object key"));
                };
                b.work(k.len() as u64 + 1)?;
                out.push((k.clone(), extract(input, v, b, d + 1, left)?));
            }
            J::Object(out)
        }
        _ => return Err(b.invalid("JSON payload kind")),
    })
}
fn failure(
    layout: usize,
    e: crate::compiler::v2::JsonError,
    parse: bool,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let code = match e.code {
        "duplicate" => "json_duplicate",
        "number" => "json_number",
        "limit" | "policy" => "json_limit",
        "utf8" => "json_utf8",
        _ => "json_syntax",
    };
    let offset = if parse {
        Some(i64::try_from(e.offset).map_err(|_| b.limit("JSON offset"))?)
    } else {
        None
    };
    data_error_result(layout, code, "JSON operation failed", offset, ir, b)
}
pub(super) fn execute(
    builtin: GenericBuiltin,
    layout: usize,
    input: &Value,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let limits = b.json;
    if builtin == GenericBuiltin::HostCall {
        let mut left = limits.values;
        let request = extract(input, input.nodes.len() - 1, b, 0, &mut left)?;
        let host = b.host;
        let response = match host {
            DataExecutionHost::V3(host) => host.call(&request, || b.work(0)),
            DataExecutionHost::Network(host) => host
                .try_borrow_mut()
                .map_err(|_| b.invalid("network host is already borrowed"))?
                .call(&request, || b.work(0)),
        };
        let value = match response? {
            Ok(value) => value,
            Err(e) => {
                if e.code == "cleanup" {
                    return Err(b.error(GenericVmErrorKind::HostCleanup(e.message)));
                }
                return data_error_result(layout, e.code, &e.message, None, ir, b);
            }
        };
        let json = result(ir, layout, b)?;
        let (n, t) = cost(&value, json, ir, b, 0)?;
        let label = labels(ir, layout, Some(0), b)?;
        b.fuel
            .preflight(
                (n as u64) + 1,
                t.checked_add(label)
                    .ok_or_else(|| b.limit("host Result text"))?,
            )
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        let value = build(&value, json, ir, b, 0)?;
        return b.wrapper(layout, vec![value], label, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    if builtin == GenericBuiltin::JsonParse {
        let Kind::String(text) = &input.root().kind else {
            return Err(b.invalid("JSON input String"));
        };
        let ast =
            match super::super::runtime_json::parse_json_checked(text.as_bytes(), limits, |n| {
                b.work(n)
            })? {
                Ok(v) => v,
                Err(e) => return failure(layout, e, true, ir, b),
            };
        let json = result(ir, layout, b)?;
        let (n, t) = cost(&ast, json, ir, b, 0)?;
        let label = labels(ir, layout, Some(0), b)?;
        let total = t
            .checked_add(label)
            .ok_or_else(|| b.limit("JSON Result text"))?;
        b.fuel
            .preflight((n as u64) + 1, total)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        let value = build(&ast, json, ir, b, 0)?;
        return b.wrapper(layout, vec![value], label, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    let mut left = limits.values;
    let ast = extract(input, input.nodes.len() - 1, b, 0, &mut left)?;
    let text =
        match super::super::runtime_json::serialize_json_checked(&ast, limits, |n| b.work(n))? {
            Ok(text) => text,
            Err(e) => return failure(layout, e, false, ir, b),
        };
    let label = labels(ir, layout, Some(0), b)?;
    b.fuel
        .preflight(
            2,
            (text.len() as u64)
                .checked_add(label)
                .ok_or_else(|| b.limit("JSON serialization text"))?,
        )
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    let value = b.string(result(ir, layout, b)?, &text)?;
    b.wrapper(layout, vec![value], label, |v| Kind::Enum {
        tag: 0,
        payload: Some(v[0]),
    })
}
