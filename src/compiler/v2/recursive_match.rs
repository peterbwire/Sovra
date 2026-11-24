//! Program-only recursive match-arm bodies; earlier standalone syntax stays strict.
use super::*;
use crate::compiler::{
    diagnostics::{Diagnostic, Diagnostics, Severity, Span},
    lexer::{Token, TokenKind},
    parser::Parser,
};
/// A match header and separate recursive arm bodies, preserving earlier header shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveMatchSyntax {
    /// Original scrutinee/pattern metadata; each header arm's core body is empty.
    pub header: MatchStatementSyntax,
    /// Complete bodies aligned one-to-one with header arms; not projected into v1 AST.
    pub bodies: Vec<Vec<EnumFunctionStatement>>,
}
/// A program control header with separately retained complete versioned branches.
#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveControlSyntax {
    /// Original If/While condition/span; its legacy body vectors are empty headers only.
    pub header: crate::compiler::ast::Statement,
    /// Then and optional else bodies for If, or one loop body for While.
    pub bodies: Vec<Vec<EnumFunctionStatement>>,
}
fn error(code: &'static str, message: &'static str, span: Span) -> EnumFunctionError {
    EnumFunctionError::Syntax(Diagnostics {
        items: vec![Diagnostic {
            source_file: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
        }],
    })
}
struct BodyParser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    limits: EnumFunctionLimits,
    arms: usize,
    text: usize,
    depth: usize,
}
impl BodyParser<'_> {
    fn punctuation(&mut self, c: char) -> Result<(), EnumFunctionError> {
        if self.tokens[self.cursor].kind != TokenKind::Punctuation(c) {
            return Err(error(
                "E2010",
                "expected match/body punctuation",
                self.tokens[self.cursor].span,
            ));
        }
        self.cursor += 1;
        Ok(())
    }
    fn operator(&mut self, c: &'static str) -> Result<(), EnumFunctionError> {
        if self.tokens[self.cursor].kind != TokenKind::Operator(c) {
            return Err(error(
                "E2010",
                "expected match pattern operator",
                self.tokens[self.cursor].span,
            ));
        }
        self.cursor += 1;
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), EnumFunctionError> {
        self.text = self
            .text
            .checked_add(n)
            .filter(|n| *n <= self.limits.max_match_header_text_bytes)
            .ok_or(EnumFunctionError::Limit("match header text"))?;
        Ok(())
    }
    fn name(&mut self) -> Result<(String, Span), EnumFunctionError> {
        let TokenKind::Identifier(name) = &self.tokens[self.cursor].kind else {
            return Err(error(
                "E2010",
                "expected match identifier",
                self.tokens[self.cursor].span,
            ));
        };
        self.charge(name.len())?;
        let result = (name.clone(), self.tokens[self.cursor].span);
        self.cursor += 1;
        Ok(result)
    }
    fn body(&mut self, root: bool) -> Result<Vec<EnumFunctionStatement>, EnumFunctionError> {
        self.punctuation('{')?;
        let mut body = Vec::new();
        while self.tokens[self.cursor].kind != TokenKind::Punctuation('}') {
            if root && body.len() >= self.limits.max_statements {
                return Err(EnumFunctionError::Limit("statements"));
            }
            body.push(self.statement()?);
        }
        self.cursor += 1;
        Ok(body)
    }
    fn statement(&mut self) -> Result<EnumFunctionStatement, EnumFunctionError> {
        if self.depth >= crate::compiler::limits::MAX_STRUCTURAL_DEPTH {
            return Err(error(
                "E2007",
                "maximum structural depth exceeded",
                self.tokens[self.cursor].span,
            ));
        }
        self.depth += 1;
        let result = self.statement_inner();
        self.depth -= 1;
        result
    }
    fn statement_inner(&mut self) -> Result<EnumFunctionStatement, EnumFunctionError> {
        if matches!(
            self.tokens[self.cursor].kind,
            TokenKind::Keyword("if" | "while")
        ) {
            return Ok(EnumFunctionStatement::Control(self.control()?));
        }
        let nested = matches!(&self.tokens[self.cursor].kind,TokenKind::Identifier(n) if n=="match")
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|t| matches!(t.kind, TokenKind::Identifier(_)))
            && self
                .tokens
                .get(self.cursor + 2)
                .is_some_and(|t| t.kind == TokenKind::Punctuation('{'));
        if nested {
            Ok(EnumFunctionStatement::RecursiveMatch(self.match_body()?))
        } else {
            let (statement, n) = Parser::new()
                .parse_statement_prefix(
                    &self.tokens[self.cursor..],
                    self.limits.frontend.max_diagnostics,
                )
                .map_err(EnumFunctionError::Syntax)?;
            self.cursor += n;
            Ok(EnumFunctionStatement::Core(statement))
        }
    }
    fn control(&mut self) -> Result<RecursiveControlSyntax, EnumFunctionError> {
        use crate::compiler::ast::Statement;
        let start = self.tokens[self.cursor].span;
        let is_if = self.tokens[self.cursor].kind == TokenKind::Keyword("if");
        let (condition, consumed) = Parser::new()
            .parse_control_condition_prefix(
                &self.tokens[self.cursor..],
                self.limits.frontend.max_diagnostics,
            )
            .map_err(EnumFunctionError::Syntax)?;
        self.cursor += consumed;
        let first = self.body(false)?;
        let mut bodies = vec![first];
        let has_else = is_if && self.tokens[self.cursor].kind == TokenKind::Keyword("else");
        if has_else {
            self.cursor += 1;
            let body = if self.tokens[self.cursor].kind == TokenKind::Punctuation('{') {
                self.body(false)?
            } else {
                vec![self.statement()?]
            };
            bodies.push(body);
        }
        let span = Span {
            end: self.tokens[self.cursor - 1].span.end,
            ..start
        };
        let header = if is_if {
            Statement::If {
                condition,
                then_block: Vec::new(),
                else_block: has_else.then(Vec::new),
                span,
            }
        } else {
            Statement::While {
                condition,
                body: Vec::new(),
                span,
            }
        };
        Ok(RecursiveControlSyntax { header, bodies })
    }
    fn match_body(&mut self) -> Result<RecursiveMatchSyntax, EnumFunctionError> {
        let start = self.tokens[self.cursor].span;
        self.cursor += 1;
        let (scrutinee, scrutinee_span) = self.name()?;
        self.punctuation('{')?;
        let mut arms = Vec::new();
        let mut bodies = Vec::new();
        while self.tokens[self.cursor].kind != TokenKind::Punctuation('}') {
            self.arms = self
                .arms
                .checked_add(1)
                .filter(|n| *n <= self.limits.max_match_arms)
                .ok_or(EnumFunctionError::Limit("match arms"))?;
            let (mut enum_name, first) = self.name()?;
            self.operator("::")?;
            let (mut variant_name, _) = self.name()?;
            while self.tokens[self.cursor].kind == TokenKind::Operator("::") {
                self.charge(2)?;
                enum_name.push_str("::");
                enum_name.push_str(&variant_name);
                self.cursor += 1;
                variant_name = self.name()?.0;
            }
            self.punctuation('(')?;
            let (binding, binding_span) =
                if self.tokens[self.cursor].kind == TokenKind::Punctuation(')') {
                    (None, None)
                } else {
                    let (name, span) = self.name()?;
                    if name == "_" {
                        return Err(error("E2010", "wildcard bindings are unavailable", span));
                    }
                    (Some(name), Some(span))
                };
            self.punctuation(')')?;
            self.operator("=>")?;
            let body = self.body(false)?;
            let span = Span {
                end: self.tokens[self.cursor - 1].span.end,
                ..first
            };
            arms.push(MatchArmSyntax {
                enum_name,
                variant_name,
                binding,
                binding_span,
                body: Vec::new(),
                span,
            });
            bodies.push(body);
            if self.tokens[self.cursor].kind == TokenKind::Punctuation('}') {
                break;
            }
            self.punctuation(',')?;
        }
        let end = self.tokens[self.cursor].span.end;
        self.punctuation('}')?;
        Ok(RecursiveMatchSyntax {
            header: MatchStatementSyntax {
                scrutinee,
                scrutinee_span,
                arms,
                span: Span { end, ..start },
            },
            bodies,
        })
    }
}
pub(super) fn parse_program_function(
    tokens: &[Token],
    limits: EnumFunctionLimits,
) -> Result<EnumFunctionSyntax, EnumFunctionError> {
    let (header, cursor) = Parser::new()
        .parse_function_header_prefix(tokens, limits.frontend.max_diagnostics)
        .map_err(EnumFunctionError::Syntax)?;
    if header.parameters.len() > limits.max_parameters {
        return Err(EnumFunctionError::Limit("parameters"));
    }
    let mut parser = BodyParser {
        tokens,
        cursor,
        limits,
        arms: 0,
        text: 0,
        depth: 0,
    };
    let body = parser.body(true)?;
    if tokens[parser.cursor].kind != TokenKind::Eof {
        return Err(error(
            "E2000",
            "expected exactly one function",
            tokens[parser.cursor].span,
        ));
    }
    Ok(EnumFunctionSyntax {
        name: header.name,
        is_exported: header.is_exported,
        parameters: header.parameters,
        return_type: header.return_type,
        return_type_span: header.return_type_span,
        body,
        span: Span {
            end: tokens[parser.cursor - 1].span.end,
            ..header.span
        },
    })
}
