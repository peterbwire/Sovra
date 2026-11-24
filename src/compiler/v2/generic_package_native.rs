//! Strict source/topology package images; no paths become filesystem capabilities.
use super::*;
use crate::compiler::{
    diagnostics::{diagnostic_message, Span},
    project::packages::{
        CapturedPackageSource, LocalGraph, LocalPackage, PackageSource, PackageSourceInventory,
        PackageSourceKind,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
const MAGIC: &[u8] = b"SVRGP\0";
const HEADER: usize = 16;
const VERSION: u8 = 1;
const FEATURES: u32 = 511;
const COLLECTION_FEATURES: u32 = 1023;
const DATA_FEATURES: u32 = 2047;
const PACKAGES: usize = 64;
const SOURCES: usize = 256;
const EDGES: usize = 512;
const METADATA: usize = 65_536;
/// Maximum complete package image, including framing and topology metadata.
pub const GENERIC_PACKAGE_NATIVE_IMAGE_LIMIT: usize = 2 * 1024 * 1024;
/// Maximum aggregate original UTF-8 source in a package image.
pub const GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT: usize = 1024 * 1024;
/// Recognize the reserved package family; malformed members still require decoding.
pub fn is_generic_package_native_image(image: &[u8]) -> bool {
    image.starts_with(b"SVRGP")
}
fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn portable(value: &str) -> bool {
    !value.is_empty()
        && value.ends_with(".svr")
        && !value.contains('\\')
        && !value.contains(':')
        && value.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.chars().any(char::is_control)
        })
}
fn relative(path: &Path, root: &Path) -> Result<String, String> {
    let path = path
        .strip_prefix(root)
        .map_err(|_| "native source is outside its package")?;
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::Normal(part) => {
                parts.push(part.to_str().ok_or("native source paths require UTF-8")?)
            }
            _ => return Err("native source path must be normalized".into()),
        }
    }
    let value = parts.join("/");
    if !portable(&value) {
        return Err("invalid native relative source path".into());
    }
    Ok(value)
}
struct Writer {
    bytes: Vec<u8>,
    metadata: usize,
    sources: usize,
}
impl Writer {
    fn push(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|n| *n <= GENERIC_PACKAGE_NATIVE_IMAGE_LIMIT)
            .is_none()
        {
            return Err("native package image allowance exceeded".into());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn number(&mut self, value: usize) -> Result<(), String> {
        let value = u32::try_from(value).map_err(|_| "native package integer overflow")?;
        self.push(&value.to_le_bytes())
    }
    fn text(&mut self, value: &str, source: bool) -> Result<(), String> {
        let left = if source {
            &mut self.sources
        } else {
            &mut self.metadata
        };
        let max = if source {
            GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT
        } else {
            4096
        };
        if value.len() > max {
            return Err("native package string allowance exceeded".into());
        }
        *left = left
            .checked_sub(value.len())
            .ok_or("native package text allowance exceeded")?;
        self.number(value.len())?;
        self.push(value.as_bytes())
    }
}
impl CompiledGenericPackage {
    /// Embed source and validated import topology without original host roots,
    /// manifests, lock bytes, grants or filesystem startup dependencies.
    pub fn encode_generic_package_native_image(&self) -> Result<Vec<u8>, String> {
        self.encode_package_native_mode(0)
    }
    /// Encode the explicitly checked collection package profile with exact feature word 1023.
    /// Ordinary encoders/decoders and CLI/native launch admission remain unchanged.
    pub fn encode_generic_collection_package_native_image(&self) -> Result<Vec<u8>, String> {
        self.encode_package_native_mode(1)
    }
    /// Encode only the explicit data profile with exact feature word 2047.
    pub fn encode_generic_data_package_native_image(&self) -> Result<Vec<u8>, String> {
        self.encode_package_native_mode(2)
    }
    fn encode_package_native_mode(&self, profile: u8) -> Result<Vec<u8>, String> {
        let collections = profile >= 1;
        let data = profile == 2;
        if self.ir().checked_ir().has_data_operations() != data {
            return Err("native data feature is not enabled".into());
        }
        if !collections && self.ir().checked_ir().has_collection_operations() {
            return Err("native collection feature is not enabled".into());
        }
        if collections && !self.ir().checked_ir().has_collection_operations() {
            return Err("native collection profile requires collection compilation".into());
        }
        let features = if data {
            DATA_FEATURES
        } else if collections {
            COLLECTION_FEATURES
        } else {
            FEATURES
        };
        self.validate_command_entry()
            .map_err(|e| diagnostic_message!("{e:?}"))?;
        let inputs = self.bodies().types().declarations().inputs();
        let inventory = inputs.inventory();
        if inventory.graph.packages.len() > PACKAGES
            || inventory.sources.len() > SOURCES
            || inputs.source_bytes() > GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT
        {
            return Err("native package graph/source allowance exceeded".into());
        }
        let mut roots = Vec::new();
        let mut seen = BTreeSet::new();
        for source in &inventory.sources {
            if seen.insert(&source.package_root) {
                roots.push(&source.package_root);
            }
        }
        if roots.len() != inventory.graph.packages.len()
            || roots.first().copied() != Some(&inventory.graph.root)
        {
            return Err("invalid native package ownership order".into());
        }
        let ids: BTreeMap<_, _> = roots
            .iter()
            .enumerate()
            .map(|(i, root)| (*root, i))
            .collect();
        let mut writer = Writer {
            bytes: Vec::new(),
            metadata: METADATA,
            sources: GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT,
        };
        writer.push(MAGIC)?;
        writer.push(&[VERSION, VALUE_MODEL_V2 as u8])?;
        writer.push(&features.to_le_bytes())?;
        writer.number(0)?;
        writer.number(roots.len())?;
        let mut edges = EDGES;
        for root in &roots {
            let package = inventory
                .graph
                .packages
                .get(*root)
                .ok_or("missing native package")?;
            edges = edges
                .checked_sub(package.dependencies.len())
                .ok_or("native dependency allowance exceeded")?;
            writer.number(package.dependencies.len())?;
            for (alias, target) in &package.dependencies {
                if alias.len() > 1024 || !identifier(alias) {
                    return Err("native dependency alias must be an identifier".into());
                }
                writer.text(alias, false)?;
                writer.number(*ids.get(target).ok_or("native dependency target absent")?)?;
            }
        }
        writer.number(inputs.sources().len())?;
        for unit in inputs.sources() {
            let source = inventory
                .sources
                .get(unit.source_index())
                .ok_or("native source owner absent")?;
            writer.number(
                *ids.get(&source.package_root)
                    .ok_or("native source package absent")?,
            )?;
            match &source.kind {
                PackageSourceKind::Entry => writer.push(&[0])?,
                PackageSourceKind::Module(name) => {
                    if name.len() > 1024 || !identifier(name) {
                        return Err("native module name must be an identifier".into());
                    }
                    writer.push(&[1])?;
                    writer.text(name, false)?;
                }
            }
            writer.text(
                &relative(&source.canonical_path, &source.package_root)?,
                false,
            )?;
            writer.text(unit.source(), true)?;
        }
        let length = u32::try_from(writer.bytes.len() - HEADER)
            .map_err(|_| "native image length overflow")?;
        writer.bytes[12..16].copy_from_slice(&length.to_le_bytes());
        // Validate framing/topology too; encoding never normalizes an invalid graph.
        parse(&writer.bytes, features)?;
        Ok(writer.bytes)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    metadata: usize,
    source: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(length)
            .ok_or("native package length overflow")?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or("truncated native package")?;
        self.at = end;
        Ok(bytes)
    }
    fn number(&mut self, max: usize) -> Result<usize, String> {
        let value =
            u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "native integer")?) as usize;
        if value > max {
            return Err("native package count allowance exceeded".into());
        }
        Ok(value)
    }
    fn text(&mut self, source: bool) -> Result<&'a str, String> {
        let length = self.number(if source {
            GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT
        } else {
            4096
        })?;
        let left = if source {
            &mut self.source
        } else {
            &mut self.metadata
        };
        *left = left
            .checked_sub(length)
            .ok_or("native package text allowance exceeded")?;
        std::str::from_utf8(self.take(length)?)
            .map_err(|_| "native package strings require UTF-8".into())
    }
}
struct WireSource<'a> {
    owner: usize,
    module: Option<&'a str>,
    path: &'a str,
    text: &'a str,
}
struct Wire<'a> {
    packages: Vec<Vec<(&'a str, usize)>>,
    sources: Vec<WireSource<'a>>,
    bytes: usize,
}
fn parse(image: &[u8], features: u32) -> Result<Wire<'_>, String> {
    if image.len() < HEADER
        || image.len() > GENERIC_PACKAGE_NATIVE_IMAGE_LIMIT
        || image.get(..6) != Some(MAGIC)
        || image[6] != VERSION
        || image[7] != VALUE_MODEL_V2 as u8
        || image[8..12] != features.to_le_bytes()
    {
        return Err("unsupported native package header/profile".into());
    }
    let declared = u32::from_le_bytes(
        image[12..16]
            .try_into()
            .map_err(|_| "native image header")?,
    ) as usize;
    if declared != image.len() - HEADER {
        return Err("native package length mismatch".into());
    }
    let mut reader = Reader {
        bytes: image,
        at: HEADER,
        metadata: METADATA,
        source: GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT,
    };
    let count = reader.number(PACKAGES)?;
    if count == 0 {
        return Err("native package graph is empty".into());
    }
    let mut packages = Vec::new();
    let mut left = EDGES;
    for _ in 0..count {
        let n = reader.number(left)?;
        left -= n;
        let mut edges = Vec::new();
        let mut previous = None;
        for _ in 0..n {
            let alias = reader.text(false)?;
            if alias.len() > 1024 || !identifier(alias) || previous.is_some_and(|p| p >= alias) {
                return Err("native aliases must be unique sorted identifiers".into());
            }
            previous = Some(alias);
            let target = reader.number(count - 1)?;
            edges.push((alias, target));
        }
        packages.push(edges);
    }
    let mut reached = vec![false; count];
    let mut pending = vec![0];
    reached[0] = true;
    while let Some(at) = pending.pop() {
        for &(_, target) in &packages[at] {
            if !reached[target] {
                reached[target] = true;
                pending.push(target);
            }
        }
    }
    if reached.iter().any(|seen| !*seen) {
        return Err("native graph has unreachable packages".into());
    }
    let mut incoming = vec![0usize; count];
    for package in &packages {
        for &(_, target) in package {
            incoming[target] += 1;
        }
    }
    let mut pending: Vec<_> = incoming
        .iter()
        .enumerate()
        .filter(|(_, n)| **n == 0)
        .map(|(i, _)| i)
        .collect();
    let mut visited = 0;
    while let Some(at) = pending.pop() {
        visited += 1;
        for &(_, target) in &packages[at] {
            incoming[target] -= 1;
            if incoming[target] == 0 {
                pending.push(target);
            }
        }
    }
    if visited != count {
        return Err("native dependency cycles are unavailable".into());
    }
    let n = reader.number(SOURCES)?;
    let mut sources = Vec::new();
    let mut entries = vec![false; count];
    let mut paths = BTreeSet::new();
    let mut previous_owner = 0;
    let mut previous_module = None;
    for _ in 0..n {
        let owner = reader.number(count - 1)?;
        if owner < previous_owner {
            return Err("native sources must follow package order".into());
        }
        if owner != previous_owner {
            previous_module = None;
        }
        previous_owner = owner;
        let kind = reader.take(1)?[0];
        let module = match kind {
            0 => {
                if entries[owner] {
                    return Err("duplicate native entry".into());
                }
                entries[owner] = true;
                None
            }
            1 => {
                let name = reader.text(false)?;
                if !entries[owner]
                    || name.len() > 1024
                    || !identifier(name)
                    || previous_module.is_some_and(|p| p >= name)
                {
                    return Err("native modules must follow entry in unique sorted order".into());
                }
                previous_module = Some(name);
                Some(name)
            }
            _ => return Err("unknown native source kind".into()),
        };
        let path = reader.text(false)?;
        if !portable(path) || !paths.insert((owner, path)) {
            return Err("invalid or duplicate native source path".into());
        }
        let text = reader.text(true)?;
        sources.push(WireSource {
            owner,
            module,
            path,
            text,
        });
    }
    if entries.iter().any(|entry| !*entry) || reader.at != image.len() {
        return Err("missing native entries or trailing bytes".into());
    }
    Ok(Wire {
        packages,
        sources,
        bytes: GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT - reader.source,
    })
}
fn embedded(wire: Wire<'_>) -> super::GenericPackageSources {
    let roots: Vec<_> = (0..wire.packages.len())
        .map(|i| PathBuf::from(format!("@sovra-image/p{i}")))
        .collect();
    let mut packages = BTreeMap::new();
    for (index, edges) in wire.packages.iter().enumerate() {
        let root = &roots[index];
        let entry = wire
            .sources
            .iter()
            .find(|s| s.owner == index && s.module.is_none())
            .expect("validated entry");
        packages.insert(
            root.clone(),
            LocalPackage {
                root: root.clone(),
                name: format!("p{index}"),
                version: None,
                entry: root.join(entry.path),
                dependencies: edges
                    .iter()
                    .map(|(name, target)| ((*name).into(), roots[*target].clone()))
                    .collect(),
            },
        );
    }
    let mut registrations = Vec::new();
    let mut sources = Vec::new();
    for (index, source) in wire.sources.into_iter().enumerate() {
        let root = &roots[source.owner];
        registrations.push(PackageSource {
            package_root: root.clone(),
            kind: source.module.map_or(PackageSourceKind::Entry, |name| {
                PackageSourceKind::Module(name.into())
            }),
            declared_path: source.path.into(),
            canonical_path: root.join(source.path),
            manifest_span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
            observed_bytes: source.text.len() as u64,
        });
        sources.push(CapturedPackageSource::embedded(index, source.text.into()));
    }
    super::GenericPackageSources::embedded(super::generic_package_inputs::EmbeddedPackageSources {
        inventory: PackageSourceInventory {
            graph: LocalGraph {
                root: roots[0].clone(),
                packages,
            },
            sources: registrations,
        },
        sources,
        bytes: wire.bytes,
    })
}
/// Decode bounded source/topology bytes and recheck the entire package in memory.
/// Logical image locations never become guarded filesystem-capture authority.
pub fn decode_generic_package_native_image(image: &[u8]) -> Result<CompiledGenericPackage, String> {
    decode_package_native_mode(image, 0)
}
/// Recheck an exact word-1023 collection image in memory under finite package limits.
/// Embedded paths remain logical locations and cannot grant filesystem authority.
pub fn decode_generic_collection_package_native_image(
    image: &[u8],
) -> Result<CompiledGenericPackage, String> {
    decode_package_native_mode(image, 1)
}
fn decode_package_native_mode(image: &[u8], profile: u8) -> Result<CompiledGenericPackage, String> {
    let collections = profile >= 1;
    let data = profile == 2;
    let features = if data {
        DATA_FEATURES
    } else if collections {
        COLLECTION_FEATURES
    } else {
        FEATURES
    };
    let inputs = embedded(parse(image, features)?);
    let program = super::generic_package_compile::compile_package_inputs_profile(
        inputs,
        "main",
        GenericPackageCompilationLimits::DEFAULT,
        collections,
        data,
    )
    .map_err(|e| diagnostic_message!("{e:?}"))?;
    program
        .validate_command_entry()
        .map_err(|e| diagnostic_message!("{e:?}"))?;
    Ok(program)
}
/// Recheck and execute a source-backed package image with finite host-free policy.
/// No argument transport, environment grants, project-file reads or host fallback.
pub fn execute_generic_package_native_image(image: &[u8]) -> Result<GenericVmOutcome, String> {
    let program = decode_generic_package_native_image(image)?;
    execute_generic_package(&program, generic_command_runtime_policy())
        .map_err(|e| diagnostic_message!("{e:?}"))
}

/// Recheck and execute only the explicit collection package image profile.
/// Uses the existing finite host-free command policy; performs no project reads or host I/O.
pub fn execute_generic_collection_package_native_image(
    image: &[u8],
) -> Result<GenericVmOutcome, String> {
    let program = decode_generic_collection_package_native_image(image)?;
    execute_generic_collection_package(&program, generic_command_runtime_policy())
        .map_err(|e| diagnostic_message!("{e:?}"))
}

/// Recheck a data image, accepting only exact word 2047 and finite package limits.
pub fn decode_generic_data_package_native_image(
    image: &[u8],
) -> Result<CompiledGenericPackage, String> {
    decode_package_native_mode(image, 2)
}
/// Execute data operations under finite host-free command policy, without project files.
pub fn execute_generic_data_package_native_image(image: &[u8]) -> Result<GenericVmOutcome, String> {
    let program = decode_generic_data_package_native_image(image)?;
    execute_generic_data_package(&program, generic_command_runtime_policy())
        .map_err(|e| diagnostic_message!("{e:?}"))
}
