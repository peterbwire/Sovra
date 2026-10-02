//! Package-aware source checking followed by collision-free IR linking.

use std::collections::{BTreeMap, BTreeSet, HashMap};
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
/// Explicit `use dependency::module;` imports expose exported functions and records.
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
    let mut prepared = BTreeMap::<std::path::PathBuf, Program>::new();
    let mut interfaces = BTreeMap::new();
    let mut public_names = BTreeMap::new();
    let mut dependency_order = Vec::new();
    let mut done = BTreeSet::new();
    while done.len() < order.len() {
        for root in &order {
            if !done.contains(root)
                && graph.packages[root]
                    .dependencies
                    .values()
                    .all(|path| done.contains(path))
            {
                done.insert(root.clone());
                dependency_order.push(root);
            }
        }
    }
    for root in dependency_order {
        let package = &graph.packages[root];
        let (original, imports) = &units[root];
        let mut program = original.clone();
        let mut attached = BTreeSet::new();
        let mut imported_types = BTreeMap::new();
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
            let Some(module) = prepared[target]
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
            let exported = module
                .functions
                .iter()
                .filter(|function| function.is_exported)
                .collect::<Vec<_>>();
            let normalized = crate::compiler::semantic::normalize_imported_signatures(
                &prepared[target],
                &exported,
                &public_names[target],
            )
            .map_err(|diagnostics| locate(diagnostics, &graph.packages[target].entry))?;
            for record in &interfaces[target] {
                let record: &crate::compiler::ast::StructDeclaration = record;
                if attached.insert(record.name.clone()) {
                    program.struct_declarations.push(record.clone());
                }
            }
            for record in module
                .struct_declarations
                .iter()
                .filter(|record| record.is_exported)
            {
                let spelling = format!("{}::{}::{}", import.alias, import.module, record.name);
                let identity =
                    public_names[target][&format!("{}::{}", module.name, record.name)].clone();
                if imported_types
                    .insert(spelling.clone(), identity.clone())
                    .is_none()
                {
                    program
                        .type_declarations
                        .push(crate::compiler::ast::TypeDeclaration {
                            name: spelling,
                            target: identity,
                            span: import.span,
                        });
                }
            }
            for function in normalized {
                let name = format!("{}::{}::{}", import.alias, import.module, function.name);
                targets.insert(
                    name.clone(),
                    symbol(ids[target], &format!("{}::{}", module.name, function.name)),
                );
                visible.insert(name, function);
            }
        }
        let signatures = visible
            .iter()
            .map(|(name, function)| (name.clone(), function))
            .collect::<Vec<_>>();
        let typed = SemanticAnalyzer::new()
            .analyze_with_imports(&program, &signatures)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        let names = record_names(&program, ids[root], true);
        let exposed = crate::compiler::semantic::exported_record_fields(&program, &names)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        let exports = program
            .modules
            .iter()
            .flat_map(|module| &module.functions)
            .filter(|function| function.is_exported)
            .collect::<Vec<_>>();
        crate::compiler::semantic::normalize_imported_signatures(&program, &exports, &names)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        let mut runtime_types = record_names(&program, ids[root], false);
        runtime_types.extend(imported_types);
        let mut lowered = ir::lower_with_imports(&typed, &signatures);
        for function in &lowered.functions {
            targets.insert(function.name.clone(), symbol(ids[root], &function.name));
        }
        for function in &mut lowered.functions {
            function.name = targets[&function.name].clone();
            for instruction in &mut function.instructions {
                if let Instruction::MakeStruct { type_name, .. } = instruction {
                    if let Some(identity) = runtime_types.get(type_name) {
                        *type_name = identity.clone();
                    }
                }
                if let Instruction::Call { name, .. } = instruction {
                    if stdlib::lookup(name).is_none() {
                        *name = targets[name].clone();
                    }
                }
            }
        }
        linked.functions.extend(lowered.functions);
        prepared.insert(root.clone(), program);
        interfaces.insert(root.clone(), exposed);
        public_names.insert(root.clone(), names);
    }
    Ok(linked)
}

fn record_names(program: &Program, package: usize, exported_only: bool) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for record in &program.struct_declarations {
        if exported_only && !record.is_exported {
            continue;
        }
        let identity = if record.name.starts_with('@') {
            record.name.clone()
        } else {
            format!("@package{package}::type::root::{}", record.name)
        };
        names.insert(record.name.clone(), identity);
    }
    for module in &program.modules {
        for record in &module.struct_declarations {
            if exported_only && !record.is_exported {
                continue;
            }
            let identity = format!("@package{package}::type::{}::{}", module.name, record.name);
            names.insert(record.name.clone(), identity.clone());
            names.insert(format!("{}::{}", module.name, record.name), identity);
        }
    }
    loop {
        let mut changed = false;
        for declaration in program.type_declarations.iter().chain(
            program
                .modules
                .iter()
                .flat_map(|module| &module.type_declarations),
        ) {
            if !names.contains_key(&declaration.name) {
                if let Some(identity) = names.get(&declaration.target).cloned() {
                    names.insert(declaration.name.clone(), identity);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    names
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
