//! Source-owned generic alias attachment; not alias resolution or specialization.
use super::{
    generic_types::type_tokens, parse_generic_parameters, parse_generic_type, GenericParameterId,
    GenericParameterLimits, GenericParameterUses, GenericParameters, GenericTypeError,
    GenericTypeLimits,
};
use crate::compiler::{diagnostics::Span, lexer::TokenKind};
use std::sync::Arc;
/// Whole-declaration and aggregate quotas, in addition to fragment quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericAliasLimits {
    /// Complete original declaration bytes before lexing.
    pub max_source_bytes: usize,
    /// Complete original lexer tokens, including EOF.
    pub max_tokens: usize,
    /// Total retained full-source plus parameter/target fragment bytes.
    pub max_retained_source_bytes: usize,
    /// Aggregate alias/parameter/target copied name bytes.
    pub max_name_bytes: usize,
    /// Annotation parameter-use records.
    pub max_parameter_uses: usize,
    /// Independent parameter-list quotas.
    pub parameters: GenericParameterLimits,
    /// Independent target-type quotas.
    pub target: GenericTypeLimits,
}
impl GenericAliasLimits {
    /// Finite opt-in declaration syntax profile; no source command changes.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 128 * 1024,
        max_tokens: 32_768,
        max_retained_source_bytes: 256 * 1024,
        max_name_bytes: 48 * 1024,
        max_parameter_uses: 4096,
        parameters: GenericParameterLimits::DEFAULT,
        target: GenericTypeLimits::DEFAULT,
    };
}
/// A complete generic alias declaration with attached parameter-use ownership.
#[derive(Debug, Clone)]
pub struct GenericAliasSyntax {
    source: Arc<str>,
    name: String,
    span: Span,
    name_span: Span,
    parameter_range: Span,
    target_range: Span,
    target: GenericParameterUses,
    retained_source_bytes: usize,
    name_bytes: usize,
}
impl GenericAliasSyntax {
    /// Complete original declaration, including surrounding trivia.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Original short declaration name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Full declaration range, excluding surrounding trivia.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Original alias-name selection range.
    pub fn name_span(&self) -> Span {
        self.name_span
    }
    /// Full-source range corresponding exactly to the parameter fragment.
    pub fn parameter_range(&self) -> Span {
        self.parameter_range
    }
    /// Full-source range corresponding exactly to the target fragment.
    pub fn target_range(&self) -> Span {
        self.target_range
    }
    /// Attached declaration-local parameter owner. Its internal spans are fragment-local.
    pub fn parameters(&self) -> &GenericParameters {
        self.target.parameters()
    }
    /// Bound target annotation; its internal spans are target-fragment-local.
    pub fn target(&self) -> &GenericParameterUses {
        &self.target
    }
    /// A parameter's full declaration range in this alias's original full source.
    /// Foreign parameter identities refuse.
    pub fn parameter_span(&self, id: GenericParameterId) -> Option<Span> {
        Some(translate(
            self.parameters().parameter(id)?.span,
            self.parameter_range,
        ))
    }
    /// A target node range in this alias's original full source.
    pub fn target_node_span(&self, node: usize) -> Option<Span> {
        Some(translate(
            self.target.syntax().nodes().get(node)?.span,
            self.target_range,
        ))
    }
    /// Aggregate retained original/fragment bytes, charged before copying.
    pub fn retained_source_bytes(&self) -> usize {
        self.retained_source_bytes
    }
    /// Aggregate alias/parameter/target copied name bytes.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
}
pub(super) fn translate(span: Span, origin: Span) -> Span {
    Span {
        start: origin.start + span.start,
        end: origin.start + span.end,
        line: origin.line + span.line,
        column: if span.line == 0 {
            origin.column + span.column
        } else {
            span.column
        },
    }
}
pub(super) fn translate_error(error: GenericTypeError, origin: Span) -> GenericTypeError {
    match error {
        GenericTypeError::Syntax { span, message } => GenericTypeError::Syntax {
            span: translate(span, origin),
            message,
        },
        GenericTypeError::Lexical(mut diagnostics) => {
            for diagnostic in &mut diagnostics.items {
                diagnostic.span = translate(diagnostic.span, origin);
            }
            GenericTypeError::Lexical(diagnostics)
        }
        error => error,
    }
}
/// Parse exactly one root `type Name<T> = Target<T>;` and attach its parameter owner.
/// Name/arity/visibility resolution, recursive alias checking and substitution are
/// later semantic stages. No generic function/type is made executable by this API.
pub fn parse_generic_alias(
    source: &str,
    limits: GenericAliasLimits,
) -> Result<GenericAliasSyntax, GenericTypeError> {
    let tokens = type_tokens(source, limits.max_source_bytes, limits.max_tokens)?;
    let error = |index: usize, message| GenericTypeError::Syntax {
        span: tokens[index].span,
        message,
    };
    if tokens[0].kind != TokenKind::Keyword("type") {
        return Err(error(0, "expected generic alias declaration"));
    }
    let Some(name_token) = tokens.get(1) else {
        return Err(error(0, "expected alias name"));
    };
    let TokenKind::Identifier(name) = &name_token.kind else {
        return Err(error(1, "expected alias name"));
    };
    let Some(open) = tokens.get(2) else {
        return Err(error(1, "expected generic parameters"));
    };
    if open.kind != TokenKind::Operator("<") {
        return Err(error(2, "expected generic parameters"));
    }
    let close = (3..tokens.len())
        .find(|&i| matches!(tokens[i].kind, TokenKind::Operator(">" | ">=")))
        .ok_or_else(|| error(tokens.len() - 1, "expected closing parameter bracket"))?;
    let joined = tokens[close].kind == TokenKind::Operator(">=");
    // The ordinary lexer owns >= as one comparison token. Only this declaration
    // grammar splits its first byte as the parameter terminator; bodies/operators
    // are never rewritten. This keeps `type Rows<T>=[T];` valid syntax.
    let end = if joined {
        tokens[close].span.start + 1
    } else {
        tokens[close].span.end
    };
    let parameter_range = Span { end, ..open.span };
    let target_start = if joined {
        close + 1
    } else {
        let assignment = close + 1;
        if tokens.get(assignment).map(|t| &t.kind) != Some(&TokenKind::Operator("=")) {
            return Err(error(
                assignment.min(tokens.len() - 1),
                "expected alias assignment",
            ));
        }
        close + 2
    };
    let last = tokens.len().saturating_sub(2);
    if tokens[last].kind != TokenKind::Punctuation(';') {
        return Err(error(last, "expected alias terminator"));
    }
    if target_start >= last {
        return Err(error(last, "expected alias target type"));
    }
    let target_range = Span {
        end: tokens[last - 1].span.end,
        ..tokens[target_start].span
    };
    let parameter_source = &source[parameter_range.start..parameter_range.end];
    let target_source = &source[target_range.start..target_range.end];
    let retained_source_bytes = source
        .len()
        .checked_add(parameter_source.len())
        .and_then(|n| n.checked_add(target_source.len()))
        .filter(|n| *n <= limits.max_retained_source_bytes)
        .ok_or(GenericTypeError::Limit("retained alias source bytes"))?;
    let mut names_left = limits
        .max_name_bytes
        .checked_sub(name.len())
        .ok_or(GenericTypeError::Limit("alias name bytes"))?;
    let mut parameter_limits = limits.parameters;
    parameter_limits.max_name_bytes = parameter_limits.max_name_bytes.min(names_left);
    let parameters = parse_generic_parameters(parameter_source, parameter_limits)
        .map_err(|e| translate_error(e, parameter_range))?;
    names_left = names_left
        .checked_sub(parameters.name_bytes())
        .ok_or(GenericTypeError::Limit("alias name bytes"))?;
    let mut target_limits = limits.target;
    target_limits.max_name_bytes = target_limits.max_name_bytes.min(names_left);
    let syntax = parse_generic_type(target_source, target_limits)
        .map_err(|e| translate_error(e, target_range))?;
    names_left = names_left
        .checked_sub(syntax.name_bytes())
        .ok_or(GenericTypeError::Limit("alias name bytes"))?;
    let target = parameters
        .bind_type(&syntax, limits.max_parameter_uses)
        .map_err(|e| translate_error(e, target_range))?;
    Ok(GenericAliasSyntax {
        source: Arc::from(source),
        name: name.clone(),
        span: Span {
            end: tokens[last].span.end,
            ..tokens[0].span
        },
        name_span: name_token.span,
        parameter_range,
        target_range,
        target,
        retained_source_bytes,
        name_bytes: limits.max_name_bytes - names_left,
    })
}
