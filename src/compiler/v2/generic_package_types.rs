//! Semantic package type/signature proofs, without executable body/linker authority.
use super::*;
use crate::compiler::diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span};
use std::{collections::BTreeMap, sync::Arc};
/// Aggregate package type/signature admission; every existing stage remains bounded.
#[derive(Debug, Clone, Copy)]
pub struct GenericPackageTypeLimits {
    /// Parameter-scope context records, hard capped at 8192.
    pub max_scopes: usize,
    /// Context mapping visits and copied/compared declaration-name bytes.
    pub max_context_work: usize,
    /// Whole type registry limits, including injected standard types.
    pub registry: GenericRegistryLimits,
    /// Whole transparent alias environment limits.
    pub aliases: GenericAliasEnvironmentLimits,
    /// Whole record/enum/alias definition limits.
    pub declarations: GenericDeclarationLimits,
    /// All function signatures across all source scopes.
    pub signatures: GenericSignatureLimits,
}
impl GenericPackageTypeLimits {
    /// Finite defaults; no per-file reset of semantic allowances.
    pub const DEFAULT: Self = Self {
        max_scopes: 8192,
        max_context_work: 1_000_000,
        registry: GenericRegistryLimits::DEFAULT,
        aliases: GenericAliasEnvironmentLimits::DEFAULT,
        declarations: GenericDeclarationLimits::DEFAULT,
        signatures: GenericSignatureLimits::DEFAULT,
    };
}
/// Sealed semantic type definitions and signatures retaining original package owners.
#[derive(Debug)]
pub struct CheckedGenericPackageTypes {
    table: Arc<GenericPackageDeclarations>,
    types: CheckedGenericDeclarations,
    signatures: CheckedGenericSignatures,
    type_ids: BTreeMap<GenericPackageDeclarationId, GenericTypeDeclarationId>,
    function_ids: BTreeMap<GenericPackageDeclarationId, GenericSignatureId>,
}
impl CheckedGenericPackageTypes {
    /// Original source/import/visibility table.
    pub fn declarations(&self) -> &GenericPackageDeclarations {
        &self.table
    }
    /// Complete checked definitions, including compiler-owned standard types.
    pub fn types(&self) -> &CheckedGenericDeclarations {
        &self.types
    }
    /// Checked signatures. Flat body compilation and unscoped name lookup stay gated.
    pub fn signatures(&self) -> &CheckedGenericSignatures {
        &self.signatures
    }
    pub(super) fn declaration_for_signature(
        &self,
        id: GenericSignatureId,
    ) -> Option<GenericPackageDeclarationId> {
        self.signatures.signature(id)?;
        self.function_ids
            .iter()
            .find(|(_, signature)| **signature == id)
            .map(|(declaration, _)| *declaration)
    }
    pub(super) fn declaration_for_type(
        &self,
        id: GenericTypeDeclarationId,
    ) -> Option<GenericPackageDeclarationId> {
        self.types.definition(id)?;
        self.type_ids
            .iter()
            .find(|(_, nominal)| **nominal == id)
            .map(|(declaration, _)| *declaration)
    }
    /// Map only a declaration identity owned by this package table to its nominal ID.
    pub fn type_id(&self, id: GenericPackageDeclarationId) -> Option<GenericTypeDeclarationId> {
        self.type_ids.get(&id).copied()
    }
    /// Map only an owned function declaration to its checked signature identity.
    pub fn signature_id(&self, id: GenericPackageDeclarationId) -> Option<GenericSignatureId> {
        self.function_ids.get(&id).copied()
    }
}
pub(super) fn failure(
    table: &GenericPackageDeclarations,
    location: Option<(GenericPackageDeclarationId, Option<Span>)>,
    message: String,
) -> Diagnostics {
    let mut span = Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    let mut source_file = None;
    if let Some((id, local)) = location {
        if let Some(declaration) = table.declaration(id) {
            span = declaration.syntax().span();
            if let Some(local) = local {
                if local.start <= local.end && local.end <= span.end - span.start {
                    span = Span {
                        start: span.start + local.start,
                        end: span.start + local.end,
                        line: span.line + local.line,
                        column: if local.line == 0 {
                            span.column + local.column
                        } else {
                            local.column
                        },
                    };
                }
            }
            source_file = table
                .source_registration(declaration.source())
                .map(|s| s.canonical_path.to_string_lossy().into_owned());
        }
    }
    Diagnostics {
        items: vec![Diagnostic {
            severity: Severity::Error,
            code: "E3123",
            message,
            span,
            source_file,
        }
        .bounded()],
    }
}
/// Check all alias targets, fields, enum payloads and function signatures using
/// the exact importing source's visibility. Same-spelled nominal types retain
/// distinct registry identities. This never checks or executes a function body.
pub fn check_generic_package_types(
    table: GenericPackageDeclarations,
    limits: GenericPackageTypeLimits,
) -> Result<CheckedGenericPackageTypes, Diagnostics> {
    check_package_types(table, limits, false, false)
}
/// Opt-in semantic collection annotation checking only; body/layout/VM/native admission is separate.
/// Uses the same cumulative source visibility, arity, Key-bound and metadata profiles.
pub fn check_generic_collection_package_types(
    table: GenericPackageDeclarations,
    limits: GenericPackageTypeLimits,
) -> Result<CheckedGenericPackageTypes, Diagnostics> {
    check_package_types(table, limits, true, false)
}
/// Check Byte/Bytes annotations and Byte Key bounds with the collection package schema.
/// This semantic API does not authorize executable data layouts, CLI or native features.
pub fn check_generic_data_package_types(
    table: GenericPackageDeclarations,
    limits: GenericPackageTypeLimits,
) -> Result<CheckedGenericPackageTypes, Diagnostics> {
    check_package_types(table, limits, true, true)
}
fn check_package_types(
    table: GenericPackageDeclarations,
    mut limits: GenericPackageTypeLimits,
    collections: bool,
    data: bool,
) -> Result<CheckedGenericPackageTypes, Diagnostics> {
    let table = Arc::new(table);
    let mut scopes = BTreeMap::new();
    let mut type_indices = BTreeMap::new();
    let (mut inputs, mut aliases, mut functions, mut type_declarations, mut function_declarations) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut scope_left = limits.max_scopes.min(8192);
    for declaration in table.declarations() {
        scope_left = scope_left
            .checked_sub(1)
            .ok_or_else(|| failure(&table, None, "package type scope allowance exceeded".into()))?;
        limits.max_context_work = limits
            .max_context_work
            .checked_sub(declaration.syntax().name().len() + 1)
            .ok_or_else(|| failure(&table, None, "package type context work exceeded".into()))?;
        let parameters = match declaration.syntax().kind() {
            GenericSourceDeclarationKind::Alias(a) => {
                aliases.push(a);
                type_indices.insert(declaration.id(), inputs.len());
                inputs.push(GenericTypeDeclaration::Alias(a));
                type_declarations.push(declaration.id());
                a.parameters()
            }
            GenericSourceDeclarationKind::Record(r) => {
                type_indices.insert(declaration.id(), inputs.len());
                inputs.push(GenericTypeDeclaration::Record(r));
                type_declarations.push(declaration.id());
                r.parameters()
            }
            GenericSourceDeclarationKind::Enum(e) => {
                type_indices.insert(declaration.id(), inputs.len());
                inputs.push(GenericTypeDeclaration::Enum(e));
                type_declarations.push(declaration.id());
                e.parameters()
            }
            GenericSourceDeclarationKind::Function(f) => {
                functions.push(f);
                function_declarations.push(declaration.id());
                f.parameters()
            }
        };
        if scopes
            .insert(parameters.scope_key(), Some(declaration.source()))
            .is_some()
        {
            return Err(failure(
                &table,
                Some((declaration.id(), None)),
                "duplicate package parameter owner".into(),
            ));
        }
    }
    let mut standards = super::generic_enums::standard_generic_enums()
        .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?
        .to_vec();
    if data {
        standards.push(
            super::generic_enums::standard_json_enum()
                .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?,
        );
    }

    let error = super::generic_records::standard_error_record()
        .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?;
    let results = super::generic_records::standard_result_records()
        .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?;
    let collection_types = if collections {
        super::standard_generic_collection_types()
            .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?
    } else {
        Vec::new()
    };
    let entry = if collections {
        Some(
            super::generic_records::standard_map_entry_record()
                .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?,
        )
    } else {
        None
    };
    let data_types = if data {
        super::standard_generic_data_types()
            .map_err(|e| failure(&table, None, diagnostic_message!("{e}")))?
    } else {
        Vec::new()
    };
    for item in standards
        .iter()
        .map(GenericTypeDeclaration::Enum)
        .chain(std::iter::once(GenericTypeDeclaration::Record(&error)))
        .chain(results.iter().map(GenericTypeDeclaration::Record))
        .chain(entry.iter().map(GenericTypeDeclaration::Record))
        .chain(data_types.iter().map(GenericTypeDeclaration::Data))
        .chain(
            collection_types
                .iter()
                .map(GenericTypeDeclaration::Collection),
        )
    {
        scope_left = scope_left
            .checked_sub(1)
            .ok_or_else(|| failure(&table, None, "package type scope allowance exceeded".into()))?;
        let (name, parameters, _, _) = item.parts();
        limits.max_context_work = limits
            .max_context_work
            .checked_sub(name.len() + 1)
            .ok_or_else(|| failure(&table, None, "package type context work exceeded".into()))?;
        scopes.insert(parameters.scope_key(), None);
        inputs.push(item);
    }
    let registry = super::generic_resolution::register_package_types(
        &inputs,
        limits.registry,
        super::generic_resolution::PackageTypeContext {
            table: Arc::clone(&table),
            scopes,
            types: type_indices,
        },
    )
    .map_err(|e| {
        let location = match &e {
            GenericResolutionError::Declaration { index, span, .. } => {
                type_declarations.get(*index).map(|id| (*id, Some(*span)))
            }
            _ => None,
        };
        failure(&table, location, diagnostic_message!("{e}"))
    })?;
    let location =
        |id: GenericTypeDeclarationId| type_declarations.get(id.ordinal()).map(|id| (*id, None));
    let environment =
        prepare_generic_aliases(&registry, &aliases, limits.aliases).map_err(|e| {
            let at = match &e {
                GenericAliasExpansionError::Cycle(id) => location(*id),
                GenericAliasExpansionError::Resolution {
                    alias: Some(id), ..
                } => location(*id),
                GenericAliasExpansionError::Bound(origin) => origin.alias.and_then(location),
                _ => None,
            };
            failure(&table, at, diagnostic_message!("{e}"))
        })?;
    let types =
        check_generic_declarations(&environment, &inputs, limits.declarations).map_err(|e| {
            let at = match &e {
                GenericDeclarationError::Ownership { input, span, .. } => {
                    type_declarations.get(*input).map(|id| (*id, Some(*span)))
                }
                GenericDeclarationError::Annotation {
                    declaration,
                    annotation_range,
                    ..
                } => type_declarations
                    .get(declaration.ordinal())
                    .map(|id| (*id, Some(*annotation_range))),
                GenericDeclarationError::EmptyEnum { declaration, span } => type_declarations
                    .get(declaration.ordinal())
                    .map(|id| (*id, Some(*span))),
                GenericDeclarationError::Missing(id) => location(*id),
                _ => None,
            };
            failure(&table, at, diagnostic_message!("{e}"))
        })?;
    let signatures =
        check_generic_signatures(&types, &functions, limits.signatures).map_err(|e| {
            let at = match &e {
                GenericSignatureError::Definition { function, span, .. } => function_declarations
                    .get(*function)
                    .map(|id| (*id, Some(*span))),
                GenericSignatureError::Annotation {
                    function,
                    annotation_range,
                    ..
                } => function_declarations
                    .get(*function)
                    .map(|id| (*id, Some(*annotation_range))),
                _ => None,
            };
            failure(&table, at, diagnostic_message!("{e}"))
        })?;
    let type_ids = type_declarations
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, registry.headers()[index].id))
        .collect();
    let function_ids = function_declarations
        .into_iter()
        .zip(signatures.signatures().iter().map(|s| s.id()))
        .collect();
    drop(inputs);
    drop(aliases);
    drop(functions);
    Ok(CheckedGenericPackageTypes {
        table,
        types,
        signatures,
        type_ids,
        function_ids,
    })
}
