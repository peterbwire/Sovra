//! Opt-in single-entry source-unit integration with original token coordinates.
use super::*;
use crate::compiler::{
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    lexer::{Lexer, Token, TokenKind},
    parser::Parser,
    resources::FrontendLimits,
    sum_syntax::{parse_enum_tokens, EnumSyntaxError},
};
use std::sync::Arc;

/// Input/type/function admission for one complete supported source unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntrySourceLimits {
    /// Whole-file byte/token/diagnostic admission, before declaration projection.
    pub frontend: FrontendLimits,
    /// Top-level declarations, admitted before retaining each projection.
    pub max_declarations: usize,
    /// Opaque source-label bytes, checked before retention.
    pub max_source_label_bytes: usize,
    /// Aggregate enum fragments and retained type-environment quotas.
    pub types: EnumTypeLimits,
    /// Single-entry source/call/type/scope quotas; its fragment caps also apply.
    pub callable: ScopedCallableLimits,
}
impl EntrySourceLimits {
    /// Explicit finite profile; default parsers and CLI remain unchanged.
    pub const DEFAULT: Self = Self {
        frontend: FrontendLimits::DEFAULT,
        max_declarations: 4096,
        max_source_label_bytes: 256 * 1024,
        types: EnumTypeLimits::DEFAULT,
        callable: ScopedCallableLimits::DEFAULT,
    };
}
/// Bounded projections and lowering performed before starting VM execution control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryLoweringLimits {
    /// Structural syntax/fact mapping admission.
    pub mapping: MappingLimits,
    /// Expression register emission/validation admission.
    pub expressions: ExpressionIrLimits,
    /// Function instruction emission/flow validation admission.
    pub function: FunctionIrLimits,
}
impl EntryLoweringLimits {
    /// Existing opt-in layer defaults; no unlimited fallback.
    pub const DEFAULT: Self = Self {
        mapping: MappingLimits::DEFAULT,
        expressions: ExpressionIrLimits::DEFAULT,
        function: FunctionIrLimits::DEFAULT,
    };
}
/// Complete supported top-level declaration coverage, in original source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryDeclarationKind {
    /// Nominal enum declaration.
    Enum,
    /// Ordinary record declaration.
    Record,
    /// Ordinary type alias declaration.
    Alias,
    /// An inline type-only module container.
    Module,
    /// A supported root executable function.
    Function,
}
/// Declaration coordinates in the retained original source, including export intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryDeclaration {
    /// Supported declaration category.
    pub kind: EntryDeclarationKind,
    /// Original token range, excluding surrounding trivia.
    pub span: Span,
}
/// No partial checked unit is returned on any refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntrySourceError {
    /// Whole-source lexer, nesting or core-context parsing failed.
    Syntax(Diagnostics),
    /// Enum grammar failed, with original file coordinates.
    EnumSyntax(EnumSyntaxError),
    /// Nominal type/header validation failed.
    Types(EnumTypeError),
    /// Single-entry body/type/evidence validation failed.
    Function(EnumFunctionError),
    /// An explicit admission dimension was exhausted.
    Limit(&'static str),
    /// The source contains no entry function.
    MissingEntry,
    /// Additional functions or unsupported top-level syntax are not ignored.
    Unsupported {
        /// Unsupported category.
        kind: &'static str,
        /// Original first unsupported token.
        span: Span,
    },
}
impl std::fmt::Display for EntrySourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entry source refused: {self:?}")
    }
}
impl std::error::Error for EntrySourceError {}
/// Lowering/runtime errors remain distinct from successful source checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryExecutionError {
    /// Structural mapping refused.
    Mapping(MappingError),
    /// Expression emission/validation refused.
    Expression(ExpressionIrError),
    /// Function emission/validation refused.
    Function(FunctionIrError),
    /// VM execution refused without another engine.
    Runtime(RuntimeVmError),
}
impl std::fmt::Display for EntryExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entry execution refused: {self:?}")
    }
}
impl std::error::Error for EntryExecutionError {}
/// Source-owned checked one-entry unit. No external enum/type metadata is trusted.
#[derive(Debug)]
pub struct CheckedEntrySource {
    source: Arc<str>,
    label: Arc<str>,
    declarations: Vec<EntryDeclaration>,
    entry: CheckedScopedCallableFunction,
}
impl CheckedEntrySource {
    /// Original admitted UTF-8 source, never whitespace-masked or reconstructed.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Opaque source label; not a canonical path or authority.
    pub fn source_label(&self) -> &str {
        &self.label
    }
    /// Full supported declaration coverage in source order.
    pub fn declarations(&self) -> &[EntryDeclaration] {
        &self.declarations
    }
    /// Checked entry with original source-owned scopes, expressions and call targets.
    pub fn entry(&self) -> &CheckedScopedCallableFunction {
        &self.entry
    }
    fn with_ir<T>(
        &self,
        limits: EntryLoweringLimits,
        run: impl FnOnce(&FunctionIr<'_>) -> Result<T, RuntimeVmError>,
    ) -> Result<T, EntryExecutionError> {
        let mapping = self
            .entry
            .map_for_lowering(limits.mapping)
            .map_err(EntryExecutionError::Mapping)?;
        let expressions = mapping
            .emit_expression_ir(limits.expressions)
            .map_err(EntryExecutionError::Expression)?;
        let function = expressions
            .emit_function_ir(limits.function)
            .map_err(EntryExecutionError::Function)?;
        run(&function).map_err(EntryExecutionError::Runtime)
    }
    /// Lower and execute the entry, retaining the VM's pure/memory-only default boundary.
    /// Lowering precedes the VM clock; logical compilation limits are separate.
    pub fn execute(
        &self,
        arguments: &[RuntimeValueV2],
        lowering: EntryLoweringLimits,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmOutcome, EntryExecutionError> {
        self.with_ir(lowering, |ir| ir.execute(arguments, policy))
    }
    /// Lower and execute with explicit program arguments and independently granted services.
    /// No default filesystem/input/output host or CLI integration is installed.
    pub fn execute_with_services<'a>(
        &self,
        arguments: &[RuntimeValueV2],
        program_arguments: &'a [String],
        services: RuntimeVmServices<'a>,
        lowering: EntryLoweringLimits,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmCommandOutcome, EntryExecutionError> {
        self.with_ir(lowering, |ir| {
            ir.execute_with_services(arguments, program_arguments, services, policy)
        })
    }
}
fn syntax(code: &'static str, message: &'static str, span: Span) -> EntrySourceError {
    EntrySourceError::Syntax(Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
        }],
    })
}
fn fragment(tokens: &[Token], start: usize, end: usize) -> Vec<Token> {
    let mut result = tokens[start..end].to_vec();
    result.push(tokens.last().expect("lexer supplies EOF").clone());
    result
}
/// Check a complete root source with enums, aliases, records and exactly one function.
/// Additional functions, module functions, nested modules, imports and nested matches
/// remain explicit refusals; type-only inline modules are admitted.
/// Tokens are lexed once; disjoint projections retain original byte/line/column spans.
pub fn check_entry_source(
    label: &str,
    source: &str,
    limits: EntrySourceLimits,
) -> Result<CheckedEntrySource, EntrySourceError> {
    let parts = parse_source_parts(label, source, limits, 1, false, false)?;
    let entry = parts
        .environment
        .check_scoped_callable_function_syntax(
            label,
            parts
                .functions
                .into_iter()
                .next()
                .ok_or(EntrySourceError::MissingEntry)?,
            limits.callable,
        )
        .map_err(EntrySourceError::Function)?;
    Ok(CheckedEntrySource {
        source: Arc::from(source),
        label: Arc::from(label),
        declarations: parts.declarations,
        entry,
    })
}
pub(super) struct SourceParts {
    pub function_modules: Vec<Option<Arc<str>>>,
    pub declarations: Vec<EntryDeclaration>,
    pub environment: CheckedEnumTypes,
    pub functions: Vec<EnumFunctionSyntax>,
}
/// Owned untyped v2 root syntax; not type/visibility/evidence validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProgramSyntax {
    pub(crate) declarations: Vec<EntryDeclaration>,
    pub(crate) core: crate::compiler::ast::Program,
    pub(crate) enums: Vec<crate::compiler::sum_syntax::EnumDeclarationSyntax>,
    pub(crate) enum_modules: Vec<Option<Arc<str>>>,
    pub(crate) functions: Vec<EnumFunctionSyntax>,
    pub(crate) function_modules: Vec<Option<Arc<str>>>,
}
impl ParsedProgramSyntax {
    /// Complete original root declaration ranges, in source order.
    pub fn declarations(&self) -> &[EntryDeclaration] {
        &self.declarations
    }
    /// Core alias/record headers only; never a projection of executable v2 bodies.
    pub fn core_declarations(&self) -> &crate::compiler::ast::Program {
        &self.core
    }
    /// Original enum syntax; names/payloads remain unresolved.
    pub fn enums(&self) -> &[crate::compiler::sum_syntax::EnumDeclarationSyntax] {
        &self.enums
    }
    /// Declaration-module paths parallel to `enums`; None denotes the root.
    pub fn enum_modules(&self) -> impl Iterator<Item = Option<&str>> {
        self.enum_modules.iter().map(|module| module.as_deref())
    }
    /// Declaration-module paths parallel to `functions`; None denotes the root.
    pub fn function_modules(&self) -> impl Iterator<Item = Option<&str>> {
        self.function_modules.iter().map(|module| module.as_deref())
    }
    /// Owned syntax-only bodies with original coordinates; see `function_modules`.
    pub fn functions(&self) -> &[EnumFunctionSyntax] {
        &self.functions
    }
}
/// Parse supported root syntax without requiring main/functions or type validity.
/// Retains root and single-level module function bodies without checking them.
/// Semantic source checking still refuses module functions; nested modules/imports
/// remain unavailable even to this syntax-only API.
pub fn parse_program_syntax(
    source: &str,
    limits: EntrySourceLimits,
    max_functions: usize,
) -> Result<ParsedProgramSyntax, EntrySourceError> {
    parse_syntax_parts(source, limits, max_functions, true, false, true)
}
pub(super) fn parse_source_parts(
    label: &str,
    source: &str,
    limits: EntrySourceLimits,
    max_functions: usize,
    recursive_matches: bool,
    module_functions: bool,
) -> Result<SourceParts, EntrySourceError> {
    if label.len() > limits.max_source_label_bytes {
        return Err(EntrySourceError::Limit("source label"));
    }
    let syntax = parse_syntax_parts(
        source,
        limits,
        max_functions,
        recursive_matches,
        true,
        module_functions,
    )?;
    if syntax.functions.is_empty() {
        return Err(EntrySourceError::MissingEntry);
    }
    let mut sources = Vec::new();
    let mut enum_labels = Vec::new();
    for declaration in &syntax.declarations {
        if declaration.kind == EntryDeclarationKind::Enum {
            sources.push(&source[declaration.span.start..declaration.span.end]);
            enum_labels.push(format!("enum:{}", enum_labels.len()));
        }
    }
    let fragments: Vec<_> = sources
        .iter()
        .zip(&enum_labels)
        .zip(&syntax.enum_modules)
        .map(|((source, label), module)| ScopedEnumSource {
            module: module.as_deref(),
            declaration: EnumSource { label, source },
        })
        .collect();
    let environment = super::enum_types::check_scoped_enum_types_parsed(
        label,
        label,
        &syntax.core,
        &fragments,
        limits.types,
        Some(syntax.enums),
    )
    .map_err(EntrySourceError::Types)?;
    Ok(SourceParts {
        function_modules: syntax.function_modules,
        environment,
        declarations: syntax.declarations,
        functions: syntax.functions,
    })
}
fn parse_syntax_parts(
    source: &str,
    limits: EntrySourceLimits,
    max_functions: usize,
    recursive_matches: bool,
    require_function: bool,
    allow_module_functions: bool,
) -> Result<ParsedProgramSyntax, EntrySourceError> {
    let tokens = Lexer::new()
        .tokenize_with_limits(source, limits.frontend)
        .map_err(EntrySourceError::Syntax)?;
    if let Err(span) = crate::compiler::limits::check_nesting(&tokens, true) {
        return Err(syntax("E2007", "maximum structural depth exceeded", span));
    }
    let mut projection = SyntaxProjection {
        tokens: &tokens,
        limits,
        max_functions,
        recursive_matches,
        allow_module_functions,
        declarations: Vec::new(),
        enums: Vec::new(),
        enum_modules: Vec::new(),
        context: Vec::new(),
        functions: Vec::new(),
        function_modules: Vec::new(),
        enum_bytes: 0,
        module_scope_bytes: 0,
    };
    let mut cursor = 0;
    projection.sequence(&mut cursor, None)?;
    let SyntaxProjection {
        declarations,
        enums,
        enum_modules,
        mut context,
        functions,
        function_modules,
        ..
    } = projection;
    if require_function && functions.is_empty() {
        return Err(EntrySourceError::MissingEntry);
    }
    context.push(tokens.last().expect("lexer supplies EOF").clone());
    let context = Parser::new()
        .parse_tokens_inner(&context, limits.frontend.max_diagnostics.max(1))
        .map_err(EntrySourceError::Syntax)?;
    Ok(ParsedProgramSyntax {
        declarations,
        core: context,
        enums,
        enum_modules,
        functions,
        function_modules,
    })
}

