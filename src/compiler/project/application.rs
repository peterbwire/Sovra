//! Experimental structured body parsing and receiver inspection.
//!
//! Supports bindings, nested blocks, returns, literals, qualified/member calls,
//! and arithmetic/comparison expressions without application type checking.
//! Unsupported syntax fails the whole inspection; no partial call list escapes.

use super::scope::{Receiver, Scope, ServiceIdentity};
use crate::compiler::diagnostics::Span;
use crate::compiler::lexer::{Lexer, Token, TokenKind};

/// Explicit per-file outcome for experimental project receiver inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInspection {
    /// Source inspected.
    pub source_file: std::path::PathBuf,
    /// Function records, or the reason this file could not be fully inspected.
    pub functions: Result<Vec<FunctionCalls>, String>,
}

/// Service-call validation with explicit per-file syntax coverage.
#[derive(Debug, Clone)]
pub struct ServiceCheck {
    /// Files successfully inspected or rejected as unsupported/partial.
    pub files: Vec<FileInspection>,
    /// Contract errors in inspected files and E4096 for each unsupported file.
    pub diagnostics: crate::compiler::diagnostics::Diagnostics,
}

/// Check resolved service member names and positional argument counts.
///
/// Requires a successfully checked project (including service manifest bindings).
/// Unsupported files remain in `files` as errors and produce E4096 diagnostics.
/// Local and unresolved receivers are not service calls.
/// Parameter and return annotation text is not used for type checking.
pub fn check_service_calls(project: &super::ProjectCheck) -> ServiceCheck {
    use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity};
    let files = inspect_project(project);
    let mut diagnostics = Diagnostics::new();
    let mut operations = std::collections::BTreeMap::new();
    for operation in &project.service_operations {
        if let Ok(module) = operation.source_file.canonicalize() {
            operations.insert(
                (
                    module.to_string_lossy().into_owned(),
                    operation.service.clone(),
                    operation.name.clone(),
                ),
                operation.parameters.len(),
            );
        }
    }
    for file in &files {
        let Ok(functions) = &file.functions else {
            continue;
        };
        for function in functions {
            for call in &function.calls {
                let issue = match &call.receiver {
                    Receiver::Service(service) => {
                        match operations.get(&(
                            service.module.clone(),
                            service.name.clone(),
                            call.operation.clone(),
                        )) {
                            None => Some((
                                "E4093",
                                format!(
                                    "service `{}` has no operation `{}`",
                                    service.name, call.operation
                                ),
                            )),
                            Some(expected) if *expected != call.arguments => Some((
                                "E4094",
                                format!(
                                    "service operation `{}.{}` expects {} argument(s), found {}",
                                    service.name, call.operation, expected, call.arguments
                                ),
                            )),
                            _ => None,
                        }
                    }
                    Receiver::Ambiguous(_) => Some((
                        "E4095",
                        format!(
                            "ambiguous service receiver for operation `{}`",
                            call.operation
                        ),
                    )),
                    Receiver::Local | Receiver::Unresolved => None,
                };
                if let Some((code, message)) = issue {
                    diagnostics.push(Diagnostic {
                        source_file: Some(file.source_file.to_string_lossy().into_owned()),
                        severity: Severity::Error,
                        code,
                        message,
                        span: call.span,
                    });
                }
            }
        }
    }
    for file in &files {
        if let Err(reason) = &file.functions {
            diagnostics.push(Diagnostic {
                source_file: None,
                severity: Severity::Error,
                code: "E4096",
                message: format!(
                    "service-call inspection incomplete for `{}`: {reason}",
                    file.source_file.display()
                ),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
            });
        }
    }
    ServiceCheck { files, diagnostics }
}

