//! Callable syntax admitted by application inspection and parsed by the executable parser.

use super::ApplicationTypeSyntax;
use crate::compiler::{
    ast::Function,
    diagnostics::Span,
    lexer::{Lexer, Token, TokenKind},
    parser::Parser,
};
use std::sync::Arc;

/// Maximum retained application callables per source.
pub const MAX_CALLABLES: usize = 4096;
/// Maximum non-EOF tokens admitted by callable syntax parsing.
pub const MAX_CALLABLE_TOKENS: usize = 65_536;

/// Application declaration context of an executable callable body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallableKind {
    /// A top-level ordinary function.
    Function,
    /// A top-level application task; this does not establish task execution.
    Task,
    /// A service operation with a body, retaining its declared service owner.
    ServiceOperation {
        /// Source service spelling, not a canonical resolved identity.
        service: String,
    },
}

/// A callable with its original source envelope and shared executable AST.
#[derive(Debug, Clone, PartialEq)]
pub struct CallableSyntax {
    /// Function/task/service declaration context.
    pub kind: CallableKind,
    /// Full original declaration range, including export where present.
    pub span: Span,
    /// Original header and signature range, excluding the body opening brace.
    pub header_span: Span,
    /// Original braced body range.
    pub body_span: Span,
    /// Executable parser nodes with original source spans and annotation spelling.
    /// Task keyword tokens are adapted to fn only for this parser invocation.
    pub function: Function,
}

/// Partial application syntax; even empty gaps never establish semantic validity.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationCallableSyntax {
    /// Alias/record syntax from the same source snapshot.
    pub types: Arc<ApplicationTypeSyntax>,
    /// Callable bodies in original declaration order.
    pub callables: Vec<CallableSyntax>,
    /// Token envelopes outside retained type/callable nodes.
    /// Imports, service containers and bodyless prototypes remain explicit gaps.
    pub unparsed_ranges: Vec<Span>,
}

/// Retain application callables without executing or resolving filesystem imports.
///
/// Existing application inspection admits supported file syntax, then each
/// body-bearing callable uses executable parsing. Unsupported syntax or quotas
/// reject the entire request; no partial callable table escapes. This API does
/// not type-check names, authorize service calls or change ordinary checking.
pub fn parse_callables(source: &str) -> Result<ApplicationCallableSyntax, String> {
    parse_skipping(source, &[])
}

pub(super) fn parse_skipping(
    source: &str,
    skipped: &[Span],
) -> Result<ApplicationCallableSyntax, String> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err("application callable syntax exceeds 1 MiB source limit".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "application callable syntax contains lexical errors".to_owned())?;
    if tokens.len().saturating_sub(1) > MAX_CALLABLE_TOKENS {
        return Err("application callable syntax token limit exceeded".into());
    }
    let types = super::parse(source)?;
    let inspections = super::super::inspect_functions_skipping(source, skipped)?;
    if inspections.len() > MAX_CALLABLES {
        return Err("application callable syntax declaration limit exceeded".into());
    }
    let mut callables = Vec::new();
    for inspected in inspections {
        let start = tokens.partition_point(|token| token.span.start < inspected.span.start);
        let end = tokens.partition_point(|token| token.span.start < inspected.span.end);
        let mut selected = tokens[start..end].to_vec();
        let kind = if inspected.is_task {
            let keyword = selected.first_mut().ok_or("missing task keyword")?;
            if keyword.kind != TokenKind::Keyword("task")
                && keyword.kind != TokenKind::Identifier("task".into())
            {
                return Err("application callable syntax task provenance mismatch".into());
            }
            keyword.kind = TokenKind::Keyword("fn");
            CallableKind::Task
        } else if let Some((service, _)) = inspected.name.split_once('.') {
            CallableKind::ServiceOperation {
                service: service.to_owned(),
            }
        } else {
            CallableKind::Function
        };
        // The first brace is the body opener: current supported annotations have
        // no brace syntax, and inspection has already validated the signature.
        let body_start = selected
            .iter()
            .find(|token| token.kind == TokenKind::Punctuation('{'))
            .ok_or("missing callable body")?
            .span;
        selected.push(tokens.last().ok_or("missing EOF")?.clone());
        let mut program = Parser::new().parse_tokens(&selected).map_err(|_| {
            "application callable body is outside shared executable syntax".to_owned()
        })?;
        if program.functions.len() != 1
            || !program.modules.is_empty()
            || !program.type_declarations.is_empty()
            || !program.struct_declarations.is_empty()
        {
            return Err("application callable syntax provenance mismatch".into());
        }
        callables.push(CallableSyntax {
            kind,
            span: inspected.span,
            header_span: Span {
                end: body_start.start,
                ..inspected.span
            },
            body_span: Span {
                end: inspected.span.end,
                ..body_start
            },
            function: program.functions.remove(0),
        });
    }
    let spans: Vec<_> = callables.iter().map(|callable| callable.span).collect();
    let unparsed_ranges = unparsed(&tokens, &types.unparsed_ranges, &spans);
    Ok(ApplicationCallableSyntax {
        types,
        callables,
        unparsed_ranges,
    })
}

