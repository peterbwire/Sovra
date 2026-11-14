//! Nominal IDs and match plans select targets; tokens validate original ranges.
use super::*;
use crate::compiler::{diagnostics::Span, lexer::Token};
pub(super) struct Projection {
    pub declarations: String,
    pub references: String,
}
fn checked(program: &CheckedProgramSource) -> Option<&CheckedEnumTypes> {
    Some(
        program
            .functions()
            .first()?
            .typed()
            .callable()
            .function()
            .environment(),
    )
}
// Explicit checked header annotations only. Transparent aliases retain their
// own written declaration identity and are deliberately not redirected here.
fn enum_annotations(function: &CheckedEnumFunction) -> impl Iterator<Item = (&str, Span, EnumId)> {
    let syntax = function.syntax();
    let env = function.environment();
    syntax
        .parameters
        .iter()
        .filter_map(|parameter| Some((parameter.type_name.as_deref()?, parameter.type_span?)))
        .chain(syntax.return_type.as_deref().zip(syntax.return_type_span))
        .filter_map(move |(mut name, span)| {
            while let Some(inner) = name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                name = inner;
            }
            let id = env.resolve_enum(name)?;
            let declaration = env.declarations().get(id.ordinal())?;
            let direct = name == declaration.name
                || env.module().is_some_and(|module| {
                    declaration
                        .name
                        .strip_prefix(module)
                        .and_then(|s| s.strip_prefix("::"))
                        == Some(name)
                });
            direct.then_some((name, span, id))
        })
}
pub(super) fn count(program: &CheckedProgramSource) -> Option<usize> {
    let env = checked(program)?;
    let mut count = env.declarations().len().checked_add(
        program
            .declarations()
            .iter()
            .filter(|d| {
                matches!(
                    d.kind,
                    EntryDeclarationKind::Module
                        | EntryDeclarationKind::Record
                        | EntryDeclarationKind::Alias
                )
            })
            .count(),
    )?;
    for declaration in env.declarations() {
        count = count.checked_add(declaration.variants.len())?;
    }
    for function in program.functions() {
        let f = function.typed().callable();
        count = count.checked_add(enum_annotations(f.function()).count())?;
        count = count.checked_add(
            f.evidence()
                .calls()
                .iter()
                .filter(|call| matches!(call.target, CallableTarget::Constructor { .. }))
                .count(),
        )?;
        for plan in f.function().matches() {
            count = count.checked_add(plan.variant_order().len())?;
        }
    }
    Some(count)
}
fn payload(env: &CheckedEnumTypes, mut id: EnumTypeId) -> Option<String> {
    let mut depth = 0usize;
    let leaf = loop {
        match env.payload_type(id)? {
            EnumPayloadType::Array(inner) => {
                depth = depth.checked_add(1)?;
                id = *inner;
                if depth.checked_mul(2)? > 4096 {
                    return None;
                }
            }
            EnumPayloadType::Unit => break "Unit",
            EnumPayloadType::Bool => break "Bool",
            EnumPayloadType::Int => break "Int",
            EnumPayloadType::Float => break "Float",
            EnumPayloadType::String => break "String",
            EnumPayloadType::Record(name) => break name,
            EnumPayloadType::Enum(id) => {
                let d = env.declarations().get(id.ordinal())?;
                if d.id != *id {
                    return None;
                }
                break &d.name;
            }
        }
    };
    if !leaf.split("::").all(identifier) || depth.checked_mul(2)?.checked_add(leaf.len())? > 4096 {
        return None;
    }
    Some("[".repeat(depth) + leaf + &"]".repeat(depth))
}
fn declaration(
    out: &mut String,
    name: &str,
    signature: &str,
    kind: u8,
    range: Span,
    selection: Span,
    limit: usize,
) -> Option<()> {
    if name.len() > 4096
        || signature.len() > 4096
        || !name.split("::").all(identifier)
        || range.start > selection.start
        || selection.end > range.end
    {
        return None;
    }
    if !out.is_empty() {
        out.push(',');
    }
    out.push_str("{\"name\":");
    push_string(out, name);
    out.push_str(",\"signature\":");
    push_string(out, signature);
    write!(
        out,
        ",\"kind\":{kind},\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}}}",
        range.start, range.end, selection.start, selection.end
    )
    .ok()?;
    (out.len() <= limit).then_some(())
}
fn original_name(tokens: &[Token], span: Span, expected: &str) -> Option<()> {
    let index = tokens
        .binary_search_by_key(&span.start, |token| token.span.start)
        .ok()?;
    let t = tokens.get(index)?;
    (t.span == span && matches!(&t.kind, TokenKind::Identifier(name) if name == expected))
        .then_some(())
}
fn reference(
    out: &mut String,
    tokens: &[Token],
    span: Span,
    variant: &EnumVariantFact,
    definition: usize,
    pattern: bool,
    limit: usize,
) -> Option<()> {
    let start = tokens
        .binary_search_by_key(&span.start, |token| token.span.start)
        .ok()?;
    let end = if pattern {
        tokens[start..]
            .iter()
            .position(|token| token.kind == TokenKind::Punctuation('('))
            .map(|n| start + n)?
    } else {
        tokens.partition_point(|token| token.span.start < span.end)
    };
    let token = tokens.get(end.checked_sub(1)?)?;
    if token.span.start < span.start
        || token.span.end > span.end
        || !matches!(&token.kind, TokenKind::Identifier(name) if name == &variant.name)
    {
        return None;
    }
    if !out.is_empty() {
        out.push(',');
    }
    write!(
        out,
        "{{\"start\":{},\"end\":{},\"definition\":{definition}}}",
        token.span.start, token.span.end
    )
    .ok()?;
    (out.len() <= limit).then_some(())
}
pub(super) fn project(
    program: &CheckedProgramSource,
    tokens: &[Token],
    limit: usize,
) -> Option<Projection> {
    let env = checked(program)?;
    let mut out = Projection {
        declarations: String::new(),
        references: String::new(),
    };
    let mut indices = Vec::new();
    let mut next = program.functions().len();
    let mut ranges = program
        .declarations()
        .iter()
        .filter(|d| d.kind == EntryDeclarationKind::Enum);
    for (ordinal, d) in env.declarations().iter().enumerate() {
        if d.id.ordinal() != ordinal {
            return None;
        }
        original_name(tokens, d.span, d.name.rsplit("::").next()?)?;
        let range = ranges.next()?.span;
        declaration(
            &mut out.declarations,
            &d.name,
            &format!("enum {}", d.name),
            10,
            range,
            d.span,
            limit,
        )?;
        indices.push(next.checked_add(1)?);
        next = next.checked_add(1)?.checked_add(d.variants.len())?;
        for v in &d.variants {
            original_name(tokens, v.span, &v.name)?;
            let name = format!("{}::{}", d.name, v.name);
            let value = match v.payload {
                Some(id) => payload(env, id)?,
                None => String::new(),
            };
            declaration(
                &mut out.declarations,
                &name,
                &format!("{name}({value}) -> {}", d.name),
                22,
                v.span,
                v.span,
                limit,
            )?;
        }
    }
    if ranges.next().is_some() {
        return None;
    }
    // Append containers after nominal declarations so retained variant indices
    // continue to address their original declarations.
    for module in program
        .declarations()
        .iter()
        .filter(|d| d.kind == EntryDeclarationKind::Module)
    {
        let mut head = tokens
            .binary_search_by_key(&module.span.start, |token| token.span.start)
            .ok()?;
        if tokens.get(head)?.kind == TokenKind::Keyword("export") {
            head += 1;
        }
        if tokens.get(head)?.kind != TokenKind::Keyword("mod") {
            return None;
        }
        let token = tokens.get(head.checked_add(1)?)?;
        let TokenKind::Identifier(name) = &token.kind else {
            return None;
        };
        declaration(
            &mut out.declarations,
            name,
            &format!("mod {name}"),
            2,
            module.span,
            token.span,
            limit,
        )?;
    }
    // Source-order declaration ownership is retained by the checked program.
    // One-level modules precede their members; their closing span ends the scope.
    let mut module: Option<(&str, usize)> = None;
    for item in program.declarations() {
        if module.is_some_and(|(_, end)| item.span.start >= end) {
            module = None;
        }
        if !matches!(
            item.kind,
            EntryDeclarationKind::Module
                | EntryDeclarationKind::Record
                | EntryDeclarationKind::Alias
        ) {
            continue;
        }
        let mut head = tokens
            .binary_search_by_key(&item.span.start, |token| token.span.start)
            .ok()?;
        if tokens.get(head)?.kind == TokenKind::Keyword("export") {
            head += 1;
        }
        let (keyword, kind) = match item.kind {
            EntryDeclarationKind::Module => ("mod", 2),
            EntryDeclarationKind::Record => ("struct", 23),
            EntryDeclarationKind::Alias => ("type", 26),
            _ => return None,
        };
        if tokens.get(head)?.kind != TokenKind::Keyword(keyword) {
            return None;
        }
        let token = tokens.get(head.checked_add(1)?)?;
        let TokenKind::Identifier(written) = &token.kind else {
            return None;
        };
        if item.kind == EntryDeclarationKind::Module {
            module = Some((written, item.span.end));
            continue;
        }
        let prefix = module.map_or("", |(name, _)| name);
        let length = prefix
            .len()
            .checked_add(written.len())?
            .checked_add(if module.is_some() { 2 } else { 0 })?;
        if length > 4096 {
            return None;
        }
        let name = if module.is_some() {
            format!("{prefix}::{written}")
        } else {
            written.clone()
        };
        // Written alias headers show the author's spelling; they do not imply
        // resolved type-use targets or an inferred public API signature.
        let signature = if item.kind == EntryDeclarationKind::Alias {
            let text = program
                .source()
                .get(tokens[head].span.start..item.span.end)?;
            if text.len() > 4096 {
                return None;
            }
            text.to_owned()
        } else {
            if length.checked_add(7)? > 4096 {
                return None;
            }
            format!("struct {name}")
        };
        declaration(
            &mut out.declarations,
            &name,
            &signature,
            kind,
            item.span,
            token.span,
            limit,
        )?;
    }
    for function in program.functions() {
        let f = function.typed().callable();
        for (name, span, id) in enum_annotations(f.function()) {
            let end = tokens.partition_point(|token| token.span.end <= span.end);
            let token = tokens
                .get(..end)?
                .iter()
                .rev()
                .take_while(|token| token.span.start >= span.start)
                .find(|token| matches!(token.kind, TokenKind::Identifier(_)))?;
            original_name(tokens, token.span, name.rsplit("::").next()?)?;
            let definition = indices.get(id.ordinal())?.checked_sub(1)?;
            if !out.references.is_empty() {
                out.references.push(',');
            }
            write!(
                out.references,
                "{{\"start\":{},\"end\":{},\"definition\":{definition}}}",
                token.span.start, token.span.end
            )
            .ok()?;
            if out.references.len() > limit {
                return None;
            }
        }
        for call in f.evidence().calls() {
            if let CallableTarget::Constructor { enum_id, variant } = call.target {
                let d = env.declarations().get(enum_id.ordinal())?;
                if d.id != enum_id {
                    return None;
                }
                reference(
                    &mut out.references,
                    tokens,
                    call.span,
                    d.variants.get(variant)?,
                    indices[enum_id.ordinal()].checked_add(variant)?,
                    false,
                    limit,
                )?;
            }
        }
        let checked = f.function();
        let mut plans = checked.matches().iter();
        let mut pending: Vec<_> = checked.syntax().body.iter().rev().collect();
        while let Some(statement) = pending.pop() {
            let (header, bodies) = match statement {
                EnumFunctionStatement::Match(header) => (Some(header), None),
                EnumFunctionStatement::RecursiveMatch(s) => (Some(&s.header), Some(&s.bodies)),
                EnumFunctionStatement::Control(s) => (None, Some(&s.bodies)),
                EnumFunctionStatement::Core(_) => (None, None),
            };
            if let Some(header) = header {
                let plan = plans.next()?;
                let id = plan.enum_id();
                let d = env.declarations().get(id.ordinal())?;
                if d.id != id || plan.variant_order().len() != header.arms.len() {
                    return None;
                }
                for (arm, &variant) in header.arms.iter().zip(plan.variant_order()) {
                    let v = d.variants.get(variant)?;
                    if arm.variant_name != v.name {
                        return None;
                    }
                    reference(
                        &mut out.references,
                        tokens,
                        arm.span,
                        v,
                        indices[id.ordinal()].checked_add(variant)?,
                        true,
                        limit,
                    )?;
                }
            }
            if let Some(bodies) = bodies {
                for body in bodies.iter().rev() {
                    pending.extend(body.iter().rev());
                }
            }
        }
        if plans.next().is_some() {
            return None;
        }
    }
    Some(out)
}
