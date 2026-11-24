//! Checked flat-VM codecs; shared strict digit rules, no intermediate packed buffer.
use super::*;
pub(super) fn hex(
    builtin: GenericBuiltin,
    layout: usize,
    input: &Value,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("hex Result layout"));
    };
    let payload_layout = arguments[0];
    if builtin == GenericBuiltin::BytesToHex {
        let Kind::Bytes(items) = &input.root().kind else {
            return Err(b.invalid("hex buffer operand"));
        };
        let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) = ir
            .plans()
            .layouts()
            .layout(input.root().layout)
            .map(|l| l.kind())
        else {
            return Err(b.invalid("hex buffer layout"));
        };
        if !matches!(
            ir.plans()
                .layouts()
                .layout(payload_layout)
                .map(|l| l.kind()),
            Some(GenericRuntimeLayoutKind::Primitive(
                GenericPrimitive::String
            ))
        ) {
            return Err(b.invalid("hex String payload"));
        }
        let len = items
            .len()
            .checked_mul(2)
            .ok_or_else(|| b.limit("hex encoded length"))?;
        if items.len() > b.max_byte_buffer_bytes
            || len > super::super::RuntimeBytesLimits::DEFAULT.max_text_bytes
        {
            return data_error_result(layout, "capacity", "codec capacity exceeded", None, ir, b);
        }
        let labels = labels(ir, layout, Some(0), b)?;
        let bytes = u64::try_from(len).map_err(|_| b.limit("hex text size"))?;
        let total = bytes
            .checked_add(labels)
            .ok_or_else(|| b.limit("hex result text"))?;
        b.fuel
            .preflight(2, total)
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        b.work(
            bytes
                .checked_add(1)
                .ok_or_else(|| b.limit("hex encode work"))?,
        )?;
        b.admit(1, bytes)?;
        let mut text = String::with_capacity(len);
        for child in items {
            b.work(0)?;
            let node = &input.nodes[*child];
            let Kind::Byte(value) = node.kind else {
                return Err(b.invalid("hex Byte"));
            };
            if node.layout != *byte {
                return Err(b.invalid("hex Byte identity"));
            }
            text.push(b"0123456789abcdef"[(value >> 4) as usize] as char);
            text.push(b"0123456789abcdef"[(value & 15) as usize] as char);
        }
        let value = Value {
            nodes: vec![Node {
                layout: payload_layout,
                kind: Kind::String(text),
                count: 1,
                text: bytes,
            }],
        };
        return b.wrapper(layout, vec![value], labels, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    let Kind::String(text) = &input.root().kind else {
        return Err(b.invalid("hex String operand"));
    };
    let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) = ir
        .plans()
        .layouts()
        .layout(payload_layout)
        .map(|l| l.kind())
    else {
        return Err(b.invalid("hex buffer payload"));
    };
    let len = text.len() / 2;
    if text.len() > super::super::RuntimeBytesLimits::DEFAULT.max_text_bytes
        || len > b.max_byte_buffer_bytes
    {
        return data_error_result(layout, "capacity", "codec capacity exceeded", None, ir, b);
    }
    // Match the packed helper: capacity, then first invalid original byte, then odd length.
    for (i, c) in text.bytes().enumerate() {
        b.work(1)?;
        if super::super::runtime_bytes::hex(c).is_none() {
            return invalid(layout, i, ir, b);
        }
    }
    if text.len() % 2 != 0 {
        return invalid(layout, text.len() - 1, ir, b);
    }
    let count = len
        .checked_add(1)
        .ok_or_else(|| b.limit("hex buffer size"))?;
    let labels = labels(ir, layout, Some(0), b)?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .ok_or_else(|| b.limit("hex decode work"))?,
    )?;
    b.fuel
        .preflight((count as u64) + 1, labels)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.admit(count, 0)?;
    let mut nodes = Vec::with_capacity(count);
    let mut items = Vec::with_capacity(len);
    for pair in text.as_bytes().chunks_exact(2) {
        b.work(0)?;
        let hi = super::super::runtime_bytes::hex(pair[0])
            .ok_or_else(|| b.invalid("validated hex digit"))?;
        let lo = super::super::runtime_bytes::hex(pair[1])
            .ok_or_else(|| b.invalid("validated hex digit"))?;
        items.push(nodes.len());
        nodes.push(Node {
            layout: *byte,
            kind: Kind::Byte((hi << 4) | lo),
            count: 1,
            text: 0,
        });
    }
    nodes.push(Node {
        layout: payload_layout,
        kind: Kind::Bytes(items),
        count,
        text: 0,
    });
    b.wrapper(layout, vec![Value { nodes }], labels, |v| Kind::Enum {
        tag: 0,
        payload: Some(v[0]),
    })
}
fn invalid(
    layout: usize,
    offset: usize,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let offset = i64::try_from(offset).map_err(|_| b.limit("hex error offset"))?;
    data_error_result(
        layout,
        "hex",
        "invalid hexadecimal input",
        Some(offset),
        ir,
        b,
    )
}

