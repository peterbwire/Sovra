//! Bounded JSON for the approved lock schema; no permissive unknown-data fallback.
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Value {
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    Number(u64),
    Null,
    Bool(bool),
}
const MAX_STRING: usize = 16 * 1024;
const MAX_NODES: usize = 262_144;
const MAX_DEPTH: usize = 16;
pub(super) fn parse(source: &str) -> Result<Value, String> {
    if source.len() > super::MAX_LOCK_BYTES {
        return Err("lock exceeds 8 MiB".into());
    }
    let mut parser = Parser {
        source,
        pos: 0,
        nodes: 0,
        strings: 0,
    };
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.pos != source.len() {
        return Err("trailing lock JSON data".into());
    }
    Ok(value)
}
struct Parser<'a> {
    source: &'a str,
    pos: usize,
    nodes: usize,
    strings: usize,
}
impl Parser<'_> {
    fn whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.pos)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.pos += 1;
        }
    }
    fn byte(&mut self, expected: u8) -> Result<(), String> {
        self.whitespace();
        if self.source.as_bytes().get(self.pos) != Some(&expected) {
            return Err("malformed lock JSON".into());
        }
        self.pos += 1;
        Ok(())
    }
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH || self.nodes == MAX_NODES {
            return Err("lock JSON structural limit exceeded".into());
        }
        self.nodes += 1;
        self.whitespace();
        match self.source.as_bytes().get(self.pos).copied() {
            Some(b'{') => {
                self.pos += 1;
                let mut values = BTreeMap::new();
                self.whitespace();
                if self.source.as_bytes().get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    return Ok(Value::Object(values));
                }
                loop {
                    let key = self.string()?;
                    self.byte(b':')?;
                    let value = self.value(depth + 1)?;
                    if values.insert(key, value).is_some() {
                        return Err("duplicate lock JSON key".into());
                    }
                    self.whitespace();
                    if self.source.as_bytes().get(self.pos) == Some(&b'}') {
                        self.pos += 1;
                        break;
                    }
                    self.byte(b',')?;
                }
                Ok(Value::Object(values))
            }
            Some(b'[') => {
                self.pos += 1;
                let mut values = Vec::new();
                self.whitespace();
                if self.source.as_bytes().get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    return Ok(Value::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.whitespace();
                    if self.source.as_bytes().get(self.pos) == Some(&b']') {
                        self.pos += 1;
                        break;
                    }
                    self.byte(b',')?;
                }
                Ok(Value::Array(values))
            }
            Some(b'"') => self.string().map(Value::String),
            Some(b'0'..=b'9') => {
                let start = self.pos;
                while self
                    .source
                    .as_bytes()
                    .get(self.pos)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.pos += 1;
                }
                let raw = &self.source[start..self.pos];
                if raw.len() > 1 && raw.starts_with('0') {
                    return Err("invalid lock number".into());
                }
                raw.parse::<u64>()
                    .map(Value::Number)
                    .map_err(|_| "lock number overflow".into())
            }
            _ => {
                for (token, value) in [
                    ("null", Value::Null),
                    ("true", Value::Bool(true)),
                    ("false", Value::Bool(false)),
                ] {
                    if self.source[self.pos..].starts_with(token) {
                        self.pos += token.len();
                        return Ok(value);
                    }
                }
                Err("invalid lock JSON value".into())
            }
        }
    }
    fn append(&mut self, text: &mut String, ch: char) -> Result<(), String> {
        let bytes = ch.len_utf8();
        if text.len() + bytes > MAX_STRING || self.strings + bytes > super::MAX_LOCK_BYTES {
            return Err("lock decoded string limit exceeded".into());
        }
        self.strings += bytes;
        text.push(ch);
        Ok(())
    }
    fn hex(&mut self) -> Result<u32, String> {
        let mut value = 0;
        for _ in 0..4 {
            let byte = *self
                .source
                .as_bytes()
                .get(self.pos)
                .ok_or("truncated lock Unicode escape")?;
            self.pos += 1;
            let digit = (byte as char)
                .to_digit(16)
                .ok_or("invalid lock Unicode escape")?;
            value = value * 16 + digit;
        }
        Ok(value)
    }
    fn string(&mut self) -> Result<String, String> {
        self.byte(b'"')?;
        let mut text = String::new();
        loop {
            let byte = *self
                .source
                .as_bytes()
                .get(self.pos)
                .ok_or("unterminated lock string")?;
            if byte == b'"' {
                self.pos += 1;
                return Ok(text);
            }
            let ch = if byte == b'\\' {
                self.pos += 1;
                let escaped = *self
                    .source
                    .as_bytes()
                    .get(self.pos)
                    .ok_or("truncated lock escape")?;
                self.pos += 1;
                match escaped {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'b' => '\u{0008}',
                    b'f' => '\u{000c}',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'u' => {
                        let mut code = self.hex()?;
                        if (0xd800..=0xdbff).contains(&code) {
                            if !self.source[self.pos..].starts_with("\\u") {
                                return Err("missing low surrogate".into());
                            }
                            self.pos += 2;
                            let low = self.hex()?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return Err("invalid low surrogate".into());
                            }
                            code = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                        }
                        char::from_u32(code).ok_or("invalid lock Unicode scalar")?
                    }
                    _ => return Err("invalid lock escape".into()),
                }
            } else {
                if byte < 0x20 {
                    return Err("control character in lock string".into());
                }
                let ch = self.source[self.pos..]
                    .chars()
                    .next()
                    .ok_or("unterminated lock string")?;
                self.pos += ch.len_utf8();
                ch
            };
            self.append(&mut text, ch)?;
        }
    }
}

