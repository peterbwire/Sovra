//! Source-owned package declarations and explicit visibility; no body checking or execution.
use super::*;
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostics, Span},
    lexer::{Lexer, TokenKind},
    project::packages::{CapturedPackageSources, PackageSourceKind},
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(1);
/// Distinct function and type namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenericPackageNamespace {
    /// Function declarations, independently of type names.
    Function,
    /// Alias, record and enum declarations.
    Type,
}
/// Source identity belongs to one immutable declaration table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPackageSourceId {
    owner: u64,
    index: usize,
}
impl GenericPackageSourceId {
    pub(super) fn ordinal(self) -> usize {
        self.index
    }
}
/// Declaration identity belongs to one immutable declaration table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GenericPackageDeclarationId {
    owner: u64,
    index: usize,
}
/// Admitted syntax and source ownership; not a semantic type/signature proof.
#[derive(Debug)]
pub struct GenericPackageDeclaration {
    id: GenericPackageDeclarationId,
    source: GenericPackageSourceId,
    syntax: GenericSourceDeclaration,
    exported: bool,
    namespace: GenericPackageNamespace,
}
impl GenericPackageDeclaration {
    /// Table-owned declaration identity.
    pub fn id(&self) -> GenericPackageDeclarationId {
        self.id
    }
    /// Exact owning original source.
    pub fn source(&self) -> GenericPackageSourceId {
        self.source
    }
    /// Original declaration syntax and file-local range.
    pub fn syntax(&self) -> &GenericSourceDeclaration {
        &self.syntax
    }
    /// Explicit export flag; aliases remain private in the admitted grammar.
    pub fn is_exported(&self) -> bool {
        self.exported
    }
    /// Value/function or type namespace.
    pub fn namespace(&self) -> GenericPackageNamespace {
        self.namespace
    }
}
/// Finite graph-wide syntax/import admission; no body/type validation is implied.
#[derive(Debug, Clone, Copy)]
pub struct GenericPackageDeclarationLimits {
    /// File-local dependency imports across the graph, hard capped at 8192.
    pub max_imports: usize,
    /// Shared declaration/source/name/token/node/work allowances across all files.
    pub syntax: GenericSourceSyntaxLimits,
}
impl GenericPackageDeclarationLimits {
    /// Reuse finite generic source quotas cumulatively, not once per file.
    pub const DEFAULT: Self = Self {
        max_imports: 8192,
        syntax: GenericSourceSyntaxLimits::DEFAULT,
    };
}
type Names =
    BTreeMap<(usize, GenericPackageNamespace), BTreeMap<String, GenericPackageDeclarationId>>;
