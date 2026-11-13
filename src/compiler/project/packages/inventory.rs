//! Read-only registered source selection; no source parsing or verified byte snapshot.
use super::{error, LocalGraph, LocalPackage, Manifest, PackageGraphLimits};
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    resources::{FrontendLimits, DEFAULT_PACKAGE_SOURCE_BYTES},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// Maximum registered external modules in one package.
pub const MAX_PACKAGE_MODULES: usize = 1024;
/// Maximum selected sources across canonical packages, including entries.
pub const MAX_INVENTORY_SOURCES: usize = 4096;

/// Why a source is selected; this is not a parsed declaration identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSourceKind {
    /// The package's existing executable entry.
    Entry,
    /// One explicitly registered external module, not yet header-checked.
    Module(String),
}

/// A selection-time source location, never authority to reuse mutable path bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSource {
    /// Canonical owning package root.
    pub package_root: PathBuf,
    /// Entry or registered module spelling.
    pub kind: PackageSourceKind,
    /// Decoded path text from the owning manifest.
    pub declared_path: String,
    /// Canonical selected location observed during inventory.
    pub canonical_path: PathBuf,
    /// Original owning manifest assignment line.
    pub manifest_span: Span,
    /// Observed size, used only for preliminary admission, not content integrity.
    pub observed_bytes: u64,
}

/// Complete optional local graph/source inventory; no parsed or checked source claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSourceInventory {
    /// Existing canonical-root dependency graph.
    pub graph: LocalGraph,
    /// Deterministic alias-DFS package order, then entry and sorted module names.
    pub sources: Vec<PackageSource>,
}

/// Inspect approved `[modules]` registrations without reading or executing sources.
/// Old resolve/compile/classifier/tooling APIs retain strict entry-only admission.
/// Counts and observed sizes are bounded; errors expose no partial inventory.
/// Unix file identities detect hard-link aliases. Other hosts explicitly report
/// unavailable identity checks until implemented. Paths/metadata may change after
/// selection: compilation/locks must later capture and validate opened bytes.
pub fn inspect_source_inventory(
    root: impl AsRef<Path>,
) -> Result<PackageSourceInventory, Diagnostics> {
    let mut builder = Builder::default();
    let graph = super::resolve_internal(
        root.as_ref(),
        PackageGraphLimits::DEFAULT,
        Some(&mut builder),
    )?;
    Ok(PackageSourceInventory {
        graph,
        sources: builder.sources,
    })
}

/// Exact manifest bytes retained from the same read used by graph parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifestSnapshot {
    root: PathBuf,
    path: PathBuf,
    source: String,
    dependencies: BTreeMap<String, String>,
}
impl PackageManifestSnapshot {
    /// Canonical owning package root, never a portable published identity.
    pub fn package_root(&self) -> &Path {
        &self.root
    }
    /// Owning manifest locator.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Exact UTF-8 text parsed by the graph resolver, including comments/line endings.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Decoded declared dependency paths, indexed by direct alias.
    pub fn dependencies(&self) -> &BTreeMap<String, String> {
        &self.dependencies
    }
    /// SHA-256 of retained original bytes; integrity metadata, not authentication.
    pub fn sha256(&self) -> String {
        crate::native_build::sha256::digest(self.source.as_bytes())
    }
}

pub(super) fn capture_inventory(
    root: &Path,
) -> Result<(PackageSourceInventory, Vec<PackageManifestSnapshot>), Diagnostics> {
    let mut builder = Builder {
        capture: true,
        ..Builder::default()
    };
    let graph = super::resolve_internal(root, PackageGraphLimits::DEFAULT, Some(&mut builder))?;
    Ok((
        PackageSourceInventory {
            graph,
            sources: builder.sources,
        },
        builder.manifests,
    ))
}

#[derive(Default)]
pub(super) struct Builder {
    sources: Vec<PackageSource>,
    capture: bool,
    manifests: Vec<PackageManifestSnapshot>,
    reserved: usize,
    bytes: u64,
    identities: BTreeMap<(u64, u64), PathBuf>,
}

fn failure(manifest: &Manifest, key: &str, code: &'static str, message: &str) -> Diagnostics {
    let mut diagnostics = error(code, message.into(), None);
    manifest.attach(
        if key == "<entry>" {
            "project"
        } else {
            "modules"
        },
        if key == "<entry>" { "entry" } else { key },
        &mut diagnostics.items,
    );
    diagnostics
}

