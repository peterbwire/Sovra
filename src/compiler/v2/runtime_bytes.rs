//! Checked byte values and immutable strict-codec buffers; explicit Rust API only.
use super::{RuntimeValueError, RuntimeValueFuel};
use crate::compiler::execution::{Budget, ExecutionControl, RuntimePolicy};

/// Structured data refusal. Codec offsets refer to original zero-based input bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDataError {
    /// Stable refusal category.
    pub kind: RuntimeDataErrorKind,
    /// Original offending byte, buffer boundary, or unexpected-end position.
    pub offset: Option<usize>,
}
/// Data errors do not contain a partial successful buffer or substituted text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeDataErrorKind {
    /// Int is outside 0..255.
    ByteRange,
    /// Index or half-open slice is invalid.
    Bounds,
    /// Nonhexadecimal input or unmatched trailing nibble.
    Hex,
    /// Invalid standard alphabet, length, padding or nonzero discarded bits.
    Base64,
    /// Invalid UTF-8; no replacement characters are inserted.
    Utf8,
    /// Invalid caller caps, result length or checked arithmetic overflow.
    Capacity,
    /// Shared finite helper work exhausted.
    WorkLimit,
    /// Atomic result node/text admission refused.
    Materialization(RuntimeValueError),
    /// Cooperative cancellation or elapsed-time control refused.
    Control(String),
}
impl std::fmt::Display for RuntimeDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "runtime data refused: {:?} at {:?}",
            self.kind, self.offset
        )
    }
}
impl std::error::Error for RuntimeDataError {}
fn error(kind: RuntimeDataErrorKind, offset: Option<usize>) -> RuntimeDataError {
    RuntimeDataError { kind, offset }
}
fn materialization(e: RuntimeValueError) -> RuntimeDataError {
    error(RuntimeDataErrorKind::Materialization(e), None)
}
fn capacity() -> RuntimeDataError {
    error(RuntimeDataErrorKind::Capacity, None)
}
fn scalar_admit(fuel: &mut RuntimeValueFuel) -> Result<(), RuntimeDataError> {
    fuel.checkpoint()
        .map_err(|e| error(RuntimeDataErrorKind::Control(e), None))?;
    fuel.admit(1, 0).map_err(materialization)
}
/// Finite byte/text bounds. Callers may select other finite u32-sized ceilings.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeBytesLimits {
    /// Maximum binary buffer length.
    pub max_bytes: usize,
    /// Maximum codec input/output UTF-8 byte length.
    pub max_text_bytes: usize,
}
impl RuntimeBytesLimits {
    /// Initial opt-in binary/text profile; existing VM/CLI defaults are unchanged.
    pub const DEFAULT: Self = Self {
        max_bytes: 16 * 1024 * 1024,
        max_text_bytes: 64 * 1024 * 1024,
    };
    fn validate(self) -> Result<(), RuntimeDataError> {
        if self.max_bytes as u128 > u32::MAX as u128
            || self.max_text_bytes as u128 > u32::MAX as u128
        {
            return Err(capacity());
        }
        Ok(())
    }
    fn binary(self, n: usize) -> Result<(), RuntimeDataError> {
        self.validate()?;
        if n > self.max_bytes || n > isize::MAX as usize {
            return Err(capacity());
        }
        Ok(())
    }
    fn text(self, n: usize) -> Result<(), RuntimeDataError> {
        self.validate()?;
        if n > self.max_text_bytes || n > isize::MAX as usize {
            return Err(capacity());
        }
        Ok(())
    }
}
/// Shared finite work and optional cooperative controls for data helpers.
pub struct RuntimeDataWork {
    remaining: u64,
    control: Budget,
}
impl std::fmt::Debug for RuntimeDataWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeDataWork")
            .field("remaining", &self.remaining)
            .finish()
    }
}
impl RuntimeDataWork {
    /// Create a finite allowance without additional cooperative controls.
    pub fn new(units: u64) -> Self {
        Self::with_control(units, ExecutionControl::UNLIMITED)
    }
    /// Select optional cancellation/elapsed controls without changing host authority.
    pub fn with_control(units: u64, control: ExecutionControl) -> Self {
        Self {
            remaining: units,
            control: Budget::with_control(RuntimePolicy::UNLIMITED, control),
        }
    }
    /// Unspent shared work; failures never refund completed work.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    fn charge(&mut self, n: u64) -> Result<(), RuntimeDataError> {
        self.control
            .checkpoint()
            .map_err(|e| error(RuntimeDataErrorKind::Control(e), None))?;
        self.remaining = self
            .remaining
            .checked_sub(n)
            .ok_or_else(|| error(RuntimeDataErrorKind::WorkLimit, None))?;
        Ok(())
    }
}
/// Distinct checked byte scalar. Construction/copy/conversion admit owned nodes.
#[derive(Debug, PartialEq, Eq)]
pub struct RuntimeByteV2(u8);
impl RuntimeByteV2 {
    /// Checked Int conversion; range failure consumes no materialization fuel.
    pub fn from_int(value: i64, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeDataError> {
        let value =
            u8::try_from(value).map_err(|_| error(RuntimeDataErrorKind::ByteRange, None))?;
        scalar_admit(fuel)?;
        Ok(Self(value))
    }
    /// Borrowed Rust scalar inspection, not a materialized language Int.
    pub fn value(&self) -> u8 {
        self.0
    }
    /// Independent owned Byte copy.
    pub fn snapshot(&self, fuel: &mut RuntimeValueFuel) -> Result<Self, RuntimeDataError> {
        scalar_admit(fuel)?;
        Ok(Self(self.0))
    }
    /// Explicit owned Int conversion.
    pub fn to_int(&self, fuel: &mut RuntimeValueFuel) -> Result<i64, RuntimeDataError> {
        scalar_admit(fuel)?;
        Ok(i64::from(self.0))
    }
}
/// Packed physical bytes with one logical Byte per element and one buffer node.
/// No unchecked mutation, public field, Clone or shared-storage snapshot API.
#[derive(Debug)]
pub struct RuntimeBytesV2 {
    data: Vec<u8>,
}
fn admit(
    n: usize,
    text: bool,
    fuel: &mut RuntimeValueFuel,
    work: &mut RuntimeDataWork,
) -> Result<(), RuntimeDataError> {
    let (nodes, bytes) = if text {
        (1, n as u64)
    } else {
        ((n as u64).checked_add(1).ok_or_else(capacity)?, 0)
    };
    fuel.checkpoint()
        .map_err(|e| error(RuntimeDataErrorKind::Control(e), None))?;
    fuel.preflight(nodes, bytes).map_err(materialization)?;
    work.charge(
        (n as u64)
            .checked_mul(2)
            .and_then(|v| v.checked_add(1))
            .ok_or_else(capacity)?,
    )?;
    fuel.admit(nodes, bytes).map_err(materialization)
}
fn copied(data: &[u8], work: &mut RuntimeDataWork) -> Result<Vec<u8>, RuntimeDataError> {
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks(65_536) {
        work.charge(0)?;
        out.extend_from_slice(chunk);
    }
    work.charge(0)?;
    Ok(out)
}
pub(super) fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
pub(super) const BASE64: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub(super) fn sextet(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
impl RuntimeBytesV2 {
    /// Copy raw Rust bytes under complete binary node admission.
    pub fn from_slice(
        data: &[u8],
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        limits.binary(data.len())?;
        admit(data.len(), false, fuel, work)?;
        Ok(Self {
            data: copied(data, work)?,
        })
    }
    /// Copy already-valid String contents into binary storage; binary owns no text units.
    pub fn from_text(
        text: &str,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        limits.text(text.len())?;
        Self::from_slice(text.as_bytes(), limits, fuel, work)
    }
    /// Copy a typed Byte array; output buffer/descendants are admitted once.
    pub fn from_array(
        data: &[RuntimeByteV2],
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        limits.binary(data.len())?;
        admit(data.len(), false, fuel, work)?;
        let mut out = Vec::with_capacity(data.len());
        for value in data {
            work.charge(0)?;
            out.push(value.0);
        }
        Ok(Self { data: out })
    }
    /// Borrowed read-only Rust byte view; does not materialize a language array.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    /// Borrowed length metadata.
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /// Borrowed emptiness metadata.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    /// Logical full-result node/text footprint.
    pub fn footprint(&self) -> (u64, u64) {
        (self.data.len() as u64 + 1, 0)
    }
    /// Complete independent buffer snapshot.
    pub fn snapshot(
        &self,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        Self::from_slice(&self.data, limits, fuel, work)
    }
    /// Owned Byte at an index; an invalid boundary returns an error without a default Byte.
    pub fn get(
        &self,
        index: usize,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<RuntimeByteV2, RuntimeDataError> {
        work.charge(1)?;
        let value = *self
            .data
            .get(index)
            .ok_or_else(|| error(RuntimeDataErrorKind::Bounds, Some(index)))?;
        scalar_admit(fuel)?;
        Ok(RuntimeByteV2(value))
    }
    /// Strict half-open slice with complete independent result admission.
    pub fn slice(
        &self,
        start: usize,
        end: usize,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        limits.validate()?;
        if start > end {
            return Err(error(RuntimeDataErrorKind::Bounds, Some(start)));
        }
        if end > self.len() {
            return Err(error(RuntimeDataErrorKind::Bounds, Some(end)));
        }
        Self::from_slice(&self.data[start..end], limits, fuel, work)
    }
    /// Concatenate snapshots without mutation or release refunds.
    pub fn concat(
        &self,
        other: &Self,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        let n = self.len().checked_add(other.len()).ok_or_else(capacity)?;
        limits.binary(n)?;
        admit(n, false, fuel, work)?;
        let mut data = Vec::with_capacity(n);
        for source in [&self.data, &other.data] {
            for chunk in source.chunks(65_536) {
                work.charge(0)?;
                data.extend_from_slice(chunk);
            }
        }
        work.charge(0)?;
        Ok(Self { data })
    }
    /// Materialize an independent Byte array with one container and all Byte nodes.
    pub fn to_array(
        &self,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Vec<RuntimeByteV2>, RuntimeDataError> {
        admit(self.len(), false, fuel, work)?;
        let mut out = Vec::with_capacity(self.len());
        for value in &self.data {
            work.charge(0)?;
            out.push(RuntimeByteV2(*value));
        }
        Ok(out)
    }
    /// Borrowed logical equality; the returned Rust bool is not an owned language result.
    pub fn equal(
        &self,
        other: &Self,
        work: &mut RuntimeDataWork,
    ) -> Result<bool, RuntimeDataError> {
        work.charge(
            (self.len() as u64)
                .checked_add(other.len() as u64)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(capacity)?,
        )?;
        Ok(self.data == other.data)
    }
    /// Lowercase hex encoding. Output admits one String and all output text bytes.
    pub fn to_hex(
        &self,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<String, RuntimeDataError> {
        limits.binary(self.len())?;
        let n = self.len().checked_mul(2).ok_or_else(capacity)?;
        limits.text(n)?;
        admit(n, true, fuel, work)?;
        let mut out = String::with_capacity(n);
        for byte in &self.data {
            work.charge(0)?;
            out.push(b"0123456789abcdef"[(byte >> 4) as usize] as char);
            out.push(b"0123456789abcdef"[(byte & 15) as usize] as char);
        }
        Ok(out)
    }
    /// Strict hex decoding; malformed input consumes work but no result fuel.
    pub fn from_hex(
        text: &str,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        limits.text(text.len())?;
        let n = text.len() / 2;
        limits.binary(n)?;
        for (i, c) in text.bytes().enumerate() {
            work.charge(1)?;
            if hex(c).is_none() {
                return Err(error(RuntimeDataErrorKind::Hex, Some(i)));
            }
        }
        if text.len() % 2 != 0 {
            return Err(error(RuntimeDataErrorKind::Hex, Some(text.len() - 1)));
        }
        admit(n, false, fuel, work)?;
        let mut data = Vec::with_capacity(n);
        for (i, pair) in text.as_bytes().chunks_exact(2).enumerate() {
            work.charge(0)?;
            let a = hex(pair[0]).ok_or_else(|| error(RuntimeDataErrorKind::Hex, Some(i * 2)))?;
            let b =
                hex(pair[1]).ok_or_else(|| error(RuntimeDataErrorKind::Hex, Some(i * 2 + 1)))?;
            data.push((a << 4) | b);
        }
        Ok(Self { data })
    }
    /// Standard padded base64 encoding without whitespace.
    pub fn to_base64(
        &self,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<String, RuntimeDataError> {
        limits.binary(self.len())?;
        let n = self
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(capacity)?;
        limits.text(n)?;
        admit(n, true, fuel, work)?;
        let mut out = String::with_capacity(n);
        for chunk in self.data.chunks(3) {
            work.charge(0)?;
            let a = chunk[0];
            let b = chunk.get(1).copied().unwrap_or(0);
            let c = chunk.get(2).copied().unwrap_or(0);
            out.push(BASE64[(a >> 2) as usize] as char);
            out.push(BASE64[(((a & 3) << 4) | (b >> 4)) as usize] as char);
            out.push(if chunk.len() > 1 {
                BASE64[(((b & 15) << 2) | (c >> 6)) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                BASE64[(c & 63) as usize] as char
            } else {
                '='
            });
        }
        Ok(out)
    }
    /// Strict standard base64: no whitespace, missing padding or nonzero discarded bits.
    pub fn from_base64(
        text: &str,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<Self, RuntimeDataError> {
        limits.text(text.len())?;
        let n = validate_base64(text, || work.charge(1))??;
        limits.binary(n)?;
        admit(n, false, fuel, work)?;
        let mut data = Vec::with_capacity(n);
        for (group, chunk) in text.as_bytes().chunks_exact(4).enumerate() {
            work.charge(0)?;
            let mut bits = 0u32;
            for (j, c) in chunk.iter().enumerate() {
                let v = if *c == b'=' {
                    0
                } else {
                    sextet(*c)
                        .ok_or_else(|| error(RuntimeDataErrorKind::Base64, Some(group * 4 + j)))?
                };
                bits = (bits << 6) | u32::from(v);
            }
            for shift in [16, 8, 0] {
                if data.len() < n {
                    data.push((bits >> shift) as u8);
                }
            }
        }
        Ok(Self { data })
    }
    /// Strict UTF-8 decoding with original invalid-byte offset and no replacement.
    pub fn to_utf8(
        &self,
        limits: RuntimeBytesLimits,
        fuel: &mut RuntimeValueFuel,
        work: &mut RuntimeDataWork,
    ) -> Result<String, RuntimeDataError> {
        limits.binary(self.len())?;
        limits.text(self.len())?;
        work.charge(self.len() as u64)?;
        std::str::from_utf8(&self.data)
            .map_err(|e| error(RuntimeDataErrorKind::Utf8, Some(e.valid_up_to())))?;
        admit(self.len(), true, fuel, work)?;
        let data = copied(&self.data, work)?;
        String::from_utf8(data).map_err(|e| {
            error(
                RuntimeDataErrorKind::Utf8,
                Some(e.utf8_error().valid_up_to()),
            )
        })
    }
}

// Outer errors preserve the caller's control/work refusal. Inner errors are format/capacity facts.
pub(super) fn validate_base64<E>(
    text: &str,
    mut step: impl FnMut() -> Result<(), E>,
) -> Result<Result<usize, RuntimeDataError>, E> {
    let mut padding = None;
    for (i, c) in text.bytes().enumerate() {
        step()?;
        if c == b'=' {
            if padding.is_none() {
                padding = Some(i);
            }
        } else if sextet(c).is_none() || padding.is_some() {
            return Ok(Err(error(RuntimeDataErrorKind::Base64, Some(i))));
        }
    }
    if text.len() % 4 != 0 {
        return Ok(Err(error(RuntimeDataErrorKind::Base64, Some(text.len()))));
    }
    let pad = padding.map_or(0, |i| text.len() - i);
    if pad > 2 {
        return Ok(Err(error(RuntimeDataErrorKind::Base64, padding)));
    }
    if pad > 0 {
        let index = text.len() - pad - 1;
        let Some(v) = sextet(text.as_bytes()[index]) else {
            return Ok(Err(error(RuntimeDataErrorKind::Base64, Some(index))));
        };
        if (pad == 2 && v & 15 != 0) || (pad == 1 && v & 3 != 0) {
            return Ok(Err(error(RuntimeDataErrorKind::Base64, Some(index))));
        }
    }
    Ok((text.len() / 4)
        .checked_mul(3)
        .and_then(|n| n.checked_sub(pad))
        .ok_or_else(capacity))
}
