//! Experimental structured body parsing and receiver inspection.
//!
//! Supports bindings, nested blocks, returns, literals, qualified/member calls,
//! and arithmetic/comparison expressions with partial primitive type checking.
//! Unsupported syntax fails the whole inspection; no partial call list escapes.

use super::scope::{Receiver, Scope, ServiceIdentity};
use crate::compiler::diagnostics::Span;
use crate::compiler::lexer::{Lexer, Token, TokenKind};
use crate::compiler::semantic::Type;

mod imports;
mod ordinary;

/// Explicit per-file outcome for experimental project receiver inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInspection {
    /// Source inspected.
    pub source_file: std::path::PathBuf,
    /// Function records, or the reason this file could not be fully inspected.
    pub functions: Result<Vec<FunctionCalls>, String>,
}

/// Service-call validation with explicit per-file syntax coverage.
#[derive(Debug, Clone)]
pub struct ServiceCheck {
    /// Files successfully inspected or rejected as unsupported/partial.
    pub files: Vec<FileInspection>,
    /// Contract errors in inspected files and E4096 for each unsupported file.
    pub diagnostics: crate::compiler::diagnostics::Diagnostics,
}

/// Check service signature types, resolved member names and argument counts.
///
/// Requires a successfully checked project (including service manifest bindings).
/// Unsupported files remain in `files` as errors and produce E4096 diagnostics.
/// Local and unresolved receivers are not service calls.
/// Primitive service and same-file ordinary contracts, arguments, return paths
/// and conditions are checked. File-local scalar aliases and direct exported
/// ordinary imports are resolved; record construction and unsupported expressions still
/// require complete application typing.
pub fn check_service_calls(project: &super::ProjectCheck) -> ServiceCheck {
    use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity};
    let (files, signatures) = imports::inspect(project);
    let mut diagnostics = Diagnostics::new();
    let mut operations = std::collections::BTreeMap::new();
    for operation in &project.service_operations {
        if let Ok(module) = operation.source_file.canonicalize() {
            operations.insert(
                (
                    module.to_string_lossy().into_owned(),
                    operation.service.clone(),
                    operation.name.clone(),
                ),
                operation.parameters.len(),
            );
        }
    }
    for file in &files {
        let Ok(functions) = &file.functions else {
            continue;
        };
        ordinary::check(functions, &file.source_file, &mut diagnostics);
        for function in functions {
            for span in &function.constructor_errors {
                diagnostics.push(Diagnostic {
                    source_file: Some(file.source_file.to_string_lossy().into_owned()),
                    severity: Severity::Error,
                    code: "E4139",
                    message: "invalid record construction: require a known record and each declared field exactly once with a compatible value".into(),
                    span: *span,
                });
            }
            for (span, message) in &function.field_errors {
                diagnostics.push(Diagnostic {
                    source_file: Some(file.source_file.to_string_lossy().into_owned()),
                    severity: Severity::Error,
                    code: "E4138",
                    message: message.clone(),
                    span: *span,
                });
            }
            for span in &function.invalid_integer_literals {
                diagnostics.push(Diagnostic {
                    source_file: Some(file.source_file.to_string_lossy().into_owned()),
                    severity: Severity::Error,
                    code: "E3012",
                    message: "integer literal is outside the signed 64-bit range".into(),
                    span: *span,
                });
            }
            for span in &function.unresolved_discarded_expressions {
                diagnostics.push(Diagnostic {
                    source_file: Some(file.source_file.to_string_lossy().into_owned()),
                    severity: Severity::Error,
                    code: "E4137",
                    message: "cannot validate discarded application expression: type is unresolved"
                        .into(),
                    span: *span,
                });
            }
            for binding in &function.local_bindings {
                let issue = if binding.declared_type == Some(Type::Unknown) {
                    Some((
                        "E4135",
                        "local annotation is unresolved",
                        binding.annotation_span.expect("annotated binding"),
                    ))
                } else if binding.initializer_type.is_none() {
                    Some((
                        "E4136",
                        "local initializer type is unresolved",
                        binding.initializer_span,
                    ))
                } else {
                    None
                };
                if let Some((code, message, span)) = issue {
                    diagnostics.push(Diagnostic {
                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                        severity: Severity::Error,
                        code,
                        message: format!("{message} for `{}`", binding.name),
                        span,
                    });
                }
            }
            if let Some(signature) = signatures.operations.iter().find(|signature| {
                signature.has_body
                    && signature.source_file == file.source_file
                    && function.name == format!("{}.{}", signature.service.name, signature.name)
            }) {
                if signature.return_type != Type::Unit && !function.always_returns {
                    diagnostics.push(Diagnostic {
                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                        severity: Severity::Error,
                        code: "E4123",
                        message: format!(
                            "service operation `{}` requires an explicit return for {:?}",
                            function.name, signature.return_type
                        ),
                        span: function.span,
                    });
                }
                for returned in &function.returns {
                    if returned.known_type.is_none() {
                        diagnostics.push(Diagnostic {
                            source_file: Some(file.source_file.to_string_lossy().into_owned()),
                            severity: Severity::Error,
                            code: "E4126",
                            message: format!("cannot validate return type for service operation `{}`: expression type is unresolved", function.name),
                            span: returned.span,
                        });
                    }
                    if let Some(actual) = &returned.known_type {
                        let expected = &signature.return_type;
                        if actual != expected && !(actual == &Type::Int && expected == &Type::Float)
                        {
                            diagnostics.push(Diagnostic {
                                source_file: Some(file.source_file.to_string_lossy().into_owned()),
                                severity: Severity::Error,
                                code: "E4122",
                                message: format!(
                                    "service operation `{}` returns {expected:?}, found {actual:?}",
                                    function.name
                                ),
                                span: returned.span,
                            });
                        }
                    }
                }
            }
            for (kind, span) in &function.conditions {
                if kind.is_none() {
                    diagnostics.push(Diagnostic {
                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                        severity: Severity::Error,
                        code: "E4125",
                        message:
                            "cannot validate application condition: expression type is unresolved"
                                .into(),
                        span: *span,
                    });
                }
                if kind.as_ref().is_some_and(|kind| *kind != Type::Bool) {
                    diagnostics.push(Diagnostic {
                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                        severity: Severity::Error,
                        code: "E4124",
                        message: "application condition requires Bool".into(),
                        span: *span,
                    });
                }
            }
            for (span, message) in &function.operator_errors {
                diagnostics.push(Diagnostic {
                    source_file: Some(file.source_file.to_string_lossy().into_owned()),
                    severity: Severity::Error,
                    code: "E4121",
                    message: message.clone(),
                    span: *span,
                });
            }
            for mismatch in &function.initializer_mismatches {
                diagnostics.push(Diagnostic {
                    source_file: Some(file.source_file.to_string_lossy().into_owned()),
                    severity: Severity::Error,
                    code: "E4120",
                    message: format!(
                        "local `{}` expects {:?}, found {:?}",
                        mismatch.name, mismatch.expected, mismatch.actual
                    ),
                    span: mismatch.span,
                });
            }
            for call in &function.calls {
                let issue = match &call.receiver {
                    Receiver::Service(service) => {
                        match operations.get(&(
                            service.module.clone(),
                            service.name.clone(),
                            call.operation.clone(),
                        )) {
                            None => Some((
                                "E4093",
                                format!(
                                    "service `{}` has no operation `{}`",
                                    service.name, call.operation
                                ),
                            )),
                            Some(expected) if *expected != call.arguments => Some((
                                "E4094",
                                format!(
                                    "service operation `{}.{}` expects {} argument(s), found {}",
                                    service.name, call.operation, expected, call.arguments
                                ),
                            )),
                            _ => None,
                        }
                    }
                    Receiver::Ambiguous(_) => Some((
                        "E4095",
                        format!(
                            "ambiguous service receiver for operation `{}`",
                            call.operation
                        ),
                    )),
                    Receiver::Local | Receiver::Unresolved => None,
                };
                if let Some((code, message)) = issue {
                    diagnostics.push(Diagnostic {
                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                        severity: Severity::Error,
                        code,
                        message,
                        span: call.span,
                    });
                }
                if let Receiver::Service(service) = &call.receiver {
                    if let Some(signature) = signatures.operations.iter().find(|signature| {
                        signature.service == *service && signature.name == call.operation
                    }) {
                        if signature.parameters.len() == call.arguments {
                            for (argument, parameter) in
                                call.argument_types.iter().zip(&signature.parameters)
                            {
                                let Some(actual) = argument.resolved_type() else {
                                    continue;
                                };
                                let expected = &parameter.parameter_type;
                                if actual != expected
                                    && !(actual == &Type::Int && expected == &Type::Float)
                                {
                                    diagnostics.push(Diagnostic {
                                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                                        severity: Severity::Error,
                                        code: "E4119",
                                        message: format!("service operation `{}.{}` parameter `{}` expects {expected:?}, found {actual:?}", service.name, call.operation, parameter.name),
                                        span: argument.span,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for file in &files {
        if let Err(reason) = &file.functions {
            diagnostics.push(Diagnostic {
                source_file: None,
                severity: Severity::Error,
                code: "E4096",
                message: format!(
                    "service-call inspection incomplete for `{}`: {reason}",
                    file.source_file.display()
                ),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
            });
        }
    }
    diagnostics.items.extend(signatures.diagnostics.items);
    ServiceCheck { files, diagnostics }
}

/// Inspect supported files using validated direct imports and same-file services.
/// Unsupported files return errors individually; this does not change check
/// success, enforce service contracts, or treat dependencies as transitive imports.
pub fn inspect_project(project: &super::ProjectCheck) -> Vec<FileInspection> {
    imports::inspect(project).0
}

fn inspect_project_local(
    project: &super::ProjectCheck,
    sources: &std::collections::BTreeMap<std::path::PathBuf, Result<String, String>>,
    signatures: &super::service_types::ServiceSignatures,
    modules: &std::collections::BTreeMap<std::path::PathBuf, super::application_types::Aliases>,
) -> Vec<FileInspection> {
    project
        .source_files
        .iter()
        .map(|file| {
            let functions = (|| {
                let own = file.canonicalize().map_err(|error| error.to_string())?;
                let mut visible = std::collections::BTreeSet::from([own]);
                for import in &project.imports {
                    if import.source_file == *file {
                        visible.insert(import.target_file.clone());
                    }
                }
                let mut services = Vec::new();
                for declaration in &project.service_declarations {
                    let module = declaration
                        .source_file
                        .canonicalize()
                        .map_err(|error| error.to_string())?;
                    if visible.contains(&module) {
                        services.push(ServiceIdentity {
                            module: module.to_string_lossy().into_owned(),
                            name: declaration.name.clone(),
                        });
                    }
                }
                let source = sources
                    .get(file)
                    .expect("discovered source snapshot")
                    .as_ref()
                    .map_err(Clone::clone)?;
                inspect_functions_in_module(
                    source,
                    &services,
                    &signatures.operations,
                    &Default::default(),
                    &file
                        .canonicalize()
                        .map_err(|error| error.to_string())?
                        .to_string_lossy(),
                    &imports::Records {
                        module: modules.get(file).cloned(),
                        ..Default::default()
                    },
                )
            })();
            FileInspection {
                source_file: file.clone(),
                functions,
            }
        })
        .collect()
}

/// A member call discovered through structured expression traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberCall {
    /// Member name invoked by the call.
    pub operation: String,
    /// Number of positional arguments.
    pub arguments: usize,
    /// Primitive type evidence in positional order; unsupported expressions remain unknown.
    pub argument_types: Vec<ArgumentType>,
    /// Classification of the receiver; complex expressions remain unresolved.
    pub receiver: Receiver,
    /// Range of the member expression in the supplied body source.
    pub span: Span,
}

/// Conservative type evidence for one inspected argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentType {
    /// Literal type, including parenthesized literals; None is not validation.
    pub literal_type: Option<Type>,
    /// Type inferred from bindings or binary expressions; absent when unresolved.
    pub binding_type: Option<Type>,
    /// Entire argument expression range in the original source.
    pub span: Span,
}

impl ArgumentType {
    /// Known primitive evidence; absence does not imply successful type validation.
    pub fn resolved_type(&self) -> Option<&Type> {
        self.literal_type.as_ref().or(self.binding_type.as_ref())
    }
}

#[derive(Debug)]
enum Expression {
    Record(Type, Vec<(String, Expression)>, Span),
    Name(String, Span),
    // Namespace resolution is outside service inspection; retain the full range.
    QualifiedName(String, Span),
    Literal(Type, Span),
    Member(Box<Expression>, String, Span),
    Call(Box<Expression>, Vec<Expression>, Span),
    Binary(Box<Expression>, &'static str, Box<Expression>, Span),
}

fn binary_type(operator: &str, left: &Type, right: &Type) -> Option<Type> {
    let numeric =
        matches!(left, Type::Int | Type::Float) && matches!(right, Type::Int | Type::Float);
    let strings = *left == Type::String && *right == Type::String;
    match operator {
        "&&" | "||" if *left == Type::Bool && *right == Type::Bool => Some(Type::Bool),
        "+" if strings => Some(Type::String),
        "+" | "-" | "*" | "/" if numeric => {
            Some(if *left == Type::Float || *right == Type::Float {
                Type::Float
            } else {
                Type::Int
            })
        }
        "==" | "!=" if numeric || strings || (*left == Type::Bool && *right == Type::Bool) => {
            Some(Type::Bool)
        }
        "<" | "<=" | ">" | ">=" if numeric || strings => Some(Type::Bool),
        _ => None,
    }
}

impl Expression {
    fn record_valid(kind: &Type, fields: &[(String, Expression)], scope: &Scope<'_>) -> bool {
        let Some(expected) = scope.record_fields(kind) else {
            return false;
        };
        let mut seen = std::collections::BTreeSet::new();
        fields.len() == expected.len()
            && fields.iter().all(|(name, value)| {
                seen.insert(name)
                    && expected.get(name).is_some_and(|expected| {
                        value.known_type(scope).is_some_and(|actual| {
                            actual == *expected || (actual == Type::Int && *expected == Type::Float)
                        })
                    })
            })
    }
    fn argument_type(&self, scope: &Scope<'_>) -> ArgumentType {
        let span = match self {
            Self::Record(_, _, span) => *span,
            Self::Name(_, span)
            | Self::QualifiedName(_, span)
            | Self::Literal(_, span)
            | Self::Member(_, _, span)
            | Self::Call(_, _, span)
            | Self::Binary(_, _, _, span) => *span,
        };
        let (literal_type, binding_type) = match self {
            Self::Literal(kind, _) => (Some(kind.clone()), None),
            _ => (None, self.known_type(scope)),
        };
        ArgumentType {
            literal_type,
            binding_type,
            span,
        }
    }
    fn known_type(&self, scope: &Scope<'_>) -> Option<Type> {
        match self {
            Self::Record(kind, fields, _) => {
                Self::record_valid(kind, fields, scope).then(|| kind.clone())
            }
            Self::Member(receiver, name, _) => scope.field_type(&receiver.known_type(scope)?, name),
            Self::Literal(kind, _) => Some(kind.clone()),
            Self::Name(name, span) => scope.binding_type(name, span.start),
            Self::Call(callee, arguments, _) => {
                if let Self::Name(name, span) | Self::QualifiedName(name, span) = callee.as_ref() {
                    let signature = scope.function(name, span.start)?;
                    if arguments.len() != signature.parameters.len()
                        || signature.return_type == Type::Unknown
                    {
                        return None;
                    }
                    for (index, argument) in arguments.iter().enumerate() {
                        let actual = argument.known_type(scope)?;
                        if !signature.accepts(index, &actual) {
                            return None;
                        }
                    }
                    return Some(signature.return_type.clone());
                }
                let Self::Member(receiver, name, _) = callee.as_ref() else {
                    return None;
                };
                let Self::Name(receiver, span) = receiver.as_ref() else {
                    return None;
                };
                let Receiver::Service(service) = scope.resolve(receiver, span.start) else {
                    return None;
                };
                let operation = scope.operation(&service, name)?;
                if arguments.len() != operation.parameters.len() {
                    return None;
                }
                for (argument, parameter) in arguments.iter().zip(&operation.parameters) {
                    let actual = argument.known_type(scope)?;
                    if actual != parameter.parameter_type
                        && !(actual == Type::Int && parameter.parameter_type == Type::Float)
                    {
                        return None;
                    }
                }
                Some(operation.return_type.clone())
            }
            Self::Binary(left, operator, right, _) => binary_type(
                operator,
                &left.known_type(scope)?,
                &right.known_type(scope)?,
            ),
            _ => None,
        }
    }
    fn depth(&self) -> usize {
        let mut pending = vec![(self, 1)];
        let mut maximum = 0;
        while let Some((expression, depth)) = pending.pop() {
            maximum = maximum.max(depth);
            match expression {
                Self::Record(_, fields, _) => {
                    pending.extend(fields.iter().map(|(_, value)| (value, depth + 1)))
                }
                Self::Binary(left, _, right, _) => {
                    pending.push((left, depth + 1));
                    pending.push((right, depth + 1));
                }
                Self::Member(receiver, _, _) => pending.push((receiver, depth + 1)),
                Self::Call(callee, arguments, _) => {
                    pending.push((callee, depth + 1));
                    pending.extend(arguments.iter().map(|argument| (argument, depth + 1)));
                }
                _ => {}
            }
        }
        maximum
    }
    fn span(&mut self) -> &mut Span {
        match self {
            Self::Record(_, _, span) => span,
            Self::Name(_, span)
            | Self::QualifiedName(_, span)
            | Self::Literal(_, span)
            | Self::Member(_, _, span)
            | Self::Call(_, _, span)
            | Self::Binary(_, _, _, span) => span,
        }
    }
}

/// A function or task inspected from a complete source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCalls {
    /// Explicit task result annotation; absent annotations do not infer a task result.
    pub task_return_annotation: Option<Type>,
    /// Names of task parameters whose annotations could not be resolved.
    pub unresolved_task_parameters: Vec<String>,
    /// Invalid record constructor ranges; no valid type is propagated for them.
    pub constructor_errors: Vec<Span>,
    /// Invalid field accesses on known receivers, with original expression ranges.
    pub field_errors: Vec<(Span, String)>,
    /// Integer literal token ranges outside the executable signed 64-bit domain.
    pub invalid_integer_literals: Vec<Span>,
    /// Discarded non-call expressions without a resolved type, in source order.
    pub unresolved_discarded_expressions: Vec<Span>,
    /// Local binding type evidence in source order, including unresolved bindings.
    pub local_bindings: Vec<LocalBinding>,
    /// Whether this top-level ordinary function explicitly exports its interface.
    pub is_exported: bool,
    /// Same-file ordinary signature; absent for tasks and service operations.
    pub signature: Option<FunctionSignature>,
    /// Resolved ordinary calls; unresolved names and shadowed bindings are excluded.
    pub function_calls: Vec<FunctionCall>,
    /// Bare, qualified or computed calls whose callable contract is unresolved.
    pub unresolved_calls: Vec<UnresolvedCall>,
    /// Declaration name.
    pub name: String,
    /// Whether this is a task rather than an ordinary function.
    pub is_task: bool,
    /// Full declaration range in the original source file.
    pub span: Span,
    /// Structured member calls in the supported body subset.
    pub calls: Vec<MemberCall>,
    /// Known primitive initializer mismatches; not a complete body type check.
    pub initializer_mismatches: Vec<InitializerMismatch>,
    /// Known incompatible binary operands, with expression ranges and messages.
    pub operator_errors: Vec<(Span, String)>,
    /// Explicit returns with conservative primitive type evidence.
    pub returns: Vec<ReturnType>,
    /// Whether the supported control flow guarantees an explicit return.
    pub always_returns: bool,
    /// Condition type evidence and original expression ranges.
    pub conditions: Vec<(Option<Type>, Span)>,
}

/// Type evidence for one local declaration; None does not establish compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBinding {
    /// Declared local name.
    pub name: String,
    /// Annotation type; None means inferred, Unknown means unsupported annotation.
    pub declared_type: Option<Type>,
    /// Known initializer type before widening.
    pub initializer_type: Option<Type>,
    /// Validated binding type after widening, if known.
    pub resolved_type: Option<Type>,
    /// Full declaration range excluding optional semicolon.
    pub span: Span,
    /// Annotation range, if explicitly written.
    pub annotation_span: Option<Span>,
    /// Initializer expression range.
    pub initializer_span: Span,
}

/// Ordinary or builtin function interface retained by structured inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    /// Public nominal field interfaces carried with record arguments/results.
    /// This metadata never imports a source type name into the consumer.
    pub record_fields:
        std::sync::Arc<std::collections::HashMap<String, std::collections::HashMap<String, Type>>>,
    /// Canonical declaring file for an imported function; None for local/builtin interfaces.
    pub source_file: Option<std::path::PathBuf>,
    /// Positional types; unsupported annotations retain Unknown.
    pub parameters: Vec<Type>,
    /// Rust-owned builtin Any slots; user annotations are never wildcards.
    pub any_parameters: Vec<bool>,
    /// Declared return type, defaulting to Unit.
    pub return_type: Type,
}

impl FunctionSignature {
    pub(super) fn exposes_private_records(&self) -> bool {
        self.parameters.iter().chain(std::iter::once(&self.return_type)).any(|kind| {
            matches!(kind, Type::Named(identity) if !self.record_fields.contains_key(identity))
        })
    }
    fn accepts(&self, index: usize, actual: &Type) -> bool {
        self.any_parameters.get(index).copied().unwrap_or(false)
            || self.parameters.get(index).is_some_and(|expected| {
                expected == actual || (*expected == Type::Float && *actual == Type::Int)
            })
    }
}

/// Resolved same-file ordinary or builtin call and type evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    /// Declared function name.
    pub name: String,
    /// Resolved declaration interface.
    pub signature: FunctionSignature,
    /// Positional argument evidence.
    pub arguments: Vec<ArgumentType>,
    /// Whole call expression range.
    pub span: Span,
}

/// An ordinary call retained even when no callable signature can be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedCall {
    /// Callee spelling, or None for a computed callee expression.
    pub name: Option<String>,
    /// Explanation of the unresolved boundary, not a callable type assertion.
    pub reason: String,
    /// Positional argument evidence.
    pub arguments: Vec<ArgumentType>,
    /// Whole call expression range.
    pub span: Span,
}

/// An inspected explicit return; this does not prove return-path completeness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnType {
    /// Known expression type, or Unit for a bare return.
    pub known_type: Option<Type>,
    /// Expression range, or the return keyword for a bare return.
    pub span: Span,
}

/// An annotated local whose known initializer cannot convert to its primitive type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializerMismatch {
    /// Local binding name.
    pub name: String,
    /// Declared primitive type.
    pub expected: Type,
    /// Known initializer type.
    pub actual: Type,
    /// Initializer expression range.
    pub span: Span,
}

/// Inspect a file containing top-level function/task declarations.
///
/// Same-file primitive ordinary signatures are collected before body inspection.
/// Named annotations remain unresolved. Visible services must be supplied
/// by the caller; this API does not infer imports. Unsupported top-level or body
/// syntax rejects the whole inspection. All spans are relative to `source`.
pub fn inspect_functions(
    source: &str,
    services: &[ServiceIdentity],
) -> Result<Vec<FunctionCalls>, String> {
    inspect_functions_with_signatures(source, services, &[])
}

fn inspect_functions_with_signatures(
    source: &str,
    services: &[ServiceIdentity],
    signatures: &[super::service_types::TypedServiceOperation],
) -> Result<Vec<FunctionCalls>, String> {
    inspect_functions_with_imports(source, services, signatures, &Default::default())
}

fn inspect_functions_with_imports(
    source: &str,
    services: &[ServiceIdentity],
    signatures: &[super::service_types::TypedServiceOperation],
    imported: &std::collections::BTreeMap<String, Option<FunctionSignature>>,
) -> Result<Vec<FunctionCalls>, String> {
    inspect_functions_in_module(
        source,
        services,
        signatures,
        imported,
        "<source>",
        &Default::default(),
    )
}

fn inspect_functions_in_module(
    source: &str,
    services: &[ServiceIdentity],
    signatures: &[super::service_types::TypedServiceOperation],
    imported: &std::collections::BTreeMap<String, Option<FunctionSignature>>,
    owner: &str,
    records: &imports::Records,
) -> Result<Vec<FunctionCalls>, String> {
    let preliminary = parse_functions(
        source,
        services,
        signatures,
        &Default::default(),
        owner,
        records,
    )?;
    let mut ordinary = std::collections::BTreeMap::new();
    for builtin in crate::compiler::stdlib::functions() {
        let signature = FunctionSignature {
            record_fields: Default::default(),
            source_file: None,
            parameters: builtin
                .parameters
                .iter()
                .map(|name| Type::from_name_with_known(name, None))
                .collect(),
            any_parameters: builtin
                .parameters
                .iter()
                .map(|name| crate::compiler::stdlib::is_any_type(name))
                .collect(),
            return_type: Type::from_name_with_known(builtin.return_type, None),
        };
        ordinary.insert(builtin.name.to_owned(), Some(signature.clone()));
        if builtin.name == "std::print" {
            ordinary.insert("print".into(), Some(signature));
        }
    }
    for function in &preliminary {
        if let Some(signature) = &function.signature {
            ordinary
                .entry(function.name.clone())
                .and_modify(|entry| *entry = None)
                .or_insert(Some(signature.clone()));
        }
    }
    for (name, signature) in imported {
        ordinary
            .entry(name.clone())
            .and_modify(|entry| *entry = None)
            .or_insert_with(|| signature.clone());
    }
    parse_functions(source, services, signatures, &ordinary, owner, records)
}

fn parse_functions(
    source: &str,
    services: &[ServiceIdentity],
    signatures: &[super::service_types::TypedServiceOperation],
    ordinary: &std::collections::BTreeMap<String, Option<FunctionSignature>>,
    owner: &str,
    records: &imports::Records,
) -> Result<Vec<FunctionCalls>, String> {
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "file contains unsupported lexical syntax".to_owned())?;
    check_nesting(&tokens)?;
    let mut aliases = match &records.module {
        Some(module) => module.clone(),
        None => super::application_types::Aliases::collect_in_module(&tokens, owner)?,
    };
    aliases.types.extend(records.types.clone());
    aliases.fields.extend(records.fields.clone());
    let mut parser = BodyParser {
        aliases: &aliases.types,
        public_fields: std::sync::Arc::new(aliases.visible_fields.clone()),
        allow_record: true,
        constructor_errors: Vec::new(),
        fields: &aliases.fields,
        field_errors: Vec::new(),
        invalid_integer_literals: Vec::new(),
        tokens: &tokens,
        position: 0,
        calls: Vec::new(),
        initializer_mismatches: Vec::new(),
        unresolved_discarded_expressions: Vec::new(),
        local_bindings: Vec::new(),
        operator_errors: Vec::new(),
        returns: Vec::new(),
        conditions: Vec::new(),
        signatures,
        ordinary,
        function_calls: Vec::new(),
        unresolved_calls: Vec::new(),
    };
    let mut functions = Vec::new();
    while parser.peek() != &TokenKind::Eof {
        if let Some(end) = aliases.ends.get(&parser.position) {
            parser.position = *end;
            continue;
        }
        let start = tokens[parser.position].span;
        if parser.consume(TokenKind::Keyword("use")) {
            while parser.peek() != &TokenKind::Eof
                && tokens[parser.position].span.line == start.line
            {
                if parser.consume(TokenKind::Punctuation(';')) {
                    break;
                }
                parser.position += 1;
            }
            let end = tokens[parser.position - 1].span.end;
            super::imports::parse(&source[start.start..end]).map_err(str::to_owned)?;
            continue;
        }
        if matches!(parser.peek(), TokenKind::Identifier(name) if name == "service") {
            parser.position += 1;
            let owner = match parser.peek() {
                TokenKind::Identifier(name) => name.clone(),
                _ => return Err("expected service name".into()),
            };
            parser.position += 1;
            parser.require(TokenKind::Punctuation('{'))?;
            while !parser.consume(TokenKind::Punctuation('}')) {
                if parser.peek() != &TokenKind::Keyword("fn") {
                    return Err("expected service operation or closing brace".into());
                }
                if let Some(function) = parser.declaration(source, services, Some(&owner), false)? {
                    functions.push(function);
                }
            }
            continue;
        }
        let is_exported = parser.consume(TokenKind::Keyword("export"));
        if is_exported && parser.peek() != &TokenKind::Keyword("fn") {
            return Err("application exports require a top-level function".into());
        }
        let is_task = match parser.peek() {
            TokenKind::Keyword("fn") => false,
            TokenKind::Keyword("task") => true,
            TokenKind::Identifier(name) if name == "task" => true,
            _ => {
                return Err(format!(
                    "unsupported application declaration at byte {}",
                    start.start
                ))
            }
        };
        if let Some(mut function) = parser.declaration(source, services, None, is_task)? {
            function.is_exported = is_exported;
            if is_exported {
                function.span = Span {
                    end: function.span.end,
                    ..start
                };
            }
            functions.push(function);
        }
    }
    Ok(functions)
}

