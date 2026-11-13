//! Bounded source-owned parsing, prior to multi-file semantic linking.
use super::{error, PackageSource, PackageSourceInventory, PackageSourceKind};
use crate::compiler::{
    ast::Program,
    diagnostics::{Diagnostics, Span},
    lexer::Lexer,
    resources::{check_ast_budget, PackageCompilationLimits, DEFAULT_PACKAGE_SOURCE_BYTES},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Maximum original non-EOF tokens across loaded package sources.
pub const MAX_PACKAGE_SOURCE_TOKENS: usize = 4_000_000;

/// One original file-local executable dependency import, not a resolved interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSourceImport {
    /// Direct dependency alias spelling.
    pub alias: String,
    /// Imported module spelling.
    pub module: String,
    /// Exact original import range in the owning source unit.
    pub span: Span,
}

/// Captured source and AST owned by one inventory source, never concatenated.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageSourceUnit {
    pub(super) source_index: usize,
    pub(super) source: String,
    pub(super) program: Program,
    pub(super) imports: Vec<PackageSourceImport>,
}
impl PackageSourceUnit {
    /// Index in the owning inventory source table.
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    /// Exact captured UTF-8 text used for parsing.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Source-owned syntax, not a semantically checked program.
    pub fn program(&self) -> &Program {
        &self.program
    }
    /// Original file-local imports; no sibling-file import leakage is introduced.
    pub fn imports(&self) -> &[PackageSourceImport] {
        &self.imports
    }
}

/// Complete captured/parsed sources, without linked IR or checked type facts.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPackageSources {
    pub(super) inventory: PackageSourceInventory,
    pub(super) units: Vec<PackageSourceUnit>,
    manifests: Vec<super::PackageManifestSnapshot>,
    tokens: usize,
    nodes: usize,
}
impl LoadedPackageSources {
    /// Owning canonical graph and source registration metadata.
    pub fn inventory(&self) -> &PackageSourceInventory {
        &self.inventory
    }
    /// Parsed sources in inventory order, with no mutable source/AST access.
    pub fn units(&self) -> &[PackageSourceUnit] {
        &self.units
    }
    /// Exact captured manifest text used by graph resolution, once per canonical root.
    pub fn manifests(&self) -> &[super::PackageManifestSnapshot] {
        &self.manifests
    }
    /// Total original non-EOF tokens admitted before parsing.
    pub fn token_count(&self) -> usize {
        self.tokens
    }
    /// Cumulative parsed AST node count.
    pub fn ast_nodes(&self) -> usize {
        self.nodes
    }
}

#[derive(Clone, Copy)]
struct LoadLimits {
    bytes: usize,
    tokens: usize,
    nodes: usize,
}
impl LoadLimits {
    const DEFAULT: Self = Self {
        bytes: DEFAULT_PACKAGE_SOURCE_BYTES,
        tokens: MAX_PACKAGE_SOURCE_TOKENS,
        nodes: PackageCompilationLimits::DEFAULT.max_ast_nodes,
    };
}

/// Capture and parse approved registered sources, validating matching module headers.
/// Original file ownership/imports/spans are retained; duplicates are rejected.
/// No semantic checking, linking, emission or execution occurs. The guarded read
/// path currently requires Linux x86/x86_64/aarch64 and procfs handle inspection;
/// other hosts explicitly fail until an equivalent reader is implemented.
/// This is not an atomic filesystem or verified manifest/lock snapshot.
pub fn load_package_sources(root: impl AsRef<Path>) -> Result<LoadedPackageSources, Diagnostics> {
    load(root.as_ref(), LoadLimits::DEFAULT)
}

fn located(mut errors: Diagnostics, path: &Path) -> Diagnostics {
    for diagnostic in &mut errors.items {
        diagnostic.source_file = Some(path.to_string_lossy().into_owned());
    }
    errors
}
fn fail(code: &'static str, message: &str, path: &Path, span: Span) -> Diagnostics {
    error(code, message.into(), Some((path, span)))
}
fn file_span() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    }
}

