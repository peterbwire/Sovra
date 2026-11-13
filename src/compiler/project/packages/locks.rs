//! Deterministic local-source lock generation and verification from retained inputs.
mod json;
use super::{error, LoadedPackageSources, PackageSourceKind};
use crate::compiler::diagnostics::Diagnostics;
use json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};
/// Maximum encoded lock input/output and cumulative decoded string bytes.
pub const MAX_LOCK_BYTES: usize = 8 * 1024 * 1024;
/// A complete canonical lock document, generated without filesystem writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLock {
    text: String,
}
impl SourceLock {
    /// Canonical version-one JSON with one final LF.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}
fn problem(message: impl Into<String>) -> Diagnostics {
    error("E4165", message.into(), None)
}
fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}
fn text(value: &str) -> Result<Value, Diagnostics> {
    if value.len() > 16 * 1024 {
        return Err(problem("lock string exceeds 16 KiB"));
    }
    Ok(Value::String(value.into()))
}
fn relative(path: &str) -> Result<String, Diagnostics> {
    if path.contains('\\') {
        return Err(problem("lock source paths require portable slash spelling"));
    }
    let mut parts = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| problem("lock paths require UTF-8"))?,
            ),
            Component::CurDir => {}
            _ => return Err(problem("lock source path must remain package-relative")),
        }
    }
    Ok(parts.join("/"))
}
fn source(path: &str, bytes: &str) -> Result<Value, Diagnostics> {
    Ok(object([
        ("path", text(&relative(path)?)?),
        (
            "sha256",
            text(&crate::native_build::sha256::digest(bytes.as_bytes()))?,
        ),
        ("bytes", Value::Number(bytes.len() as u64)),
    ]))
}
fn expected(inputs: &LoadedPackageSources) -> Result<Value, Diagnostics> {
    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();
    for source in &inputs.inventory().sources {
        if seen.insert(source.package_root.clone()) {
            roots.push(source.package_root.clone());
        }
    }
    let ids: BTreeMap<_, _> = roots
        .iter()
        .enumerate()
        .map(|(index, root)| (root.clone(), format!("p{index}")))
        .collect();
    let mut packages = Vec::new();
    for root in roots {
        let package = &inputs.inventory().graph.packages[&root];
        let manifest = inputs
            .manifests()
            .iter()
            .find(|manifest| manifest.package_root() == root)
            .ok_or_else(|| problem("missing captured manifest"))?;
        let mut entry = None;
        let mut modules = Vec::new();
        for unit in inputs.units() {
            let registration = &inputs.inventory().sources[unit.source_index()];
            if registration.package_root != root {
                continue;
            }
            let record = source(&registration.declared_path, unit.source())?;
            match &registration.kind {
                PackageSourceKind::Entry => entry = Some(record),
                PackageSourceKind::Module(name) => {
                    modules.push(object([("name", text(name)?), ("source", record)]))
                }
            }
        }
        let dependencies = package
            .dependencies
            .iter()
            .map(|(alias, target)| {
                Ok(object([
                    ("alias", text(alias)?),
                    (
                        "path",
                        text(
                            manifest
                                .dependencies()
                                .get(alias)
                                .ok_or_else(|| problem("missing captured dependency locator"))?,
                        )?,
                    ),
                    ("target", text(&ids[target])?),
                ]))
            })
            .collect::<Result<Vec<_>, Diagnostics>>()?;
        packages.push(object([
            ("id", text(&ids[&root])?),
            ("name", text(&package.name)?),
            (
                "version",
                package
                    .version
                    .as_deref()
                    .map(text)
                    .transpose()?
                    .unwrap_or(Value::Null),
            ),
            (
                "manifest",
                object([
                    ("sha256", text(&manifest.sha256())?),
                    ("bytes", Value::Number(manifest.source().len() as u64)),
                ]),
            ),
            (
                "entry",
                entry.ok_or_else(|| problem("missing captured package entry"))?,
            ),
            ("modules", Value::Array(modules)),
            ("dependencies", Value::Array(dependencies)),
        ]));
    }
    Ok(object([
        ("format", text("sovra-lock")?),
        ("version", Value::Number(1)),
        ("root", text("p0")?),
        ("packages", Value::Array(packages)),
    ]))
}
/// Generate deterministic version-one lock text from the exact retained graph inputs.
/// No source/manifest rereads, code execution, filesystem writes or network access.
/// Size/string/structural admission returns E4165 without a partial document.
pub fn generate_source_lock(inputs: &LoadedPackageSources) -> Result<SourceLock, Diagnostics> {
    let value = expected(inputs)?;
    let output = json::canonical(&value).map_err(problem)?;
    json::parse(&output).map_err(problem)?;
    Ok(SourceLock { text: output })
}
/// Verify the entire lock against captured graph topology and manifest/source bytes.
/// Whitespace/key order may vary; duplicate/unknown/missing data, wrong versions,
/// hashes, types, paths or graph references fail exact normalized comparison.
/// Never rewrites a stale lock or falls back to unlocked compilation.
pub fn verify_source_lock(inputs: &LoadedPackageSources, lock: &str) -> Result<(), Diagnostics> {
    let actual = json::parse(lock).map_err(problem)?;
    let expected = expected(inputs)?;
    if actual != expected {
        return Err(problem(
            "lock does not match captured graph and input bytes",
        ));
    }
    Ok(())
}

