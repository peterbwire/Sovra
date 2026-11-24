//! Bounded strict JSON values; no evaluation, implicit records or host authority.
use std::fmt::Write;
/// Explicit JSON value kinds; object order is insertion order.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// JSON null.
    Null,
    /// JSON Boolean.
    Bool(bool),
    /// Exact signed integer syntax.
    Int(i64),
    /// Finite fraction/exponent syntax; numeric kind survives serialization.
    Float(f64),
    /// Unicode text without normalization.
    String(String),
    /// Ordered values.
    Array(Vec<JsonValue>),
    /// Ordered unique decoded keys.
    Object(Vec<(String, JsonValue)>),
}
/// Explicit parser and serializer ceilings, validated before processing.
#[derive(Debug, Clone, Copy)]
pub struct JsonLimits {
    /// Input/output UTF-8 bytes.
    pub bytes: usize,
    /// Nesting levels, at most 128.
    pub depth: usize,
    /// Combined values and object members.
    pub values: usize,
    /// Aggregate decoded/serialized text bytes.
    pub text: usize,
    /// Shared scan/comparison/copy work.
    pub work: u64,
}
impl JsonLimits {
    /// Finite initial JSON profile.
    pub const DEFAULT: Self = Self {
        bytes: 16 * 1024 * 1024,
        depth: 128,
        values: 1_000_000,
        text: 64 * 1024 * 1024,
        work: 128 * 1024 * 1024,
    };
    pub(super) fn validate(self) -> Result<(), JsonError> {
        if self.depth > 128
            || self.bytes > Self::DEFAULT.bytes
            || self.values > Self::DEFAULT.values
            || self.text > Self::DEFAULT.text
            || self.work > Self::DEFAULT.work
        {
            Err(JsonError {
                offset: 0,
                code: "policy",
            })
        } else {
            Ok(())
        }
    }
}
/// Original zero-based UTF-8 byte refusal; no partial successful value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    /// Original byte position or unexpected end.
    pub offset: usize,
    /// Stable syntax/utf8/number/duplicate/limit/policy category.
    pub code: &'static str,
}
struct Parser<'a, 'b, E> {
    input: &'a str,
    pos: usize,
    left: JsonLimits,
    charge: &'b mut dyn FnMut(u64) -> Result<(), E>,
    external: Option<E>,
}
impl<E> Parser<'_, '_, E> {
    fn err(&self, code: &'static str) -> JsonError {
        JsonError {
            offset: self.pos,
            code,
        }
    }
    fn work(&mut self, n: u64) -> Result<(), JsonError> {
        if let Err(e) = (self.charge)(n) {
            self.external = Some(e);
            return Err(self.err("control"));
        }

        self.left.work = self
            .left
            .work
            .checked_sub(n)
            .ok_or_else(|| self.err("limit"))?;
        Ok(())
    }
    fn bump(&mut self) -> Result<u8, JsonError> {
        self.work(1)?;
        let b = *self
            .input
            .as_bytes()
            .get(self.pos)
            .ok_or_else(|| self.err("syntax"))?;
        self.pos += 1;
        Ok(b)
    }
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }
    fn ws(&mut self) -> Result<(), JsonError> {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\t' | b'\r')) {
            self.bump()?;
        }
        Ok(())
    }
    fn eat(&mut self, c: u8) -> Result<(), JsonError> {
        self.ws()?;
        if self.bump()? == c {
            Ok(())
        } else {
            Err(JsonError {
                offset: self.pos - 1,
                code: "syntax",
            })
        }
    }
    fn node(&mut self) -> Result<(), JsonError> {
        self.left.values = self
            .left
            .values
            .checked_sub(1)
            .ok_or_else(|| self.err("limit"))?;
        Ok(())
    }
    fn push(&mut self, out: &mut String, c: char) -> Result<(), JsonError> {
        let n = c.len_utf8();
        self.left.text = self
            .left
            .text
            .checked_sub(n)
            .ok_or_else(|| self.err("limit"))?;
        self.work(n as u64)?;
        out.push(c);
        Ok(())
    }
    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut n = 0;
        for _ in 0..4 {
            let p = self.pos;
            let b = self.bump()?;
            let d = super::runtime_bytes::hex(b).ok_or(JsonError {
                offset: p,
                code: "syntax",
            })?;
            n = (n << 4) | u32::from(d);
        }
        Ok(n)
    }
    fn string(&mut self) -> Result<String, JsonError> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let at = self.pos;
            let b = self.bump()?;
            match b {
                b'"' => return Ok(out),
                0..=31 => {
                    return Err(JsonError {
                        offset: at,
                        code: "syntax",
                    })
                }
                b'\\' => {
                    let esc = self.bump()?;
                    let c = match esc {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xd800..=0xdbff).contains(&hi) {
                                if self.bump()? != b'\\' || self.bump()? != b'u' {
                                    return Err(JsonError {
                                        offset: at,
                                        code: "syntax",
                                    });
                                }
                                let lo = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&lo) {
                                    return Err(JsonError {
                                        offset: at,
                                        code: "syntax",
                                    });
                                }
                                0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                            } else {
                                hi
                            };
                            char::from_u32(cp).ok_or(JsonError {
                                offset: at,
                                code: "syntax",
                            })?
                        }
                        _ => {
                            return Err(JsonError {
                                offset: self.pos - 1,
                                code: "syntax",
                            })
                        }
                    };
                    self.push(&mut out, c)?;
                }
                _ => {
                    let c = self.input[at..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.err("syntax"))?;
                    let extra = c.len_utf8() - 1;
                    self.work(extra as u64)?;
                    self.pos += extra;
                    self.push(&mut out, c)?;
                }
            }
        }
    }
    fn digits(&mut self) -> Result<(), JsonError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump()?;
        }
        if self.pos == start {
            Err(self.err("syntax"))
        } else {
            Ok(())
        }
    }
    fn number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump()?;
        }
        if self.peek() == Some(b'0') {
            self.bump()?;
        } else {
            if !matches!(self.peek(), Some(b'1'..=b'9')) {
                return Err(self.err("syntax"));
            }
            self.digits()?;
        }
        let mut float = false;
        if self.peek() == Some(b'.') {
            float = true;
            self.bump()?;
            self.digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            float = true;
            self.bump()?;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump()?;
            }
            self.digits()?;
        }
        let text = &self.input[start..self.pos];
        self.work(text.len() as u64)?;
        if float {
            let n = text.parse::<f64>().map_err(|_| JsonError {
                offset: start,
                code: "number",
            })?;
            if !n.is_finite() {
                return Err(JsonError {
                    offset: start,
                    code: "number",
                });
            }
            Ok(JsonValue::Float(n))
        } else {
            text.parse::<i64>()
                .map(JsonValue::Int)
                .map_err(|_| JsonError {
                    offset: start,
                    code: "number",
                })
        }
    }
    fn value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        if depth > self.left.depth {
            return Err(self.err("limit"));
        }
        self.node()?;
        self.ws()?;
        match self.peek() {
            Some(b'n' | b't' | b'f') => {
                let (text, value) = match self.peek() {
                    Some(b'n') => ("null", JsonValue::Null),
                    Some(b't') => ("true", JsonValue::Bool(true)),
                    _ => ("false", JsonValue::Bool(false)),
                };
                for c in text.bytes() {
                    if self.bump()? != c {
                        return Err(self.err("syntax"));
                    }
                }
                Ok(value)
            }
            Some(b'"') => self.string().map(JsonValue::String),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'[') => {
                self.bump()?;
                self.ws()?;
                let mut out = Vec::new();
                if self.peek() == Some(b']') {
                    self.bump()?;
                    return Ok(JsonValue::Array(out));
                }
                loop {
                    out.push(self.value(depth + 1)?);
                    self.ws()?;
                    if self.peek() == Some(b']') {
                        self.bump()?;
                        break;
                    }
                    self.eat(b',')?;
                }
                Ok(JsonValue::Array(out))
            }
            Some(b'{') => {
                self.bump()?;
                self.ws()?;
                let mut out: Vec<(String, JsonValue)> = Vec::new();
                if self.peek() == Some(b'}') {
                    self.bump()?;
                    return Ok(JsonValue::Object(out));
                }
                loop {
                    self.node()?;
                    self.ws()?;
                    let at = self.pos;
                    let key = self.string()?;
                    for (previous, _) in &out {
                        self.work((previous.len() + key.len() + 1) as u64)?;
                        if previous == &key {
                            return Err(JsonError {
                                offset: at,
                                code: "duplicate",
                            });
                        }
                    }
                    self.eat(b':')?;
                    let value = self.value(depth + 1)?;
                    out.push((key, value));
                    self.ws()?;
                    if self.peek() == Some(b'}') {
                        self.bump()?;
                        break;
                    }
                    self.eat(b',')?;
                }
                Ok(JsonValue::Object(out))
            }
            _ => Err(self.err("syntax")),
        }
    }
}
/// Parse strict JSON with exact integer/finite Float kinds, unique decoded keys and bounded growth.
pub fn parse_json(input: &[u8], limits: JsonLimits) -> Result<JsonValue, JsonError> {
    match parse_json_checked(input, limits, |_| Ok::<(), std::convert::Infallible>(())) {
        Ok(out) => out,
        Err(never) => match never {},
    }
}
pub(super) fn parse_json_checked<E>(
    input: &[u8],
    limits: JsonLimits,
    mut charge: impl FnMut(u64) -> Result<(), E>,
) -> Result<Result<JsonValue, JsonError>, E> {
    if let Err(e) = limits.validate() {
        return Ok(Err(e));
    }
    if input.len() > limits.bytes {
        return Ok(Err(JsonError {
            offset: 0,
            code: "limit",
        }));
    }
    let input = match std::str::from_utf8(input) {
        Ok(input) => input,
        Err(e) => {
            return Ok(Err(JsonError {
                offset: e.valid_up_to(),
                code: "utf8",
            }))
        }
    };
    let mut p = Parser {
        input,
        pos: 0,
        left: limits,
        charge: &mut charge,
        external: None,
    };
    let out = (|| {
        let out = p.value(0)?;
        p.ws()?;
        if p.pos != input.len() {
            return Err(p.err("syntax"));
        }
        Ok(out)
    })();
    if let Some(e) = p.external {
        Err(e)
    } else {
        Ok(out)
    }
}
/// Serialize JSON while preserving numeric kind and insertion order; not canonical signing text.
pub fn serialize_json(value: &JsonValue, limits: JsonLimits) -> Result<String, JsonError> {
    match serialize_json_checked(value, limits, |_| Ok::<(), std::convert::Infallible>(())) {
        Ok(out) => out,
        Err(never) => match never {},
    }
}
struct EmitBudget<'a, E> {
    limits: JsonLimits,
    charge: &'a mut dyn FnMut(u64) -> Result<(), E>,
    external: Option<E>,
}
impl<E> EmitBudget<'_, E> {
    fn work(&mut self, n: u64) -> Result<(), JsonError> {
        if let Err(e) = (self.charge)(n) {
            self.external = Some(e);
            return Err(JsonError {
                offset: 0,
                code: "control",
            });
        }
        self.limits.work = self.limits.work.checked_sub(n).ok_or(JsonError {
            offset: 0,
            code: "limit",
        })?;
        Ok(())
    }
}
pub(super) fn serialize_json_checked<E>(
    value: &JsonValue,
    limits: JsonLimits,
    mut charge: impl FnMut(u64) -> Result<(), E>,
) -> Result<Result<String, JsonError>, E> {
    if let Err(e) = limits.validate() {
        return Ok(Err(e));
    }
    let mut out = String::new();
    let mut left = EmitBudget {
        limits,
        charge: &mut charge,
        external: None,
    };
    let result = emit(value, 0, &mut left, &mut out).map(|_| out);
    if let Some(e) = left.external {
        Err(e)
    } else {
        Ok(result)
    }
}
fn add<E>(out: &mut String, text: &str, left: &mut EmitBudget<'_, E>) -> Result<(), JsonError> {
    left.limits.bytes = left.limits.bytes.checked_sub(text.len()).ok_or(JsonError {
        offset: 0,
        code: "limit",
    })?;
    left.work(text.len() as u64 + 1)?;
    out.push_str(text);
    Ok(())
}
fn quote<E>(out: &mut String, text: &str, left: &mut EmitBudget<'_, E>) -> Result<(), JsonError> {
    left.limits.text = left.limits.text.checked_sub(text.len()).ok_or(JsonError {
        offset: 0,
        code: "limit",
    })?;
    add(out, "\"", left)?;
    for c in text.chars() {
        match c {
            '"' => add(out, "\\\"", left)?,
            '\\' => add(out, "\\\\", left)?,
            '\n' => add(out, "\\n", left)?,
            '\r' => add(out, "\\r", left)?,
            '\t' => add(out, "\\t", left)?,
            c if c < ' ' => {
                let mut escaped = String::new();
                write!(&mut escaped, "\\u{:04x}", c as u32).map_err(|_| JsonError {
                    offset: 0,
                    code: "limit",
                })?;
                add(out, &escaped, left)?;
            }
            _ => {
                let mut bytes = [0; 4];
                add(out, c.encode_utf8(&mut bytes), left)?;
            }
        }
    }
    add(out, "\"", left)
}
fn emit<E>(
    value: &JsonValue,
    depth: usize,
    left: &mut EmitBudget<'_, E>,
    out: &mut String,
) -> Result<(), JsonError> {
    if depth > left.limits.depth {
        return Err(JsonError {
            offset: 0,
            code: "limit",
        });
    }
    left.limits.values = left.limits.values.checked_sub(1).ok_or(JsonError {
        offset: 0,
        code: "limit",
    })?;
    match value {
        JsonValue::Null => add(out, "null", left),
        JsonValue::Bool(b) => add(out, if *b { "true" } else { "false" }, left),
        JsonValue::Int(n) => add(out, &n.to_string(), left),
        JsonValue::Float(n) => {
            if !n.is_finite() {
                return Err(JsonError {
                    offset: 0,
                    code: "number",
                });
            }
            let mut text = n.to_string();
            if !text.contains(['.', 'e', 'E']) {
                text.push_str(".0");
            }
            add(out, &text, left)
        }
        JsonValue::String(s) => quote(out, s, left),
        JsonValue::Array(items) => {
            add(out, "[", left)?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    add(out, ",", left)?;
                }
                emit(item, depth + 1, left, out)?;
            }
            add(out, "]", left)
        }
        JsonValue::Object(items) => {
            add(out, "{", left)?;
            for (i, (key, value)) in items.iter().enumerate() {
                left.limits.values = left.limits.values.checked_sub(1).ok_or(JsonError {
                    offset: 0,
                    code: "limit",
                })?;
                for (previous, _) in &items[..i] {
                    left.work((previous.len() + key.len() + 1) as u64)?;
                    if previous == key {
                        return Err(JsonError {
                            offset: 0,
                            code: "duplicate",
                        });
                    }
                }
                if i > 0 {
                    add(out, ",", left)?;
                }
                quote(out, key, left)?;
                add(out, ":", left)?;
                emit(value, depth + 1, left, out)?;
            }
            add(out, "}", left)
        }
    }
}
