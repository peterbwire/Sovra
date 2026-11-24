//! Optional package function navigation from sealed original body/call identities.
use super::*;
use crate::compiler::check_report::push_string;
use std::{collections::BTreeMap, fmt::Write};
const UNAVAILABLE: &str = "{\"schema_version\":1,\"unavailable\":true}";
pub(super) fn package_symbols(
    program: &CompiledGenericPackage,
    limits: GenericSymbolLimits,
) -> String {
    project(program, limits).unwrap_or_else(|| UNAVAILABLE.into())
}
fn project(program: &CompiledGenericPackage, mut limits: GenericSymbolLimits) -> Option<String> {
    let bytes = limits.max_bytes.min(512 * 1024);
    let string = limits.max_string_bytes.min(4096);
    if limits.max_declarations == 0 || limits.max_work == 0 || bytes < UNAVAILABLE.len() {
        return None;
    }
    let types = program.bodies().types();
    let table = types.declarations();
    let inputs = table.inputs();
    let sources = inputs.sources();
    if sources.is_empty() || sources.len() > 32 || inputs.source_bytes() > 4 * 1024 * 1024 {
        return None;
    }
    fn charge(work: &mut usize, n: usize) -> Option<()> {
        *work = work.checked_sub(n)?;
        Some(())
    }
    fn room(out: &str, max: usize) -> Option<()> {
        if out.len() > max {
            None
        } else {
            Some(())
        }
    }
    let mut out = String::from("{\"schema_version\":1,\"unavailable\":false,\"files\":[");
    for (i, unit) in sources.iter().enumerate() {
        if unit.source_index() != i || unit.source().len() > 512 * 1024 {
            return None;
        }
        let path = inputs.inventory().sources.get(i)?.canonical_path.to_str()?;
        if path.len() > string {
            return None;
        }
        charge(
            &mut limits.max_work,
            unit.source().len().checked_add(path.len())?,
        )?;
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"path\":");
        push_string(&mut out, path);
        let hash = crate::native_build::sha256::digest(unit.source().as_bytes());
        write!(
            out,
            ",\"bytes\":{},\"sha256\":\"{hash}\"}}",
            unit.source().len()
        )
        .ok()?;
        room(&out, bytes)?;
    }
    out.push_str("],\"declarations\":[");
    let mut definitions = BTreeMap::new();
    for decl in table.declarations() {
        charge(&mut limits.max_work, 1)?;
        let (name, local_name, fragment, header_end, kind) = match decl.syntax().kind() {
            GenericSourceDeclarationKind::Function(v) => {
                (v.name(), v.name_span(), v.source(), v.body_span().start, 12)
            }
            GenericSourceDeclarationKind::Record(v) => (
                v.name(),
                v.name_span(),
                v.source(),
                v.parameter_range().end,
                23,
            ),
            GenericSourceDeclarationKind::Enum(v) => (
                v.name(),
                v.name_span(),
                v.source(),
                v.parameter_range().end,
                10,
            ),
            GenericSourceDeclarationKind::Alias(v) => {
                (v.name(), v.name_span(), v.source(), v.span().end, 26)
            }
        };
        if definitions.len() >= limits.max_declarations.min(8192) {
            return None;
        }
        let file = decl.source().ordinal();
        let source = sources.get(file)?.source();
        let origin = decl.syntax().span();
        let name_start = origin.start.checked_add(local_name.start)?;
        let name_end = origin.start.checked_add(local_name.end)?;
        let raw_header = fragment.get(..header_end)?;
        if raw_header.len() > string || name.is_empty() || name.len() > string {
            return None;
        }
        charge(
            &mut limits.max_work,
            raw_header
                .len()
                .checked_add(fragment.len())?
                .checked_add(name.len())?,
        )?;
        let header = raw_header.trim_end();
        if header.is_empty()
            || source.get(origin.start..origin.end)? != fragment
            || source.get(name_start..name_end)? != name
        {
            return None;
        }
        let index = definitions.len();
        if definitions.insert(decl.id(), index).is_some() {
            return None;
        }
        if index > 0 {
            out.push(',');
        }
        write!(out,"{{\"file\":{file},\"start\":{},\"end\":{},\"name_start\":{name_start},\"name_end\":{name_end},\"kind\":{kind},\"name\":",origin.start,origin.end).ok()?;
        push_string(&mut out, name);
        out.push_str(",\"signature\":");
        push_string(&mut out, header);
        out.push('}');
        room(&out, bytes)?;
    }
    let mut field_definitions = BTreeMap::new();
    for decl in table.declarations() {
        charge(&mut limits.max_work, 1)?;
        let GenericSourceDeclarationKind::Record(record) = decl.syntax().kind() else {
            continue;
        };
        let origin = decl.syntax().span();
        let file = decl.source().ordinal();
        let source = sources.get(file)?.source();
        for (ordinal, field) in record.fields().iter().enumerate() {
            let index = definitions.len().checked_add(field_definitions.len())?;
            if index >= limits.max_declarations.min(8192) {
                return None;
            }
            let start = origin.start.checked_add(field.span.start)?;
            let end = origin.start.checked_add(field.span.end)?;
            let name_start = origin.start.checked_add(field.name_span.start)?;
            let name_end = origin.start.checked_add(field.name_span.end)?;
            if start > name_start
                || name_start >= name_end
                || name_end > end
                || end > origin.end
                || source.get(name_start..name_end)? != field.name
            {
                return None;
            }
            let signature = source.get(start..end)?;
            if signature.len() > string || field.name.len() > string {
                return None;
            }
            charge(
                &mut limits.max_work,
                signature
                    .len()
                    .checked_add(field.name.len())?
                    .checked_add(1)?,
            )?;
            if field_definitions
                .insert((decl.id(), ordinal), index)
                .is_some()
            {
                return None;
            }
            out.push(',');
            write!(out,"{{\"file\":{file},\"start\":{start},\"end\":{end},\"name_start\":{name_start},\"name_end\":{name_end},\"kind\":8,\"name\":").ok()?;
            push_string(&mut out, &field.name);
            out.push_str(",\"signature\":");
            push_string(&mut out, signature);
            out.push('}');
            room(&out, bytes)?;
        }
    }
    out.push_str("],\"references\":[");
    let mut references = 0;
    for body in program.bodies().bodies().bodies() {
        charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
        let caller = table.declaration(types.declaration_for_signature(body.signature().id())?)?;
        let origin = caller.syntax().span();
        let file = caller.source().ordinal();
        let source = sources.get(file)?.source();
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
            if references >= limits.max_references.min(8192)
                || signature.binding().caller() != body.signature().id()
            {
                return None;
            }
            charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
            let target = types.declaration_for_signature(signature.binding().callee())?;
            let definition = *definitions.get(&target)?;
            let start = origin.start.checked_add(callee_span.start)?;
            let end = origin.start.checked_add(callee_span.end)?;
            if callee_span.start < expression.span.start
                || callee_span.end > expression.span.end
                || start >= end
                || end > origin.end
            {
                return None;
            }
            let written = source.get(start..end)?;
            charge(&mut limits.max_work, written.len())?;
            if references > 0 {
                out.push(',');
            }
            write!(
                out,
                "{{\"file\":{file},\"start\":{start},\"end\":{end},\"definition\":{definition}}}"
            )
            .ok()?;
            references += 1;
            room(&out, bytes)?;
        }
    }
    for body in program.bodies().bodies().bodies() {
        charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
        let caller = table.declaration(types.declaration_for_signature(body.signature().id())?)?;
        let origin = caller.syntax().span();
        let file = caller.source().ordinal();
        let source = sources.get(file)?.source();
        for expression in body.expressions() {
            charge(&mut limits.max_work, 1)?;
            let GenericCopyExpressionKind::Field {
                declaration,
                field_index,
                ..
            } = &expression.kind
            else {
                continue;
            };
            charge(
                &mut limits.max_work,
                definitions
                    .len()
                    .checked_add(field_definitions.len())?
                    .checked_add(1)?,
            )?;
            let Some(target) = types.declaration_for_type(*declaration) else {
                continue;
            };
            let definition = *field_definitions.get(&(target, *field_index))?;
            let GenericSourceDeclarationKind::Record(record) =
                table.declaration(target)?.syntax().kind()
            else {
                return None;
            };
            let field = record.fields().get(*field_index)?;
            let fragment = body
                .signature()
                .syntax()
                .source()
                .get(expression.span.start..expression.span.end)?;
            charge(
                &mut limits.max_work,
                fragment
                    .len()
                    .checked_mul(2)?
                    .checked_add(field.name.len())?,
            )?;
            let tokens =
                super::generic_types::type_tokens(fragment, 128 * 1024, 32768.min(limits.max_work))
                    .ok()?;
            charge(&mut limits.max_work, tokens.len())?;
            let mut end_token = tokens.len().checked_sub(1)?; // EOF
            while end_token > 0
                && tokens[end_token - 1].kind == crate::compiler::lexer::TokenKind::Punctuation(')')
            {
                end_token -= 1;
            }
            let name = tokens.get(end_token.checked_sub(1)?)?;
            if !matches!(&name.kind, crate::compiler::lexer::TokenKind::Identifier(actual) if actual == &field.name)
                || tokens.get(end_token.checked_sub(2)?)?.kind
                    != crate::compiler::lexer::TokenKind::Punctuation('.')
            {
                return None;
            }
            let start = origin
                .start
                .checked_add(expression.span.start)?
                .checked_add(name.span.start)?;
            let end = origin
                .start
                .checked_add(expression.span.start)?
                .checked_add(name.span.end)?;
            if start >= end
                || end > origin.end
                || source.get(start..end)? != field.name
                || references >= limits.max_references.min(8192)
            {
                return None;
            }
            if references > 0 {
                out.push(',');
            }
            write!(
                out,
                "{{\"file\":{file},\"start\":{start},\"end\":{end},\"definition\":{definition}}}"
            )
            .ok()?;
            references += 1;
            room(&out, bytes)?;
        }
    }
    // Constructor labels follow the checked source-order initializer identities.
    for body in program.bodies().bodies().bodies() {
        charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
        let caller = table.declaration(types.declaration_for_signature(body.signature().id())?)?;
        let origin = caller.syntax().span();
        let file = caller.source().ordinal();
        for expression in body.expressions() {
            charge(&mut limits.max_work, 1)?;
            let GenericCopyExpressionKind::Record {
                declaration,
                type_range,
                fields,
            } = &expression.kind
            else {
                continue;
            };
            charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
            let Some(target) = types.declaration_for_type(*declaration) else {
                continue;
            };
            let GenericSourceDeclarationKind::Record(record) =
                table.declaration(target)?.syntax().kind()
            else {
                return None;
            };
            let fragment = body
                .signature()
                .syntax()
                .source()
                .get(type_range.end..expression.span.end)?;
            charge(
                &mut limits.max_work,
                fragment.len().checked_mul(2)?.checked_add(1)?,
            )?;
            let tokens =
                super::generic_types::type_tokens(fragment, 128 * 1024, 32768.min(limits.max_work))
                    .ok()?;
            charge(&mut limits.max_work, tokens.len())?;
            if tokens.first()?.kind != crate::compiler::lexer::TokenKind::Punctuation('{') {
                return None;
            }
            let mut cursor = 1;
            for initializer in fields {
                charge(
                    &mut limits.max_work,
                    field_definitions.len().checked_add(1)?,
                )?;
                let field = record.fields().get(initializer.field_index)?;
                let definition = *field_definitions.get(&(target, initializer.field_index))?;
                let token = tokens.get(cursor)?;
                if !matches!(&token.kind,crate::compiler::lexer::TokenKind::Identifier(name) if name == &field.name)
                    || tokens.get(cursor + 1)?.kind
                        != crate::compiler::lexer::TokenKind::Punctuation(':')
                {
                    return None;
                }
                let value = body.expression(initializer.value)?;
                let local_start = type_range.end.checked_add(token.span.start)?;
                let local_end = type_range.end.checked_add(token.span.end)?;
                if local_start >= local_end
                    || local_end > value.span.start
                    || value.span.end > expression.span.end
                    || references >= limits.max_references.min(8192)
                {
                    return None;
                }
                let start = origin.start.checked_add(local_start)?;
                let end = origin.start.checked_add(local_end)?;
                if references > 0 {
                    out.push(',');
                }
                write!(out,"{{\"file\":{file},\"start\":{start},\"end\":{end},\"definition\":{definition}}}").ok()?;
                references += 1;
                room(&out, bytes)?;
                let value_end = value.span.end.checked_sub(type_range.end)?;
                cursor = tokens.partition_point(|t| t.span.start < value_end);
                if tokens.get(cursor)?.kind == crate::compiler::lexer::TokenKind::Punctuation(',') {
                    cursor += 1;
                }
            }
        }
    }
    // Only checked local annotation ranges are admitted; never scan body text for types.
    let mut local_annotations = BTreeMap::new();
    for body in program.bodies().bodies().bodies() {
        charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
        let owner = types.declaration_for_signature(body.signature().id())?;
        let mut annotations = Vec::new();
        for statement in body.statements() {
            charge(&mut limits.max_work, 1)?;
            if let GenericCopyStatement::Let {
                annotation: Some(annotation),
                ..
            } = statement
            {
                let range = annotation.range;
                let source = body
                    .signature()
                    .syntax()
                    .source()
                    .get(range.start..range.end)?;
                charge(
                    &mut limits.max_work,
                    source.len().checked_mul(2)?.checked_add(1)?,
                )?;
                let syntax = parse_generic_type(
                    source,
                    GenericTypeLimits {
                        max_tokens: 16384.min(limits.max_work),
                        max_nodes: 4096.min(limits.max_work),
                        max_edges: 4096.min(limits.max_work),
                        ..GenericTypeLimits::DEFAULT
                    },
                )
                .ok()?;
                charge(
                    &mut limits.max_work,
                    syntax.token_count().checked_add(syntax.nodes().len())?,
                )?;
                annotations.push((range, syntax));
            }
        }
        for expression in body.expressions() {
            charge(&mut limits.max_work, 1)?;
            let (range, constructor) = match &expression.kind {
                GenericCopyExpressionKind::Call { type_range, .. }
                    if type_range.start < type_range.end =>
                {
                    (*type_range, false)
                }
                GenericCopyExpressionKind::Record { type_range, .. }
                | GenericCopyExpressionKind::Enum { type_range, .. } => (*type_range, true),
                _ => continue,
            };
            let source = body
                .signature()
                .syntax()
                .source()
                .get(range.start..range.end)?;
            charge(
                &mut limits.max_work,
                source.len().checked_mul(2)?.checked_add(1)?,
            )?;
            let syntax_limits = GenericTypeLimits {
                max_tokens: 16384.min(limits.max_work),
                max_nodes: 4096.min(limits.max_work),
                max_edges: 4096.min(limits.max_work),
                ..GenericTypeLimits::DEFAULT
            };
            let tokens = super::generic_types::type_tokens(
                source,
                syntax_limits.max_source_bytes,
                syntax_limits.max_tokens,
            )
            .ok()?;
            charge(&mut limits.max_work, tokens.len())?;
            let list = if constructor {
                tokens.windows(2).position(|pair| {
                    pair[0].kind == crate::compiler::lexer::TokenKind::Operator("::")
                        && pair[1].kind == crate::compiler::lexer::TokenKind::Operator("<")
                })
            } else {
                Some(0)
            };
            // Compiler-owned non-generic standard constructors have no user type destination.
            let Some(list) = list else {
                continue;
            };
            let offset = tokens.get(list)?.span.start;
            let arguments = parse_generic_type_arguments(
                source.get(offset..)?,
                GenericTypeArgumentLimits {
                    types: syntax_limits,
                    max_arguments: 1024.min(limits.max_work),
                },
            )
            .ok()?;
            charge(
                &mut limits.max_work,
                arguments
                    .token_count()
                    .checked_add(arguments.nodes().len())?
                    .checked_add(arguments.name_bytes())?,
            )?;
            if constructor {
                let mut name = String::new();
                for (index, token) in tokens[..list].iter().enumerate() {
                    match (&token.kind, index % 2) {
                        (crate::compiler::lexer::TokenKind::Identifier(part), 0) => {
                            name.push_str(part)
                        }
                        (crate::compiler::lexer::TokenKind::Operator("::"), 1) => {
                            name.push_str("::")
                        }
                        _ => return None,
                    }
                }
                if name.is_empty() || list % 2 == 0 {
                    return None;
                }
                let origin = tokens[list].span;
                let span = crate::compiler::diagnostics::Span {
                    end: source.len(),
                    ..tokens.first()?.span
                };
                let syntax = arguments.constructor_type(
                    std::sync::Arc::from(source),
                    origin,
                    &name,
                    span,
                    list,
                );
                annotations.push((range, syntax));
            } else {
                for ordinal in 0..arguments.roots().len() {
                    charge(
                        &mut limits.max_work,
                        arguments
                            .nodes()
                            .len()
                            .checked_add(arguments.name_bytes())?,
                    )?;
                    annotations.push((range, arguments.argument_syntax(ordinal)?));
                }
            }
        }
        if local_annotations.insert(owner, annotations).is_some() {
            return None;
        }
    }
    // Resolve original annotations under their retained package parameter scopes.
    // This preserves written alias identities before transparent expansion.
    let registry = types.types().environment().registry();
    for decl in table.declarations() {
        charge(&mut limits.max_work, 1)?;
        let (parameters, mut annotations): (_, Vec<_>) = match decl.syntax().kind() {
            GenericSourceDeclarationKind::Function(v) => (
                v.parameters(),
                v.value_parameters()
                    .iter()
                    .map(|p| (p.ty.type_range, p.ty.annotation.syntax().clone()))
                    .chain(
                        v.return_type()
                            .map(|p| (p.type_range, p.annotation.syntax().clone())),
                    )
                    .collect(),
            ),
            GenericSourceDeclarationKind::Record(v) => (
                v.parameters(),
                v.fields()
                    .iter()
                    .map(|p| (p.type_range, p.annotation.syntax().clone()))
                    .collect(),
            ),
            GenericSourceDeclarationKind::Enum(v) => (
                v.parameters(),
                v.variants()
                    .iter()
                    .filter_map(|p| p.payload.as_ref())
                    .map(|p| (p.type_range, p.annotation.syntax().clone()))
                    .collect(),
            ),
            GenericSourceDeclarationKind::Alias(v) => (
                v.parameters(),
                vec![(v.target_range(), v.target().syntax().clone())],
            ),
        };
        let origin = decl.syntax().span();
        let file = decl.source().ordinal();
        let source = sources.get(file)?.source();
        if let Some(locals) = local_annotations.remove(&decl.id()) {
            annotations.extend(locals);
        }
        for (range, annotation) in annotations {
            let input = &annotation;
            charge(
                &mut limits.max_work,
                input.source().len().checked_mul(2)?.checked_add(1)?,
            )?;
            if range.start > range.end
                || range.end > origin.end.checked_sub(origin.start)?
                || source.get(
                    origin.start.checked_add(range.start)?..origin.start.checked_add(range.end)?,
                )? != input.source()
            {
                return None;
            }
            let (resolved, work) = registry
                .resolve_nodes(
                    input,
                    parameters,
                    GenericResolutionLimits {
                        max_nodes: 4096.min(limits.max_work),
                        max_edges: 4096.min(limits.max_work),
                        max_work: limits.max_work,
                    },
                    false,
                )
                .ok()?;
            charge(&mut limits.max_work, work)?;
            if resolved.len() != input.nodes().len() {
                return None;
            }
            let tokens = crate::compiler::lexer::Lexer::new()
                .tokenize_with_limits(
                    input.source(),
                    crate::compiler::resources::FrontendLimits {
                        max_source_bytes: 128 * 1024,
                        max_tokens: 32768,
                        max_diagnostics: 64,
                    },
                )
                .ok()?;
            charge(&mut limits.max_work, tokens.len())?;
            for (node, bound) in input.nodes().iter().zip(&resolved) {
                charge(&mut limits.max_work, 1)?;
                let ResolvedGenericTypeKind::Declared { declaration, .. } = bound else {
                    continue;
                };
                charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
                let Some(target) = types.declaration_for_type(*declaration) else {
                    continue;
                };
                let definition = *definitions.get(&target)?;
                let GenericTypeKind::Named { name, .. } = &node.kind else {
                    return None;
                };
                charge(&mut limits.max_work, name.len())?;
                let mut at = tokens
                    .binary_search_by_key(&node.span.start, |t| t.span.start)
                    .ok()?;
                let mut terminal = None;
                for (i, part) in name.split("::").enumerate() {
                    if i > 0 {
                        if tokens.get(at)?.kind != crate::compiler::lexer::TokenKind::Operator("::")
                        {
                            return None;
                        }
                        at += 1;
                    }
                    let token = tokens.get(at)?;
                    if !matches!(&token.kind,crate::compiler::lexer::TokenKind::Identifier(actual) if actual==part)
                        || token.span.end > node.span.end
                    {
                        return None;
                    }
                    terminal = Some(token.span);
                    at += 1;
                }
                let terminal = terminal?;
                let start = origin
                    .start
                    .checked_add(range.start)?
                    .checked_add(terminal.start)?;
                let end = origin
                    .start
                    .checked_add(range.start)?
                    .checked_add(terminal.end)?;
                if start >= end
                    || end > origin.start.checked_add(range.end)?
                    || range.end > origin.end.checked_sub(origin.start)?
                    || source.get(start..end)? != table.declaration(target)?.syntax().name()
                {
                    return None;
                }
                if references >= limits.max_references.min(8192) {
                    return None;
                }
                if references > 0 {
                    out.push(',');
                }
                write!(out,"{{\"file\":{file},\"start\":{start},\"end\":{end},\"definition\":{definition}}}").ok()?;
                references += 1;
                room(&out, bytes)?;
            }
        }
    }
    out.push_str("],\"locals\":[");
    let mut locals = 0usize;
    for body in program.bodies().bodies().bodies() {
        charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
        let caller_id = types.declaration_for_signature(body.signature().id())?;
        let caller = table.declaration(caller_id)?;
        let owner = *definitions.get(&caller_id)?;
        let origin = caller.syntax().span();
        let file = caller.source().ordinal();
        let source = sources.get(file)?.source();
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
            charge(&mut limits.max_work, name.len())?;
            let start = origin.start.checked_add(binding.name_span.start)?;
            let end = origin.start.checked_add(binding.name_span.end)?;
            if start >= end || end > origin.end || source.get(start..end)? != name.as_ref() {
                return None;
            }
            let available =
                string.checked_sub(prefix.len().checked_add(name.len())?.checked_add(2)?)?;
            let label =
                super::generic_symbols::type_label(&binding.ty, available, &mut limits.max_work)?;
            let signature = format!("{prefix}{name}: {label}");
            charge(&mut limits.max_work, signature.len().checked_add(160)?)?;
            if locals > 0 {
                out.push(',');
            }
            write!(out,"{{\"file\":{file},\"owner\":{owner},\"start\":{start},\"end\":{end},\"name_start\":{start},\"name_end\":{end},\"kind\":\"{kind}\",\"name\":").ok()?;
            push_string(&mut out, name);
            out.push_str(",\"signature\":");
            push_string(&mut out, &signature);
            out.push('}');
            locals += 1;
            room(&out, bytes)?;
        }
    }
    out.push_str("],\"local_references\":[");
    let (mut base, mut uses) = (0usize, 0usize);
    for body in program.bodies().bodies().bodies() {
        charge(&mut limits.max_work, definitions.len().checked_add(1)?)?;
        let caller_id = types.declaration_for_signature(body.signature().id())?;
        let caller = table.declaration(caller_id)?;
        let owner = *definitions.get(&caller_id)?;
        let origin = caller.syntax().span();
        let file = caller.source().ordinal();
        let source = sources.get(file)?.source();
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
            let start = origin.start.checked_add(usage.span.start)?;
            let end = origin.start.checked_add(usage.span.end)?;
            charge(&mut limits.max_work, binding.name.len().checked_add(128)?)?;
            if start >= end || end > origin.end || source.get(start..end)? != binding.name.as_ref()
            {
                return None;
            }
            if uses > 0 {
                out.push(',');
            }
            write!(out,"{{\"file\":{file},\"owner\":{owner},\"start\":{start},\"end\":{end},\"definition\":{definition}}}").ok()?;
            uses += 1;
            room(&out, bytes)?;
        }
        base = base.checked_add(body.bindings().len())?;
    }
    if base != locals {
        return None;
    }
    out.push_str("]}");
    room(&out, bytes)?;
    charge(&mut limits.max_work, out.len())?;
    Some(out)
}
