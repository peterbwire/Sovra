//! Source-owned E2 parameter lists and bounded annotation parameter-use binding.
use super::{generic_types::type_tokens, GenericTypeError, GenericTypeKind, GenericTypeSyntax};
use crate::compiler::{diagnostics::Span, lexer::TokenKind};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
/// Independent admission for a single parameter list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericParameterLimits {
    /// Complete original input bytes.
    pub max_source_bytes: usize,
    /// Lexer tokens including EOF.
    pub max_tokens: usize,
    /// Parameter records before name/index retention.
    pub max_parameters: usize,
    /// Cumulative copied parameter-name bytes; bounds are enum tags, not copied paths.
    pub max_name_bytes: usize,
}
impl GenericParameterLimits {
    /// Finite opt-in defaults, independent of eventual specialization counts.
    pub const DEFAULT: Self = Self {
        max_source_bytes: 64 * 1024,
        max_tokens: 16_384,
        max_parameters: 128,
        max_name_bytes: 4096,
    };
}
/// Parameter identity owned by exactly one successfully parsed declaration list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericParameterId {
    owner: u64,
    ordinal: usize,
}
impl GenericParameterId {
    /// Source-order index within the owning list, not a global type identity.
    pub fn ordinal(self) -> usize {
        self.ordinal
    }
}
/// Only the approved initial compiler-known bound is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericParameterBound {
    /// `std::Key`; concrete admissibility is a later semantic check.
    Key,
}
/// Original declaration metadata with a source-owned parameter identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParameter {
    /// Owner-scoped identity.
    pub id: GenericParameterId,
    /// Written unqualified parameter name.
    pub name: String,
    /// Full parameter range including the bound, if any.
    pub span: Span,
    /// Original short parameter-name range.
    pub name_span: Span,
    /// Optional approved bound.
    pub bound: Option<GenericParameterBound>,
    /// Original qualified bound range, excluding the colon.
    pub bound_span: Option<Span>,
}
/// Complete immutable list; duplicate names and unsupported bounds have refused.
#[derive(Debug, Clone)]
pub struct GenericParameters {
    source: Arc<str>,
    owner: u64,
    parameters: Arc<[GenericParameter]>,
    name_bytes: usize,
}
impl GenericParameters {
    pub(super) fn scope_key(&self) -> u64 {
        self.owner
    }
    pub(super) fn empty() -> Result<Self, GenericTypeError> {
        Ok(Self {
            source: Arc::from(""),
            owner: allocate_owner()?,
            parameters: Arc::from([]),
            name_bytes: 0,
        })
    }