/// Verify an existing root lock and compile exactly the same captured inputs.
/// Missing/stale/malformed locks fail before checking/emission, with no rereads
/// of manifest/source paths and no unlocked fallback or lock modification.
pub fn compile_locked(
    root: impl AsRef<Path>,
) -> Result<super::RegisteredFunctionPackage, super::CheckedPackageError> {
    let inputs = super::load_package_sources(root)?;
    check_existing(&inputs)?;
    super::registered_functions::compile_captured_records(inputs)
}

pub(crate) fn check_existing(inputs: &LoadedPackageSources) -> Result<(), Diagnostics> {
    let root = &inputs.inventory().graph.root;
    let lock = super::source_units::read_guarded(&root.join("sovra.lock"), root, MAX_LOCK_BYTES)
        .map_err(|_| problem("cannot capture lock: missing, unsafe, unreadable or over budget"))?;
    verify_source_lock(inputs, &lock)
}

/// Generate and publish the root lock with staged, guarded atomic replacement.
/// Existing unsupported formats and unsafe publication targets are refused.
/// Package sources are captured/parsed but not executed; semantic validity is
/// still required by subsequent locked compilation.
pub fn write_source_lock(root: impl AsRef<Path>) -> Result<(), Diagnostics> {
    let inputs = super::load_package_sources(root)?;
    let lock = generate_source_lock(&inputs)?;
    publication::publish(&inputs.inventory().graph.root, lock.as_str()).map_err(problem)
}
mod publication;

/// Verify the on-disk lock against captured sources without compiling or writing.
pub fn check_source_lock(root: &Path) -> Result<(), Diagnostics> {
    let inputs = super::load_package_sources(root)?;
    check_existing(&inputs)
}

#[cfg(all(
    test,
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod tests {
    use super::*;
    #[test]
    fn verified_captured_inputs_compile_after_the_entire_disk_graph_is_removed() {
        let root =
            std::env::temp_dir().join(format!("sovra-verified-owned-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname=\"owned\"\nentry=\"main.svr\"\n[modules]\nmath=\"math.svr\"\n",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){print(math::answer());}").unwrap();
        std::fs::write(
            root.join("math.svr"),
            "mod math{export fn answer()->Int{return 7;}}",
        )
        .unwrap();
        let inputs = super::super::load_package_sources(&root).unwrap();
        let lock = generate_source_lock(&inputs).unwrap();
        verify_source_lock(&inputs, lock.as_str()).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        let compiled =
            super::super::registered_functions::compile_captured_records(inputs).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(compiled.ir()).unwrap(),
            ["7"]
        );
    }
}
