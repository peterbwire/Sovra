//! Structured service containers and bodyless signatures, without service typing.
use super::{ApplicationCallableSyntax, CallableSyntax};
use crate::compiler::{
    ast::Parameter,
    diagnostics::Span,
    lexer::{Lexer, Token, TokenKind},
    parser::Parser,
};

/// Maximum retained service containers per source.
pub const MAX_SERVICES: usize = 1024;
/// Maximum cumulative retained service operations per source.
pub const MAX_SERVICE_OPERATIONS: usize = 4096;

/// A service operation's declared signature, without any invented body.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceSignatureSyntax {
    /// Declared operation name.
    pub name: String,
    /// Original parameter nodes and annotation spelling.
    pub parameters: Vec<Parameter>,
    /// Explicit return annotation, or None for the default Unit convention.
    pub return_type: Option<String>,
    /// Header range against the original source, excluding body/terminator.
    pub span: Span,
}

/// A bodyless or body-bearing operation in a service container.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceOperationSyntax {
    /// Signature only: this is not a resolved service contract.
    pub signature: ServiceSignatureSyntax,
    /// Full original operation range, including terminator/body where present.
    pub span: Span,
    /// Parsed executable body when explicitly declared; None for a prototype.
    pub body: Option<CallableSyntax>,
}

/// Service declaration syntax with operations in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceSyntax {
    /// Source service name, not a resolved canonical identity.
    pub name: String,
    /// Original range including container braces.
    pub span: Span,
    /// Ordered explicit operation declarations.
    pub operations: Vec<ServiceOperationSyntax>,
}

/// Partial application syntax including service containers and prototypes.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationServiceSyntax {
    /// Previously retained type/callable nodes from the same source.
    pub base: ApplicationCallableSyntax,
    /// Service containers in source order.
    pub services: Vec<ServiceSyntax>,
    /// Remaining unparsed token envelopes, including application imports.
    /// Empty gaps still do not establish semantic validity or full M12 checking.
    pub unparsed_ranges: Vec<Span>,
}

/// Parse currently supported service syntax without resolving contracts or imports.
///
/// Uses existing application admission and executable signature parsing. Unsupported
/// syntax/quotas fail the whole request. No synthetic body is exposed for prototypes.
pub fn parse_services(source: &str) -> Result<ApplicationServiceSyntax, String> {
    parse_skipping(source, &[])
}