fn load(root: &Path, limits: LoadLimits) -> Result<LoadedPackageSources, Diagnostics> {
    let (inventory, manifests) = super::inventory::capture_inventory(root)?;
    let mut units = Vec::new();
    let mut bytes = 0usize;
    let mut tokens = 0usize;
    let mut nodes = 0usize;
    let mut identities = BTreeSet::new();
    let mut modules = BTreeMap::new();
    for (source_index, registration) in inventory.sources.iter().enumerate() {
        let path = &registration.canonical_path;
        let remaining = limits.bytes.checked_sub(bytes).ok_or_else(|| {
            fail(
                "E1003",
                "aggregate package source byte limit exceeded",
                path,
                file_span(),
            )
        })?;
        let (source, identity) = read_snapshot(registration, remaining)?;
        if !identities.insert(identity) {
            return Err(fail(
                "E4161",
                "opened source identity is already loaded",
                path,
                file_span(),
            ));
        }
        bytes += source.len();
        let lexed = Lexer::new()
            .tokenize(&source)
            .map_err(|e| located(e, path))?;
        tokens = tokens
            .checked_add(lexed.len().saturating_sub(1))
            .filter(|n| *n <= limits.tokens)
            .ok_or_else(|| {
                fail(
                    "E1003",
                    "aggregate package token limit exceeded",
                    path,
                    file_span(),
                )
            })?;
        let (program, imports) =
            super::parse_import_tokens(&lexed).map_err(|e| located(e, path))?;
        let used = check_ast_budget(&program, limits.nodes.saturating_sub(nodes))
            .map_err(|e| located(e, path))?;
        nodes += used;
        if let PackageSourceKind::Module(name) = &registration.kind {
            if program.modules.len() != 1
                || program.modules[0].name != *name
                || !program.functions.is_empty()
                || !program.type_declarations.is_empty()
                || !program.struct_declarations.is_empty()
            {
                return Err(fail("E4162","registered source requires exactly its matching inline module and optional dependency imports",path,program.modules.first().map_or(file_span(),|module|module.span)));
            }
        }
        for module in &program.modules {
            if modules
                .insert(
                    (registration.package_root.clone(), module.name.clone()),
                    source_index,
                )
                .is_some()
            {
                return Err(fail(
                    "E3008",
                    "duplicate module name in package sources",
                    path,
                    module.span,
                ));
            }
        }
        units.push(PackageSourceUnit {
            source_index,
            source,
            program,
            imports: imports
                .into_iter()
                .map(|import| PackageSourceImport {
                    alias: import.alias,
                    module: import.module,
                    span: import.span,
                })
                .collect(),
        });
    }
    Ok(LoadedPackageSources {
        inventory,
        units,
        manifests,
        tokens,
        nodes,
    })
}

fn read_snapshot(
    registration: &PackageSource,
    limit: usize,
) -> Result<(String, (u64, u64)), Diagnostics> {
    read_snapshot_with(registration, limit, || {})
}

fn read_snapshot_with(
    registration: &PackageSource,
    limit: usize,
    before_open: impl FnOnce(),
) -> Result<(String, (u64, u64)), Diagnostics> {
    read_guarded_with(
        &registration.canonical_path,
        &registration.package_root,
        limit,
        before_open,
    )
}

pub(super) fn read_guarded(path: &Path, root: &Path, limit: usize) -> Result<String, Diagnostics> {
    read_guarded_with(path, root, limit, || {}).map(|(source, _)| source)
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn read_guarded_with(
    path: &Path,
    root: &Path,
    limit: usize,
    before_open: impl FnOnce(),
) -> Result<(String, (u64, u64)), Diagnostics> {
    use crate::compiler::resources::FrontendLimits;
    use std::os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    };
    use std::{
        fs::{self, OpenOptions},
        io::Read,
    };
    let bad = || {
        fail(
            "E4161",
            "source changed or opened-handle containment unavailable",
            path,
            file_span(),
        )
    };
    let expected = fs::symlink_metadata(path).map_err(|_| bad())?;
    if !expected.is_file() || expected.file_type().is_symlink() {
        return Err(bad());
    }
    let allowance = limit.min(FrontendLimits::DEFAULT.max_source_bytes);
    if expected.len() > allowance as u64 {
        return Err(fail(
            "E1003",
            "package source byte allowance exceeded",
            path,
            file_span(),
        ));
    }
    before_open();
    // Linux UAPI values, restricted to the supported architectures above.
    const O_NONBLOCK: i32 = 0o4000;
    const O_NOFOLLOW: i32 = 0o400000;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK | O_NOFOLLOW)
        .open(path)
        .map_err(|_| bad())?;
    let opened = file.metadata().map_err(|_| bad())?;
    let same = |a: &fs::Metadata, b: &fs::Metadata| {
        a.is_file()
            && b.is_file()
            && a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.len() == b.len()
            && a.mtime() == b.mtime()
            && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
            && a.mode() == b.mode()
            && a.nlink() == b.nlink()
    };
    let descriptor = std::path::PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    let contained = || {
        fs::read_link(&descriptor)
            .is_ok_and(|opened_path| opened_path == *path && opened_path.starts_with(root))
    };
    if !same(&expected, &opened) || !contained() {
        return Err(bad());
    }
    let mut buffer = Vec::new();
    (&file)
        .take(allowance as u64 + 1)
        .read_to_end(&mut buffer)
        .map_err(|_| bad())?;
    if buffer.len() > allowance {
        return Err(fail(
            "E1003",
            "package source byte allowance exceeded",
            path,
            file_span(),
        ));
    }
    if !same(&opened, &file.metadata().map_err(|_| bad())?) || !contained() {
        return Err(bad());
    }
    let source = String::from_utf8(buffer)
        .map_err(|_| fail("E4161", "package source must be UTF-8", path, file_span()))?;
    Ok((source, (opened.dev(), opened.ino())))
}

