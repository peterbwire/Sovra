//! Bounded UTF-8 file operations used by the process runtime host.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use super::stdlib::MAX_TEXT_FILE_BYTES;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Result of one bounded UTF-8 file read by a runtime host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRead {
    /// Whether reading succeeded.
    pub ok: bool,
    /// File contents on success, otherwise empty.
    pub text: String,
    /// Stable failure category, or empty on success.
    pub error: String,
}

/// Result of one bounded UTF-8 file replacement by a runtime host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextWrite {
    /// Whether replacement succeeded.
    pub ok: bool,
    /// Stable failure category, or empty on success.
    pub error: String,
}

fn category(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        std::io::ErrorKind::InvalidInput => "invalid_path",
        _ => "io",
    }
}

fn valid_path(path: &str) -> bool {
    !path.is_empty() && !path.contains('\0')
}

pub(crate) fn read_text(path: &str) -> TextRead {
    let failure = |error: &str| TextRead {
        ok: false,
        text: String::new(),
        error: error.to_owned(),
    };
    if !valid_path(path) {
        return failure("invalid_path");
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return failure(category(&error)),
    };
    let mut bytes = Vec::new();
    if let Err(error) = file
        .take((MAX_TEXT_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
    {
        return failure(category(&error));
    }
    if bytes.len() > MAX_TEXT_FILE_BYTES {
        return failure("too_large");
    }
    match String::from_utf8(bytes) {
        Ok(text) => TextRead {
            ok: true,
            text,
            error: String::new(),
        },
        Err(_) => failure("invalid_utf8"),
    }
}

pub(crate) fn write_text(path: &str, text: &str) -> TextWrite {
    write_text_with(path, text, || Ok(()))
}

pub(crate) fn write_text_if(
    path: &str,
    expected_exists: bool,
    expected_text: &str,
    replacement: &str,
) -> TextWrite {
    write_text_if_with(path, expected_exists, expected_text, replacement, || Ok(()))
}

struct WriteMarker {
    path: PathBuf,
    file: Option<fs::File>,
}

impl Drop for WriteMarker {
    fn drop(&mut self) {
        drop(self.file.take());
        // A retained marker fails closed. Cleanup cannot undo a committed write.
        let _ = fs::remove_file(&self.path);
    }
}

fn write_text_if_with(
    path: &str,
    expected_exists: bool,
    expected_text: &str,
    replacement: &str,
    before_rename: impl FnOnce() -> std::io::Result<()>,
) -> TextWrite {
    let failure = |error: &str| TextWrite {
        ok: false,
        error: error.to_owned(),
    };
    let destination = Path::new(path);
    if !valid_path(path) || destination.file_name().is_none() {
        return failure("invalid_path");
    }
    if expected_text.len() > MAX_TEXT_FILE_BYTES || replacement.len() > MAX_TEXT_FILE_BYTES {
        return failure("too_large");
    }
    if !expected_exists && !expected_text.is_empty() {
        return failure("invalid_snapshot");
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = match fs::canonicalize(parent) {
        Ok(parent) => parent,
        Err(error) => return failure(category(&error)),
    };
    let destination = parent.join(destination.file_name().unwrap());
    let Some(destination_text) = destination.to_str() else {
        return failure("invalid_path");
    };
    let mut marker_name = destination.file_name().unwrap().to_os_string();
    marker_name.push(".svr-write-lock");
    let marker_path = parent.join(marker_name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = match options.open(&marker_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return failure("busy"),
        Err(error) => return failure(category(&error)),
    };
    let _marker = WriteMarker {
        path: marker_path,
        file: Some(file),
    };
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if !metadata.file_type().is_file() => return failure("invalid_path"),
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return failure(category(&error)),
    }
    let current = read_text(destination_text);
    if current.ok {
        if !expected_exists || current.text != expected_text {
            return failure("conflict");
        }
    } else if current.error == "not_found" {
        if expected_exists {
            return failure("conflict");
        }
    } else {
        return failure(&current.error);
    }
    write_text_with(destination_text, replacement, before_rename)
}

fn write_text_with(
    path: &str,
    text: &str,
    before_rename: impl FnOnce() -> std::io::Result<()>,
) -> TextWrite {
    let failure = |error: &str| TextWrite {
        ok: false,
        error: error.to_owned(),
    };
    if !valid_path(path) || Path::new(path).file_name().is_none() {
        return failure("invalid_path");
    }
    if text.len() > MAX_TEXT_FILE_BYTES {
        return failure("too_large");
    }
    let destination = Path::new(path);
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = None;
    for _ in 0..16 {
        let next = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let mut name = destination.file_name().unwrap().to_os_string();
        name.push(format!(".svr-tmp-{}-{next}", std::process::id()));
        let candidate: PathBuf = parent.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        match options.open(&candidate) {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return failure(category(&error)),
        }
    }
    let Some((temporary_path, mut file)) = temporary else {
        return failure("io");
    };
    let result = file
        .write_all(text.as_bytes())
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .and_then(|_| before_rename());
    drop(file);
    let result = result.and_then(|_| fs::rename(&temporary_path, destination));
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return failure(category(&error));
    }
    TextWrite {
        ok: true,
        error: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conditional_directory() -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "svr-conditional-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        directory
    }

    #[test]
    fn conditional_writes_compare_existence_bytes_and_preserve_foreign_markers() {
        let directory = conditional_directory();
        let path = directory.join("state");
        let path = path.to_str().unwrap();
        assert_eq!(write_text_if(path, true, "", "x").error, "conflict");
        assert!(write_text_if(path, false, "", "\u{feff}猫").ok);
        assert_eq!(write_text_if(path, false, "", "lost").error, "conflict");
        assert_eq!(write_text_if(path, true, "猫", "lost").error, "conflict");
        assert!(write_text_if(path, true, "\u{feff}猫", "").ok);
        let marker = format!("{path}.svr-write-lock");
        fs::write(&marker, "foreign owner").unwrap();
        assert_eq!(write_text_if(path, true, "", "lost").error, "busy");
        assert_eq!(fs::read_to_string(&marker).unwrap(), "foreign owner");
        assert_eq!(fs::read_to_string(path).unwrap(), "");
        // The existing unconditional API is deliberately noncooperating.
        assert!(write_text(path, "unconditional").ok);
        fs::remove_file(marker).unwrap();
        assert!(write_text_if(path, true, "unconditional", "new").ok);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn conditional_validation_and_faults_never_replace_existing_data() {
        let directory = conditional_directory();
        let path = directory.join("state");
        let path = path.to_str().unwrap();
        fs::write(path, "old").unwrap();
        assert_eq!(
            write_text_if(path, false, "nonempty", "x").error,
            "invalid_snapshot"
        );
        assert_eq!(write_text_if("", false, "", "x").error, "invalid_path");
        assert_eq!(
            write_text_if(path, true, &"x".repeat(MAX_TEXT_FILE_BYTES + 1), "x").error,
            "too_large"
        );
        assert_eq!(
            write_text_if(path, true, "old", &"x".repeat(MAX_TEXT_FILE_BYTES + 1)).error,
            "too_large"
        );
        let marker = format!("{path}.svr-write-lock");
        assert!(!Path::new(&marker).exists());
        let failed = write_text_if_with(path, true, "old", "lost", || {
            assert!(Path::new(&marker).exists());
            Err(std::io::Error::other("injected pre-rename failure"))
        });
        assert_eq!(failed.error, "io");
        assert_eq!(fs::read_to_string(path).unwrap(), "old");
        assert!(!Path::new(&marker).exists());
        fs::write(path, [255]).unwrap();
        assert_eq!(write_text_if(path, true, "", "lost").error, "invalid_utf8");
        assert_eq!(fs::read(path).unwrap(), [255]);
        fs::remove_file(path).unwrap();
        assert_eq!(
            write_text_if(directory.to_str().unwrap(), false, "", "x").error,
            "invalid_path"
        );
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn conditional_competing_writers_commit_only_one_snapshot() {
        let directory = conditional_directory();
        let path = directory.join("state");
        fs::write(&path, "base").unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let writers: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|replacement| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    write_text_if(path.to_str().unwrap(), true, "base", replacement)
                })
            })
            .collect();
        let results: Vec<_> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.ok).count(), 1);
        assert!(results
            .iter()
            .filter(|result| !result.ok)
            .all(|result| result.error == "busy" || result.error == "conflict"));
        let current = fs::read_to_string(&path).unwrap();
        assert!(write_text_if(path.to_str().unwrap(), true, &current, "reused").ok);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn failed_pre_rename_write_keeps_existing_file() {
        let path = std::env::temp_dir().join(format!(
            "svr-file-fault-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, "old").unwrap();
        let result = write_text_with(path.to_str().unwrap(), "new", || {
            Err(std::io::Error::other("injected"))
        });
        assert_eq!(result.error, "io");
        assert_eq!(fs::read_to_string(&path).unwrap(), "old");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn bounded_text_results_distinguish_empty_invalid_and_oversized_content() {
        let path = std::env::temp_dir().join(format!(
            "svr-file-bounds-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let path = path.to_str().unwrap();
        assert_eq!(read_text("").error, "invalid_path");
        assert_eq!(write_text("", "x").error, "invalid_path");
        assert!(write_text(path, "").ok);
        assert_eq!(read_text(path).text, "");
        assert_eq!(
            write_text(path, &"a".repeat(MAX_TEXT_FILE_BYTES + 1)).error,
            "too_large"
        );
        assert_eq!(read_text(path).text, "");
        fs::write(path, [0xff]).unwrap();
        assert_eq!(read_text(path).error, "invalid_utf8");
        fs::remove_file(path).unwrap();
    }
}
