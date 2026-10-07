//! Direct exported application function interfaces (ADR 0013).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::{FileInspection, FunctionSignature};
use crate::compiler::project::{scope::ServiceIdentity, service_types, ProjectCheck};

#[derive(Debug, Default)]
pub(super) struct Records {
    pub module: Option<crate::compiler::project::application_types::Aliases>,
    pub types: std::collections::HashMap<String, crate::compiler::semantic::Type>,
    pub fields: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::compiler::semantic::Type>,
    >,
}

pub(super) fn inspect(
    project: &ProjectCheck,
) -> (Vec<FileInspection>, service_types::ServiceSignatures) {
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
    let modules = crate::compiler::project::application_types::resolve_modules(project, &sources);
    let signatures = service_types::resolve_with_modules(project, &modules);
    let preliminary = super::inspect_project_local(project, &sources, &signatures, &modules);
    let mut record_exports = BTreeMap::new();
    for file in &preliminary {
        if file.functions.is_err() {
            continue;
        }
        let (Ok(owner), Some(types)) = (
            file.source_file.canonicalize(),
            modules.get(&file.source_file),
        ) else {
            continue;
        };
        record_exports.insert(owner, types.public_fields.clone());
    }
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
                    let visible = !signature.exposes_private_records();
                    exports.insert(function.name.clone(), visible.then_some(signature));
                }
            }
        }
        interfaces.insert(owner, exports);
    }
    let files = preliminary
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
                let mut visible = BTreeSet::from([own.clone()]);
                let mut imported = BTreeMap::new();
                let mut records = Records {
                    module: modules.get(&file.source_file).cloned(),
                    ..Default::default()
                };
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
                    if let Some(fields) = record_exports.get(&import.target_file) {
                        for (identity, members) in fields {
                            let name = identity.rsplit("::").next().expect("record identity");
                            records.types.insert(
                                format!("{prefix}::{name}"),
                                crate::compiler::semantic::Type::Named(identity.clone()),
                            );
                            records.fields.insert(identity.clone(), members.clone());
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
                super::inspect_functions_in_module(
                    source,
                    &services,
                    &signatures.operations,
                    &imported,
                    &own.to_string_lossy(),
                    &records,
                )
            })();
            file
        })
        .collect();
    (files, signatures)
}