/// Inspect supported files using validated direct imports and same-file services.
/// Unsupported files return errors individually; this does not change check
/// success, enforce service contracts, or treat dependencies as transitive imports.
pub fn inspect_project(project: &super::ProjectCheck) -> Vec<FileInspection> {
    project
        .source_files
        .iter()
        .map(|file| {
            let functions = (|| {
                let own = file.canonicalize().map_err(|error| error.to_string())?;
                let mut visible = std::collections::BTreeSet::from([own]);
                for import in &project.imports {
                    if import.source_file == *file {
                        visible.insert(import.target_file.clone());
                    }
                }
                let mut services = Vec::new();
                for declaration in &project.service_declarations {
                    let module = declaration
                        .source_file
                        .canonicalize()
                        .map_err(|error| error.to_string())?;
                    if visible.contains(&module) {
                        services.push(ServiceIdentity {
                            module: module.to_string_lossy().into_owned(),
                            name: declaration.name.clone(),
                        });
                    }
                }
                let source = std::fs::read_to_string(file).map_err(|error| error.to_string())?;
                inspect_functions(&source, &services)
            })();
            FileInspection {
                source_file: file.clone(),
                functions,
            }
        })
        .collect()
}

/// A member call discovered through structured expression traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberCall {
    /// Member name invoked by the call.
    pub operation: String,
    /// Number of positional arguments.
    pub arguments: usize,
    /// Classification of the receiver; complex expressions remain unresolved.
    pub receiver: Receiver,
    /// Range of the member expression in the supplied body source.
    pub span: Span,
}

#[derive(Debug)]
enum Expression {
    Name(String, Span),
    // Namespace resolution is outside service inspection; retain the full range.
    QualifiedName(Span),
    Literal(Span),
    Member(Box<Expression>, String, Span),
    Call(Box<Expression>, Vec<Expression>, Span),
    Binary(Box<Expression>, Box<Expression>, Span),
}

impl Expression {
    fn depth(&self) -> usize {
        let mut pending = vec![(self, 1)];
        let mut maximum = 0;
        while let Some((expression, depth)) = pending.pop() {
            maximum = maximum.max(depth);
            match expression {
                Self::Binary(left, right, _) => {
                    pending.push((left, depth + 1));
                    pending.push((right, depth + 1));
                }
                Self::Member(receiver, _, _) => pending.push((receiver, depth + 1)),
                Self::Call(callee, arguments, _) => {
                    pending.push((callee, depth + 1));
                    pending.extend(arguments.iter().map(|argument| (argument, depth + 1)));
                }
                _ => {}
            }
        }
        maximum
    }
    fn span(&mut self) -> &mut Span {
        match self {
            Self::Name(_, span)
            | Self::QualifiedName(span)
            | Self::Literal(span)
            | Self::Member(_, _, span)
            | Self::Call(_, _, span)
            | Self::Binary(_, _, span) => span,
        }
    }
}

/// A function or task inspected from a complete source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCalls {
    /// Declaration name.
    pub name: String,
    /// Whether this is a task rather than an ordinary function.
    pub is_task: bool,
    /// Full declaration range in the original source file.
    pub span: Span,
    /// Structured member calls in the supported body subset.
    pub calls: Vec<MemberCall>,
}