pub(super) fn parse_skipping(
    source: &str,
    skipped: &[Span],
) -> Result<ApplicationServiceSyntax, String> {
    let base = super::callables::parse_skipping(source, skipped)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "service syntax lexical errors")?;
    let mut top_level_ranges: Vec<_> = base
        .types
        .aliases
        .iter()
        .map(|alias| alias.span)
        .chain(base.types.records.iter().map(|record| record.span))
        .chain(
            base.callables
                .iter()
                .filter(|callable| {
                    !matches!(callable.kind, super::CallableKind::ServiceOperation { .. })
                })
                .map(|callable| callable.span),
        )
        .collect();
    top_level_ranges.extend_from_slice(skipped);
    top_level_ranges.sort_by_key(|span| span.start);
    let mut services = Vec::new();
    let mut position = 0;
    let mut depth = 0usize;
    let mut operation_count = 0;
    while position + 1 < tokens.len() {
        let known =
            top_level_ranges.partition_point(|span| span.end <= tokens[position].span.start);
        if let Some(span) = top_level_ranges
            .get(known)
            .filter(|span| span.start <= tokens[position].span.start)
        {
            position = tokens.partition_point(|token| token.span.start < span.end);
            continue;
        }

        if depth == 0
            && matches!(&tokens[position].kind, TokenKind::Identifier(name) if name == "service")
        {
            if services.len() == MAX_SERVICES {
                return Err("application service syntax container limit exceeded".into());
            }
            let start = tokens[position].span;
            let TokenKind::Identifier(name) = &tokens[position + 1].kind else {
                return Err("missing service name".into());
            };
            let name = name.clone();
            position += 3; // Existing admission validated name and opening brace.
            let mut operations = Vec::new();
            while tokens[position].kind != TokenKind::Punctuation('}') {
                operation_count += 1;
                if operation_count > MAX_SERVICE_OPERATIONS {
                    return Err("application service syntax operation limit exceeded".into());
                }
                let begin = position;
                position += 1; // fn, validated by application admission.
                while !matches!(
                    tokens[position].kind,
                    TokenKind::Punctuation('{' | '}' | ';')
                        | TokenKind::Keyword("fn")
                        | TokenKind::Eof
                ) {
                    position += 1;
                }
                let header_end = tokens[position - 1].span.end;
                let header_span = Span {
                    end: header_end,
                    ..tokens[begin].span
                };
                let body = if tokens[position].kind == TokenKind::Punctuation('{') {
                    let index = base
                        .callables
                        .partition_point(|callable| callable.span.start < tokens[begin].span.start);
                    let found = base
                        .callables
                        .get(index)
                        .filter(|callable| callable.span.start == tokens[begin].span.start)
                        .ok_or("service operation body provenance unavailable")?;
                    let end = found.span.end;
                    position = tokens.partition_point(|token| token.span.start < end);
                    Some(found.clone())
                } else {
                    None
                };
                let mut selected = tokens
                    [begin..tokens.partition_point(|token| token.span.start < header_end)]
                    .to_vec();
                // Executable parsing validates header grammar. Empty adapter braces
                // are discarded: only original signature nodes leave this helper.
                let adapter = Span {
                    start: header_end,
                    end: header_end,
                    ..header_span
                };
                selected.push(Token {
                    kind: TokenKind::Punctuation('{'),
                    span: adapter,
                });
                selected.push(Token {
                    kind: TokenKind::Punctuation('}'),
                    span: adapter,
                });
                selected.push(tokens.last().ok_or("missing EOF")?.clone());
                let mut program = Parser::new()
                    .parse_tokens(&selected)
                    .map_err(|_| "service signature outside shared executable syntax")?;
                if program.functions.len() != 1 {
                    return Err("service signature provenance mismatch".into());
                }
                let function = program.functions.remove(0);
                if body.is_none() && tokens[position].kind == TokenKind::Punctuation(';') {
                    position += 1;
                }
                let span = Span {
                    end: body
                        .as_ref()
                        .map_or_else(|| tokens[position - 1].span.end, |body| body.span.end),
                    ..header_span
                };
                operations.push(ServiceOperationSyntax {
                    signature: ServiceSignatureSyntax {
                        name: function.name,
                        parameters: function.parameters,
                        return_type: function.return_type,
                        span: header_span,
                    },
                    span,
                    body,
                });
            }
            let span = Span {
                end: tokens[position].span.end,
                ..start
            };
            position += 1;
            services.push(ServiceSyntax {
                name,
                span,
                operations,
            });
            continue;
        }
        match tokens[position].kind {
            TokenKind::Punctuation('{') => depth += 1,
            TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
            _ => {}
        }
        position += 1;
    }
    let spans: Vec<_> = services.iter().map(|service| service.span).collect();
    let unparsed_ranges = super::callables::unparsed(&tokens, &base.unparsed_ranges, &spans);
    Ok(ApplicationServiceSyntax {
        base,
        services,
        unparsed_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_empty_containers_prototypes_and_bodies_without_invented_nodes() {
        let source = "// λ\r\nuse app.remote;\r\nservice Empty{}\r\nservice Api{fn declared(x:Int)->String; fn implicit()\r\nfn implemented()->Int{return 7;}}";
        let parsed = parse_services(source).unwrap();
        assert_eq!(parsed.services.len(), 2);
        assert!(parsed.services[0].operations.is_empty());
        let operations = &parsed.services[1].operations;
        assert_eq!(operations.len(), 3);
        assert!(operations[0].body.is_none());
        assert!(operations[1].body.is_none());
        assert!(operations[2].body.is_some());
        assert_eq!(
            operations[0].signature.return_type.as_deref(),
            Some("String")
        );
        let parameter = &operations[0].signature.parameters[0];
        let annotation = parameter.type_span.unwrap();
        assert_eq!(&source[annotation.start..annotation.end], "Int");
        assert_eq!(
            &source[operations[0].span.start..operations[0].span.end],
            "fn declared(x:Int)->String;"
        );
        assert_eq!(parsed.unparsed_ranges.len(), 1);
        let gap = parsed.unparsed_ranges[0];
        assert_eq!(&source[gap.start..gap.end], "use app.remote;");
    }
    #[test]
    fn rejects_unsupported_signatures_without_asserting_semantic_contracts() {
        let names = parse_services("type service = Int; fn service(){} service Real{}").unwrap();
        assert_eq!(names.services.len(), 1);
        assert_eq!(names.services[0].name, "Real");
        assert!(parse_services("service Api{fn f(x);}").is_err());
        assert!(parse_services("service Api{fn f()->Task<Int>;}").is_err());
        let parsed = parse_services("service Api{fn f(x:Missing)->Unknown;}").unwrap();
        assert!(parsed.unparsed_ranges.is_empty());
        assert_eq!(
            parsed.services[0].operations[0].signature.parameters[0]
                .type_name
                .as_deref(),
            Some("Missing")
        );
    }
    #[test]
    fn service_and_operation_limits_accept_exact_boundaries() {
        let services = (0..MAX_SERVICES)
            .map(|n| format!("service S{n}{{}}\n"))
            .collect::<String>();
        assert_eq!(
            parse_services(&services).unwrap().services.len(),
            MAX_SERVICES
        );
        assert!(parse_services(&(services + "service Extra{}"))
            .unwrap_err()
            .contains("container limit"));
        let operations = (0..MAX_SERVICE_OPERATIONS)
            .map(|n| format!("fn f{n}();"))
            .collect::<String>();
        assert_eq!(
            parse_services(&format!("service Api{{{operations}}}"))
                .unwrap()
                .services[0]
                .operations
                .len(),
            MAX_SERVICE_OPERATIONS
        );
        assert!(
            parse_services(&format!("service Api{{{operations}fn extra();}}"))
                .unwrap_err()
                .contains("operation limit")
        );
    }
}