#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
)))]
fn read_guarded_with(
    path: &Path,
    _root: &Path,
    _limit: usize,
    _before_open: impl FnOnce(),
) -> Result<(String, (u64, u64)), Diagnostics> {
    Err(fail(
        "E4161",
        "guarded package source capture unavailable on this host",
        path,
        file_span(),
    ))
}

#[cfg(all(
    test,
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod tests {
    use super::*;
    use crate::compiler::project::packages::inspect_source_inventory;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "sovra-source-capture-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            fs::write(
                root.join("sovra.toml"),
                "[project]\nname=\"capture\"\nentry=\"main.svr\"\n",
            )
            .unwrap();
            fs::write(root.join("main.svr"), "fn main(){}").unwrap();
            Self(root)
        }
        fn registration(&self) -> PackageSource {
            inspect_source_inventory(&self.0).unwrap().sources.remove(0)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn byte_token_and_ast_admission_use_actual_buffers_and_exact_cumulative_limits() {
        let fixture = Fixture::new();
        fs::write(
            fixture.0.join("sovra.toml"),
            "[project]\nname=\"capture\"\nentry=\"main.svr\"\n[modules]\nmath=\"math.svr\"\n",
        )
        .unwrap();
        fs::write(fixture.0.join("math.svr"), "mod math{}").unwrap();
        let full = load(&fixture.0, LoadLimits::DEFAULT).unwrap();
        assert_eq!(full.units().len(), 2);
        let bytes = full
            .units()
            .iter()
            .map(|unit| unit.source().len())
            .sum::<usize>();
        let tokens = full.token_count();
        let nodes = full.ast_nodes();
        assert!(load(
            &fixture.0,
            LoadLimits {
                bytes,
                tokens,
                nodes
            }
        )
        .is_ok());
        for limits in [
            LoadLimits {
                bytes: bytes - 1,
                tokens,
                nodes,
            },
            LoadLimits {
                bytes,
                tokens: tokens - 1,
                nodes,
            },
            LoadLimits {
                bytes,
                tokens,
                nodes: nodes - 1,
            },
        ] {
            assert!(load(&fixture.0, limits).is_err());
        }
        // Inventory sizes cannot override the actual opened byte allowance.
        let registration = fixture.registration();
        fs::write(&registration.canonical_path, "x".repeat(bytes + 1)).unwrap();
        assert!(read_snapshot(&registration, bytes)
            .unwrap_err()
            .items
            .iter()
            .any(|d| d.code == "E1003"));
    }
    #[test]
    fn substitutions_before_open_refuse_links_fifos_and_other_regular_files() {
        use std::os::unix::fs::symlink;
        for kind in ["link", "fifo", "regular"] {
            let fixture = Fixture::new();
            let registration = fixture.registration();
            let path = registration.canonical_path.clone();
            let outside = fixture.0.join("other.svr");
            fs::write(&outside, "fn main(){}").unwrap();
            let result = read_snapshot_with(&registration, 1024, || {
                fs::remove_file(&path).unwrap();
                match kind {
                    "link" => symlink(&outside, &path).unwrap(),
                    "fifo" => assert!(std::process::Command::new("mkfifo")
                        .arg(&path)
                        .status()
                        .unwrap()
                        .success()),
                    _ => {
                        fs::rename(&outside, &path).unwrap();
                    }
                }
            });
            assert!(
                result.unwrap_err().items.iter().any(|d| d.code == "E4161"),
                "{kind}"
            );
        }
    }
    #[test]
    fn parent_link_substitution_cannot_pass_handle_containment() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let directory = fixture.0.join("src");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("code.svr"), "fn main(){}").unwrap();
        fs::write(
            fixture.0.join("sovra.toml"),
            "[project]\nname=\"capture\"\nentry=\"src/code.svr\"\n",
        )
        .unwrap();
        let registration = fixture.registration();
        let saved = fixture.0.join("moved");
        // Same inode through a redirected parent still fails exact opened-path proof.
        let result = read_snapshot_with(&registration, 1024, || {
            fs::rename(&directory, &saved).unwrap();
            symlink(&saved, &directory).unwrap();
        });
        assert!(result.unwrap_err().items.iter().any(|d| d.code == "E4161"));
    }
}
