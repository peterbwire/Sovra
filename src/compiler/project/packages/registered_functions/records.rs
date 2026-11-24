//! Canonical record interfaces with consumer-owned attached spans.
use super::{charge, issue, locate, spelling, zero};
use crate::compiler::project::packages::LoadedPackageSources;
use crate::compiler::{
    ast::{Program, StructDeclaration, StructField, TypeDeclaration},
    diagnostics::{diagnostic_message, Diagnostics, Span},
    resources::interface_strings::InterfaceStrings,
    semantic::{self, Type},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

struct Record {
    unit: usize,
    package: usize,
    module: Option<String>,
    local_name: String,
    alias: String,
    identity: String,
    declaration: StructDeclaration,
}
struct Alias {
    name: String,
    record: usize,
    span: Span,
}
pub(super) struct RecordPlan {
    pub programs: Vec<Program>,
    pub public_types: HashMap<String, String>,
    pub all_types: HashMap<String, String>,
}
impl RecordPlan {
    pub fn new(
        sources: &LoadedPackageSources,
        unit_packages: &[usize],
        packages: &BTreeMap<std::path::PathBuf, usize>,
        nodes: &mut usize,
        strings: &mut InterfaceStrings,
    ) -> Result<Self, Diagnostics> {
        let mut programs = Vec::new();
        let mut records = Vec::new();
        let mut identities = BTreeSet::new();
        let mut modules = BTreeMap::<(usize, String), Vec<usize>>::new();
        for (unit_index, unit) in sources.units.iter().enumerate() {
            let path = &sources.inventory.sources[unit.source_index].canonical_path;
            let scoped =
                semantic::resolve_source_type_scopes(&unit.program).map_err(|e| locate(e, path))?;
            for (module, declarations) in std::iter::once((None, &scoped.struct_declarations))
                .chain(
                    scoped
                        .modules
                        .iter()
                        .map(|m| (Some(m.name.as_str()), &m.struct_declarations)),
                )
            {
                if let Some(module) = module {
                    let key = strings.copy(module, zero()).map_err(|e| locate(e, path))?;
                    modules.entry((unit_packages[unit_index], key)).or_default();
                }
                for record in declarations {
                    charge(nodes, 1 + record.fields.len(), path, record.span)?;
                    let alias = spelling(module, &record.name, strings, record.span)
                        .map_err(|e| locate(e, path))?;
                    let prefix = format!("@package{}::type::", unit_packages[unit_index]);
                    strings
                        .charge(
                            prefix.len() + module.unwrap_or("root").len() + 2 + record.name.len(),
                            record.span,
                        )
                        .map_err(|e| locate(e, path))?;
                    if module == Some("root") {
                        strings
                            .charge("module::".len(), record.span)
                            .map_err(|e| locate(e, path))?;
                    }
                    let identity = super::super::executable::record_identity(
                        unit_packages[unit_index],
                        module,
                        &record.name,
                    );
                    if !identities.insert(
                        strings
                            .copy(&identity, record.span)
                            .map_err(|e| locate(e, path))?,
                    ) {
                        return Err(issue(
                            "E3008",
                            diagnostic_message!("duplicate record `{}`", record.name),
                            path,
                            record.span,
                        ));
                    }
                    let local_name = strings
                        .copy(&record.name, record.span)
                        .map_err(|e| locate(e, path))?;
                    let mut fields = Vec::new();
                    for field in &record.fields {
                        fields.push(StructField {
                            name: strings
                                .copy(&field.name, field.span)
                                .map_err(|e| locate(e, path))?,
                            type_name: strings
                                .copy(&field.type_name, field.span)
                                .map_err(|e| locate(e, path))?,
                            type_span: field.type_span,
                            span: field.span,
                        });
                    }
                    let module = module
                        .map(|name| strings.copy(name, record.span))
                        .transpose()
                        .map_err(|e| locate(e, path))?;
                    if let Some(module) = &module {
                        modules
                            .get_mut(&(unit_packages[unit_index], module.clone()))
                            .expect("registered module")
                            .push(records.len());
                    }
                    records.push(Record {
                        unit: unit_index,
                        package: unit_packages[unit_index],
                        module,
                        local_name,
                        alias,
                        identity: strings
                            .copy(&identity, record.span)
                            .map_err(|e| locate(e, path))?,
                        declaration: StructDeclaration {
                            name: identity,
                            fields,
                            is_exported: record.is_exported,
                            span: record.span,
                        },
                    });
                }
            }
            programs.push(scoped);
        }
        // Every checked source needs one canonical shape per record. Reject an
        // impossible graph before repeated source/record context scans.
        let shape_nodes = records
            .iter()
            .map(|record| 1 + record.declaration.fields.len())
            .sum::<usize>();
        if shape_nodes
            .checked_mul(programs.len())
            .map_or(true, |minimum| minimum > *nodes)
        {
            let path = &sources.inventory.sources[0].canonical_path;
            return Err(issue(
                "E3023",
                "registered record shape copies exceed the remaining interface node allowance"
                    .into(),
                path,
                zero(),
            ));
        }
        let mut aliases = Vec::new();
        for (unit_index, unit) in sources.units.iter().enumerate() {
            let registration = &sources.inventory.sources[unit.source_index];
            let path = &registration.canonical_path;
            let mut visible = BTreeMap::<String, Alias>::new();
            for (index, record) in records.iter().enumerate().filter(|(_, record)| {
                record.package == unit_packages[unit_index] && record.unit != unit_index
            }) {
                insert_alias(
                    &mut visible,
                    strings
                        .copy(&record.alias, zero())
                        .map_err(|e| locate(e, path))?,
                    index,
                    zero(),
                    nodes,
                    path,
                    strings,
                )?;
            }
            for import in &unit.imports {
                let package = &sources.inventory.graph.packages[&registration.package_root];
                let target = package.dependencies.get(&import.alias).ok_or_else(|| {
                    issue(
                        "E4115",
                        diagnostic_message!("undeclared dependency `{}`", import.alias),
                        path,
                        import.span,
                    )
                })?;
                let candidates = modules
                    .get(&(packages[target], import.module.clone()))
                    .ok_or_else(|| {
                        issue(
                            "E4115",
                            diagnostic_message!(
                                "dependency `{}` has no module `{}`",
                                import.alias,
                                import.module
                            ),
                            path,
                            import.span,
                        )
                    })?;
                for &index in candidates {
                    let record = &records[index];
                    if !record.declaration.is_exported {
                        continue;
                    }
                    strings
                        .charge(import.alias.len() + 2 + record.alias.len(), import.span)
                        .map_err(|e| locate(e, path))?;
                    insert_alias(
                        &mut visible,
                        format!("{}::{}", import.alias, record.alias),
                        index,
                        import.span,
                        nodes,
                        path,
                        strings,
                    )?;
                }
            }
            aliases.push(visible.into_values().collect::<Vec<_>>());
        }
        // Resolve aliases and validate original fields with core rules.
        for unit_index in 0..programs.len() {
            let path =
                &sources.inventory.sources[sources.units[unit_index].source_index].canonical_path;
            let mut preview = programs[unit_index].clone();
            for alias in &aliases[unit_index] {
                preview.struct_declarations.push(StructDeclaration {
                    name: alias.name.clone(),
                    fields: Vec::new(),
                    is_exported: false,
                    span: alias.span,
                });
            }
            let mut diagnostics = Diagnostics::new();
            let known = semantic::collect_named_types(&preview, &mut diagnostics);
            semantic::collect_struct_fields(&preview, &known, &mut diagnostics);
            if !diagnostics.is_empty() {
                return Err(locate(diagnostics, path));
            }
            // Resolve field edges by declared, visible record identity without
            // recursively expanding the referenced layout or graph.
            let mut targets = HashMap::new();
            for record in records.iter().filter(|record| record.unit == unit_index) {
                targets.insert(
                    strings
                        .copy(&record.alias, record.declaration.span)
                        .map_err(|e| locate(e, path))?,
                    (
                        strings
                            .copy(&record.identity, record.declaration.span)
                            .map_err(|e| locate(e, path))?,
                        record.declaration.is_exported,
                    ),
                );
            }
            for alias in &aliases[unit_index] {
                let target = &records[alias.record];
                targets.insert(
                    strings
                        .copy(&alias.name, alias.span)
                        .map_err(|e| locate(e, path))?,
                    (
                        strings
                            .copy(&target.identity, alias.span)
                            .map_err(|e| locate(e, path))?,
                        target.declaration.is_exported,
                    ),
                );
            }
            for record in records
                .iter_mut()
                .filter(|record| record.unit == unit_index)
            {
                for field in &mut record.declaration.fields {
                    let kind = Type::from_name_with_known(&field.type_name, Some(&known))
                        .canonicalize(&known);
                    let span = field.type_span.unwrap_or(field.span);
                    let mut leaf = &kind;
                    let mut arrays = 0;
                    while let Type::Array(inner) = leaf {
                        arrays += 1;
                        leaf = inner;
                    }
                    charge(nodes, arrays, path, span)?;
                    let name = match leaf {
                        Type::Unit => "Unit",
                        Type::Bool => "Bool",
                        Type::Int => "Int",
                        Type::Float => "Float",
                        Type::String => "String",
                        Type::Named(name)
                            if crate::compiler::stdlib::is_standard_record_type(name) =>
                        {
                            name
                        }
                        Type::Named(name) => {
                            let (identity, exported) = targets.get(name).ok_or_else(|| {
                                issue(
                                    "E4164",
                                    "nonregistered record field context unavailable".into(),
                                    path,
                                    span,
                                )
                            })?;
                            if record.declaration.is_exported && !exported {
                                return Err(issue(
                                    "E4116",
                                    diagnostic_message!(
                                        "exported record exposes private field type `{name}`"
                                    ),
                                    path,
                                    span,
                                ));
                            }
                            identity
                        }
                        _ => {
                            return Err(issue(
                                "E4164",
                                "unsupported record field context unavailable".into(),
                                path,
                                span,
                            ))
                        }
                    };
                    strings
                        .charge(name.len() + 2 * arrays, span)
                        .map_err(|e| locate(e, path))?;
                    field.type_name =
                        format!("{}{}{}", "[".repeat(arrays), name, "]".repeat(arrays));
                }
            }
        }
        let mut all_types = HashMap::new();
        let mut public_types = HashMap::new();
        // Standard records retain their globally defined nominal identities and
        // core field schemas; they never become user-declared record layouts.
        let path = &sources.inventory.sources[0].canonical_path;
        for name in [
            crate::compiler::stdlib::INPUT_LINE_TYPE,
            crate::compiler::stdlib::TEXT_READ_TYPE,
            crate::compiler::stdlib::TEXT_WRITE_TYPE,
            crate::compiler::stdlib::SPLIT_ONCE_TYPE,
            crate::compiler::stdlib::PARSED_INT_TYPE,
        ] {
            charge(nodes, 2, path, zero())?;
            let copy = |strings: &mut InterfaceStrings| {
                strings.copy(name, zero()).map_err(|e| locate(e, path))
            };
            all_types.insert(copy(strings)?, copy(strings)?);
            public_types.insert(copy(strings)?, copy(strings)?);
        }
        for record in &records {
            let path =
                &sources.inventory.sources[sources.units[record.unit].source_index].canonical_path;
            all_types.insert(
                strings
                    .copy(&record.identity, record.declaration.span)
                    .map_err(|e| locate(e, path))?,
                strings
                    .copy(&record.identity, record.declaration.span)
                    .map_err(|e| locate(e, path))?,
            );
            if record.declaration.is_exported {
                public_types.insert(
                    strings
                        .copy(&record.identity, record.declaration.span)
                        .map_err(|e| locate(e, path))?,
                    strings
                        .copy(&record.identity, record.declaration.span)
                        .map_err(|e| locate(e, path))?,
                );
            }
        }
        for (unit_index, program) in programs.iter_mut().enumerate() {
            let path =
                &sources.inventory.sources[sources.units[unit_index].source_index].canonical_path;
            program.struct_declarations.clear();
            for module in &mut program.modules {
                module.struct_declarations.clear();
            }
            for record in records.iter().filter(|record| record.unit == unit_index) {
                charge(nodes, 1, path, record.declaration.span)?;
                let alias = TypeDeclaration {
                    name: strings
                        .copy(&record.local_name, record.declaration.span)
                        .map_err(|e| locate(e, path))?,
                    target: strings
                        .copy(&record.identity, record.declaration.span)
                        .map_err(|e| locate(e, path))?,
                    span: record.declaration.span,
                };
                if let Some(module) = &record.module {
                    program
                        .modules
                        .iter_mut()
                        .find(|m| m.name == *module)
                        .expect("original module")
                        .type_declarations
                        .push(alias);
                } else {
                    program.type_declarations.push(alias);
                }
            }
            for alias in &aliases[unit_index] {
                program.type_declarations.push(TypeDeclaration {
                    name: strings
                        .copy(&alias.name, alias.span)
                        .map_err(|e| locate(e, path))?,
                    target: strings
                        .copy(&records[alias.record].identity, alias.span)
                        .map_err(|e| locate(e, path))?,
                    span: alias.span,
                });
            }
            for (index, record) in records.iter().enumerate() {
                let context = if record.unit == unit_index {
                    None
                } else {
                    Some(
                        aliases[unit_index]
                            .iter()
                            .find(|alias| alias.record == index)
                            .map_or(zero(), |alias| alias.span),
                    )
                };
                let span = context.unwrap_or(record.declaration.span);
                let arrays: usize = record
                    .declaration
                    .fields
                    .iter()
                    .map(|field| {
                        semantic::array_parts(&field.type_name).map_or(0, |(depth, _)| depth)
                    })
                    .sum();
                charge(
                    nodes,
                    1 + record.declaration.fields.len() + arrays,
                    path,
                    span,
                )?;
                let fields = record
                    .declaration
                    .fields
                    .iter()
                    .map(|field| {
                        Ok(StructField {
                            name: strings.copy(&field.name, context.unwrap_or(field.span))?,
                            type_name: strings
                                .copy(&field.type_name, context.unwrap_or(field.span))?,
                            span: context.unwrap_or(field.span),
                            type_span: context.or(field.type_span),
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostics>>()
                    .map_err(|e| locate(e, path))?;
                program.struct_declarations.push(StructDeclaration {
                    name: strings
                        .copy(&record.identity, span)
                        .map_err(|e| locate(e, path))?,
                    fields,
                    is_exported: record.declaration.is_exported,
                    span,
                });
            }
        }
        Ok(Self {
            programs,
            public_types,
            all_types,
        })
    }
    pub fn runtime_types(
        &self,
        unit: usize,
        path: &Path,
        strings: &mut InterfaceStrings,
    ) -> Result<HashMap<String, String>, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        let known = semantic::collect_named_types(&self.programs[unit], &mut diagnostics);
        if !diagnostics.is_empty() {
            return Err(locate(diagnostics, path));
        }
        let mut result = HashMap::new();
        for (name, kind) in &known {
            if let Type::Named(identity) = kind.canonicalize(&known) {
                result.insert(
                    strings.copy(name, zero()).map_err(|e| locate(e, path))?,
                    strings
                        .copy(&identity, zero())
                        .map_err(|e| locate(e, path))?,
                );
            }
        }
        Ok(result)
    }
}
fn insert_alias(
    visible: &mut BTreeMap<String, Alias>,
    name: String,
    record: usize,
    span: Span,
    nodes: &mut usize,
    path: &Path,
    strings: &mut InterfaceStrings,
) -> Result<(), Diagnostics> {
    charge(nodes, 1, path, span)?;
    if let Some(previous) = visible.get(&name) {
        if previous.record != record {
            return Err(issue(
                "E3008",
                "ambiguous record type binding".into(),
                path,
                span,
            ));
        }
    }
    let key = strings.copy(&name, span).map_err(|e| locate(e, path))?;
    visible.insert(key, Alias { name, record, span });
    Ok(())
}
