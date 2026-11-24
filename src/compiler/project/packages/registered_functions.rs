//! Source-aware checked linking for the first registered primitive callable slice.
mod records;
use super::{
    checked::EvidenceBudget, error, load_package_sources, CheckedPackageError,
    LoadedPackageSources, PackageEvidenceLimits,
};
use crate::compiler::{
    ast::Function,
    checked::{CheckedUnit, ImportBinding},
    diagnostics::{diagnostic_message, Diagnostics, Span},
    ir::{self, IrProgram},
    resources::{
        interface_strings::InterfaceStrings, PackageCompilationLimits,
        DEFAULT_INTERFACE_STRING_BYTES,
    },
    semantic::{self, InterfaceTypes},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

/// Checked source units and linked IR, without mutable access or a CLI-mode change.
#[derive(Debug)]
pub struct RegisteredFunctionPackage {
    sources: LoadedPackageSources,
    checked: Vec<CheckedUnit>,
    ir: IrProgram,
    remaining: PackageEvidenceLimits,
}
impl RegisteredFunctionPackage {
    /// Captured file-owned source/AST/import inputs, in checked-unit owner order.
    pub fn sources(&self) -> &LoadedPackageSources {
        &self.sources
    }
    /// One checked unit per original source, with package owner IDs equal to indices.
    pub fn checked_units(&self) -> &[CheckedUnit] {
        &self.checked
    }
    /// Complete linked IR; no partial program is returned on failure.
    pub fn ir(&self) -> &IrProgram {
        &self.ir
    }
    pub(crate) fn into_ir(self) -> IrProgram {
        self.ir
    }
    pub(crate) fn into_test_parts(
        self,
    ) -> (IrProgram, crate::compiler::ast::Program, std::path::PathBuf) {
        let entry = self.sources.inventory().graph.packages[&self.sources.inventory().graph.root]
            .entry
            .clone();
        let program = self
            .checked
            .first()
            .expect("root entry is the first checked source")
            .program()
            .clone();
        (self.ir, program, entry)
    }
    /// Unspent cumulative checked-evidence allowances.
    pub fn remaining(&self) -> PackageEvidenceLimits {
        self.remaining
    }
}

#[derive(Clone, Copy)]
struct Limits {
    compilation: PackageCompilationLimits,
    strings: usize,
    evidence: PackageEvidenceLimits,
}
impl Limits {
    const DEFAULT: Self = Self {
        compilation: PackageCompilationLimits::DEFAULT,
        strings: DEFAULT_INTERFACE_STRING_BYTES,
        evidence: PackageEvidenceLimits::DEFAULT,
    };
}
struct Definition {
    unit: usize,
    spelling: String,
    symbol: String,
    function: Function,
}
struct Binding {
    definition: usize,
    span: Span,
}
fn zero() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    }
}
fn locate(mut errors: Diagnostics, path: &Path) -> Diagnostics {
    for item in &mut errors.items {
        item.source_file = Some(path.to_string_lossy().into_owned());
    }
    errors
}
fn issue(code: &'static str, message: String, path: &Path, span: Span) -> Diagnostics {
    error(code, message, Some((path, span)))
}
fn charge(nodes: &mut usize, count: usize, path: &Path, span: Span) -> Result<(), Diagnostics> {
    *nodes = nodes.checked_sub(count).ok_or_else(|| {
        issue(
            "E3023",
            "registered function interface node limit exceeded".into(),
            path,
            span,
        )
    })?;
    Ok(())
}
fn spelling(
    module: Option<&str>,
    name: &str,
    strings: &mut InterfaceStrings,
    span: Span,
) -> Result<String, Diagnostics> {
    strings.charge(name.len(), span)?;
    if let Some(module) = module {
        strings.charge(module.len(), span)?;
        strings.charge(2, span)?;
        Ok(format!("{module}::{name}"))
    } else {
        Ok(name.to_owned())
    }
}
fn symbol(
    package: usize,
    name: &str,
    strings: &mut InterfaceStrings,
    span: Span,
) -> Result<String, Diagnostics> {
    if package == 0 && name == "main" {
        return strings.copy("main", span);
    }
    let prefix = format!("@package{package}::");
    strings.charge(prefix.len(), span)?;
    strings.charge(name.len(), span)?;
    Ok(format!("{prefix}{name}"))
}