    /// Original complete list with trivia and coordinates.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Parameters in declaration order.
    pub fn parameters(&self) -> &[GenericParameter] {
        &self.parameters
    }
    /// Cumulative copied names, excluding shared source text.
    pub fn name_bytes(&self) -> usize {
        self.name_bytes
    }
    /// Lookup in this declaration only; unrelated lists cannot share identities.
    pub fn find(&self, name: &str) -> Option<GenericParameterId> {
        self.parameters
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.id)
    }
    /// Read an identity only when its owner is this list.
    pub fn parameter(&self, id: GenericParameterId) -> Option<&GenericParameter> {
        (id.owner == self.owner)
            .then(|| self.parameters.get(id.ordinal))
            .flatten()
    }
    /// Bind exact unqualified parameter names in an admitted annotation.
    /// This checks only parameter uses: other names remain unresolved syntax.
    /// The returned immutable table retains both source owners.
    pub fn bind_type(
        &self,
        syntax: &GenericTypeSyntax,
        max_uses: usize,
    ) -> Result<GenericParameterUses, GenericTypeError> {
        let mut uses = Vec::new();
        for (node, item) in syntax.nodes().iter().enumerate() {
            let GenericTypeKind::Named { name, arguments } = &item.kind else {
                continue;
            };
            let Some(parameter) = self.find(name) else {
                continue;
            };
            if !arguments.is_empty() {
                return Err(GenericTypeError::Syntax {
                    span: item.span,
                    message: "type parameter cannot take type arguments",
                });
            }
            if uses.len() >= max_uses {
                return Err(GenericTypeError::Limit("parameter uses"));
            }
            uses.push(GenericParameterUse { node, parameter });
        }
        Ok(GenericParameterUses {
            parameters: self.clone(),
            syntax: syntax.clone(),
            uses: uses.into(),
        })
    }
}
/// One syntactically bound parameter occurrence in an immutable annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericParameterUse {
    /// Postorder node index in the retained annotation.
    pub node: usize,
    /// Identity in the retained declaration list.
    pub parameter: GenericParameterId,
}
/// Annotation-local parameter bindings; not complete semantic type evidence.
#[derive(Debug, Clone)]
pub struct GenericParameterUses {
    parameters: GenericParameters,
    syntax: GenericTypeSyntax,
    uses: Arc<[GenericParameterUse]>,
}
impl GenericParameterUses {
    /// Retained immutable parameter owner.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    /// Retained immutable annotation owner; node indices address this syntax only.
    pub fn syntax(&self) -> &GenericTypeSyntax {
        &self.syntax
    }
    /// Source/postorder parameter occurrences.
    pub fn uses(&self) -> &[GenericParameterUse] {
        &self.uses
    }
}
/// Parse exactly `<T, K:std::Key>` with at least one unique parameter.
/// No user traits, defaults, inference, declaration bodies or specialization are enabled.
pub fn parse_generic_parameters(
    source: &str,
    limits: GenericParameterLimits,
) -> Result<GenericParameters, GenericTypeError> {
    let tokens = type_tokens(source, limits.max_source_bytes, limits.max_tokens)?;
    let mut cursor = 0;
    let error = |cursor: usize, message| GenericTypeError::Syntax {
        span: tokens[cursor].span,
        message,
    };
    if tokens[cursor].kind != TokenKind::Operator("<") {
        return Err(error(cursor, "expected opening parameter bracket"));
    }
    cursor += 1;
    let mut names = BTreeSet::new();
    let mut parameters = Vec::new();
    let mut name_bytes = 0usize;
    loop {
        let token = &tokens[cursor];
        let TokenKind::Identifier(name) = &token.kind else {
            return Err(error(cursor, "expected type parameter name"));
        };
        if parameters.len() >= limits.max_parameters {
            return Err(GenericTypeError::Limit("parameters"));
        }
        name_bytes = name_bytes
            .checked_add(name.len())
            .filter(|n| *n <= limits.max_name_bytes)
            .ok_or(GenericTypeError::Limit("parameter name bytes"))?;
        if !names.insert(name.as_str()) {
            return Err(error(cursor, "duplicate type parameter"));
        }
        let name_span = token.span;
        let mut span = name_span;
        cursor += 1;
        let (bound, bound_span) = if tokens[cursor].kind == TokenKind::Punctuation(':') {
            cursor += 1;
            let start = tokens[cursor].span;
            for expected in [
                TokenKind::Identifier("std".into()),
                TokenKind::Operator("::"),
                TokenKind::Identifier("Key".into()),
            ] {
                if tokens[cursor].kind != expected {
                    return Err(error(cursor, "only std::Key is an admitted generic bound"));
                }
                cursor += 1;
            }
            span.end = tokens[cursor - 1].span.end;
            (
                Some(GenericParameterBound::Key),
                Some(Span {
                    end: span.end,
                    ..start
                }),
            )
        } else {
            (None, None)
        };
        parameters.push(GenericParameter {
            id: GenericParameterId {
                owner: 0,
                ordinal: parameters.len(),
            },
            name: name.clone(),
            span,
            name_span,
            bound,
            bound_span,
        });
        if tokens[cursor].kind == TokenKind::Operator(">") {
            cursor += 1;
            break;
        }
        if tokens[cursor].kind != TokenKind::Punctuation(',') {
            return Err(error(cursor, "expected comma or closing parameter bracket"));
        }
        cursor += 1;
    }
    if tokens[cursor].kind != TokenKind::Eof {
        return Err(error(cursor, "expected end of parameter list"));
    }
    let owner = allocate_owner()?;
    for parameter in &mut parameters {
        parameter.id.owner = owner;
    }
    Ok(GenericParameters {
        source: Arc::from(source),
        owner,
        parameters: parameters.into(),
        name_bytes,
    })
}

fn allocate_owner() -> Result<u64, GenericTypeError> {
    // Compare/exchange preserves the declared Rust 1.74 baseline and refuses
    // exhaustion without wrapping an owner identity.
    loop {
        let current = NEXT_OWNER.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .ok_or(GenericTypeError::Limit("parameter identities"))?;
        if NEXT_OWNER
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(current);
        }
    }
}
