//! Source-owned literal values for concrete generic bodies, before executable lowering.
use super::{
    CheckedConcreteGenericBodies, ConcreteGenericBody, GenericCopyExpressionId,
    GenericCopyExpressionKind, GenericPrimitive,
};
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, TokenKind},
    resources::FrontendLimits,
};
use std::{collections::BTreeMap, sync::Arc};
/// Admission for literal payload decoding across a complete concrete graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericLiteralLimits {
    /// Concrete instance wrappers.
    pub max_instances: usize,
    /// Unique original source literal facts.
    pub max_literals: usize,
    /// Literal references across all instances, including reused facts.
    pub max_references: usize,
    /// Relexed original literal fragment bytes across unique facts.
    pub max_source_bytes: usize,
    /// Fragment lexer tokens including one EOF per unique literal.
    pub max_tokens: usize,
    /// Copied decoded UTF-8 string bytes across unique facts.
    pub max_string_bytes: usize,
    /// Source-byte, token, expression and reference processing work.
    pub max_work: usize,
    /// Independent original fragment byte allowance.
    pub max_fragment_bytes: usize,
    /// Independent fragment token allowance including EOF.
    pub max_fragment_tokens: usize,
    /// Grouping depth in 1..=128; no new expression syntax is introduced.
    pub max_depth: usize,
    /// Independent lexer diagnostic allowance.
    pub max_diagnostics: usize,
}
impl GenericLiteralLimits {
    /// Finite library profile; literal decoding does not enable execution.
    pub const DEFAULT: Self = Self {
        max_instances: 1024,
        max_literals: 65_536,
        max_references: 131_072,
        max_source_bytes: 4 * 1024 * 1024,
        max_tokens: 262_144,
        max_string_bytes: 1024 * 1024,
        max_work: 8_000_000,
        max_fragment_bytes: 128 * 1024,
        max_fragment_tokens: 258,
        max_depth: 128,
        max_diagnostics: 64,
    };
}
/// Complete refusal with no partially decoded graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericLiteralError {
    /// Selected admission dimension exhausted or depth profile invalid.
    Limit(&'static str),
    /// Fragment lexing failed; locations are translated to the original source.
    Lexical {
        /// Original graph-local instance being decoded.
        instance: usize,
        /// Original-source lexer diagnostics.
        diagnostics: Diagnostics,
    },
    /// Source range, literal shape/type or arena identity is inconsistent.
    Source {
        /// Graph-local instance.
        instance: usize,
        /// Original full-function expression range.
        span: Span,
    },
}
impl std::fmt::Display for GenericLiteralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic literal decoding refused: {self:?}")
    }
}
impl std::error::Error for GenericLiteralError {}
/// Exact primitive payload; FloatBits preserves the finite f64 bit representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericLiteralValue {
    /// Checked signed 64-bit integer value.
    Int(i64),
    /// Finite IEEE-754 binary64 bits, without a later textual reparse.
    FloatBits(u64),
    /// Lexer-decoded UTF-8 text with no added formatting or normalization.
    String(Arc<str>),
    /// Boolean value.
    Bool(bool),
}
impl GenericLiteralValue {
    /// Original expression primitive, before any contextual conversion.
    pub fn primitive(&self) -> GenericPrimitive {
        match self {
            Self::Int(_) => GenericPrimitive::Int,
            Self::FloatBits(_) => GenericPrimitive::Float,
            Self::String(_) => GenericPrimitive::String,
            Self::Bool(_) => GenericPrimitive::Bool,
        }
    }
}
/// One decoded original literal, shared by all instances of its source body.
#[derive(Debug, Clone)]
pub struct GenericDecodedLiteral {
    /// Original symbolic body expression identity.
    pub expression: GenericCopyExpressionId,
    /// Original expression range, including grouping parentheses.
    pub span: Span,
    /// Exact primitive value, before conversion edges are applied.
    pub value: GenericLiteralValue,
}
/// Concrete instance with an immutable, source-order literal payload table.
#[derive(Debug, Clone)]
pub struct GenericLiteralBody {
    body: ConcreteGenericBody,
    literals: Arc<[GenericDecodedLiteral]>,
}
impl GenericLiteralBody {
    /// Original concrete type/context proof for this instance.
    pub fn body(&self) -> &ConcreteGenericBody {
        &self.body
    }
    /// Literal facts in original expression order; other expressions are excluded.
    pub fn literals(&self) -> &[GenericDecodedLiteral] {
        &self.literals
    }
    /// Inspect a literal only when its source ID belongs to this original body arena.
    pub fn literal(&self, id: GenericCopyExpressionId) -> Option<&GenericDecodedLiteral> {
        self.body.body().expression(id)?;
        let at = self
            .literals
            .binary_search_by_key(&id.ordinal(), |fact| fact.expression.ordinal())
            .ok()?;
        self.literals.get(at)
    }
}
/// Logical decoding usage; payload copies are charged once and instance references separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericLiteralUsage {
    /// Concrete instances.
    pub instances: usize,
    /// Unique decoded source literals.
    pub literals: usize,
    /// Literal references across instances.
    pub references: usize,
    /// Original fragment bytes relexed.
    pub source_bytes: usize,
    /// Fragment tokens including EOFs.
    pub tokens: usize,
    /// Copied decoded UTF-8 string bytes.
    pub string_bytes: usize,
    /// Logical processing work.
    pub work: usize,
}
/// Sealed decoded payloads for a complete concrete graph, not executable instructions.
#[derive(Debug, Clone)]
pub struct CheckedGenericLiterals {
    concrete: CheckedConcreteGenericBodies,
    bodies: Arc<[GenericLiteralBody]>,
    usage: GenericLiteralUsage,
}
impl CheckedGenericLiterals {
    /// Retained source/graph/concrete checking proofs.
    pub fn concrete(&self) -> &CheckedConcreteGenericBodies {
        &self.concrete
    }
    /// Instance-order literal bodies matching the retained specialization graph.
    pub fn bodies(&self) -> &[GenericLiteralBody] {
        &self.bodies
    }
    /// Exact cumulative processing usage.
    pub fn usage(&self) -> GenericLiteralUsage {
        self.usage
    }
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<(), GenericLiteralError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericLiteralError::Limit(name))?;
    Ok(())
}
fn decode(
    source: &str,
    span: Span,
    expected: GenericPrimitive,
    instance: usize,
    limits: GenericLiteralLimits,
    left: &mut GenericLiteralLimits,
) -> Result<GenericLiteralValue, GenericLiteralError> {
    let invalid = || GenericLiteralError::Source { instance, span };
    let fragment = source.get(span.start..span.end).ok_or_else(invalid)?;
    if fragment.len() > limits.max_fragment_bytes {
        return Err(GenericLiteralError::Limit("literal fragment bytes"));
    }
    charge(
        &mut left.max_source_bytes,
        fragment.len(),
        "literal source bytes",
    )?;
    charge(&mut left.max_work, fragment.len(), "literal work")?;
    let token_cap = limits.max_fragment_tokens.min(left.max_tokens);
    if token_cap == 0 {
        return Err(GenericLiteralError::Limit("literal tokens"));
    }
    let mut tokens = Lexer::new()
        .tokenize_with_limits(
            fragment,
            FrontendLimits {
                max_source_bytes: fragment.len(),
                max_tokens: token_cap - 1,
                max_diagnostics: limits.max_diagnostics,
            },
        )
        .map_err(|mut diagnostics| {
            for diagnostic in &mut diagnostics.items {
                let original = diagnostic.span;
                diagnostic.span = Span {
                    start: span.start + original.start,
                    end: span.start + original.end,
                    line: span.line + original.line,
                    column: if original.line == 0 {
                        span.column + original.column
                    } else {
                        original.column
                    },
                };
            }
            GenericLiteralError::Lexical {
                instance,
                diagnostics,
            }
        })?;
    charge(&mut left.max_tokens, tokens.len(), "literal tokens")?;
    charge(&mut left.max_work, tokens.len(), "literal work")?;
    let mut at = 0;
    while tokens.get(at).map(|t| &t.kind) == Some(&TokenKind::Punctuation('(')) {
        at += 1;
        if at > limits.max_depth {
            return Err(GenericLiteralError::Limit("literal grouping depth"));
        }
    }
    // Exactly one literal with balanced surrounding grouping; no calls, reads,
    // operators, trailing statements or source rewriting can enter this table.
    if tokens.len()
        != at
            .checked_mul(2)
            .and_then(|n| n.checked_add(2))
            .ok_or_else(invalid)?
    {
        return Err(invalid());
    }
    if tokens[at + 1..tokens.len() - 1]
        .iter()
        .any(|t| t.kind != TokenKind::Punctuation(')'))
    {
        return Err(invalid());
    }
    let token = std::mem::replace(&mut tokens[at].kind, TokenKind::Eof);
    let value = match token {
        TokenKind::Integer(text) => GenericLiteralValue::Int(text.parse().map_err(|_| invalid())?),
        TokenKind::Float(text) => {
            let value: f64 = text.parse().map_err(|_| invalid())?;
            if !value.is_finite() {
                return Err(invalid());
            }
            GenericLiteralValue::FloatBits(value.to_bits())
        }
        TokenKind::String(text) => {
            charge(
                &mut left.max_string_bytes,
                text.len(),
                "literal string bytes",
            )?;
            charge(&mut left.max_work, text.len(), "literal work")?;
            GenericLiteralValue::String(text.into())
        }
        TokenKind::Keyword("true") => GenericLiteralValue::Bool(true),
        TokenKind::Keyword("false") => GenericLiteralValue::Bool(false),
        _ => return Err(invalid()),
    };
    if value.primitive() != expected {
        return Err(invalid());
    }
    Ok(value)
}
/// Decode every original literal in the concrete graph using the shared lexer.
/// Duplicate function instances share immutable payload facts; every instance
/// reference is checked against its concrete type and charged separately.
/// Decoding does not apply conversions, evaluate expressions or enable a backend.
pub fn decode_generic_literals(
    concrete: &CheckedConcreteGenericBodies,
    limits: GenericLiteralLimits,
) -> Result<CheckedGenericLiterals, GenericLiteralError> {
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(GenericLiteralError::Limit("literal depth profile"));
    }
    let mut left = limits;
    let mut cache: BTreeMap<usize, Arc<[GenericDecodedLiteral]>> = BTreeMap::new();
    let mut bodies = Vec::new();
    for (instance, body) in concrete.bodies().iter().enumerate() {
        charge(&mut left.max_instances, 1, "literal instances")?;
        charge(&mut left.max_work, 1, "literal work")?;
        let signature = body.body().signature();
        let key = signature.id().ordinal();
        let literals = if let Some(cached) = cache.get(&key) {
            cached.clone()
        } else {
            let mut facts = Vec::new();
            for expression in body.body().expressions() {
                charge(&mut left.max_work, 1, "literal work")?;
                let GenericCopyExpressionKind::Literal(expected) = expression.kind else {
                    continue;
                };
                charge(&mut left.max_literals, 1, "source literals")?;
                let value = decode(
                    signature.syntax().source(),
                    expression.span,
                    expected,
                    instance,
                    limits,
                    &mut left,
                )?;
                facts.push(GenericDecodedLiteral {
                    expression: expression.id,
                    span: expression.span,
                    value,
                });
            }
            let facts: Arc<[GenericDecodedLiteral]> = facts.into();
            cache.insert(key, facts.clone());
            facts
        };
        for fact in literals.iter() {
            charge(&mut left.max_references, 1, "literal references")?;
            charge(&mut left.max_work, 1, "literal work")?;
            let expected = body
                .expression_type(fact.expression)
                .and_then(|ty| ty.primitive());
            if expected != Some(fact.value.primitive()) {
                return Err(GenericLiteralError::Source {
                    instance,
                    span: fact.span,
                });
            }
        }
        bodies.push(GenericLiteralBody {
            body: body.clone(),
            literals,
        });
    }
    Ok(CheckedGenericLiterals {
        concrete: concrete.clone(),
        bodies: bodies.into(),
        usage: GenericLiteralUsage {
            instances: limits.max_instances - left.max_instances,
            literals: limits.max_literals - left.max_literals,
            references: limits.max_references - left.max_references,
            source_bytes: limits.max_source_bytes - left.max_source_bytes,
            tokens: limits.max_tokens - left.max_tokens,
            string_bytes: limits.max_string_bytes - left.max_string_bytes,
            work: limits.max_work - left.max_work,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fresh_limits() -> GenericLiteralLimits {
        GenericLiteralLimits::DEFAULT
    }
    fn span(source: &str) -> Span {
        Span {
            start: 0,
            end: source.len(),
            line: 0,
            column: 0,
        }
    }
    #[test]
    fn nonliteral_shapes_overflow_nonfinite_and_wrong_primitive_never_decode() {
        for source in [
            "",
            "()",
            "1+2",
            "call()",
            "true false",
            "((1)",
            "1;",
            "9223372036854775808",
        ] {
            assert!(
                decode(
                    source,
                    span(source),
                    GenericPrimitive::Int,
                    0,
                    GenericLiteralLimits::DEFAULT,
                    &mut fresh_limits()
                )
                .is_err(),
                "{source}"
            );
        }
        let overflow = format!("{}.0", "9".repeat(309));
        assert!(decode(
            &overflow,
            span(&overflow),
            GenericPrimitive::Float,
            0,
            GenericLiteralLimits::DEFAULT,
            &mut fresh_limits()
        )
        .is_err());
        assert!(decode(
            "true",
            span("true"),
            GenericPrimitive::Int,
            0,
            GenericLiteralLimits::DEFAULT,
            &mut fresh_limits()
        )
        .is_err());
        assert!(decode(
            "true",
            span("true"),
            GenericPrimitive::Unit,
            0,
            GenericLiteralLimits::DEFAULT,
            &mut fresh_limits()
        )
        .is_err());
        let grouped = format!("{}1{}", "(".repeat(128), ")".repeat(128));
        assert_eq!(
            decode(
                &grouped,
                span(&grouped),
                GenericPrimitive::Int,
                0,
                GenericLiteralLimits::DEFAULT,
                &mut fresh_limits()
            )
            .unwrap(),
            GenericLiteralValue::Int(1)
        );
        assert!(decode(
            &grouped,
            span(&grouped),
            GenericPrimitive::Int,
            0,
            GenericLiteralLimits {
                max_depth: 127,
                ..GenericLiteralLimits::DEFAULT
            },
            &mut fresh_limits()
        )
        .is_err());
    }
    #[test]
    fn lexer_diagnostics_are_located_in_original_unicode_multiline_source() {
        let source = concat!(
            "// λ😀\r\nfn f(){",
            r#"(( // grouping
"bad\q"))}"#
        );
        let start = source.find("((").unwrap();
        let end = source.rfind("))").unwrap() + 2;
        let range = Span {
            start,
            end,
            line: 1,
            column: 7,
        };
        let error = decode(
            source,
            range,
            GenericPrimitive::String,
            3,
            GenericLiteralLimits::DEFAULT,
            &mut fresh_limits(),
        )
        .unwrap_err();
        let GenericLiteralError::Lexical {
            instance,
            diagnostics,
        } = error
        else {
            panic!()
        };
        assert_eq!(instance, 3);
        let diagnostic = &diagnostics.items[0];
        assert_eq!(diagnostic.code, "E1002");
        assert_eq!(diagnostic.span.line, 2);
        assert_eq!(diagnostic.span.column, 0);
        assert!(diagnostic.span.start >= range.start && diagnostic.span.end <= range.end);
        assert!(source[diagnostic.span.start..diagnostic.span.end].contains("bad"));
    }
}
