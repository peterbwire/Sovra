//! Approved primitive-only model field inspection, separate from executable records.
use super::syntax::{self, ApplicationDataSyntax, DataDeclarationKind};
use crate::compiler::{
    ast::Program,
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span},
    lexer::{Lexer, TokenKind},
    semantic::{collect_named_types_with_imports, Type},
};
use std::collections::{HashMap, HashSet};

/// Maximum cumulative fields in this optional source projection.
pub const MAX_MODEL_FIELDS: usize = 8192;

/// Original field syntax, owned by a model rather than an executable record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFieldSyntax {
    /// Original identifier.
    pub name: String,
    /// Exact field-name token.
    pub name_span: Span,
    /// Explicit simple annotation.
    pub annotation: String,
    /// Exact annotation token.
    pub annotation_span: Span,
    /// Complete original field, excluding separator trivia.
    pub span: Span,
}

/// Ordered fields belonging to one retained model declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFieldsSyntax {
    /// Index into `ModelPrimitiveFieldCheck::data.declarations` (always Model).
    pub declaration_index: usize,
    /// Original declaration envelope.
    pub declaration_span: Span,
    /// Source-order fields, including invalid declarations for diagnostic inspection.
    pub fields: Vec<ModelFieldSyntax>,
}

/// Primitive fact with explicit source declaration/field ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPrimitiveField {
    /// Index into `ModelPrimitiveFieldCheck::models`.
    pub model_index: usize,
    /// Index into that model's fields.
    pub field_index: usize,
    /// Canonical primitive type; never a model/record constructor registration.
    pub field_type: Type,
}

/// Partial source-only model validation, independent of other application checks.
#[derive(Debug, Clone)]
pub struct ModelPrimitiveFieldCheck {
    /// Original non-EOF tokens, including opaque/unvalidated source.
    pub token_count: usize,
    /// Header-only coverage remains unchanged, including opaque body gaps.
    pub data: ApplicationDataSyntax,
    /// Model-owned field syntax.
    pub models: Vec<ModelFieldsSyntax>,
    /// Complete primitive table, empty whenever semantic diagnostics exist.
    pub resolved: Vec<ModelPrimitiveField>,
    /// Bounded core alias and model duplicate/unknown-annotation errors.
    pub diagnostics: Diagnostics,
}

fn emit(diagnostics: &mut Diagnostics, code: &'static str, message: String, span: Span) {
    diagnostics.push(Diagnostic {
        source_file: None,
        severity: Severity::Error,
        code,
        message,
        span,
    });
}

fn check_diagnostics(diagnostics: &Diagnostics) -> Result<(), String> {
    if diagnostics.items.len() > super::MAX_ENTRY_PATH_DIAGNOSTICS
        || diagnostics
            .items
            .iter()
            .map(|d| d.message.len())
            .sum::<usize>()
            > super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES
        || diagnostics.items.iter().any(|d| d.code == "E3019")
    {
        return Err("model field diagnostic limit exceeded; projection unavailable".into());
    }
    Ok(())
}

