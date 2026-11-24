//! Syntax-only v2 layout with token/comment/AST/fixed-point gates.
use super::{equivalence, files, layout, Failure};
use crate::compiler::{
    ast::Statement,
    diagnostics::Span,
    lexer::{Lexer, Token, TokenKind},
    v2::*,
};
use std::collections::BTreeSet;
const ZERO: Span = Span {
    start: 0,
    end: 0,
    line: 0,
    column: 0,
};
fn parse(source: &str) -> Result<ParsedProgramSyntax, Failure> {
    parse_program_syntax(
        source,
        EntrySourceLimits::DEFAULT,
        ProgramSourceLimits::DEFAULT.max_functions,
    )
    .map_err(|error| {
        let diagnostics = ProgramSourceError::Source(error).into_diagnostics();
        if diagnostics.items.len() == 1 && diagnostics.items[0].span.end == 0 {
            let diagnostic = &diagnostics.items[0];
            Failure::Message(format!(
                "error[{}]: {}",
                diagnostic.code, diagnostic.message
            ))
        } else {
            Failure::Diagnostics(diagnostics)
        }
    })
}
fn statement_span(statement: &Statement) -> Span {
    match statement {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::Return { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. } => *span,
        Statement::Expression(expression) => expression.span,
    }
}
fn mark(span: Span, tokens: &[Token], before: &mut BTreeSet<usize>, after: &mut BTreeSet<usize>) {
    before.insert(span.start);
    let index = tokens.partition_point(|token| token.span.start < span.end);
    let end = tokens
        .get(index)
        .filter(|token| token.kind == TokenKind::Punctuation(';'))
        .map_or(span.end, |token| token.span.end);
    after.insert(end);
}
pub(super) fn core(
    statement: &Statement,
    tokens: &[Token],
    before: &mut BTreeSet<usize>,
    after: &mut BTreeSet<usize>,
) {
    mark(statement_span(statement), tokens, before, after);
    match statement {
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            for statement in then_block.iter().chain(else_block.iter().flatten()) {
                core(statement, tokens, before, after);
            }
        }
        Statement::While { body, .. } => {
            for statement in body {
                core(statement, tokens, before, after);
            }
        }
        _ => {}
    }
}
fn match_header(
    header: &MatchStatementSyntax,
    tokens: &[Token],
    before: &mut BTreeSet<usize>,
    after: &mut BTreeSet<usize>,
) {
    mark(header.span, tokens, before, after);
    for arm in &header.arms {
        before.insert(arm.span.start);
        for statement in &arm.body {
            core(statement, tokens, before, after);
        }
    }
}
fn body(
    statements: &[EnumFunctionStatement],
    tokens: &[Token],
    before: &mut BTreeSet<usize>,
    after: &mut BTreeSet<usize>,
) {
    for statement in statements {
        match statement {
            EnumFunctionStatement::Core(statement) => core(statement, tokens, before, after),
            EnumFunctionStatement::Match(header) => match_header(header, tokens, before, after),
            EnumFunctionStatement::RecursiveMatch(syntax) => {
                match_header(&syntax.header, tokens, before, after);
                for statements in &syntax.bodies {
                    body(statements, tokens, before, after);
                }
            }
            EnumFunctionStatement::Control(syntax) => {
                core(&syntax.header, tokens, before, after);
                for statements in &syntax.bodies {
                    body(statements, tokens, before, after);
                }
            }
        }
    }
}
fn render(source: &str, tokens: &[Token], syntax: &ParsedProgramSyntax) -> Result<String, Failure> {
    let mut before = BTreeSet::new();
    let mut after = BTreeSet::new();
    for declaration in syntax.declarations() {
        before.insert(declaration.span.start);
    }
    for function in syntax.functions() {
        body(&function.body, tokens, &mut before, &mut after);
    }
    Ok(layout::render_marked(source, tokens, &before, &after)?)
}
fn normalize_match(header: &mut MatchStatementSyntax) {
    header.span = ZERO;
    header.scrutinee_span = ZERO;
    for arm in &mut header.arms {
        arm.span = ZERO;
        arm.binding_span = arm.binding_span.map(|_| ZERO);
        equivalence::statements(&mut arm.body);
    }
}
fn normalize_body(statements: &mut [EnumFunctionStatement]) {
    for statement in statements {
        match statement {
            EnumFunctionStatement::Core(statement) => {
                equivalence::statements(std::slice::from_mut(statement))
            }
            EnumFunctionStatement::Match(header) => normalize_match(header),
            EnumFunctionStatement::RecursiveMatch(syntax) => {
                normalize_match(&mut syntax.header);
                for body in &mut syntax.bodies {
                    normalize_body(body);
                }
            }
            EnumFunctionStatement::Control(syntax) => {
                equivalence::statements(std::slice::from_mut(&mut syntax.header));
                for body in &mut syntax.bodies {
                    normalize_body(body);
                }
            }
        }
    }
}
fn normalize(syntax: &mut ParsedProgramSyntax) {
    equivalence::normalize(&mut syntax.core);
    for declaration in &mut syntax.declarations {
        declaration.span = ZERO;
    }
    for declaration in &mut syntax.enums {
        declaration.span = ZERO;
        declaration.name_span = ZERO;
        for variant in &mut declaration.variants {
            variant.span = ZERO;
            variant.name_span = ZERO;
            variant.payload_type_span = variant.payload_type_span.map(|_| ZERO);
        }
    }
    for function in &mut syntax.functions {
        function.span = ZERO;
        function.return_type_span = function.return_type_span.map(|_| ZERO);
        for parameter in &mut function.parameters {
            parameter.span = ZERO;
            parameter.type_span = parameter.type_span.map(|_| ZERO);
        }
        normalize_body(&mut function.body);
    }
}
pub(super) fn format_source(source: &str) -> Result<String, Failure> {
    if source.len() > files::INPUT_LIMIT {
        return Err(Failure::Message("formatter source exceeds 1 MiB".into()));
    }
    let tokens = Lexer::new().tokenize(source)?;
    let mut syntax = parse(source)?;
    let output = render(source, &tokens, &syntax)?;
    let formatted_tokens = Lexer::new()
        .tokenize(&output)
        .map_err(|_| Failure::Message("formatter v2 safety check failed: output lexing".into()))?;
    if tokens.len() != formatted_tokens.len()
        || tokens.iter().zip(&formatted_tokens).any(|(a, b)| {
            a.kind != b.kind || source[a.span.start..a.span.end] != output[b.span.start..b.span.end]
        })
    {
        return Err(Failure::Message(
            "formatter v2 safety check failed: token spelling/order changed".into(),
        ));
    }
    if layout::comments(source, &tokens)? != layout::comments(&output, &formatted_tokens)? {
        return Err(Failure::Message(
            "formatter v2 safety check failed: comment content/attachment changed".into(),
        ));
    }
    let mut formatted = parse(&output)
        .map_err(|_| Failure::Message("formatter v2 safety check failed: output parsing".into()))?;
    if render(&output, &formatted_tokens, &formatted)? != output {
        return Err(Failure::Message(
            "formatter v2 safety check failed: output is not idempotent".into(),
        ));
    }
    normalize(&mut syntax);
    normalize(&mut formatted);
    if syntax != formatted {
        return Err(Failure::Message(
            "formatter v2 safety check failed: AST structure changed".into(),
        ));
    }
    Ok(output)
}
