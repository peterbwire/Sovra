//! Complete bounded standalone source collection through checked generic IR execution.
use super::*;
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, TokenKind},
    resources::FrontendLimits,
};
use std::sync::Arc;
/// Whole-file and cumulative declaration syntax admission, before semantic stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSourceSyntaxLimits {
    /// Original whole-file lexer/source/diagnostic admission; token ceiling excludes EOF.
    pub frontend: FrontendLimits,
    /// Declarations and retained origin records.
    pub max_declarations: usize,
    /// Per-declaration original bytes before its parser is called.
    pub max_declaration_bytes: usize,
    /// Whole-file source plus every declaration parser's retained source/fragments.
    pub max_retained_source_bytes: usize,
    /// Whole-file lexer tokens plus whole-declaration parse tokens, each including EOF.
    pub max_tokens: usize,
    /// Aggregate copied declaration/header/type names plus selected entry name.
    pub max_name_bytes: usize,
    /// Aggregate explicit type-parameter records.
    pub max_parameters: usize,
    /// Aggregate record fields, enum variants and value parameters.
    pub max_members: usize,
    /// Aggregate annotation nodes.
    pub max_type_nodes: usize,
    /// Aggregate annotation edges.
    pub max_type_edges: usize,
    /// Aggregate annotation parameter uses.
    pub max_parameter_uses: usize,
    /// Source-byte/token/metadata scanning, copying and entry lookup work.
    pub max_work: usize,
    /// Declaration body brace nesting ceiling, in 1..=128.
    pub max_depth: usize,
    /// Selected entry name bytes.
    pub max_entry_bytes: usize,
    /// Independent alias syntax profile, also capped by remaining cumulative allowances.
    pub alias: GenericAliasLimits,
    /// Independent record syntax profile.
    pub record: GenericRecordLimits,
    /// Independent enum syntax profile.
    pub enumeration: GenericEnumLimits,
    /// Independent generic/plain function syntax profile.
    pub function: GenericFunctionLimits,
}
impl GenericSourceSyntaxLimits {
    /// Finite additive single-file profile; older Program/CLI syntax remains separate.
    pub const DEFAULT: Self = Self {
        frontend: FrontendLimits {
            max_source_bytes: 4 * 1024 * 1024,
            max_tokens: 262_144,
            max_diagnostics: 64,
        },
        max_declarations: 512,
        max_declaration_bytes: 128 * 1024,
        max_retained_source_bytes: 16 * 1024 * 1024,
        max_tokens: 1_048_576,
        max_name_bytes: 512 * 1024,
        max_parameters: 8192,
        max_members: 16_384,
        max_type_nodes: 65_536,
        max_type_edges: 131_072,
        max_parameter_uses: 65_536,
        max_work: 32_000_000,
        max_depth: 128,
        max_entry_bytes: 1024,
        alias: GenericAliasLimits::DEFAULT,
        record: GenericRecordLimits::DEFAULT,
        enumeration: GenericEnumLimits::DEFAULT,
        function: GenericFunctionLimits::DEFAULT,
    };
}
/// Explicit cumulative stage profiles for one complete standalone compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSourceLimits {
    /// Whole source/declaration syntax.
    pub syntax: GenericSourceSyntaxLimits,
    /// Whole type registry.
    pub registry: GenericRegistryLimits,
    /// Whole alias environment.
    pub aliases: GenericAliasEnvironmentLimits,
    /// Every type definition, including unused definitions.
    pub declarations: GenericDeclarationLimits,
    /// Every function signature, including unused functions.
    pub signatures: GenericSignatureLimits,
    /// Every symbolic function body, including unreachable statements.
    pub bodies: GenericCopyBodyLimits,
    /// Reachable concrete function closure.
    pub specialization: GenericSpecializationLimits,
    /// Whole concrete body/context rechecking.
    pub concrete: GenericConcreteBodyLimits,
    /// Whole concrete literal decoding.
    pub literals: GenericLiteralLimits,
    /// Combined function/type closure and layouts.
    pub layouts: GenericLayoutLimits,
    /// Whole nominal operation planning.
    pub construction: GenericConstructionLimits,
    /// Whole instruction emission/validation.
    pub ir: GenericIrLimits,
}
impl GenericSourceLimits {
    /// Reuse finite approved profiles at every stage; no exhaustion fallback.
    pub const DEFAULT: Self = Self {
        syntax: GenericSourceSyntaxLimits::DEFAULT,
        registry: GenericRegistryLimits::DEFAULT,
        aliases: GenericAliasEnvironmentLimits::DEFAULT,
        declarations: GenericDeclarationLimits::DEFAULT,
        signatures: GenericSignatureLimits::DEFAULT,
        bodies: GenericCopyBodyLimits::DEFAULT,
        specialization: GenericSpecializationLimits::DEFAULT,
        concrete: GenericConcreteBodyLimits::DEFAULT,
        literals: GenericLiteralLimits::DEFAULT,
        layouts: GenericLayoutLimits::DEFAULT,
        construction: GenericConstructionLimits::DEFAULT,
        ir: GenericIrLimits::DEFAULT,
    };
}
/// Original parsed declaration kind; inner sources are unchanged declaration fragments.
#[derive(Debug, Clone)]
pub enum GenericSourceDeclarationKind {
    /// Explicit generic alias.
    Alias(GenericAliasSyntax),
    /// Explicit generic record.
    Record(GenericRecordSyntax),
    /// Explicit generic enum.
    Enum(GenericEnumSyntax),
    /// Generic function or nongeneric function using the bounded root adapter.
    Function(GenericFunctionSyntax),
}
/// Original whole-file declaration origin and complete admitted syntax.
#[derive(Debug, Clone)]
pub struct GenericSourceDeclaration {
    span: Span,
    kind: GenericSourceDeclarationKind,
}
impl GenericSourceDeclaration {
    /// Exact original declaration range, including an export prefix when admitted.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Original source fragment syntax, never rewritten or token-filtered.
    pub fn kind(&self) -> &GenericSourceDeclarationKind {
        &self.kind
    }
    /// Original declaration name.
    pub fn name(&self) -> &str {
        match &self.kind {
            GenericSourceDeclarationKind::Alias(v) => v.name(),
            GenericSourceDeclarationKind::Record(v) => v.name(),
            GenericSourceDeclarationKind::Enum(v) => v.name(),
            GenericSourceDeclarationKind::Function(v) => v.name(),
        }
    }
}
/// Explicit source and independent propagation-stage admission profiles.
#[derive(Debug, Clone, Copy)]
pub struct GenericPropagationSourceLimits {
    /// Existing complete standalone pipeline admission.
    pub source: GenericSourceLimits,
    /// Shared symbolic propagation checks and, separately, concrete rechecking.
    pub propagation: GenericPropagationLimits,
    /// Shared concrete propagation operation planning.
    pub operations: GenericPropagationLayoutLimits,
}
impl GenericPropagationSourceLimits {
    /// Finite library profile; no implicit CLI or native selection.
    pub const DEFAULT: Self = Self {
        source: GenericSourceLimits::DEFAULT,
        propagation: GenericPropagationLimits::DEFAULT,
        operations: GenericPropagationLayoutLimits::DEFAULT,
    };
}
/// Complete compile refusal. Nested stage errors keep their original fragment/alias provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericSourceErrorKind {
    /// Source admission exhausted.
    Limit(&'static str),
    /// Original full-file lexer failure.
    Lexical(Diagnostics),
    /// Declaration collection syntax/unsupported context.
    Syntax(&'static str),
    /// Per-declaration parser failure.
    Declaration(GenericTypeError),
    /// Type registry failure.
    Registry(GenericResolutionError),
    /// Alias preparation failure.
    Alias(GenericAliasExpansionError),
    /// Complete type-definition failure.
    Types(GenericDeclarationError),
    /// Complete signature failure.
    Signatures(GenericSignatureError),
    /// Complete symbolic body failure.
    Bodies(GenericCopyBodyError),
    /// Missing or unsupported selected entry.
    Entry(&'static str),
    /// Concrete closure failure.
    Specialization(GenericSpecializationError),
    /// Concrete context/type recheck failure.
    Concrete(GenericConcreteBodyError),
    /// Literal payload failure.
    Literals(GenericLiteralError),
    /// Type layout failure.
    Layouts(GenericLayoutError),
    /// Propagation operation layout failure.
    PropagationLayouts(GenericPropagationLayoutError),
    /// Nominal operation failure.
    Construction(GenericConstructionError),
    /// Instruction lowering/validation failure.
    Ir(GenericIrError),
}
/// Original whole-file primary location with preserved bounded stage error detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericSourceError {
    /// Original whole-file primary range, when available; nested error spans are not relabeled.
    pub span: Option<Span>,
    /// Exact failed stage and its original error.
    pub kind: Box<GenericSourceErrorKind>,
}
impl std::fmt::Display for GenericSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic source compilation refused: {self:?}")
    }
}
impl std::error::Error for GenericSourceError {}
/// Exact source collection/declaration syntax usage; semantic stages expose their own ledgers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSourceUsage {
    /// Admitted declarations.
    pub declarations: usize,
    /// Full-file and declaration/annotation retained source bytes.
    pub retained_source_bytes: usize,
    /// Full-file plus complete declaration parse tokens including EOFs.
    pub tokens: usize,
    /// Copied header/type/entry names.
    pub name_bytes: usize,
    /// Explicit generic parameters.
    pub parameters: usize,
    /// Fields/variants/value parameters.
    pub members: usize,
    /// Annotation nodes.
    pub type_nodes: usize,
    /// Annotation edges.
    pub type_edges: usize,
    /// Parameter-use records.
    pub parameter_uses: usize,
    /// Source syntax/lookup processing work.
    pub work: usize,
}
/// Complete source-owned standalone compilation, with no partially checked declarations.
#[derive(Debug, Clone)]
pub struct CompiledGenericSource {
    source: Arc<str>,
    declarations: Arc<[GenericSourceDeclaration]>,
    functions: Arc<[usize]>,
    entry: Arc<str>,
    ir: CheckedGenericIr,
    propagation: Option<CheckedGenericPropagationIr>,
    standard_results: bool,
    standard_builtins: bool,
    standard_strings: bool,
    standard_values: bool,
    standard_arrays: bool,
    usage: GenericSourceUsage,
}
impl CompiledGenericSource {
    /// Complete original file including comments/trivia.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// All declarations in original file order, including unused declarations.
    pub fn declarations(&self) -> &[GenericSourceDeclaration] {
        &self.declarations
    }
    /// Selected zero-type/zero-value-argument function name.
    pub fn entry(&self) -> &str {
        &self.entry
    }
    /// Fully checked executable-form instructions; root index is zero.
    pub fn ir(&self) -> &CheckedGenericIr {
        &self.ir
    }
    /// Whether this profile admits array append/concat/slice.
    pub fn has_standard_arrays(&self) -> bool {
        self.standard_arrays
    }
    /// Whether this profile admits polymorphic display and array length.
    pub fn has_standard_values(&self) -> bool {
        self.standard_values
    }
    /// Whether this profile additionally admits checked String utility calls.
    pub fn has_standard_strings(&self) -> bool {
        self.standard_strings
    }
    /// Whether this explicit profile admits host-free standard builtin calls.
    pub fn has_standard_builtins(&self) -> bool {
        self.standard_builtins
    }
    /// Whether this profile admits immutable legacy standard result records.
    pub fn has_standard_results(&self) -> bool {
        self.standard_results
    }
    /// Explicit propagation proof, present only for the opt-in source profile.
    pub fn propagation_ir(&self) -> Option<&CheckedGenericPropagationIr> {
        self.propagation.as_ref()
    }
    /// Cumulative syntax admission; each semantic stage also keeps its own complete-program caps.
    pub fn usage(&self) -> GenericSourceUsage {
        self.usage
    }
    pub(super) fn function_declaration(&self, id: GenericSignatureId) -> Option<usize> {
        self.ir
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .signature(id)?;
        let index = *self.functions.get(id.ordinal())?;
        if matches!(
            self.declarations.get(index)?.kind(),
            GenericSourceDeclarationKind::Function(_)
        ) {
            Some(index)
        } else {
            None
        }
    }
    /// Translate an instance's function-local byte range into original file coordinates.
    /// Caller-supplied line/column fields are ignored; invalid/foreign ranges return None.
    pub fn file_span(&self, instance: usize, local: Span) -> Option<Span> {
        let function = self
            .ir
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph()
            .instances()
            .get(instance)?
            .function();
        let declaration = self
            .declarations
            .get(*self.functions.get(function.ordinal())?)?;
        map_span(&self.source, declaration.span, local)
    }
}
/// Mapped runtime refusal; the inner error keeps original function-fragment coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericSourceExecutionError {
    /// Primary range in the original whole source file.
    pub span: Option<Span>,
    /// Original VM error and concrete instance/instruction identity.
    pub error: GenericVmError,
}
impl std::fmt::Display for GenericSourceExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic source execution refused: {self:?}")
    }
}
impl std::error::Error for GenericSourceExecutionError {}
/// Owned source and successful execution result; borrowed value inspection remains VM-owned.
#[derive(Debug)]
pub struct GenericSourceOutcome {
    program: CompiledGenericSource,
    execution: GenericVmOutcome,
}
impl GenericSourceOutcome {
    /// Retained original source and complete compiler proof.
    pub fn program(&self) -> &CompiledGenericSource {
        &self.program
    }
    /// Runtime result, policy remainder and usage.
    pub fn execution(&self) -> &GenericVmOutcome {
        &self.execution
    }
}
fn error(kind: GenericSourceErrorKind, span: Option<Span>) -> GenericSourceError {
    GenericSourceError {
        span,
        kind: Box::new(kind),
    }
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<(), GenericSourceError> {
    *left = left
        .checked_sub(n)
        .ok_or_else(|| error(GenericSourceErrorKind::Limit(name), None))?;
    Ok(())
}
fn map_span(source: &str, origin: Span, local: Span) -> Option<Span> {
    let fragment = source.get(origin.start..origin.end)?;
    fragment.get(local.start..local.end)?;
    let prefix = fragment.get(..local.start)?;
    let lines = prefix.bytes().filter(|b| *b == b'\n').count();
    let column = if lines == 0 {
        origin.column.checked_add(prefix.chars().count())?
    } else {
        prefix.rsplit('\n').next()?.chars().count()
    };
    Some(Span {
        start: origin.start.checked_add(local.start)?,
        end: origin.start.checked_add(local.end)?,
        line: origin.line.checked_add(lines)?,
        column,
    })
}
fn syntax_error(error_value: GenericTypeError, source: &str, origin: Span) -> GenericSourceError {
    let local = match &error_value {
        GenericTypeError::Syntax { span, .. } => Some(*span),
        GenericTypeError::Lexical(d) => d.items.first().map(|d| d.span),
        _ => None,
    };
    error(
        GenericSourceErrorKind::Declaration(error_value),
        local
            .and_then(|s| map_span(source, origin, s))
            .or(Some(origin)),
    )
}
struct Collected {
    source: Arc<str>,
    declarations: Vec<GenericSourceDeclaration>,
    functions: Vec<usize>,
    types: Vec<usize>,
    usage: GenericSourceUsage,
    left: GenericSourceSyntaxLimits,
}
fn collect(
    source: &str,
    entry: &str,
    limits: GenericSourceSyntaxLimits,
    propagation: bool,
) -> Result<Collected, GenericSourceError> {
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(error(
            GenericSourceErrorKind::Limit("source depth profile"),
            None,
        ));
    }
    if entry.len() > limits.max_entry_bytes {
        return Err(error(
            GenericSourceErrorKind::Limit("entry name bytes"),
            None,
        ));
    }
    let mut left = limits;
    charge(&mut left.max_name_bytes, entry.len(), "source name bytes")?;
    charge(
        &mut left.max_retained_source_bytes,
        source.len(),
        "retained source bytes",
    )?;
    charge(&mut left.max_work, source.len(), "source work")?;
    if left.max_tokens == 0 {
        return Err(error(GenericSourceErrorKind::Limit("source tokens"), None));
    }
    let frontend = FrontendLimits {
        max_tokens: limits.frontend.max_tokens.min(left.max_tokens - 1),
        ..limits.frontend
    };
    let tokens = if propagation {
        Lexer::new().tokenize_propagation(source, frontend)
    } else {
        Lexer::new().tokenize_with_limits(source, frontend)
    }
    .map_err(|d| {
        let span = d.items.first().map(|d| d.span);
        error(GenericSourceErrorKind::Lexical(d), span)
    })?;
    charge(&mut left.max_tokens, tokens.len(), "source tokens")?;
    charge(&mut left.max_work, tokens.len(), "source work")?;
    let owned: Arc<str> = Arc::from(source);
    let mut declarations = Vec::new();
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut cursor = 0;
    while tokens[cursor].kind != TokenKind::Eof {
        charge(&mut left.max_declarations, 1, "source declarations")?;
        charge(&mut left.max_work, 1, "source work")?;
        let start = cursor;
        let exported = tokens[cursor].kind == TokenKind::Keyword("export");
        let head = cursor + usize::from(exported);
        let kind = match tokens.get(head).map(|t| &t.kind) {
            Some(TokenKind::Keyword("type")) if !exported => 0,
            Some(TokenKind::Keyword("struct")) => 1,
            Some(TokenKind::Identifier(name)) if name == "enum" => 2,
            Some(TokenKind::Keyword("fn")) => 3,
            _ => {
                return Err(error(
                    GenericSourceErrorKind::Syntax(
                        "expected an admitted alias, record, enum or function declaration",
                    ),
                    Some(tokens[cursor].span),
                ))
            }
        };
        let mut opened = false;
        let mut depth = 0usize;
        let mut end = None;
        while cursor < tokens.len() - 1 {
            charge(&mut left.max_work, 1, "source work")?;
            let token = &tokens[cursor];
            if kind == 0 {
                if token.kind == TokenKind::Punctuation(';') {
                    end = Some(cursor + 1);
                    break;
                }
                if token.kind == TokenKind::Punctuation('{') {
                    break;
                }
            } else {
                match token.kind {
                    TokenKind::Punctuation('{') => {
                        opened = true;
                        depth += 1;
                        if depth > limits.max_depth {
                            return Err(error(
                                GenericSourceErrorKind::Limit("source body depth"),
                                Some(token.span),
                            ));
                        }
                    }
                    TokenKind::Punctuation('}') if opened => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(cursor + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            cursor += 1;
        }
        let end = end.ok_or_else(|| {
            error(
                GenericSourceErrorKind::Syntax("incomplete declaration boundary"),
                Some(tokens[start].span),
            )
        })?;
        let origin = Span {
            end: tokens[end - 1].span.end,
            ..tokens[start].span
        };
        let fragment = &source[origin.start..origin.end];
        let count = end - start + 1;
        if fragment.len() > limits.max_declaration_bytes {
            return Err(error(
                GenericSourceErrorKind::Limit("declaration source bytes"),
                Some(origin),
            ));
        }
        charge(&mut left.max_tokens, count, "source tokens")?;
        charge(&mut left.max_work, fragment.len(), "source work")?;
        charge(&mut left.max_work, count, "source work")?;
        macro_rules! common {
            ($v:ident) => {{
                $v.max_retained_source_bytes = $v
                    .max_retained_source_bytes
                    .min(left.max_retained_source_bytes);
                $v.max_name_bytes = $v.max_name_bytes.min(left.max_name_bytes);
                $v.max_tokens = $v.max_tokens.min(count);
                $v.max_parameter_uses = $v.max_parameter_uses.min(left.max_parameter_uses);
                $v.parameters.max_parameters =
                    $v.parameters.max_parameters.min(left.max_parameters);
            }};
        }
        macro_rules! annotations {
            ($v:ident) => {{
                common!($v);
                $v.max_type_nodes = $v.max_type_nodes.min(left.max_type_nodes);
                $v.max_type_edges = $v.max_type_edges.min(left.max_type_edges);
            }};
        }
        let parsed = match kind {
            0 => {
                let mut v = limits.alias;
                common!(v);
                v.target.max_nodes = v.target.max_nodes.min(left.max_type_nodes);
                v.target.max_edges = v.target.max_edges.min(left.max_type_edges);
                GenericSourceDeclarationKind::Alias(
                    parse_generic_alias(fragment, v)
                        .map_err(|e| syntax_error(e, source, origin))?,
                )
            }
            1 => {
                let mut v = limits.record;
                annotations!(v);
                v.max_fields = v.max_fields.min(left.max_members);
                GenericSourceDeclarationKind::Record(
                    parse_generic_record(fragment, v)
                        .map_err(|e| syntax_error(e, source, origin))?,
                )
            }
            2 => {
                let mut v = limits.enumeration;
                annotations!(v);
                v.max_variants = v.max_variants.min(left.max_members);
                GenericSourceDeclarationKind::Enum(
                    parse_generic_enum(fragment, v).map_err(|e| syntax_error(e, source, origin))?,
                )
            }
            _ => {
                let mut v = limits.function;
                annotations!(v);
                v.max_value_parameters = v.max_value_parameters.min(left.max_members);
                let generic =
                    tokens.get(head + 2).map(|t| &t.kind) == Some(&TokenKind::Operator("<"));
                let parsed = if propagation {
                    parse_generic_propagation_function(fragment, v)
                } else if generic {
                    parse_generic_function(fragment, v)
                } else {
                    parse_specialization_root(fragment, v)
                }
                .map_err(|e| syntax_error(e, source, origin))?;
                GenericSourceDeclarationKind::Function(parsed)
            }
        };
        let (retained, names, parameters, members, nodes, edges, uses) = match &parsed {
            GenericSourceDeclarationKind::Alias(v) => (
                v.retained_source_bytes(),
                v.name_bytes(),
                v.parameters().parameters().len(),
                0,
                v.target().syntax().nodes().len(),
                v.target().syntax().edges(),
                v.target().uses().len(),
            ),
            GenericSourceDeclarationKind::Record(v) => (
                v.retained_source_bytes(),
                v.name_bytes(),
                v.parameters().parameters().len(),
                v.fields().len(),
                v.type_nodes(),
                v.type_edges(),
                v.parameter_uses(),
            ),
            GenericSourceDeclarationKind::Enum(v) => (
                v.retained_source_bytes(),
                v.name_bytes(),
                v.parameters().parameters().len(),
                v.variants().len(),
                v.type_nodes(),
                v.type_edges(),
                v.parameter_uses(),
            ),
            GenericSourceDeclarationKind::Function(v) => (
                v.retained_source_bytes(),
                v.name_bytes(),
                v.parameters().parameters().len(),
                v.value_parameters().len(),
                v.type_nodes(),
                v.type_edges(),
                v.parameter_uses(),
            ),
        };
        for amount in [names, parameters, members, nodes, edges, uses] {
            charge(&mut left.max_work, amount, "source work")?;
        }
        charge(
            &mut left.max_retained_source_bytes,
            retained,
            "retained source bytes",
        )?;
        charge(&mut left.max_name_bytes, names, "source name bytes")?;
        charge(&mut left.max_parameters, parameters, "source parameters")?;
        charge(&mut left.max_members, members, "source members")?;
        charge(&mut left.max_type_nodes, nodes, "source type nodes")?;
        charge(&mut left.max_type_edges, edges, "source type edges")?;
        charge(&mut left.max_parameter_uses, uses, "source parameter uses")?;
        if kind == 3 {
            functions.push(declarations.len());
        } else {
            types.push(declarations.len());
        }
        declarations.push(GenericSourceDeclaration {
            span: origin,
            kind: parsed,
        });
        cursor = end;
    }
    let usage = GenericSourceUsage {
        declarations: limits.max_declarations - left.max_declarations,
        retained_source_bytes: limits.max_retained_source_bytes - left.max_retained_source_bytes,
        tokens: limits.max_tokens - left.max_tokens,
        name_bytes: limits.max_name_bytes - left.max_name_bytes,
        parameters: limits.max_parameters - left.max_parameters,
        members: limits.max_members - left.max_members,
        type_nodes: limits.max_type_nodes - left.max_type_nodes,
        type_edges: limits.max_type_edges - left.max_type_edges,
        parameter_uses: limits.max_parameter_uses - left.max_parameter_uses,
        work: limits.max_work - left.max_work,
    };
    Ok(Collected {
        source: owned,
        declarations,
        functions,
        types,
        usage,
        left,
    })
}
impl Collected {
    fn function_span(&self, index: usize, local: Option<Span>) -> Option<Span> {
        let declaration = self.declarations.get(*self.functions.get(index)?)?;
        local
            .and_then(|s| map_span(&self.source, declaration.span, s))
            .or(Some(declaration.span))
    }
    fn type_span(&self, index: usize, local: Option<Span>) -> Option<Span> {
        let declaration = self.declarations.get(*self.types.get(index)?)?;
        local
            .and_then(|s| map_span(&self.source, declaration.span, s))
            .or(Some(declaration.span))
    }
    fn instance_span(
        &self,
        graph: &CheckedGenericSpecializations,
        index: usize,
        local: Option<Span>,
    ) -> Option<Span> {
        self.function_span(graph.instances().get(index)?.function().ordinal(), local)
    }
    fn body_span(&self, err: &GenericCopyBodyError) -> Option<Span> {
        match err {
            GenericCopyBodyError::Propagation { function, span, .. } => {
                self.function_span(function.ordinal(), Some(*span))
            }
            GenericCopyBodyError::Parse {
                function,
                diagnostics,
            } => self.function_span(
                function.ordinal(),
                diagnostics.items.first().map(|d| d.span),
            ),
            GenericCopyBodyError::Semantic { function, span, .. }
            | GenericCopyBodyError::FieldType { function, span, .. } => {
                self.function_span(function.ordinal(), Some(*span))
            }
            GenericCopyBodyError::Annotation {
                function,
                annotation_range,
                ..
            } => self.function_span(function.ordinal(), Some(*annotation_range)),
            GenericCopyBodyError::Call {
                function,
                type_range,
                ..
            }
            | GenericCopyBodyError::RecordType {
                function,
                type_range,
                ..
            }
            | GenericCopyBodyError::EnumType {
                function,
                type_range,
                ..
            } => self.function_span(function.ordinal(), Some(*type_range)),
            GenericCopyBodyError::Pattern { function, .. }
            | GenericCopyBodyError::Comparison { function, .. } => {
                self.function_span(function.ordinal(), None)
            }
            GenericCopyBodyError::Limit(_) => None,
        }
    }
}
/// Collect every admitted original declaration, check all definitions/signatures/
/// bodies, select an explicit zero-argument nongeneric entry, and run the complete
/// finite specialization/layout/IR pipeline. No input is skipped or rewritten.
/// Imports/modules/application declarations remain outside this standalone profile.
pub fn compile_generic_source(
    source: &str,
    entry: &str,
    limits: GenericSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_generic_source_profile(source, entry, limits, true, true)
}
pub(super) fn compile_generic_source_profile(
    source: &str,
    entry: &str,
    limits: GenericSourceLimits,
    standard_enums: bool,
    standard_error: bool,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits,
        standard_enums,
        standard_error,
        None,
        StandardResultProfile::None,
    )
}
/// Compile complete original standalone source with explicit bounded postfix support.
/// Ordinary compilation, CLI and native-image profiles remain strict.
pub fn compile_generic_propagation_source(
    source: &str,
    entry: &str,
    limits: GenericPropagationSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits.source,
        true,
        true,
        Some(limits),
        StandardResultProfile::None,
    )
}
/// Compile explicit standard-result adapter consumers; no host builtins are enabled.
pub fn compile_generic_adapter_source(
    source: &str,
    entry: &str,
    limits: GenericPropagationSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits.source,
        true,
        true,
        Some(limits),
        StandardResultProfile::Records,
    )
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum StandardResultProfile {
    None,
    Records,
    Builtins,
    Strings,
    Values,
    Arrays,
}
/// Compile host-free parsing/splitting builtins and explicit standard result adapters.
pub fn compile_generic_builtin_source(
    source: &str,
    entry: &str,
    limits: GenericPropagationSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits.source,
        true,
        true,
        Some(limits),
        StandardResultProfile::Builtins,
    )
}
/// Compile host-free parsing/splitting and checked String utility calls.
pub fn compile_generic_string_source(
    source: &str,
    entry: &str,
    limits: GenericPropagationSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits.source,
        true,
        true,
        Some(limits),
        StandardResultProfile::Strings,
    )
}
/// Compile host-free builtins including polymorphic display and array length.
pub fn compile_generic_value_source(
    source: &str,
    entry: &str,
    limits: GenericPropagationSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits.source,
        true,
        true,
        Some(limits),
        StandardResultProfile::Values,
    )
}
/// Compile host-free builtins including checked array growth and slicing.
pub fn compile_generic_array_source(
    source: &str,
    entry: &str,
    limits: GenericPropagationSourceLimits,
) -> Result<CompiledGenericSource, GenericSourceError> {
    compile_profile_mode(
        source,
        entry,
        limits.source,
        true,
        true,
        Some(limits),
        StandardResultProfile::Arrays,
    )
}
fn compile_profile_mode(
    source: &str,
    entry: &str,
    limits: GenericSourceLimits,
    standard_enums: bool,
    standard_error: bool,
    propagation: Option<GenericPropagationSourceLimits>,
    result_profile: StandardResultProfile,
) -> Result<CompiledGenericSource, GenericSourceError> {
    let standard_results = result_profile != StandardResultProfile::None;
    let standard_arrays = result_profile == StandardResultProfile::Arrays;
    let standard_values = matches!(
        result_profile,
        StandardResultProfile::Values | StandardResultProfile::Arrays
    );
    let standard_strings = matches!(
        result_profile,
        StandardResultProfile::Strings
            | StandardResultProfile::Values
            | StandardResultProfile::Arrays
    );
    let standard_builtins = matches!(
        result_profile,
        StandardResultProfile::Builtins
            | StandardResultProfile::Strings
            | StandardResultProfile::Values
            | StandardResultProfile::Arrays
    );
    let mut parsed = collect(source, entry, limits.syntax, propagation.is_some())?;
    let standards = if standard_enums {
        Some(
            super::generic_enums::standard_generic_enums().map_err(|error_value| {
                error(GenericSourceErrorKind::Declaration(error_value), None)
            })?,
        )
    } else {
        None
    };
    let error_record = if standard_error {
        if !standard_enums {
            return Err(error(
                GenericSourceErrorKind::Limit("standard feature profile"),
                None,
            ));
        }
        Some(
            super::generic_records::standard_error_record()
                .map_err(|e| error(GenericSourceErrorKind::Declaration(e), None))?,
        )
    } else {
        None
    };
    let result_records = if standard_results {
        super::generic_records::standard_result_records()
            .map_err(|e| error(GenericSourceErrorKind::Declaration(e), None))?
    } else {
        Vec::new()
    };
    let mut declarations: Vec<_> = parsed
        .types
        .iter()
        .map(|i| match &parsed.declarations[*i].kind {
            GenericSourceDeclarationKind::Alias(v) => GenericTypeDeclaration::Alias(v),
            GenericSourceDeclarationKind::Record(v) => GenericTypeDeclaration::Record(v),
            GenericSourceDeclarationKind::Enum(v) => GenericTypeDeclaration::Enum(v),
            _ => unreachable!("type source index"),
        })
        .collect();
    if let Some(standards) = &standards {
        declarations.extend(standards.iter().map(GenericTypeDeclaration::Enum));
    }
    if let Some(record) = &error_record {
        declarations.push(GenericTypeDeclaration::Record(record));
    }
    declarations.extend(result_records.iter().map(GenericTypeDeclaration::Record));
    let aliases: Vec<_> = parsed
        .declarations
        .iter()
        .filter_map(|d| {
            if let GenericSourceDeclarationKind::Alias(v) = &d.kind {
                Some(v)
            } else {
                None
            }
        })
        .collect();
    let functions: Vec<_> = parsed
        .functions
        .iter()
        .map(|i| {
            if let GenericSourceDeclarationKind::Function(v) = &parsed.declarations[*i].kind {
                v
            } else {
                unreachable!("function source index")
            }
        })
        .collect();
    let registry = register_generic_types(&declarations, limits.registry).map_err(|e| {
        let span = if let GenericResolutionError::Declaration { index, span, .. } = &e {
            parsed.type_span(*index, Some(*span))
        } else {
            None
        };
        error(GenericSourceErrorKind::Registry(e), span)
    })?;
    let environment =
        prepare_generic_aliases(&registry, &aliases, limits.aliases).map_err(|e| {
            let span = match &e {
                GenericAliasExpansionError::Cycle(id) => parsed.type_span(id.ordinal(), None),
                GenericAliasExpansionError::Resolution {
                    alias: Some(id), ..
                } => parsed.type_span(id.ordinal(), None),
                _ => None,
            };
            error(GenericSourceErrorKind::Alias(e), span)
        })?;
    let types = check_generic_declarations(&environment, &declarations, limits.declarations)
        .map_err(|e| {
            let span = match &e {
                GenericDeclarationError::Ownership { input, span, .. } => {
                    parsed.type_span(*input, Some(*span))
                }
                GenericDeclarationError::Missing(id) => parsed.type_span(id.ordinal(), None),
                GenericDeclarationError::EmptyEnum { declaration, span } => {
                    parsed.type_span(declaration.ordinal(), Some(*span))
                }
                GenericDeclarationError::Annotation {
                    declaration,
                    annotation_range,
                    ..
                } => parsed.type_span(declaration.ordinal(), Some(*annotation_range)),
                _ => None,
            };
            error(GenericSourceErrorKind::Types(e), span)
        })?;
    let signatures =
        check_generic_signatures(&types, &functions, limits.signatures).map_err(|e| {
            let span = match &e {
                GenericSignatureError::Definition { function, span, .. } => {
                    parsed.function_span(*function, Some(*span))
                }
                GenericSignatureError::Annotation {
                    function,
                    annotation_range,
                    ..
                } => parsed.function_span(*function, Some(*annotation_range)),
                _ => None,
            };
            error(GenericSourceErrorKind::Signatures(e), span)
        })?;
    let mut selected = None;
    for (index, function) in functions.iter().enumerate() {
        for amount in [1, entry.len(), function.name().len()] {
            charge(&mut parsed.left.max_work, amount, "source work")?;
        }
        if function.name() == entry {
            selected = Some(index);
            break;
        }
    }
    let selected = selected.ok_or_else(|| {
        error(
            GenericSourceErrorKind::Entry("selected entry function is absent"),
            None,
        )
    })?;
    if !functions[selected].parameters().parameters().is_empty()
        || !functions[selected].value_parameters().is_empty()
    {
        return Err(error(
            GenericSourceErrorKind::Entry("entry must have no type or value arguments"),
            parsed.function_span(selected, Some(functions[selected].name_span())),
        ));
    }
    parsed.usage.work = limits.syntax.max_work - parsed.left.max_work;
    let bodies = if standard_arrays {
        super::generic_copy_bodies::check_generic_array_bodies(
            &signatures,
            limits.bodies,
            propagation.expect("array propagation profile").propagation,
        )
    } else if standard_values {
        super::generic_copy_bodies::check_generic_value_bodies(
            &signatures,
            limits.bodies,
            propagation.expect("value propagation profile").propagation,
        )
    } else if standard_strings {
        super::generic_copy_bodies::check_generic_string_bodies(
            &signatures,
            limits.bodies,
            propagation.expect("String propagation profile").propagation,
        )
    } else if standard_builtins {
        super::generic_copy_bodies::check_generic_builtin_bodies(
            &signatures,
            limits.bodies,
            propagation
                .expect("builtin propagation profile")
                .propagation,
        )
    } else if let Some(p) = propagation {
        check_generic_propagation_bodies(&signatures, limits.bodies, p.propagation)
    } else {
        check_generic_copy_bodies(&signatures, limits.bodies)
    }
    .map_err(|e| {
        let span = parsed.body_span(&e);
        error(GenericSourceErrorKind::Bodies(e), span)
    })?;
    let root = signatures.signatures()[selected].id();
    let graph = if propagation.is_some() {
        discover_generic_propagation_specializations(&bodies, root, limits.specialization)
    } else {
        discover_generic_specializations(&bodies, root, limits.specialization)
    }
    .map_err(|e| error(GenericSourceErrorKind::Specialization(e), None))?;
    let concrete = if let Some(p) = propagation {
        recheck_generic_propagation_concrete_bodies(&graph, limits.concrete, p.propagation)
    } else {
        recheck_generic_concrete_bodies(&graph, limits.concrete)
    }
    .map_err(|e| {
        let span = match &e {
            GenericConcreteBodyError::Context { instance, .. } => {
                parsed.instance_span(&graph, *instance, None)
            }
            GenericConcreteBodyError::Propagation { instance, span, .. } => {
                parsed.instance_span(&graph, *instance, Some(*span))
            }
            _ => None,
        };
        error(GenericSourceErrorKind::Concrete(e), span)
    })?;
    let literals = decode_generic_literals(&concrete, limits.literals).map_err(|e| {
        let span = match &e {
            GenericLiteralError::Source { instance, span } => {
                parsed.instance_span(&graph, *instance, Some(*span))
            }
            GenericLiteralError::Lexical {
                instance,
                diagnostics,
            } => parsed.instance_span(&graph, *instance, diagnostics.items.first().map(|d| d.span)),
            _ => None,
        };
        error(GenericSourceErrorKind::Literals(e), span)
    })?;
    let layouts = plan_generic_layouts(&literals, limits.layouts).map_err(|e| {
        let span = if let GenericLayoutError::Member { declaration, .. } = &e {
            parsed.type_span(declaration.ordinal(), None)
        } else {
            None
        };
        error(GenericSourceErrorKind::Layouts(e), span)
    })?;
    let checked_propagation = if let Some(p) = propagation {
        let operations = plan_generic_propagation_layouts(&layouts, p.operations)
            .map_err(|e| error(GenericSourceErrorKind::PropagationLayouts(e), None))?;
        Some(
            lower_generic_propagation_ir(&operations, limits.construction, limits.ir).map_err(
                |e| {
                    let span = match &e {
                        GenericIrError::Invalid { function, span, .. } => {
                            parsed.instance_span(&graph, *function, Some(*span))
                        }
                        GenericIrError::Construction(GenericConstructionError::Context {
                            instance,
                            span,
                            ..
                        }) => parsed.instance_span(&graph, *instance, Some(*span)),
                        _ => None,
                    };
                    error(GenericSourceErrorKind::Ir(e), span)
                },
            )?,
        )
    } else {
        None
    };
    let ir = if let Some(p) = &checked_propagation {
        Ok(p.checked_ir().clone())
    } else {
        let construction =
            plan_generic_construction(&layouts, limits.construction).map_err(|e| {
                let span = if let GenericConstructionError::Context { instance, span, .. } = &e {
                    parsed.instance_span(&graph, *instance, Some(*span))
                } else {
                    None
                };
                error(GenericSourceErrorKind::Construction(e), span)
            })?;
        lower_generic_ir(&construction, limits.ir).map_err(|e| {
            let span = if let GenericIrError::Invalid { function, span, .. } = &e {
                parsed.instance_span(&graph, *function, Some(*span))
            } else {
                None
            };
            error(GenericSourceErrorKind::Ir(e), span)
        })
    }?;
    Ok(CompiledGenericSource {
        source: parsed.source,
        declarations: parsed.declarations.into(),
        functions: parsed.functions.into(),
        entry: Arc::from(entry),
        ir,
        propagation: checked_propagation,
        standard_results,
        standard_builtins,
        standard_strings,
        standard_values,
        standard_arrays,
        usage: parsed.usage,
    })
}
/// Execute the checked selected root and retain its complete original source.
/// Runtime errors expose an original-file primary span plus unchanged inner detail.
pub fn execute_generic_source(
    program: &CompiledGenericSource,
    policy: GenericVmPolicy,
) -> Result<GenericSourceOutcome, GenericSourceExecutionError> {
    let execution = match &program.propagation {
        Some(p) => execute_generic_propagation_ir(p, policy),
        None => execute_generic_ir(&program.ir, policy),
    }
    .map_err(|error| GenericSourceExecutionError {
        span: program.file_span(error.function, error.span),
        error,
    })?;
    Ok(GenericSourceOutcome {
        program: program.clone(),
        execution,
    })
}

