//! Generic function header ownership with an explicitly opaque, bounded body range.
use super::{
    generic_aliases::{translate, translate_error},
    generic_records::charge,
    generic_types::type_tokens,
    parse_generic_parameters, parse_generic_type, GenericParameterId, GenericParameterLimits,
    GenericParameterUses, GenericParameters, GenericTypeError, GenericTypeLimits,
};
use crate::compiler::{
    diagnostics::Span,
    lexer::{Token, TokenKind},
};
use std::{collections::BTreeSet, sync::Arc};
/// Whole function and cumulative header/body admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericFunctionLimits {
    /// Original whole-source bytes.
    pub max_source_bytes: usize,
    /// Original lexer tokens including EOF.
    pub max_tokens: usize,
    /// Full source plus retained parameter/annotation fragments.
    pub max_retained_source_bytes: usize,
    /// Aggregate function/type-parameter/value-parameter/annotation copied names.
    pub max_name_bytes: usize,
    /// Value parameter records and duplicate-name index entries.
    pub max_value_parameters: usize,
    /// Aggregate parameter/return annotation nodes.
    pub max_type_nodes: usize,
    /// Aggregate annotation child edges.
    pub max_type_edges: usize,
    /// Aggregate annotation type-parameter use records.
    pub max_parameter_uses: usize,
    /// Opaque body tokens, including opening/closing braces.
    pub max_body_tokens: usize,
    /// Body delimiter nesting, including outer braces; fixed ceiling 128.
    pub max_body_depth: usize,
    /// Independent type-parameter-list admission.
    pub parameters: GenericParameterLimits,
    /// Independent per-annotation admission, additionally capped by remaining totals.
    pub annotations: GenericTypeLimits,
}
impl GenericFunctionLimits {
    /// Finite opt-in source-ownership profile; no executable generic checking.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 128 * 1024,
        max_tokens: 32_768,
        max_retained_source_bytes: 256 * 1024,
        max_name_bytes: 48 * 1024,
        max_value_parameters: 128,
        max_type_nodes: 4096,
        max_type_edges: 4096,
        max_parameter_uses: 4096,
        max_body_tokens: 16_384,
        max_body_depth: 128,
        parameters: GenericParameterLimits::DEFAULT,
        annotations: GenericTypeLimits::DEFAULT,
    };
}
/// An explicit annotation with fragment-local nodes and original full-source range.
#[derive(Debug, Clone)]
pub struct GenericFunctionAnnotation {
    /// Full-source range of the exact annotation fragment.
    pub type_range: Span,
    /// Annotation and uses sharing this function's type-parameter owner.
    pub annotation: GenericParameterUses,
}
/// One explicitly annotated value parameter, not a checked runtime binding.
#[derive(Debug, Clone)]
pub struct GenericFunctionParameter {
    /// Written value parameter name.
    pub name: String,
    /// Full parameter range in the original function source.
    pub span: Span,
    /// Original name selection range.
    pub name_span: Span,
    /// Required explicit annotation.
    pub ty: GenericFunctionAnnotation,
}
/// Complete source-owned function header and opaque body. Not a checked function.
#[derive(Debug, Clone)]
pub struct GenericFunctionSyntax {
    source: Arc<str>,
    name: String,
    span: Span,
    name_span: Span,
    is_exported: bool,
    parameter_range: Span,
    parameters: GenericParameters,
    values: Arc<[GenericFunctionParameter]>,
    return_type: Option<GenericFunctionAnnotation>,
    body_span: Span,
    body_tokens: usize,
    retained_source_bytes: usize,
    name_bytes: usize,
    type_nodes: usize,
    type_edges: usize,
    parameter_uses: usize,
}
impl GenericFunctionSyntax {
    pub(super) fn source_owner(&self) -> Arc<str> {
        self.source.clone()
    }
    /// Original complete input including trivia.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Written function name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Full original declaration range.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Original name selection range.
    pub fn name_span(&self) -> Span {
        self.name_span
    }
    /// Written export intent, not checked visibility authority.
    pub fn is_exported(&self) -> bool {
        self.is_exported
    }
    /// Function-local type-parameter owner; internal spans are fragment-local.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    /// Full-source type-parameter fragment, or empty range at a nongeneric root value-list opener.
    pub fn parameter_range(&self) -> Span {
        self.parameter_range
    }
    /// Explicit value parameters in source order.
    pub fn value_parameters(&self) -> &[GenericFunctionParameter] {
        &self.values
    }
    /// Written return annotation; None retains the existing implicit Unit convention.
    pub fn return_type(&self) -> Option<&GenericFunctionAnnotation> {
        self.return_type.as_ref()
    }
    /// Original body range, including braces. Its contents are not parsed or checked.
    pub fn body_span(&self) -> Span {
        self.body_span
    }
    /// Borrowed original body including braces, without another source copy.
    pub fn body_source(&self) -> &str {
        &self.source[self.body_span.start..self.body_span.end]
    }
    /// Admitted body token count, including outer braces.
    pub fn body_tokens(&self) -> usize {
        self.body_tokens
    }
    /// Original full-source type-parameter declaration range; foreign IDs refuse.
    pub fn parameter_span(&self, id: GenericParameterId) -> Option<Span> {
        Some(translate(
            self.parameters.parameter(id)?.span,
            self.parameter_range,
        ))
    }
    /// Full-source node range in a value parameter annotation; bad indices refuse.
    pub fn value_type_node_span(&self, value: usize, node: usize) -> Option<Span> {
        let ty = &self.values.get(value)?.ty;
        Some(translate(
            ty.annotation.syntax().nodes().get(node)?.span,
            ty.type_range,
        ))
    }
    /// Full-source node range in the explicit return annotation.
    pub fn return_type_node_span(&self, node: usize) -> Option<Span> {
        let ty = self.return_type.as_ref()?;
        Some(translate(
            ty.annotation.syntax().nodes().get(node)?.span,
            ty.type_range,
        ))
    }
    /// Total retained full/fragment source bytes.
    pub fn retained_source_bytes(&self) -> usize {
        self.retained_source_bytes
    }
    /// Aggregate retained copied names.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
    /// Aggregate annotation nodes.
    pub fn type_nodes(&self) -> usize {
        self.type_nodes
    }
    /// Aggregate annotation edges.
    pub fn type_edges(&self) -> usize {
        self.type_edges
    }
    /// Aggregate annotation type-parameter uses.
    pub fn parameter_uses(&self) -> usize {
        self.parameter_uses
    }
}
struct Admission {
    source: usize,
    names: usize,
    nodes: usize,
    edges: usize,
    uses: usize,
}
impl Admission {
    fn annotation(
        &mut self,
        source: &str,
        range: Span,
        parameters: &GenericParameters,
        mut limits: GenericTypeLimits,
    ) -> Result<GenericFunctionAnnotation, GenericTypeError> {
        let fragment = &source[range.start..range.end];
        charge(
            &mut self.source,
            fragment.len(),
            "function retained source bytes",
        )?;
        limits.max_nodes = limits.max_nodes.min(self.nodes);
        limits.max_edges = limits.max_edges.min(self.edges);
        limits.max_name_bytes = limits.max_name_bytes.min(self.names);
        let syntax = parse_generic_type(fragment, limits).map_err(|e| translate_error(e, range))?;
        charge(&mut self.names, syntax.name_bytes(), "function names")?;
        charge(&mut self.nodes, syntax.nodes().len(), "function type nodes")?;
        charge(&mut self.edges, syntax.edges(), "function type edges")?;
        let annotation = parameters
            .bind_type(&syntax, self.uses)
            .map_err(|e| translate_error(e, range))?;
        charge(
            &mut self.uses,
            annotation.uses().len(),
            "function parameter uses",
        )?;
        Ok(GenericFunctionAnnotation {
            type_range: range,
            annotation,
        })
    }
}
fn error(tokens: &[Token], at: usize, message: &'static str) -> GenericTypeError {
    GenericTypeError::Syntax {
        span: tokens[at.min(tokens.len() - 1)].span,
        message,
    }
}
fn annotation_end(
    tokens: &[Token],
    mut cursor: usize,
    value: bool,
) -> Result<usize, GenericTypeError> {
    let mut angles = 0usize;
    let mut arrays = 0usize;
    loop {
        match tokens[cursor].kind {
            TokenKind::Punctuation(',' | ')') if value && angles == 0 && arrays == 0 => {
                return Ok(cursor)
            }
            TokenKind::Punctuation('{') if !value && angles == 0 && arrays == 0 => {
                return Ok(cursor)
            }
            TokenKind::Eof => return Err(error(tokens, cursor, "expected annotation terminator")),
            TokenKind::Operator("<") => {
                angles = angles
                    .checked_add(1)
                    .ok_or(GenericTypeError::Limit("annotation nesting"))?
            }
            TokenKind::Operator(">") => {
                angles = angles
                    .checked_sub(1)
                    .ok_or_else(|| error(tokens, cursor, "unexpected type argument terminator"))?
            }
            TokenKind::Punctuation('[') => {
                arrays = arrays
                    .checked_add(1)
                    .ok_or(GenericTypeError::Limit("annotation nesting"))?
            }
            TokenKind::Punctuation(']') => {
                arrays = arrays
                    .checked_sub(1)
                    .ok_or_else(|| error(tokens, cursor, "unexpected array terminator"))?
            }
            _ => {}
        }
        cursor += 1;
    }
}
/// Attach source-owned type parameters to explicit value/return annotations of one
/// root generic function. The body is only lexed and delimiter-balanced, never
/// interpreted as valid statement syntax or executable checked evidence.
pub fn parse_generic_function(
    source: &str,
    limits: GenericFunctionLimits,
) -> Result<GenericFunctionSyntax, GenericTypeError> {
    parse_function_mode(source, limits, false, false)
}
/// Admit one nongeneric source function as a symbolic specialization root.
/// Uses the same bounded annotation/body ownership as generic syntax; does not
/// validate an application entry convention or authorize compilation/execution.
/// `parameter_range()` is an empty original-source range at the value-list opener.
pub fn parse_specialization_root(
    source: &str,
    limits: GenericFunctionLimits,
) -> Result<GenericFunctionSyntax, GenericTypeError> {
    parse_function_mode(source, limits, true, false)
}
/// Parse a generic or plain header with opaque postfix-capable body syntax.
/// This never establishes body validity or executable propagation support.
pub fn parse_generic_propagation_function(
    source: &str,
    limits: GenericFunctionLimits,
) -> Result<GenericFunctionSyntax, GenericTypeError> {
    parse_function_mode(source, limits, false, true)
}
fn parse_function_mode(
    source: &str,
    limits: GenericFunctionLimits,
    plain: bool,
    propagation: bool,
) -> Result<GenericFunctionSyntax, GenericTypeError> {
    if limits.max_body_depth > 128 {
        return Err(GenericTypeError::Limit("fixed body depth ceiling"));
    }
    let tokens = if propagation {
        if source.len() > limits.max_source_bytes {
            return Err(GenericTypeError::Limit("source bytes"));
        }
        let tokens = crate::compiler::lexer::Lexer::new()
            .tokenize_propagation(
                source,
                crate::compiler::resources::FrontendLimits {
                    max_source_bytes: limits.max_source_bytes,
                    max_tokens: limits.max_tokens,
                    ..crate::compiler::resources::FrontendLimits::DEFAULT
                },
            )
            .map_err(GenericTypeError::Lexical)?;
        if tokens.len() > limits.max_tokens {
            return Err(GenericTypeError::Limit("tokens"));
        }
        tokens
    } else {
        type_tokens(source, limits.max_source_bytes, limits.max_tokens)?
    };
    let is_exported = tokens[0].kind == TokenKind::Keyword("export");
    let head = usize::from(is_exported);
    if tokens.get(head).map(|t| &t.kind) != Some(&TokenKind::Keyword("fn")) {
        return Err(error(
            &tokens,
            head,
            "expected generic function declaration",
        ));
    }
    let name_token = tokens
        .get(head + 1)
        .ok_or_else(|| error(&tokens, head, "expected function name"))?;
    let TokenKind::Identifier(name) = &name_token.kind else {
        return Err(error(&tokens, head + 1, "expected function name"));
    };
    let open = tokens
        .get(head + 2)
        .ok_or_else(|| error(&tokens, head + 1, "expected generic parameters"))?;
    let plain = if propagation {
        open.kind == TokenKind::Punctuation('(')
    } else {
        plain
    };
    let (parameter_range, value_open) = if plain {
        if open.kind != TokenKind::Punctuation('(') {
            return Err(error(
                &tokens,
                head + 2,
                "expected nongeneric value parameter list",
            ));
        }
        (
            Span {
                end: open.span.start,
                ..open.span
            },
            head + 2,
        )
    } else {
        if open.kind != TokenKind::Operator("<") {
            return Err(error(&tokens, head + 2, "expected generic parameters"));
        }
        let close = (head + 3..tokens.len())
            .find(|&i| tokens[i].kind == TokenKind::Operator(">"))
            .ok_or_else(|| {
                error(
                    &tokens,
                    tokens.len() - 1,
                    "expected closing generic parameter bracket",
                )
            })?;
        let parameter_range = Span {
            end: tokens[close].span.end,
            ..open.span
        };
        (parameter_range, close + 1)
    };
    if tokens.get(value_open).map(|t| &t.kind) != Some(&TokenKind::Punctuation('(')) {
        return Err(error(&tokens, value_open, "expected value parameter list"));
    }
    let mut admission = Admission {
        source: limits.max_retained_source_bytes,
        names: limits.max_name_bytes,
        nodes: limits.max_type_nodes,
        edges: limits.max_type_edges,
        uses: limits.max_parameter_uses,
    };
    charge(
        &mut admission.source,
        source.len(),
        "function retained source bytes",
    )?;
    charge(&mut admission.names, name.len(), "function names")?;
    let parameter_source = &source[parameter_range.start..parameter_range.end];
    charge(
        &mut admission.source,
        parameter_source.len(),
        "function retained source bytes",
    )?;
    let mut parameter_limits = limits.parameters;
    parameter_limits.max_name_bytes = parameter_limits.max_name_bytes.min(admission.names);
    let parameters = if plain {
        GenericParameters::empty()?
    } else {
        parse_generic_parameters(parameter_source, parameter_limits)
            .map_err(|e| translate_error(e, parameter_range))?
    };
    charge(
        &mut admission.names,
        parameters.name_bytes(),
        "function names",
    )?;
    let mut cursor = value_open + 1;
    let mut values = Vec::new();
    let mut names = BTreeSet::new();
    while tokens[cursor].kind != TokenKind::Punctuation(')') {
        let token = &tokens[cursor];
        let TokenKind::Identifier(value_name) = &token.kind else {
            return Err(error(&tokens, cursor, "expected value parameter name"));
        };
        if values.len() >= limits.max_value_parameters {
            return Err(GenericTypeError::Limit("value parameters"));
        }
        charge(&mut admission.names, value_name.len(), "function names")?;
        if !names.insert(value_name.as_str()) {
            return Err(error(&tokens, cursor, "duplicate value parameter"));
        }
        let name_span = token.span;
        cursor += 1;
        if tokens[cursor].kind != TokenKind::Punctuation(':') {
            return Err(error(
                &tokens,
                cursor,
                "value parameter requires an explicit annotation",
            ));
        }
        cursor += 1;
        let start = cursor;
        cursor = annotation_end(&tokens, cursor, true)?;
        if cursor == start {
            return Err(error(&tokens, cursor, "expected value parameter type"));
        }
        let range = Span {
            end: tokens[cursor - 1].span.end,
            ..tokens[start].span
        };
        let ty = admission.annotation(source, range, &parameters, limits.annotations)?;
        values.push(GenericFunctionParameter {
            name: value_name.clone(),
            span: Span {
                end: range.end,
                ..name_span
            },
            name_span,
            ty,
        });
        if tokens[cursor].kind == TokenKind::Punctuation(')') {
            break;
        }
        cursor += 1;
    }
    cursor += 1;
    let return_type = if tokens[cursor].kind == TokenKind::Operator("->") {
        cursor += 1;
        let start = cursor;
        cursor = annotation_end(&tokens, cursor, false)?;
        if start == cursor {
            return Err(error(&tokens, cursor, "expected return type"));
        }
        Some(admission.annotation(
            source,
            Span {
                end: tokens[cursor - 1].span.end,
                ..tokens[start].span
            },
            &parameters,
            limits.annotations,
        )?)
    } else {
        None
    };
    if tokens[cursor].kind != TokenKind::Punctuation('{') {
        return Err(error(&tokens, cursor, "expected function body"));
    }
    let body_start = tokens[cursor].span;
    let mut stack = Vec::new();
    let mut body_tokens = 0usize;
    loop {
        let token = &tokens[cursor];
        if token.kind == TokenKind::Eof {
            return Err(error(&tokens, cursor, "expected closing body delimiter"));
        }
        if body_tokens >= limits.max_body_tokens {
            return Err(GenericTypeError::Limit("body tokens"));
        }
        body_tokens += 1;
        match token.kind {
            TokenKind::Punctuation(open @ ('{' | '[' | '(')) => {
                if stack.len() >= limits.max_body_depth {
                    return Err(GenericTypeError::Limit("body depth"));
                }
                stack.push(open);
            }
            TokenKind::Punctuation(close @ ('}' | ']' | ')')) => {
                let expected = match close {
                    '}' => '{',
                    ']' => '[',
                    _ => '(',
                };
                if stack.pop() != Some(expected) {
                    return Err(error(&tokens, cursor, "mismatched body delimiter"));
                }
            }
            _ => {}
        }
        cursor += 1;
        if stack.is_empty() {
            break;
        }
    }
    let body_span = Span {
        end: tokens[cursor - 1].span.end,
        ..body_start
    };
    if tokens[cursor].kind != TokenKind::Eof {
        return Err(error(
            &tokens,
            cursor,
            "expected end of function declaration",
        ));
    }
    Ok(GenericFunctionSyntax {
        source: Arc::from(source),
        name: name.clone(),
        span: Span {
            end: body_span.end,
            ..tokens[0].span
        },
        name_span: name_token.span,
        is_exported,
        parameter_range,
        parameters,
        values: values.into(),
        return_type,
        body_span,
        body_tokens,
        retained_source_bytes: limits.max_retained_source_bytes - admission.source,
        name_bytes: limits.max_name_bytes - admission.names,
        type_nodes: limits.max_type_nodes - admission.nodes,
        type_edges: limits.max_type_edges - admission.edges,
        parameter_uses: limits.max_parameter_uses - admission.uses,
    })
}