pub(super) fn base64(
    builtin: GenericBuiltin,
    layout: usize,
    input: &Value,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    use super::super::runtime_bytes::{sextet, validate_base64, BASE64};
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("base64 Result"));
    };
    let payload_layout = arguments[0];
    if builtin == GenericBuiltin::BytesToBase64 {
        let Kind::Bytes(items) = &input.root().kind else {
            return Err(b.invalid("base64 buffer"));
        };
        let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) = ir
            .plans()
            .layouts()
            .layout(input.root().layout)
            .map(|l| l.kind())
        else {
            return Err(b.invalid("base64 buffer layout"));
        };
        if !matches!(
            ir.plans()
                .layouts()
                .layout(payload_layout)
                .map(|l| l.kind()),
            Some(GenericRuntimeLayoutKind::Primitive(
                GenericPrimitive::String
            ))
        ) {
            return Err(b.invalid("base64 String payload"));
        }
        let len = items
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| b.limit("base64 encoded length"))?;
        if items.len() > b.max_byte_buffer_bytes
            || len > super::super::RuntimeBytesLimits::DEFAULT.max_text_bytes
        {
            return data_error_result(layout, "capacity", "codec capacity exceeded", None, ir, b);
        }
        let labels = labels(ir, layout, Some(0), b)?;
        let bytes = u64::try_from(len).map_err(|_| b.limit("base64 text size"))?;
        b.fuel
            .preflight(
                2,
                bytes
                    .checked_add(labels)
                    .ok_or_else(|| b.limit("base64 result text"))?,
            )
            .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
        b.work(
            bytes
                .checked_add(1)
                .ok_or_else(|| b.limit("base64 encode work"))?,
        )?;
        b.admit(1, bytes)?;
        let mut text = String::with_capacity(len);
        for chunk in items.chunks(3) {
            b.work(0)?;
            let mut values = [0u8; 3];
            for (i, child) in chunk.iter().enumerate() {
                let node = &input.nodes[*child];
                let Kind::Byte(value) = node.kind else {
                    return Err(b.invalid("base64 Byte"));
                };
                if node.layout != *byte {
                    return Err(b.invalid("base64 Byte identity"));
                }
                values[i] = value;
            }
            let [a, c, d] = values;
            text.push(BASE64[(a >> 2) as usize] as char);
            text.push(BASE64[(((a & 3) << 4) | (c >> 4)) as usize] as char);
            text.push(if chunk.len() > 1 {
                BASE64[(((c & 15) << 2) | (d >> 6)) as usize] as char
            } else {
                '='
            });
            text.push(if chunk.len() > 2 {
                BASE64[(d & 63) as usize] as char
            } else {
                '='
            });
        }
        let value = Value {
            nodes: vec![Node {
                layout: payload_layout,
                kind: Kind::String(text),
                count: 1,
                text: bytes,
            }],
        };
        return b.wrapper(layout, vec![value], labels, |v| Kind::Enum {
            tag: 0,
            payload: Some(v[0]),
        });
    }
    let Kind::String(text) = &input.root().kind else {
        return Err(b.invalid("base64 String operand"));
    };
    let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) = ir
        .plans()
        .layouts()
        .layout(payload_layout)
        .map(|l| l.kind())
    else {
        return Err(b.invalid("base64 buffer payload"));
    };
    if text.len() > super::super::RuntimeBytesLimits::DEFAULT.max_text_bytes {
        return data_error_result(layout, "capacity", "codec capacity exceeded", None, ir, b);
    }
    let len = match validate_base64(text, || b.work(1))? {
        Ok(n) => n,
        Err(e) => {
            if e.kind == super::super::RuntimeDataErrorKind::Capacity {
                return data_error_result(
                    layout,
                    "capacity",
                    "codec capacity exceeded",
                    None,
                    ir,
                    b,
                );
            }
            let offset = e.offset.ok_or_else(|| b.invalid("base64 error offset"))?;
            let offset = i64::try_from(offset).map_err(|_| b.limit("base64 error offset"))?;
            return data_error_result(
                layout,
                "base64",
                "invalid base64 input",
                Some(offset),
                ir,
                b,
            );
        }
    };
    if len > b.max_byte_buffer_bytes {
        return data_error_result(layout, "capacity", "codec capacity exceeded", None, ir, b);
    }
    let count = len
        .checked_add(1)
        .ok_or_else(|| b.limit("base64 buffer size"))?;
    let labels = labels(ir, layout, Some(0), b)?;
    b.work(
        (count as u64)
            .checked_mul(2)
            .ok_or_else(|| b.limit("base64 decode work"))?,
    )?;
    b.fuel
        .preflight((count as u64) + 1, labels)
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.admit(count, 0)?;
    let mut nodes = Vec::with_capacity(count);
    let mut items = Vec::with_capacity(len);
    for chunk in text.as_bytes().chunks_exact(4) {
        b.work(0)?;
        let mut bits = 0u32;
        for c in chunk {
            let v = if *c == b'=' {
                0
            } else {
                sextet(*c).ok_or_else(|| b.invalid("validated base64 digit"))?
            };
            bits = (bits << 6) | u32::from(v);
        }
        for shift in [16, 8, 0] {
            if items.len() < len {
                items.push(nodes.len());
                nodes.push(Node {
                    layout: *byte,
                    kind: Kind::Byte((bits >> shift) as u8),
                    count: 1,
                    text: 0,
                });
            }
        }
    }
    nodes.push(Node {
        layout: payload_layout,
        kind: Kind::Bytes(items),
        count,
        text: 0,
    });
    b.wrapper(layout, vec![Value { nodes }], labels, |v| Kind::Enum {
        tag: 0,
        payload: Some(v[0]),
    })
}

