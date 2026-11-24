//! Syntax-independent graph snapshots for versioned compiler/linker consumers.
use super::{inventory, source_units, PackageManifestSnapshot, PackageSourceInventory};
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    resources::{FrontendLimits, DEFAULT_PACKAGE_SOURCE_BYTES},
};
use std::{collections::BTreeSet, path::Path};
/// Reduced whole-graph admission; callers cannot raise existing read ceilings.
#[derive(Debug, Clone, Copy)]
pub struct PackageCaptureLimits {
    /// Selected entry/module files, capped by inventory admission.
    pub max_sources: usize,
    /// Actual UTF-8 bytes retained across every source.
    pub max_source_bytes: usize,
    /// Actual bytes in any one source, capped by frontend admission.
    pub max_file_bytes: usize,
}
impl PackageCaptureLimits {
    /// Existing inventory/frontend/package ceilings; zero never selects a fallback.
    pub const DEFAULT: Self = Self {
        max_sources: super::MAX_INVENTORY_SOURCES,
        max_source_bytes: DEFAULT_PACKAGE_SOURCE_BYTES,
        max_file_bytes: FrontendLimits::DEFAULT.max_source_bytes,
    };
}
/// One original file's bytes, owned by its inventory registration; not parsed syntax.
#[derive(Debug)]
pub struct CapturedPackageSource {
    index: usize,
    source: String,
}
impl CapturedPackageSource {
    pub(crate) fn embedded(index: usize, source: String) -> Self {
        Self { index, source }
    }
    /// Exact registration index in the owning capture's inventory.
    pub fn source_index(&self) -> usize {
        self.index
    }
    /// Original admitted UTF-8 bytes, without concatenation or rewriting.
    pub fn source(&self) -> &str {
        &self.source
    }
}
/// Complete graph/manifests/source snapshots, without checking or linker authority.
#[derive(Debug)]
pub struct CapturedPackageSources {
    inventory: PackageSourceInventory,
    manifests: Vec<PackageManifestSnapshot>,
    sources: Vec<CapturedPackageSource>,
    bytes: usize,
}
impl CapturedPackageSources {
    /// Canonical package identities and selected source registrations.
    pub fn inventory(&self) -> &PackageSourceInventory {
        &self.inventory
    }
    /// Exact manifest snapshots used by graph resolution.
    pub fn manifests(&self) -> &[PackageManifestSnapshot] {
        &self.manifests
    }
    /// Sources in deterministic inventory order, with distinct opened identities.
    pub fn sources(&self) -> &[CapturedPackageSource] {
        &self.sources
    }
    /// Total actual retained source bytes.
    pub fn source_bytes(&self) -> usize {
        self.bytes
    }
    /// Canonical root of this captured dependency graph.
    pub fn root(&self) -> &Path {
        &self.inventory.graph.root
    }
}
/// Capture all selected package entries and registered modules using guarded reads.
/// Does not invoke a parser, check module headers, resolve language imports, compile,
/// execute or publish a lock. This permits generic syntax without the legacy AST
/// parser. Existing graph/path/manifest safeguards remain active. Reads currently
/// require supported Linux/procfs hosts; no weaker fallback is available.
pub fn capture_package_sources(
    root: impl AsRef<Path>,
    limits: PackageCaptureLimits,
) -> Result<CapturedPackageSources, Diagnostics> {
    let (inventory, manifests) = inventory::capture_inventory(root.as_ref())?;
    let span = Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    let failure =
        |path: &Path, message: &str| super::error("E1003", message.into(), Some((path, span)));
    if inventory.sources.len() > limits.max_sources.min(super::MAX_INVENTORY_SOURCES) {
        return Err(failure(
            &inventory.graph.root.join("sovra.toml"),
            "captured package source count allowance exceeded",
        ));
    }
    let ceiling = limits.max_source_bytes.min(DEFAULT_PACKAGE_SOURCE_BYTES);
    let per_file = limits
        .max_file_bytes
        .min(FrontendLimits::DEFAULT.max_source_bytes);
    let mut observed = 0u64;
    for source in &inventory.sources {
        observed = observed.checked_add(source.observed_bytes).ok_or_else(|| {
            failure(
                &source.canonical_path,
                "captured package source byte allowance exceeded",
            )
        })?;
        if observed > ceiling as u64 || source.observed_bytes > per_file as u64 {
            return Err(failure(
                &source.canonical_path,
                "captured package source byte allowance exceeded",
            ));
        }
    }
    let (mut sources, mut identities, mut bytes) = (Vec::new(), BTreeSet::new(), 0usize);
    for (index, registration) in inventory.sources.iter().enumerate() {
        let remaining = ceiling.checked_sub(bytes).ok_or_else(|| {
            failure(
                &registration.canonical_path,
                "captured package source byte allowance exceeded",
            )
        })?;
        let (source, identity) =
            source_units::read_snapshot(registration, remaining.min(per_file))?;
        if !identities.insert(identity) {
            return Err(super::error(
                "E4161",
                "opened source identity is already captured".into(),
                Some((&registration.canonical_path, span)),
            ));
        }
        bytes = bytes
            .checked_add(source.len())
            .filter(|n| *n <= ceiling)
            .ok_or_else(|| {
                failure(
                    &registration.canonical_path,
                    "captured package source byte allowance exceeded",
                )
            })?;
        sources.push(CapturedPackageSource { index, source });
    }
    Ok(CapturedPackageSources {
        inventory,
        manifests,
        sources,
        bytes,
    })
}