/// Inspect one braced body using explicitly supplied parameters and services.
/// This does not validate service operations, argument types or imports. Offsets
/// are relative to `source`. No source substring matching is used for calls.
pub fn inspect_body(
    source: &str,
    parameters: &[&str],
    services: &[ServiceIdentity],
) -> Result<Vec<MemberCall>, String> {
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "body contains unsupported lexical syntax".to_owned())?;
    check_nesting(&tokens)?;
    let mut parser = BodyParser {
        aliases: &Default::default(),
        public_fields: Default::default(),
        allow_record: true,
        constructor_errors: Vec::new(),
        fields: &Default::default(),
        field_errors: Vec::new(),
        invalid_integer_literals: Vec::new(),
        tokens: &tokens,
        position: 0,
        calls: Vec::new(),
        initializer_mismatches: Vec::new(),
        unresolved_discarded_expressions: Vec::new(),
        local_bindings: Vec::new(),
        operator_errors: Vec::new(),
        returns: Vec::new(),
        conditions: Vec::new(),
        signatures: &[],
        ordinary: &Default::default(),
        function_calls: Vec::new(),
        unresolved_calls: Vec::new(),
    };
    let mut root = Scope::new();
    for service in services {
        root.add_service(service.clone());
    }
    for parameter in parameters {
        root.bind(*parameter, 0);
    }
    parser.block(&root)?;
    if parser.peek() != &TokenKind::Eof {
        return Err("unexpected text after application body".into());
    }
    Ok(parser.calls)
}