pub(super) fn utf8(
    layout: usize,
    input: &Value,
    ir: &CheckedGenericIr,
    b: &mut Budget,
) -> Result<Value, GenericVmError> {
    let Kind::Bytes(items) = &input.root().kind else {
        return Err(b.invalid("UTF-8 buffer operand"));
    };
    let Some(GenericRuntimeLayoutKind::Bytes { byte, .. }) = ir
        .plans()
        .layouts()
        .layout(input.root().layout)
        .map(|l| l.kind())
    else {
        return Err(b.invalid("UTF-8 buffer layout"));
    };
    let Some(GenericRuntimeLayoutKind::Enum { arguments, .. }) =
        ir.plans().layouts().layout(layout).map(|l| l.kind())
    else {
        return Err(b.invalid("UTF-8 Result layout"));
    };
    let payload = arguments[0];
    if !matches!(
        ir.plans().layouts().layout(payload).map(|l| l.kind()),
        Some(GenericRuntimeLayoutKind::Primitive(
            GenericPrimitive::String
        ))
    ) {
        return Err(b.invalid("UTF-8 String payload"));
    }
    if items.len() > b.max_byte_buffer_bytes
        || items.len() > super::super::RuntimeBytesLimits::DEFAULT.max_text_bytes
    {
        return data_error_result(layout, "capacity", "codec capacity exceeded", None, ir, b);
    }
    b.work(items.len() as u64)?;
    let mut index = 0;
    while index < items.len() {
        let Some((_, width)) = utf8_segment(input, items, index, *byte, b)? else {
            let offset = i64::try_from(index).map_err(|_| b.limit("UTF-8 error offset"))?;
            return data_error_result(layout, "utf8", "invalid UTF-8 input", Some(offset), ir, b);
        };
        index += width;
    }
    let labels = labels(ir, layout, Some(0), b)?;
    let bytes = items.len() as u64;
    b.fuel
        .preflight(
            2,
            bytes
                .checked_add(labels)
                .ok_or_else(|| b.limit("UTF-8 result text"))?,
        )
        .map_err(|e| b.error(GenericVmErrorKind::Materialization(e)))?;
    b.work(
        bytes
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| b.limit("UTF-8 copy work"))?,
    )?;
    b.admit(1, bytes)?;
    let mut text = String::with_capacity(items.len());
    index = 0;
    while index < items.len() {
        let (segment, width) = utf8_segment(input, items, index, *byte, b)?
            .ok_or_else(|| b.invalid("validated UTF-8 segment"))?;
        let segment = std::str::from_utf8(&segment[..width])
            .map_err(|_| b.invalid("validated UTF-8 text"))?;
        text.push_str(segment);
        index += width;
    }
    let value = Value {
        nodes: vec![Node {
            layout: payload,
            kind: Kind::String(text),
            count: 1,
            text: bytes,
        }],
    };
    b.wrapper(layout, vec![value], labels, |v| Kind::Enum {
        tag: 0,
        payload: Some(v[0]),
    })
}
// A stack-only, at-most-four-byte view lets std enforce RFC 3629 without a temporary buffer.
fn utf8_segment(
    input: &Value,
    items: &[usize],
    start: usize,
    byte_layout: usize,
    b: &mut Budget,
) -> Result<Option<([u8; 4], usize)>, GenericVmError> {
    let mut read = |i: usize| -> Result<u8, GenericVmError> {
        b.work(0)?;
        let node = &input.nodes[items[i]];
        let Kind::Byte(value) = node.kind else {
            return Err(b.invalid("UTF-8 Byte"));
        };
        if node.layout != byte_layout {
            return Err(b.invalid("UTF-8 Byte identity"));
        }
        Ok(value)
    };
    let first = read(start)?;
    let width = match first {
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 1,
    };
    if width > items.len() - start {
        return Ok(None);
    }
    let mut segment = [0u8; 4];
    segment[0] = first;
    for (i, slot) in segment.iter_mut().enumerate().take(width).skip(1) {
        *slot = read(start + i)?;
    }
    if std::str::from_utf8(&segment[..width]).is_err() {
        return Ok(None);
    }
    Ok(Some((segment, width)))
}
