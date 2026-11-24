//! Guarded lock publication on the same Linux/procfs hosts as source capture.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use std::{
        fs::{self, Metadata, OpenOptions},
        io::Write,
        os::{
            fd::AsRawFd,
            unix::fs::{MetadataExt, OpenOptionsExt},
        },
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn same(a: &Metadata, b: &Metadata) -> bool {
        a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.len() == b.len()
            && a.mtime() == b.mtime()
            && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
            && a.mode() == b.mode()
            && a.uid() == b.uid()
            && a.gid() == b.gid()
            && a.nlink() == b.nlink()
    }
    fn regular(path: &Path) -> Result<Metadata, String> {
        let value = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !value.is_file()
            || value.file_type().is_symlink()
            || value.nlink() != 1
            || value.mode() & 0o7000 != 0
            || value.permissions().readonly()
        {
            return Err("unsafe or read-only lock publication target".into());
        }
        Ok(value)
    }
    struct Stage {
        path: PathBuf,
        identity: (u64, u64),
    }
    impl Drop for Stage {
        fn drop(&mut self) {
            if fs::symlink_metadata(&self.path).is_ok_and(|m| (m.dev(), m.ino()) == self.identity) {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
    pub(crate) fn publish(root: &Path, text: &str) -> Result<(), String> {
        publish_with(root, text, || {})
    }
    fn publish_with(root: &Path, text: &str, before_commit: impl FnOnce()) -> Result<(), String> {
        if text.len() > super::super::MAX_LOCK_BYTES {
            return Err("lock output exceeds 8 MiB".into());
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(0o400000 | 0o200000)
            .open(root)
            .map_err(|e| e.to_string())?;
        let anchor = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
        let anchored = || fs::read_link(&anchor).is_ok_and(|path| path == root);
        if !anchored() {
            return Err("lock directory identity/location changed".into());
        }
        let destination = anchor.join("sovra.lock");
        let existing = match fs::symlink_metadata(&destination) {
            Ok(_) => Some(regular(&destination)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if existing.is_some() {
            let old = super::super::super::source_units::read_guarded(
                &root.join("sovra.lock"),
                root,
                super::super::MAX_LOCK_BYTES,
            )
            .map_err(|_| "cannot snapshot existing lock")?;
            if old == text {
                return Ok(());
            }
            let parsed = super::super::json::parse(&old)?;
            let super::super::json::Value::Object(fields) = parsed else {
                return Err("existing lock is not a supported lock document".into());
            };
            if fields.get("format") != Some(&super::super::json::Value::String("sovra-lock".into()))
                || fields.get("version") != Some(&super::super::json::Value::Number(1))
            {
                return Err("existing lock format/version is unsupported".into());
            }
        }
        let mut selected = None;
        for _ in 0..128 {
            let path = anchor.join(format!(
                ".svr-lock-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => {
                    let metadata = file.metadata().map_err(|e| e.to_string())?;
                    selected = Some((
                        Stage {
                            path,
                            identity: (metadata.dev(), metadata.ino()),
                        },
                        file,
                    ));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        let (stage, mut file) = selected.ok_or("cannot reserve lock staging file")?;
        if let Some(previous) = &existing {
            let created = file.metadata().map_err(|e| e.to_string())?;
            if created.uid() != previous.uid() || created.gid() != previous.gid() {
                return Err("cannot preserve lock ownership/group".into());
            }
            file.set_permissions(previous.permissions())
                .map_err(|e| e.to_string())?;
        }
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        before_commit();
        if !anchored() {
            return Err("lock directory moved during publication".into());
        }
        let current = regular(&stage.path)?;
        let handle = file.metadata().map_err(|e| e.to_string())?;
        if !same(&current, &handle) || (handle.dev(), handle.ino()) != stage.identity {
            return Err("lock staging identity changed".into());
        }
        // Verify the held staging bytes, not just its pathname metadata.
        use std::io::{Read, Seek, SeekFrom};
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut actual = Vec::new();
        (&file)
            .take(super::super::MAX_LOCK_BYTES as u64 + 1)
            .read_to_end(&mut actual)
            .map_err(|e| e.to_string())?;
        if actual != text.as_bytes() {
            return Err("lock staging bytes changed".into());
        }
        match &existing {
            Some(previous) => {
                if !same(previous, &regular(&destination)?) {
                    return Err("lock changed before publication".into());
                }
                fs::rename(&stage.path, &destination).map_err(|e| e.to_string())?;
            }
            None => {
                fs::hard_link(&stage.path, &destination).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn creation_and_staging_conflicts_never_clobber_other_files() {
            let root =
                std::env::temp_dir().join(format!("sovra-lock-conflicts-{}", std::process::id()));
            fs::create_dir(&root).unwrap();
            let target = root.join("sovra.lock");
            assert!(
                publish_with(&root, "new", || fs::write(&target, "concurrent").unwrap()).is_err()
            );
            assert_eq!(fs::read_to_string(&target).unwrap(), "concurrent");
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
            fs::remove_file(&target).unwrap();
            assert!(publish_with(&root, "new", || {
                let stage = fs::read_dir(&root).unwrap().next().unwrap().unwrap().path();
                fs::write(stage, "tampered").unwrap();
            })
            .is_err());
            assert!(!target.exists());
            assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
            let moved = root.with_extension("moved");
            assert!(publish_with(&root, "new", || fs::rename(&root, &moved).unwrap()).is_err());
            assert!(!moved.join("sovra.lock").exists());
            assert_eq!(fs::read_dir(&moved).unwrap().count(), 0);
            fs::remove_dir(moved).unwrap();
        }
        #[test]
        fn replacement_conflicts_preserve_the_intervening_lock_and_cleanup_owned_staging() {
            let root =
                std::env::temp_dir().join(format!("sovra-lock-publication-{}", std::process::id()));
            fs::create_dir(&root).unwrap();
            struct Temp(PathBuf);
            impl Drop for Temp {
                fn drop(&mut self) {
                    let _ = fs::remove_dir_all(&self.0);
                }
            }
            let _temp = Temp(root.clone());
            let old = "{\"format\":\"sovra-lock\",\"version\":1}\n";
            fs::write(root.join("sovra.lock"), old).unwrap();
            assert!(publish_with(&root, "different", || fs::write(
                root.join("sovra.lock"),
                "intervening"
            )
            .unwrap())
            .is_err());
            assert_eq!(
                fs::read_to_string(root.join("sovra.lock")).unwrap(),
                "intervening"
            );
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        }
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) use linux::publish;
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
)))]
pub(super) fn publish(_root: &std::path::Path, _text: &str) -> Result<(), String> {
    Err("guarded lock publication unavailable on this host".into())
}
