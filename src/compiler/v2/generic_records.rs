//! Bounded generic record attachment with one parameter owner across all fields.
use super::{
    generic_aliases::{translate, translate_error},
    generic_types::type_tokens,
    parse_generic_parameters, parse_generic_type, GenericParameterId, GenericParameterLimits,
    GenericParameterUses, GenericParameters, GenericTypeError, GenericTypeLimits,
};
use crate::compiler::{diagnostics::Span, lexer::TokenKind};
use std::{collections::BTreeSet, sync::Arc};
/// Complete record and cumulative annotation admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericRecordLimits {
    /// Original full input bytes before lexing.
    pub max_source_bytes: usize,
    /// Full-source lexer tokens including EOF.
    pub max_tokens: usize,
    /// Total full source, parameter fragment and all field-type fragments.
    pub max_retained_source_bytes: usize,
    /// Aggregate record/parameter/field/type copied names.
    pub max_name_bytes: usize,
    /// Field records and duplicate-name index entries.
    pub max_fields: usize,
    /// Aggregate type arena nodes across all fields.
    pub max_type_nodes: usize,
    /// Aggregate type child edges across all fields.
    pub max_type_edges: usize,
    /// Aggregate parameter-use records across all fields.
    pub max_parameter_uses: usize,
    /// Independent parameter-list admission.
    pub parameters: GenericParameterLimits,
    /// Independent per-field annotation admission, additionally capped by remaining totals.
    pub annotations: GenericTypeLimits,
}
impl GenericRecordLimits {
    /// Finite opt-in syntax profile; source execution remains strict.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 128 * 1024,
        max_tokens: 32_768,
        max_retained_source_bytes: 256 * 1024,
        max_name_bytes: 48 * 1024,
        max_fields: 1024,
        max_type_nodes: 4096,
        max_type_edges: 4096,
        max_parameter_uses: 4096,
        parameters: GenericParameterLimits::DEFAULT,
        annotations: GenericTypeLimits::DEFAULT,
    };
}
/// One immutable original field and its attached annotation parameter uses.
#[derive(Debug, Clone)]
pub struct GenericRecordField {
    /// Written field name.
    pub name: String,
    /// Full original field range, excluding comma and surrounding trivia.
    pub span: Span,
    /// Original field-name range.
    pub name_span: Span,
    /// Full-source range of the retained annotation fragment.
    pub type_range: Span,
    /// Annotation-local nodes/uses retaining the record's shared parameter owner.
    pub annotation: GenericParameterUses,
}
/// Complete generic record syntax, not a resolved/checked record type.
#[derive(Debug, Clone)]
pub struct GenericRecordSyntax {
    standard_error: bool,
    standard_map_entry: bool,
    standard_result: Option<&'static str>,
    source: Arc<str>,
    name: String,
    span: Span,
    name_span: Span,
    is_exported: bool,
    parameter_range: Span,
    parameters: GenericParameters,
    fields: Arc<[GenericRecordField]>,
    retained_source_bytes: usize,
    name_bytes: usize,
    type_nodes: usize,
    type_edges: usize,
    parameter_uses: usize,
}
impl GenericRecordSyntax {
    /// Whether this is the sealed compiler-owned std::MapEntry<K,V> schema.
    pub fn is_standard_map_entry(&self) -> bool {
        self.standard_map_entry
    }
    /// Whether this is the immutable compiler-owned std::Error definition.
    pub fn is_standard_error(&self) -> bool {
        self.standard_error
    }
    /// Whether this is an immutable compiler-owned legacy standard result schema.
    pub fn is_standard_result(&self) -> bool {
        self.standard_result.is_some()
    }
    /// Canonical registered name; user declarations retain their written name.
    pub fn qualified_name(&self) -> &str {
        if self.standard_map_entry {
            "std::MapEntry"
        } else if self.standard_error {
            "std::Error"
        } else {
            self.standard_result.unwrap_or(&self.name)
        }
    }
    /// Original full declaration including surrounding trivia.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Written record name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Full original declaration range.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Original record-name selection range.
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
    /// All fields in source order, with the same retained parameter owner.
    pub fn fields(&self) -> &[GenericRecordField] {
        &self.fields
    }
    /// Parameter declaration range in the full original source; foreign IDs refuse.
    pub fn parameter_span(&self, id: GenericParameterId) -> Option<Span> {
        Some(translate(
            self.parameters.parameter(id)?.span,
            self.parameter_range,
        ))
    }
    /// Field annotation node range in the full original source; bad indices refuse.
    pub fn field_type_node_span(&self, field: usize, node: usize) -> Option<Span> {
        let field = self.fields.get(field)?;
        Some(translate(
            field.annotation.syntax().nodes().get(node)?.span,
            field.type_range,
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
    /// Aggregate field annotation nodes.
    pub fn type_nodes(&self) -> usize {
        self.type_nodes
    }
    /// Aggregate field annotation edges.
    pub fn type_edges(&self) -> usize {
        self.type_edges
    }
    /// Aggregate annotation parameter-use records.
    pub fn parameter_uses(&self) -> usize {
        self.parameter_uses
    }
}
pub(super) fn charge(
    left: &mut usize,
    amount: usize,
    dimension: &'static str,
) -> Result<(), GenericTypeError> {
    *left = left
        .checked_sub(amount)
        .ok_or(GenericTypeError::Limit(dimension))?;
    Ok(())
}
/// Parse one root `[export] struct Name<T>{field:Type,...}` with owned parameters.
/// Symbolic/concrete checking, field access, layout and execution remain later stages.
pub fn parse_generic_record(
    source: &str,
    limits: GenericRecordLimits,
) -> Result<GenericRecordSyntax, GenericTypeError> {
    parse_record_mode(source, limits, false)
}
fn parse_record_mode(
    source: &str,
    limits: GenericRecordLimits,
    plain: bool,
) -> Result<GenericRecordSyntax, GenericTypeError> {
    let tokens = type_tokens(source, limits.max_source_bytes, limits.max_tokens)?;
    let error = |at: usize, message| GenericTypeError::Syntax {
        span: tokens[at.min(tokens.len() - 1)].span,
        message,
    };
    let is_exported = tokens[0].kind == TokenKind::Keyword("export");
    let head = usize::from(is_exported);
    if tokens.get(head).map(|t| &t.kind) != Some(&TokenKind::Keyword("struct")) {
        return Err(error(head, "expected generic record declaration"));
    }
    let name_token = tokens
        .get(head + 1)
        .ok_or_else(|| error(head, "expected record name"))?;
    let TokenKind::Identifier(name) = &name_token.kind else {
        return Err(error(head + 1, "expected record name"));
    };
    let open = tokens
        .get(head + 2)
        .ok_or_else(|| error(head + 1, "expected generic parameters"))?;
    let (close, parameter_range) = if plain && open.kind == TokenKind::Punctuation('{') {
        (
            head + 1,
            Span {
                end: open.span.start,
                ..open.span
            },
        )
    } else {
        if open.kind != TokenKind::Operator("<") {
            return Err(error(head + 2, "expected generic parameters"));
        }
        let close = (head + 3..tokens.len())
            .find(|&i| tokens[i].kind == TokenKind::Operator(">"))
            .ok_or_else(|| error(tokens.len() - 1, "expected closing parameter bracket"))?;
        (
            close,
            Span {
                end: tokens[close].span.end,
                ..open.span
            },
        )
    };
    if tokens.get(close + 1).map(|t| &t.kind) != Some(&TokenKind::Punctuation('{')) {
        return Err(error(close + 1, "expected record body"));
    }
    let mut source_left = limits.max_retained_source_bytes;
    charge(
        &mut source_left,
        source.len(),
        "record retained source bytes",
    )?;
    let parameter_source = &source[parameter_range.start..parameter_range.end];
    charge(
        &mut source_left,
        parameter_source.len(),
        "record retained source bytes",
    )?;
    let mut names_left = limits.max_name_bytes;
    charge(&mut names_left, name.len(), "record names")?;
    let mut parameter_limits = limits.parameters;
    parameter_limits.max_name_bytes = parameter_limits.max_name_bytes.min(names_left);
    let parameters = if parameter_source.is_empty() {
        GenericParameters::empty()?
    } else {
        parse_generic_parameters(parameter_source, parameter_limits)
            .map_err(|e| translate_error(e, parameter_range))?
    };
    charge(&mut names_left, parameters.name_bytes(), "record names")?;
    let mut nodes_left = limits.max_type_nodes;
    let mut edges_left = limits.max_type_edges;
    let mut uses_left = limits.max_parameter_uses;
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();
    let mut cursor = close + 2;
    loop {
        if tokens[cursor].kind == TokenKind::Punctuation('}') {
            break;
        }
        let token = &tokens[cursor];
        let TokenKind::Identifier(field_name) = &token.kind else {
            return Err(error(cursor, "expected record field name or closing brace"));
        };
        if fields.len() >= limits.max_fields {
            return Err(GenericTypeError::Limit("record fields"));
        }
        charge(&mut names_left, field_name.len(), "record names")?;
        if !names.insert(field_name.as_str()) {
            return Err(error(cursor, "duplicate record field"));
        }
        let name_span = token.span;
        cursor += 1;
        if tokens[cursor].kind != TokenKind::Punctuation(':') {
            return Err(error(cursor, "expected field annotation"));
        }
        cursor += 1;
        let start = cursor;
        let mut angles = 0usize;
        let mut arrays = 0usize;
        loop {
            match tokens[cursor].kind {
                TokenKind::Punctuation(',' | '}') if angles == 0 && arrays == 0 => break,
                TokenKind::Eof => {
                    return Err(error(cursor, "expected field type or closing record brace"))
                }
                TokenKind::Operator("<") => {
                    angles = angles
                        .checked_add(1)
                        .ok_or(GenericTypeError::Limit("field nesting"))?
                }
                TokenKind::Operator(">") => {
                    angles = angles
                        .checked_sub(1)
                        .ok_or_else(|| error(cursor, "unexpected closing type argument bracket"))?
                }
                TokenKind::Punctuation('[') => {
                    arrays = arrays
                        .checked_add(1)
                        .ok_or(GenericTypeError::Limit("field nesting"))?
                }
                TokenKind::Punctuation(']') => {
                    arrays = arrays
                        .checked_sub(1)
                        .ok_or_else(|| error(cursor, "unexpected closing array bracket"))?
                }
                _ => {}
            }
            cursor += 1;
        }
        if start == cursor {
            return Err(error(cursor, "expected field type"));
        }
        let type_range = Span {
            end: tokens[cursor - 1].span.end,
            ..tokens[start].span
        };
        let type_source = &source[type_range.start..type_range.end];
        charge(
            &mut source_left,
            type_source.len(),
            "record retained source bytes",
        )?;
        let mut annotation_limits = limits.annotations;
        annotation_limits.max_nodes = annotation_limits.max_nodes.min(nodes_left);
        annotation_limits.max_edges = annotation_limits.max_edges.min(edges_left);
        annotation_limits.max_name_bytes = annotation_limits.max_name_bytes.min(names_left);
        let syntax = parse_generic_type(type_source, annotation_limits)
            .map_err(|e| translate_error(e, type_range))?;
        charge(&mut nodes_left, syntax.nodes().len(), "record type nodes")?;
        charge(&mut edges_left, syntax.edges(), "record type edges")?;
        charge(&mut names_left, syntax.name_bytes(), "record names")?;
        let annotation = parameters
            .bind_type(&syntax, uses_left)
            .map_err(|e| translate_error(e, type_range))?;
        charge(
            &mut uses_left,
            annotation.uses().len(),
            "record parameter uses",
        )?;
        fields.push(GenericRecordField {
            name: field_name.clone(),
            span: Span {
                end: type_range.end,
                ..name_span
            },
            name_span,
            type_range,
            annotation,
        });
        if tokens[cursor].kind == TokenKind::Punctuation('}') {
            break;
        }
        cursor += 1;
    }
    let span = Span {
        end: tokens[cursor].span.end,
        ..tokens[0].span
    };
    cursor += 1;
    if tokens[cursor].kind != TokenKind::Eof {
        return Err(error(cursor, "expected end of record declaration"));
    }
    Ok(GenericRecordSyntax {
        standard_error: false,
        standard_map_entry: false,
        standard_result: None,
        source: Arc::from(source),
        name: name.clone(),
        span,
        name_span: name_token.span,
        is_exported,
        parameter_range,
        parameters,
        fields: fields.into(),
        retained_source_bytes: limits.max_retained_source_bytes - source_left,
        name_bytes: limits.max_name_bytes - names_left,
        type_nodes: limits.max_type_nodes - nodes_left,
        type_edges: limits.max_type_edges - edges_left,
        parameter_uses: limits.max_parameter_uses - uses_left,
    })
}

pub(super) fn standard_map_entry_record() -> Result<GenericRecordSyntax, GenericTypeError> {
    let mut record = parse_generic_record(
        "struct MapEntry<K:std::Key,V>{key:K,value:V}",
        GenericRecordLimits::DEFAULT,
    )?;
    record.standard_map_entry = true;
    Ok(record)
}
pub(super) fn standard_error_record() -> Result<GenericRecordSyntax, GenericTypeError> {
    let mut record = parse_record_mode(
        "struct Error{code:String,message:String,offset:std::Option<Int>}",
        GenericRecordLimits::DEFAULT,
        true,
    )?;
    record.standard_error = true;
    Ok(record)
}

pub(super) fn standard_result_records() -> Result<Vec<GenericRecordSyntax>, GenericTypeError> {
    let sources = [
        ("std::InputLine", "struct InputLine{eof:Bool,text:String}"),
        (
            "std::TextRead",
            "struct TextRead{ok:Bool,text:String,error:String}",
        ),
        ("std::TextWrite", "struct TextWrite{ok:Bool,error:String}"),
        (
            "std::SplitOnce",
            "struct SplitOnce{found:Bool,before:String,after:String}",
        ),
        ("std::ParsedInt", "struct ParsedInt{ok:Bool,value:Int}"),
    ];
    sources
        .into_iter()
        .map(|(name, source)| {
            let mut record = parse_record_mode(source, GenericRecordLimits::DEFAULT, true)?;
            record.standard_result = Some(name);
            Ok(record)
        })
        .collect()
}

#[cfg(test)]
mod standard_result_tests {
    use super::*;
    #[test]
    fn legacy_schemas_match_the_existing_core_standard_record_contract() {
        let mut core = std::collections::HashMap::new();
        crate::compiler::semantic::add_standard_record_fields(&mut core);
        for record in standard_result_records().unwrap() {
            let fields = &core[record.qualified_name()];
            assert_eq!(fields.len(), record.fields().len());
            assert!(record.is_standard_result());
            assert!(!record.is_standard_error());
            for field in record.fields() {
                let super::super::GenericTypeKind::Named { name, arguments } =
                    &field.annotation.syntax().root().kind
                else {
                    panic!("primitive field")
                };
                assert!(arguments.is_empty());
                let expected = match fields[&field.name] {
                    crate::compiler::semantic::Type::Bool => "Bool",
                    crate::compiler::semantic::Type::Int => "Int",
                    crate::compiler::semantic::Type::String => "String",
                    _ => panic!("unexpected legacy field"),
                };
                assert_eq!(name, expected);
            }
        }
    }
}
