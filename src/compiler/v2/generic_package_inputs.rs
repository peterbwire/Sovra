//! Separate source metadata from authority to read original filesystem inputs.
use crate::compiler::project::packages::{
    CapturedPackageSource, CapturedPackageSources, PackageManifestSnapshot, PackageSourceInventory,
};
/// Immutable package sources: guarded captures, validated images, or check-only overlays.
#[derive(Debug)]
pub struct GenericPackageSources(Origin);
#[derive(Debug)]
enum Origin {
    Captured(CapturedPackageSources),
    Embedded(EmbeddedPackageSources),
    Overlay(EmbeddedPackageSources),
}
#[derive(Debug)]
pub(super) struct EmbeddedPackageSources {
    pub inventory: PackageSourceInventory,
    pub sources: Vec<CapturedPackageSource>,
    pub bytes: usize,
}
impl From<CapturedPackageSources> for GenericPackageSources {
    fn from(value: CapturedPackageSources) -> Self {
        Self(Origin::Captured(value))
    }
}
impl GenericPackageSources {
    pub(super) fn embedded(value: EmbeddedPackageSources) -> Self {
        Self(Origin::Embedded(value))
    }
    pub(super) fn overlay(value: EmbeddedPackageSources) -> Self {
        Self(Origin::Overlay(value))
    }
    /// True for image-owned sources with logical locations and no filesystem authority.
    pub fn is_embedded(&self) -> bool {
        matches!(self.0, Origin::Embedded(_))
    }
    /// Guarded filesystem inputs only when this snapshot actually came from capture.
    /// Images and unsaved overlays never yield a capture accepted by lock readers.
    pub fn captured(&self) -> Option<&CapturedPackageSources> {
        match &self.0 {
            Origin::Captured(value) => Some(value),
            Origin::Embedded(_) | Origin::Overlay(_) => None,
        }
    }
    /// Package/source ownership. Embedded roots and paths are logical identifiers.
    pub fn inventory(&self) -> &PackageSourceInventory {
        match &self.0 {
            Origin::Captured(value) => value.inventory(),
            Origin::Embedded(value) | Origin::Overlay(value) => &value.inventory,
        }
    }
    /// Exact manifest snapshots for filesystem captures; logical inputs have none.
    pub fn manifests(&self) -> &[PackageManifestSnapshot] {
        match &self.0 {
            Origin::Captured(value) => value.manifests(),
            Origin::Embedded(_) | Origin::Overlay(_) => &[],
        }
    }
    /// Immutable source records in inventory order, without source-path rereads.
    pub fn sources(&self) -> &[CapturedPackageSource] {
        match &self.0 {
            Origin::Captured(value) => value.sources(),
            Origin::Embedded(value) | Origin::Overlay(value) => &value.sources,
        }
    }
    /// Aggregate actual retained source bytes.
    pub fn source_bytes(&self) -> usize {
        match &self.0 {
            Origin::Captured(value) => value.source_bytes(),
            Origin::Embedded(value) | Origin::Overlay(value) => value.bytes,
        }
    }
    /// Canonical capture root or logical embedded root, never an implicit read request.
    pub fn root(&self) -> &std::path::Path {
        &self.inventory().graph.root
    }
}
