//! Strict bounded validation of the existing Windows package integrity format.

use crate::native_build::sha256;

const REQUIRED: &[&str] = &[
    "bin/svr.exe",
    "bin/svr-new.ps1",
    "templates/console/main.svr",
    "templates/console/sovra.toml",
    "docs/spec.md",
    "examples/local-packages/app/main.svr",
    "samples/task-manager/main.svr",
    "README.md",
    "verify-install.ps1",
    "imports.json",
    "LICENSE",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "runtime-source/payload.json",
    "runtime-source/payload.sha256",
    "runtime-source/Cargo.toml",
    "runtime-source/Cargo.lock",
    "runtime-source/src/lib.rs",
    "runtime-source/examples/native_builder.rs",
    "runtime-source/toolchain/templates/console/main.svr",
    "runtime-source/toolchain/templates/console/sovra.toml",
];
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

#[derive(Debug)]
enum Json {
    String(String),
    One,
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn ws(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
        {
            self.at += 1;
        }
    }
    fn take(&mut self, byte: u8) -> Result<(), String> {
        self.ws();
        if self.bytes.get(self.at) != Some(&byte) {
            return Err("invalid integrity JSON".into());
        }
        self.at += 1;
        Ok(())
    }
    fn string(&mut self) -> Result<String, String> {
        self.take(b'"')?;
        let mut output = String::new();
        loop {
            let byte = *self.bytes.get(self.at).ok_or("unterminated JSON string")?;
            self.at += 1;
            match byte {
                b'"' => return Ok(output),
                b'\\' => {
                    let escaped = *self.bytes.get(self.at).ok_or("invalid JSON escape")?;
                    self.at += 1;
                    output.push(match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let high = self.hex()?;
                            let scalar = if (0xd800..=0xdbff).contains(&high) {
                                // No whitespace is permitted between surrogate escapes.
                                if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
                                    return Err("invalid JSON surrogate".into());
                                }
                                self.at += 2;
                                let low = self.hex()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err("invalid JSON surrogate".into());
                                }
                                0x10000 + (high - 0xd800) * 1024 + low - 0xdc00
                            } else {
                                high
                            };
                            char::from_u32(scalar).ok_or("invalid JSON scalar")?
                        }
                        _ => return Err("invalid JSON escape".into()),
                    });
                }
                0..=31 => return Err("JSON control character".into()),
                32..=127 => output.push(char::from(byte)),
                _ => {
                    self.at -= 1;
                    let width = match byte {
                        0xc2..=0xdf => 2,
                        0xe0..=0xef => 3,
                        0xf0..=0xf4 => 4,
                        _ => return Err("invalid JSON UTF-8".into()),
                    };
                    let encoded = self
                        .bytes
                        .get(self.at..self.at + width)
                        .ok_or("short JSON UTF-8")?;
                    let text = std::str::from_utf8(encoded).map_err(|_| "invalid JSON UTF-8")?;
                    let character = text.chars().next().ok_or("missing JSON character")?;
                    self.at += character.len_utf8();
                    output.push(character);
                }
            }
        }
    }
    fn hex(&mut self) -> Result<u32, String> {
        let bytes = self
            .bytes
            .get(self.at..self.at + 4)
            .ok_or("short JSON escape")?;
        let text = std::str::from_utf8(bytes).map_err(|_| "invalid JSON escape")?;
        if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid JSON escape".into());
        }
        self.at += 4;
        u32::from_str_radix(text, 16).map_err(|_| "invalid JSON escape".into())
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > 8 {
            return Err("integrity JSON nesting limit".into());
        }
        self.ws();
        match self.bytes.get(self.at) {
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b'1') => {
                self.at += 1;
                Ok(Json::One)
            }
            Some(b'[') => {
                self.at += 1;
                let mut values = Vec::new();
                self.ws();
                if self.bytes.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Json::Array(values));
                }
                loop {
                    if values.len() >= 10000 {
                        return Err("integrity entry limit".into());
                    }
                    values.push(self.value(depth + 1)?);
                    self.ws();
                    if self.bytes.get(self.at) == Some(&b']') {
                        self.at += 1;
                        break;
                    }
                    self.take(b',')?;
                }
                Ok(Json::Array(values))
            }
            Some(b'{') => {
                self.at += 1;
                let mut values = BTreeMap::new();
                self.ws();
                if self.bytes.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Json::Object(values));
                }
                loop {
                    if values.len() >= 8 {
                        return Err("integrity object field limit".into());
                    }
                    let key = self.string()?;
                    self.take(b':')?;
                    let value = self.value(depth + 1)?;
                    if values.insert(key, value).is_some() {
                        return Err("duplicate integrity JSON key".into());
                    }
                    self.ws();
                    if self.bytes.get(self.at) == Some(&b'}') {
                        self.at += 1;
                        break;
                    }
                    self.take(b',')?;
                }
                Ok(Json::Object(values))
            }
            _ => Err("unsupported integrity JSON value".into()),
        }
    }
}