/// Compile registered sources with primitive callable interfaces/local primitive aliases.
/// Every body/private/unused source is checked through bounded CheckedUnit lowering.
/// File-local direct imports and ordinary same-package visibility are enforced;
/// user record interfaces remain explicitly unavailable (E4163), without fallback.
/// Uses captured source-owned units; default compile/CLI/tooling remain unchanged.
/// Guarded source loading has the same explicit supported-host limits as its API.
pub fn compile_registered_primitive_functions(
    root: impl AsRef<Path>,
) -> Result<RegisteredFunctionPackage, CheckedPackageError> {
    compile(load_package_sources(root)?, Limits::DEFAULT)
}

/// Compile registered functions and nominal records with primitive/visible record fields.
/// Field edges retain canonical identities without recursively expanding layouts.
/// Record/function exports and source-owned diagnostics follow existing rules.
/// Existing public standard result records are supported without new layouts.
/// The older primitive-only compiler retains its separate scope.
pub fn compile_registered_records(
    root: impl AsRef<Path>,
) -> Result<RegisteredFunctionPackage, CheckedPackageError> {
    compile_captured_records(load_package_sources(root)?)
}

pub(super) fn compile_captured_records(
    sources: LoadedPackageSources,
) -> Result<RegisteredFunctionPackage, CheckedPackageError> {
    compile_mode(sources, Limits::DEFAULT, true)
}

fn compile(
    sources: LoadedPackageSources,
    limits: Limits,
) -> Result<RegisteredFunctionPackage, CheckedPackageError> {
    compile_mode(sources, limits, false)
}

