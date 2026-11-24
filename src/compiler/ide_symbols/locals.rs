//! Bounded projection of compiler-owned local identities and checked types.

use std::{collections::HashMap, fmt::Write as _};

use super::super::check_report::push_string;
use super::super::checked::{
    CheckedUnit, DeclarationKind, NodeId, RecordedType, SemanticEvidence, TypeId, UseKind,
    UseTarget,
};
use super::super::lexer::{Token, TokenKind};

pub(super) struct Evidence {
    pub declarations: String,
    pub references: String,
    pub truncated: bool,
}

// Formatting only: Unknown at any array depth is unavailable, never Any.
fn type_label(evidence: &SemanticEvidence, mut id: TypeId) -> (Option<String>, bool) {
    let mut depth = 0;
    let leaf = loop {
        match &evidence.types()[id.ordinal()] {
            RecordedType::Array(element) => {
                depth += 1;
                id = *element;
            }
            RecordedType::Unknown => return (None, false),
            RecordedType::Unit => break "Unit",
            RecordedType::Bool => break "Bool",
            RecordedType::Int => break "Int",
            RecordedType::Float => break "Float",
            RecordedType::String => break "String",
            RecordedType::Named(name) => break name.as_ref(),
        }
    };
    if depth * 7 + leaf.len() > 4096 {
        return (None, true);
    }
    (
        Some("Array<".repeat(depth) + leaf + &">".repeat(depth)),
        false,
    )
}

// Preserve source annotation spelling without resolving it again.
fn annotation(tokens: &[Token], name: usize) -> Result<Option<String>, ()> {
    if !matches!(
        tokens.get(name + 1).map(|t| &t.kind),
        Some(TokenKind::Punctuation(':'))
    ) {
        return Ok(None);
    }
    let mut cursor = name + 2;
    let mut text = String::new();
    let mut arrays = 0;
    while matches!(
        tokens.get(cursor).map(|token| &token.kind),
        Some(TokenKind::Punctuation('['))
    ) {
        arrays += 1;
        if arrays >= crate::compiler::limits::MAX_STRUCTURAL_DEPTH {
            return Err(());
        }
        text.push('[');
        cursor += 1;
    }
    loop {
        let Some(Token {
            kind: TokenKind::Identifier(part),
            ..
        }) = tokens.get(cursor)
        else {
            return Err(());
        };
        if text.len() + part.len() > 4096 {
            return Err(());
        }
        text.push_str(part);
        if !matches!(
            tokens.get(cursor + 1).map(|t| &t.kind),
            Some(TokenKind::Operator("::"))
        ) {
            break;
        }
        text.push_str("::");
        cursor += 2;
    }
    for offset in 1..=arrays {
        if !matches!(
            tokens.get(cursor + offset).map(|token| &token.kind),
            Some(TokenKind::Punctuation(']'))
        ) {
            return Err(());
        }
        text.push(']');
    }
    if text.len() > 4096 {
        return Err(());
    }
    Ok(Some(text))
}

pub(super) fn collect(unit: &CheckedUnit, tokens: &[Token], limit: usize) -> Evidence {
    let mut output = Evidence {
        declarations: String::new(),
        references: String::new(),
        truncated: false,
    };
    let evidence = unit
        .semantic_evidence()
        .expect("IDE snapshot has checked types");
    let flow = evidence.flow().expect("IDE snapshot has resolved uses");
    let facts: HashMap<_, _> = evidence
        .facts()
        .iter()
        .map(|fact| (fact.node, fact.ty))
        .collect();
    let mut emitted: HashMap<NodeId, usize> = HashMap::new();
    let limit = limit.saturating_sub(64);
    for (visited, declaration) in flow
        .declarations()
        .iter()
        .filter(|d| d.kind != DeclarationKind::Function)
        .enumerate()
    {
        if visited >= super::MAX_VISITS {
            output.truncated = true;
            break;
        }
        let span = unit.nodes()[declaration.node.ordinal()].span;
        let parameter = declaration.kind == DeclarationKind::Parameter;
        let mutable = matches!(declaration.kind, DeclarationKind::Local { mutable: true });
        let Ok(mut index) = tokens.binary_search_by_key(&span.start, |token| token.span.start)
        else {
            output.truncated = true;
            continue;
        };
        if !parameter {
            index += 1;
            if matches!(
                tokens.get(index).map(|t| &t.kind),
                Some(TokenKind::Keyword("mut"))
            ) {
                index += 1;
            }
        }
        let Some(token) = tokens.get(index) else {
            output.truncated = true;
            continue;
        };
        if !matches!(&token.kind, TokenKind::Identifier(name) if name == declaration.name.as_ref())
        {
            output.truncated = true;
            continue;
        }
        let Ok(annotation) = annotation(tokens, index) else {
            output.truncated = true;
            continue;
        };
        let inferred = !parameter && annotation.is_none();
        let label = if inferred {
            let (label, truncated) = facts
                .get(&declaration.node)
                .map_or((None, false), |id| type_label(evidence, *id));
            output.truncated |= truncated;
            label
        } else {
            None
        };
        let name = declaration.name.as_ref();
        if name.len() > 4096 {
            output.truncated = true;
            continue;
        }
        let prefix = if parameter {
            "parameter "
        } else if mutable {
            "let mut "
        } else {
            "let "
        };
        let signature = if let Some(annotation) = &annotation {
            format!("{prefix}{name}: {annotation}")
        } else if let Some(label) = &label {
            format!("{prefix}{name}: {label} (inferred)")
        } else {
            format!("{prefix}{name} (inferred type unavailable)")
        };
        if signature.len() > 4096 {
            output.truncated = true;
            continue;
        }
        let checkpoint = output.declarations.len();
        if !emitted.is_empty() {
            output.declarations.push(',');
        }
        let text = &mut output.declarations;
        text.push_str("{\"name\":");
        push_string(text, name);
        text.push_str(",\"signature\":");
        push_string(text, &signature);
        let _ = write!(
            text,
            ",\"kind\":\"{}\",\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}",
            if parameter { "parameter" } else { "local" },
            span.start,
            span.end,
            token.span.start,
            token.span.end
        );
        if inferred {
            text.push_str(",\"inferred_type\":");
            if let Some(label) = label {
                push_string(text, &label);
            } else {
                text.push_str("null");
            }
        }
        text.push('}');
        if output.declarations.len() > limit {
            output.declarations.truncate(checkpoint);
            output.truncated = true;
            break;
        }
        emitted.insert(declaration.node, emitted.len());
    }
    let mut count = 0;
    for (visited, usage) in flow
        .uses()
        .iter()
        .filter(|u| matches!(u.kind, UseKind::Read | UseKind::Write))
        .enumerate()
    {
        if visited >= super::MAX_VISITS {
            output.truncated = true;
            break;
        }
        let UseTarget::Declaration(target) = usage.target else {
            continue;
        };
        let Some(target) = emitted.get(&target) else {
            output.truncated = true;
            continue;
        };
        let span = unit.nodes()[usage.node.ordinal()].span;
        let checkpoint = output.references.len();
        if count > 0 {
            output.references.push(',');
        }
        let _ = write!(
            output.references,
            "{{\"start\":{},\"end\":{},\"definition\":{target}}}",
            span.start, span.end
        );
        if output.declarations.len() + output.references.len() > limit {
            output.references.truncate(checkpoint);
            output.truncated = true;
            break;
        }
        count += 1;
    }
    output
}