/// Check explicitly annotated newline/comma-separated model fields against primitives
/// and file-local primitive aliases. Unsupported syntax/context or quota exhaustion
/// rejects the entire projection. Diagnostics prevent publication of type facts.
/// Imports, model values, defaults, optional/generic types and runtime semantics
/// require later checkers. No older checker/CLI behavior or policy grants change.
pub fn check_model_primitive_fields(source: &str) -> Result<ModelPrimitiveFieldCheck, String> {
    let data = syntax::parse_data_declarations(source)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "model field lexical errors")?;
    let mut depth = 0usize;
    let starts: HashSet<_> = data.declarations.iter().map(|d| d.span.start).collect();
    for token in &tokens {
        if depth == 0 {
            match &token.kind {
                TokenKind::Keyword("use" | "import") => {
                    return Err("model fields require local context; imports unavailable".into())
                }
                TokenKind::Identifier(name) if name == "import" => {
                    return Err("model fields require local context; imports unavailable".into())
                }
                TokenKind::Identifier(name)
                    if name == "model" && !starts.contains(&token.span.start) =>
                {
                    return Err("unsupported model declaration; projection unavailable".into())
                }
                _ => {}
            }
        }
        match token.kind {
            TokenKind::Punctuation('{') => depth += 1,
            TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    for declaration in &data.declarations {
        let position = tokens.partition_point(|token| token.span.start < declaration.span.start);
        if position > 0 && !matches!(tokens[position - 1].kind, TokenKind::Punctuation('}' | ';')) {
            return Err(
                "interrupted declaration before data header; projection unavailable".into(),
            );
        }
    }
    let types = syntax::parse(source)?;
    let program = Program {
        functions: Vec::new(),
        modules: Vec::new(),
        type_declarations: types.aliases.clone(),
        struct_declarations: types.records.clone(),
    };
    // Nominal sentinels permit core alias resolution to identify declared but
    // unsupported data types, without fabricating executable record AST nodes.
    let mut nominal = HashMap::new();
    let mut seen: HashSet<&str> = types
        .aliases
        .iter()
        .map(|d| d.name.as_str())
        .chain(types.records.iter().map(|d| d.name.as_str()))
        .collect();
    let mut diagnostics = Diagnostics::new();
    for declaration in &data.declarations {
        if !seen.insert(&declaration.name)
            || matches!(
                declaration.name.as_str(),
                "Unit" | "Bool" | "Int" | "Float" | "String"
            )
        {
            emit(
                &mut diagnostics,
                "E3008",
                diagnostic_message!("duplicate type `{}`", declaration.name),
                declaration.name_span,
            );
            check_diagnostics(&diagnostics)?;
        }
        nominal.insert(
            declaration.name.clone(),
            Type::Named(declaration.name.clone()),
        );
    }
    let known = collect_named_types_with_imports(&program, &nominal, &mut diagnostics);
    check_diagnostics(&diagnostics)?;
    let mut models = Vec::new();
    let mut resolved = Vec::new();
    let mut count = 0usize;
    for (declaration_index, declaration) in data.declarations.iter().enumerate() {
        if declaration.kind != DataDeclarationKind::Model {
            continue;
        }
        let mut fields = Vec::new();
        let mut names = HashSet::new();
        if let Some(body) = declaration.unparsed_body {
            let mut position = tokens.partition_point(|token| token.span.start < body.start);
            while position < tokens.len() && tokens[position].span.start < body.end {
                if count == MAX_MODEL_FIELDS {
                    return Err("model field count limit exceeded".into());
                }
                let name_token = &tokens[position];
                let TokenKind::Identifier(name) = &name_token.kind else {
                    return Err("model field requires identifier; projection unavailable".into());
                };
                if tokens.get(position + 1).map(|t| &t.kind) != Some(&TokenKind::Punctuation(':')) {
                    return Err("model field requires colon; projection unavailable".into());
                }
                let annotation_token = tokens
                    .get(position + 2)
                    .ok_or("model field requires annotation")?;
                let TokenKind::Identifier(annotation) = &annotation_token.kind else {
                    return Err(
                        "model field requires simple annotation; projection unavailable".into(),
                    );
                };
                if annotation_token.span.end > body.end {
                    return Err("model field requires annotation".into());
                }
                position += 3;
                let next = &tokens[position];
                if next.span.start < body.end {
                    if next.kind == TokenKind::Punctuation(',') {
                        position += 1;
                    } else if !matches!(next.kind, TokenKind::Identifier(_))
                        || next.span.line <= annotation_token.span.line
                    {
                        return Err("unsupported model field suffix or missing separator; projection unavailable".into());
                    }
                }
                if !names.insert(name.as_str()) {
                    emit(
                        &mut diagnostics,
                        "E3008",
                        diagnostic_message!(
                            "duplicate field `{name}` in model `{}`",
                            declaration.name
                        ),
                        name_token.span,
                    );
                }
                let field_type = Type::from_name_with_known(annotation, Some(&known));
                match field_type {
                    Type::Named(_) | Type::Array(_) => return Err("nonprimitive model field context unavailable".into()),
                    Type::Unknown => emit(&mut diagnostics, "E3017", diagnostic_message!("unknown type `{annotation}`; expected Unit, Bool, Int, Float, String or a declared alias"), annotation_token.span),
                    _ => resolved.push(ModelPrimitiveField { model_index: models.len(), field_index: fields.len(), field_type }),
                }
                check_diagnostics(&diagnostics)?;
                fields.push(ModelFieldSyntax {
                    name: name.clone(),
                    name_span: name_token.span,
                    annotation: annotation.clone(),
                    annotation_span: annotation_token.span,
                    span: Span {
                        end: annotation_token.span.end,
                        ..name_token.span
                    },
                });
                count += 1;
            }
        }
        models.push(ModelFieldsSyntax {
            declaration_index,
            declaration_span: declaration.span,
            fields,
        });
    }
    if !diagnostics.is_empty() {
        resolved.clear();
    }
    Ok(ModelPrimitiveFieldCheck {
        token_count: tokens.len().saturating_sub(1),
        data,
        models,
        resolved,
        diagnostics,
    })
}
