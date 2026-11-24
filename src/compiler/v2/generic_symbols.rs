//! Optional fail-closed declaration projection from complete checked original source.
use super::*;
use crate::compiler::{check_report::push_string, diagnostics::Span};
use std::fmt::Write as _;
/// Independent whole-batch optional evidence admission.
#[derive(Debug, Clone, Copy)]
pub struct GenericSymbolLimits {
    /// User source declarations; compiler-owned injected definitions are omitted.
    pub max_declarations: usize,
    /// Source calls and header type references; hard-capped at 8192.
    pub max_references: usize,
    /// Checked parameter/local/payload declarations across source bodies.
    pub max_locals: usize,
    /// Checked read/write references across source bodies.
    pub max_local_references: usize,
    /// JSON envelope bytes, independently hard-capped at 512 KiB.
    pub max_bytes: usize,
    /// Each name or retained declaration header, hard-capped at 4096 bytes.
    pub max_string_bytes: usize,
    /// Metadata visits, input-string scanning and emitted byte work.
    pub max_work: usize,
}
impl GenericSymbolLimits {
    /// Finite optional projection profile; failure does not invalidate compilation.
    pub const DEFAULT: Self = Self {
        max_declarations: 8192,
        max_references: 8192,
        max_locals: 8192,
        max_local_references: 8192,
        max_bytes: 512 * 1024,
        max_string_bytes: 4096,
        max_work: 4 * 1024 * 1024,
    };
}
// Iterative rendering of checked identities, never annotation text or Debug output.
fn primitive_name(value: GenericPrimitive) -> &'static str {
    match value {
        GenericPrimitive::Unit => "Unit",
        GenericPrimitive::Bool => "Bool",
        GenericPrimitive::Int => "Int",
        GenericPrimitive::Float => "Float",
        GenericPrimitive::String => "String",
    }
}
pub(super) fn type_label(ty: &GenericCopyType, max: usize, work: &mut usize) -> Option<String> {
    fn append(out: &mut String, text: &str, max: usize, work: &mut usize) -> Option<()> {
        if out.len().checked_add(text.len())? > max {
            return None;
        }
        charge(work, text.len())?;
        out.push_str(text);
        Some(())
    }
    let mut out = String::new();
    if let Some(p) = ty.primitive() {
        append(&mut out, primitive_name(p), max, work)?;
        return Some(out);
    }
    let expanded = ty.expanded()?;
    enum Task {
        Node(usize),
        Text(&'static str),
    }
    let mut pending = vec![Task::Node(expanded.root())];
    while let Some(task) = pending.pop() {
        charge(work, 1)?;
        match task {
            Task::Text(text) => append(&mut out, text, max, work)?,
            Task::Node(index) => match &expanded.nodes().get(index)?.kind {
                ResolvedGenericTypeKind::Primitive(p) => {
                    append(&mut out, primitive_name(*p), max, work)?
                }
                ResolvedGenericTypeKind::Parameter(id) => {
                    let parameter = expanded.parameters().parameter(*id)?;
                    append(&mut out, &parameter.name, max, work)?;
                }
                ResolvedGenericTypeKind::Array { element } => {
                    if *element >= index || pending.len().checked_add(2)? > max {
                        return None;
                    }
                    append(&mut out, "[", max, work)?;
                    pending.push(Task::Text("]"));
                    pending.push(Task::Node(*element));
                }
                ResolvedGenericTypeKind::Declared {
                    declaration,
                    arguments,
                } => {
                    let header = expanded.environment().registry().header(*declaration)?;
                    append(&mut out, &header.name, max, work)?;
                    if !arguments.is_empty() {
                        let slots = arguments.len().checked_mul(2)?;
                        if pending.len().checked_add(slots)? > max {
                            return None;
                        }
                        charge(work, arguments.len())?;
                        append(&mut out, "<", max, work)?;
                        pending.push(Task::Text(">"));
                        for (position, child) in arguments.iter().enumerate().rev() {
                            if *child >= index {
                                return None;
                            }
                            pending.push(Task::Node(*child));
                            if position > 0 {
                                pending.push(Task::Text(", "));
                            }
                        }
                    }
                }
            },
        }
    }
    Some(out)
}
const UNAVAILABLE:&str="{\"declarations\":[],\"references\":[],\"locals\":[],\"local_references\":[],\"truncated\":true,\"unavailable\":true}";
fn charge(left: &mut usize, n: usize) -> Option<()> {
    *left = left.checked_sub(n)?;
    Some(())
}
fn escaped_len(s: &str) -> Option<usize> {
    s.chars().try_fold(2usize, |n, c| {
        n.checked_add(match c {
            '"' | '\\' | '\n' | '\r' | '\t' | '\u{0008}' | '\u{000c}' => 2,
            c if (c as u32) < 32 || (0x7f..=0x9f).contains(&(c as u32)) => 6,
            c => c.len_utf8(),
        })
    })
}
/// Render a complete bounded root declaration/call-reference symbol envelope. Quota or provenance
/// refusal returns only the fixed unavailable envelope, never partial facts.
/// Call/local references use checked identities and original name tokens. Header
/// type references resolve retained syntax under the sealed signature registry.
pub fn generic_source_symbols(
    program: &CompiledGenericSource,
    limits: GenericSymbolLimits,
) -> String {
    project(program, limits).unwrap_or_else(|| UNAVAILABLE.to_owned())
}
fn project(program: &CompiledGenericSource, mut limits: GenericSymbolLimits) -> Option<String> {
    let max_bytes = limits.max_bytes.min(512 * 1024);
    let max_string = limits.max_string_bytes.min(4096);
    if program.declarations().len() > limits.max_declarations.min(8192) {
        return None;
    }
    let mut out = String::from("{\"declarations\":[");
    for (index, decl) in program.declarations().iter().enumerate() {
        charge(&mut limits.max_work, 1)?;
        let (name_span, fragment, header_end, kind) = match decl.kind() {
            GenericSourceDeclarationKind::Function(v) => {
                (v.name_span(), v.source(), v.body_span().start, 12)
            }
            GenericSourceDeclarationKind::Record(v) => {
                (v.name_span(), v.source(), v.parameter_range().end, 23)
            }
            GenericSourceDeclarationKind::Enum(v) => {
                (v.name_span(), v.source(), v.parameter_range().end, 10)
            }
            GenericSourceDeclarationKind::Alias(v) => (v.name_span(), v.source(), v.span().end, 26),
        };
        let raw_header = fragment.get(..header_end)?;
        charge(&mut limits.max_work, raw_header.len())?;
        let header = raw_header.trim_end();
        let name = decl.name();
        if header.is_empty()
            || header.len() > max_string
            || name.is_empty()
            || name.len() > max_string
        {
            return None;
        }
        charge(&mut limits.max_work, header.len().checked_add(name.len())?)?;
        let span = decl.span();
        let name_span = Span {
            start: span.start.checked_add(name_span.start)?,
            end: span.start.checked_add(name_span.end)?,
            ..name_span
        };
        charge(
            &mut limits.max_work,
            fragment.len().checked_add(name.len())?,
        )?;
        if span.start >= span.end
            || name_span.start < span.start
            || name_span.end > span.end
            || name_span.start >= name_span.end
            || program.source().get(name_span.start..name_span.end)? != name
            || program.source().get(span.start..span.end)? != fragment
        {
            return None;
        }
        let bound = escaped_len(name)?
            .checked_add(escaped_len(header)?)?
            .checked_add(160)?;
        if out.len().checked_add(bound)?.checked_add(256)? > max_bytes {
            return None;
        }
        charge(&mut limits.max_work, bound)?;
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":");
        push_string(&mut out, name);
        out.push_str(",\"signature\":");
        push_string(&mut out, header);
        let _ = write!(
            out,
            ",\"kind\":{kind},\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}}}",
            span.start, span.end, name_span.start, name_span.end
        );
    }
    out.push_str("],\"references\":[");
    let bodies = program
        .ir()
        .plans()
        .layouts()
        .literals()
        .concrete()
        .graph()
        .bodies();
    // Registry ordinals follow source type declarations, then injected definitions.
    let mut type_definitions = Vec::new();
    for (index, declaration) in program.declarations().iter().enumerate() {
        charge(&mut limits.max_work, 1)?;
        if !matches!(
            declaration.kind(),
            GenericSourceDeclarationKind::Function(_)
        ) {
            type_definitions.push(index);
        }
    }
    let mut references = 0usize;
    for body in bodies.bodies() {
        charge(&mut limits.max_work, 1)?;
        let origin = program
            .declarations()
            .get(program.function_declaration(body.signature().id())?)?
            .span();
        for expression in body.expressions() {
            charge(&mut limits.max_work, 1)?;
            let GenericCopyExpressionKind::Call {
                signature,
                callee_span,
                ..
            } = &expression.kind
            else {
                continue;
            };
            if references >= limits.max_references.min(8192) {
                return None;
            }
            let binding = signature.binding();
            if binding.caller() != body.signature().id() {
                return None;
            }
            let definition = program.function_declaration(binding.callee())?;
            let callee = bodies.signatures().signature(binding.callee())?;
            let name = callee.syntax().name();
            charge(&mut limits.max_work, name.len())?;
            let start = origin.start.checked_add(callee_span.start)?;
            let end = origin.start.checked_add(callee_span.end)?;
            if callee_span.start < expression.span.start
                || callee_span.end > expression.span.end
                || start >= end
                || end > origin.end
                || program.source().get(start..end)? != name
            {
                return None;
            }
            if out.len().checked_add(128)?.checked_add(256)? > max_bytes {
                return None;
            }
            charge(&mut limits.max_work, 128)?;
            if references > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"start\":{start},\"end\":{end},\"definition\":{definition}}}"
            );
            references += 1;
        }
    }
    // Function-header annotations are checked original syntax. Re-resolve their
    // heads under the sealed signature registry: transparent expansion otherwise
    // erases written alias identities. No body/type-text scan supplies bindings.
    for body in bodies.bodies() {
        charge(&mut limits.max_work, 1)?;
        let signature = body.signature();
        let origin = program
            .declarations()
            .get(program.function_declaration(signature.id())?)?
            .span();
        let syntax = signature.syntax();
        let registry = signature.types().environment().registry();
        let annotations = syntax
            .value_parameters()
            .iter()
            .map(|value| &value.ty)
            .chain(syntax.return_type());
        for annotation in annotations {
            charge(&mut limits.max_work, 1)?;
            let input = annotation.annotation.syntax();
            charge(&mut limits.max_work, input.source().len())?;
            let profile = GenericResolutionLimits {
                max_nodes: 4096.min(limits.max_work),
                max_edges: 4096.min(limits.max_work),
                max_work: limits.max_work,
            };
            // Bounds already checked by signature expansion, including aliases.
            let (nodes, work) = registry
                .resolve_nodes(input, syntax.parameters(), profile, false)
                .ok()?;
            charge(&mut limits.max_work, work)?;
            if nodes.len() != input.nodes().len() {
                return None;
            }
            for (node, resolved) in input.nodes().iter().zip(&nodes) {
                charge(&mut limits.max_work, 1)?;
                let ResolvedGenericTypeKind::Declared { declaration, .. } = resolved else {
                    continue;
                };
                let header = registry.header(*declaration)?;
                let Some(&definition) = type_definitions.get(declaration.ordinal()) else {
                    continue; // Compiler-injected types have no user-source destination.
                };
                let target = program.declarations().get(definition)?;
                charge(&mut limits.max_work, header.name.len())?;
                if target.name() != header.name.as_ref() {
                    return None;
                }
                if references >= limits.max_references.min(8192) {
                    return None;
                }
                let GenericTypeKind::Named { name, .. } = &node.kind else {
                    return None;
                };
                charge(&mut limits.max_work, name.len())?;
                if name != header.name.as_ref() {
                    return None;
                }
                let start = origin
                    .start
                    .checked_add(annotation.type_range.start)?
                    .checked_add(node.span.start)?;
                let end = start.checked_add(name.len())?;
                let annotation_end = origin.start.checked_add(annotation.type_range.end)?;
                if start >= end
                    || end > annotation_end
                    || annotation_end > origin.end
                    || program.source().get(start..end)? != name
                {
                    return None;
                }
                if out.len().checked_add(128)?.checked_add(256)? > max_bytes {
                    return None;
                }
                charge(&mut limits.max_work, 128)?;
                if references > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "{{\"start\":{start},\"end\":{end},\"definition\":{definition}}}"
                );
                references += 1;
            }
        }
    }
    out.push_str("],\"locals\":[");
    let mut locals = 0usize;
    for body in bodies.bodies() {
        charge(&mut limits.max_work, 1)?;
        let origin = program
            .declarations()
            .get(program.function_declaration(body.signature().id())?)?
            .span();
        for (index, binding) in body.bindings().iter().enumerate() {
            charge(&mut limits.max_work, 1)?;
            if locals >= limits.max_locals.min(8192)
                || binding.id.ordinal() != index
                || body.binding(binding.id)?.id != binding.id
            {
                return None;
            }
            let name = &binding.name;
            let (kind, prefix) = match binding.kind {
                GenericCopyBindingKind::Parameter(_) => ("parameter", "parameter "),
                GenericCopyBindingKind::MatchPayload { .. } => ("local", "match payload "),
                GenericCopyBindingKind::MutableLocal => ("local", "let mut "),
                GenericCopyBindingKind::Local => ("local", "let "),
            };
            if name.is_empty() || name.len().checked_add(prefix.len())? > max_string {
                return None;
            }
            charge(&mut limits.max_work, name.len())?;
            let start = origin.start.checked_add(binding.name_span.start)?;
            let end = origin.start.checked_add(binding.name_span.end)?;
            if start < origin.start
                || start >= end
                || end > origin.end
                || program.source().get(start..end)? != name.as_ref()
            {
                return None;
            }
            let remaining =
                max_string.checked_sub(prefix.len().checked_add(name.len())?.checked_add(2)?)?;
            let label = type_label(&binding.ty, remaining, &mut limits.max_work)?;
            let bound = escaped_len(name)?
                .checked_mul(2)?
                .checked_add(prefix.len())?
                .checked_add(escaped_len(&label)?)?
                .checked_add(162)?;
            if out.len().checked_add(bound)?.checked_add(256)? > max_bytes {
                return None;
            }
            charge(&mut limits.max_work, bound)?;
            let signature = format!("{prefix}{name}: {label}");
            if locals > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            push_string(&mut out, name);
            out.push_str(",\"signature\":");
            push_string(&mut out, &signature);
            let _=write!(out,",\"kind\":\"{kind}\",\"start\":{start},\"end\":{end},\"name_start\":{start},\"name_end\":{end}}}");
            locals += 1;
        }
    }
    out.push_str("],\"local_references\":[");
    let (mut base, mut uses) = (0usize, 0usize);
    for body in bodies.bodies() {
        charge(&mut limits.max_work, 1)?;
        let origin = program
            .declarations()
            .get(program.function_declaration(body.signature().id())?)?
            .span();
        for usage in body.name_uses() {
            charge(&mut limits.max_work, 1)?;
            if uses >= limits.max_local_references.min(8192) {
                return None;
            }
            let binding = body.binding(usage.binding)?;
            if binding.id != usage.binding {
                return None;
            }
            let definition = base.checked_add(usage.binding.ordinal())?;
            if definition >= locals {
                return None;
            }
            let name = &binding.name;
            charge(&mut limits.max_work, name.len())?;
            let start = origin.start.checked_add(usage.span.start)?;
            let end = origin.start.checked_add(usage.span.end)?;
            if start < origin.start
                || start >= end
                || end > origin.end
                || program.source().get(start..end)? != name.as_ref()
            {
                return None;
            }
            if out.len().checked_add(128)?.checked_add(256)? > max_bytes {
                return None;
            }
            charge(&mut limits.max_work, 128)?;
            if uses > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"start\":{start},\"end\":{end},\"definition\":{definition}}}"
            );
            uses += 1;
        }
        base = base.checked_add(body.bindings().len())?;
    }
    if base != locals {
        return None;
    }
    out.push_str("],\"truncated\":false}");
    if out.len() > max_bytes {
        return None;
    }
    Some(out)
}
