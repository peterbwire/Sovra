//! Bounded snapshots and guarded atomic replacement for formatter-owned writes.

use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) const INPUT_LIMIT: usize = 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(super) struct Snapshot {
    pub text: String,
    path: PathBuf,
    metadata: Metadata,
    parent: PathBuf,
}

fn regular(path: &Path) -> Result<Metadata, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("formatter refuses links/reparse points".into());
        }
    }
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("formatter requires a regular file, not a link".into());
    }
    Ok(metadata)
}

fn same(left: &Metadata, right: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if left.dev() != right.dev()
            || left.ino() != right.ino()
            || left.mode() != right.mode()
            || left.uid() != right.uid()
            || left.gid() != right.gid()
        {
            return false;
        }
    }
    left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
        && left.created().ok() == right.created().ok()
        && left.permissions() == right.permissions()
}

fn bounded(file: File) -> Result<String, String> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve(INPUT_LIMIT + 1)
        .map_err(|_| "cannot allocate source snapshot")?;
    file.take(INPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > INPUT_LIMIT {
        return Err("formatter source exceeds 1 MiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "formatter source is not UTF-8".into())
}

impl Snapshot {
    pub fn read(path: &Path) -> Result<Self, String> {
        let before = regular(path)?;
        if before.len() > INPUT_LIMIT as u64 {
            return Err("formatter source exceeds 1 MiB".into());
        }
        let parent = fs::canonicalize(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .map_err(|e| e.to_string())?;
        let file = File::open(path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let text = bounded(file)?;
        if !same(&before, &metadata) || !same(&metadata, &regular(path)?) {
            return Err("source changed while reading".into());
        }
        Ok(Self {
            text,
            path: path.to_owned(),
            metadata,
            parent,
        })
    }

    pub fn writable(&self) -> Result<(), String> {
        if self.metadata.permissions().readonly() {
            return Err("formatter refuses to write a read-only file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if self.metadata.nlink() != 1 || self.metadata.mode() & 0o7000 != 0 {
                return Err(
                    "formatter cannot preserve hard links or special file permissions".into(),
                );
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if self.metadata.file_attributes() & !(0x20 | 0x80) != 0 {
                return Err("formatter cannot preserve special Windows file attributes".into());
            }
        }
        Ok(())
    }

    fn unchanged(&self) -> Result<(), String> {
        let parent = fs::canonicalize(
            self.path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .map_err(|e| e.to_string())?;
        if parent != self.parent
            || !same(&self.metadata, &regular(&self.path)?)
            || bounded(File::open(&self.path).map_err(|e| e.to_string())?)? != self.text
        {
            return Err("source changed before replacement".into());
        }
        Ok(())
    }

    pub fn replace(&self, formatted: &str) -> Result<(), String> {
        self.replace_with(formatted, |_| Ok(()))
    }

    fn replace_with(
        &self,
        formatted: &str,
        before_commit: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        self.writable()?;
        if formatted == self.text {
            return self.unchanged();
        }
        let mut reserved = None;
        for _ in 0..128 {
            let path = self.parent.join(format!(
                ".svr-fmt-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => {
                    reserved = Some((Temporary::new(path), file));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        let (mut temporary, mut file) = reserved.ok_or("cannot reserve formatter staging file")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let created = file.metadata().map_err(|e| e.to_string())?;
            if created.uid() != self.metadata.uid() || created.gid() != self.metadata.gid() {
                return Err("formatter cannot preserve file ownership/group".into());
            }
        }
        file.write_all(formatted.as_bytes())
            .map_err(|e| e.to_string())?;
        file.set_permissions(self.metadata.permissions())
            .map_err(|e| e.to_string())?;
        file.flush()
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        before_commit(&temporary.path)?;
        self.unchanged()?;
        let destination = self
            .parent
            .join(self.path.file_name().ok_or("source filename missing")?);
        temporary.publish(&destination)?;
        Ok(())
    }
}

struct Temporary {
    path: PathBuf,
    published: bool,
}
impl Temporary {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            published: false,
        }
    }

    fn publish(&mut self, destination: &Path) -> Result<(), String> {
        fs::rename(&self.path, destination)
            .map_err(|e| format!("atomic source replacement failed: {e}"))?;
        // Ownership moved to the destination. The old name can be reused by
        // another writer and must no longer participate in our cleanup.
        self.published = true;
        Ok(())
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "sovra-fmt-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    #[test]
    fn replacement_preserves_original_on_failure_and_detects_concurrent_changes() {
        let root = fixture();
        let path = root.join("source.svr");
        fs::write(&path, "old").unwrap();
        let snapshot = Snapshot::read(&path).unwrap();
        assert!(snapshot
            .replace_with("new", |_| Err("injected".into()))
            .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old");
        assert!(snapshot
            .replace_with("new", |temporary| {
                fs::remove_file(temporary).unwrap();
                Ok(())
            })
            .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old");
        assert!(snapshot
            .replace_with("new", |_| {
                fs::write(&path, "external").unwrap();
                Ok(())
            })
            .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn atomic_replacement_noop_and_readonly_policy() {
        let root = fixture();
        let path = root.join("source.svr");
        fs::write(&path, "old").unwrap();
        let snapshot = Snapshot::read(&path).unwrap();
        let modified = snapshot.metadata.modified().unwrap();
        snapshot.replace("old").unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        let permissions = fs::metadata(&path).unwrap().permissions();
        snapshot.replace("new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(fs::metadata(&path).unwrap().permissions(), permissions);
        let mut readonly = permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        assert!(Snapshot::read(&path)
            .unwrap()
            .replace("another")
            .unwrap_err()
            .contains("read-only"));
        fs::set_permissions(&path, permissions).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_publication_does_not_delete_a_reused_staging_path() {
        let root = fixture();
        let staging = root.join("staged.svr");
        let destination = root.join("source.svr");
        fs::write(&staging, "formatted").unwrap();
        let mut temporary = Temporary::new(staging.clone());
        temporary.publish(&destination).unwrap();
        fs::write(&staging, "belongs to another writer").unwrap();
        drop(temporary);
        let reused = fs::read_to_string(&staging);
        assert_eq!(fs::read_to_string(&destination).unwrap(), "formatted");
        let failed_staging = root.join("failed-stage.svr");
        let directory = root.join("cannot-replace-directory");
        fs::create_dir(&directory).unwrap();
        fs::write(&failed_staging, "unpublished").unwrap();
        let mut failed = Temporary::new(failed_staging.clone());
        assert!(failed.publish(&directory).is_err());
        drop(failed);
        assert!(!failed_staging.exists());
        assert!(directory.is_dir());
        fs::remove_dir_all(root).unwrap();
        assert_eq!(reused.unwrap(), "belongs to another writer");
    }
    #[cfg(unix)]
    #[test]
    fn refuses_links_and_preserves_unix_permissions() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = fixture();
        let path = root.join("source.svr");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        Snapshot::read(&path).unwrap().replace("new").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let link = root.join("linked.svr");
        symlink(&path, &link).unwrap();
        assert!(Snapshot::read(&link).is_err());
        let hard = root.join("hard.svr");
        fs::hard_link(&path, &hard).unwrap();
        assert!(Snapshot::read(&path).unwrap().replace("changed").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_attribute_changes_invalidate_the_source_snapshot() {
        use std::os::windows::fs::MetadataExt;
        let root = fixture();
        let path = root.join("source.svr");
        fs::write(&path, "original").unwrap();
        let snapshot = Snapshot::read(&path).unwrap();
        let attrib = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("attrib.exe");
        assert!(std::process::Command::new(&attrib)
            .arg("+H")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        let changed = fs::metadata(&path).unwrap();
        // Hidden status changes neither source bytes nor ordinary permissions.
        assert_ne!(
            snapshot.metadata.file_attributes(),
            changed.file_attributes()
        );
        let matches = same(&snapshot.metadata, &changed);
        let unchanged = snapshot.unchanged();
        assert!(std::process::Command::new(&attrib)
            .arg("-H")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        fs::remove_dir_all(root).unwrap();
        assert!(
            !matches,
            "changed Windows attributes must invalidate metadata"
        );
        assert!(
            unchanged.is_err(),
            "changed attributes must prevent replacement"
        );
    }
}
