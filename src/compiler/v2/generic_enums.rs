//! Bounded generic enum attachment with one parameter owner across all variants.
use super::{
    generic_aliases::{translate, translate_error},
    generic_records::charge,
    generic_types::type_tokens,
    parse_generic_parameters, parse_generic_type, GenericParameterId, GenericParameterLimits,
    GenericParameterUses, GenericParameters, GenericTypeError, GenericTypeLimits,
};
use crate::compiler::{diagnostics::Span, lexer::TokenKind};
use std::{collections::BTreeSet, sync::Arc};
/// Complete enum and cumulative annotation admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEnumLimits {
    /// Original full input bytes before lexing.
    pub max_source_bytes: usize,
    /// Full-source lexer tokens including EOF.
    pub max_tokens: usize,
    /// Total full source, parameter fragment and all variant-type fragments.
    pub max_retained_source_bytes: usize,
    /// Aggregate enum/parameter/variant/type copied names.
    pub max_name_bytes: usize,
    /// Variant records and duplicate-name index entries.
    pub max_variants: usize,
    /// Aggregate type arena nodes across all variants.
    pub max_type_nodes: usize,
    /// Aggregate type child edges across all variants.
    pub max_type_edges: usize,
    /// Aggregate parameter-use records across all variants.
    pub max_parameter_uses: usize,
    /// Independent parameter-list admission.
    pub parameters: GenericParameterLimits,
    /// Independent per-variant annotation admission, additionally capped by remaining totals.
    pub annotations: GenericTypeLimits,
}
impl GenericEnumLimits {
    /// Finite opt-in syntax profile; source execution remains strict.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 128 * 1024,
        max_tokens: 32_768,
        max_retained_source_bytes: 256 * 1024,
        max_name_bytes: 48 * 1024,
        max_variants: 1024,
        max_type_nodes: 4096,
        max_type_edges: 4096,
        max_parameter_uses: 4096,
        parameters: GenericParameterLimits::DEFAULT,
        annotations: GenericTypeLimits::DEFAULT,
    };
}
/// Single admitted payload annotation with its original full-source range.
#[derive(Debug, Clone)]
pub struct GenericEnumPayload {
    /// Full-source range of the payload fragment, excluding parentheses.
    pub type_range: Span,
    /// Payload-local annotation and parameter-use owner.
    pub annotation: GenericParameterUses,
}
/// One nullary or single-payload variant in original source order.
#[derive(Debug, Clone)]
pub struct GenericEnumVariant {
    /// Written variant name.
    pub name: String,
    /// Full original variant range, including payload parentheses when present.
    pub span: Span,
    /// Original short variant-name range.
    pub name_span: Span,
    /// None is a nullary declaration; Some owns exactly one type annotation.
    pub payload: Option<GenericEnumPayload>,
}
/// Compiler-owned standard enum identity, unavailable to source declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericStandardEnum {
    /// Qualified optional snapshot value.
    Option,
    /// Qualified success/error snapshot value.
    Result,
    /// Recursive explicit JSON value, available only in the data profile.
    JsonValue,
}
/// Complete generic enum syntax, not a resolved/checked enum type.
#[derive(Debug, Clone)]
pub struct GenericEnumSyntax {
    standard: Option<GenericStandardEnum>,
    source: Arc<str>,
    name: String,
    span: Span,
    name_span: Span,
    is_exported: bool,
    parameter_range: Span,
    parameters: GenericParameters,
    variants: Arc<[GenericEnumVariant]>,
    retained_source_bytes: usize,
    name_bytes: usize,
    type_nodes: usize,
    type_edges: usize,
    parameter_uses: usize,
}
impl GenericEnumSyntax {
    /// Compiler-owned standard identity, absent for ordinary parsed declarations.
    pub fn standard_identity(&self) -> Option<GenericStandardEnum> {
        self.standard
    }
    /// Registered canonical name; ordinary declarations retain their written name.
    pub fn qualified_name(&self) -> &str {
        match self.standard {
            Some(GenericStandardEnum::Option) => "std::Option",
            Some(GenericStandardEnum::Result) => "std::Result",
            Some(GenericStandardEnum::JsonValue) => "std::JsonValue",
            None => &self.name,
        }
    }
    /// Original full declaration including surrounding trivia.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Written enum name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Full original declaration range.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Original enum-name selection range.
    pub fn name_span(&self) -> Span {
        self.name_span
    }
    /// Written export intent; not checked visibility authority.
    pub fn is_exported(&self) -> bool {
        self.is_exported
    }
    /// Declaration-local parameter owner; its spans are fragment-local.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    /// Exact full-source parameter-fragment range.
    pub fn parameter_range(&self) -> Span {
        self.parameter_range
    }
    /// All variants in source order, with the same retained parameter owner.
    pub fn variants(&self) -> &[GenericEnumVariant] {
        &self.variants
    }
    /// Parameter declaration range in the full original source; foreign IDs refuse.
    pub fn parameter_span(&self, id: GenericParameterId) -> Option<Span> {
        Some(translate(
            self.parameters.parameter(id)?.span,
            self.parameter_range,
        ))
    }
    /// Variant annotation node range in the full original source; bad indices refuse.
    pub fn payload_type_node_span(&self, variant: usize, node: usize) -> Option<Span> {
        let payload = self.variants.get(variant)?.payload.as_ref()?;
        Some(translate(
            payload.annotation.syntax().nodes().get(node)?.span,
            payload.type_range,
        ))
    }
    /// Total retained full/fragment source bytes.
    pub fn retained_source_bytes(&self) -> usize {
        self.retained_source_bytes
    }
    /// Aggregate copied names.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
    /// Aggregate variant annotation nodes.
    pub fn type_nodes(&self) -> usize {
        self.type_nodes
    }
    /// Aggregate variant annotation edges.
    pub fn type_edges(&self) -> usize {
        self.type_edges
    }
    /// Aggregate annotation parameter-use records.
    pub fn parameter_uses(&self) -> usize {
        self.parameter_uses
    }
}
/// Parse one root `[export] enum Name<T>{Variant,Payload(Type),...}`.
/// Name/arity/bound checking, constructor generation, matching and specialization
/// remain later semantic stages. Generic executable programs remain unsupported.
pub fn parse_generic_enum(
    source: &str,
    limits: GenericEnumLimits,
) -> Result<GenericEnumSyntax, GenericTypeError> {
    parse_enum_mode(source, limits, false)
}
fn parse_enum_mode(
    source: &str,
    limits: GenericEnumLimits,
    allow_empty: bool,
) -> Result<GenericEnumSyntax, GenericTypeError> {
    let tokens = type_tokens(source, limits.max_source_bytes, limits.max_tokens)?;
    let error = |at: usize, message| GenericTypeError::Syntax {
        span: tokens[at.min(tokens.len() - 1)].span,
        message,
    };
    let is_exported = tokens[0].kind == TokenKind::Keyword("export");
    let head = usize::from(is_exported);
    if !matches!(tokens.get(head).map(|t|&t.kind),Some(TokenKind::Identifier(name)) if name=="enum")
    {
        return Err(error(head, "expected generic enum declaration"));
    }
    let name_token = tokens
        .get(head + 1)
        .ok_or_else(|| error(head, "expected enum name"))?;
    let TokenKind::Identifier(name) = &name_token.kind else {
        return Err(error(head + 1, "expected enum name"));
    };
    let open = tokens
        .get(head + 2)
        .ok_or_else(|| error(head + 1, "expected generic parameters"))?;
    let (parameter_range, close) = if allow_empty && open.kind == TokenKind::Punctuation('{') {
        let span = Span {
            start: name_token.span.end,
            end: name_token.span.end,
            line: name_token.span.line,
            column: name_token.span.column + name.len(),
        };
        (span, head + 1)
    } else {
        if open.kind != TokenKind::Operator("<") {
            return Err(error(head + 2, "expected generic parameters"));
        }
        let close = (head + 3..tokens.len())
            .find(|&i| tokens[i].kind == TokenKind::Operator(">"))
            .ok_or_else(|| error(tokens.len() - 1, "expected closing parameter bracket"))?;
        (
            Span {
                end: tokens[close].span.end,
                ..open.span
            },
            close,
        )
    };
    if tokens.get(close + 1).map(|t| &t.kind) != Some(&TokenKind::Punctuation('{')) {
        return Err(error(close + 1, "expected enum body"));
    }
    let mut source_left = limits.max_retained_source_bytes;
    charge(&mut source_left, source.len(), "enum retained source bytes")?;
    let parameter_source = &source[parameter_range.start..parameter_range.end];
    charge(
        &mut source_left,
        parameter_source.len(),
        "enum retained source bytes",
    )?;
    let mut names_left = limits.max_name_bytes;
    charge(&mut names_left, name.len(), "enum names")?;
    let mut parameter_limits = limits.parameters;
    parameter_limits.max_name_bytes = parameter_limits.max_name_bytes.min(names_left);
    let parameters = if parameter_source.is_empty() {
        GenericParameters::empty()?
    } else {
        parse_generic_parameters(parameter_source, parameter_limits)
            .map_err(|e| translate_error(e, parameter_range))?
    };
    charge(&mut names_left, parameters.name_bytes(), "enum names")?;
    let mut nodes_left = limits.max_type_nodes;
    let mut edges_left = limits.max_type_edges;
    let mut uses_left = limits.max_parameter_uses;
    let mut variants = Vec::new();
    let mut names = BTreeSet::new();
    let mut cursor = close + 2;
    loop {
        if tokens[cursor].kind == TokenKind::Punctuation('}') {
            break;
        }
        let token = &tokens[cursor];
        let TokenKind::Identifier(variant_name) = &token.kind else {
            return Err(error(cursor, "expected enum variant name or closing brace"));
        };
        if variants.len() >= limits.max_variants {
            return Err(GenericTypeError::Limit("enum variants"));
        }
        charge(&mut names_left, variant_name.len(), "enum names")?;
        if !names.insert(variant_name.as_str()) {
            return Err(error(cursor, "duplicate enum variant"));
        }
        let name_span = token.span;
        let mut span = name_span;
        cursor += 1;
        let payload = if tokens[cursor].kind == TokenKind::Punctuation('(') {
            cursor += 1;
            let start = cursor;
            // Type annotations have no parenthesized subgrammar. The first
            // closing parenthesis ends the payload; the type parser rejects
            // tuple/named/function/extra-parenthesis forms in the fragment.
            while tokens[cursor].kind != TokenKind::Punctuation(')') {
                if tokens[cursor].kind == TokenKind::Eof {
                    return Err(error(cursor, "expected closing payload parenthesis"));
                }
                cursor += 1;
            }
            if start == cursor {
                return Err(error(cursor, "expected payload type"));
            }
            let type_range = Span {
                end: tokens[cursor - 1].span.end,
                ..tokens[start].span
            };
            let type_source = &source[type_range.start..type_range.end];
            charge(
                &mut source_left,
                type_source.len(),
                "enum retained source bytes",
            )?;
            let mut annotation_limits = limits.annotations;
            annotation_limits.max_nodes = annotation_limits.max_nodes.min(nodes_left);
            annotation_limits.max_edges = annotation_limits.max_edges.min(edges_left);
            annotation_limits.max_name_bytes = annotation_limits.max_name_bytes.min(names_left);
            let syntax = parse_generic_type(type_source, annotation_limits)
                .map_err(|e| translate_error(e, type_range))?;
            charge(&mut nodes_left, syntax.nodes().len(), "enum type nodes")?;
            charge(&mut edges_left, syntax.edges(), "enum type edges")?;
            charge(&mut names_left, syntax.name_bytes(), "enum names")?;
            let annotation = parameters
                .bind_type(&syntax, uses_left)
                .map_err(|e| translate_error(e, type_range))?;
            charge(
                &mut uses_left,
                annotation.uses().len(),
                "enum parameter uses",
            )?;
            span.end = tokens[cursor].span.end;
            cursor += 1;
            Some(GenericEnumPayload {
                type_range,
                annotation,
            })
        } else {
            None
        };
        variants.push(GenericEnumVariant {
            name: variant_name.clone(),
            span,
            name_span,
            payload,
        });
        if tokens[cursor].kind == TokenKind::Punctuation('}') {
            break;
        }
        if tokens[cursor].kind != TokenKind::Punctuation(',') {
            return Err(error(cursor, "expected comma or closing enum brace"));
        }
        cursor += 1;
    }
    let span = Span {
        end: tokens[cursor].span.end,
        ..tokens[0].span
    };
    cursor += 1;
    if tokens[cursor].kind != TokenKind::Eof {
        return Err(error(cursor, "expected end of enum declaration"));
    }
    Ok(GenericEnumSyntax {
        standard: None,
        source: Arc::from(source),
        name: name.clone(),
        span,
        name_span: name_token.span,
        is_exported,
        parameter_range,
        parameters,
        variants: variants.into(),
        retained_source_bytes: limits.max_retained_source_bytes - source_left,
        name_bytes: limits.max_name_bytes - names_left,
        type_nodes: limits.max_type_nodes - nodes_left,
        type_edges: limits.max_type_edges - edges_left,
        parameter_uses: limits.max_parameter_uses - uses_left,
    })
}

// Definitions are compiler-owned, fixed source templates; their fragment spans
// never identify a user file. All subsequent registry/type budgets still apply.
pub(super) fn standard_generic_enums() -> Result<[GenericEnumSyntax; 2], GenericTypeError> {
    let mut option =
        parse_generic_enum("enum Option<T>{Some(T),None}", GenericEnumLimits::DEFAULT)?;
    let mut result =
        parse_generic_enum("enum Result<T,E>{Ok(T),Err(E)}", GenericEnumLimits::DEFAULT)?;
    option.standard = Some(GenericStandardEnum::Option);
    result.standard = Some(GenericStandardEnum::Result);
    Ok([option, result])
}

pub(super) fn standard_json_enum() -> Result<GenericEnumSyntax, GenericTypeError> {
    let mut value=parse_enum_mode("enum JsonValue{Null,Bool(Bool),Int(Int),Float(Float),String(String),Array([std::JsonValue]),Object(std::Map<String,std::JsonValue>)}",GenericEnumLimits::DEFAULT,true)?;
    value.standard = Some(GenericStandardEnum::JsonValue);
    Ok(value)
}
