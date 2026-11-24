//! Guarded, immutable single-entry capture for explicitly selected compiler pipelines.
use super::{
    inventory, source_units, PackageManifestSnapshot, PackageSourceInventory, PackageSourceKind,
};
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    resources::FrontendLimits,
};
use std::path::{Path, PathBuf};
/// Captured root-only package input, not a parsed/checked/locked package.
#[derive(Debug)]
pub struct CapturedRootSource {
    inventory: PackageSourceInventory,
    manifests: Vec<PackageManifestSnapshot>,
    source: String,
}
impl CapturedRootSource {
    /// Canonical package/source selection; source text below is the captured content.
    pub fn inventory(&self) -> &PackageSourceInventory {
        &self.inventory
    }
    /// Exact manifest snapshots used by resolution; not a whole-filesystem transaction.
    pub fn manifests(&self) -> &[PackageManifestSnapshot] {
        &self.manifests
    }
    /// Canonical owning package root.
    pub fn root(&self) -> &Path {
        &self.inventory.graph.root
    }
    /// Canonical entry locator; never reread by this capture.
    pub fn entry(&self) -> &Path {
        &self.inventory.sources[0].canonical_path
    }
    /// Original admitted UTF-8 entry bytes, never concatenated or rewritten.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Transfer captured bytes and canonical entry location without another read.
    pub fn into_source(self) -> (PathBuf, String) {
        (
            self.inventory.sources[0].canonical_path.clone(),
            self.source,
        )
    }
}
/// Resolve/capture existing manifests and exactly one root entry with guarded reads.
/// Dependencies and registered modules refuse before source capture, not silently
/// ignored. Existing graph/manifest/path/source quotas remain in force. This is
/// not source parsing, lock verification or an atomic filesystem snapshot. The
/// source reader requires supported Linux/procfs hosts and refuses elsewhere.
pub fn capture_root_source(root: impl AsRef<Path>) -> Result<CapturedRootSource, Diagnostics> {
    capture_root_source_with_source_limit(root, FrontendLimits::DEFAULT.max_source_bytes)
}
/// Capture the same guarded root entry under a caller-selected byte ceiling.
/// The ceiling may only reduce existing frontend admission; no weaker read path
/// or dependency/module fallback is selected, including when the ceiling is zero.
pub fn capture_root_source_with_source_limit(
    root: impl AsRef<Path>,
    max_source_bytes: usize,
) -> Result<CapturedRootSource, Diagnostics> {
    let (inventory, manifests) = inventory::capture_inventory(root.as_ref())?;
    let package = inventory
        .graph
        .packages
        .get(&inventory.graph.root)
        .expect("resolved root");
    if !package.dependencies.is_empty()
        || inventory.graph.packages.len() != 1
        || inventory.sources.len() != 1
        || inventory.sources[0].kind != PackageSourceKind::Entry
    {
        return Err(super::error("E3124","experimental package compilation requires one root entry and no dependencies or registered modules".into(),
            Some((&inventory.graph.root.join("sovra.toml"),Span{start:0,end:0,line:0,column:0}))));
    }
    let (source, _) = source_units::read_snapshot(
        &inventory.sources[0],
        max_source_bytes.min(FrontendLimits::DEFAULT.max_source_bytes),
    )?;
    Ok(CapturedRootSource {
        inventory,
        manifests,
        source,
    })
}
