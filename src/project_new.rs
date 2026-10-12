//! Create the first console project without changing Sovra source semantics.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const MAIN: &str = include_str!("../toolchain/templates/console/main.svr");
const MANIFEST: &str = include_str!("../toolchain/templates/console/sovra.toml");

/// A project creation failure, separated from invocation errors by the CLI.
#[derive(Debug)]
pub(crate) enum CreateError {
    /// The target already exists, including symlinks and empty directories.
    Exists,
    /// An I/O operation failed while creating the project.
    Io(std::io::Error),
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exists => formatter.write_str("project path already exists"),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

/// Return the valid project name derived from the last path component.
pub(crate) fn project_name(path: &Path) -> Result<&str, &'static str> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("project path must end in a valid directory name")?;
    let mut bytes = name.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(
            "project name must start with a letter and contain only ASCII letters, digits, _ or -",
        );
    }
    Ok(name)
}

/// Create a new console project at a path that does not already exist.
pub(crate) fn create(path: &Path, name: &str) -> Result<(), CreateError> {
    if fs::symlink_metadata(path).is_ok() {
        return Err(CreateError::Exists);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(CreateError::Io)?;
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(CreateError::Exists)
        }
        Err(error) => return Err(CreateError::Io(error)),
    }
    let manifest = MANIFEST.replace("name = \"console-app\"", &format!("name = \"{name}\""));
    let result = write_new(&path.join("main.svr"), MAIN)
        .and_then(|()| write_new(&path.join("sovra.toml"), &manifest));
    if let Err(error) = result {
        let _ = fs::remove_file(path.join("main.svr"));
        let _ = fs::remove_file(path.join("sovra.toml"));
        let _ = fs::remove_dir(path);
        return Err(CreateError::Io(error));
    }
    Ok(())
}

fn write_new(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_existing_script_name_rule() {
        for name in ["hello", "A2", "task_manager", "task-manager"] {
            assert_eq!(project_name(Path::new(name)), Ok(name));
        }
        for name in ["2task", "a.b", "café", "a space", "."] {
            assert!(project_name(Path::new(name)).is_err(), "{name}");
        }
    }
}