struct BodyParser<'a> {
    public_fields:
        std::sync::Arc<std::collections::HashMap<String, std::collections::HashMap<String, Type>>>,
    allow_record: bool,
    constructor_errors: Vec<Span>,
    fields: &'a std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
    field_errors: Vec<(Span, String)>,
    invalid_integer_literals: Vec<Span>,
    aliases: &'a std::collections::HashMap<String, Type>,
    unresolved_discarded_expressions: Vec<Span>,
    local_bindings: Vec<LocalBinding>,
    unresolved_calls: Vec<UnresolvedCall>,
    ordinary: &'a std::collections::BTreeMap<String, Option<FunctionSignature>>,
    function_calls: Vec<FunctionCall>,
    conditions: Vec<(Option<Type>, Span)>,
    returns: Vec<ReturnType>,
    signatures: &'a [super::service_types::TypedServiceOperation],
    tokens: &'a [Token],
    position: usize,
    calls: Vec<MemberCall>,
    initializer_mismatches: Vec<InitializerMismatch>,
    operator_errors: Vec<(Span, String)>,
}

fn check_nesting(tokens: &[Token]) -> Result<(), String> {
    crate::compiler::limits::check_nesting(tokens, true).map_err(depth_error)
}

fn depth_error(span: Span) -> String {
    format!(
        "{} at byte {}",
        crate::compiler::limits::depth_message(),
        span.start
    )
}

