//! Resolved primitive service contracts, separate from partial source inspection.

use std::path::PathBuf;

use super::{scope::ServiceIdentity, ProjectCheck};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::compiler::semantic::Type;

/// A service parameter whose annotation has been resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedServiceParameter {
    /// Source parameter name.
    pub name: String,
    /// Resolved canonical type, never `Unknown`.
    pub parameter_type: Type,
}

/// A resolved service operation interface; this does not validate its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedServiceOperation {
    /// Canonical source-module and service identity.
    pub service: ServiceIdentity,
    /// Source operation name.
    pub name: String,
    /// Resolved parameters in declaration order.
    pub parameters: Vec<TypedServiceParameter>,
    /// Resolved return type; an omitted annotation means Unit.
    pub return_type: Type,
    /// Whether a body was recorded by project inspection.
    pub has_body: bool,
    /// Original source path for diagnostics and inspection.
    pub source_file: PathBuf,
    /// Full declaration range supplied by project inspection.
    pub span: Span,
}

/// Service signature resolution with errors retained separately from valid records.
#[derive(Debug, Clone)]
pub struct ServiceSignatures {
    /// Only fully resolved operation signatures. Invalid operations are omitted.
    pub operations: Vec<TypedServiceOperation>,
    /// Missing/unsupported annotations or inaccessible source identities.
    pub diagnostics: Diagnostics,
}

/// Resolve service contract annotations using the executable primitive types.
///
/// Requires checked project metadata. No body or call-argument typing is implied.
/// Application named types, generics and executable package type imports are not
/// connected yet and produce E4117 rather than becoming placeholder types.
/// Missing annotations retain E4097. Owner-path failures produce E4118.
/// Source locations are declaration ranges, not individual annotation ranges.
pub fn resolve_service_signatures(project: &ProjectCheck) -> ServiceSignatures {
    let mut report = ServiceSignatures {
        operations: Vec::new(),
        diagnostics: Diagnostics::new(),
    };
    for operation in &project.service_operations {
        let start_errors = report.diagnostics.items.len();
        let mut emit = |code, message| {
            report.diagnostics.push(Diagnostic {
                source_file: Some(operation.source_file.to_string_lossy().into_owned()),
                severity: Severity::Error,
                code,
                message,
                span: operation.span,
            })
        };
        let module = match operation.source_file.canonicalize() {
            Ok(module) => module.to_string_lossy().into_owned(),
            Err(error) => {
                emit(
                    "E4118",
                    format!(
                        "cannot resolve source identity for service `{}.{}`: {error}",
                        operation.service, operation.name
                    ),
                );
                continue;
            }
        };
        let mut parameters = Vec::new();
        for parameter in &operation.parameters {
            let Some(annotation) = &parameter.annotation else {
                emit("E4097", format!("parameter `{}` in service operation `{}.{}` requires an explicit type annotation", parameter.name, operation.service, operation.name));
                continue;
            };
            let parameter_type = Type::from_name_with_known(annotation, None);
            if parameter_type == Type::Unknown {
                emit("E4117", format!("unresolved service parameter type `{annotation}` for `{}.{}.{}`; supported contract types are Unit, Bool, Int, Float and String", operation.service, operation.name, parameter.name));
            } else {
                parameters.push(TypedServiceParameter {
                    name: parameter.name.clone(),
                    parameter_type,
                });
            }
        }
        let return_type = match operation.return_annotation.as_deref() {
            None => Type::Unit,
            Some(annotation) => {
                let resolved = Type::from_name_with_known(annotation, None);
                if resolved == Type::Unknown {
                    emit("E4117", format!("unresolved service return type `{annotation}` for `{}.{}`; supported contract types are Unit, Bool, Int, Float and String", operation.service, operation.name));
                }
                resolved
            }
        };
        if report.diagnostics.items.len() == start_errors {
            report.operations.push(TypedServiceOperation {
                service: ServiceIdentity {
                    module,
                    name: operation.service.clone(),
                },
                name: operation.name.clone(),
                parameters,
                return_type,
                has_body: operation.has_body,
                source_file: operation.source_file.clone(),
                span: operation.span,
            });
        }
    }
    report
}
