//! Version-one, bounded stdin transport for unsaved package source checking.
use super::{error, SourceOverlay, SourceOverlayLimits};
use crate::compiler::diagnostics::Diagnostics;
use std::{io::Read, path::PathBuf};

/// Request magic/version, followed by big-endian u32 request ID and buffer count.
pub const SOURCE_OVERLAY_MAGIC: &[u8; 8] = b"SVROVL1\n";
/// Maximum UTF-8 bytes of one transported absolute path.
pub const MAX_OVERLAY_PATH_BYTES: usize = 4096;
/// Version-one wire admission, independent of future library overlay defaults.
pub const SOURCE_OVERLAY_REQUEST_LIMITS: SourceOverlayLimits = SourceOverlayLimits {
    max_files: 32,
    max_source_bytes: 512 * 1024,
    max_total_source_bytes: 4 * 1024 * 1024,
};
/// Maximum encoded request bytes, including headers and path/source payload.
pub const MAX_OVERLAY_REQUEST_BYTES: usize = 16
    + SOURCE_OVERLAY_REQUEST_LIMITS.max_files * (8 + MAX_OVERLAY_PATH_BYTES)
    + SOURCE_OVERLAY_REQUEST_LIMITS.max_total_source_bytes;
/// Fully validated framed request. Graph membership/aliases are checked later.
#[derive(Debug)]
pub struct SourceOverlayRequest {
    /// Caller correlation value; not authorization or proof of source freshness.
    pub request_id: u32,
    /// Admitted UTF-8 buffers. Compilation borrows these owned strings.
    pub sources: Vec<SourceOverlay>,
}
fn invalid() -> Diagnostics {
    error(
        "E4119",
        "invalid or oversized source-overlay request".to_owned(),
        None,
    )
}

/// Read exactly one request through EOF with finite storage. Each buffer has
/// big-endian u32 path/source lengths, then those exact UTF-8 bytes. No trailing
/// data, relative paths or NUL path characters are accepted. The caller must close
/// stdin and enforce its own process deadline; this does not impose an I/O timeout.
pub fn read_source_overlay_request(reader: impl Read) -> Result<SourceOverlayRequest, Diagnostics> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_OVERLAY_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > MAX_OVERLAY_REQUEST_BYTES || !bytes.starts_with(SOURCE_OVERLAY_MAGIC) {
        return Err(invalid());
    }
    let mut at = SOURCE_OVERLAY_MAGIC.len();
    fn word(bytes: &[u8], at: &mut usize) -> Result<usize, Diagnostics> {
        let end = at.checked_add(4).ok_or_else(invalid)?;
        let value = bytes.get(*at..end).ok_or_else(invalid)?;
        *at = end;
        Ok(u32::from_be_bytes(value.try_into().map_err(|_| invalid())?) as usize)
    }
    fn text<'a>(bytes: &'a [u8], at: &mut usize, len: usize) -> Result<&'a str, Diagnostics> {
        let end = at.checked_add(len).ok_or_else(invalid)?;
        let value =
            std::str::from_utf8(bytes.get(*at..end).ok_or_else(invalid)?).map_err(|_| invalid())?;
        *at = end;
        Ok(value)
    }
    let request_id = word(&bytes, &mut at)? as u32;
    let count = word(&bytes, &mut at)?;
    let limits = SOURCE_OVERLAY_REQUEST_LIMITS;
    if count > limits.max_files {
        return Err(invalid());
    }
    let mut sources = Vec::new();
    let mut remaining = limits.max_total_source_bytes;
    for _ in 0..count {
        let path_len = word(&bytes, &mut at)?;
        let source_len = word(&bytes, &mut at)?;
        if path_len == 0
            || path_len > MAX_OVERLAY_PATH_BYTES
            || source_len > limits.max_source_bytes
            || source_len > remaining
        {
            return Err(invalid());
        }
        let path = text(&bytes, &mut at, path_len)?;
        if path.contains('\0') || !std::path::Path::new(path).is_absolute() {
            return Err(invalid());
        }
        let entry = PathBuf::from(path);
        let source = text(&bytes, &mut at, source_len)?.to_owned();
        remaining -= source_len;
        sources.push(SourceOverlay { entry, source });
    }
    if at != bytes.len() {
        return Err(invalid());
    }
    Ok(SourceOverlayRequest {
        request_id,
        sources,
    })
}
