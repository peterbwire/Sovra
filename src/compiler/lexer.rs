//! Lexical analysis for the M1 source foundation.

use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};

/// The kinds of lexical tokens recognized by Sovra M1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A reserved language keyword.
    Keyword(&'static str),
    /// A user-defined name.
    Identifier(String),
    /// A base-10 integer literal.
    Integer(String),
    /// A decimal floating-point literal.
    Float(String),
    /// A string literal with escapes preserved.
    String(String),
    /// A single-character punctuation token.
    Punctuation(char),
    /// A supported operator.
    Operator(&'static str),
    /// End of input.
    Eof,
}

/// A token and its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The token's category and value.
    pub kind: TokenKind,
    /// The source span occupied by the token.
    pub span: Span,
}

/// The Sovra lexer.
#[derive(Debug, Default)]
pub struct Lexer;

impl Lexer {
    /// Construct a lexer.
    pub const fn new() -> Self {
        Self
    }

    /// Tokenize source, returning all lexical errors together.
    pub fn tokenize(&self, source: &str) -> Result<Vec<Token>, Diagnostics> {
        let mut scanner = Scanner::new(source);
        let mut tokens = Vec::new();

        while let Some(character) = scanner.peek() {
            if character.is_whitespace() {
                scanner.advance();
                continue;
            }

            if character == '/' && scanner.peek_next() == Some('/') {
                scanner.advance();
                scanner.advance();
                while scanner.peek().is_some_and(|ch| ch != '\n') {
                    scanner.advance();
                }
                continue;
            }

            let span = scanner.current_span();
            let kind = if character.is_ascii_alphabetic() || character == '_' {
                scanner.identifier()
            } else if character.is_ascii_digit() {
                scanner.number()
            } else if character == '"' {
                match scanner.string() {
                    Ok(value) => TokenKind::String(value),
                    Err(diagnostic) => {
                        scanner.diagnostics.push(diagnostic);
                        continue;
                    }
                }
            } else if let Some(operator) = scanner.operator() {
                TokenKind::Operator(operator)
            } else if "{}()[],:.;".contains(character) {
                scanner.advance();
                TokenKind::Punctuation(character)
            } else {
                scanner.advance();
                scanner.diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    code: "E1000",
                    message: format!("unexpected character `{character}`"),
                    span,
                });
                continue;
            };
            tokens.push(Token { kind, span });
        }

        let eof_span = scanner.current_span();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: eof_span,
        });
        if scanner.diagnostics.is_empty() {
            Ok(tokens)
        } else {
            Err(scanner.diagnostics)
        }
    }
}