pub(super) fn unparsed(tokens: &[Token], initial_gaps: &[Span], covered: &[Span]) -> Vec<Span> {
    let mut unparsed_ranges: Vec<Span> = Vec::new();
    let mut run: Option<Span> = None;
    for token in tokens.iter().filter(|token| token.kind != TokenKind::Eof) {
        let gap = initial_gaps.partition_point(|range| range.end <= token.span.start);
        let in_type_gap = initial_gaps
            .get(gap)
            .is_some_and(|range| range.start <= token.span.start);
        let callable = covered.partition_point(|span| span.end <= token.span.start);
        let in_callable = covered
            .get(callable)
            .is_some_and(|span| span.start <= token.span.start && token.span.end <= span.end);
        if in_type_gap && !in_callable {
            match &mut run {
                Some(span) => span.end = token.span.end,
                None => run = Some(token.span),
            }
        } else if let Some(span) = run.take() {
            unparsed_ranges.push(span);
        }
    }
    if let Some(span) = run {
        unparsed_ranges.push(span);
    }
    unparsed_ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_order_context_signatures_and_nested_executable_body_nodes() {
        let source = "// λ\r\ntype Count = Int;\r\nexport fn f(x:Count)->Int{if true{return x;}else{return 0;}}\r\ntask job(x:Int){while false{print(x);}}\r\nservice Api { fn prototype(x: Int); fn value()->Int{return 1;} }";
        let parsed = parse_callables(source).unwrap();
        assert_eq!(parsed.callables.len(), 3);
        assert_eq!(parsed.callables[0].kind, CallableKind::Function);
        assert!(parsed.callables[0].function.is_exported);
        assert_eq!(
            parsed.callables[0].function.parameters[0]
                .type_name
                .as_deref(),
            Some("Count")
        );
        assert!(matches!(
            parsed.callables[0].function.body[0],
            crate::compiler::ast::Statement::If { .. }
        ));
        assert_eq!(parsed.callables[1].kind, CallableKind::Task);
        assert!(
            source[parsed.callables[1].span.start..parsed.callables[1].span.end]
                .starts_with("task job")
        );
        assert_eq!(
            parsed.callables[2].kind,
            CallableKind::ServiceOperation {
                service: "Api".into()
            }
        );
        assert_eq!(parsed.callables[2].function.name, "value");
        assert_eq!(
            parsed
                .unparsed_ranges
                .iter()
                .map(|span| &source[span.start..span.end])
                .collect::<Vec<_>>(),
            ["service Api { fn prototype(x: Int);", "}"]
        );
        for callable in &parsed.callables {
            assert_eq!(
                &source[callable.body_span.start..callable.body_span.start + 1],
                "{"
            );
            assert_eq!(callable.header_span.end, callable.body_span.start);
            assert_eq!(callable.body_span.end, callable.span.end);
        }
    }
    #[test]
    fn malformed_and_unsupported_files_never_publish_partial_callable_tables() {
        for source in [
            "fn good(){} app Unknown {}",
            "fn good(){} fn broken(){let x=;}",
            "task bad(x){}",
            "service Api {fn bad(x);}",
        ] {
            assert!(parse_callables(source).is_err(), "{source}");
        }
        let parsed = parse_callables("fn unchecked(x:Unknown)->Int{return missing;}").unwrap();
        assert_eq!(parsed.callables.len(), 1);
        assert!(parsed.unparsed_ranges.is_empty()); // Syntax only, never full checking.
        assert!(parse_callables(&" ".repeat(super::super::MAX_SOURCE_BYTES + 1)).is_err());
    }
    #[test]
    fn callable_and_token_limits_accept_exact_boundaries() {
        let functions = (0..MAX_CALLABLES)
            .map(|n| format!("fn f{n}(){{}}\n"))
            .collect::<String>();
        assert_eq!(
            parse_callables(&functions).unwrap().callables.len(),
            MAX_CALLABLES
        );
        assert!(parse_callables(&(functions + "fn extra(){}"))
            .unwrap_err()
            .contains("declaration limit"));
        let source = format!(
            "fn main(){{{}}}",
            "print(1);".repeat((MAX_CALLABLE_TOKENS - 6) / 5)
        );
        assert_eq!(
            Lexer::new().tokenize(&source).unwrap().len() - 1,
            MAX_CALLABLE_TOKENS
        );
        assert_eq!(parse_callables(&source).unwrap().callables.len(), 1);
        assert!(parse_callables(&(source + "fn extra(){}"))
            .unwrap_err()
            .contains("token limit"));
    }
}
