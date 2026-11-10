//! Package-aware source checking followed by collision-free IR linking.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use super::{error, resolve_with_limits};
use crate::compiler::{
    ast::Program,
    diagnostics::Diagnostics,
    ir::{self, IrProgram},
    lexer::{Lexer, TokenKind},
    parser::Parser,
    resources::{PackageCompilationLimits, PackageGraphLimits},
    semantic::SemanticAnalyzer,
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
/// The approved aggregate entry-source ceiling is 64 MiB per canonical graph.
/// Approved cumulative AST/interface and final linked IR ceilings also apply.
pub fn compile(root: impl AsRef<Path>) -> Result<IrProgram, Diagnostics> {
    compile_with_root(root).map(|(ir, _, _)| ir)
}

/// Compile with cumulative AST, interface-materialization and final linked IR
/// quotas. Canonical sources count once; interface copies count per consumer.
/// Existing source/manifest graph defaults still apply. No package code executes.
pub fn compile_with_compilation_limits(
    root: impl AsRef<Path>,
    limits: PackageCompilationLimits,
) -> Result<IrProgram, Diagnostics> {
    compile_internal(
        root,
        crate::compiler::resources::DEFAULT_PACKAGE_SOURCE_BYTES,
        PackageGraphLimits::DEFAULT,
        Some(limits),
    )
    .map(|(ir, _, _)| ir)
}

/// Compile with a caller-selected aggregate UTF-8 entry-source byte budget.
/// Canonical packages are counted once, including the root; per-file frontend
/// limits still apply. Exhaustion rejects before reading a full excess file.
/// Approved graph and compilation defaults still apply. This byte allowance
/// is not a token-total, semantic-work or process-memory quota.
pub fn compile_with_source_limit(
    root: impl AsRef<Path>,
    max_source_bytes: usize,
) -> Result<IrProgram, Diagnostics> {
    compile_with_root_and_source_limit(root, max_source_bytes).map(|(ir, _, _)| ir)
}

/// Compile with caller-selected manifest/edge graph budgets while retaining the
/// approved frontend, 64 MiB aggregate entry-source and compilation defaults.
/// Resolution failure produces no linked program and executes no package code.
pub fn compile_with_graph_limits(
    root: impl AsRef<Path>,
    limits: PackageGraphLimits,
) -> Result<IrProgram, Diagnostics> {
    compile_with_root_and_limits(
        root,
        crate::compiler::resources::DEFAULT_PACKAGE_SOURCE_BYTES,
        limits,
    )
    .map(|(ir, _, _)| ir)
}

pub(crate) fn compile_with_root(
    root: impl AsRef<Path>,
) -> Result<(IrProgram, Program, std::path::PathBuf), Diagnostics> {
    compile_with_root_and_source_limit(
        root,
        crate::compiler::resources::DEFAULT_PACKAGE_SOURCE_BYTES,
    )
}

fn compile_with_root_and_source_limit(
    root: impl AsRef<Path>,
    max_source_bytes: usize,
) -> Result<(IrProgram, Program, std::path::PathBuf), Diagnostics> {
    compile_with_root_and_limits(root, max_source_bytes, PackageGraphLimits::DEFAULT)
}

fn compile_with_root_and_limits(
    root: impl AsRef<Path>,
    max_source_bytes: usize,
    graph_limits: PackageGraphLimits,
) -> Result<(IrProgram, Program, std::path::PathBuf), Diagnostics> {
    compile_internal(
        root,
        max_source_bytes,
        graph_limits,
        Some(PackageCompilationLimits::DEFAULT),
    )
}

fn compile_internal(
    root: impl AsRef<Path>,
    max_source_bytes: usize,
    graph_limits: PackageGraphLimits,
    compilation_limits: Option<PackageCompilationLimits>,
) -> Result<(IrProgram, Program, std::path::PathBuf), Diagnostics> {
    let mut remaining_ast = compilation_limits.map_or(usize::MAX, |limits| limits.max_ast_nodes);
    let mut remaining_interface =
        compilation_limits.map_or(usize::MAX, |limits| limits.max_interface_nodes);
    let mut emission = ir::Budget::new(compilation_limits.map(|limits| limits.lowering));
    let graph = resolve_with_limits(root, graph_limits)?;
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
    let mut remaining_source_bytes = max_source_bytes;
    for root in &order {
        let entry = &graph.packages[root].entry;
        let source = crate::compiler::resources::read_source_with_limit(entry, remaining_source_bytes).map_err(|cause| {
            let cause = match cause {
                crate::compiler::resources::SourceReadError::Limit(mut errors) => {
                    if remaining_source_bytes < crate::compiler::resources::FrontendLimits::DEFAULT.max_source_bytes {
                        for diagnostic in &mut errors.items {
                            diagnostic.message = format!("aggregate package source byte limit of {max_source_bytes} exceeded");
                        }
                    }
                    return locate(errors, entry)
                }
                crate::compiler::resources::SourceReadError::Io(error) => error,
            };
            error(
                "E4110",
                format!("cannot read package source `{}`: {cause}", entry.display()),
                None,
            )
        })?;
        remaining_source_bytes -= source.len();
        let parsed = parse(&source).map_err(|diagnostics| locate(diagnostics, entry))?;
        if let Some(limits) = compilation_limits {
            let nodes = crate::compiler::resources::check_ast_budget(&parsed.0, remaining_ast)
                .map_err(|mut errors| {
                    for diagnostic in &mut errors.items {
                        diagnostic.message = format!(
                            "aggregate package AST node limit of {} exceeded",
                            limits.max_ast_nodes
                        );
                    }
                    locate(errors, entry)
                })?;
            remaining_ast -= nodes;
        }
        units.insert(root.clone(), parsed);
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
        let mut attached = BTreeMap::new();
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
            for function in &exported {
                charge_interface(
                    &mut remaining_interface,
                    compilation_limits,
                    &package.entry,
                    import.span,
                    1,
                )?;
                for _ in &function.parameters {
                    charge_interface(
                        &mut remaining_interface,
                        compilation_limits,
                        &package.entry,
                        import.span,
                        1,
                    )?;
                }
            }
            let normalized = crate::compiler::semantic::normalize_imported_signatures(
                &prepared[target],
                &exported,
                &public_names[target],
            )
            .map_err(|diagnostics| locate(diagnostics, &graph.packages[target].entry))?;
            for record in &interfaces[target] {
                let record: &crate::compiler::ast::StructDeclaration = record;
                if !attached.contains_key(&record.name) {
                    charge_record(
                        &mut remaining_interface,
                        compilation_limits,
                        &package.entry,
                        import.span,
                        record,
                    )?;
                    attached.insert(record.name.clone(), import.span);
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
                if !imported_types.contains_key(&spelling) {
                    charge_interface(
                        &mut remaining_interface,
                        compilation_limits,
                        &package.entry,
                        import.span,
                        1,
                    )?;
                }
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
        let program = &typed.program;
        let names = record_names(program, ids[root], true);
        for record in program
            .struct_declarations
            .iter()
            .chain(
                program
                    .modules
                    .iter()
                    .flat_map(|module| &module.struct_declarations),
            )
            .filter(|record| record.is_exported)
        {
            // Cached imported records retain the owner's AST ranges. Attribute
            // consumer-side copies to the import, rather than pairing those
            // foreign offsets with this consumer's source path.
            let span = attached.get(&record.name).copied().unwrap_or(record.span);
            charge_record(
                &mut remaining_interface,
                compilation_limits,
                &package.entry,
                span,
                record,
            )?;
        }
        let exposed = crate::compiler::semantic::exported_record_fields(program, &names)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        let exports = program
            .modules
            .iter()
            .flat_map(|module| &module.functions)
            .filter(|function| function.is_exported)
            .collect::<Vec<_>>();
        for function in &exports {
            charge_interface(
                &mut remaining_interface,
                compilation_limits,
                &package.entry,
                function.span,
                1,
            )?;
            for parameter in &function.parameters {
                charge_interface(
                    &mut remaining_interface,
                    compilation_limits,
                    &package.entry,
                    parameter.span,
                    1,
                )?;
            }
        }
        crate::compiler::semantic::normalize_imported_signatures(program, &exports, &names)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        let mut runtime_types = record_names(program, ids[root], false);
        runtime_types.extend(imported_types);
        for function in &program.functions {
            targets.insert(function.name.clone(), symbol(ids[root], &function.name));
        }
        for module in &program.modules {
            for function in &module.functions {
                let name = format!("{}::{}", module.name, function.name);
                targets.insert(name.clone(), symbol(ids[root], &name));
            }
        }
        emission.link_names(targets.into_iter().collect(), runtime_types);
        let lowered = ir::lower_with_budget(&typed, &signatures, &mut emission)
            .map_err(|diagnostics| locate(diagnostics, &package.entry))?;
        linked.functions.extend(lowered.functions);
        prepared.insert(root.clone(), program.clone());
        interfaces.insert(root.clone(), exposed);
        public_names.insert(root.clone(), names);
    }
    let entry = graph.packages[&graph.root].entry.clone();
    let root_program = prepared
        .remove(&graph.root)
        .expect("root package was compiled");
    Ok((linked, root_program, entry))
}

fn charge_interface(
    remaining: &mut usize,
    limits: Option<PackageCompilationLimits>,
    entry: &Path,
    span: crate::compiler::diagnostics::Span,
    nodes: usize,
) -> Result<(), Diagnostics> {
    if let Some(limits) = limits {
        if nodes > *remaining {
            return Err(error(
                "E3023",
                format!(
                    "aggregate package interface node limit of {} exceeded",
                    limits.max_interface_nodes
                ),
                Some((entry, span)),
            ));
        }
        *remaining -= nodes;
    }
    Ok(())
}

fn charge_record(
    remaining: &mut usize,
    limits: Option<PackageCompilationLimits>,
    entry: &Path,
    span: crate::compiler::diagnostics::Span,
    record: &crate::compiler::ast::StructDeclaration,
) -> Result<(), Diagnostics> {
    charge_interface(remaining, limits, entry, span, 1)?;
    for _ in &record.fields {
        charge_interface(remaining, limits, entry, span, 1)?;
    }
    Ok(())
}

pub(crate) fn root_symbol(name: &str) -> String {
    symbol(0, name)
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
            names.insert(format!("{}::{}", module.name, record.name), identity);
        }
    }
    loop {
        let mut changed = false;
        for (name, target) in program
            .type_declarations
            .iter()
            .map(|decl| (decl.name.clone(), &decl.target))
            .chain(program.modules.iter().flat_map(|module| {
                module
                    .type_declarations
                    .iter()
                    .map(move |decl| (format!("{}::{}", module.name, decl.name), &decl.target))
            }))
        {
            if !names.contains_key(&name) {
                if let Some(identity) = names.get(target).cloned() {
                    names.insert(name, identity);
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