pub(super) fn metadata(path: &Path) -> Result<fs::Metadata, String> {
    let data = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let link = data.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if data.file_attributes() & 0x400 != 0 {
            return Err("installation contains a link/reparse point".into());
        }
    }
    if link {
        return Err("installation contains a link/reparse point".into());
    }
    Ok(data)
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if !metadata(path)?.is_file() {
        return Err("installation entry is not a regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("installation file exceeds size limit".into());
    }
    Ok(bytes)
}

fn inventory(
    root: &Path,
    directory: &Path,
    depth: usize,
    budget: &mut usize,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    if depth > 64 || *budget >= 10000 {
        return Err("installation structural limit".into());
    }
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        *budget += 1;
        if *budget > 10000 {
            return Err("installation structural limit".into());
        }
        let data = metadata(&path)?;
        if data.is_dir() {
            inventory(root, &path, depth + 1, budget, files)?;
        } else if data.is_file() {
            let name = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non-UTF-8 installation path")?
                .replace('\\', "/");
            files.insert(name);
        } else {
            return Err("installation contains a non-file entry".into());
        }
    }
    Ok(())
}

fn entries(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let mut parser = Reader { bytes, at: 0 };
    let Json::Object(mut root) = parser.value(0)? else {
        return Err("integrity manifest must be an object".into());
    };
    parser.ws();
    if parser.at != bytes.len() {
        return Err("trailing integrity JSON".into());
    }
    if !matches!(root.remove("schema"), Some(Json::One)) {
        return Err("unsupported integrity schema".into());
    }
    let Some(Json::Array(files)) = root.remove("files") else {
        return Err("integrity files must be an array".into());
    };
    if !root.is_empty() {
        return Err("unknown integrity field".into());
    }
    let mut result = BTreeMap::new();
    let mut insensitive = BTreeSet::new();
    for entry in files {
        let Json::Object(mut entry) = entry else {
            return Err("invalid integrity entry".into());
        };
        let Some(Json::String(path)) = entry.remove("path") else {
            return Err("missing integrity path".into());
        };
        let Some(Json::String(hash)) = entry.remove("sha256") else {
            return Err("missing integrity hash".into());
        };
        if !entry.is_empty()
            || path.is_empty()
            || path.contains(['\\', ':'])
            || path.starts_with('/')
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with([' ', '.']))
            || !Path::new(&path)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            || hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !insensitive.insert(path.to_lowercase())
            || result.insert(path, hash).is_some()
        {
            return Err("invalid or duplicate integrity entry".into());
        }
    }
    Ok(result)
}

