//! Lossless token/trivia layout, guided by parsed declaration/statement boundaries.

use crate::compiler::{
    ast::*,
    lexer::{Token, TokenKind},
};
use std::collections::BTreeSet;

pub(super) const OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Comment {
    text: String,
    previous: Option<usize>,
    next: usize,
    trailing: bool,
}

#[derive(Debug)]
enum Trivia {
    Blank,
    Comment(Comment),
}

fn trivia(
    source: &str,
    start: usize,
    end: usize,
    previous: Option<usize>,
    next: usize,
) -> Result<Vec<Trivia>, String> {
    let mut result = Vec::new();
    let mut at = start;
    while at < end {
        let whitespace_start = at;
        while at < end {
            let ch = source[at..end]
                .chars()
                .next()
                .ok_or("invalid formatter gap")?;
            if !ch.is_whitespace() {
                break;
            }
            at += ch.len_utf8();
        }
        if source[whitespace_start..at]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            >= 2
        {
            result.push(Trivia::Blank);
        }
        if at == end {
            break;
        }
        if !source[at..end].starts_with("//") {
            return Err("unsupported source trivia".into());
        }
        let stop = source[at..end].find('\n').map_or(end, |offset| at + offset);
        let text = source[at..stop]
            .strip_suffix('\r')
            .unwrap_or(&source[at..stop])
            .to_owned();
        result.push(Trivia::Comment(Comment {
            text,
            previous,
            next,
            trailing: previous.is_some() && !source[start..at].contains('\n'),
        }));
        at = stop;
    }
    Ok(result)
}

pub(super) fn comments(source: &str, tokens: &[Token]) -> Result<Vec<Comment>, String> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        for item in trivia(source, start, token.span.start, index.checked_sub(1), index)? {
            if let Trivia::Comment(comment) = item {
                result.push(comment);
            }
        }
        start = token.span.end;
    }
    Ok(result)
}

fn boundaries(program: &Program, tokens: &[Token]) -> (BTreeSet<usize>, BTreeSet<usize>) {
    fn start(position: usize, tokens: &[Token], before: &mut BTreeSet<usize>) {
        let index = tokens.partition_point(|token| token.span.start < position);
        if index > 0 && tokens[index - 1].kind == TokenKind::Keyword("export") {
            before.insert(tokens[index - 1].span.start);
        } else {
            before.insert(position);
        }
    }
    fn statements(
        body: &[Statement],
        tokens: &[Token],
        before: &mut BTreeSet<usize>,
        after: &mut BTreeSet<usize>,
    ) {
        for statement in body {
            let span = match statement {
                Statement::Let { span, .. }
                | Statement::Assign { span, .. }
                | Statement::Return { span, .. }
                | Statement::If { span, .. }
                | Statement::While { span, .. } => *span,
                Statement::Expression(expr) => expr.span,
            };
            before.insert(span.start);
            let index = tokens.partition_point(|token| token.span.start < span.end);
            let end = if tokens
                .get(index)
                .is_some_and(|token| token.kind == TokenKind::Punctuation(';'))
            {
                tokens[index].span.end
            } else {
                span.end
            };
            after.insert(end);
            match statement {
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    statements(then_block, tokens, before, after);
                    if let Some(body) = else_block {
                        statements(body, tokens, before, after);
                    }
                }
                Statement::While { body, .. } => statements(body, tokens, before, after),
                _ => {}
            }
        }
    }
    fn declarations(
        aliases: &[TypeDeclaration],
        records: &[StructDeclaration],
        tokens: &[Token],
        before: &mut BTreeSet<usize>,
    ) {
        for position in aliases
            .iter()
            .map(|a| a.span.start)
            .chain(records.iter().map(|s| s.span.start))
        {
            start(position, tokens, before);
        }
    }
    let mut before = BTreeSet::new();
    let mut after = BTreeSet::new();
    declarations(
        &program.type_declarations,
        &program.struct_declarations,
        tokens,
        &mut before,
    );
    for module in &program.modules {
        start(module.span.start, tokens, &mut before);
        declarations(
            &module.type_declarations,
            &module.struct_declarations,
            tokens,
            &mut before,
        );
    }
    for function in program
        .functions
        .iter()
        .chain(program.modules.iter().flat_map(|module| &module.functions))
    {
        start(function.span.start, tokens, &mut before);
        statements(&function.body, tokens, &mut before, &mut after);
    }
    (before, after)
}