/// Inspect a file containing top-level function/task declarations.
///
/// Signature annotations remain unresolved. Visible services must be supplied
/// by the caller; this API does not infer imports. Unsupported top-level or body
/// syntax rejects the whole inspection. All spans are relative to `source`.
pub fn inspect_functions(
    source: &str,
    services: &[ServiceIdentity],
) -> Result<Vec<FunctionCalls>, String> {
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "file contains unsupported lexical syntax".to_owned())?;
    check_nesting(&tokens)?;
    let mut parser = BodyParser {
        tokens: &tokens,
        position: 0,
        calls: Vec::new(),
    };
    let mut functions = Vec::new();
    while parser.peek() != &TokenKind::Eof {
        let start = tokens[parser.position].span;
        if parser.consume(TokenKind::Keyword("use")) {
            while parser.peek() != &TokenKind::Eof
                && tokens[parser.position].span.line == start.line
            {
                if parser.consume(TokenKind::Punctuation(';')) {
                    break;
                }
                parser.position += 1;
            }
            let end = tokens[parser.position - 1].span.end;
            super::imports::parse(&source[start.start..end]).map_err(str::to_owned)?;
            continue;
        }
        if matches!(parser.peek(), TokenKind::Identifier(name) if name == "service") {
            parser.position += 1;
            let owner = match parser.peek() {
                TokenKind::Identifier(name) => name.clone(),
                _ => return Err("expected service name".into()),
            };
            parser.position += 1;
            parser.require(TokenKind::Punctuation('{'))?;
            while !parser.consume(TokenKind::Punctuation('}')) {
                if parser.peek() != &TokenKind::Keyword("fn") {
                    return Err("expected service operation or closing brace".into());
                }
                if let Some(function) = parser.declaration(source, services, Some(&owner), false)? {
                    functions.push(function);
                }
            }
            continue;
        }
        let is_task = match parser.peek() {
            TokenKind::Keyword("fn") => false,
            TokenKind::Keyword("task") => true,
            TokenKind::Identifier(name) if name == "task" => true,
            _ => {
                return Err(format!(
                    "unsupported application declaration at byte {}",
                    start.start
                ))
            }
        };
        if let Some(function) = parser.declaration(source, services, None, is_task)? {
            functions.push(function);
        }
    }
    Ok(functions)
}

/// Inspect one braced body using explicitly supplied parameters and services.
/// This does not validate service operations, argument types or imports. Offsets
/// are relative to `source`. No source substring matching is used for calls.
pub fn inspect_body(
    source: &str,
    parameters: &[&str],
    services: &[ServiceIdentity],
) -> Result<Vec<MemberCall>, String> {
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "body contains unsupported lexical syntax".to_owned())?;
    check_nesting(&tokens)?;
    let mut parser = BodyParser {
        tokens: &tokens,
        position: 0,
        calls: Vec::new(),
    };
    let mut root = Scope::new();
    for service in services {
        root.add_service(service.clone());
    }
    for parameter in parameters {
        root.bind(*parameter, 0);
    }
    parser.block(&root)?;
    if parser.peek() != &TokenKind::Eof {
        return Err("unexpected text after application body".into());
    }
    Ok(parser.calls)
}

struct BodyParser<'a> {
    tokens: &'a [Token],
    position: usize,
    calls: Vec<MemberCall>,
}

fn check_nesting(tokens: &[Token]) -> Result<(), String> {
    crate::compiler::limits::check_nesting(tokens, true).map_err(depth_error)
}

fn depth_error(span: Span) -> String {
    format!(
        "{} at byte {}",
        crate::compiler::limits::depth_message(),
        span.start
    )
}

fn check_depth(depth: usize, span: Span) -> Result<(), String> {
    if depth > crate::compiler::limits::MAX_STRUCTURAL_DEPTH {
        Err(depth_error(span))
    } else {
        Ok(())
    }
}