fn check_depth(depth: usize, span: Span) -> Result<(), String> {
    if depth > crate::compiler::limits::MAX_STRUCTURAL_DEPTH {
        Err(depth_error(span))
    } else {
        Ok(())
    }
}

impl BodyParser<'_> {
    fn declaration(
        &mut self,
        source: &str,
        services: &[ServiceIdentity],
        owner: Option<&str>,
        is_task: bool,
    ) -> Result<Option<FunctionCalls>, String> {
        let start = self.tokens[self.position].span;
        let keyword_end = start.end;
        self.position += 1;
        while !matches!(
            self.peek(),
            TokenKind::Punctuation('{' | '}' | ';') | TokenKind::Keyword("fn") | TokenKind::Eof
        ) {
            self.position += 1;
        }
        let header = source[keyword_end..self.tokens[self.position].span.start]
            .lines()
            .map(|line| super::strip_line_comment(line, "//"))
            .collect::<Vec<_>>()
            .join("\n");
        let header = format!("fn {header}");
        let operation = super::service_contract::parse_operation(&header)
            .map_err(str::to_owned)?
            .ok_or("missing declaration signature")?;
        if let Some(parameter) = operation
            .declarations
            .iter()
            .find(|item| item.annotation.is_none())
        {
            return Err(format!(
                "parameter `{}` requires an explicit type annotation at byte {}",
                parameter.name, start.start
            ));
        }
        if self.peek() != &TokenKind::Punctuation('{') {
            if owner.is_none() {
                return Err("missing function/task body".into());
            }
            self.consume(TokenKind::Punctuation(';'));
            return Ok(None);
        }
        let mut scope = Scope::new();
        scope.add_functions(self.ordinary);
        scope.add_fields(self.fields);
        scope.add_operations(self.signatures);
        for service in services {
            scope.add_service(service.clone());
        }
        for parameter in &operation.declarations {
            let kind = parameter
                .annotation
                .map(|annotation| Type::from_name_with_known(annotation, Some(self.aliases)))
                .filter(|kind| *kind != Type::Unknown);
            scope.bind_typed(parameter.name, start.start, kind);
        }
        let always_returns = self.block(&scope)?;
        Ok(Some(FunctionCalls {
            task_return_annotation: if is_task {
                operation
                    .return_annotation
                    .map(|annotation| Type::from_name_with_known(annotation, Some(self.aliases)))
            } else {
                None
            },
            unresolved_task_parameters: operation
                .declarations
                .iter()
                .filter(|parameter| {
                    is_task
                        && Type::from_name_with_known(
                            parameter.annotation.unwrap_or(""),
                            Some(self.aliases),
                        ) == Type::Unknown
                })
                .map(|parameter| parameter.name.to_owned())
                .collect(),
            constructor_errors: std::mem::take(&mut self.constructor_errors),
            field_errors: std::mem::take(&mut self.field_errors),
            invalid_integer_literals: std::mem::take(&mut self.invalid_integer_literals),
            unresolved_discarded_expressions: std::mem::take(
                &mut self.unresolved_discarded_expressions,
            ),
            local_bindings: std::mem::take(&mut self.local_bindings),
            is_exported: false,
            signature: if owner.is_none() && !is_task {
                Some(FunctionSignature {
                    record_fields: self.public_fields.clone(),
                    source_file: None,
                    any_parameters: vec![false; operation.declarations.len()],
                    parameters: operation
                        .declarations
                        .iter()
                        .map(|parameter| {
                            Type::from_name_with_known(
                                parameter.annotation.unwrap_or(""),
                                Some(self.aliases),
                            )
                        })
                        .collect(),
                    return_type: operation
                        .return_annotation
                        .map_or(Type::Unit, |annotation| {
                            Type::from_name_with_known(annotation, Some(self.aliases))
                        }),
                })
            } else {
                None
            },
            function_calls: std::mem::take(&mut self.function_calls),
            unresolved_calls: std::mem::take(&mut self.unresolved_calls),
            name: owner.map_or_else(
                || operation.name.to_owned(),
                |owner| format!("{owner}.{}", operation.name),
            ),
            is_task,
            span: Span {
                end: self.tokens[self.position - 1].span.end,
                ..start
            },
            calls: std::mem::take(&mut self.calls),
            initializer_mismatches: std::mem::take(&mut self.initializer_mismatches),
            operator_errors: std::mem::take(&mut self.operator_errors),
            returns: std::mem::take(&mut self.returns),
            always_returns,
            conditions: std::mem::take(&mut self.conditions),
        }))
    }
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.position].kind
    }
    fn consume(&mut self, kind: TokenKind) -> bool {
        if self.peek() == &kind {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn require(&mut self, kind: TokenKind) -> Result<(), String> {
        if self.consume(kind) {
            Ok(())
        } else {
            Err(format!(
                "unsupported or incomplete application syntax at byte {}",
                self.tokens[self.position].span.start
            ))
        }
    }
    fn condition(&mut self, scope: &Scope<'_>) -> Result<(), String> {
        let previous = std::mem::replace(&mut self.allow_record, false);
        let expression = self.expression();
        self.allow_record = previous;
        let mut expression = expression?;
        self.inspect(&expression, scope);
        self.conditions
            .push((expression.known_type(scope), *expression.span()));
        Ok(())
    }

    fn conditional(&mut self, scope: &Scope<'_>, depth: usize) -> Result<bool, String> {
        check_depth(depth, self.tokens[self.position].span)?;
        self.condition(scope)?;
        let then_returns = self.block(scope)?;
        let else_returns = if self.consume(TokenKind::Keyword("else")) {
            if self.consume(TokenKind::Keyword("if")) {
                self.conditional(scope, depth + 1)?
            } else {
                self.block(scope)?
            }
        } else {
            false
        };
        Ok(then_returns && else_returns)
    }

    fn block(&mut self, parent: &Scope<'_>) -> Result<bool, String> {
        self.require(TokenKind::Punctuation('{'))?;
        let mut scope = parent.child();
        let mut always_returns = false;
        while !self.consume(TokenKind::Punctuation('}')) {
            if self.peek() == &TokenKind::Eof {
                return Err("unclosed application block".into());
            }
            if self.peek() == &TokenKind::Punctuation('{') {
                always_returns |= self.block(&scope)?;
                continue;
            }
            if self.consume(TokenKind::Keyword("if")) {
                always_returns |= self.conditional(&scope, 1)?;
                continue;
            }
            if self.consume(TokenKind::Keyword("while")) {
                self.condition(&scope)?;
                self.block(&scope)?;
                continue;
            }
            if self.consume(TokenKind::Keyword("let")) {
                let declaration_start = self.tokens[self.position - 1].span;
                let TokenKind::Identifier(name) = self.peek().clone() else {
                    return Err("expected local binding name".into());
                };
                self.position += 1;
                let annotated = self.consume(TokenKind::Punctuation(':'));
                let annotation_start = self.position;
                if annotated {
                    self.local_annotation()?;
                }
                let declared = if annotated {
                    let tokens = &self.tokens[annotation_start..self.position];
                    let mut name = String::new();
                    let valid = tokens
                        .iter()
                        .enumerate()
                        .all(|(index, token)| match &token.kind {
                            TokenKind::Identifier(part) if index % 2 == 0 => {
                                name.push_str(part);
                                true
                            }
                            TokenKind::Operator("::") if index % 2 == 1 => {
                                name.push_str("::");
                                true
                            }
                            _ => false,
                        });
                    (valid && tokens.len() % 2 == 1)
                        .then(|| Type::from_name_with_known(&name, Some(self.aliases)))
                        .filter(|kind| *kind != Type::Unknown)
                } else {
                    None
                };
                let annotation_span = annotated.then(|| Span {
                    end: self.tokens[self.position - 1].span.end,
                    ..self.tokens[annotation_start].span
                });
                self.require(TokenKind::Operator("="))?;
                let mut expression = self.expression()?;
                self.inspect(&expression, &scope);
                let actual = expression.known_type(&scope);
                let recorded_declared = if annotated {
                    Some(declared.clone().unwrap_or(Type::Unknown))
                } else {
                    None
                };
                let recorded_actual = actual.clone();
                let kind = if annotated {
                    match (declared, actual) {
                        (Some(expected), Some(actual))
                            if expected == actual
                                || (expected == Type::Float && actual == Type::Int) =>
                        {
                            Some(expected)
                        }
                        (Some(expected), Some(actual)) => {
                            self.initializer_mismatches.push(InitializerMismatch {
                                name: name.clone(),
                                expected,
                                actual,
                                span: *expression.span(),
                            });
                            None
                        }
                        _ => None,
                    }
                } else {
                    actual
                };
                self.local_bindings.push(LocalBinding {
                    name: name.clone(),
                    declared_type: recorded_declared,
                    initializer_type: recorded_actual,
                    resolved_type: kind.clone(),
                    span: Span {
                        end: expression.span().end,
                        ..declaration_start
                    },
                    annotation_span,
                    initializer_span: *expression.span(),
                });
                scope.bind_typed(name, self.tokens[self.position - 1].span.end, kind);
            } else {
                let return_span = self.tokens[self.position].span;
                let returning = self.consume(TokenKind::Keyword("return"));
                always_returns |= returning;
                if !returning || !matches!(self.peek(), TokenKind::Punctuation(';' | '}')) {
                    let mut expression = self.expression()?;
                    self.inspect(&expression, &scope);
                    if returning {
                        self.returns.push(ReturnType {
                            known_type: expression.known_type(&scope),
                            span: *expression.span(),
                        });
                    } else if !matches!(expression, Expression::Call(_, _, _))
                        && expression.known_type(&scope).is_none()
                    {
                        // Calls already validate their callable contracts and arguments.
                        // Other discarded values must not bypass type validation.
                        self.unresolved_discarded_expressions
                            .push(*expression.span());
                    }
                } else {
                    self.returns.push(ReturnType {
                        known_type: Some(Type::Unit),
                        span: return_span,
                    });
                }
            }
            self.consume(TokenKind::Punctuation(';'));
        }
        Ok(always_returns)
    }
    // Consume structural annotation tokens without resolving their type names.
    // Stop only at a top-level initializer marker, never at arbitrary body text.
    fn local_annotation(&mut self) -> Result<(), String> {
        let mut delimiters = Vec::new();
        let mut has_name = false;
        loop {
            match self.peek() {
                TokenKind::Operator("=") if delimiters.is_empty() && has_name => return Ok(()),
                TokenKind::Identifier(_) => has_name = true,
                TokenKind::Operator("<") => delimiters.push('<'),
                TokenKind::Punctuation('(') => delimiters.push('('),
                TokenKind::Punctuation('[') => delimiters.push('['),
                TokenKind::Operator(">") | TokenKind::Punctuation(')' | ']') => {
                    let expected = match self.peek() {
                        TokenKind::Operator(">") => '<',
                        TokenKind::Punctuation(')') => '(',
                        _ => '[',
                    };
                    if delimiters.pop() != Some(expected) {
                        return Err("mismatched local annotation delimiter".into());
                    }
                }
                TokenKind::Punctuation(',') if !delimiters.is_empty() => {}
                TokenKind::Operator("->" | "::") => {}
                _ => return Err("unsupported or incomplete local annotation".into()),
            }
            self.position += 1;
        }
    }

    fn expression(&mut self) -> Result<Expression, String> {
        self.binary_expression(1)
    }

    fn binary_expression(&mut self, minimum: u8) -> Result<Expression, String> {
        let mut left = self.postfix_expression()?;
        loop {
            let precedence = match self.tokens[self.position].kind {
                TokenKind::Operator("||") => 1,
                TokenKind::Operator("&&") => 2,
                TokenKind::Operator("==" | "!=" | "<" | "<=" | ">" | ">=") => 3,
                TokenKind::Operator("+" | "-") => 4,
                TokenKind::Operator("*" | "/") => 5,
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            let operator_span = self.tokens[self.position].span;
            let TokenKind::Operator(operator) = self.tokens[self.position].kind else {
                unreachable!()
            };
            self.position += 1;
            let mut right = self.binary_expression(precedence + 1)?;
            check_depth(1 + left.depth().max(right.depth()), operator_span)?;
            let span = Span {
                end: right.span().end,
                ..*left.span()
            };
            left = Expression::Binary(Box::new(left), operator, Box::new(right), span);
        }
        Ok(left)
    }

    fn postfix_expression(&mut self) -> Result<Expression, String> {
        let token = &self.tokens[self.position];
        let mut expression = match &token.kind {
            TokenKind::Identifier(name) => {
                let value = Expression::Name(name.clone(), token.span);
                self.position += 1;
                value
            }
            TokenKind::String(_)
            | TokenKind::Integer(_)
            | TokenKind::Float(_)
            | TokenKind::Keyword("true" | "false") => {
                if let TokenKind::Integer(value) = &token.kind {
                    if value.parse::<i64>().is_err() {
                        self.invalid_integer_literals.push(token.span);
                    }
                }
                self.position += 1;
                let literal_type = match token.kind {
                    TokenKind::String(_) => Type::String,
                    TokenKind::Integer(_) => Type::Int,
                    TokenKind::Float(_) => Type::Float,
                    _ => Type::Bool,
                };
                Expression::Literal(literal_type, token.span)
            }
            TokenKind::Punctuation('(') => {
                self.position += 1;
                let previous = std::mem::replace(&mut self.allow_record, true);
                let value = self.expression();
                self.allow_record = previous;
                let mut value = value?;
                self.require(TokenKind::Punctuation(')'))?;
                *value.span() = Span {
                    end: self.tokens[self.position - 1].span.end,
                    ..token.span
                };
                value
            }
            _ => {
                return Err(format!(
                    "unsupported application expression at byte {}",
                    token.span.start
                ))
            }
        };
        while self.consume(TokenKind::Operator("::")) {
            let (module, start) = match expression {
                Expression::Name(module, start) | Expression::QualifiedName(module, start) => {
                    (module, start)
                }
                _ => return Err("expected a module name before `::`".into()),
            };
            if !matches!(self.peek(), TokenKind::Identifier(_)) {
                return Err("expected a module member name after `::`".into());
            }
            let TokenKind::Identifier(member) = self.peek() else {
                unreachable!()
            };
            expression = Expression::QualifiedName(
                format!("{module}::{member}"),
                Span {
                    end: self.tokens[self.position].span.end,
                    ..start
                },
            );
            self.position += 1;
        }
        if self.allow_record && self.peek() == &TokenKind::Punctuation('{') {
            if let Expression::Name(name, start) | Expression::QualifiedName(name, start) =
                &expression
            {
                let kind = Type::from_name_with_known(name, Some(self.aliases));
                let start = *start;
                self.position += 1;
                let mut fields = Vec::new();
                while self.peek() != &TokenKind::Punctuation('}') {
                    let TokenKind::Identifier(field) = self.peek().clone() else {
                        return Err("expected record field name".into());
                    };
                    self.position += 1;
                    self.require(TokenKind::Punctuation(':'))?;
                    fields.push((field, self.expression()?));
                    if !self.consume(TokenKind::Punctuation(',')) {
                        break;
                    }
                }
                self.require(TokenKind::Punctuation('}'))?;
                check_depth(
                    1 + fields
                        .iter()
                        .map(|(_, value)| value.depth())
                        .max()
                        .unwrap_or(0),
                    start,
                )?;
                expression = Expression::Record(
                    kind,
                    fields,
                    Span {
                        end: self.tokens[self.position - 1].span.end,
                        ..start
                    },
                );
            }
        }
        loop {
            if self.consume(TokenKind::Punctuation('.')) {
                let token = &self.tokens[self.position];
                let TokenKind::Identifier(name) = &token.kind else {
                    return Err("expected member name".into());
                };
                let start = *expression.span();
                check_depth(1 + expression.depth(), token.span)?;
                expression = Expression::Member(
                    Box::new(expression),
                    name.clone(),
                    Span {
                        end: token.span.end,
                        ..start
                    },
                );
                self.position += 1;
            } else if self.consume(TokenKind::Punctuation('(')) {
                let mut arguments = Vec::new();
                if !self.consume(TokenKind::Punctuation(')')) {
                    loop {
                        let previous = std::mem::replace(&mut self.allow_record, true);
                        let argument = self.expression();
                        self.allow_record = previous;
                        arguments.push(argument?);
                        if self.consume(TokenKind::Punctuation(')')) {
                            break;
                        }
                        self.require(TokenKind::Punctuation(','))?;
                        if self.consume(TokenKind::Punctuation(')')) {
                            break;
                        }
                    }
                }
                let span = Span {
                    end: self.tokens[self.position - 1].span.end,
                    ..*expression.span()
                };
                let depth = 1 + arguments
                    .iter()
                    .map(Expression::depth)
                    .chain(std::iter::once(expression.depth()))
                    .max()
                    .unwrap_or(0);
                check_depth(depth, span)?;
                expression = Expression::Call(Box::new(expression), arguments, span);
            } else {
                break;
            }
        }
        Ok(expression)
    }
    fn inspect(&mut self, expression: &Expression, scope: &Scope<'_>) {
        match expression {
            Expression::Record(kind, fields, span) => {
                for (_, value) in fields {
                    self.inspect(value, scope);
                }
                if !Expression::record_valid(kind, fields, scope) {
                    self.constructor_errors.push(*span);
                }
            }
            Expression::Call(callee, arguments, call_span) => {
                if let Expression::Member(receiver, name, _) = callee.as_ref() {
                    self.inspect(receiver, scope);
                    let reason = match receiver.known_type(scope) {
                        Some(Type::Named(_)) => {
                            Some("record fields are values, not callable operations".into())
                        }
                        Some(kind) => {
                            Some(format!("receiver type {kind:?} has no callable members"))
                        }
                        None => {
                            let resolved = match receiver.as_ref() {
                                Expression::Name(name, span) => scope.resolve(name, span.start),
                                _ => Receiver::Unresolved,
                            };
                            match resolved {
                                Receiver::Service(_) | Receiver::Ambiguous(_) => None,
                                Receiver::Local | Receiver::Unresolved => {
                                    Some("member receiver has no resolved callable contract".into())
                                }
                            }
                        }
                    };
                    if let Some(reason) = reason {
                        self.unresolved_calls.push(UnresolvedCall {
                            name: Some(name.clone()),
                            reason,
                            arguments: arguments
                                .iter()
                                .map(|argument| argument.argument_type(scope))
                                .collect(),
                            span: *call_span,
                        });
                    }
                } else {
                    self.inspect(callee, scope);
                }
                for argument in arguments {
                    self.inspect(argument, scope);
                }
                if let Expression::Name(name, span) | Expression::QualifiedName(name, span) =
                    callee.as_ref()
                {
                    if let Some(signature) = scope.function(name, span.start) {
                        self.function_calls.push(FunctionCall {
                            name: name.clone(),
                            signature: signature.clone(),
                            arguments: arguments
                                .iter()
                                .map(|argument| argument.argument_type(scope))
                                .collect(),
                            span: *call_span,
                        });
                    } else {
                        let reason = if matches!(scope.resolve(name, span.start), Receiver::Local) {
                            "callee is a local binding; callable values are not resolved"
                        } else {
                            "no unique supported callable signature is available"
                        };
                        self.unresolved_calls.push(UnresolvedCall {
                            name: Some(name.clone()),
                            reason: reason.into(),
                            arguments: arguments
                                .iter()
                                .map(|argument| argument.argument_type(scope))
                                .collect(),
                            span: *call_span,
                        });
                    }
                } else if !matches!(callee.as_ref(), Expression::Member(_, _, _)) {
                    self.unresolved_calls.push(UnresolvedCall {
                        name: None,
                        reason: "computed callable expressions are not resolved".into(),
                        arguments: arguments
                            .iter()
                            .map(|argument| argument.argument_type(scope))
                            .collect(),
                        span: *call_span,
                    });
                }
                if let Expression::Member(receiver, operation, span) = callee.as_ref() {
                    let receiver = match receiver.as_ref() {
                        Expression::Name(name, location) => scope.resolve(name, location.start),
                        _ => Receiver::Unresolved,
                    };
                    self.calls.push(MemberCall {
                        operation: operation.clone(),
                        arguments: arguments.len(),
                        argument_types: arguments
                            .iter()
                            .map(|argument| {
                                let (literal_type, span) = match argument {
                                    Expression::Literal(kind, span) => (Some(kind.clone()), *span),
                                    Expression::Record(_, _, span)
                                    | Expression::Name(_, span)
                                    | Expression::QualifiedName(_, span)
                                    | Expression::Member(_, _, span)
                                    | Expression::Call(_, _, span)
                                    | Expression::Binary(_, _, _, span) => (None, *span),
                                };
                                let binding_type = if !matches!(argument, Expression::Literal(_, _))
                                {
                                    argument.known_type(scope)
                                } else {
                                    None
                                };
                                ArgumentType {
                                    literal_type,
                                    binding_type,
                                    span,
                                }
                            })
                            .collect(),
                        receiver,
                        span: *span,
                    });
                }
            }
            Expression::Member(receiver, name, span) => {
                self.inspect(receiver, scope);
                if let Some(kind) = receiver.known_type(scope) {
                    if scope.field_type(&kind, name).is_none() {
                        self.field_errors
                            .push((*span, format!("cannot resolve field `{name}` on {kind:?}")));
                    }
                }
            }
            Expression::Binary(left, operator, right, span) => {
                self.inspect(left, scope);
                self.inspect(right, scope);
                if let (Some(left), Some(right)) = (left.known_type(scope), right.known_type(scope))
                {
                    if binary_type(operator, &left, &right).is_none() {
                        self.operator_errors.push((
                            *span,
                            format!(
                                "operator `{operator}` cannot be applied to {left:?} and {right:?}"
                            ),
                        ));
                    }
                }
            }
            Expression::Name(_, _)
            | Expression::QualifiedName(_, _)
            | Expression::Literal(_, _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_and_excessively_nested_record_constructors_reject_inspection() {
        for expression in ["Point { x 1 }", "Point { x: }", "Point { x: 1 y: 2 }"] {
            let source =
                format!("struct Point {{ x: Int }} fn main() {{ let value = {expression}; }}");
            assert!(inspect_functions(&source, &[]).is_err(), "{expression}");
        }
        let source = format!(
            "struct Box {{ value: Box }} fn main() {{ let value = {}0{}; }}",
            "Box { value: ".repeat(140),
            " }".repeat(140)
        );
        assert!(inspect_functions(&source, &[]).is_err());
    }

    #[test]
    fn integer_range_evidence_preserves_token_spans_and_function_isolation() {
        let source = "// λ\r\nfn bad() { if false { print((9223372036854775808)); } } fn good() { 0; 9223372036854775807; }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(functions[0].invalid_integer_literals.len(), 1);
        let span = functions[0].invalid_integer_literals[0];
        assert_eq!(&source[span.start..span.end], "9223372036854775808");
        assert!(functions[1].invalid_integer_literals.is_empty());
    }

    #[test]
    fn discarded_expression_evidence_is_scoped_to_each_function() {
        let source = "fn bad() { while false { missing; } } fn good() { 1; print(1); }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(functions[0].unresolved_discarded_expressions.len(), 1);
        let span = functions[0].unresolved_discarded_expressions[0];
        assert_eq!(&source[span.start..span.end], "missing");
        assert!(functions[1].unresolved_discarded_expressions.is_empty());
    }

    #[test]
    fn logical_operands_are_checked_even_on_short_circuit_paths() {
        let source = "fn main() { if false && (1 || true) {} if true || missing() {} }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(functions[0].operator_errors.len(), 1);
        let span = functions[0].operator_errors[0].0;
        assert_eq!(&source[span.start..span.end], "(1 || true)");
        assert_eq!(functions[0].unresolved_calls.len(), 1);
        assert!(functions[0]
            .conditions
            .iter()
            .all(|(kind, _)| kind.is_none()));
    }

    #[test]
    fn logical_call_order_and_depth_limits_are_preserved() {
        let calls = inspect_body(
            "{ maps.first() || maps.second() && maps.third(); }",
            &[],
            &[service()],
        )
        .unwrap();
        assert_eq!(
            calls
                .iter()
                .map(|call| call.operation.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "third"]
        );
        let source = format!("fn main() {{ if {}true {{}} }}", "true && ".repeat(140));
        assert!(inspect_functions(&source, &[]).is_err());
    }

    #[test]
    fn application_logical_guards_follow_executable_precedence() {
        let source = "fn main() { if 1 < 2 && false || true { print(1); } }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(functions[0].conditions[0].0, Some(Type::Bool));
        assert!(functions[0].operator_errors.is_empty());
    }

    #[test]
    fn unresolved_member_receivers_require_a_callable_contract() {
        let functions =
            inspect_functions("fn main() { missing.send(); missing.child.send(); }", &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 2);
        assert!(diagnostics.items.iter().all(|item| item.code == "E4133"));
    }

    #[test]
    fn scalar_receivers_cannot_supply_callable_members() {
        let source = "fn text() -> String { return \"value\"; } fn main() { let value = 1; value.send(); (true).send(); text().send(); }";
        let functions = inspect_functions(source, &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 3);
        assert!(diagnostics.items.iter().all(|item| item.code == "E4133"));
    }

    #[test]
    fn task_return_annotations_must_resolve_even_when_unused() {
        let source = "task unused() -> Missing {} task valid() -> Int { return 1; }";
        let functions = inspect_functions(source, &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 1);
        assert_eq!(diagnostics.items[0].code, "E4131");
        assert_eq!(diagnostics.items[0].span, functions[0].span);
    }

    #[test]
    fn unused_task_parameters_still_require_resolved_annotations() {
        let source = "task unused(value: Missing, other: Any) {} task valid(value: Int) {}";
        let functions = inspect_functions(source, &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 2);
        assert!(diagnostics.items.iter().all(|error| error.code == "E4134"));
        assert!(functions
            .iter()
            .all(|function| function.signature.is_none()));
    }

    #[test]
    fn unused_ordinary_parameters_still_require_resolved_annotations() {
        let source = "fn unused(value: Text, other: Any) {}";
        let functions = inspect_functions(source, &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 2);
        assert!(diagnostics.items.iter().all(|error| error.code == "E4134"));
    }

    #[test]
    fn builtin_calls_validate_arity_types_and_preserve_shadowing() {
        let source = "fn main() { std::len(1); std::len(); std::to_string(unknown); print(1); std::println(false); } fn local(print: String) { mail.send(print(1)); mail.send(std::len(\"ok\")); }";
        let functions = inspect_functions(source, &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(
            diagnostics
                .items
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            ["E4129", "E4128", "E4130", "E4133", "E4133", "E4133"]
        );
        assert_eq!(
            functions[1].calls[0].argument_types[0].resolved_type(),
            None
        );
        assert_eq!(
            functions[1].calls[1].argument_types[0].resolved_type(),
            Some(&Type::Int)
        );
    }

    #[test]
    fn builtin_collision_and_user_any_annotations_are_not_callable_wildcards() {
        let functions =
            inspect_functions("fn print(value: Int) {} fn main() { print(1); }", &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 2);
        assert_eq!(diagnostics.items[0].code, "E4132");
        assert_eq!(diagnostics.items[1].code, "E4133");
        let functions =
            inspect_functions("fn user(value: Any) {} fn main() { user(1); }", &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 2);
        assert_eq!(diagnostics.items[0].code, "E4134");
        assert_eq!(diagnostics.items[1].code, "E4130");
    }

    #[test]
    fn standard_library_results_feed_application_types() {
        let source = "fn main() { mail.send(std::len(\"hello\")); mail.send(std::to_string(12)); mail.send(print(true)); }";
        let functions = inspect_functions(source, &[]).unwrap();
        for (call, expected) in functions[0]
            .calls
            .iter()
            .zip([Type::Int, Type::String, Type::Unit])
        {
            assert_eq!(call.argument_types[0].resolved_type(), Some(&expected));
        }
    }

    #[test]
    fn ordinary_result_inference_excludes_invalid_calls_and_task_names() {
        let functions = inspect_functions("fn helper(value: Int) -> Float { return value; } task work() -> Int { return 1; } fn caller() { mail.send(helper(true)); mail.send(helper()); mail.send(helper(unknown)); mail.send(work()); mail.send(other::helper(1)); }", &[]).unwrap();
        let calls = &functions[2].calls;
        assert_eq!(calls.len(), 5);
        assert!(calls
            .iter()
            .all(|call| call.argument_types[0].resolved_type().is_none()));
        assert_eq!(functions[2].function_calls.len(), 3);
        let isolated = inspect_functions("fn caller() { mail.send(helper(1)); }", &[]).unwrap();
        assert_eq!(isolated[0].calls[0].argument_types[0].resolved_type(), None);
    }

    #[test]
    fn ordinary_return_checks_cover_unknown_annotations_and_paths() {
        for (source, count) in [
            ("fn value() -> Int {}", 1),
            ("fn value() -> Int { return; }", 1),
            ("fn value() -> Float { return 1; }", 0),
            (
                "fn value(flag: Bool) -> Int { if flag { return 1; } else { return 2; } }",
                0,
            ),
            ("fn value() -> Int { return unknown; }", 1),
            ("fn value() -> Missing { return 1; }", 1),
        ] {
            let functions = inspect_functions(source, &[]).unwrap();
            let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
            ordinary::check(
                &functions,
                std::path::Path::new("main.svr"),
                &mut diagnostics,
            );
            assert_eq!(diagnostics.items.len(), count, "{source}");
            assert!(diagnostics.items.iter().all(|error| error.code == "E4131"));
        }
    }

    #[test]
    fn ordinary_calls_respect_shadowing_recursion_and_duplicate_boundaries() {
        let source = "fn value(n: Int) -> Int { if n == 0 { return 0; } else { return value(n - 1); } } fn caller(value: String) { mail.send(value(1)); } fn sibling() { mail.send(value(1)); }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(functions[0].returns[1].known_type, Some(Type::Int));
        assert!(functions[1].function_calls.is_empty());
        assert_eq!(
            functions[1].calls[0].argument_types[0].resolved_type(),
            None
        );
        assert_eq!(
            functions[2].calls[0].argument_types[0].resolved_type(),
            Some(&Type::Int)
        );
        let duplicate = inspect_functions("fn value() -> Int { return 1; } fn value() -> Float { return 1; } fn caller() { mail.send(value()); }", &[]).unwrap();
        assert_eq!(
            duplicate[2].calls[0].argument_types[0].resolved_type(),
            None
        );
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &duplicate,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 3);
        assert_eq!(diagnostics.items[0].code, "E4127");
        assert_eq!(diagnostics.items[1].code, "E4133");
        assert_eq!(diagnostics.items[2].code, "E4133");
    }

    #[test]
    fn ordinary_contract_checks_preserve_ranges_and_unknowns() {
        let source = "fn main() { take(); take(true); take(unknown); } fn take(value: Float) -> Int { return \"bad\"; }";
        let functions = inspect_functions(source, &[]).unwrap();
        let mut diagnostics = crate::compiler::diagnostics::Diagnostics::new();
        ordinary::check(
            &functions,
            std::path::Path::new("main.svr"),
            &mut diagnostics,
        );
        for (error, (code, spelling)) in diagnostics.items.iter().zip([
            ("E4128", "take()"),
            ("E4129", "true"),
            ("E4130", "unknown"),
            ("E4131", "\"bad\""),
        ]) {
            assert_eq!(error.code, code);
            assert_eq!(&source[error.span.start..error.span.end], spelling);
        }
        assert_eq!(diagnostics.items.len(), 4);
    }

    #[test]
    fn ordinary_forward_calls_propagate_primitive_results() {
        let source = "fn main() { mail.send(amount(1)); if ready() { mail.send(2); } } fn amount(value: Float) -> Float { return value; } fn ready() -> Bool { return true; }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(
            functions[0].calls[0].argument_types[0].resolved_type(),
            Some(&Type::Float)
        );
        assert_eq!(functions[0].conditions[0].0, Some(Type::Bool));
    }

    #[test]
    fn structured_return_paths_are_conservative() {
        for (body, expected) in [
            ("if flag { return 1; } else { return 2; }", true),
            ("if flag { return 1; }", false),
            ("while flag { return 1; }", false),
            ("while flag {} return 1;", true),
            (
                "if flag { return 1; } else if flag { return 2; } else { return 3; }",
                true,
            ),
            ("if flag { return 1; } else if flag { return 2; }", false),
        ] {
            let source = format!("fn example(flag: Bool) {{ {body} }}");
            assert_eq!(
                inspect_functions(&source, &[]).unwrap()[0].always_returns,
                expected,
                "{body}"
            );
        }
        let chain = "if true {} else ".repeat(140);
        assert!(inspect_functions(&format!("fn example() {{ {chain} {{}} }}"), &[]).is_err());
    }

    #[test]
    fn branch_bindings_do_not_leak_to_other_paths() {
        let functions = inspect_functions("fn example() { let value = 1; if true { let value = false; mail.send(value); } else { mail.send(value); } while false { let value = \"x\"; mail.send(value); } mail.send(value); }", &[]).unwrap();
        let expected = [Type::Bool, Type::Int, Type::String, Type::Int];
        for (call, expected) in functions[0].calls.iter().zip(&expected) {
            assert_eq!(call.argument_types[0].resolved_type(), Some(expected));
        }
    }

    #[test]
    fn branch_and_loop_inspection_preserves_calls_and_scope() {
        let source = "fn example(flag: Bool) { if flag { let value = 1; mail.send(value); } else { mail.send(\"other\"); } while flag { mail.send(false); } }";
        let functions = inspect_functions(source, &[]).unwrap();
        assert_eq!(functions[0].calls.len(), 3);
        assert_eq!(
            functions[0].calls[0].argument_types[0].resolved_type(),
            Some(&Type::Int)
        );
    }

    #[test]
    fn return_evidence_tracks_nested_scopes_and_resets_between_functions() {
        let source = "fn first(value: Int) { { let value = \"text\"; return value; } return value + 1; return unknown; } fn second() { return; }";
        let functions = inspect_functions(source, &[]).unwrap();
        let returns = &functions[0].returns;
        assert_eq!(returns.len(), 3);
        assert_eq!(returns[0].known_type, Some(Type::String));
        assert_eq!(returns[1].known_type, Some(Type::Int));
        assert_eq!(returns[2].known_type, None);
        assert_eq!(
            &source[returns[1].span.start..returns[1].span.end],
            "value + 1"
        );
        assert_eq!(functions[1].returns.len(), 1);
        assert_eq!(functions[1].returns[0].known_type, Some(Type::Unit));
    }

    #[test]
    fn invalid_binary_operands_report_once_without_result_type() {
        let source = "fn main() { mail.send((true + 1) * 2); mail.send(true < false); mail.send(unknown + 1); }";
        let functions = inspect_functions(source, &[]).unwrap();
        let function = &functions[0];
        assert_eq!(function.operator_errors.len(), 2);
        assert_eq!(
            &source[function.operator_errors[0].0.start..function.operator_errors[0].0.end],
            "(true + 1)"
        );
        for call in &function.calls {
            assert_eq!(call.argument_types[0].resolved_type(), None);
        }
    }

    #[test]
    fn compound_arguments_retain_primitive_result_types() {
        let source = "fn example(value: Int) { mail.send(value + 1.5, 1 + 2 * 3, \"a\" + \"b\", value < 2, true == false, unknown + 1); }";
        let functions = inspect_functions(source, &[]).unwrap();
        let arguments = &functions[0].calls[0].argument_types;
        let expected = [
            Some(Type::Float),
            Some(Type::Int),
            Some(Type::String),
            Some(Type::Bool),
            Some(Type::Bool),
            None,
        ];
        for (argument, expected) in arguments.iter().zip(&expected) {
            assert_eq!(argument.resolved_type(), expected.as_ref());
        }
    }

    #[test]
    fn annotated_local_types_require_valid_known_initializers() {
        let source = "fn example() { let a: Float = 1; mail.send(a); let b: String = unknown; mail.send(b); let c: Custom = 1; mail.send(c); let d: Int = true; mail.send(d); } fn next() { mail.send(1); }";
        let functions = inspect_functions(source, &[]).unwrap();
        let first = &functions[0];
        assert_eq!(
            first.calls[0].argument_types[0].resolved_type(),
            Some(&Type::Float)
        );
        for call in &first.calls[1..] {
            assert_eq!(call.argument_types[0].resolved_type(), None);
        }
        assert_eq!(first.initializer_mismatches.len(), 1);
        assert!(functions[1].initializer_mismatches.is_empty());
        let mismatch = &first.initializer_mismatches[0];
        assert_eq!(&source[mismatch.span.start..mismatch.span.end], "true");
    }

    #[test]
    fn variable_arguments_follow_lexical_types_without_scope_leaks() {
        let source = "fn example(value: String) { mail.send(value); let copy = value; { let copy = true; mail.send(copy); } mail.send(copy); let copy = unknown; mail.send(copy); }";
        let functions = inspect_functions(source, &[]).unwrap();
        let calls = &functions[0].calls;
        assert_eq!(calls.len(), 4);
        assert_eq!(
            calls[0].argument_types[0].resolved_type(),
            Some(&Type::String)
        );
        assert_eq!(
            calls[1].argument_types[0].resolved_type(),
            Some(&Type::Bool)
        );
        assert_eq!(
            calls[2].argument_types[0].resolved_type(),
            Some(&Type::String)
        );
        assert_eq!(calls[3].argument_types[0].resolved_type(), None);
    }

    #[test]
    fn argument_evidence_preserves_unknown_expressions_and_nested_calls() {
        let source = "{ mail.send(value, 1 + 2, other.read(1), (false)); }";
        let calls = inspect_body(source, &["value"], &[]).unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].argument_types[0].literal_type, Some(Type::Int));
        let arguments = &calls[1].argument_types;
        assert_eq!(arguments.len(), 4);
        for argument in &arguments[..3] {
            assert_eq!(argument.literal_type, None);
        }
        assert_eq!(arguments[3].literal_type, Some(Type::Bool));
        for (argument, expected) in
            arguments
                .iter()
                .zip(["value", "1 + 2", "other.read(1)", "(false)"])
        {
            assert_eq!(&source[argument.span.start..argument.span.end], expected);
        }
    }

    #[test]
    fn inspection_rejects_untyped_declaration_parameters() {
        for source in [
            "fn example(value) {}",
            "task example(value) {}",
            "service mail { fn send(value); }",
        ] {
            let error = inspect_functions(source, &[]).unwrap_err();
            assert!(error.contains("explicit type annotation"), "{error}");
        }
    }

    #[test]
    fn service_bodies_use_explicit_file_and_independent_lexical_scopes() {
        let source = "service maps { fn send(x: Int); fn relay() { maps.send(1); let maps = maps.make(); maps.local(); } fn shadow(maps: Client) { maps.local(); } fn next() { maps.send(2); } }";
        let functions = inspect_functions(source, &[service()]).unwrap();
        assert_eq!(
            functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            ["maps.relay", "maps.shadow", "maps.next"]
        );
        assert_eq!(functions[0].calls[0].receiver, Receiver::Service(service()));
        assert_eq!(functions[0].calls[1].receiver, Receiver::Service(service()));
        assert_eq!(functions[0].calls[2].receiver, Receiver::Local);
        assert_eq!(functions[1].calls[0].receiver, Receiver::Local);
        assert_eq!(functions[2].calls[0].receiver, Receiver::Service(service()));
        for function in functions {
            assert!(!function.is_task);
            for call in function.calls {
                assert!(source[call.span.start..call.span.end].starts_with("maps."));
            }
        }
    }

    #[test]
    fn application_depth_limits_cover_blocks_trees_and_nesting() {
        for (body, accepted) in [
            (
                format!(
                    "{}{}1{}{}",
                    "{".repeat(128),
                    "(".repeat(128),
                    ")".repeat(128),
                    "}".repeat(128)
                ),
                true,
            ),
            (format!("{}{}", "{".repeat(128), "}".repeat(128)), true),
            (format!("{}{}", "{".repeat(129), "}".repeat(129)), false),
            (
                format!("{{ {}1{} }}", "(".repeat(128), ")".repeat(128)),
                true,
            ),
            (
                format!("{{ {}1{} }}", "(".repeat(129), ")".repeat(129)),
                false,
            ),
            (format!("{{ {} }}", vec!["1"; 128].join("+")), true),
            (format!("{{ {} }}", vec!["1"; 129].join("+")), false),
            (
                format!("{{ {}1{} }}", "f(".repeat(127), ")".repeat(127)),
                true,
            ),
            (
                format!("{{ {}1{} }}", "f(".repeat(128), ")".repeat(128)),
                false,
            ),
            (format!("{{ maps{}() }}", ".member".repeat(126)), true),
            (format!("{{ maps{}() }}", ".member".repeat(127)), false),
            (format!("{{ f({}) }}", vec!["1"; 1024].join(",")), true),
        ] {
            let result = inspect_body(&body, &[], &[service()]);
            if accepted {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result
                    .unwrap_err()
                    .contains("maximum structural depth of 128"));
            }
        }
        let source = format!(
            "fn good() {{ maps.send(); }} fn bad() {{ {}1{} }}",
            "(".repeat(2048),
            ")".repeat(2048)
        );
        assert!(inspect_functions(&source, &[service()])
            .unwrap_err()
            .contains("128"));
    }

    #[test]
    fn complex_receiver_ranges_cover_the_entire_member_expression() {
        for (expression, expected) in [
            ("maps.make().send()", "maps.make().send"),
            ("(maps.make()).send()", "(maps.make()).send"),
            ("(maps.make() + 1).send()", "(maps.make() + 1).send"),
            ("\"text\".send()", "\"text\".send"),
            ("(maps).send()", "(maps).send"),
        ] {
            let source = format!("{{ // Unicode: λ\r\n  {expression}; }}");
            let calls = inspect_body(&source, &[], &[service()]).unwrap();
            let call = calls.last().unwrap();
            assert_eq!(&source[call.span.start..call.span.end], expected);
            assert_eq!(call.span.line, 1);
            assert_eq!(call.span.column, 2);
            assert_eq!(
                call.receiver,
                if expression == "(maps).send()" {
                    Receiver::Service(service())
                } else {
                    Receiver::Unresolved
                }
            );
        }
    }

    #[test]
    fn qualified_calls_inspect_arguments_without_becoming_services() {
        let source =
            "{ std::println(maps.send(1)); maps::send(2); maps::client.send(maps.make()); }";
        let calls = inspect_body(source, &[], &[service()]).unwrap();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].operation, "send");
        assert_eq!(calls[0].receiver, Receiver::Service(service()));
        assert_eq!(calls[1].operation, "make");
        assert_eq!(calls[1].receiver, Receiver::Service(service()));
        assert_eq!(calls[2].receiver, Receiver::Unresolved);
        assert_eq!(
            &source[calls[2].span.start..calls[2].span.end],
            "maps::client.send"
        );
    }

    #[test]
    fn malformed_qualified_names_discard_inspection() {
        for expression in [
            "std::",
            "std::(1)",
            "1::send()",
            "maps.send::other()",
            "std::io::(1)",
        ] {
            let source = format!("{{ maps.first(); {expression}; }}");
            assert!(
                inspect_body(&source, &[], &[service()]).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn unsupported_service_implementations_discard_inspection() {
        for body in ["if true", "let x = ;", "maps.send("] {
            let source =
                format!("fn first() {{ maps.send(); }} service maps {{ fn send() {{ {body} }} }}");
            assert!(inspect_functions(&source, &[service()]).is_err());
        }
    }

    #[test]
    fn empty_service_bodies_and_signatures_have_no_hidden_calls() {
        // Line comments and whitespace do not constitute an implementation.
        let source = "service maps { fn send(); fn empty() { // maps.hidden()\n } }";
        let functions = inspect_functions(source, &[service()]).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "maps.empty");
        assert!(functions[0].calls.is_empty());
    }

    #[test]
    fn binary_operands_preserve_calls_arguments_and_shadowing() {
        let source = "{ let maps = maps.first() + maps.second() * 2; maps.local(); return other.send((maps.third() - 1) / 2 >= maps.fourth(), 1 != 2); }";
        let calls = inspect_body(source, &[], &[service()]).unwrap();
        assert_eq!(
            calls
                .iter()
                .map(|call| call.operation.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "local", "third", "fourth", "send"]
        );
        for call in &calls[..2] {
            assert_eq!(call.receiver, Receiver::Service(service()));
        }
        for call in &calls[2..5] {
            assert_eq!(call.receiver, Receiver::Local);
        }
        assert_eq!(calls[5].arguments, 2);
    }

    #[test]
    fn incomplete_binary_operands_discard_all_calls() {
        for expression in [
            "maps.send() +",
            "1 * / maps.send()",
            "maps.send() ==",
            "1 + (maps.send() * )",
        ] {
            let source = format!("{{ maps.first(); {expression}; }}");
            assert!(
                inspect_body(&source, &[], &[service()]).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn typed_locals_preserve_initializer_and_shadowing_order() {
        for annotation in [
            "MapClient",
            "Result<MapClient, Error>",
            "(Int, Text) -> MapClient",
            "[MapClient]",
        ] {
            let source = format!("{{ let maps: {annotation} = maps.create(); maps.send(); }}");
            let calls = inspect_body(&source, &[], &[service()]).unwrap();
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0].receiver, Receiver::Service(service()));
            assert_eq!(calls[1].receiver, Receiver::Local);
        }
    }

    #[test]
    fn malformed_local_annotations_discard_all_inspection_results() {
        for annotation in ["", "Result<Text, Error", "Text]", "Text, Int", "Text + Int"] {
            let source = format!("{{ maps.send(); let value: {annotation} = maps.create(); }}");
            assert!(
                inspect_body(&source, &[], &[service()]).is_err(),
                "{annotation}"
            );
        }
    }
    fn service() -> ServiceIdentity {
        ServiceIdentity {
            module: "app.services".into(),
            name: "maps".into(),
        }
    }

    #[test]
    fn file_declarations_supply_parameters_and_preserve_source_spans() {
        let source = "// Unicode: λ\r\nfn first(\n maps: Object // shadows import\n) -> Result<Text, Error> { return maps.send(); }\ntask second() { maps.send(1); }";
        let functions = inspect_functions(source, &[service()]).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "first");
        assert!(!functions[0].is_task);
        assert_eq!(functions[0].calls[0].receiver, Receiver::Local);
        assert!(functions[1].is_task);
        assert_eq!(functions[1].calls[0].receiver, Receiver::Service(service()));
        for function in functions {
            let call = &function.calls[0];
            assert_eq!(&source[call.span.start..call.span.end], "maps.send");
            assert!(source[function.span.start..function.span.end].ends_with('}'));
        }
    }

    #[test]
    fn file_inspection_rejects_unsupported_and_incomplete_declarations() {
        for source in [
            "fn good() {} fn bad() { if true }",
            "fn missing()",
            "fn missing() fn next() {}",
            "fn bad(); {}",
            "use ../services\nfn main() {}",
            "model Example {}",
        ] {
            assert!(inspect_functions(source, &[service()]).is_err(), "{source}");
        }
    }

    #[test]
    fn parsed_bindings_and_blocks_control_receiver_resolution() {
        let source =
            "{ maps.travel(1); { let maps = maps.make(); maps.travel(2); } maps.travel(3); }";
        let calls = inspect_body(source, &[], &[service()]).unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].receiver, Receiver::Service(service()));
        assert_eq!(calls[1].receiver, Receiver::Service(service()));
        assert_eq!(calls[2].receiver, Receiver::Local);
        assert_eq!(calls[3].receiver, Receiver::Service(service()));
        assert_eq!(
            &source[calls[0].span.start..calls[0].span.end],
            "maps.travel"
        );
        assert_eq!(calls[0].arguments, 1);
    }

    #[test]
    fn comments_strings_parameters_and_nested_calls_are_structural() {
        let calls = inspect_body(
            "{ // maps.fake()\n print(\"maps.fake()\"); return maps.send(other.make()); }",
            &["maps"],
            &[service()],
        )
        .unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].receiver, Receiver::Unresolved);
        assert_eq!(calls[1].receiver, Receiver::Local);
    }

    #[test]
    fn unsupported_or_incomplete_bodies_do_not_return_partial_calls() {
        for source in [
            "{ maps.send(); if true }",
            "{ maps.send(); let f = fn value => value; }",
            "{ maps.send(",
            "{ maps.send();",
            "{ let x: = 1; }",
            "{ maps.send() && ; }",
        ] {
            assert!(inspect_body(source, &[], &[service()]).is_err(), "{source}");
        }
    }
}