struct SyntaxProjection<'a> {
    tokens: &'a [Token],
    limits: EntrySourceLimits,
    max_functions: usize,
    recursive_matches: bool,
    allow_module_functions: bool,
    declarations: Vec<EntryDeclaration>,
    enums: Vec<crate::compiler::sum_syntax::EnumDeclarationSyntax>,
    enum_modules: Vec<Option<Arc<str>>>,
    context: Vec<Token>,
    functions: Vec<EnumFunctionSyntax>,
    function_modules: Vec<Option<Arc<str>>>,
    enum_bytes: usize,
    module_scope_bytes: usize,
}
impl SyntaxProjection<'_> {
    fn sequence(
        &mut self,
        cursor: &mut usize,
        module: Option<Arc<str>>,
    ) -> Result<(), EntrySourceError> {
        while self.tokens[*cursor].kind != TokenKind::Eof
            && !(module.is_some() && self.tokens[*cursor].kind == TokenKind::Punctuation('}'))
        {
            if self.declarations.len() >= self.limits.max_declarations {
                return Err(EntrySourceError::Limit("declarations"));
            }
            let start = *cursor;
            let head = if self.tokens[*cursor].kind == TokenKind::Keyword("export") {
                *cursor + 1
            } else {
                *cursor
            };
            if self.tokens[head].kind == TokenKind::Keyword("mod") {
                if module.is_some() || head != start {
                    return Err(EntrySourceError::Unsupported {
                        kind: "nested or exported type module",
                        span: self.tokens[head].span,
                    });
                }
                let name = match self.tokens.get(head + 1).map(|t| &t.kind) {
                    Some(TokenKind::Identifier(name)) => name.as_str(),
                    _ => {
                        return Err(syntax(
                            "E2002",
                            "expected module name",
                            self.tokens[head].span,
                        ))
                    }
                };
                if self.tokens.get(head + 2).map(|t| &t.kind) != Some(&TokenKind::Punctuation('{'))
                {
                    return Err(syntax(
                        "E2005",
                        "expected module body",
                        self.tokens[head].span,
                    ));
                }
                // Admit qualifier expansion before type-scope resolution can copy a
                // long module prefix into many declarations/field annotations.
                let mut depth = 1usize;
                let mut identifiers = 1usize;
                for token in &self.tokens[head + 3..] {
                    match token.kind {
                        TokenKind::Punctuation('{') => depth += 1,
                        TokenKind::Punctuation('}') => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        TokenKind::Identifier(_) => {
                            identifiers = identifiers
                                .checked_add(1)
                                .ok_or(EntrySourceError::Limit("module qualifier bytes"))?;
                        }
                        TokenKind::Eof => break,
                        _ => {}
                    }
                }
                self.module_scope_bytes = identifiers
                    .checked_mul(name.len())
                    .and_then(|amount| self.module_scope_bytes.checked_add(amount))
                    .filter(|amount| *amount <= self.limits.types.max_text_bytes)
                    .ok_or(EntrySourceError::Limit("module qualifier bytes"))?;
                let name = Arc::<str>::from(name);
                let index = self.declarations.len();
                self.declarations.push(EntryDeclaration {
                    kind: EntryDeclarationKind::Module,
                    span: self.tokens[start].span,
                });
                self.context
                    .extend_from_slice(&self.tokens[start..head + 3]);
                *cursor = head + 3;
                // Only one module level is admitted, so recursive descent is bounded.
                self.sequence(cursor, Some(name))?;
                let close = self.tokens[*cursor].clone();
                self.context.push(close.clone());
                self.declarations[index].span.end = close.span.end;
                *cursor += 1;
                continue;
            }
            let Self {
                tokens,
                limits,
                max_functions,
                recursive_matches,
                allow_module_functions,
                declarations,
                enums,
                enum_modules,
                context,
                functions,
                function_modules,
                enum_bytes,
                ..
            } = self;
            let kind = match &tokens[head].kind {
                TokenKind::Identifier(name) if name == "enum" => EntryDeclarationKind::Enum,
                TokenKind::Keyword("struct") => EntryDeclarationKind::Record,
                TokenKind::Keyword("type") => EntryDeclarationKind::Alias,
                TokenKind::Keyword("fn") => EntryDeclarationKind::Function,
                _ => {
                    return Err(EntrySourceError::Unsupported {
                        kind: "top-level declaration",
                        span: tokens[head].span,
                    })
                }
            };
            if kind == EntryDeclarationKind::Function
                && module.is_some()
                && !*allow_module_functions
            {
                return Err(EntrySourceError::Unsupported {
                    kind: "function in type-only module",
                    span: tokens[head].span,
                });
            }
            if kind == EntryDeclarationKind::Function && functions.len() >= *max_functions {
                return Err(EntrySourceError::Unsupported {
                    kind: "additional function",
                    span: tokens[head].span,
                });
            }
            *cursor = head + 1;
            if kind == EntryDeclarationKind::Alias {
                while !matches!(
                    tokens[*cursor].kind,
                    TokenKind::Punctuation(';') | TokenKind::Eof
                ) {
                    *cursor += 1;
                }
                if tokens[*cursor].kind == TokenKind::Eof {
                    return Err(syntax(
                        "E2005",
                        "expected alias terminator",
                        tokens[*cursor].span,
                    ));
                }
                *cursor += 1;
            } else {
                while !matches!(
                    tokens[*cursor].kind,
                    TokenKind::Punctuation('{') | TokenKind::Eof
                ) {
                    *cursor += 1;
                }
                if tokens[*cursor].kind == TokenKind::Eof {
                    return Err(syntax(
                        "E2005",
                        "expected declaration body",
                        tokens[*cursor].span,
                    ));
                }
                let mut depth = 1;
                *cursor += 1;
                while depth > 0 {
                    match tokens[*cursor].kind {
                        TokenKind::Punctuation('{') => depth += 1,
                        TokenKind::Punctuation('}') => depth -= 1,
                        TokenKind::Eof => {
                            return Err(syntax(
                                "E2005",
                                "expected closing declaration brace",
                                tokens[*cursor].span,
                            ))
                        }
                        _ => {}
                    }
                    *cursor += 1;
                }
            }
            let span = Span {
                end: tokens[*cursor - 1].span.end,
                ..tokens[start].span
            };
            declarations.push(EntryDeclaration { kind, span });
            match kind {
                EntryDeclarationKind::Enum => {
                    if enums.len() >= limits.types.max_enums {
                        return Err(EntrySourceError::Limit("enums"));
                    }
                    let bytes = span.end - span.start;
                    *enum_bytes = enum_bytes
                        .checked_add(bytes)
                        .filter(|n| *n <= limits.types.max_source_bytes)
                        .ok_or(EntrySourceError::Limit("enum source bytes"))?;
                    let syntax = parse_enum_tokens(
                        &fragment(tokens, start, *cursor),
                        bytes,
                        limits.types.syntax,
                    )
                    .map_err(EntrySourceError::EnumSyntax)?;
                    enums.push(syntax);
                    enum_modules.push(module.clone());
                }
                EntryDeclarationKind::Function => {
                    let cap = limits.callable.typed.callable.function.frontend;
                    if span.end - span.start > cap.max_source_bytes {
                        return Err(EntrySourceError::Limit("function source bytes"));
                    }
                    if *cursor - start > cap.max_tokens {
                        return Err(EntrySourceError::Limit("function tokens"));
                    }
                    functions.push(
                        if *recursive_matches {
                            super::recursive_match::parse_program_function(
                                &fragment(tokens, start, *cursor),
                                limits.callable.typed.callable.function,
                            )
                        } else {
                            super::function::parse_tokens(
                                &fragment(tokens, start, *cursor),
                                limits.callable.typed.callable.function,
                            )
                        }
                        .map_err(EntrySourceError::Function)?,
                    );
                    function_modules.push(module.clone());
                }
                _ => context.extend_from_slice(&tokens[start..*cursor]),
            }
        }

        if module.is_some() && self.tokens[*cursor].kind == TokenKind::Eof {
            return Err(syntax(
                "E2005",
                "expected closing module brace",
                self.tokens[*cursor].span,
            ));
        }
        Ok(())
    }
}
