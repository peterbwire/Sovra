//! Bounded package graph formatting with preflight and staged publication.

use super::{files::Snapshot, format_source, Failure, Mode};
use crate::compiler::{diagnostics::terminal_message, project::packages};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 128;
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

fn at(path: &Path, failure: Failure) -> Failure {
    match failure {
        Failure::Message(message) => Failure::Message(format!("{}: {message}", path.display())),
        Failure::Diagnostics(mut diagnostics) => {
            for diagnostic in &mut diagnostics.items {
                diagnostic.source_file = Some(path.to_string_lossy().into_owned());
            }
            Failure::Diagnostics(diagnostics)
        }
    }
}

fn charge(used: &mut usize, amount: usize, limit: usize) -> Result<(), Failure> {
    *used = used
        .checked_add(amount)
        .filter(|sum| *sum <= limit)
        .ok_or_else(|| {
            Failure::Message("package formatter cumulative byte limit exceeded".into())
        })?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum Selection {
    Entries(packages::LocalGraph),
    Registered(packages::PackageSourceInventory),
}
impl Selection {
    fn load(root: &Path) -> Result<Self, Failure> {
        if packages::classify_cli(root)? == packages::CliPackageMode::Registered {
            Ok(Self::Registered(packages::inspect_source_inventory(root)?))
        } else {
            Ok(Self::Entries(packages::resolve(root)?))
        }
    }
    fn graph(&self) -> &packages::LocalGraph {
        match self {
            Self::Entries(graph) => graph,
            Self::Registered(inventory) => &inventory.graph,
        }
    }
    fn paths(&self) -> BTreeSet<PathBuf> {
        match self {
            Self::Entries(graph) => graph
                .packages
                .values()
                .map(|package| package.entry.clone())
                .collect(),
            Self::Registered(inventory) => inventory
                .sources
                .iter()
                .map(|source| source.canonical_path.clone())
                .collect(),
        }
    }
}

pub(super) fn execute(root: &Path, mode: Mode) -> Result<bool, Failure> {
    execute_with(root, mode, |_| Ok(()))
}

fn execute_with(
    root: &Path,
    mode: Mode,
    mut checkpoint: impl FnMut(usize) -> Result<(), String>,
) -> Result<bool, Failure> {
    if mode == Mode::Write && !cfg!(any(windows, target_os = "linux")) {
        return Err(Failure::Message(
            "formatter --write is qualified only on Windows and Linux".into(),
        ));
    }
    let selection = Selection::load(root)?;
    let graph = selection.graph();
    let entries = selection.paths();
    if entries.len() > MAX_ENTRIES {
        return Err(Failure::Message(
            "package formatter exceeds 128 entries".into(),
        ));
    }
    // Snapshot manifests before resolving again: the selected graph must describe
    // these exact manifest bytes, rather than a mix of revisions.
    let mut snapshots = Vec::new();
    let mut input_bytes = 0;
    for package in graph.packages.values() {
        let path = package.root.join("sovra.toml");
        let snapshot = Snapshot::read(&path).map_err(|e| at(&path, e.into()))?;
        charge(&mut input_bytes, snapshot.text.len(), MAX_INPUT_BYTES)?;
        snapshots.push((path, snapshot));
    }
    if Selection::load(&graph.root)? != selection {
        return Err(Failure::Message(
            "package graph changed during formatter preparation".into(),
        ));
    }
    let mut entries_to_format = Vec::new();
    let mut output_bytes = 0;
    for path in entries {
        let snapshot = Snapshot::read(&path).map_err(|e| at(&path, e.into()))?;
        charge(&mut input_bytes, snapshot.text.len(), MAX_INPUT_BYTES)?;
        let output = format_source(&snapshot.text).map_err(|e| at(&path, e))?;
        charge(&mut output_bytes, output.len(), MAX_OUTPUT_BYTES)?;
        if mode == Mode::Write {
            snapshot.writable().map_err(|e| at(&path, e.into()))?;
        }
        entries_to_format.push((path, snapshot, output));
    }
    let changed = entries_to_format
        .iter()
        .filter(|(_, snapshot, output)| snapshot.text != *output)
        .count();
    let mut staged = if mode == Mode::Write {
        entries_to_format
            .iter()
            .map(|(path, snapshot, output)| snapshot.stage(output).map_err(|e| at(path, e.into())))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    checkpoint(0).map_err(Failure::Message)?;
    // No publication or formatting notice until every selected input is valid.
    for (path, snapshot) in snapshots
        .iter()
        .map(|(path, snapshot)| (path, snapshot))
        .chain(
            entries_to_format
                .iter()
                .map(|(path, snapshot, _)| (path, snapshot)),
        )
    {
        snapshot.unchanged().map_err(|e| at(path, e.into()))?;
    }
    for ((path, _, _), staged) in entries_to_format.iter().zip(&mut staged) {
        staged.validate().map_err(|e| at(path, e.into()))?;
    }
    if mode == Mode::Check {
        for (path, snapshot, output) in &entries_to_format {
            if snapshot.text != *output {
                eprintln!(
                    "Formatting needed: {}",
                    terminal_message(format_args!("{}", path.display()))
                );
            }
        }
    } else {
        let mut published = 0;
        for ((path, snapshot, output), staged) in entries_to_format.iter().zip(staged) {
            let result = (|| {
                for (_, manifest) in &snapshots {
                    manifest.unchanged()?;
                }
                staged.commit()
            })();
            if let Err(cause) = result {
                return Err(at(path, Failure::Message(format!("package publication stopped after {published} of {changed} changed files; earlier replacements remain: {cause}"))));
            }
            if snapshot.text != *output {
                published += 1;
            }
            eprintln!(
                "{}: {}",
                if snapshot.text != *output {
                    "Formatted"
                } else {
                    "Unchanged"
                },
                terminal_message(format_args!("{}", path.display()))
            );
            checkpoint(published).map_err(|cause| Failure::Message(format!("package publication stopped after {published} of {changed} changed files; earlier replacements remain: {cause}")))?;
        }
    }
    Ok(changed != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cumulative_budget_accepts_exact_limit_and_rejects_overflow() {
        let mut used = 0;
        charge(&mut used, MAX_INPUT_BYTES, MAX_INPUT_BYTES).unwrap();
        assert!(charge(&mut used, 1, MAX_INPUT_BYTES).is_err());
        let mut used = usize::MAX;
        assert!(charge(&mut used, 1, MAX_OUTPUT_BYTES).is_err());
    }
    #[cfg(any(windows, target_os = "linux"))]
    fn fixture() -> (PathBuf, PathBuf, PathBuf) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "sovra-package-write-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for name in ["a", "b"] {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let deps = if name == "a" {
                "[dependencies.b]\npath = \"../b\"\n"
            } else {
                ""
            };
            std::fs::write(
                dir.join("sovra.toml"),
                format!("[project]\nname = \"fixture\"\nentry = \"main.svr\"\n{deps}"),
            )
            .unwrap();
            std::fs::write(dir.join("main.svr"), "fn main(){print(1);}").unwrap();
        }
        (
            root.clone(),
            root.join("a/main.svr"),
            root.join("b/main.svr"),
        )
    }
    #[cfg(any(windows, target_os = "linux"))]
    fn no_staging(root: &Path) {
        for name in ["a", "b"] {
            assert!(!std::fs::read_dir(root.join(name)).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".svr-fmt-")));
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn registered_dependency_conflicts_abort_before_any_source_is_published() {
        let (root, entry, _) = fixture();
        let manifest = root.join("b/sovra.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(
            &manifest,
            format!("{text}[modules]\nvalues=\"values.svr\"\n"),
        )
        .unwrap();
        let module = root.join("b/values.svr");
        std::fs::write(&module, "mod values{export fn answer()->Int{return 1;}}").unwrap();
        let original = std::fs::read(&entry).unwrap();
        let result = execute_with(&root.join("a"), Mode::Write, |published| {
            assert_eq!(published, 0);
            std::fs::write(&module, "// concurrent edit\n").unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&entry).unwrap(), original);
        assert_eq!(
            std::fs::read_to_string(module).unwrap(),
            "// concurrent edit\n"
        );
        no_staging(&root);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn package_preflight_conflicts_and_staging_tampering_preserve_sources() {
        for kind in [
            "manifest",
            "source",
            "staging",
            "staging_identity",
            "source_link",
        ] {
            if kind == "source_link" && !cfg!(unix) {
                continue;
            }
            let (root, a, b) = fixture();
            let original = std::fs::read(&a).unwrap();
            let result = execute_with(&root.join("a"), Mode::Write, |published| {
                assert_eq!(published, 0);
                match kind {
                    "manifest" => std::fs::write(root.join("b/sovra.toml"), "changed").unwrap(),
                    "source" => std::fs::write(&b, "external").unwrap(),
                    "source_link" => std::fs::hard_link(&b, root.join("extra.svr")).unwrap(),
                    _ => {
                        let staged = std::fs::read_dir(root.join("b"))
                            .unwrap()
                            .map(|e| e.unwrap().path())
                            .find(|p| {
                                p.file_name()
                                    .unwrap()
                                    .to_string_lossy()
                                    .starts_with(".svr-fmt-")
                            })
                            .unwrap();
                        if kind == "staging_identity" {
                            let bytes = std::fs::read(&staged).unwrap();
                            std::fs::rename(&staged, root.join("moved-stage")).unwrap();
                            std::fs::write(staged, bytes).unwrap();
                        } else {
                            std::fs::write(staged, "tampered").unwrap();
                        }
                    }
                }
                Ok(())
            });
            assert!(result.is_err(), "{kind}");
            assert_eq!(std::fs::read(&a).unwrap(), original);
            assert_eq!(
                std::fs::read(&b).unwrap(),
                if kind == "source" {
                    b"external".to_vec()
                } else {
                    original
                }
            );
            no_staging(&root);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn late_publication_failure_keeps_prior_replacement_and_external_edit() {
        let (root, a, b) = fixture();
        let result = execute_with(&root.join("a"), Mode::Write, |published| {
            if published == 1 {
                std::fs::write(&b, "external edit").unwrap();
            }
            Ok(())
        });
        let Err(Failure::Message(message)) = result else {
            panic!("expected publication failure")
        };
        assert!(message.contains("after 1 of 2 changed files"), "{message}");
        assert_eq!(
            std::fs::read_to_string(&a).unwrap(),
            "fn main() {\n    print(1);\n}\n"
        );
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "external edit");
        no_staging(&root);
        std::fs::remove_dir_all(root).unwrap();
    }
}
