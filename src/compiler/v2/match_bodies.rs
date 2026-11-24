//! Closed-scope match body validation, separate from whole-function execution.
use super::{
    parse_match_statement, CheckedEnumTypes, EnumId, EnumMatchArm, EnumMatchPlan, EnumTypeError,
    MatchStatementSyntax, MatchSyntaxLimits,
};
use crate::compiler::{
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    semantic::{self, DetachedBinding, Type},
};
use std::collections::HashSet;

/// An outer local supplied by a trusted embedding checker, not inferred from source.
#[derive(Debug, Clone, Copy)]
pub struct MatchOuterBinding<'a> {
    /// Local identifier, unique within the outer scope.
    pub name: &'a str,
    /// Closed annotation in the checked environment's canonical namespace.
    pub type_name: &'a str,
    /// Whether ordinary core assignment may update this local inside an arm.
    pub is_mutable: bool,
}
/// Independent admission for closed-scope match body validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBodyLimits {
    /// Original source parsing admission.
    pub syntax: MatchSyntaxLimits,
    /// Outer local count before type resolution or copying.
    pub max_outer_bindings: usize,
    /// Total label, expected return annotation and outer name/type UTF-8 bytes.
    pub max_context_text_bytes: usize,
    /// Sum of initial local/type bindings copied into all arm scopes.
    pub max_scope_bindings: usize,
}
impl MatchBodyLimits {
    /// Opt-in closed-body defaults; existing compiler command profiles are unchanged.
    pub const DEFAULT: Self = Self {
        syntax: MatchSyntaxLimits::DEFAULT,
        max_outer_bindings: 1024,
        max_context_text_bytes: 256 * 1024,
        max_scope_bindings: 65_536,
    };
}
/// Complete refusal: syntax, nominal coverage and body checking remain distinguishable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchBodyError {
    /// Original match-source grammar/admission failure.
    Syntax(Diagnostics),
    /// Environment identity or nominal coverage failure.
    Coverage(EnumTypeError),
    /// Closed-context or body diagnostics at original match-source locations.
    Diagnostics(Diagnostics),
    /// A closed-context admission ceiling was exceeded.
    Limit(&'static str),
}
impl std::fmt::Display for MatchBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match body validation refused: {self:?}")
    }
}
impl std::error::Error for MatchBodyError {}
/// Checked ordinary bodies for one exhaustive match in a caller-supplied scope.
/// This is not a checked function, executable IR, or proof of surrounding source.
#[derive(Debug, Clone)]
pub struct CheckedMatchBodies {
    syntax: MatchStatementSyntax,
    coverage: EnumMatchPlan,
    returns: Vec<bool>,
}
impl CheckedMatchBodies {
    /// Original source syntax, including ordinary body expressions.
    pub fn syntax(&self) -> &MatchStatementSyntax {
        &self.syntax
    }
    /// Nominal coverage plan retaining its checked enum environment.
    pub fn coverage(&self) -> &EnumMatchPlan {
        &self.coverage
    }
    /// Per-source-arm conservative return coverage.
    pub fn arm_returns(&self) -> &[bool] {
        &self.returns
    }
    /// True only when every exhaustive arm guarantees return; loops alone do not.
    pub fn guarantees_return(&self) -> bool {
        self.returns.iter().all(|value| *value)
    }
}
fn diagnostic(message: &'static str, span: Span) -> MatchBodyError {
    MatchBodyError::Diagnostics(Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code: "E3108",
            message: message.into(),
            span,
        }],
    })
}
fn identifier(name: &str) -> bool {
    use crate::compiler::{
        lexer::{Lexer, TokenKind},
        resources::FrontendLimits,
    };
    let Ok(tokens) = Lexer::new().tokenize_with_limits(
        name,
        FrontendLimits {
            max_source_bytes: name.len(),
            max_tokens: 1,
            max_diagnostics: 1,
        },
    ) else {
        return false;
    };
    tokens.len() == 2
        && matches!(&tokens[0].kind, TokenKind::Identifier(_))
        && tokens[0].span.start == 0
        && tokens[0].span.end == name.len()
}

