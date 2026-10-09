//! File-local alias and record contracts using executable type rules.

use std::collections::{BTreeMap, HashMap};

use crate::compiler::diagnostics::Diagnostics;
use crate::compiler::lexer::{Token, TokenKind};
use crate::compiler::parser::Parser;
use crate::compiler::semantic::{collect_named_types_with_imports, collect_struct_fields, Type};

#[derive(Debug, Clone)]
pub(super) struct Aliases {
    pub types: HashMap<String, Type>,
    pub fields: HashMap<String, HashMap<String, Type>>,
    pub public_fields: HashMap<String, HashMap<String, Type>>,
    pub visible_fields: HashMap<String, HashMap<String, Type>>,
    /// Token positions for complete top-level alias/record declarations.
    pub ends: BTreeMap<usize, usize>,
    pub diagnostics: Diagnostics,
}

impl Aliases {
    #[cfg(test)]
    pub fn collect(tokens: &[Token]) -> Result<Self, String> {
        Self::collect_in_module(tokens, "<source>")
    }

    pub fn collect_in_module(tokens: &[Token], owner: &str) -> Result<Self, String> {
        Self::collect_with_imports(tokens, owner, &HashMap::new(), &HashMap::new())
    }

    fn collect_with_imports(
        tokens: &[Token],
        owner: &str,
        imported: &HashMap<String, Type>,
        imported_fields: &HashMap<String, HashMap<String, Type>>,
    ) -> Result<Self, String> {
        let mut selected = Vec::new();
        let mut ends = BTreeMap::new();
        let mut position = 0;
        let mut depth = 0usize;
        while position < tokens.len() {
            match &tokens[position].kind {
                TokenKind::Keyword("struct" | "export")
                    if depth == 0
                        && (tokens[position].kind == TokenKind::Keyword("struct")
                            || tokens.get(position + 1).is_some_and(|token| {
                                token.kind == TokenKind::Keyword("struct")
                            })) =>
                {
                    let start = position;
                    // The executable parser validates the selected declaration.
                    while position < tokens.len()
                        && !matches!(
                            tokens[position].kind,
                            TokenKind::Punctuation('}') | TokenKind::Eof
                        )
                    {
                        position += 1;
                    }
                    if position == tokens.len() || tokens[position].kind == TokenKind::Eof {
                        return Err("unclosed application record declaration".into());
                    }
                    position += 1;
                    selected.extend_from_slice(&tokens[start..position]);
                    ends.insert(start, position);
                    continue;
                }
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
            .map_err(|_| "unsupported or malformed application type declaration".to_owned())?;
        let mut diagnostics = Diagnostics::new();
        let mut types = collect_named_types_with_imports(&program, imported, &mut diagnostics);
        let raw_fields = collect_struct_fields(&program, &types, &mut diagnostics);
        let exported: std::collections::HashSet<_> = program
            .struct_declarations
            .iter()
            .filter(|record| record.is_exported)
            .map(|record| record.name.as_str())
            .collect();
        for record in &program.struct_declarations {
            if !record.is_exported {
                continue;
            }
            for field in &record.fields {
                if let Some(Type::Named(target)) = raw_fields
                    .get(&record.name)
                    .and_then(|fields| fields.get(&field.name))
                {
                    if !exported.contains(target.as_str())
                        && !crate::compiler::stdlib::is_standard_record_type(target)
                        && !imported
                            .values()
                            .any(|kind| kind == &Type::Named(target.clone()))
                    {
                        diagnostics.push(crate::compiler::diagnostics::Diagnostic {
                            source_file: None,
                            severity: crate::compiler::diagnostics::Severity::Error,
                            code: "E4116",
                            message: format!(
                                "exported record `{}` exposes private field type `{target}`",
                                record.name
                            ),
                            span: field.span,
                        });
                    }
                }
            }
        }
        let qualify = |kind: &mut Type| {
            if let Type::Named(name) = kind {
                if !name.starts_with("application:")
                    && !crate::compiler::stdlib::is_standard_record_type(name)
                {
                    *name = format!("application:{owner}::{name}");
                }
            }
        };
        let mut fields = HashMap::new();
        for (name, mut members) in raw_fields {
            for kind in members.values_mut() {
                qualify(kind);
            }
            fields.insert(format!("application:{owner}::{name}"), members);
        }
        for kind in types.values_mut() {
            qualify(kind);
        }
        // Never select one interpretation of invalid/ambiguous declarations.
        if !diagnostics.is_empty() {
            types.clear();
            fields.clear();
        }
        let public_fields: HashMap<_, _> = exported
            .iter()
            .filter_map(|name| {
                let key = format!("application:{owner}::{name}");
                fields
                    .get(&key)
                    .map(|members| (key.clone(), members.clone()))
            })
            .collect();
        let mut visible_fields = public_fields.clone();
        if diagnostics.is_empty() {
            visible_fields.extend(imported_fields.clone());
            fields.extend(imported_fields.clone());
        }
        Ok(Self {
            types,
            fields,
            public_fields,
            visible_fields,
            ends,
            diagnostics,
        })
    }
}

/// Resolve module interfaces from one snapshot, propagating only valid exports.
pub(super) fn resolve_modules(
    project: &super::ProjectCheck,
    sources: &BTreeMap<std::path::PathBuf, Result<String, String>>,
) -> BTreeMap<std::path::PathBuf, Aliases> {
    let inputs: BTreeMap<_, _> = sources
        .iter()
        .filter_map(|(file, source)| {
            let source = source.as_ref().ok()?;
            // Unsupported bodies must not contribute importable interfaces.
            super::application::inspect_functions(source, &[]).ok()?;
            let tokens = crate::compiler::lexer::Lexer::new().tokenize(source).ok()?;
            Some((file.clone(), (file.canonicalize().ok()?, tokens)))
        })
        .collect();
    let owners: BTreeMap<_, _> = inputs
        .iter()
        .map(|(file, (owner, _))| (owner.clone(), file.clone()))
        .collect();
    let mut modules: BTreeMap<std::path::PathBuf, Aliases> = BTreeMap::new();
    // A valid dependency interface can advance at least one graph edge per pass.
    // Unresolvable cross-file type cycles retain errors, never placeholders.
    for _ in 0..=inputs.len() {
        let mut next = BTreeMap::new();
        for (file, (owner, tokens)) in &inputs {
            let mut names = HashMap::new();
            let mut fields = HashMap::new();
            for import in project
                .imports
                .iter()
                .filter(|import| import.source_file == *file)
            {
                let Some(target) = owners.get(&import.target_file) else {
                    continue;
                };
                let Some(interface) = modules.get(target) else {
                    continue;
                };
                for identity in interface.public_fields.keys() {
                    let name = identity.rsplit("::").next().expect("record identity");
                    names.insert(
                        format!("{}::{name}", import.module.replace('.', "::")),
                        Type::Named(identity.clone()),
                    );
                }
                fields.extend(interface.visible_fields.clone());
            }
            if let Ok(interface) =
                Aliases::collect_with_imports(tokens, &owner.to_string_lossy(), &names, &fields)
            {
                next.insert(file.clone(), interface);
            }
        }
        let stable = next.len() == modules.len()
            && next.iter().all(|(file, current)| {
                modules.get(file).is_some_and(|old| {
                    old.types == current.types
                        && old.fields == current.fields
                        && old.public_fields == current.public_fields
                        && old.visible_fields == current.visible_fields
                })
            });
        modules = next;
        if stable {
            break;
        }
    }
    modules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;

    fn aliases(source: &str) -> Aliases {
        Aliases::collect(&Lexer::new().tokenize(source).unwrap()).unwrap()
    }

    #[test]
    fn malformed_record_declarations_fail_inspection() {
        for source in [
            "struct Point { x: Int",
            "struct Point { x Int }",
            "struct Point { x: }",
            "struct { x: Int }",
        ] {
            assert!(
                Aliases::collect(&Lexer::new().tokenize(source).unwrap()).is_err(),
                "{source}"
            );
        }
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
        assert_eq!(report.types.len(), 6);
        assert_eq!(report.types["Visible"], Type::String);
        assert_eq!(
            report.types[crate::compiler::stdlib::INPUT_LINE_TYPE],
            Type::Named(crate::compiler::stdlib::INPUT_LINE_TYPE.to_owned())
        );
        for name in [
            crate::compiler::stdlib::TEXT_READ_TYPE,
            crate::compiler::stdlib::TEXT_WRITE_TYPE,
            crate::compiler::stdlib::SPLIT_ONCE_TYPE,
            crate::compiler::stdlib::PARSED_INT_TYPE,
        ] {
            assert_eq!(report.types[name], Type::Named(name.to_owned()));
        }
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
