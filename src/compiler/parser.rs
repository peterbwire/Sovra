//! Recursive-descent parser for the M2 grammar.

use crate::compiler::ast::{Expression, Function, Parameter, Program, Statement};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::compiler::lexer::{Lexer, Token, TokenKind};

/// The Sovra parser.
#[derive(Debug, Default)]
pub struct Parser;

impl Parser {
    /// Construct a parser.
    pub const fn new() -> Self {
        Self
    }

    /// Lex and parse a complete source file.
    pub fn parse_source(&self, source: &str) -> Result<Program, Diagnostics> {
        let tokens = Lexer::new().tokenize(source)?;
        self.parse_tokens(&tokens)
    }

    /// Parse a token stream that ends with an EOF token.
    pub fn parse_tokens(&self, tokens: &[Token]) -> Result<Program, Diagnostics> {
        let mut parser = TokenParser {
            tokens,
            position: 0,
            diagnostics: Diagnostics::new(),
        };
        let program = parser.program();
        if parser.diagnostics.is_empty() {
            Ok(program)
        } else {
            Err(parser.diagnostics)
        }
    }
}

struct TokenParser<'a> {
    tokens: &'a [Token],
    position: usize,
    diagnostics: Diagnostics,
}

impl<'a> TokenParser<'a> {
    fn program(&mut self) -> Program {
        let mut functions = Vec::new();
        let mut modules = Vec::new();
        while !self.at_eof() {
            if self.check_keyword("mod") {
                if let Some(module) = self.module() {
                    modules.push(module);
                }
            } else if self.check_keyword("fn") || self.check_keyword("export") {
                if let Some(function) = self.function() {
                    functions.push(function);
                }
            } else {
                self.error("E2000", "expected a function or module declaration");
                self.advance();
            }
        }
        Program { functions, modules }
    }

    fn module(&mut self) -> Option<crate::compiler::ast::Module> {
        let start = self.expect_keyword("mod")?.span;
        let name = self.expect_identifier("module name")?;
        self.expect_punctuation('{');
        let mut functions = Vec::new();
        while !self.check_punctuation('}') && !self.at_eof() {
            if self.check_keyword("fn") || self.check_keyword("export") {
                if let Some(function) = self.function() {
                    functions.push(function);
                }
            } else {
                self.error("E2000", "expected a function declaration in module");
                self.advance();
            }
        }
        let end = self.expect_punctuation('}').unwrap_or(start);
        Some(crate::compiler::ast::Module {
            name,
            functions,
            span: Span {
                end: end.end,
                ..start
            },
        })
    }

    fn function(&mut self) -> Option<Function> {
        let is_exported = self.consume_keyword("export");
        let start = self.expect_keyword("fn")?.span;
        let name = self.expect_identifier("function name")?;
        self.expect_punctuation('(');
        let mut parameters = Vec::new();
        while !self.check_punctuation(')') && !self.at_eof() {
            let parameter_start = self.peek().span;
            let parameter_name = self.expect_identifier("parameter name")?;
            let type_name = if self.consume_punctuation(':') {
                self.expect_identifier("parameter type")
            } else {
                None
            };
            parameters.push(Parameter {
                name: parameter_name,
                type_name,
                span: parameter_start,
            });
            if !self.consume_punctuation(',') {
                break;
            }
        }
        self.expect_punctuation(')');
        let return_type = if self.consume_operator("->") {
            self.expect_identifier("return type")
        } else {
            None
        };
        self.expect_punctuation('{');
        let mut body = Vec::new();
        while !self.check_punctuation('}') && !self.at_eof() {
            if let Some(statement) = self.statement() {
                body.push(statement);
            } else {
                self.synchronize_statement();
            }
        }
        let end = self.expect_punctuation('}').unwrap_or(start);
        Some(Function {
            name,
            is_exported,
            parameters,
            return_type,
            body,
            span: Span {
                end: end.end,
                ..start
            },
        })
    }

    fn statement(&mut self) -> Option<Statement> {
        if self.consume_keyword("let") {
            let start = self.previous().span;
            let name = self.expect_identifier("binding name")?;
            let type_name = if self.consume_punctuation(':') {
                Some(self.expect_identifier("binding type")?)
            } else {
                None
            };
            self.expect_operator("=");
            let value = self.expression()?;
            self.consume_punctuation(';');
            return Some(Statement::Let {
                name,
                type_name,
                value,
                span: Span {
                    end: self.previous().span.end,
                    ..start
                },
            });
        }
        if self.consume_keyword("return") {
            let start = self.previous().span;
            let value = if self.check_punctuation(';') || self.check_punctuation('}') {
                None
            } else {
                Some(self.expression()?)
            };
            self.consume_punctuation(';');
            return Some(Statement::Return {
                value,
                span: Span {
                    end: self.previous().span.end,
                    ..start
                },
            });
        }
        let expression = self.expression()?;
        self.consume_punctuation(';');
        Some(Statement::Expression(expression))
    }

    fn expression(&mut self) -> Option<Expression> {
        self.binary_expression(0)
    }