impl BodyParser<'_> {
    fn declaration(
        &mut self,
        source: &str,
        services: &[ServiceIdentity],
        owner: Option<&str>,
        is_task: bool,
    ) -> Result<Option<FunctionCalls>, String> {
        let start = self.tokens[self.position].span;
        let keyword_end = start.end;
        self.position += 1;
        while !matches!(
            self.peek(),
            TokenKind::Punctuation('{' | '}' | ';') | TokenKind::Keyword("fn") | TokenKind::Eof
        ) {
            self.position += 1;
        }
        let header = source[keyword_end..self.tokens[self.position].span.start]
            .lines()
            .map(|line| super::strip_line_comment(line, "//"))
            .collect::<Vec<_>>()
            .join("\n");
        let header = format!("fn {header}");
        let operation = super::service_contract::parse_operation(&header)
            .map_err(str::to_owned)?
            .ok_or("missing declaration signature")?;
        if let Some(parameter) = operation
            .declarations
            .iter()
            .find(|item| item.annotation.is_none())
        {
            return Err(format!(
                "parameter `{}` requires an explicit type annotation at byte {}",
                parameter.name, start.start
            ));
        }
        if self.peek() != &TokenKind::Punctuation('{') {
            if owner.is_none() {
                return Err("missing function/task body".into());
            }
            self.consume(TokenKind::Punctuation(';'));
            return Ok(None);
        }
        let mut scope = Scope::new();
        for service in services {
            scope.add_service(service.clone());
        }
        for parameter in &operation.declarations {
            scope.bind(parameter.name, start.start);
        }
        self.block(&scope)?;
        Ok(Some(FunctionCalls {
            name: owner.map_or_else(
                || operation.name.to_owned(),
                |owner| format!("{owner}.{}", operation.name),
            ),
            is_task,
            span: Span {
                end: self.tokens[self.position - 1].span.end,
                ..start
            },
            calls: std::mem::take(&mut self.calls),
        }))
    }
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.position].kind
    }
    fn consume(&mut self, kind: TokenKind) -> bool {
        if self.peek() == &kind {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn require(&mut self, kind: TokenKind) -> Result<(), String> {
        if self.consume(kind) {
            Ok(())
        } else {
            Err(format!(
                "unsupported or incomplete application syntax at byte {}",
                self.tokens[self.position].span.start
            ))
        }
    }
    fn block(&mut self, parent: &Scope<'_>) -> Result<(), String> {
        self.require(TokenKind::Punctuation('{'))?;
        let mut scope = parent.child();
        while !self.consume(TokenKind::Punctuation('}')) {
            if self.peek() == &TokenKind::Eof {
                return Err("unclosed application block".into());
            }
            if self.peek() == &TokenKind::Punctuation('{') {
                self.block(&scope)?;
                continue;
            }
            if self.consume(TokenKind::Keyword("let")) {
                let TokenKind::Identifier(name) = self.peek().clone() else {
                    return Err("expected local binding name".into());
                };
                self.position += 1;
                if self.consume(TokenKind::Punctuation(':')) {
                    self.local_annotation()?;
                }
                self.require(TokenKind::Operator("="))?;
                let expression = self.expression()?;
                self.inspect(&expression, &scope);
                scope.bind(name, self.tokens[self.position - 1].span.end);
            } else {
                let returning = self.consume(TokenKind::Keyword("return"));
                if !returning || !matches!(self.peek(), TokenKind::Punctuation(';' | '}')) {
                    let expression = self.expression()?;
                    self.inspect(&expression, &scope);
                }
            }
            self.consume(TokenKind::Punctuation(';'));
        }
        Ok(())
    }
    // Consume structural annotation tokens without resolving their type names.
    // Stop only at a top-level initializer marker, never at arbitrary body text.
    fn local_annotation(&mut self) -> Result<(), String> {
        let mut delimiters = Vec::new();
        let mut has_name = false;
        loop {
            match self.peek() {
                TokenKind::Operator("=") if delimiters.is_empty() && has_name => return Ok(()),
                TokenKind::Identifier(_) => has_name = true,
                TokenKind::Operator("<") => delimiters.push('<'),
                TokenKind::Punctuation('(') => delimiters.push('('),
                TokenKind::Punctuation('[') => delimiters.push('['),
                TokenKind::Operator(">") | TokenKind::Punctuation(')' | ']') => {
                    let expected = match self.peek() {
                        TokenKind::Operator(">") => '<',
                        TokenKind::Punctuation(')') => '(',
                        _ => '[',
                    };
                    if delimiters.pop() != Some(expected) {
                        return Err("mismatched local annotation delimiter".into());
                    }
                }
                TokenKind::Punctuation(',') if !delimiters.is_empty() => {}
                TokenKind::Operator("->") => {}
                _ => return Err("unsupported or incomplete local annotation".into()),
            }
            self.position += 1;
        }
    }

    fn expression(&mut self) -> Result<Expression, String> {
        self.binary_expression(1)
    }

    fn binary_expression(&mut self, minimum: u8) -> Result<Expression, String> {
        let mut left = self.postfix_expression()?;
        loop {
            let precedence = match self.tokens[self.position].kind {
                TokenKind::Operator("==" | "!=" | "<" | "<=" | ">" | ">=") => 1,
                TokenKind::Operator("+" | "-") => 2,
                TokenKind::Operator("*" | "/") => 3,
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            let operator_span = self.tokens[self.position].span;
            self.position += 1;
            let mut right = self.binary_expression(precedence + 1)?;
            check_depth(1 + left.depth().max(right.depth()), operator_span)?;
            let span = Span {
                end: right.span().end,
                ..*left.span()
            };
            left = Expression::Binary(Box::new(left), Box::new(right), span);
        }
        Ok(left)
    }

    fn postfix_expression(&mut self) -> Result<Expression, String> {
        let token = &self.tokens[self.position];
        let mut expression = match &token.kind {
            TokenKind::Identifier(name) => {
                let value = Expression::Name(name.clone(), token.span);
                self.position += 1;
                value
            }
            TokenKind::String(_)
            | TokenKind::Integer(_)
            | TokenKind::Float(_)
            | TokenKind::Keyword("true" | "false") => {
                self.position += 1;
                Expression::Literal(token.span)
            }
            TokenKind::Punctuation('(') => {
                self.position += 1;
                let mut value = self.expression()?;
                self.require(TokenKind::Punctuation(')'))?;
                *value.span() = Span {
                    end: self.tokens[self.position - 1].span.end,
                    ..token.span
                };
                value
            }
            _ => {
                return Err(format!(
                    "unsupported application expression at byte {}",
                    token.span.start
                ))
            }
        };
        if self.consume(TokenKind::Operator("::")) {
            let Expression::Name(_, start) = expression else {
                return Err("expected a module name before `::`".into());
            };
            if !matches!(self.peek(), TokenKind::Identifier(_)) {
                return Err("expected a module member name after `::`".into());
            }
            expression = Expression::QualifiedName(Span {
                end: self.tokens[self.position].span.end,
                ..start
            });
            self.position += 1;
        }
        loop {
            if self.consume(TokenKind::Punctuation('.')) {
                let token = &self.tokens[self.position];
                let TokenKind::Identifier(name) = &token.kind else {
                    return Err("expected member name".into());
                };
                let start = *expression.span();
                check_depth(1 + expression.depth(), token.span)?;
                expression = Expression::Member(
                    Box::new(expression),
                    name.clone(),
                    Span {
                        end: token.span.end,
                        ..start
                    },
                );
                self.position += 1;
            } else if self.consume(TokenKind::Punctuation('(')) {
                let mut arguments = Vec::new();
                if !self.consume(TokenKind::Punctuation(')')) {
                    loop {
                        arguments.push(self.expression()?);
                        if self.consume(TokenKind::Punctuation(')')) {
                            break;
                        }
                        self.require(TokenKind::Punctuation(','))?;
                        if self.consume(TokenKind::Punctuation(')')) {
                            break;
                        }
                    }
                }
                let span = Span {
                    end: self.tokens[self.position - 1].span.end,
                    ..*expression.span()
                };
                let depth = 1 + arguments
                    .iter()
                    .map(Expression::depth)
                    .chain(std::iter::once(expression.depth()))
                    .max()
                    .unwrap_or(0);
                check_depth(depth, span)?;
                expression = Expression::Call(Box::new(expression), arguments, span);
            } else {
                break;
            }
        }
        Ok(expression)
    }
    fn inspect(&mut self, expression: &Expression, scope: &Scope<'_>) {
        match expression {
            Expression::Call(callee, arguments, _) => {
                self.inspect(callee, scope);
                for argument in arguments {
                    self.inspect(argument, scope);
                }
                if let Expression::Member(receiver, operation, span) = callee.as_ref() {
                    let receiver = match receiver.as_ref() {
                        Expression::Name(name, location) => scope.resolve(name, location.start),
                        _ => Receiver::Unresolved,
                    };
                    self.calls.push(MemberCall {
                        operation: operation.clone(),
                        arguments: arguments.len(),
                        receiver,
                        span: *span,
                    });
                }
            }
            Expression::Member(receiver, _, _) => self.inspect(receiver, scope),
            Expression::Binary(left, right, _) => {
                self.inspect(left, scope);
                self.inspect(right, scope);
            }
            Expression::Name(_, _) | Expression::QualifiedName(_) | Expression::Literal(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_rejects_untyped_declaration_parameters() {
        for source in [
            "fn example(value) {}",
            "task example(value) {}",
            "service mail { fn send(value); }",
        ] {
            let error = inspect_functions(source, &[]).unwrap_err();
            assert!(error.contains("explicit type annotation"), "{error}");
        }
    }

    #[test]
    fn service_bodies_use_explicit_file_and_independent_lexical_scopes() {
        let source = "service maps { fn send(x: Int); fn relay() { maps.send(1); let maps = maps.make(); maps.local(); } fn shadow(maps: Client) { maps.local(); } fn next() { maps.send(2); } }";
        let functions = inspect_functions(source, &[service()]).unwrap();
        assert_eq!(
            functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            ["maps.relay", "maps.shadow", "maps.next"]
        );
        assert_eq!(functions[0].calls[0].receiver, Receiver::Service(service()));
        assert_eq!(functions[0].calls[1].receiver, Receiver::Service(service()));
        assert_eq!(functions[0].calls[2].receiver, Receiver::Local);
        assert_eq!(functions[1].calls[0].receiver, Receiver::Local);
        assert_eq!(functions[2].calls[0].receiver, Receiver::Service(service()));
        for function in functions {
            assert!(!function.is_task);
            for call in function.calls {
                assert!(source[call.span.start..call.span.end].starts_with("maps."));
            }
        }
    }

    #[test]
    fn application_depth_limits_cover_blocks_trees_and_nesting() {
        for (body, accepted) in [
            (
                format!(
                    "{}{}1{}{}",
                    "{".repeat(128),
                    "(".repeat(128),
                    ")".repeat(128),
                    "}".repeat(128)
                ),
                true,
            ),
            (format!("{}{}", "{".repeat(128), "}".repeat(128)), true),
            (format!("{}{}", "{".repeat(129), "}".repeat(129)), false),
            (
                format!("{{ {}1{} }}", "(".repeat(128), ")".repeat(128)),
                true,
            ),
            (
                format!("{{ {}1{} }}", "(".repeat(129), ")".repeat(129)),
                false,
            ),
            (format!("{{ {} }}", vec!["1"; 128].join("+")), true),
            (format!("{{ {} }}", vec!["1"; 129].join("+")), false),
            (
                format!("{{ {}1{} }}", "f(".repeat(127), ")".repeat(127)),
                true,
            ),
            (
                format!("{{ {}1{} }}", "f(".repeat(128), ")".repeat(128)),
                false,
            ),
            (format!("{{ maps{}() }}", ".member".repeat(126)), true),
            (format!("{{ maps{}() }}", ".member".repeat(127)), false),
            (format!("{{ f({}) }}", vec!["1"; 1024].join(",")), true),
        ] {
            let result = inspect_body(&body, &[], &[service()]);
            if accepted {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result
                    .unwrap_err()
                    .contains("maximum structural depth of 128"));
            }
        }
        let source = format!(
            "fn good() {{ maps.send(); }} fn bad() {{ {}1{} }}",
            "(".repeat(2048),
            ")".repeat(2048)
        );
        assert!(inspect_functions(&source, &[service()])
            .unwrap_err()
            .contains("128"));
    }

    #[test]
    fn complex_receiver_ranges_cover_the_entire_member_expression() {
        for (expression, expected) in [
            ("maps.make().send()", "maps.make().send"),
            ("(maps.make()).send()", "(maps.make()).send"),
            ("(maps.make() + 1).send()", "(maps.make() + 1).send"),
            ("\"text\".send()", "\"text\".send"),
            ("(maps).send()", "(maps).send"),
        ] {
            let source = format!("{{ // Unicode: λ\r\n  {expression}; }}");
            let calls = inspect_body(&source, &[], &[service()]).unwrap();
            let call = calls.last().unwrap();
            assert_eq!(&source[call.span.start..call.span.end], expected);
            assert_eq!(call.span.line, 1);
            assert_eq!(call.span.column, 2);
            assert_eq!(
                call.receiver,
                if expression == "(maps).send()" {
                    Receiver::Service(service())
                } else {
                    Receiver::Unresolved
                }
            );
        }
    }

    #[test]
    fn qualified_calls_inspect_arguments_without_becoming_services() {
        let source =
            "{ std::println(maps.send(1)); maps::send(2); maps::client.send(maps.make()); }";
        let calls = inspect_body(source, &[], &[service()]).unwrap();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].operation, "send");
        assert_eq!(calls[0].receiver, Receiver::Service(service()));
        assert_eq!(calls[1].operation, "make");
        assert_eq!(calls[1].receiver, Receiver::Service(service()));
        assert_eq!(calls[2].receiver, Receiver::Unresolved);
        assert_eq!(
            &source[calls[2].span.start..calls[2].span.end],
            "maps::client.send"
        );
    }

    #[test]
    fn malformed_qualified_names_discard_inspection() {
        for expression in [
            "std::",
            "std::(1)",
            "1::send()",
            "maps.send::other()",
            "std::io::print(1)",
        ] {
            let source = format!("{{ maps.first(); {expression}; }}");
            assert!(
                inspect_body(&source, &[], &[service()]).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn unsupported_service_implementations_discard_inspection() {
        for body in ["if true {}", "let x = ;", "maps.send("] {
            let source =
                format!("fn first() {{ maps.send(); }} service maps {{ fn send() {{ {body} }} }}");
            assert!(inspect_functions(&source, &[service()]).is_err());
        }
    }

    #[test]
    fn empty_service_bodies_and_signatures_have_no_hidden_calls() {
        // Line comments and whitespace do not constitute an implementation.
        let source = "service maps { fn send(); fn empty() { // maps.hidden()\n } }";
        let functions = inspect_functions(source, &[service()]).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "maps.empty");
        assert!(functions[0].calls.is_empty());
    }

    #[test]
    fn binary_operands_preserve_calls_arguments_and_shadowing() {
        let source = "{ let maps = maps.first() + maps.second() * 2; maps.local(); return other.send((maps.third() - 1) / 2 >= maps.fourth(), 1 != 2); }";
        let calls = inspect_body(source, &[], &[service()]).unwrap();
        assert_eq!(
            calls
                .iter()
                .map(|call| call.operation.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "local", "third", "fourth", "send"]
        );
        for call in &calls[..2] {
            assert_eq!(call.receiver, Receiver::Service(service()));
        }
        for call in &calls[2..5] {
            assert_eq!(call.receiver, Receiver::Local);
        }
        assert_eq!(calls[5].arguments, 2);
    }

    #[test]
    fn incomplete_binary_operands_discard_all_calls() {
        for expression in [
            "maps.send() +",
            "1 * / maps.send()",
            "maps.send() ==",
            "1 + (maps.send() * )",
        ] {
            let source = format!("{{ maps.first(); {expression}; }}");
            assert!(
                inspect_body(&source, &[], &[service()]).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn typed_locals_preserve_initializer_and_shadowing_order() {
        for annotation in [
            "MapClient",
            "Result<MapClient, Error>",
            "(Int, Text) -> MapClient",
            "[MapClient]",
        ] {
            let source = format!("{{ let maps: {annotation} = maps.create(); maps.send(); }}");
            let calls = inspect_body(&source, &[], &[service()]).unwrap();
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0].receiver, Receiver::Service(service()));
            assert_eq!(calls[1].receiver, Receiver::Local);
        }
    }

    #[test]
    fn malformed_local_annotations_discard_all_inspection_results() {
        for annotation in ["", "Result<Text, Error", "Text]", "Text, Int", "Text + Int"] {
            let source = format!("{{ maps.send(); let value: {annotation} = maps.create(); }}");
            assert!(
                inspect_body(&source, &[], &[service()]).is_err(),
                "{annotation}"
            );
        }
    }
    fn service() -> ServiceIdentity {
        ServiceIdentity {
            module: "app.services".into(),
            name: "maps".into(),
        }
    }

    #[test]
    fn file_declarations_supply_parameters_and_preserve_source_spans() {
        let source = "// Unicode: λ\r\nfn first(\n maps: Object // shadows import\n) -> Result<Text, Error> { return maps.send(); }\ntask second() { maps.send(1); }";
        let functions = inspect_functions(source, &[service()]).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "first");
        assert!(!functions[0].is_task);
        assert_eq!(functions[0].calls[0].receiver, Receiver::Local);
        assert!(functions[1].is_task);
        assert_eq!(functions[1].calls[0].receiver, Receiver::Service(service()));
        for function in functions {
            let call = &function.calls[0];
            assert_eq!(&source[call.span.start..call.span.end], "maps.send");
            assert!(source[function.span.start..function.span.end].ends_with('}'));
        }
    }

    #[test]
    fn file_inspection_rejects_unsupported_and_incomplete_declarations() {
        for source in [
            "fn good() {} fn bad() { if true {} }",
            "fn missing()",
            "fn missing() fn next() {}",
            "fn bad(); {}",
            "use ../services\nfn main() {}",
            "model Example {}",
        ] {
            assert!(inspect_functions(source, &[service()]).is_err(), "{source}");
        }
    }

    #[test]
    fn parsed_bindings_and_blocks_control_receiver_resolution() {
        let source =
            "{ maps.travel(1); { let maps = maps.make(); maps.travel(2); } maps.travel(3); }";
        let calls = inspect_body(source, &[], &[service()]).unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].receiver, Receiver::Service(service()));
        assert_eq!(calls[1].receiver, Receiver::Service(service()));
        assert_eq!(calls[2].receiver, Receiver::Local);
        assert_eq!(calls[3].receiver, Receiver::Service(service()));
        assert_eq!(
            &source[calls[0].span.start..calls[0].span.end],
            "maps.travel"
        );
        assert_eq!(calls[0].arguments, 1);
    }

    #[test]
    fn comments_strings_parameters_and_nested_calls_are_structural() {
        let calls = inspect_body(
            "{ // maps.fake()\n print(\"maps.fake()\"); return maps.send(other.make()); }",
            &["maps"],
            &[service()],
        )
        .unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].receiver, Receiver::Unresolved);
        assert_eq!(calls[1].receiver, Receiver::Local);
    }

    #[test]
    fn unsupported_or_incomplete_bodies_do_not_return_partial_calls() {
        for source in [
            "{ maps.send(); if true {} }",
            "{ maps.send(); let f = fn value => value; }",
            "{ maps.send(",
            "{ maps.send();",
            "{ let x: = 1; }",
            "{ maps.send() && true; }",
        ] {
            assert!(inspect_body(source, &[], &[service()]).is_err(), "{source}");
        }
    }
}
