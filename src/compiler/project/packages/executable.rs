//! Package-aware source checking followed by collision-free IR linking.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{error, resolve};
use crate::compiler::{
    ast::Program,
    diagnostics::Diagnostics,
    ir::{self, Instruction, IrProgram},
    lexer::{Lexer, TokenKind},
    parser::Parser,
    semantic::SemanticAnalyzer,
    stdlib,
};

#[derive(Debug)]
struct Import {
    alias: String,
    module: String,
    span: crate::compiler::diagnostics::Span,
}

/// Resolve, parse, type-check and link all local package entry files into IR.
/// Only explicit `use dependency::module;` imports expose exported functions.
/// All loaded bodies are checked; dependency entry points are never invoked.
pub fn compile(root: impl AsRef<Path>) -> Result<IrProgram, Diagnostics> {
    let graph = resolve(root)?;
    // Number packages by alias traversal, not absolute paths: relocation preserves IR.
    let mut order = vec![graph.root.clone()];
    let mut seen = BTreeSet::from([graph.root.clone()]);
    let mut index = 0;
    while index < order.len() {
        for target in graph.packages[&order[index]].dependencies.values() {
            if seen.insert(target.clone()) {
                order.push(target.clone());
            }
        }
        index += 1;
    }
    let ids = order
        .iter()
        .enumerate()
        .map(|(index, path)| (path.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut units = BTreeMap::new();
    for root in &order {
        let entry = &graph.packages[root].entry;
        let source = std::fs::read_to_string(entry).map_err(|cause| {
            error(
                "E4110",
                format!("cannot read package source `{}`: {cause}", entry.display()),
                None,
            )
        })?;
        units.insert(
            root.clone(),
            parse(&source).map_err(|diagnostics| locate(diagnostics, entry))?,
        );
    }
    let mut linked = IrProgram {
        functions: Vec::new(),
    };
    for root in &order {
        let package = &graph.packages[root];
        let (program, imports) = &units[root];
        let mut visible = BTreeMap::new();
        let mut targets = BTreeMap::new();
        for import in imports {
            let Some(target) = package.dependencies.get(&import.alias) else {
                return Err(error(
                    "E4115",
                    format!("undeclared dependency `{}`", import.alias),
                    Some((&package.entry, import.span)),
                ));
            };
            let Some(module) = units[target]
                .0
                .modules
                .iter()
                .find(|module| module.name == import.module)
            else {
                return Err(error(
                    "E4115",
                    format!(
                        "dependency `{}` has no module `{}`",
                        import.alias, import.module
                    ),
                    Some((&package.entry, import.span)),
                ));
            };
            for function in module
                .functions
                .iter()
                .filter(|function| function.is_exported)
            {
                let name = format!("{}::{}::{}", import.alias, import.module, function.name);
                visible.insert(name.clone(), function);
                targets.insert(
                    name,
                    symbol(ids[target], &format!("{}::{}", module.name, function.name)),
                );
            }
        }
        let signatures = visible.into_iter().collect::<Vec<_>>();
        let typed = SemanticAnalyzer::new()
            .analyze_with_imports(program, &signatures)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        let mut lowered = ir::lower(&typed);
        for function in &lowered.functions {
            targets.insert(function.name.clone(), symbol(ids[root], &function.name));
        }
        for function in &mut lowered.functions {
            function.name = targets[&function.name].clone();
            for instruction in &mut function.instructions {
                if let Instruction::Call { name, .. } = instruction {
                    if stdlib::lookup(name).is_none() {
                        *name = targets[name].clone();
                    }
                }
            }
        }
        linked.functions.extend(lowered.functions);
    }
    Ok(linked)
}

fn symbol(package: usize, name: &str) -> String {
    if package == 0 && name == "main" {
        "main".into()
    } else {
        format!("@package{package}::{name}")
    }
}

fn locate(mut diagnostics: Diagnostics, path: &Path) -> Diagnostics {
    for diagnostic in &mut diagnostics.items {
        diagnostic.source_file = Some(path.to_string_lossy().into_owned());
    }
    diagnostics
}

fn parse(source: &str) -> Result<(Program, Vec<Import>), Diagnostics> {
    let tokens = Lexer::new().tokenize(source)?;
    let mut retained = Vec::new();
    let mut imports = Vec::new();
    let mut position = 0;
    let mut depth = 0usize;
    while position < tokens.len() {
        let token = &tokens[position];
        if depth == 0 && token.kind == TokenKind::Keyword("use") {
            let tail = &tokens[position..];
            match tail {
                [_, alias, separator, module, terminator, ..]
                    if separator.kind == TokenKind::Operator("::")
                        && terminator.kind == TokenKind::Punctuation(';') =>
                {
                    if let (TokenKind::Identifier(alias), TokenKind::Identifier(module)) =
                        (&alias.kind, &module.kind)
                    {
                        imports.push(Import {
                            alias: alias.clone(),
                            module: module.clone(),
                            span: crate::compiler::diagnostics::Span {
                                end: terminator.span.end,
                                ..token.span
                            },
                        });
                        position += 5;
                        continue;
                    }
                }
                _ => {}
            }
            return Err(error(
                "E4114",
                "expected `use dependency::module;`".into(),
                Some((Path::new(""), token.span)),
            ));
        }
        match token.kind {
            TokenKind::Punctuation('{') => depth += 1,
            TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
            _ => {}
        }
        retained.push(token.clone());
        position += 1;
    }
    Ok((Parser::new().parse_tokens(&retained)?, imports))
}
