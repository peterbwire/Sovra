//! Experimental structured body parsing and receiver inspection.
//!
//! Only bindings, nested blocks, returns, literals and calls are supported.
//! Unsupported syntax fails the whole inspection; no partial call list escapes.

use super::scope::{Receiver, Scope, ServiceIdentity};
use crate::compiler::diagnostics::Span;
use crate::compiler::lexer::{Lexer, Token, TokenKind};

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
    Literal,
    Member(Box<Expression>, String, Span),
    Call(Box<Expression>, Vec<Expression>),
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

impl BodyParser<'_> {
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
    fn expression(&mut self) -> Result<Expression, String> {
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
                Expression::Literal
            }
            TokenKind::Punctuation('(') => {
                self.position += 1;
                let value = self.expression()?;
                self.require(TokenKind::Punctuation(')'))?;
                value
            }
            _ => {
                return Err(format!(
                    "unsupported application expression at byte {}",
                    token.span.start
                ))
            }
        };
        loop {
            if self.consume(TokenKind::Punctuation('.')) {
                let token = &self.tokens[self.position];
                let TokenKind::Identifier(name) = &token.kind else {
                    return Err("expected member name".into());
                };
                let start = match &expression {
                    Expression::Name(_, span) | Expression::Member(_, _, span) => *span,
                    _ => token.span,
                };
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
                expression = Expression::Call(Box::new(expression), arguments);
            } else {
                break;
            }
        }
        Ok(expression)
    }
    fn inspect(&mut self, expression: &Expression, scope: &Scope<'_>) {
        match expression {
            Expression::Call(callee, arguments) => {
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
            Expression::Name(_, _) | Expression::Literal => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn service() -> ServiceIdentity {
        ServiceIdentity {
            module: "app.services".into(),
            name: "maps".into(),
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
            "{ let x: Text = 1; }",
            "{ maps.send() + 1; }",
        ] {
            assert!(inspect_body(source, &[], &[service()]).is_err(), "{source}");
        }
    }
}
