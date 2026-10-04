//! File-local scalar aliases using the executable parser and type resolver.

use std::collections::{BTreeMap, HashMap};

use crate::compiler::diagnostics::Diagnostics;
use crate::compiler::lexer::{Token, TokenKind};
use crate::compiler::parser::Parser;
use crate::compiler::semantic::{collect_named_types, Type};

#[derive(Debug)]
pub(super) struct Aliases {
    pub types: HashMap<String, Type>,
    /// Token positions for complete top-level alias declarations.
    pub ends: BTreeMap<usize, usize>,
    pub diagnostics: Diagnostics,
}

impl Aliases {
    pub fn collect(tokens: &[Token]) -> Result<Self, String> {
        let mut selected = Vec::new();
        let mut ends = BTreeMap::new();
        let mut position = 0;
        let mut depth = 0usize;
        while position < tokens.len() {
            match &tokens[position].kind {
                TokenKind::Keyword("type") if depth == 0 => {
                    let start = position;
                    while position < tokens.len()
                        && !matches!(
                            tokens[position].kind,
                            TokenKind::Punctuation(';') | TokenKind::Eof
                        )
                    {
                        position += 1;
                    }
                    if position == tokens.len() || tokens[position].kind == TokenKind::Eof {
                        return Err("application type alias requires a semicolon".into());
                    }
                    position += 1;
                    selected.extend_from_slice(&tokens[start..position]);
                    ends.insert(start, position);
                    continue;
                }
                TokenKind::Punctuation('{') => depth += 1,
                TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
                _ => {}
            }
            position += 1;
        }
        let eof = tokens
            .last()
            .filter(|token| token.kind == TokenKind::Eof)
            .ok_or("application alias inspection requires EOF")?;
        selected.push(eof.clone());
        let program = Parser::new()
            .parse_tokens(&selected)
            .map_err(|_| "unsupported or malformed application type alias".to_owned())?;
        let mut diagnostics = Diagnostics::new();
        let mut types = collect_named_types(&program, &mut diagnostics);
        // Never select one interpretation of invalid/ambiguous declarations.
        if !diagnostics.is_empty() {
            types.clear();
        }
        Ok(Self {
            types,
            ends,
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;

    fn aliases(source: &str) -> Aliases {
        Aliases::collect(&Lexer::new().tokenize(source).unwrap()).unwrap()
    }

    #[test]
    fn forward_scalar_aliases_reuse_executable_resolution() {
        let report = aliases("type Amount = Scalar; fn helper() {} type Scalar = Float;");
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.types["Amount"], Type::Float);
        assert_eq!(report.types["Scalar"], Type::Float);
        assert_eq!(report.ends.len(), 2);
    }

    #[test]
    fn invalid_alias_graphs_never_provide_validated_types() {
        for (source, code) in [
            ("type A = B; type B = A;", "E3017"),
            ("type A = Missing;", "E3017"),
            ("type A = Int; type A = Float;", "E3008"),
            ("type Int = Float;", "E3008"),
        ] {
            let report = aliases(source);
            assert!(report.types.is_empty(), "{source}");
            assert!(report
                .diagnostics
                .items
                .iter()
                .any(|error| error.code == code));
        }
    }

    #[test]
    fn only_top_level_aliases_are_collected() {
        let report = aliases(
            "fn main() { type Hidden = Int; } // type Comment = Bool;\ntype Visible = String;",
        );
        assert_eq!(report.types.len(), 1);
        assert_eq!(report.types["Visible"], Type::String);
    }

    #[test]
    fn malformed_aliases_fail_closed() {
        for source in [
            "type A = Int",
            "type = Int;",
            "type A = ;",
            "type A = [Int];",
        ] {
            assert!(
                Aliases::collect(&Lexer::new().tokenize(source).unwrap()).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn long_alias_chains_do_not_use_recursive_resolution() {
        let mut source = String::from("type A0 = Int;\n");
        for index in 1..5000 {
            source.push_str(&format!("type A{index} = A{};\n", index - 1));
        }
        let report = aliases(&source);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.types["A4999"], Type::Int);
    }
}