pub(super) fn validate(root: &Path) -> Result<(), String> {
    if !metadata(root)?.is_dir() {
        return Err("installation root is not a directory".into());
    }
    let expected = entries(&read(&root.join("integrity.json"), 16 * 1024 * 1024)?)?;
    let mut actual = BTreeSet::new();
    inventory(root, root, 0, &mut 0, &mut actual)?;
    actual.remove("integrity.json");
    if actual != expected.keys().cloned().collect() {
        return Err("installation file inventory mismatch".into());
    }
    for required in REQUIRED {
        if !expected.contains_key(*required) {
            return Err(format!("installation missing {required}"));
        }
    }
    let mut total = 0usize;
    for (path, digest) in expected {
        let bytes = read(&root.join(&path), 64 * 1024 * 1024)?;
        total = total.saturating_add(bytes.len());
        if total > 512 * 1024 * 1024 {
            return Err("installation total size limit".into());
        }
        if sha256::digest(&bytes) != digest {
            return Err(format!("installation hash mismatch: {path}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_duplicate_and_malformed_manifest_entries() {
        let hash = "0".repeat(64);
        for path in [
            "../escape",
            "C:/escape",
            "/root",
            "a//b",
            "a/./b",
            "a/../b",
            "a\\b",
            "file.",
        ] {
            let mut json = String::new();
            crate::compiler::check_report::push_string(&mut json, path);
            assert!(entries(
                format!("{{\"schema\":1,\"files\":[{{\"path\":{json},\"sha256\":\"{hash}\"}}]}}")
                    .as_bytes()
            )
            .is_err());
        }
        for json in [
            "{\"schema\":1,\"schema\":1,\"files\":[]}",
            "{\"schema\":1,\"files\":[],}",
            "{\"schema\":1,\"files\":[]} trailing",
        ] {
            assert!(entries(json.as_bytes()).is_err());
        }
        let item = format!("{{\"path\":\"a\",\"sha256\":\"{hash}\"}}");
        assert!(entries(format!("{{\"schema\":1,\"files\":[{item},{item}]}}").as_bytes()).is_err());
        assert!(entries(b"{\"schema\":1,\"files\":[]}").unwrap().is_empty());
    }
    #[test]
    fn decodes_unicode_and_rejects_invalid_escapes() {
        let mut reader = Reader {
            bytes: br#""\uD83D\uDE80\u0027""#,
            at: 0,
        };
        assert_eq!(reader.string().unwrap(), "🚀'");
        for bytes in [br#""\uD800""#.as_slice(), br#""\uDC00""#, br#""\x01""#] {
            assert!(Reader { bytes, at: 0 }.string().is_err());
        }
    }

    #[test]
    fn validates_real_inventory_and_rejects_changed_missing_and_extra_files() {
        let root = std::env::temp_dir().join(format!(
            "sovra-integrity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let result = std::panic::catch_unwind(|| {
            let mut manifest = String::from("{\"schema\":1,\"files\":[");
            for (index, path) in REQUIRED.iter().enumerate() {
                let file = root.join(path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(&file, b"original").unwrap();
                if index > 0 {
                    manifest.push(',');
                }
                manifest.push_str(&format!(
                    "{{\"path\":\"{path}\",\"sha256\":\"{}\"}}",
                    sha256::digest(b"original")
                ));
            }
            manifest.push_str("]}");
            fs::write(root.join("integrity.json"), &manifest).unwrap();
            validate(&root).unwrap();
            fs::write(root.join("README.md"), "changed").unwrap();
            assert!(validate(&root).unwrap_err().contains("hash mismatch"));
            fs::write(root.join("README.md"), "original").unwrap();
            fs::write(root.join("extra"), "extra").unwrap();
            assert!(validate(&root).unwrap_err().contains("inventory"));
            fs::remove_file(root.join("extra")).unwrap();
            fs::remove_file(root.join("README.md")).unwrap();
            assert!(validate(&root).unwrap_err().contains("inventory"));
            fs::write(root.join("README.md"), "original").unwrap();
            fs::write(
                root.join("integrity.json"),
                manifest.replace("\"schema\":1", "\"schema\":2"),
            )
            .unwrap();
            assert!(validate(&root).is_err());
            #[cfg(unix)]
            {
                fs::write(root.join("integrity.json"), &manifest).unwrap();
                fs::remove_file(root.join("README.md")).unwrap();
                std::os::unix::fs::symlink(root.join("LICENSE"), root.join("README.md")).unwrap();
                assert!(validate(&root).unwrap_err().contains("link"));
            }
        });
        fs::remove_dir_all(&root).unwrap();
        if let Err(error) = result {
            std::panic::resume_unwind(error);
        }
    }
}
