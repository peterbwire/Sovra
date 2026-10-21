//! Create the first console project without changing Sovra source semantics.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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
    create_with(path, name, |_| Ok(()))
}

fn create_with(
    path: &Path,
    name: &str,
    after_reserve: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), CreateError> {
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
    let mut created = Vec::new();
    let result = after_reserve(path)
        .and_then(|()| write_new(&path.join("main.svr"), MAIN, &mut created))
        .and_then(|()| write_new(&path.join("sovra.toml"), &manifest, &mut created));
    if let Err(error) = result {
        // A create_new collision belongs to somebody else. Roll back only files
        // for which this operation successfully reserved a new file handle.
        for owned in created.into_iter().rev() {
            let _ = fs::remove_file(owned);
        }
        let _ = fs::remove_dir(path);
        return Err(CreateError::Io(error));
    }
    Ok(())
}

fn write_new(path: &Path, text: &str, created: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    created.push(path.to_path_buf());
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

    #[test]
    fn creation_failure_preserves_files_created_by_another_writer() {
        for collided in ["main.svr", "sovra.toml"] {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("svr-new-collision-{}-{unique}", std::process::id()));
            fs::create_dir(&root).unwrap();
            let project = root.join("app");
            let result = create_with(&project, "app", |reserved| {
                fs::write(reserved.join(collided), "another writer's file")
            });
            let retained = fs::read_to_string(project.join(collided));
            let own_main_exists = collided == "sovra.toml" && project.join("main.svr").exists();
            fs::remove_dir_all(root).unwrap();
            assert!(result.is_err());
            assert_eq!(retained.unwrap(), "another writer's file", "{collided}");
            assert!(
                !own_main_exists,
                "our completed source should be rolled back"
            );
        }
    }

    #[test]
    fn early_failure_removes_only_an_empty_reserved_directory() {
        for foreign in [false, true] {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("svr-new-rollback-{}-{unique}", std::process::id()));
            fs::create_dir(&root).unwrap();
            let project = root.join("app");
            let result = create_with(&project, "app", |reserved| {
                if foreign {
                    fs::write(reserved.join("notes.txt"), "keep notes")?;
                }
                Err(std::io::ErrorKind::PermissionDenied.into())
            });
            let retained_directory = project.exists();
            let notes = if foreign {
                Some(fs::read_to_string(project.join("notes.txt")).unwrap())
            } else {
                None
            };
            fs::remove_dir_all(root).unwrap();
            assert!(matches!(result, Err(CreateError::Io(_))));
            assert_eq!(retained_directory, foreign);
            assert_eq!(notes.as_deref(), foreign.then_some("keep notes"));
        }
    }
}
