//! Projection only: retained scope identities select targets, tokens validate ranges.
use super::*;
use crate::compiler::{lexer::Token, semantic::Type};
fn type_label(mut ty: &Type) -> Option<String> {
    let mut depth = 0usize;
    while let Type::Array(item) = ty {
        depth = depth.checked_add(1)?;
        ty = item;
    }
    let leaf = match ty {
        Type::Unit => "Unit",
        Type::Bool => "Bool",
        Type::Int => "Int",
        Type::Float => "Float",
        Type::String => "String",
        Type::Named(name) if name.split("::").all(identifier) => name,
        Type::Unknown => return None,
        _ => return None,
    };
    if depth.checked_mul(7)?.checked_add(leaf.len())? > 4096 {
        return None;
    }
    Some("Array<".repeat(depth) + leaf + &">".repeat(depth))
}
fn name_token<'a>(
    tokens: &'a [Token],
    span: crate::compiler::diagnostics::Span,
    name: &str,
    local: bool,
) -> Option<&'a Token> {
    let mut index = tokens
        .binary_search_by_key(&span.start, |token| token.span.start)
        .ok()?;
    if local {
        if tokens.get(index)?.kind != TokenKind::Keyword("let") {
            return None;
        }
        index += 1;
        if tokens.get(index)?.kind == TokenKind::Keyword("mut") {
            index += 1;
        }
    }
    let token = tokens.get(index)?;
    if token.span.end > span.end
        || !matches!(&token.kind, TokenKind::Identifier(value) if value == name)
    {
        return None;
    }
    Some(token)
}
pub(super) fn append(
    program: &CheckedProgramSource,
    tokens: &[Token],
    output: &mut String,
    limit: usize,
) -> Option<()> {
    output.push_str(",\"locals\":[");
    let mut emitted = 0usize;
    for function in program.functions() {
        let scope = function.bindings();
        let range = function.typed().callable().function().syntax().span;
        for declaration in scope.declarations() {
            scope.declaration(declaration.id)?;
            let name = declaration.name.as_ref();
            if !identifier(name)
                || name.len() > 4096
                || declaration.span.start < range.start
                || declaration.span.end > range.end
            {
                return None;
            }
            let token = name_token(
                tokens,
                declaration.span,
                name,
                matches!(declaration.kind, ScopedDeclarationKind::Local { .. }),
            )?;
            let (kind, prefix) = match declaration.kind {
                ScopedDeclarationKind::Parameter => ("parameter", "parameter "),
                ScopedDeclarationKind::Local { mutable: true } => ("local", "let mut "),
                ScopedDeclarationKind::Local { mutable: false } => ("local", "let "),
                ScopedDeclarationKind::Payload { .. } => ("local", "match payload "),
            };
            // A missing label is explicitly unavailable, never Unknown/Any inference.
            let signature = match type_label(&declaration.ty) {
                Some(label) => format!("{prefix}{name}: {label}"),
                None => format!("{prefix}{name} (checked type unavailable)"),
            };
            if signature.len() > 4096 {
                return None;
            }
            if emitted > 0 {
                output.push(',');
            }
            output.push_str("{\"name\":");
            push_string(output, name);
            output.push_str(",\"kind\":");
            push_string(output, kind);
            output.push_str(",\"signature\":");
            push_string(output, &signature);
            write!(
                output,
                ",\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}}}",
                declaration.span.start, declaration.span.end, token.span.start, token.span.end
            )
            .ok()?;
            if output.len() > limit {
                return None;
            }
            emitted = emitted.checked_add(1)?;
        }
    }
    output.push_str("],\"local_references\":[");
    let mut offset = 0usize;
    let mut emitted = 0usize;
    for function in program.functions() {
        let scope = function.bindings();
        let range = function.typed().callable().function().syntax().span;
        for usage in scope.uses() {
            scope.usage(usage.id)?;
            let declaration = scope.declaration(usage.declaration)?;
            if usage.span.start < range.start || usage.span.end > range.end {
                return None;
            }
            let token = name_token(tokens, usage.span, &declaration.name, false)?;
            let definition = offset.checked_add(declaration.id.ordinal())?;
            if emitted > 0 {
                output.push(',');
            }
            write!(
                output,
                "{{\"start\":{},\"end\":{},\"definition\":{}}}",
                token.span.start, token.span.end, definition
            )
            .ok()?;
            if output.len() > limit {
                return None;
            }
            emitted = emitted.checked_add(1)?;
        }
        offset = offset.checked_add(scope.declarations().len())?;
    }
    output.push(']');
    (output.len() <= limit).then_some(())
}