/// Whole-suite admission, in addition to finite per-compilation and per-test policies.
#[derive(Debug, Clone, Copy)]
pub struct GenericTestLimits {
    /// Root tests, hard capped at 32.
    pub max_tests: usize,
    /// Sum of source bytes across discovery and both compilation passes.
    pub max_compiled_source_bytes: usize,
}
impl GenericTestLimits {
    /// Finite command/library defaults. No hosts or unbounded fallback.
    pub const DEFAULT: Self = Self {
        max_tests: 32,
        max_compiled_source_bytes: 16 * 1024 * 1024,
    };
}
/// Check all selected roots before executing any; run each with fresh VM state.
/// No main is required or called. Immutable source is compiled in two passes to
/// avoid retaining every specialized program at once. Runtime failures do not
/// stop subsequent tests. Dependencies/modules and I/O hosts remain unavailable.
pub fn test_generic_source(
    source: &str,
    label: &str,
    compilation: GenericPropagationSourceLimits,
    policy: GenericVmPolicy,
    limits: GenericTestLimits,
) -> crate::compiler::testing::TestReport {
    use crate::compiler::{
        diagnostics::{diagnostic_message, Diagnostic, Severity},
        testing::{TestOutcome, TestReport},
    };
    let mut report = TestReport {
        target: label.into(),
        package: false,
        diagnostics: Diagnostics::new(),
        tests: Vec::new(),
    };
    let diagnostic = |code, span: Option<Span>, message| {
        Diagnostic {
            code,
            severity: Severity::Error,
            message,
            source_file: span.map(|_| label.into()),
            span: span.unwrap_or(Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            }),
        }
        .bounded()
    };
    if source.len() > limits.max_compiled_source_bytes {
        report.diagnostics.items.push(diagnostic(
            "E5003",
            None,
            "generic suite compilation allowance exceeded".into(),
        ));
        return report;
    }
    let parsed = match collect(source, "", compilation.source.syntax, true) {
        Ok(parsed) => parsed,
        Err(error) => {
            report.diagnostics.items.push(diagnostic(
                "E3123",
                error.span,
                diagnostic_message!("{error}"),
            ));
            return report;
        }
    };
    let mut selected = Vec::new();
    for declaration in &parsed.declarations {
        let GenericSourceDeclarationKind::Function(function) = declaration.kind() else {
            continue;
        };
        if !function.name().starts_with("test_") {
            continue;
        }
        if selected.len() >= limits.max_tests.min(32) {
            report.diagnostics.items.push(diagnostic(
                "E5003",
                Some(declaration.span()),
                "generic test count allowance exceeded".into(),
            ));
            return report;
        }
        if !function.parameters().parameters().is_empty() || !function.value_parameters().is_empty()
        {
            report.diagnostics.items.push(diagnostic(
                "E5001",
                Some(declaration.span()),
                "test functions must be nongeneric, parameterless and return Unit".into(),
            ));
        }
        selected.push((function.name().to_owned(), declaration.span()));
    }
    if selected.is_empty() {
        report.diagnostics.items.push(diagnostic(
            "E5002",
            None,
            "no root-level test_ functions were found".into(),
        ));
    }
    if !report.diagnostics.is_empty() {
        return report;
    }
    // Preflight all compilation passes before performing even the first one.
    let admitted = selected
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .and_then(|n| n.checked_mul(source.len()));
    if admitted.map_or(true, |n| n > limits.max_compiled_source_bytes) {
        report.diagnostics.items.push(diagnostic(
            "E5003",
            None,
            "generic suite compilation allowance exceeded".into(),
        ));
        return report;
    }
    drop(parsed);
    for (name, span) in &selected {
        match compile_generic_array_source(source, name, compilation) {
            Ok(program) => {
                let root = &program.ir().functions()[0];
                if !matches!(
                    program.ir().plans().layouts().layouts()[root.return_layout()].kind(),
                    GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Unit)
                ) {
                    report.diagnostics.items.push(diagnostic(
                        "E5001",
                        Some(*span),
                        "test functions must return Unit".into(),
                    ));
                }
            }
            Err(error) => report.diagnostics.items.push(diagnostic(
                "E3123",
                error.span,
                diagnostic_message!("{error}"),
            )),
        }
        if !report.diagnostics.is_empty() {
            return report;
        }
    }
    for (name, _) in selected {
        let program = match compile_generic_array_source(source, &name, compilation) {
            Ok(program) => program,
            Err(error) => {
                report.diagnostics.items.push(diagnostic(
                    "E3123",
                    error.span,
                    diagnostic_message!("{error}"),
                ));
                return report;
            }
        };
        let execution = execute_generic_source(&program, policy.clone());
        report.tests.push(TestOutcome {
            name,
            passed: execution.is_ok(),
            output: Vec::new(),
            exit_code: execution.as_ref().ok().map(|_| 0),
            error: execution.err().map(|error| diagnostic_message!("{error}")),
        });
    }
    report
}

