//! Direct exported application function interfaces (ADR 0013).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::{FileInspection, FunctionSignature};
use crate::compiler::project::{scope::ServiceIdentity, service_types, ProjectCheck};

pub(super) fn inspect(project: &ProjectCheck) -> Vec<FileInspection> {
    let sources: BTreeMap<_, _> = project
        .source_files
        .iter()
        .map(|file| {
            (
                file.clone(),
                std::fs::read_to_string(file).map_err(|error| error.to_string()),
            )
        })
        .collect();
    let preliminary = super::inspect_project_local(project, &sources);
    let mut interfaces: BTreeMap<PathBuf, BTreeMap<String, Option<FunctionSignature>>> =
        BTreeMap::new();
    for file in &preliminary {
        let (Ok(owner), Ok(functions)) = (file.source_file.canonicalize(), &file.functions) else {
            continue;
        };
        let mut seen = BTreeSet::new();
        let mut exports = BTreeMap::new();
        for function in functions {
            if let Some(signature) = &function.signature {
                if !seen.insert(&function.name) {
                    exports.insert(function.name.clone(), None);
                } else if function.is_exported {
                    let mut signature = signature.clone();
                    signature.source_file = Some(owner.clone());
                    exports.insert(function.name.clone(), Some(signature));
                }
            }
        }
        interfaces.insert(owner, exports);
    }
    let signatures = service_types::resolve_service_signatures(project);
    preliminary
        .into_iter()
        .map(|mut file| {
            // Never extract interfaces or records from a partially parsed file.
            if file.functions.is_err() {
                return file;
            }
            file.functions = (|| {
                let own = file
                    .source_file
                    .canonicalize()
                    .map_err(|error| error.to_string())?;
                let mut visible = BTreeSet::from([own]);
                let mut imported = BTreeMap::new();
                let mut seen_imports = BTreeSet::new();
                for import in &project.imports {
                    if import.source_file != file.source_file {
                        continue;
                    }
                    visible.insert(import.target_file.clone());
                    let prefix = import.module.replace('.', "::");
                    if !seen_imports.insert((prefix.clone(), import.target_file.clone())) {
                        continue;
                    }
                    if let Some(exports) = interfaces.get(&import.target_file) {
                        for (name, signature) in exports {
                            let key = format!("{prefix}::{name}");
                            imported
                                .entry(key)
                                .and_modify(|entry| *entry = None)
                                .or_insert_with(|| signature.clone());
                        }
                    }
                }
                let mut services = Vec::new();
                for declaration in &project.service_declarations {
                    let owner = declaration
                        .source_file
                        .canonicalize()
                        .map_err(|error| error.to_string())?;
                    if visible.contains(&owner) {
                        services.push(ServiceIdentity {
                            module: owner.to_string_lossy().into_owned(),
                            name: declaration.name.clone(),
                        });
                    }
                }
                let source = sources
                    .get(&file.source_file)
                    .expect("discovered source snapshot")
                    .as_ref()
                    .map_err(Clone::clone)?;
                super::inspect_functions_with_imports(
                    source,
                    &services,
                    &signatures.operations,
                    &imported,
                )
            })();
            file
        })
        .collect()
}
