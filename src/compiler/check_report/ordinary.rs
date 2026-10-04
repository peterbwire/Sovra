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
                output.push_str(",\"arguments\":[");
                for (index, argument) in call.arguments.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    output.push_str("{\"type\":");
                    push_type(output, argument.resolved_type());
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

fn push_location(output: &mut String, file: &str, span: Span) {
    output.push_str("{\"file\":");
    super::push_string(output, file);
    let _ = write!(
        output,
        ",\"start\":{},\"end\":{},\"line\":{},\"column\":{}}}",
        span.start, span.end, span.line, span.column
    );
}