fn portable_path(value: &str) -> bool {
    !value.is_empty()
        && value.ends_with(".svr")
        && value.split('/').all(|part| {
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with('.')
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            {
                return false;
            }
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
}

impl Builder {
    pub(super) fn captures_manifests(&self) -> bool {
        self.capture
    }
    pub(super) fn retain_manifest(&mut self, root: &Path, manifest: &Manifest, source: String) {
        if !self.capture {
            return;
        }
        let dependencies = manifest
            .sections
            .iter()
            .filter_map(|(section, _)| {
                section.strip_prefix("dependencies.").map(|alias| {
                    (
                        alias.to_owned(),
                        manifest
                            .value(section, "path")
                            .expect("validated dependency path")
                            .to_owned(),
                    )
                })
            })
            .collect();
        self.manifests.push(PackageManifestSnapshot {
            root: root.to_owned(),
            path: root.join(super::MANIFEST_FILE),
            source,
            dependencies,
        });
    }
    pub(super) fn reserve(&mut self, manifest: &Manifest) -> Result<(), Diagnostics> {
        let count = manifest.entries_in_section("modules").count();
        if count > MAX_PACKAGE_MODULES {
            return Err(failure(
                manifest,
                "<entry>",
                "E4112",
                "package exceeds 1024 registered modules",
            ));
        }
        self.reserved = self
            .reserved
            .checked_add(count + 1)
            .filter(|n| *n <= MAX_INVENTORY_SOURCES)
            .ok_or_else(|| {
                failure(
                    manifest,
                    "<entry>",
                    "E4112",
                    "source inventory exceeds 4096 selected sources",
                )
            })?;
        let mut paths = BTreeSet::new();
        if let Some(entry) = manifest.value("project", "entry") {
            paths.insert(entry.replace('\\', "/").to_ascii_lowercase());
        }
        for entry in manifest.entries_in_section("modules") {
            if !portable_path(&entry.value) {
                return Err(failure(
                    manifest,
                    &entry.key,
                    "E4160",
                    "module path must be a portable relative .svr path",
                ));
            }
            if !paths.insert(entry.value.to_ascii_lowercase()) {
                return Err(failure(
                    manifest,
                    &entry.key,
                    "E4160",
                    "module path collides with entry or another registration",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn capture(
        &mut self,
        package: &LocalPackage,
        manifest: &Manifest,
    ) -> Result<(), Diagnostics> {
        let entry = manifest
            .entries_in_section("project")
            .find(|e| e.key == "entry")
            .expect("entry validated by resolver");
        self.select(
            package,
            manifest,
            "<entry>",
            PackageSourceKind::Entry,
            entry,
            &package.entry,
        )?;
        let modules: BTreeMap<_, _> = manifest
            .entries_in_section("modules")
            .map(|e| (&e.key, e))
            .collect();
        for (name, entry) in modules {
            self.select(
                package,
                manifest,
                name,
                PackageSourceKind::Module(name.clone()),
                entry,
                &package.root.join(&entry.value),
            )?;
        }
        Ok(())
    }
    fn select(
        &mut self,
        package: &LocalPackage,
        manifest: &Manifest,
        key: &str,
        kind: PackageSourceKind,
        registration: &super::super::ManifestEntry,
        path: &Path,
    ) -> Result<(), Diagnostics> {
        let canonical = path
            .canonicalize()
            .map_err(|_| failure(manifest, key, "E4160", "cannot resolve registered source"))?;
        if !canonical.starts_with(&package.root) {
            return Err(failure(
                manifest,
                key,
                "E4113",
                "registered source escapes its package root",
            ));
        }
        let metadata = fs::metadata(&canonical)
            .map_err(|_| failure(manifest, key, "E4160", "cannot inspect registered source"))?;
        if !metadata.is_file() {
            return Err(failure(
                manifest,
                key,
                "E4160",
                "registered source must be a regular file",
            ));
        }
        let identity = file_identity(&metadata).ok_or_else(|| {
            failure(
                manifest,
                key,
                "E4160",
                "source inventory file-identity checks unavailable on this host",
            )
        })?;
        if self.identities.contains_key(&identity) {
            return Err(failure(
                manifest,
                key,
                "E4160",
                "selected source identity is already registered",
            ));
        }
        if metadata.len() > FrontendLimits::DEFAULT.max_source_bytes as u64 {
            return Err(failure(
                manifest,
                key,
                "E1003",
                "registered source exceeds 16 MiB observed size limit",
            ));
        }
        self.bytes = self
            .bytes
            .checked_add(metadata.len())
            .filter(|bytes| *bytes <= DEFAULT_PACKAGE_SOURCE_BYTES as u64)
            .ok_or_else(|| {
                failure(
                    manifest,
                    key,
                    "E1003",
                    "source inventory exceeds 64 MiB observed size limit",
                )
            })?;
        self.identities.insert(identity, canonical.clone());
        self.sources.push(PackageSource {
            package_root: package.root.clone(),
            kind,
            declared_path: registration.value.clone(),
            canonical_path: canonical,
            manifest_span: registration.span,
            observed_bytes: metadata.len(),
        });
        Ok(())
    }
}

fn file_identity(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::portable_path;
    #[test]
    fn module_paths_use_the_approved_portable_component_grammar() {
        for value in ["src/math.svr", "math.svr", "some-dir/.math.svr"] {
            assert!(portable_path(value), "{value}");
        }
        for value in [
            "",
            "../math.svr",
            "./math.svr",
            "/math.svr",
            "src//math.svr",
            "C:/math.svr",
            "\\\\server\\math.svr",
            "src\\math.svr",
            "NUL.svr",
            "src/COM1.data/math.svr",
            "Lpt9/math.svr",
            "trailing./math.svr",
            "space /math.svr",
            "λ/math.svr",
            "math.SVR",
        ] {
            assert!(!portable_path(value), "{value}");
        }
    }
}
