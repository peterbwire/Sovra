//! Borrowed, bounded unsaved entry sources for an already resolved disk graph.
use super::{error, LocalGraph};
use crate::compiler::diagnostics::Diagnostics;
use std::{collections::BTreeMap, path::PathBuf};

/// One unsaved UTF-8 source buffer. Only existing canonical graph entries qualify;
/// this does not overlay manifests or create files on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOverlay {
    /// Existing source path, canonicalized once against the resolved graph.
    pub entry: PathBuf,
    /// Caller-owned buffer, borrowed throughout compilation without copying it.
    pub source: String,
}
/// Explicit admission limits for borrowed overlay buffers, not total memory/CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceOverlayLimits {
    /// Maximum supplied buffers, before canonicalization or table growth.
    pub max_files: usize,
    /// Maximum UTF-8 bytes per supplied buffer.
    pub max_source_bytes: usize,
    /// Maximum UTF-8 bytes across all supplied buffers.
    pub max_total_source_bytes: usize,
}
impl SourceOverlayLimits {
    /// Initial workspace profile, matching current LSP open-buffer allowances.
    pub const DEFAULT: Self = Self {
        max_files: 32,
        max_source_bytes: 512 * 1024,
        max_total_source_bytes: 4 * 1024 * 1024,
    };
}
pub(super) fn admit<'a>(
    graph: &LocalGraph,
    overlays: &'a [SourceOverlay],
    limits: SourceOverlayLimits,
) -> Result<BTreeMap<PathBuf, &'a str>, Diagnostics> {
    if overlays.len() > limits.max_files {
        return Err(error(
            "E4118",
            "unsaved source buffer count allowance exceeded".to_owned(),
            None,
        ));
    }
    let mut remaining = limits.max_total_source_bytes;
    // Validate payload before filesystem work or retained path-map allocation.
    for overlay in overlays {
        if overlay.source.len() > limits.max_source_bytes || overlay.source.len() > remaining {
            return Err(error(
                "E4118",
                "unsaved source byte allowance exceeded".to_owned(),
                None,
            ));
        }
        remaining -= overlay.source.len();
    }
    let mut result = BTreeMap::new();
    for overlay in overlays {
        let path = overlay.entry.canonicalize().map_err(|_| {
            error(
                "E4117",
                "unsaved source target is not an existing graph entry".to_owned(),
                None,
            )
        })?;
        if !graph.packages.values().any(|package| package.entry == path)
            || result.contains_key(&path)
        {
            return Err(error(
                "E4117",
                "unsaved source target is unrelated or duplicated".to_owned(),
                None,
            ));
        }
        result.insert(path, overlay.source.as_str());
    }
    Ok(result)
}
