//! Bounded UTF-8 file operations used by the process runtime host.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
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
