//! Validation for resolved same-file ordinary functions.

use std::collections::BTreeSet;
use std::path::Path;

use super::FunctionCalls;
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::compiler::semantic::Type;

fn compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual || (*expected == Type::Float && *actual == Type::Int)
}

pub(super) fn check(functions: &[FunctionCalls], file: &Path, diagnostics: &mut Diagnostics) {
    let mut names = BTreeSet::new();
    let mut emit = |code, message: String, span: Span| {
        diagnostics.push(Diagnostic {
            source_file: Some(file.to_string_lossy().into_owned()),
            severity: Severity::Error,
            code,
            message,
            span,
        });
    };
    for function in functions {
        if function.task_return_annotation == Some(Type::Unknown) {
            emit(
                "E4131",
                format!(
                    "cannot validate unresolved return annotation for task `{}`",
                    function.name
                ),
                function.span,
            );
        }
        for parameter in &function.unresolved_task_parameters {
            emit(
                "E4134",
                format!(
                    "parameter `{parameter}` of task `{}` has an unresolved annotation",
                    function.name
                ),
                function.span,
            );
        }
        for call in &function.unresolved_calls {
            emit(
                "E4133",
                format!(
                    "cannot validate call `{}`: {}",
                    call.name.as_deref().unwrap_or("<expression>"),
                    call.reason
                ),
                call.span,
            );
        }
        if let Some(signature) = &function.signature {
            if function.is_exported && signature.exposes_private_records() {
                emit(
                    "E4116",
                    format!(
                        "exported function `{}` exposes a private application record",
                        function.name
                    ),
                    function.span,
                );
            }
            for (index, kind) in signature.parameters.iter().enumerate() {
                if *kind == Type::Unknown {
                    emit(
                        "E4134",
                        format!(
                            "parameter {} of ordinary function `{}` has an unresolved annotation",
                            index + 1,
                            function.name
                        ),
                        function.span,
                    );
                }
            }
            if crate::compiler::stdlib::lookup(&function.name).is_some() {
                emit(
                    "E4132",
                    format!(
                        "ordinary function `{}` collides with a builtin callable",
                        function.name
                    ),
                    function.span,
                );
            }
            if !names.insert(&function.name) {
                emit(
                    "E4127",
                    format!("duplicate ordinary function `{}`", function.name),
                    function.span,
                );
            }
            if signature.return_type == Type::Unknown {
                emit(
                    "E4131",
                    format!(
                        "cannot validate unresolved return annotation for `{}`",
                        function.name
                    ),
                    function.span,
                );
            } else {
                if signature.return_type != Type::Unit && !function.always_returns {
                    emit(
                        "E4131",
                        format!(
                            "function `{}` can finish without returning {:?}",
                            function.name, signature.return_type
                        ),
                        function.span,
                    );
                }
                for returned in &function.returns {
                    match &returned.known_type {
                        Some(actual) if compatible(&signature.return_type, actual) => {}
                        Some(actual) => emit(
                            "E4131",
                            format!(
                                "function `{}` expects return {:?}, found {actual:?}",
                                function.name, signature.return_type
                            ),
                            returned.span,
                        ),
                        None => emit(
                            "E4131",
                            format!(
                                "cannot resolve return expression type for `{}`",
                                function.name
                            ),
                            returned.span,
                        ),
                    }
                }
            }
        }
        for call in &function.function_calls {
            if call.arguments.len() != call.signature.parameters.len() {
                emit(
                    "E4128",
                    format!(
                        "function `{}` expects {} argument(s), found {}",
                        call.name,
                        call.signature.parameters.len(),
                        call.arguments.len()
                    ),
                    call.span,
                );
                continue;
            }
            if call.signature.return_type == Type::Unknown
                || call
                    .signature
                    .parameters
                    .iter()
                    .enumerate()
                    .any(|(index, kind)| {
                        *kind == Type::Unknown
                            && !call
                                .signature
                                .any_parameters
                                .get(index)
                                .copied()
                                .unwrap_or(false)
                    })
            {
                emit(
                    "E4130",
                    format!("function `{}` has an unresolved signature", call.name),
                    call.span,
                );
                continue;
            }
            for (index, (argument, expected)) in call
                .arguments
                .iter()
                .zip(&call.signature.parameters)
                .enumerate()
            {
                match argument.resolved_type() {
                    Some(actual) if call.signature.accepts(index, actual) => {}
                    Some(actual) => emit(
                        "E4129",
                        format!(
                            "function `{}` argument {} expects {expected:?}, found {actual:?}",
                            call.name,
                            index + 1
                        ),
                        argument.span,
                    ),
                    None => emit(
                        "E4130",
                        format!(
                            "cannot resolve argument {} type for function `{}`",
                            index + 1,
                            call.name
                        ),
                        argument.span,
                    ),
                }
            }
        }
    }
}
