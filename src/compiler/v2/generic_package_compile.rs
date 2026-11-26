//! Owned, selected-root package compilation and original-file runtime diagnostics.
use super::*;
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostics, Span},
    project::packages::{CapturedPackageSources, PackageSourceKind},
};
use std::{path::PathBuf, sync::Arc};
/// Finite cumulative stages for one selected package root.
#[derive(Debug, Clone, Copy)]
pub struct GenericPackageCompilationLimits {
    /// Whole-graph syntax/import admission.
    pub declarations: GenericPackageDeclarationLimits,
    /// Whole-graph semantic types and signatures.
    pub types: GenericPackageTypeLimits,
    /// Every original symbolic function body.
    pub bodies: GenericCopyBodyLimits,
    /// Shared propagation site/type compatibility admission.
    pub propagation: GenericPropagationLimits,
    /// Selected root's finite concrete closure.
    pub specialization: GenericSpecializationLimits,
    /// Whole concrete operation/type recheck.
    pub concrete: GenericConcreteBodyLimits,
    /// Source-owned literal decoding.
    pub literals: GenericLiteralLimits,
    /// Concrete nominal/value layouts.
    pub layouts: GenericLayoutLimits,
    /// Checked propagation operation layouts.
    pub operations: GenericPropagationLayoutLimits,
    /// Complete nominal construction plans.
    pub construction: GenericConstructionLimits,
    /// Complete instruction emission and validation.
    pub ir: GenericIrLimits,
}
impl GenericPackageCompilationLimits {
    /// Approved package instance ceiling with finite existing stage defaults.
    pub const DEFAULT: Self = Self {
        declarations: GenericPackageDeclarationLimits::DEFAULT,
        types: GenericPackageTypeLimits::DEFAULT,
        bodies: GenericCopyBodyLimits::PACKAGE,
        propagation: GenericPropagationLimits::DEFAULT,
        specialization: GenericSpecializationLimits::PACKAGE,
        concrete: GenericConcreteBodyLimits::DEFAULT,
        literals: GenericLiteralLimits::DEFAULT,
        layouts: GenericLayoutLimits::DEFAULT,
        operations: GenericPropagationLayoutLimits::DEFAULT,
        construction: GenericConstructionLimits::DEFAULT,
        ir: GenericIrLimits::DEFAULT,
    };
}
/// One original source-file operation location, never a synthesized combined file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericPackageLocation {
    /// Canonical owning captured source locator; no disk reread is implied.
    pub source_file: PathBuf,
    /// Byte range and scalar line/column in that file.
    pub span: Span,
}
/// Complete checked IR plus immutable snapshots and original ownership mappings.
#[derive(Debug, Clone)]
pub struct CompiledGenericPackage {
    bodies: Arc<CheckedGenericPackageBodies>,
    root: GenericPackageDeclarationId,
    entry: Arc<str>,
    ir: CheckedGenericPropagationIr,
}
impl CompiledGenericPackage {
    /// Selected unqualified function name in the root package's entry source.
    pub fn entry(&self) -> &str {
        &self.entry
    }
    /// Exact source-owned selected declaration identity.
    pub fn entry_declaration(&self) -> GenericPackageDeclarationId {
        self.root
    }
    /// Retained source/type/body proofs, including captured dependency graph bytes.
    pub fn bodies(&self) -> &CheckedGenericPackageBodies {
        &self.bodies
    }
    /// Checked propagation-aware IR for explicit backend inspection/execution.
    pub fn ir(&self) -> &CheckedGenericPropagationIr {
        &self.ir
    }
    /// Require the command/native convention separately from library compilation.
    pub fn validate_command_entry(&self) -> Result<(), Diagnostics> {
        let ir = self.ir.checked_ir();
        let root = &ir.functions()[0];
        if self.entry() != "main"
            || !matches!(
                ir.plans().layouts().layouts()[root.return_layout()].kind(),
                GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Unit)
            )
        {
            return Err(super::generic_package_types::failure(
                self.bodies.types().declarations(),
                Some((self.root, None)),
                "generic package execution requires a parameterless Unit main function".into(),
            ));
        }
        Ok(())
    }
    /// Translate a concrete instance's local byte range to its original file.
    /// Caller-supplied line/column fields are ignored; out-of-range indices, invalid
    /// ranges and UTF-8 interiors refuse without guessing a location.
    pub fn location(&self, instance: usize, local: Span) -> Option<GenericPackageLocation> {
        let graph = self
            .ir
            .checked_ir()
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph();
        let function = graph.instances().get(instance)?.function();
        let types = self.bodies.types();
        let id = types.declaration_for_signature(function)?;
        let declaration = types.declarations().declaration(id)?;
        let GenericSourceDeclarationKind::Function(syntax) = declaration.syntax().kind() else {
            return None;
        };
        syntax.source().get(local.start..local.end)?;
        let prefix = syntax.source().get(..local.start)?;
        let lines = prefix.bytes().filter(|b| *b == b'\n').count();
        let origin = declaration.syntax().span();
        let span = Span {
            start: origin.start.checked_add(local.start)?,
            end: origin.start.checked_add(local.end)?,
            line: origin.line.checked_add(lines)?,
            column: if lines == 0 {
                origin.column.checked_add(prefix.chars().count())?
            } else {
                prefix.rsplit('\n').next()?.chars().count()
            },
        };
        let registration = types
            .declarations()
            .source_registration(declaration.source())?;
        Some(GenericPackageLocation {
            source_file: registration.canonical_path.clone(),
            span,
        })
    }
}
fn diagnostic(
    bodies: &CheckedGenericPackageBodies,
    location: Option<(GenericSignatureId, Option<Span>)>,
    message: String,
) -> Diagnostics {
    let location = location.and_then(|(function, span)| {
        bodies
            .types()
            .declaration_for_signature(function)
            .map(|id| (id, span))
    });
    super::generic_package_types::failure(bodies.types().declarations(), location, message)
}
/// Compile exactly the supplied snapshots, selecting only a function in the root
/// package entry. No filesystem rereads, lock verification, execution or writes.
/// All original types and bodies are checked before selecting the concrete closure.
pub fn compile_generic_package(
    inputs: CapturedPackageSources,
    entry: &str,
    limits: GenericPackageCompilationLimits,
) -> Result<CompiledGenericPackage, Diagnostics> {
    compile_package_inputs(inputs.into(), entry, limits)
}
/// Check unsaved registered sources against a guarded captured generic package.
/// Returns diagnostics only: no executable, capture/lock authority or host grants escape.
pub fn check_generic_package_with_overlays(
    root: impl AsRef<std::path::Path>,
    overlays: &[crate::compiler::project::packages::SourceOverlay],
    limits: crate::compiler::project::packages::SourceOverlayLimits,
) -> Result<(), Diagnostics> {
    compile_overlay_package(root.as_ref(), overlays, limits).map(|_| ())
}
/// Check unsaved generic sources and project optional bounded package navigation.
/// Returns only JSON metadata; quota refusal does not invalidate source checking.
pub fn check_generic_package_overlays_with_symbols(
    root: impl AsRef<std::path::Path>,
    overlays: &[crate::compiler::project::packages::SourceOverlay],
    limits: crate::compiler::project::packages::SourceOverlayLimits,
    symbols: GenericSymbolLimits,
) -> Result<String, Diagnostics> {
    let program = compile_overlay_package(root.as_ref(), overlays, limits)?;
    Ok(super::generic_package_symbols::package_symbols(
        &program, symbols,
    ))
}
fn compile_overlay_package(
    root: &std::path::Path,
    overlays: &[crate::compiler::project::packages::SourceOverlay],
    limits: crate::compiler::project::packages::SourceOverlayLimits,
) -> Result<CompiledGenericPackage, Diagnostics> {
    use crate::compiler::{
        diagnostics::{Diagnostic, Severity},
        project::packages::*,
    };
    use std::collections::BTreeMap;
    let fail = |message: &str| Diagnostics {
        items: vec![Diagnostic {
            severity: Severity::Error,
            code: "E4118",
            message: message.into(),
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
            source_file: None,
        }],
    };
    let defaults = SourceOverlayLimits::DEFAULT;
    if overlays.len() > limits.max_files.min(defaults.max_files) {
        return Err(fail("generic unsaved buffer count allowance exceeded"));
    }
    let mut remaining = limits
        .max_total_source_bytes
        .min(defaults.max_total_source_bytes);
    for source in overlays {
        if source.source.len() > limits.max_source_bytes.min(defaults.max_source_bytes)
            || source.source.len() > remaining
        {
            return Err(fail("generic unsaved buffer byte allowance exceeded"));
        }
        remaining -= source.source.len();
    }
    let captured = capture_package_sources(root, PackageCaptureLimits::DEFAULT)?;
    let mut selected = BTreeMap::new();
    for source in overlays {
        let path = source
            .entry
            .canonicalize()
            .map_err(|_| fail("generic unsaved source must exist"))?;
        if !captured
            .inventory()
            .sources
            .iter()
            .any(|s| s.canonical_path == path)
            || selected.insert(path, source.source.as_str()).is_some()
        {
            return Err(fail("generic unsaved source is unrelated or duplicated"));
        }
    }
    let mut bytes = 0usize;
    for unit in captured.sources() {
        let path = &captured.inventory().sources[unit.source_index()].canonical_path;
        let text = selected.get(path).copied().unwrap_or_else(|| unit.source());
        bytes = bytes
            .checked_add(text.len())
            .filter(|n| *n <= PackageCaptureLimits::DEFAULT.max_source_bytes)
            .ok_or_else(|| fail("generic overlaid package byte allowance exceeded"))?;
    }
    // Admission precedes copies; overlay bytes never become a guarded disk capture.
    let sources = captured
        .sources()
        .iter()
        .map(|unit| {
            let path = &captured.inventory().sources[unit.source_index()].canonical_path;
            CapturedPackageSource::embedded(
                unit.source_index(),
                selected
                    .get(path)
                    .copied()
                    .unwrap_or_else(|| unit.source())
                    .to_owned(),
            )
        })
        .collect();
    let inputs = super::GenericPackageSources::overlay(
        super::generic_package_inputs::EmbeddedPackageSources {
            inventory: captured.inventory().clone(),
            sources,
            bytes,
        },
    );
    compile_package_inputs(inputs, "main", GenericPackageCompilationLimits::DEFAULT)
}
/// Compile the explicit initial collection operation profile; CLI/native remain separate.
pub fn compile_generic_collection_package(
    inputs: CapturedPackageSources,
    entry: &str,
    limits: GenericPackageCompilationLimits,
) -> Result<CompiledGenericPackage, Diagnostics> {
    compile_package_inputs_mode(inputs.into(), entry, limits, true)
}
pub(super) fn compile_package_inputs(
    inputs: super::GenericPackageSources,
    entry: &str,
    limits: GenericPackageCompilationLimits,
) -> Result<CompiledGenericPackage, Diagnostics> {
    compile_package_inputs_mode(inputs, entry, limits, false)
}
/// Compile explicit data operations without enabling CLI/native profiles.
pub fn compile_generic_data_package(
    inputs: CapturedPackageSources,
    entry: &str,
    limits: GenericPackageCompilationLimits,
) -> Result<CompiledGenericPackage, Diagnostics> {
    compile_package_inputs_profile(inputs.into(), entry, limits, true, true)
}
pub(super) fn compile_package_inputs_mode(
    inputs: super::GenericPackageSources,
    entry: &str,
    limits: GenericPackageCompilationLimits,
    collections: bool,
) -> Result<CompiledGenericPackage, Diagnostics> {
    compile_package_inputs_profile(inputs, entry, limits, collections, false)
}
pub(super) fn compile_package_inputs_profile(
    inputs: super::GenericPackageSources,
    entry: &str,
    limits: GenericPackageCompilationLimits,
    collections: bool,
    data: bool,
) -> Result<CompiledGenericPackage, Diagnostics> {
    if entry.len() > limits.declarations.syntax.max_entry_bytes.min(1024) {
        use crate::compiler::diagnostics::{Diagnostic, Severity};
        return Err(Diagnostics {
            items: vec![Diagnostic {
                severity: Severity::Error,
                code: "E3123",
                message: "generic package entry name allowance exceeded".into(),
                source_file: None,
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
            }
            .bounded()],
        });
    }

    let table = super::generic_package::bind_package_inputs(inputs, limits.declarations)?;
    let root_source = table
        .inputs()
        .inventory()
        .sources
        .iter()
        .position(|s| {
            s.package_root == table.inputs().inventory().graph.root
                && s.kind == PackageSourceKind::Entry
        })
        .and_then(|i| table.source_id(i));
    let selected = root_source
        .and_then(|source| table.resolve(source, &[], GenericPackageNamespace::Function, entry));
    let types = if data {
        check_generic_data_package_types(table, limits.types)
    } else if collections {
        check_generic_collection_package_types(table, limits.types)
    } else {
        check_generic_package_types(table, limits.types)
    }?;
    let bodies = if data {
        check_generic_data_package_bodies(types, limits.bodies, limits.propagation)
    } else if collections {
        check_generic_collection_package_bodies(types, limits.bodies, limits.propagation)
    } else {
        check_generic_package_bodies(types, limits.bodies, limits.propagation)
    }?;
    let root = selected.ok_or_else(|| {
        diagnostic(
            &bodies,
            None,
            "selected function is absent from root package entry".into(),
        )
    })?;
    compile_package_root_profile(Arc::new(bodies), root, limits, collections, data)
}
pub(super) fn compile_package_root_profile(
    bodies: Arc<CheckedGenericPackageBodies>,
    root: GenericPackageDeclarationId,
    limits: GenericPackageCompilationLimits,
    collections: bool,
    data: bool,
) -> Result<CompiledGenericPackage, Diagnostics> {
    let table = bodies.types().declarations();
    let declaration = table
        .declaration(root)
        .ok_or_else(|| diagnostic(&bodies, None, "foreign package root declaration".into()))?;
    let source = table
        .source_registration(declaration.source())
        .ok_or_else(|| diagnostic(&bodies, None, "package root source is unavailable".into()))?;
    if source.kind != PackageSourceKind::Entry
        || source.package_root != table.inputs().inventory().graph.root
        || declaration.syntax().name().len() > limits.declarations.syntax.max_entry_bytes.min(1024)
    {
        return Err(diagnostic(
            &bodies,
            None,
            "selected function must belong to the root package entry".into(),
        ));
    }
    let entry: Arc<str> = Arc::from(declaration.syntax().name());
    let signature = bodies.types().signature_id(root).ok_or_else(|| {
        diagnostic(
            &bodies,
            None,
            "selected package signature is unavailable".into(),
        )
    })?;
    let checked = bodies
        .types()
        .signatures()
        .signature(signature)
        .ok_or_else(|| {
            diagnostic(
                &bodies,
                None,
                "selected package signature owner is invalid".into(),
            )
        })?;
    if !checked.parameter_types().is_empty()
        || !checked.syntax().parameters().parameters().is_empty()
    {
        return Err(diagnostic(
            &bodies,
            Some((signature, None)),
            "selected package root must be nongeneric and parameterless".into(),
        ));
    }
    let graph = discover_generic_propagation_specializations(
        bodies.bodies(),
        signature,
        limits.specialization,
    )
    .map_err(|e| diagnostic(&bodies, None, diagnostic_message!("{e}")))?;
    let instance = |index: usize, span: Option<Span>| {
        graph.instances().get(index).map(|i| (i.function(), span))
    };
    let concrete =
        recheck_generic_propagation_concrete_bodies(&graph, limits.concrete, limits.propagation)
            .map_err(|e| {
                let at = match &e {
                    GenericConcreteBodyError::Context {
                        instance: index, ..
                    } => instance(*index, None),
                    GenericConcreteBodyError::Propagation {
                        instance: index,
                        span,
                        ..
                    } => instance(*index, Some(*span)),
                    _ => None,
                };
                diagnostic(&bodies, at, diagnostic_message!("{e}"))
            })?;
    let literals = decode_generic_literals(&concrete, limits.literals).map_err(|e| {
        let at = match &e {
            GenericLiteralError::Source {
                instance: index,
                span,
            } => instance(*index, Some(*span)),
            GenericLiteralError::Lexical {
                instance: index,
                diagnostics,
            } => instance(*index, diagnostics.items.first().map(|d| d.span)),
            _ => None,
        };
        diagnostic(&bodies, at, diagnostic_message!("{e}"))
    })?;
    let layouts = (if data {
        plan_generic_data_layouts(&literals, limits.layouts)
    } else if collections {
        plan_generic_collection_layouts(&literals, limits.layouts)
    } else {
        plan_generic_layouts(&literals, limits.layouts)
    })
    .map_err(|e| {
        let at = if let GenericLayoutError::Member { declaration, .. } = &e {
            bodies
                .types()
                .declaration_for_type(*declaration)
                .map(|id| (id, None))
        } else {
            None
        };
        super::generic_package_types::failure(
            bodies.types().declarations(),
            at,
            diagnostic_message!("{e}"),
        )
    })?;
    let operations = plan_generic_propagation_layouts(&layouts, limits.operations)
        .map_err(|e| diagnostic(&bodies, None, diagnostic_message!("{e}")))?;
    let ir = (if data {
        lower_generic_data_propagation_ir(&operations, limits.construction, limits.ir)
    } else if collections {
        lower_generic_collection_propagation_ir(&operations, limits.construction, limits.ir)
    } else {
        lower_generic_propagation_ir(&operations, limits.construction, limits.ir)
    })
    .map_err(|e| {
        let at = match &e {
            GenericIrError::Invalid { function, span, .. } => instance(*function, Some(*span)),
            GenericIrError::Construction(GenericConstructionError::Context {
                instance: index,
                span,
                ..
            }) => instance(*index, Some(*span)),
            _ => None,
        };
        diagnostic(&bodies, at, diagnostic_message!("{e}"))
    })?;
    Ok(CompiledGenericPackage {
        bodies,
        root,
        entry,
        ir,
    })
}
/// Execute only retained checked IR under explicit finite policy. Runtime errors
/// identify the originating captured file, even after disk inputs have changed.
pub fn execute_generic_package(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, Diagnostics> {
    execute_package(program, policy, false)
}
/// Execute the explicitly compiled collection profile with shared VM limits/control.
pub fn execute_generic_collection_package(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, Diagnostics> {
    execute_package(program, policy, true)
}
fn execute_package(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
    collections: bool,
) -> Result<GenericVmOutcome, Diagnostics> {
    execute_package_profile(program, policy, collections, false)
}
/// Execute a checked data package through its explicit VM profile.
pub fn execute_generic_data_package(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
) -> Result<GenericVmOutcome, Diagnostics> {
    execute_package_profile(program, policy, true, true)
}
fn execute_package_profile(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
    collections: bool,
    data: bool,
) -> Result<GenericVmOutcome, Diagnostics> {
    let execute = if data {
        execute_generic_data_ir
    } else if collections {
        execute_generic_collection_ir
    } else {
        execute_generic_propagation_ir
    };
    execute(&program.ir, policy).map_err(|error| {
        let mut diagnostics = super::generic_package_types::failure(
            program.bodies.types().declarations(),
            None,
            diagnostic_message!("{error}"),
        );
        let diagnostic = &mut diagnostics.items[0];
        diagnostic.code = "E3125";
        if let Some(location) = program.location(error.function, error.span) {
            diagnostic.source_file = Some(location.source_file.to_string_lossy().into_owned());
            diagnostic.span = location.span;
        }
        diagnostics
    })
}

/// Execute a data package with explicitly supplied version-three host authority.
pub fn execute_generic_data_package_with_host(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
    host: &HostV3,
) -> Result<GenericVmOutcome, Diagnostics> {
    execute_generic_data_ir_with_host(&program.ir, policy, host).map_err(|e| {
        let mut out = super::generic_package_types::failure(
            program.bodies.types().declarations(),
            None,
            diagnostic_message!("{e}"),
        );
        out.items[0].code = "E3125";
        if let Some(location) = program.location(e.function, e.span) {
            out.items[0].source_file = Some(location.source_file.to_string_lossy().into_owned());
            out.items[0].span = location.span;
        }
        out
    })
}

/// Execute a data package with owned network authority, closing sockets on success or failure.
pub fn execute_generic_data_package_with_network_host(
    program: &CompiledGenericPackage,
    policy: GenericVmPolicy,
    host: super::NetworkHostV4,
) -> Result<GenericVmOutcome, Diagnostics> {
    execute_generic_data_ir_with_network_host(&program.ir, policy, host).map_err(|e| {
        let mut out = super::generic_package_types::failure(
            program.bodies.types().declarations(),
            None,
            diagnostic_message!("{e}"),
        );
        out.items[0].code = "E3125";
        if let Some(location) = program.location(e.function, e.span) {
            out.items[0].source_file = Some(location.source_file.to_string_lossy().into_owned());
            out.items[0].span = location.span;
        }
        out
    })
}