    fn binary_expression(&mut self, minimum_precedence: u8) -> Option<Expression> {
        let mut left = self.primary()?;
        while let TokenKind::Operator(operator) = &self.peek().kind {
            let precedence = precedence(operator);
            if precedence < minimum_precedence {
                break;
            }
            let operator = (*operator).to_owned();
            self.advance();
            let right = self.binary_expression(precedence + 1)?;
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
        Some(left)
    }

    fn primary(&mut self) -> Option<Expression> {
        let token = self.peek().clone();
        let mut expression = match token.kind {
            TokenKind::String(value) => {
                self.advance();
                Expression::String(value)
            }
            TokenKind::Integer(value) => {
                self.advance();
                Expression::Integer(value)
            }
            TokenKind::Float(value) => {
                self.advance();
                Expression::Float(value)
            }
            TokenKind::Keyword("true") => {
                self.advance();
                Expression::Boolean(true)
            }
            TokenKind::Keyword("false") => {
                self.advance();
                Expression::Boolean(false)
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Expression::Identifier(name)
            }
            TokenKind::Punctuation('(') => {
                self.advance();
                let expression = self.expression();
                self.expect_punctuation(')');
                expression?
            }
            _ => {
                self.error("E2001", "expected an expression");
                return None;
            }
        };
        if self.consume_operator("::") {
            let name = self.expect_identifier("module member name")?;
            expression = Expression::QualifiedName {
                path: match expression {
                    Expression::Identifier(module) => vec![module, name],
                    Expression::QualifiedName { mut path } => {
                        path.push(name);
                        path
                    }
                    _ => {
                        self.error("E2001", "expected a module path before `::`");
                        vec![]
                    }
                },
            };
        }
        if self.consume_punctuation('(') {
            let mut arguments = Vec::new();
            while !self.check_punctuation(')') && !self.at_eof() {
                arguments.push(self.expression()?);
                if !self.consume_punctuation(',') {
                    break;
                }
            }
            self.expect_punctuation(')');
            Some(Expression::Call {
                callee: Box::new(expression),
                arguments,
            })
        } else {
            Some(expression)
        }
    }

    fn synchronize_statement(&mut self) {
        while !self.at_eof() && !self.check_punctuation('}') {
            if self.consume_punctuation(';') {
                break;
            }
            self.advance();
        }
    }

    fn expect_identifier(&mut self, description: &str) -> Option<String> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Some(name)
            }
            _ => {
                self.error("E2002", format!("expected {description}"));
                None
            }
        }
    }

    fn expect_keyword(&mut self, keyword: &'static str) -> Option<Token> {
        if self.check_keyword(keyword) {
            Some(self.advance())
        } else {
            self.error("E2003", format!("expected keyword `{keyword}`"));
            None
        }
    }

    fn expect_operator(&mut self, operator: &'static str) {
        if !self.consume_operator(operator) {
            self.error("E2004", format!("expected operator `{operator}`"));
        }
    }

    fn expect_punctuation(&mut self, punctuation: char) -> Option<Span> {
        if self.check_punctuation(punctuation) {
            return Some(self.advance().span);
        }
        self.error("E2005", format!("expected `{punctuation}`"));
        None
    }

    fn consume_keyword(&mut self, keyword: &'static str) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_operator(&mut self, operator: &'static str) -> bool {
        if matches!(self.peek().kind, TokenKind::Operator(value) if value == operator) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_punctuation(&mut self, punctuation: char) -> bool {
        if self.check_punctuation(punctuation) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_keyword(&self, keyword: &str) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(value) if value == keyword)
    }

    fn check_punctuation(&self, punctuation: char) -> bool {
        self.peek().kind == TokenKind::Punctuation(punctuation)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position.min(self.tokens.len().saturating_sub(1))]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.position.saturating_sub(1)]
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !self.at_eof() {
            self.position += 1;
        }
        token
    }

    fn at_eof(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            span: self.peek().span,
        });
    }
}

fn precedence(operator: &str) -> u8 {
    match operator {
        "==" | "!=" | "<" | "<=" | ">" | ">=" => 1,
        "+" | "-" => 2,
        "*" | "/" => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_function_and_call() {
        let program = Parser::new()
            .parse_source("fn main() { let message = \"hi\"; print(message) }")
            .expect("source should parse");
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
        assert_eq!(program.functions[0].body.len(), 2);
    }

    #[test]
    fn parses_parameters_and_precedence() {
        let program = Parser::new()
            .parse_source("fn add(a: Int, b: Int) -> Int { return a + b * 2 }")
            .expect("source should parse");
        assert_eq!(program.functions[0].parameters.len(), 2);
        assert_eq!(program.functions[0].return_type.as_deref(), Some("Int"));
        assert!(matches!(
            program.functions[0].body[0],
            Statement::Return { .. }
        ));
    }

    #[test]
    fn parses_module_functions_and_qualified_calls() {
        let program = Parser::new()
            .parse_source(
                "mod math { export fn add(a: Int, b: Int) -> Int { return a + b } } fn main() { print(math::add(2, 3)) }",
            )
            .expect("source should parse");
        assert_eq!(program.modules.len(), 1);
        assert_eq!(program.modules[0].name, "math");
        assert_eq!(program.modules[0].functions[0].name, "add");
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn parses_std_namespace_calls() {
        let program = Parser::new()
            .parse_source("fn main() { std::println(42) }")
            .expect("source should parse");
        assert!(matches!(
            &program.functions[0].body[0],
            Statement::Expression(Expression::Call { callee, .. })
                if matches!(callee.as_ref(), Expression::QualifiedName { path } if path == &["std".to_owned(), "println".to_owned()])
        ));
    }

    #[test]
    fn reports_invalid_top_level_syntax() {
        let diagnostics = Parser::new()
            .parse_source("let value = 1")
            .expect_err("source should fail");
        assert_eq!(diagnostics.items[0].code, "E2000");
    }
}