// Parse one original package scope without rewriting module wrappers/imports.
pub(crate) fn collect_package_fragment(
    source: &str,
    range: std::ops::Range<usize>,
    limits: &mut GenericSourceSyntaxLimits,
) -> Result<Vec<GenericSourceDeclaration>, GenericSourceError> {
    let fragment = source
        .get(range.clone())
        .ok_or_else(|| error(GenericSourceErrorKind::Syntax("package scope range"), None))?;
    charge(
        &mut limits.max_work,
        range.start.checked_mul(2).ok_or_else(|| {
            error(
                GenericSourceErrorKind::Limit("package source mapping"),
                None,
            )
        })?,
        "package source mapping",
    )?;
    let prefix = &source[..range.start];
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let column = prefix.rsplit('\n').next().unwrap_or("").chars().count();
    let translate = |local: Span| -> Option<Span> {
        fragment.get(local.start..local.end)?;
        Some(Span {
            start: range.start.checked_add(local.start)?,
            end: range.start.checked_add(local.end)?,
            line: line.checked_add(local.line)?,
            column: if local.line == 0 {
                column.checked_add(local.column)?
            } else {
                local.column
            },
        })
    };
    let parsed = collect(fragment, "", *limits, true).map_err(|mut failure| {
        failure.span = failure.span.and_then(translate);
        failure
    })?;
    *limits = parsed.left;
    let mut declarations = parsed.declarations;
    for declaration in &mut declarations {
        declaration.span = translate(declaration.span).ok_or_else(|| {
            error(
                GenericSourceErrorKind::Syntax("package declaration provenance"),
                None,
            )
        })?;
    }
    Ok(declarations)
}