fn compile_mode(
    sources: LoadedPackageSources,
    limits: Limits,
    records: bool,
) -> Result<RegisteredFunctionPackage, CheckedPackageError> {
    let mut strings = InterfaceStrings::new(Some(limits.strings));
    let mut interface_nodes = limits.compilation.max_interface_nodes;
    let mut evidence = EvidenceBudget {
        remaining: limits.evidence,
        units: 0,
    };
    let mut emission = ir::Budget::new(Some(limits.compilation.lowering));
    let mut packages = BTreeMap::new();
    let mut unit_packages = Vec::new();
    for registration in &sources.inventory.sources {
        let next = packages.len();
        let id = *packages
            .entry(registration.package_root.clone())
            .or_insert(next);
        unit_packages.push(id);
    }
    let record_plan = if records {
        Some(records::RecordPlan::new(
            &sources,
            &unit_packages,
            &packages,
            &mut interface_nodes,
            &mut strings,
        )?)
    } else {
        None
    };
    let mut definitions = Vec::new();
    let mut same_package = BTreeMap::<usize, Vec<usize>>::new();
    let mut modules = BTreeMap::<usize, BTreeMap<String, Vec<usize>>>::new();
    for (unit_index, unit) in sources.units.iter().enumerate() {
        let registration = &sources.inventory.sources[unit.source_index];
        let path = &registration.canonical_path;
        let program = record_plan
            .as_ref()
            .map_or(&unit.program, |plan| &plan.programs[unit_index]);
        if !records {
            if let Some(record) = program
                .struct_declarations
                .iter()
                .chain(program.modules.iter().flat_map(|m| &m.struct_declarations))
                .next()
            {
                return Err(issue("E4163","registered record interfaces require source-aware type integration; context unavailable".into(),path,record.span).into());
            }
        }
        let scoped = semantic::resolve_source_type_scopes(program).map_err(|e| locate(e, path))?;
        let types = InterfaceTypes::new(&scoped).map_err(|e| locate(e, path))?;
        let mut seen = BTreeSet::new();
        for (module, functions) in std::iter::once((None, &scoped.functions)).chain(
            scoped
                .modules
                .iter()
                .map(|m| (Some(m.name.as_str()), &m.functions)),
        ) {
            let mut exports = Vec::new();
            for function in functions {
                if !seen.insert((module, &function.name)) {
                    return Err(issue(
                        "E3008",
                        diagnostic_message!("duplicate function `{}`", function.name),
                        path,
                        function.span,
                    )
                    .into());
                }
                if module.is_some() && !function.is_exported {
                    continue;
                }
                charge(
                    &mut interface_nodes,
                    1 + function.parameters.len(),
                    path,
                    function.span,
                )?;
                let normalized = semantic::normalize_imported_signatures(
                    &types,
                    &[function],
                    record_plan.as_ref().map_or(&HashMap::new(), |plan| {
                        if module.is_some() {
                            &plan.public_types
                        } else {
                            &plan.all_types
                        }
                    }),
                    &mut strings,
                )
                .map_err(|e| locate(e, path))?
                .remove(0);
                let array_nodes = normalized
                    .parameters
                    .iter()
                    .filter_map(|parameter| parameter.type_name.as_deref())
                    .chain(normalized.return_type.as_deref())
                    .map(|name| semantic::array_parts(name).map_or(0, |(depth, _)| depth))
                    .sum();
                charge(&mut interface_nodes, array_nodes, path, function.span)?;
                let name = spelling(module, &normalized.name, &mut strings, function.span)
                    .map_err(|e| locate(e, path))?;
                let linked = symbol(
                    unit_packages[unit_index],
                    &name,
                    &mut strings,
                    function.span,
                )
                .map_err(|e| locate(e, path))?;
                let index = definitions.len();
                definitions.push(Definition {
                    unit: unit_index,
                    spelling: name,
                    symbol: linked,
                    function: normalized,
                });
                same_package
                    .entry(unit_packages[unit_index])
                    .or_default()
                    .push(index);
                exports.push(index);
            }
            if let Some(module) = module {
                let name = strings.copy(module, zero()).map_err(|e| locate(e, path))?;
                modules
                    .entry(unit_packages[unit_index])
                    .or_default()
                    .insert(name, exports);
            }
        }
    }
    let mut checked = Vec::new();
    let mut ir = IrProgram {
        functions: Vec::new(),
    };
    for (unit_index, unit) in sources.units.iter().enumerate() {
        let registration = &sources.inventory.sources[unit.source_index];
        let path = &registration.canonical_path;
        let mut visible = BTreeMap::new();
        if let Some(candidates) = same_package.get(&unit_packages[unit_index]) {
            for &index in candidates {
                if definitions[index].unit != unit_index {
                    bind(
                        &mut visible,
                        &definitions,
                        index,
                        None,
                        zero(),
                        path,
                        &mut interface_nodes,
                        &mut strings,
                    )?;
                }
            }
        }
        let package = &sources.inventory.graph.packages[&registration.package_root];
        for import in &unit.imports {
            let target = package.dependencies.get(&import.alias).ok_or_else(|| {
                issue(
                    "E4115",
                    diagnostic_message!("undeclared dependency `{}`", import.alias),
                    path,
                    import.span,
                )
            })?;
            let exports = modules
                .get(&packages[target])
                .and_then(|modules| modules.get(&import.module))
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
            for &index in exports {
                bind(
                    &mut visible,
                    &definitions,
                    index,
                    Some(&import.alias),
                    import.span,
                    path,
                    &mut interface_nodes,
                    &mut strings,
                )?;
            }
        }
        let signatures = visible
            .iter()
            .map(|(name, binding)| {
                strings
                    .copy(name, binding.span)
                    .map(|name| (name, &definitions[binding.definition].function))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| locate(e, path))?;
        let bindings = signatures.iter().map(|(name, signature)| {
            let definition = &definitions[visible[name].definition];
            ImportBinding {
                spelling: name,
                owner: definition.unit,
                symbol: &definition.symbol,
                signature,
            }
        });
        let unit_checked = CheckedUnit::from_package_for_lowering(
            record_plan
                .as_ref()
                .map_or(&unit.program, |plan| &plan.programs[unit_index]),
            &signatures,
            unit_index,
            bindings,
            evidence.remaining.max_imports,
            evidence.remaining.max_import_bytes,
            evidence.structural(),
            evidence.remaining.semantic,
            evidence.remaining.flow,
            evidence.remaining.lowering,
        )
        .map_err(|error| match error {
            crate::compiler::checked::EvidenceError::Check(
                crate::compiler::checked::CheckError::Diagnostics(errors),
            ) => CheckedPackageError::Diagnostics(locate(errors, path)),
            error => CheckedPackageError::Evidence {
                entry: path.clone(),
                error,
            },
        })?;
        evidence.charge(&unit_checked);
        let mut targets = HashMap::new();
        for (name, binding) in &visible {
            targets.insert(
                strings
                    .copy(name, binding.span)
                    .map_err(|e| locate(e, path))?,
                strings
                    .copy(&definitions[binding.definition].symbol, binding.span)
                    .map_err(|e| locate(e, path))?,
            );
        }
        for (module, functions) in std::iter::once((None, &unit.program.functions)).chain(
            unit.program
                .modules
                .iter()
                .map(|m| (Some(m.name.as_str()), &m.functions)),
        ) {
            for function in functions {
                let name = spelling(module, &function.name, &mut strings, function.span)
                    .map_err(|e| locate(e, path))?;
                let linked = symbol(
                    unit_packages[unit_index],
                    &name,
                    &mut strings,
                    function.span,
                )
                .map_err(|e| locate(e, path))?;
                targets.insert(name, linked);
            }
        }
        let runtime_types = if let Some(plan) = &record_plan {
            plan.runtime_types(unit_index, path, &mut strings)?
        } else {
            HashMap::new()
        };
        emission.link_names(targets, runtime_types);
        let lowered = ir::lower_checked_with_budget(&unit_checked, &mut emission).map_err(
            |error| match error {
                ir::CheckedLoweringError::Emission(errors) => {
                    CheckedPackageError::Diagnostics(locate(errors, path))
                }
                ir::CheckedLoweringError::EvidenceUnavailable => {
                    unreachable!("lowering-ready package constructor")
                }
            },
        )?;
        ir.functions.extend(lowered.functions);
        checked.push(unit_checked);
    }
    Ok(RegisteredFunctionPackage {
        sources,
        checked,
        ir,
        remaining: evidence.remaining,
    })
}

#[allow(clippy::too_many_arguments)]
fn bind(
    visible: &mut BTreeMap<String, Binding>,
    definitions: &[Definition],
    index: usize,
    alias: Option<&str>,
    span: Span,
    path: &Path,
    nodes: &mut usize,
    strings: &mut InterfaceStrings,
) -> Result<(), Diagnostics> {
    let definition = &definitions[index];
    let arrays: usize = definition
        .function
        .parameters
        .iter()
        .filter_map(|parameter| parameter.type_name.as_deref())
        .chain(definition.function.return_type.as_deref())
        .map(|name| semantic::array_parts(name).map_or(0, |(depth, _)| depth))
        .sum();
    charge(
        nodes,
        1 + definition.function.parameters.len() + arrays,
        path,
        span,
    )?;
    let name = if let Some(alias) = alias {
        strings
            .charge(alias.len(), span)
            .map_err(|e| locate(e, path))?;
        strings.charge(2, span).map_err(|e| locate(e, path))?;
        strings
            .charge(definition.spelling.len(), span)
            .map_err(|e| locate(e, path))?;
        format!("{alias}::{}", definition.spelling)
    } else {
        strings
            .copy(&definition.spelling, span)
            .map_err(|e| locate(e, path))?
    };
    if let Some(previous) = visible.get(&name) {
        if previous.definition != index {
            return Err(issue(
                "E3008",
                "ambiguous imported function".into(),
                path,
                span,
            ));
        }
    }
    visible.insert(
        name,
        Binding {
            definition: index,
            span,
        },
    );
    Ok(())
}

#[cfg(all(
    test,
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod tests {
    use super::*;
    #[test]
    fn array_interfaces_charge_each_imported_copy_at_the_exact_boundary() {
        let root = std::env::temp_dir().join(format!(
            "sovra-array-interface-budget-{}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        struct Temp(std::path::PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _temp = Temp(root.clone());
        std::fs::write(
            root.join("sovra.toml"),
            "[project]\nname=\"arrays\"\nentry=\"main.svr\"\n[modules]\nnumbers=\"numbers.svr\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("main.svr"),
            "fn main(){print(numbers::echo([1])[0]);}",
        )
        .unwrap();
        std::fs::write(
            root.join("numbers.svr"),
            "mod numbers{export fn echo(values:[Int])->[Int]{return values;}}",
        )
        .unwrap();
        let inputs = load_package_sources(&root).unwrap();
        let mut limits = Limits::DEFAULT;
        limits.compilation.max_interface_nodes = 10;
        assert_eq!(
            crate::compiler::interpreter::run(compile(inputs.clone(), limits).unwrap().ir())
                .unwrap(),
            ["1"]
        );
        limits.compilation.max_interface_nodes = 9;
        assert_eq!(
            compile(inputs, limits)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3023"
        );
    }
    #[test]
    fn interface_string_evidence_and_emission_exhaustion_never_fall_back() {
        let loaded = load_package_sources("examples/multi-file-packages/app").unwrap();
        assert!(compile(loaded.clone(), Limits::DEFAULT).is_ok());
        let mut limits = Limits::DEFAULT;
        limits.compilation.max_interface_nodes = 0;
        assert_eq!(
            compile(loaded.clone(), limits)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3023"
        );
        let mut limits = Limits::DEFAULT;
        limits.strings = 0;
        assert_eq!(
            compile(loaded.clone(), limits)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3024"
        );
        let mut limits = Limits::DEFAULT;
        limits.evidence.max_imports = 0;
        assert!(matches!(
            compile(loaded.clone(), limits),
            Err(CheckedPackageError::Evidence { .. })
        ));
        let mut limits = Limits::DEFAULT;
        limits.compilation.lowering.max_instructions = 0;
        assert!(compile(loaded, limits)
            .unwrap_err()
            .into_diagnostics()
            .items
            .iter()
            .any(|d| d.code == "E3021"));
    }
    #[test]
    fn exact_and_exceeded_interface_nodes_include_repeated_import_copies() {
        let loaded = load_package_sources("examples/multi-file-packages/app").unwrap();
        // Provider signatures charge 4 nodes; source-local interface copies charge 6.
        let mut limits = Limits::DEFAULT;
        limits.compilation.max_interface_nodes = 10;
        assert!(compile(loaded.clone(), limits).is_ok());
        limits.compilation.max_interface_nodes = 9;
        assert_eq!(
            compile(loaded, limits)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3023"
        );
        struct Temp(std::path::PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let temp_path = std::env::temp_dir().join(format!(
            "sovra-repeated-registered-import-{}",
            std::process::id()
        ));
        std::fs::create_dir(&temp_path).unwrap();
        let temp = Temp(temp_path);
        for relative in [
            "app/sovra.toml",
            "app/main.svr",
            "app/application.svr",
            "utilities/sovra.toml",
            "utilities/main.svr",
            "utilities/arithmetic.svr",
        ] {
            let target = temp.0.join(relative);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(
                std::path::Path::new("examples/multi-file-packages").join(relative),
                target,
            )
            .unwrap();
        }
        let path = temp.0.join("app/application.svr");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, format!("use utilities::arithmetic;\n{text}")).unwrap();
        let repeated = load_package_sources(temp.0.join("app")).unwrap();
        limits.compilation.max_interface_nodes = 10;
        assert_eq!(
            compile(repeated.clone(), limits)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3023"
        );
        limits.compilation.max_interface_nodes = 12;
        assert_eq!(
            crate::compiler::interpreter::run(compile(repeated, limits).unwrap().ir()).unwrap(),
            ["49"]
        );
    }
    #[test]
    fn record_interfaces_charge_nodes_strings_evidence_and_shared_emission() {
        let loaded = load_package_sources("examples/multi-file-records/app").unwrap();
        assert!(compile_mode(loaded.clone(), Limits::DEFAULT, true).is_ok());
        let mut too_small = Limits::DEFAULT;
        too_small.compilation.max_interface_nodes = 11;
        let errors = compile_mode(loaded.clone(), too_small, true)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .any(|d| d.code == "E3023" && d.message.contains("record shape copies")));
        // Record metadata/aliases/copies cost 15 nodes; callable interfaces cost 10;
        // public/all standard-record identity entries cost 10.
        let mut exact = Limits::DEFAULT;
        exact.compilation.max_interface_nodes = 35;
        assert!(compile_mode(loaded.clone(), exact, true).is_ok());
        exact.compilation.max_interface_nodes = 34;
        assert_eq!(
            compile_mode(loaded.clone(), exact, true)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3023"
        );

        let mut limits = Limits::DEFAULT;
        limits.compilation.max_interface_nodes = 0;
        assert_eq!(
            compile_mode(loaded.clone(), limits, true)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3023"
        );
        let mut limits = Limits::DEFAULT;
        limits.strings = 0;
        assert_eq!(
            compile_mode(loaded.clone(), limits, true)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E3024"
        );
        let mut limits = Limits::DEFAULT;
        limits.evidence.max_nodes = 0;
        assert!(matches!(
            compile_mode(loaded.clone(), limits, true),
            Err(CheckedPackageError::Evidence { .. })
        ));
        let mut limits = Limits::DEFAULT;
        limits.compilation.lowering.max_instructions = 0;
        assert!(compile_mode(loaded, limits, true)
            .unwrap_err()
            .into_diagnostics()
            .items
            .iter()
            .any(|d| d.code == "E3021"));
    }
}