struct Scanner<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
    token_start: usize,
    token_line: usize,
    token_column: usize,
    diagnostics: Diagnostics,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            line: 0,
            column: 0,
            token_start: 0,
            token_line: 0,
            token_column: 0,
            diagnostics: Diagnostics::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        self.source[self.offset..].chars().nth(1)
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        if character == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        Some(character)
    }

    fn current_span(&self) -> Span {
        Span {
            start: self.offset,
            end: self.offset,
            line: self.line,
            column: self.column,
        }
    }

    fn span(&self) -> Span {
        Span {
            start: self.token_start,
            end: self.offset,
            line: self.token_line,
            column: self.token_column,
        }
    }

    fn begin_token(&mut self) {
        self.token_start = self.offset;
        self.token_line = self.line;
        self.token_column = self.column;
    }

    fn identifier(&mut self) -> TokenKind {
        self.begin_token();
        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            self.advance();
        }
        let text = &self.source[self.token_start..self.offset];
        match text {
            "fn" => TokenKind::Keyword("fn"),
            "let" => TokenKind::Keyword("let"),
            "return" => TokenKind::Keyword("return"),
            "if" => TokenKind::Keyword("if"),
            "else" => TokenKind::Keyword("else"),
            "true" => TokenKind::Keyword("true"),
            "false" => TokenKind::Keyword("false"),
            "mod" => TokenKind::Keyword("mod"),
            "export" => TokenKind::Keyword("export"),
            "use" => TokenKind::Keyword("use"),
            "import" => TokenKind::Keyword("import"),
            _ => TokenKind::Identifier(text.to_owned()),
        }
    }

    fn number(&mut self) -> TokenKind {
        self.begin_token();
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance();
        }
        let mut is_float = false;
        if self.peek() == Some('.')
            && self
                .peek_next()
                .is_some_and(|character| character.is_ascii_digit())
        {
            is_float = true;
            self.advance();
            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance();
            }
        }
        let text = self.source[self.token_start..self.offset].to_owned();
        if is_float {
            TokenKind::Float(text)
        } else {
            TokenKind::Integer(text)
        }
    }

    fn string(&mut self) -> Result<String, Diagnostic> {
        self.begin_token();
        self.advance();
        let mut value = String::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return Ok(value);
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => value.push('\n'),
                        Some('r') => value.push('\r'),
                        Some('t') => value.push('\t'),
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some(character) => {
                            return Err(
                                self.error("E1002", format!("unsupported escape `\\{character}`"))
                            );
                        }
                        None => return Err(self.error("E1001", "unterminated string literal")),
                    }
                }
                Some('\n') | None => return Err(self.error("E1001", "unterminated string literal")),
                Some(character) => {
                    value.push(character);
                    self.advance();
                }
            }
        }
    }

    fn operator(&mut self) -> Option<&'static str> {
        self.begin_token();
        let operator = match (self.peek(), self.peek_next()) {
            (Some('='), Some('>')) => ("=>", 2),
            (Some('-'), Some('>')) => ("->", 2),
            (Some(':'), Some(':')) => ("::", 2),
            (Some('='), Some('=')) => ("==", 2),
            (Some('!'), Some('=')) => ("!=", 2),
            (Some('<'), Some('=')) => ("<=", 2),
            (Some('>'), Some('=')) => (">=", 2),
            (Some('&'), Some('&')) => ("&&", 2),
            (Some('|'), Some('|')) => ("||", 2),
            (Some('+'), _) => ("+", 1),
            (Some('-'), _) => ("-", 1),
            (Some('*'), _) => ("*", 1),
            (Some('/'), _) => ("/", 1),
            (Some('='), _) => ("=", 1),
            (Some('!'), _) => ("!", 1),
            (Some('<'), _) => ("<", 1),
            (Some('>'), _) => (">", 1),
            _ => return None,
        };
        for _ in 0..operator.1 {
            self.advance();
        }
        Some(operator.0)
    }

    fn error(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            span: self.span(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_program_foundation() {
        let tokens = Lexer::new()
            .tokenize("fn main() { let answer = 42; }")
            .expect("source should tokenize");
        assert_eq!(tokens[0].kind, TokenKind::Keyword("fn"));
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".into()));
        assert_eq!(tokens[8].kind, TokenKind::Integer("42".into()));
        assert_eq!(
            tokens.last().map(|token| &token.kind),
            Some(&TokenKind::Eof)
        );
    }

    #[test]
    fn skips_comments_and_decodes_strings() {
        let tokens = Lexer::new()
            .tokenize("// ignored\nprint(\"hello\\n\")")
            .expect("source should tokenize");
        assert_eq!(tokens[0].kind, TokenKind::Identifier("print".into()));
        assert_eq!(tokens[2].kind, TokenKind::String("hello\n".into()));
    }

    #[test]
    fn reports_invalid_character() {
        let diagnostics = Lexer::new().tokenize("@").expect_err("source should fail");
        assert_eq!(diagnostics.items[0].code, "E1000");
    }

    #[test]
    fn reports_unterminated_string() {
        let diagnostics = Lexer::new()
            .tokenize("\"missing")
            .expect_err("source should fail");
        assert_eq!(diagnostics.items[0].code, "E1001");
    }
}