type Imports = Vec<BTreeMap<String, BTreeMap<String, usize>>>;
/// Complete declaration/import table over captured bytes. No executable linker proof.
#[derive(Debug)]
pub struct GenericPackageDeclarations {
    owner: u64,
    inputs: super::GenericPackageSources,
    declarations: Vec<GenericPackageDeclaration>,
    names: Names,
    modules: BTreeMap<PathBuf, BTreeMap<String, usize>>,
    imports: Imports,
}
impl GenericPackageDeclarations {
    /// Immutable original source and manifest snapshots.
    pub fn inputs(&self) -> &super::GenericPackageSources {
        &self.inputs
    }
    /// Declarations in inventory and source order.
    pub fn declarations(&self) -> &[GenericPackageDeclaration] {
        &self.declarations
    }
    /// Bind an inventory ordinal to this table's source identity.
    pub fn source_id(&self, index: usize) -> Option<GenericPackageSourceId> {
        self.inputs.sources().get(index)?;
        Some(GenericPackageSourceId {
            owner: self.owner,
            index,
        })
    }
    /// Refuse a foreign declaration-table identity.
    pub fn declaration(
        &self,
        id: GenericPackageDeclarationId,
    ) -> Option<&GenericPackageDeclaration> {
        if id.owner != self.owner {
            return None;
        }
        self.declarations.get(id.index)
    }
    /// Exact file registration for an owned source; foreign source IDs refuse.
    pub fn source_registration(
        &self,
        id: GenericPackageSourceId,
    ) -> Option<&crate::compiler::project::packages::PackageSource> {
        if id.owner != self.owner {
            return None;
        }
        self.inputs.inventory().sources.get(id.index)
    }
    /// Resolve only an explicit request: local name, same-package module::name,
    /// or imported dependency::module::name. Imports never leak between files.
    /// This is visibility lookup, not binding/typing an executable use site.
    pub fn resolve(
        &self,
        from: GenericPackageSourceId,
        qualifier: &[&str],
        namespace: GenericPackageNamespace,
        name: &str,
    ) -> Option<GenericPackageDeclarationId> {
        let registration = self.source_registration(from)?;
        let target = match qualifier {
            [] => from.index,
            [module] => *self.modules.get(&registration.package_root)?.get(*module)?,
            [alias, module] => *self.imports.get(from.index)?.get(*alias)?.get(*module)?,
            _ => return None,
        };
        let id = *self.names.get(&(target, namespace))?.get(name)?;
        let declaration = self.declaration(id)?;
        if target != from.index && !declaration.exported {
            return None;
        }
        Some(id)
    }
}
/// Parse original root/module declarations and bind file-local direct imports.
/// Entry scopes admit root declarations; registered modules require exactly one
/// matching `mod name { ... }` after optional leading `use alias::module;` forms.
/// Captured ownership and private/export boundaries are retained without merging
/// namespaces. Function bodies remain syntax-owned, not checked or executable.
pub fn bind_generic_package_declarations(
    inputs: CapturedPackageSources,
    limits: GenericPackageDeclarationLimits,
) -> Result<GenericPackageDeclarations, Diagnostics> {
    bind_package_inputs(inputs.into(), limits)
}
pub(super) fn bind_package_inputs(
    inputs: super::GenericPackageSources,
    mut limits: GenericPackageDeclarationLimits,
) -> Result<GenericPackageDeclarations, Diagnostics> {
    let zero = Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    };
    let failure = |index: usize, span: Span, message: String| {
        let path = &inputs.inventory().sources[index].canonical_path;
        crate::compiler::project::packages::generic_package_error(path, span, message)
    };
    let mut owner = NEXT_PACKAGE.load(Ordering::Relaxed);
    loop {
        let next = owner
            .checked_add(1)
            .ok_or_else(|| failure(0, zero, "package declaration identities exhausted".into()))?;
        match NEXT_PACKAGE.compare_exchange(owner, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(current) => owner = current,
        }
    }
    limits.syntax.max_work = limits
        .syntax
        .max_work
        .checked_sub(
            inputs
                .sources()
                .len()
                .checked_mul(2)
                .ok_or_else(|| failure(0, zero, "package source allowance exceeded".into()))?,
        )
        .ok_or_else(|| failure(0, zero, "package source allowance exceeded".into()))?;
    let mut modules: BTreeMap<PathBuf, BTreeMap<String, usize>> = BTreeMap::new();
    for (index, source) in inputs.inventory().sources.iter().enumerate() {
        if let PackageSourceKind::Module(name) = &source.kind {
            limits.syntax.max_name_bytes = limits
                .syntax
                .max_name_bytes
                .checked_sub(name.len())
                .ok_or_else(|| {
                    failure(index, zero, "package module name allowance exceeded".into())
                })?;
            limits.syntax.max_work = limits
                .syntax
                .max_work
                .checked_sub(name.len() + 1)
                .ok_or_else(|| {
                    failure(index, zero, "package module name allowance exceeded".into())
                })?;
            modules
                .entry(source.package_root.clone())
                .or_default()
                .insert(name.clone(), index);
        }
    }
    let mut imports: Imports = (0..inputs.sources().len())
        .map(|_| BTreeMap::new())
        .collect();
    let (mut declarations, mut names) = (Vec::new(), Names::new());
    let mut remaining_imports = limits.max_imports.min(8192);
    for unit in inputs.sources() {
        let index = unit.source_index();
        let registration = &inputs.inventory().sources[index];
        let source = unit.source();
        let charge = |remaining: &mut usize, amount: usize| -> Result<(), Diagnostics> {
            *remaining = remaining.checked_sub(amount).ok_or_else(|| {
                failure(index, zero, "package declaration allowance exceeded".into())
            })?;
            Ok(())
        };
        charge(&mut limits.syntax.max_work, source.len())?;
        let mut frontend = limits.syntax.frontend;
        frontend.max_tokens = frontend
            .max_tokens
            .min(limits.syntax.max_tokens.saturating_sub(1));
        let tokens = Lexer::new()
            .tokenize_propagation(source, frontend)
            .map_err(|mut errors| {
                for error in &mut errors.items {
                    error.source_file =
                        Some(registration.canonical_path.to_string_lossy().into_owned());
                }
                errors
            })?;
        charge(&mut limits.syntax.max_tokens, tokens.len())?;
        charge(&mut limits.syntax.max_work, tokens.len())?;
        let mut at = 0;
        while tokens[at].kind == TokenKind::Keyword("use") {
            charge(&mut remaining_imports, 1)?;
            let tail = &tokens[at..];
            let (alias, module, end) = match tail {
                [_, alias, separator, module, end, ..]
                    if separator.kind == TokenKind::Operator("::")
                        && end.kind == TokenKind::Punctuation(';') =>
                {
                    match (&alias.kind, &module.kind) {
                        (TokenKind::Identifier(alias), TokenKind::Identifier(module)) => {
                            (alias, module, end.span.end)
                        }
                        _ => {
                            return Err(failure(
                                index,
                                tokens[at].span,
                                "expected use dependency::module;".into(),
                            ))
                        }
                    }
                }
                _ => {
                    return Err(failure(
                        index,
                        tokens[at].span,
                        "expected use dependency::module;".into(),
                    ))
                }
            };
            charge(
                &mut limits.syntax.max_name_bytes,
                alias.len() + module.len(),
            )?;
            charge(&mut limits.syntax.max_work, alias.len() + module.len() + 5)?;
            let target_root = inputs.inventory().graph.packages[&registration.package_root]
                .dependencies
                .get(alias)
                .ok_or_else(|| {
                    failure(
                        index,
                        tokens[at].span,
                        "import requires a declared direct dependency".into(),
                    )
                })?;
            let target = *modules
                .get(target_root)
                .and_then(|m| m.get(module))
                .ok_or_else(|| {
                    failure(
                        index,
                        tokens[at].span,
                        "import requires a registered dependency module".into(),
                    )
                })?;
            if imports[index]
                .entry(alias.clone())
                .or_default()
                .insert(module.clone(), target)
                .is_some()
            {
                return Err(failure(
                    index,
                    Span {
                        end,
                        ..tokens[at].span
                    },
                    "duplicate source import".into(),
                ));
            }
            at += 5;
        }
        let range = match &registration.kind {
            PackageSourceKind::Entry => tokens[at].span.start..source.len(),
            PackageSourceKind::Module(expected) => {
                let tail = &tokens[at..];
                let valid = matches!(tail,[head,name,open,..] if head.kind==TokenKind::Keyword("mod") && matches!(&name.kind,TokenKind::Identifier(name) if name==expected) && open.kind==TokenKind::Punctuation('{'));
                if !valid
                    || tokens.len() < at + 5
                    || tokens[tokens.len() - 2].kind != TokenKind::Punctuation('}')
                {
                    return Err(failure(
                        index,
                        tokens[at].span,
                        "registered source requires its matching module wrapper".into(),
                    ));
                }
                tokens[at + 2].span.end..tokens[tokens.len() - 2].span.start
            }
        };
        let parsed =
            super::generic_source::collect_package_fragment(source, range, &mut limits.syntax)
                .map_err(|error| {
                    failure(
                        index,
                        error.span.unwrap_or(zero),
                        diagnostic_message!("{error}"),
                    )
                })?;
        for syntax in parsed {
            let (namespace, exported) = match syntax.kind() {
                GenericSourceDeclarationKind::Function(f) => {
                    (GenericPackageNamespace::Function, f.is_exported())
                }
                GenericSourceDeclarationKind::Record(t) => {
                    (GenericPackageNamespace::Type, t.is_exported())
                }
                GenericSourceDeclarationKind::Enum(t) => {
                    (GenericPackageNamespace::Type, t.is_exported())
                }
                GenericSourceDeclarationKind::Alias(_) => (GenericPackageNamespace::Type, false),
            };
            charge(&mut limits.syntax.max_name_bytes, syntax.name().len())?;
            charge(&mut limits.syntax.max_work, syntax.name().len() + 1)?;
            let id = GenericPackageDeclarationId {
                owner,
                index: declarations.len(),
            };
            if names
                .entry((index, namespace))
                .or_default()
                .insert(syntax.name().into(), id)
                .is_some()
            {
                return Err(failure(
                    index,
                    syntax.span(),
                    "duplicate declaration in package source namespace".into(),
                ));
            }
            declarations.push(GenericPackageDeclaration {
                id,
                source: GenericPackageSourceId { owner, index },
                syntax,
                exported,
                namespace,
            });
        }
    }
    Ok(GenericPackageDeclarations {
        owner,
        inputs,
        declarations,
        names,
        modules,
        imports,
    })
}
