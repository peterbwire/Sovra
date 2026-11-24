//! Source-driven checking for one versioned function with direct match statements.
use super::{
    CheckedEnumTypes, EnumMatchArm, EnumMatchPlan, EnumTypeError, MatchStatementSyntax,
    MatchSyntaxLimits,
};
use crate::compiler::{
    ast::{Parameter, Statement},
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    lexer::{Lexer, TokenKind},
    parser::Parser,
    resources::FrontendLimits,
    semantic::{self, DetachedBinding, DetachedScope, Type},
};
/// Limits for one source-driven versioned function check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumFunctionLimits {
    /// Original source, tokens and parser diagnostics.
    pub frontend: FrontendLimits,
    /// Source parameters admitted before semantic scope construction.
    pub max_parameters: usize,
    /// Direct function-body statements; nested core bodies use frontend/depth limits.
    pub max_statements: usize,
    /// Cumulative match arms across the function.
    pub max_match_arms: usize,
    /// Cumulative owned match-header text across the function.
    pub max_match_header_text_bytes: usize,
    /// Cumulative initial outer/payload bindings copied into arm scopes.
    pub max_scope_bindings: usize,
    /// Opaque source-label UTF-8 bytes, checked before retention by coverage errors.
    pub max_source_label_bytes: usize,
}
impl EnumFunctionLimits {
    /// Opt-in source-checking defaults; CLI profiles remain unchanged.
    pub const DEFAULT: Self = Self {
        frontend: FrontendLimits::DEFAULT,
        max_parameters: 1024,
        max_statements: 250_000,
        max_match_arms: 4096,
        max_match_header_text_bytes: 256 * 1024,
        max_scope_bindings: 65_536,
        max_source_label_bytes: 256 * 1024,
    };
}
/// A versioned function-body statement; no match is projected as a v1 statement.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumFunctionStatement {
    /// Existing core syntax, without nested matches.
    Core(Statement),
    /// A direct match with ordinary core arm bodies.
    Match(MatchStatementSyntax),
    /// Program-only match with complete recursively retained arm bodies.
    RecursiveMatch(super::RecursiveMatchSyntax),
    /// Program-only conditional/loop with complete versioned bodies.
    Control(super::RecursiveControlSyntax),
}
/// Owned single-function syntax, separate from public v1 Function/Program stages.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumFunctionSyntax {
    /// Source name.
    pub name: String,
    /// Export intent only; package visibility is not established.
    pub is_exported: bool,
    /// Original explicitly annotated parameters.
    pub parameters: Vec<Parameter>,
    /// Written return annotation; absent means Unit.
    pub return_type: Option<String>,
    /// Original return annotation location.
    pub return_type_span: Option<Span>,
    /// Ordered versioned statements.
    pub body: Vec<EnumFunctionStatement>,
    /// Original complete function location, preserving core export-span convention.
    pub span: Span,
}
/// Whole-function refusal without partially successful checked source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumFunctionError {
    /// Lexical/parser/depth/admission diagnostics.
    Syntax(Diagnostics),
    /// Source body/signature/return diagnostics.
    Diagnostics(Diagnostics),
    /// Nominal enum identity or exhaustive coverage failure.
    Coverage(EnumTypeError),
    /// A versioned function-specific admission dimension was exhausted.
    Limit(&'static str),
}
impl std::fmt::Display for EnumFunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enum function check refused: {self:?}")
    }
}
impl std::error::Error for EnumFunctionError {}
/// Checked source scopes and return coverage; not executable/lowering evidence.
#[derive(Debug, Clone)]
pub struct CheckedEnumFunction {
    syntax: EnumFunctionSyntax,
    environment: CheckedEnumTypes,
    matches: Vec<EnumMatchPlan>,
    returns: bool,
    program: Option<std::sync::Arc<super::program_source::ProgramHeaders>>,
}
impl CheckedEnumFunction {
    pub(super) fn program_target_name(&self, id: super::ProgramFunctionId) -> Option<&str> {
        self.program
            .as_ref()?
            .functions
            .get(id.ordinal())
            .filter(|(actual, _)| *actual == id)
            .map(|(_, f)| f.name.as_str())
    }
    pub(super) fn owns_program_function(&self, id: super::ProgramFunctionId) -> bool {
        self.program
            .as_ref()
            .and_then(|p| p.functions.get(id.ordinal()))
            .is_some_and(|(actual, f)| {
                *actual == id
                    && f.span == self.syntax.span
                    && f.name.rsplit("::").next() == Some(self.syntax.name.as_str())
            })
    }
    /// Source-owned versioned syntax.
    pub fn syntax(&self) -> &EnumFunctionSyntax {
        &self.syntax
    }
    /// Immutable retained nominal type environment.
    pub fn environment(&self) -> &CheckedEnumTypes {
        &self.environment
    }
    /// Direct-match coverage plans in source order.
    pub fn matches(&self) -> &[EnumMatchPlan] {
        &self.matches
    }
    /// Conservative guaranteed return, after type correctness was established.
    pub fn guarantees_return(&self) -> bool {
        self.returns
    }
}
fn diagnostic(code: &'static str, message: &'static str, span: Span) -> Diagnostics {
    Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
        }],
    }
}
fn parse(
    source: &str,
    limits: EnumFunctionLimits,
) -> Result<EnumFunctionSyntax, EnumFunctionError> {
    let tokens = Lexer::new()
        .tokenize_with_limits(source, limits.frontend)
        .map_err(EnumFunctionError::Syntax)?;
    parse_tokens(&tokens, limits)
}
pub(super) fn parse_tokens(
    tokens: &[crate::compiler::lexer::Token],
    limits: EnumFunctionLimits,
) -> Result<EnumFunctionSyntax, EnumFunctionError> {
    if let Err(span) = crate::compiler::limits::check_nesting(tokens, true) {
        return Err(EnumFunctionError::Syntax(diagnostic(
            "E2007",
            "maximum structural depth exceeded",
            span,
        )));
    }
    let (header, mut cursor) = Parser::new()
        .parse_function_header_prefix(tokens, limits.frontend.max_diagnostics)
        .map_err(EnumFunctionError::Syntax)?;
    if header.parameters.len() > limits.max_parameters {
        return Err(EnumFunctionError::Limit("parameters"));
    }
    if tokens[cursor].kind != TokenKind::Punctuation('{') {
        return Err(EnumFunctionError::Syntax(diagnostic(
            "E2005",
            "expected function body",
            tokens[cursor].span,
        )));
    }
    cursor += 1;
    let mut body = Vec::new();
    let mut arms = 0;
    let mut text = 0;
    while tokens[cursor].kind != TokenKind::Punctuation('}') {
        if body.len() >= limits.max_statements {
            return Err(EnumFunctionError::Limit("statements"));
        }
        let is_match = matches!(&tokens[cursor].kind,TokenKind::Identifier(name) if name=="match")
            && tokens
                .get(cursor + 1)
                .is_some_and(|t| matches!(t.kind, TokenKind::Identifier(_)))
            && tokens
                .get(cursor + 2)
                .is_some_and(|t| t.kind == TokenKind::Punctuation('{'));
        if is_match {
            let (syntax, consumed, bytes) = super::match_syntax::parse_match_prefix(
                &tokens[cursor..],
                MatchSyntaxLimits {
                    frontend: limits.frontend,
                    max_arms: limits.max_match_arms - arms,
                    max_header_text_bytes: limits.max_match_header_text_bytes - text,
                },
            )
            .map_err(EnumFunctionError::Syntax)?;
            arms += syntax.arms.len();
            text += bytes;
            cursor += consumed;
            body.push(EnumFunctionStatement::Match(syntax));
        } else {
            let (statement, consumed) = Parser::new()
                .parse_statement_prefix(&tokens[cursor..], limits.frontend.max_diagnostics)
                .map_err(EnumFunctionError::Syntax)?;
            cursor += consumed;
            body.push(EnumFunctionStatement::Core(statement));
        }
    }
    let span = Span {
        end: tokens[cursor].span.end,
        ..header.span
    };
    cursor += 1;
    if tokens[cursor].kind != TokenKind::Eof {
        return Err(EnumFunctionError::Syntax(diagnostic(
            "E2000",
            "expected exactly one function",
            tokens[cursor].span,
        )));
    }
    Ok(EnumFunctionSyntax {
        name: header.name,
        is_exported: header.is_exported,
        parameters: header.parameters,
        return_type: header.return_type,
        return_type_span: header.return_type_span,
        body,
        span,
    })
}
#[derive(Clone, Copy)]
pub(super) struct FunctionEvidenceMode {
    pub callable: super::CallableLimits,
    pub expressions: Option<super::ExpressionEvidenceLimits>,
    pub bindings: Option<super::BindingEvidenceLimits>,
}
pub(super) struct FunctionCheckResult {
    pub function: CheckedEnumFunction,
    pub evidence: Option<super::CallableEvidence>,
    pub types: Option<super::TypedCallableEvidence>,
    pub bindings: Option<super::ScopedCallableEvidence>,
    pub signature_usage: (usize, usize),
}
impl CheckedEnumTypes {
    /// Check exactly one source function against this environment's canonical types.
    /// Locals come from source, not supplied metadata. Only standard calls are
    /// available; constructors, user calls, nested matches and execution remain open.
    pub fn check_function_source(
        &self,
        source_label: &str,
        source: &str,
        limits: EnumFunctionLimits,
    ) -> Result<CheckedEnumFunction, EnumFunctionError> {
        self.check_function_source_mode(source_label, source, limits, None)
            .map(|result| result.function)
    }
    pub(super) fn check_function_source_mode(
        &self,
        source_label: &str,
        source: &str,
        limits: EnumFunctionLimits,
        callable: Option<FunctionEvidenceMode>,
    ) -> Result<FunctionCheckResult, EnumFunctionError> {
        if source_label.len() > limits.max_source_label_bytes {
            return Err(EnumFunctionError::Limit("source label"));
        }
        let syntax = parse(source, limits)?;
        self.check_function_syntax_mode(source_label, syntax, limits, callable)
    }
    pub(super) fn check_function_syntax_mode(
        &self,
        source_label: &str,
        syntax: EnumFunctionSyntax,
        limits: EnumFunctionLimits,
        callable: Option<FunctionEvidenceMode>,
    ) -> Result<FunctionCheckResult, EnumFunctionError> {
        self.check_function_syntax_program_mode(source_label, syntax, limits, callable, None)
    }
    pub(super) fn check_function_syntax_program_mode(
        &self,
        source_label: &str,
        syntax: EnumFunctionSyntax,
        limits: EnumFunctionLimits,
        callable: Option<FunctionEvidenceMode>,
        program: Option<std::sync::Arc<super::program_source::ProgramHeaders>>,
    ) -> Result<FunctionCheckResult, EnumFunctionError> {
        if source_label.len() > limits.max_source_label_bytes {
            return Err(EnumFunctionError::Limit("source label"));
        }
        let canonical_name = program
            .as_ref()
            .and_then(|p| p.functions.iter().find(|(_, f)| f.span == syntax.span))
            .map_or(syntax.name.as_str(), |(_, f)| f.name.as_str());
        if crate::compiler::stdlib::lookup(canonical_name).is_some() {
            return Err(EnumFunctionError::Diagnostics(diagnostic(
                "E3016",
                "function conflicts with a builtin",
                syntax.span,
            )));
        }
        let known = self.body_named_types();
        let resolve =
            |name: &str| Type::from_name_with_known(name, Some(known)).canonicalize(known);
        let mut scope = DetachedScope::default();
        for parameter in &syntax.parameters {
            if scope.get(&parameter.name).is_some() {
                return Err(EnumFunctionError::Diagnostics(diagnostic(
                    "E3011",
                    "duplicate parameter",
                    parameter.span,
                )));
            }
            let Some(annotation) = &parameter.type_name else {
                return Err(EnumFunctionError::Diagnostics(diagnostic(
                    "E3014",
                    "parameter requires an explicit annotation",
                    parameter.span,
                )));
            };
            let ty = resolve(annotation);
            if ty == Type::Unknown {
                return Err(EnumFunctionError::Diagnostics(diagnostic(
                    "E3017",
                    "unknown parameter type",
                    parameter.type_span.unwrap_or(parameter.span),
                )));
            }
            scope.insert(&parameter.name, ty);
        }
        let expected = resolve(syntax.return_type.as_deref().unwrap_or("Unit"));
        if expected == Type::Unknown {
            return Err(EnumFunctionError::Diagnostics(diagnostic(
                "E3017",
                "unknown return type",
                syntax.return_type_span.unwrap_or(syntax.span),
            )));
        }
        if syntax.name == "main" && self.module().is_none() {
            if !syntax.parameters.is_empty() {
                return Err(EnumFunctionError::Diagnostics(diagnostic(
                    "E3009",
                    "entry function main cannot declare parameters",
                    syntax.span,
                )));
            }
            if expected != Type::Unit {
                return Err(EnumFunctionError::Diagnostics(diagnostic(
                    "E3010",
                    "entry function main must return Unit",
                    syntax.span,
                )));
            }
        }
        let mut signatures = callable
            .map(|mode| {
                super::callable::signatures(self, &syntax, mode.callable, program.as_deref())
            })
            .transpose()?;
        let mut recorder = crate::compiler::checked::Recorder::default();
        if let (Some(mode), Some(signatures)) = (callable, signatures.as_mut()) {
            let mut calls = super::callable::CallRecorder::new(
                mode.callable,
                std::mem::take(&mut signatures.targets),
            );
            if let Some(types) = mode.expressions {
                calls.enable_types(types)?;
            }
            if let Some(bindings) = mode.bindings {
                calls.enable_scopes(bindings);
            }
            recorder.detached(calls);
        }
        for parameter in &syntax.parameters {
            recorder.detached_declare(
                &parameter.name,
                parameter.span,
                super::ScopedDeclarationKind::Parameter,
                scope.get(&parameter.name).expect("validated parameter"),
                known,
            );
        }
        let functions: std::collections::HashMap<_, _> = signatures
            .as_ref()
            .map(|s| s.functions.iter().map(|f| (f.name.clone(), f)).collect())
            .unwrap_or_default();
        let mut fields = self.body_record_fields().clone();
        semantic::add_standard_record_fields(&mut fields);
        let mut returns = false;
        let mut matches = Vec::new();
        let mut copies = 0usize;
        for statement in &syntax.body {
            match statement {
                EnumFunctionStatement::Core(statement) => {
                    returns |= scope
                        .check_recorded(
                            statement,
                            &expected,
                            known,
                            &fields,
                            &functions,
                            &mut recorder,
                        )
                        .map_err(EnumFunctionError::Diagnostics)?;
                }
                EnumFunctionStatement::RecursiveMatch(_) | EnumFunctionStatement::Control(_) => {
                    let mut checker = super::recursive_check::RecursiveChecker {
                        env: self,
                        label: source_label,
                        expected: &expected,
                        known,
                        fields: &fields,
                        functions: &functions,
                        recorder: &mut recorder,
                        plans: &mut matches,
                        copies: &mut copies,
                        limits,
                    };
                    returns |= checker.statement(statement, &mut scope)?;
                }
                EnumFunctionStatement::Match(syntax) => {
                    let Some(Type::Named(name)) = scope.get(&syntax.scrutinee) else {
                        return Err(EnumFunctionError::Diagnostics(diagnostic(
                            "E3108",
                            "match scrutinee must be a bound enum",
                            syntax.scrutinee_span,
                        )));
                    };
                    let Some(id) = self.find(name) else {
                        return Err(EnumFunctionError::Diagnostics(diagnostic(
                            "E3108",
                            "match scrutinee is not an enum",
                            syntax.scrutinee_span,
                        )));
                    };
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
                    let plan = self
                        .check_match(id, source_label, syntax.span, &arms)
                        .map_err(EnumFunctionError::Coverage)?;
                    let next = scope
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
                        .and_then(|n| n.checked_add(copies))
                        .ok_or(EnumFunctionError::Limit("scope bindings"))?;
                    if next > limits.max_scope_bindings {
                        return Err(EnumFunctionError::Limit("scope bindings"));
                    }
                    copies = next;
                    recorder.detached_scrutinee(
                        &syntax.scrutinee,
                        syntax.scrutinee_span,
                        matches.len(),
                    );
                    let declaration = &self.declarations()[id.ordinal()];
                    let mut all_return = true;
                    for (arm, variant) in syntax.arms.iter().zip(plan.variant_order()) {
                        let payload = declaration.variants[*variant]
                            .payload
                            .map(|id| self.body_payload_type(id))
                            .transpose()
                            .map_err(EnumFunctionError::Coverage)?;
                        recorder.enter_scope();
                        let mut bindings: Vec<_> = scope.bindings().collect();
                        if let (Some(name), Some(ty)) = (&arm.binding, &payload) {
                            recorder.detached_declare(
                                name,
                                arm.binding_span.unwrap_or(arm.span),
                                super::ScopedDeclarationKind::Payload {
                                    match_index: matches.len(),
                                    variant: *variant,
                                },
                                ty,
                                known,
                            );
                            bindings.push(DetachedBinding {
                                name,
                                ty,
                                mutable: false,
                            });
                        }
                        all_return &= semantic::check_detached_body_recorded(
                            &arm.body,
                            &bindings,
                            &expected,
                            known,
                            &fields,
                            &functions,
                            &mut recorder,
                        )
                        .map_err(EnumFunctionError::Diagnostics)?;
                        recorder.exit_scope();
                    }
                    returns |= all_return;
                    matches.push(plan);
                }
            }
        }
        if expected != Type::Unit && !returns {
            return Err(EnumFunctionError::Diagnostics(diagnostic(
                "E3013",
                "function must return its declared type on every path",
                syntax.span,
            )));
        }
        let recorded = recorder.finish_detached()?;
        let (evidence, types, bindings) = match recorded {
            Some(recorded) => (Some(recorded.evidence), recorded.types, recorded.bindings),
            None => (None, None, None),
        };
        let signature_usage = signatures
            .as_ref()
            .map_or((0, 0), |s| (s.functions.len(), s.text_bytes));
        Ok(FunctionCheckResult {
            function: CheckedEnumFunction {
                syntax,
                environment: self.clone(),
                matches,
                returns,
                program,
            },
            evidence,
            types,
            bindings,
            signature_usage,
        })
    }
}