impl CheckedEnumTypes {
    /// Parse and check an exhaustive match in an explicitly supplied closed scope.
    /// Only standard calls are available. User call-targets, module-local shorthand,
    /// nested match statements, lowering/coercion evidence and execution remain open.
    #[allow(clippy::too_many_arguments)] // Explicit source, nominal and embedding context.
    pub fn check_match_bodies(
        &self,
        enum_id: EnumId,
        source_label: &str,
        source: &str,
        outer: &[MatchOuterBinding<'_>],
        return_type: &str,
        limits: MatchBodyLimits,
    ) -> Result<CheckedMatchBodies, MatchBodyError> {
        if outer.len() > limits.max_outer_bindings {
            return Err(MatchBodyError::Limit("outer bindings"));
        }
        let mut bytes = source_label
            .len()
            .checked_add(return_type.len())
            .ok_or(MatchBodyError::Limit("context text"))?;
        for binding in outer {
            bytes = bytes
                .checked_add(binding.name.len())
                .and_then(|n| n.checked_add(binding.type_name.len()))
                .ok_or(MatchBodyError::Limit("context text"))?;
        }
        if bytes > limits.max_context_text_bytes {
            return Err(MatchBodyError::Limit("context text"));
        }
        let syntax =
            parse_match_statement(source, limits.syntax).map_err(MatchBodyError::Syntax)?;
        let arms: Vec<_> = syntax
            .arms
            .iter()
            .map(|arm| EnumMatchArm {
                enum_name: &arm.enum_name,
                variant_name: &arm.variant_name,
                has_payload_binding: arm.binding.is_some(),
                span: arm.span,
            })
            .collect();
        let coverage = self
            .check_match(enum_id, source_label, syntax.span, &arms)
            .map_err(MatchBodyError::Coverage)?;
        let count = outer
            .len()
            .checked_mul(syntax.arms.len())
            .and_then(|n| {
                n.checked_add(
                    syntax
                        .arms
                        .iter()
                        .filter(|arm| arm.binding.is_some())
                        .count(),
                )
            })
            .ok_or(MatchBodyError::Limit("scope bindings"))?;
        if count > limits.max_scope_bindings {
            return Err(MatchBodyError::Limit("scope bindings"));
        }
        let known = self.body_named_types();
        let record_fields = self.body_record_fields();
        let resolve =
            |name: &str| Type::from_name_with_known(name, Some(known)).canonicalize(known);
        let expected = resolve(return_type);
        if expected == Type::Unknown {
            return Err(diagnostic("unresolved match return type", syntax.span));
        }
        let mut names = HashSet::new();
        let mut types = Vec::new();
        for binding in outer {
            if !identifier(binding.name) || !names.insert(binding.name) {
                return Err(diagnostic(
                    "invalid or duplicate outer binding",
                    syntax.scrutinee_span,
                ));
            }
            let ty = resolve(binding.type_name);
            if ty == Type::Unknown {
                return Err(diagnostic(
                    "unresolved outer binding type",
                    syntax.scrutinee_span,
                ));
            }
            types.push(ty);
        }
        let Some(index) = outer
            .iter()
            .position(|binding| binding.name == syntax.scrutinee)
        else {
            return Err(diagnostic(
                "match scrutinee is not bound in the supplied scope",
                syntax.scrutinee_span,
            ));
        };
        let declaration = &self.declarations()[enum_id.ordinal()];
        if types[index] != Type::Named(declaration.name.clone()) {
            return Err(diagnostic(
                "match scrutinee has a different nominal type",
                syntax.scrutinee_span,
            ));
        }
        let mut fields = record_fields.clone();
        semantic::add_standard_record_fields(&mut fields);
        let mut returns = Vec::new();
        for (arm, variant) in syntax.arms.iter().zip(coverage.variant_order()) {
            let payload = declaration.variants[*variant].payload;
            let payload_type = payload
                .map(|id| self.body_payload_type(id))
                .transpose()
                .map_err(MatchBodyError::Coverage)?;
            let mut bindings: Vec<_> = outer
                .iter()
                .zip(&types)
                .map(|(binding, ty)| DetachedBinding {
                    name: binding.name,
                    ty,
                    mutable: binding.is_mutable,
                })
                .collect();
            if let (Some(name), Some(ty)) = (&arm.binding, &payload_type) {
                bindings.push(DetachedBinding {
                    name,
                    ty,
                    mutable: false,
                });
            }
            returns.push(
                semantic::check_detached_body(&arm.body, &bindings, &expected, known, &fields)
                    .map_err(MatchBodyError::Diagnostics)?,
            );
        }
        Ok(CheckedMatchBodies {
            syntax,
            coverage,
            returns,
        })
    }
}