pub(super) fn canonical(value: &Value) -> Result<String, String> {
    let mut output = String::new();
    write(value, &mut output, 0)?;
    append(&mut output, "\n")?;
    Ok(output)
}
fn append(output: &mut String, text: &str) -> Result<(), String> {
    if output
        .len()
        .checked_add(text.len())
        .map_or(true, |size| size > super::MAX_LOCK_BYTES)
    {
        return Err("lock output exceeds 8 MiB".into());
    }
    output.push_str(text);
    Ok(())
}
fn quoted(output: &mut String, text: &str) -> Result<(), String> {
    if text.len() > MAX_STRING {
        return Err("lock string exceeds 16 KiB".into());
    }
    let mut escaped = String::new();
    crate::compiler::check_report::push_string(&mut escaped, text);
    append(output, &escaped)
}
fn write(value: &Value, output: &mut String, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("lock JSON depth exceeded".into());
    }
    match value {
        Value::Object(fields) => {
            append(output, "{")?;
            for (index, (name, value)) in fields.iter().enumerate() {
                if index != 0 {
                    append(output, ",")?;
                }
                quoted(output, name)?;
                append(output, ":")?;
                write(value, output, depth + 1)?;
            }
            append(output, "}")
        }
        Value::Array(values) => {
            append(output, "[")?;
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    append(output, ",")?;
                }
                write(value, output, depth + 1)?;
            }
            append(output, "]")
        }
        Value::String(text) => quoted(output, text),
        Value::Number(value) => append(output, &value.to_string()),
        Value::Null => append(output, "null"),
        Value::Bool(value) => append(output, if *value { "true" } else { "false" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_json_unicode_and_duplicate_keys_never_accept_ambiguous_data() {
        assert_eq!(
            parse("\"\\ud83d\\ude00\"").unwrap(),
            Value::String("😀".into())
        );
        for source in [
            "{\"x\":1,\"\\u0078\":2}",
            "[null,]",
            "{\"x\":1,}",
            "\"\\ud800\"",
            "\"\\udc00\"",
            "1.0",
            "01",
            "18446744073709551616",
            "null true",
            "/*x*/null",
            "\"raw\nline\"",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }
    #[test]
    fn encoded_string_depth_and_node_boundaries_are_exact() {
        let exact = format!("\"{}\"", "x".repeat(MAX_STRING));
        assert!(parse(&exact).is_ok());
        assert!(parse(&format!("\"{}\"", "x".repeat(MAX_STRING + 1))).is_err());
        assert!(parse(&format!(
            "{}null{}",
            "[".repeat(MAX_DEPTH),
            "]".repeat(MAX_DEPTH)
        ))
        .is_ok());
        assert!(parse(&format!(
            "{}null{}",
            "[".repeat(MAX_DEPTH + 1),
            "]".repeat(MAX_DEPTH + 1)
        ))
        .is_err());
        let exact = format!(
            "[{}]",
            std::iter::repeat("null")
                .take(MAX_NODES - 1)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(parse(&exact).is_ok());
        assert!(parse(&exact.replacen('[', "[null,", 1)).is_err());
        let exact = format!("null{}", " ".repeat(super::super::MAX_LOCK_BYTES - 4));
        assert!(parse(&exact).is_ok());
        assert!(parse(&(exact + " ")).is_err());
    }
}
