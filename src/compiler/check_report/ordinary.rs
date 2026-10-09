//! Additive JSON inspection records for ordinary and builtin calls.

use std::fmt::Write as _;

use crate::compiler::diagnostics::Span;
use crate::compiler::project::application::{ArgumentType, ServiceCheck};
use crate::compiler::semantic::Type;

#[derive(Debug)]
struct CallRecord<'a> {
    name: Option<&'a str>,
    kind: &'static str,
    reason: Option<&'a str>,
    declared_return: Option<&'a Type>,
    declaration_file: Option<&'a std::path::Path>,
    arguments: &'a [ArgumentType],
    span: Span,
}

pub(super) fn append(output: &mut String, report: &ServiceCheck) {
    output.push_str(",\"ordinary_calls\":[");
    let mut first = true;
    for file in &report.files {
        let Ok(functions) = &file.functions else {
            continue;
        };
        let file_name = file.source_file.to_string_lossy();
        for function in functions {
            let mut calls = Vec::new();
            for call in &function.function_calls {
                calls.push(CallRecord {
                    name: Some(&call.name),
                    kind: if crate::compiler::stdlib::lookup(&call.name).is_some() {
                        "builtin"
                    } else {
                        "ordinary"
                    },
                    reason: None,
                    declared_return: Some(&call.signature.return_type),
                    declaration_file: if crate::compiler::stdlib::lookup(&call.name).is_some() {
                        None
                    } else {
                        call.signature
                            .source_file
                            .as_deref()
                            .or(Some(file.source_file.as_path()))
                    },
                    arguments: &call.arguments,
                    span: call.span,
                });
            }
            for call in &function.unresolved_calls {
                calls.push(CallRecord {
                    name: call.name.as_deref(),
                    kind: "unresolved",
                    reason: Some(&call.reason),
                    declared_return: None,
                    declaration_file: None,
                    arguments: &call.arguments,
                    span: call.span,
                });
            }
            calls.sort_by_key(|call| (call.span.start, call.span.end));
            for call in calls {
                if !first {
                    output.push(',');
                }
                first = false;
                output.push_str("{\"function\":");
                super::push_string(output, &function.name);
                let _ = write!(output, ",\"is_task\":{},\"callee\":", function.is_task);
                super::push_optional_string(output, call.name);
                output.push_str(",\"kind\":");
                super::push_string(output, call.kind);
                output.push_str(",\"reason\":");
                super::push_optional_string(output, call.reason);
                output.push_str(",\"declared_return_type\":");
                push_type(output, call.declared_return);
                output.push_str(",\"declared_return_type_info\":");
                push_type_info(output, call.declared_return);
                output.push_str(",\"declaration_file\":");
                if let Some(file) = call.declaration_file {
                    super::push_string(output, &file.to_string_lossy());
                } else {
                    output.push_str("null");
                }
                output.push_str(",\"arguments\":[");
                for (index, argument) in call.arguments.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    output.push_str("{\"type\":");
                    push_type(output, argument.resolved_type());
                    output.push_str(",\"type_info\":");
                    push_type_info(output, argument.resolved_type());
                    output.push_str(",\"location\":");
                    push_location(output, &file_name, argument.span);
                    output.push('}');
                }
                output.push_str("],\"location\":");
                push_location(output, &file_name, call.span);
                output.push('}');
            }
        }
    }
    output.push(']');
    append_locals(output, report);
}

fn append_locals(output: &mut String, report: &ServiceCheck) {
    output.push_str(",\"local_bindings\":[");
    let mut first = true;
    for file in &report.files {
        let Ok(functions) = &file.functions else {
            continue;
        };
        let file_name = file.source_file.to_string_lossy();
        for function in functions {
            for binding in &function.local_bindings {
                if !first {
                    output.push(',');
                }
                first = false;
                output.push_str("{\"function\":");
                super::push_string(output, &function.name);
                let _ = write!(output, ",\"is_task\":{},\"name\":", function.is_task);
                super::push_string(output, &binding.name);
                output.push_str(",\"declared_type\":");
                push_type(output, binding.declared_type.as_ref());
                output.push_str(",\"initializer_type\":");
                push_type(output, binding.initializer_type.as_ref());
                output.push_str(",\"resolved_type\":");
                push_type(output, binding.resolved_type.as_ref());
                output.push_str(",\"declared_type_info\":");
                push_type_info(output, binding.declared_type.as_ref());
                output.push_str(",\"initializer_type_info\":");
                push_type_info(output, binding.initializer_type.as_ref());
                output.push_str(",\"resolved_type_info\":");
                push_type_info(output, binding.resolved_type.as_ref());
                output.push_str(",\"location\":");
                push_location(output, &file_name, binding.span);
                output.push_str(",\"annotation_location\":");
                if let Some(span) = binding.annotation_span {
                    push_location(output, &file_name, span);
                } else {
                    output.push_str("null");
                }
                output.push_str(",\"initializer_location\":");
                push_location(output, &file_name, binding.initializer_span);
                output.push('}');
            }
        }
    }
    output.push(']');
}

fn push_type(output: &mut String, kind: Option<&Type>) {
    super::push_optional_string(
        output,
        match kind {
            Some(Type::Unit) => Some("Unit"),
            Some(Type::Bool) => Some("Bool"),
            Some(Type::Int) => Some("Int"),
            Some(Type::Float) => Some("Float"),
            Some(Type::String) => Some("String"),
            _ => None,
        },
    );
}

pub(super) fn push_type_info(output: &mut String, kind: Option<&Type>) {
    match kind {
        Some(Type::Named(identity)) => {
            output.push_str("{\"kind\":\"record\",\"identity\":");
            super::push_string(output, identity);
            output.push('}');
        }
        Some(Type::Array(element)) => {
            output.push_str("{\"kind\":\"array\",\"element\":");
            push_type_info(output, Some(element));
            output.push('}');
        }
        Some(Type::Unit | Type::Bool | Type::Int | Type::Float | Type::String) => {
            output.push_str("{\"kind\":\"scalar\",\"name\":");
            push_type(output, kind);
            output.push('}');
        }
        _ => output.push_str("null"),
    }
}

fn push_location(output: &mut String, file: &str, span: Span) {
    output.push_str("{\"file\":");
    super::push_string(output, file);
    let _ = write!(
        output,
        ",\"start\":{},\"end\":{},\"line\":{},\"column\":{}}}",
        span.start, span.end, span.line, span.column
    );
}
