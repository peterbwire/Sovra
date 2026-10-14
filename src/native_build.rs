//! Host-native executable linking behind the approved CLI contract.

mod sha256;

use crate::compiler::{ir::IrProgram, native_image};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

const RUNNER: &str = include_str!("../toolchain/runtime-source/runner.rs");

pub(crate) fn build(program: &IrProgram, output: &Path) -> Result<PathBuf, String> {
    let entry = program
        .functions
        .iter()
        .find(|f| f.name == "main")
        .ok_or("entry function `main` was not found")?;
    if !entry.parameters.is_empty() {
        return Err("entry function `main` cannot declare parameters".into());
    }
    refuse_existing(output)?;
    let image = native_image::encode(program)?;
    native_image::decode(&image)?;
    let runtime = discover_runtime()?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let parent = PathBuf::from(plain_path(&parent)?);
    let destination = parent.join(output.file_name().ok_or("output must name a file")?);
    let stage = reserve_stage(&parent)?;
    let result = link(&image, &runtime, &stage, &destination);
    let cleanup = fs::remove_dir_all(&stage);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(format!(
            "executable published, but staging cleanup failed: {error}"
        )),
        (Ok(()), Ok(())) => Ok(destination),
    }
}

fn refuse_existing(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(format!("output already exists: {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn reserve_stage(parent: &Path) -> Result<PathBuf, String> {
    for attempt in 0..128 {
        let stage = parent.join(format!(".sovra-exe-stage-{}-{attempt}", std::process::id()));
        match fs::create_dir(&stage) {
            Ok(()) => return Ok(stage),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("cannot reserve native build staging".into())
}

fn discover_runtime() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let installed = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("cannot locate installed runtime source")?
        .join("runtime-source");
    if installed.exists() {
        validate_payload(&installed)?;
        return fs::canonicalize(installed).map_err(|e| e.to_string());
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Installed copies must never silently fall back to the builder's checkout.
    if executable.starts_with(repository.join("target")) && repository.join("src/lib.rs").is_file()
    {
        return fs::canonicalize(repository).map_err(|e| e.to_string());
    }
    Err("native runtime-source payload is missing beside the installed bin directory".into())
}

fn validate_payload(root: &Path) -> Result<(), String> {
    let mut actual = BTreeSet::new();
    collect_files(root, root, &mut actual)?;
    if fs::metadata(root.join("payload.sha256"))
        .map_err(|e| e.to_string())?
        .len()
        > 16 * 1024 * 1024
    {
        return Err("runtime payload manifest exceeds size limit".into());
    }
    let manifest = fs::read_to_string(root.join("payload.sha256"))
        .map_err(|e| format!("cannot read runtime payload manifest: {e}"))?;
    let mut lines = manifest.lines();
    if lines.next() != Some(format!("SVR-RUNTIME 1 {}", env!("CARGO_PKG_VERSION")).as_str()) {
        return Err("unsupported or version-mismatched runtime payload".into());
    }
    let mut expected = BTreeSet::new();
    for line in lines {
        let (digest, relative) = line
            .split_once("  ")
            .ok_or("invalid runtime payload manifest")?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || relative.contains('\\')
            || relative.is_empty()
            || !Path::new(relative)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            || !expected.insert(relative.to_owned())
        {
            return Err("invalid runtime payload file entry".into());
        }
        if fs::metadata(root.join(relative))
            .map_err(|e| e.to_string())?
            .len()
            > 16 * 1024 * 1024
        {
            return Err(format!(
                "runtime payload file exceeds size limit: {relative}"
            ));
        }
        let bytes = fs::read(root.join(relative))
            .map_err(|e| format!("runtime payload `{relative}`: {e}"))?;
        if sha256::digest(&bytes) != digest {
            return Err(format!("runtime payload hash mismatch: {relative}"));
        }
    }
    actual.remove("payload.sha256");
    actual.remove("payload.json");
    if actual != expected {
        return Err("runtime payload file list mismatch".into());
    }
    for required in [
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "toolchain/runtime-source/runner.rs",
    ] {
        if !expected.contains(required) {
            return Err(format!("runtime payload missing {required}"));
        }
    }
    let cargo = fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?;
    if !cargo
        .lines()
        .any(|line| line == format!("version = \"{}\"", env!("CARGO_PKG_VERSION")))
    {
        return Err("runtime Cargo package version does not match compiler".into());
    }
    Ok(())
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    if directory
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .components()
        .count()
        > 64
        || files.len() > 10_000
    {
        return Err("runtime payload exceeds structural limits".into());
    }
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("runtime payload contains a link".into());
        }
        if kind.is_dir() {
            collect_files(root, &entry.path(), files)?;
        } else if kind.is_file() {
            if files.len() >= 10_000 {
                return Err("runtime payload exceeds structural limits".into());
            }
            files.insert(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("runtime payload path is not UTF-8")?
                    .replace('\\', "/"),
            );
        } else {
            return Err("runtime payload contains a non-file entry".into());
        }
    }
    Ok(())
}

fn link(image: &[u8], runtime: &Path, stage: &Path, output: &Path) -> Result<(), String> {
    fs::create_dir(stage.join("src")).map_err(|e| e.to_string())?;
    fs::write(stage.join("program.svir"), image).map_err(|e| e.to_string())?;
    fs::write(stage.join("src/main.rs"), RUNNER).map_err(|e| e.to_string())?;
    let runtime_path = dependency_path(runtime)?;
    fs::write(stage.join("Cargo.toml"), format!(
        "[package]\nname = \"sovra-application\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\nsovra = {{ path = {runtime_path} }}\n"
    )).map_err(|e| e.to_string())?;
    let result = Command::new("cargo")
        .args(["build", "--offline", "--release", "--manifest-path"])
        .arg(stage.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(stage.join("target"))
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .map_err(|e| format!("cannot launch Cargo (Rust and a host linker are required): {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "native linking failed: {}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    let binary = stage.join("target/release").join(if cfg!(windows) {
        "sovra-application.exe"
    } else {
        "sovra-application"
    });
    // A hard link publishes atomically and fails if any output appeared during linking.
    fs::hard_link(binary, output)
        .map_err(|e| format!("cannot publish executable without overwriting: {e}"))
}

fn dependency_path(path: &Path) -> Result<String, String> {
    Ok(format!("{:?}", plain_path(path)?.replace('\\', "/")))
}

fn plain_path(path: &Path) -> Result<String, String> {
    let raw = path.to_str().ok_or("runtime path is not UTF-8")?;
    let normalized = if let Some(unc) = raw.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{unc}")
    } else {
        raw.strip_prefix("\\\\?\\").unwrap_or(raw).to_owned()
    };
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_file_budget_rejects_the_next_file_at_the_limit() {
        let root = std::env::temp_dir().join(format!("svr-file-budget-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("next"), "next").unwrap();
        let mut files = (0..10_000)
            .map(|index| format!("visited-{index}"))
            .collect();
        assert!(collect_files(&root, &root, &mut files).is_err());
        assert_eq!(files.len(), 10_000);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cargo_dependency_paths_remove_windows_verbatim_prefixes() {
        for (path, expected) in [
            (r"\\?\C:\runtime source", "\"C:/runtime source\""),
            (
                r"\\?\UNC\server\share\runtime",
                "\"//server/share/runtime\"",
            ),
            ("/tmp/runtime source", "\"/tmp/runtime source\""),
        ] {
            assert_eq!(dependency_path(Path::new(path)).unwrap(), expected);
        }
    }
    #[test]
    fn rejects_modified_missing_extra_and_version_mismatched_payloads() {
        let root =
            std::env::temp_dir().join(format!("svr-payload-validator-{}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("toolchain/runtime-source")).unwrap();
        let files = [
            "Cargo.toml",
            "Cargo.lock",
            "src/lib.rs",
            "toolchain/runtime-source/runner.rs",
        ];
        let mut manifest = format!("SVR-RUNTIME 1 {}\n", env!("CARGO_PKG_VERSION"));
        for path in files {
            let data = if path == "Cargo.toml" {
                format!("version = \"{}\"\n", env!("CARGO_PKG_VERSION"))
            } else {
                "snapshot\n".into()
            };
            fs::write(root.join(path), &data).unwrap();
            manifest.push_str(&format!("{}  {path}\n", sha256::digest(data.as_bytes())));
        }
        fs::write(root.join("payload.sha256"), &manifest).unwrap();
        assert!(validate_payload(&root).is_ok());
        fs::write(root.join("src/lib.rs"), "modified").unwrap();
        assert!(validate_payload(&root)
            .unwrap_err()
            .contains("hash mismatch"));
        fs::write(root.join("src/lib.rs"), "snapshot\n").unwrap();
        fs::write(root.join("extra"), "extra").unwrap();
        assert!(validate_payload(&root).unwrap_err().contains("file list"));
        fs::remove_file(root.join("extra")).unwrap();
        fs::write(
            root.join("payload.sha256"),
            manifest.replacen("SVR-RUNTIME 1", "SVR-RUNTIME 2", 1),
        )
        .unwrap();
        assert!(validate_payload(&root)
            .unwrap_err()
            .contains("version-mismatched"));
        fs::write(root.join("payload.sha256"), &manifest).unwrap();
        fs::remove_file(root.join("Cargo.lock")).unwrap();
        assert!(validate_payload(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn publication_never_overwrites_an_existing_file() {
        let root = std::env::temp_dir().join(format!("svr-publish-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let output = root.join("output");
        fs::write(&output, "keep").unwrap();
        let binary = root.join("binary");
        fs::write(&binary, "replace").unwrap();
        assert!(fs::hard_link(&binary, &output).is_err());
        assert!(refuse_existing(&output).is_err());
        assert_eq!(fs::read_to_string(output).unwrap(), "keep");
        fs::remove_dir_all(root).unwrap();
    }
}