#[derive(Default)]
struct Writer {
    text: String,
    indent: usize,
    pending: bool,
    blank: bool,
}
impl Writer {
    fn push(&mut self, text: &str) -> Result<(), String> {
        if self.text.len().saturating_add(text.len()) > OUTPUT_LIMIT {
            return Err("formatted output exceeds 4 MiB".into());
        }
        self.text
            .try_reserve(text.len())
            .map_err(|_| "cannot allocate formatter output")?;
        self.text.push_str(text);
        Ok(())
    }
    fn line(&mut self) -> Result<(), String> {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.push("\n")?;
        }
        if self.blank && !self.text.is_empty() && !self.text.ends_with("\n\n") {
            self.push("\n")?;
        }
        self.pending = false;
        self.blank = false;
        Ok(())
    }
    fn raw(&mut self, text: &str) -> Result<(), String> {
        if self.text.is_empty() || self.text.ends_with('\n') {
            for _ in 0..self.indent {
                self.push("    ")?;
            }
        }
        self.push(text)
    }
    fn space(&mut self) -> Result<(), String> {
        if !self.text.is_empty() && !self.text.ends_with([' ', '\n', '\t']) {
            self.push(" ")?;
        }
        Ok(())
    }
}

fn spacing(previous: Option<&TokenKind>, current: &TokenKind) -> bool {
    use TokenKind::{Keyword as K, Operator as O, Punctuation as P};
    let Some(previous) = previous else {
        return false;
    };
    if matches!(current, P(')' | ']' | ',' | ';' | ':' | '.') | O("::"))
        || matches!(previous, P('(' | '[' | '.') | O("::"))
    {
        return false;
    }
    if *current == P('(') {
        return matches!(previous, K("if" | "while" | "return" | "else") | O(_));
    }
    if *current == P('[') {
        return matches!(previous, P(':' | ',') | O(_) | K("return"));
    }
    true
}

pub(super) fn render(source: &str, tokens: &[Token], program: &Program) -> Result<String, String> {
    let (before, after) = boundaries(program, tokens);
    let mut writer = Writer::default();
    let mut start = 0;
    let mut stack = Vec::new();
    let mut previous = None;
    for (index, token) in tokens.iter().enumerate() {
        for item in trivia(source, start, token.span.start, index.checked_sub(1), index)? {
            match item {
                Trivia::Blank => {
                    writer.blank = true;
                    writer.pending = true;
                }
                Trivia::Comment(comment) => {
                    if comment.trailing {
                        writer.space()?;
                    } else {
                        writer.line()?;
                    }
                    writer.raw(&comment.text)?;
                    writer.pending = true;
                }
            }
        }
        start = token.span.end;
        if token.kind == TokenKind::Eof {
            break;
        }
        let closing = matches!(token.kind, TokenKind::Punctuation('}'));
        if closing {
            writer.indent = writer
                .indent
                .checked_sub(1)
                .ok_or("unbalanced formatter brace")?;
            writer.pending = true;
        }
        if before.contains(&token.span.start) && previous != Some(&TokenKind::Keyword("else")) {
            writer.pending = true;
        }
        if writer.pending {
            writer.line()?;
        }
        if spacing(previous, &token.kind) {
            writer.space()?;
        }
        writer.raw(&source[token.span.start..token.span.end])?;
        match token.kind {
            TokenKind::Punctuation('{') => {
                stack.push('{');
                writer.indent += 1;
                writer.pending = true;
            }
            TokenKind::Punctuation('(') => stack.push('('),
            TokenKind::Punctuation('[') => stack.push('['),
            TokenKind::Punctuation('}' | ')' | ']') => {
                stack.pop().ok_or("unbalanced formatter delimiter")?;
            }
            TokenKind::Punctuation(';') => writer.pending = true,
            TokenKind::Punctuation(',') if stack.last() == Some(&'{') => writer.pending = true,
            _ => {}
        }
        if after.contains(&token.span.end) {
            writer.pending = true;
        }
        previous = Some(&token.kind);
    }
    writer.blank = false;
    writer.line()?;
    if !writer.text.ends_with('\n') {
        writer.push("\n")?;
    }
    Ok(writer.text)
}
