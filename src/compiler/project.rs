//! Project-level validation for `svr check`.

pub mod application;
mod application_types;
mod imports;
pub mod packages;
pub mod scope;
mod service_contract;
pub mod service_types;

use service_contract::BodyStart;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};

const MANIFEST_FILE: &str = "sovra.toml";
const SUPPORTED_RUNTIME_TARGETS: &[&str] = &["web", "cli"];
const SUPPORTED_SECTIONS: &[&str] = &["project", "runtime", "services"];
const PROJECT_KEYS: &[&str] = &["name", "version", "entry"];
const RUNTIME_KEYS: &[&str] = &["target"];

/// Validated project metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCheck {
    /// Project manifest path.
    pub manifest_path: PathBuf,
    /// Project name from `[project].name`.
    pub name: String,
    /// Entry source path resolved from `[project].entry`.
    pub entry_path: PathBuf,
    /// Runtime target from `[runtime].target`, if present.
    pub runtime_target: Option<String>,
    /// Sovra source files discovered below the project directory.
    pub source_files: Vec<PathBuf>,
    /// External service names declared in Sovra source.
    pub declared_services: Vec<String>,
    /// Service declarations with file identity, including services with no operations.
    pub service_declarations: Vec<ServiceDeclaration>,
    /// Parsed service declarations; use `service_types` for primitive type resolution.
    pub service_operations: Vec<ServiceOperation>,
    /// Validated project-relative application imports, without service-call resolution.
    pub imports: Vec<ProjectImport>,
    /// External service names requested by the application entry, if present.
    pub app_services: Vec<String>,
    /// API routes declared by the application entry.
    pub routes: Vec<AppRoute>,
    /// Page routes declared by the application entry.
    pub pages: Vec<AppPage>,
    /// Authentication target requested by the application entry, if present.
    pub auth_target: Option<String>,
    /// Auth policies declared by project source.
    pub auth_policies: Vec<AuthPolicy>,
    /// Data models requested by the application entry.
    pub data_models: Vec<String>,
    /// Scheduled tasks declared by the application entry.
    pub scheduled_tasks: Vec<AppTask>,
}

/// A source service declaration retained for module visibility resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDeclaration {
    /// Declared service name.
    pub name: String,
    /// Source file owning this declaration.
    pub source_file: PathBuf,
    /// Declaration line location.
    pub span: Span,
}

/// An explicit application import whose target source exists inside the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectImport {
    /// Source file containing the import.
    pub source_file: PathBuf,
    /// Dotted module name as declared.
    pub module: String,
    /// Canonical target source path contained within the project root.
    pub target_file: PathBuf,
    /// Location of the import declaration.
    pub span: Span,
}

/// A service parameter with an optional unresolved application annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceParameter {
    /// Parameter name.
    pub name: String,
    /// Annotation text, without type resolution or inference.
    pub annotation: Option<String>,
}

/// A structurally parsed service operation and its declaration location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOperation {
    /// Owning service name.
    pub service: String,
    /// Operation name.
    pub name: String,
    /// Parameters in declaration order.
    pub parameters: Vec<ServiceParameter>,
    /// Explicit unresolved return annotation; absence does not infer a type.
    pub return_annotation: Option<String>,
    /// Whether a body follows the signature; its contents are not validated here.
    pub has_body: bool,
    /// Source file containing the operation.
    pub source_file: PathBuf,
    /// Full declaration-line byte range and zero-based line/column.
    pub span: Span,
}

/// API route declared by an application entry file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRoute {
    /// HTTP method.
    pub method: String,
    /// Public route path.
    pub path: String,
    /// Dotted Sovra handler target.
    pub target: String,
}

/// Page route declared by an application entry file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPage {
    /// Public page path.
    pub path: String,
    /// Dotted Sovra page target.
    pub target: String,
}

/// Scheduled task declared by an application entry file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppTask {
    /// Human-readable schedule expression from the app entry.
    pub schedule: String,
    /// Dotted Sovra task target.
    pub target: String,
}

/// Authorization policy declared by an `auth` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthPolicy {
    /// Role name granted by the policy.
    pub role: String,
    /// Action names granted by the policy.
    pub actions: Vec<String>,
    /// Model names guarded by the policy.
    pub models: Vec<String>,
}

/// Validate a Sovra project directory.
pub fn check_project(path: impl AsRef<Path>) -> Result<ProjectCheck, Diagnostics> {
    let root = path.as_ref();
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest = read_manifest(&manifest_path)?;
    let mut parsed = Manifest::parse(&manifest);
    parsed.source_file = Some(manifest_path.to_string_lossy().into_owned());
    attach_line_locations(&manifest_path, &manifest, &mut parsed.diagnostics.items);
    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics);
    }

    let mut diagnostics = Diagnostics::new();
    let name = require_manifest_value(&parsed, "project", "name", &mut diagnostics);
    let entry = require_manifest_value(&parsed, "project", "entry", &mut diagnostics);
    let runtime_target = parsed.value("runtime", "target").map(str::to_owned);

    if let Some(name) = name.as_deref() {
        let first = diagnostics.items.len();
        validate_project_name(name, &mut diagnostics);
        parsed.attach("project", "name", &mut diagnostics.items[first..]);
    }
    if let Some(target) = runtime_target.as_deref() {
        let first = diagnostics.items.len();
        validate_runtime_target(target, &mut diagnostics);
        parsed.attach("runtime", "target", &mut diagnostics.items[first..]);
    }

    let first = diagnostics.items.len();
    let entry_path = entry
        .as_deref()
        .and_then(|entry| resolve_project_path(root, entry, &mut diagnostics))
        .unwrap_or_else(|| root.join(""));
    if entry.is_some() {
        validate_entry_path(&entry_path, &mut diagnostics);
    }
    parsed.attach("project", "entry", &mut diagnostics.items[first..]);

    let source_files = collect_source_files(root, &mut diagnostics);
    if source_files.is_empty() {
        push_error(
            &mut diagnostics,
            "E4008",
            "project does not contain any .svr source files",
        );
    }
    let source_index = scan_project_sources(&source_files, &entry_path, &mut diagnostics);
    let imports = validate_imports(root, &source_index.imports, &mut diagnostics);
    validate_services(&parsed, &source_index, &mut diagnostics);

    if diagnostics.is_empty() {
        Ok(ProjectCheck {
            manifest_path,
            name: name.expect("name is present when diagnostics are empty"),
            entry_path,
            runtime_target,
            source_files,
            service_declarations: source_index
                .declared_services
                .iter()
                .map(|item| ServiceDeclaration {
                    name: item.value.clone(),
                    source_file: PathBuf::from(&item.location.file),
                    span: item.location.span,
                })
                .collect(),
            service_operations: source_index.service_operations,
            imports,
            declared_services: source_index
                .declared_services
                .into_iter()
                .map(|item| item.value)
                .collect(),
            app_services: source_index
                .app_services
                .into_iter()
                .map(|item| item.value)
                .collect(),
            routes: source_index
                .routes
                .into_iter()
                .map(|item| item.value)
                .collect(),
            pages: source_index
                .pages
                .into_iter()
                .map(|item| item.value)
                .collect(),
            auth_target: source_index.auth_target.map(|item| item.value),
            auth_policies: source_index
                .auth_policies
                .into_iter()
                .map(|item| item.value)
                .collect(),
            data_models: source_index
                .data_models
                .into_iter()
                .map(|item| item.value)
                .collect(),
            scheduled_tasks: source_index
                .scheduled_tasks
                .into_iter()
                .map(|item| item.value)
                .collect(),
        })
    } else {
        Err(diagnostics)
    }
}

fn read_manifest(path: &Path) -> Result<String, Diagnostics> {
    fs::read_to_string(path).map_err(|error| {
        let mut diagnostics = Diagnostics::new();
        push_error(
            &mut diagnostics,
            "E4000",
            format!("cannot read project manifest `{}`: {error}", path.display()),
        );
        diagnostics
    })
}

fn require_manifest_value(
    manifest: &Manifest,
    section: &'static str,
    key: &'static str,
    diagnostics: &mut Diagnostics,
) -> Option<String> {
    match manifest.value(section, key) {
        Some(value) if !value.trim().is_empty() => Some(value.to_owned()),
        _ => {
            let first = diagnostics.items.len();
            push_error(
                diagnostics,
                "E4001",
                format!("project manifest requires `{section}.{key}`"),
            );
            manifest.attach(section, key, &mut diagnostics.items[first..]);
            None
        }
    }
}

fn validate_project_name(name: &str, diagnostics: &mut Diagnostics) {
    let mut chars = name.chars();
    let starts_valid = chars
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic());
    let rest_valid = chars
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'));
    if !starts_valid || !rest_valid {
        push_error(
            diagnostics,
            "E4002",
            "project name must start with a letter and contain only letters, numbers, `_`, `-`, or `.`",
        );
    }
}

fn validate_runtime_target(target: &str, diagnostics: &mut Diagnostics) {
    if !SUPPORTED_RUNTIME_TARGETS.contains(&target) {
        push_error(
            diagnostics,
            "E4003",
            format!(
                "unsupported runtime target `{target}`; expected one of: {}",
                SUPPORTED_RUNTIME_TARGETS.join(", ")
            ),
        );
    }
}

fn validate_entry_path(path: &Path, diagnostics: &mut Diagnostics) {
    if path.extension().and_then(|extension| extension.to_str()) != Some("svr") {
        push_error(
            diagnostics,
            "E4004",
            "project entry path must have a .svr extension",
        );
        return;
    }
    if !path.is_file() {
        push_error(
            diagnostics,
            "E4005",
            format!("project entry `{}` does not exist", path.display()),
        );
    }
}

fn resolve_project_path(
    root: &Path,
    value: &str,
    diagnostics: &mut Diagnostics,
) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        push_error(
            diagnostics,
            "E4007",
            "project paths must be relative and stay inside the project directory",
        );
        return None;
    }
    Some(root.join(path))
}

fn collect_source_files(root: &Path, diagnostics: &mut Diagnostics) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        collect_source_directory(&directory, &mut pending, &mut files, diagnostics);
    }
    files.sort();
    files
}

fn collect_source_directory(
    root: &Path,
    pending: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
    diagnostics: &mut Diagnostics,
) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            push_error(
                diagnostics,
                "E4006",
                format!(
                    "cannot read project directory `{}`: {error}",
                    root.display()
                ),
            );
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                push_error(
                    diagnostics,
                    "E4006",
                    format!("cannot read directory entry: {error}"),
                );
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                push_error(
                    diagnostics,
                    "E4006",
                    format!("cannot inspect `{}`: {error}", path.display()),
                );
                continue;
            }
        };
        if file_type.is_dir() {
            pending.push(path);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("svr") {
            files.push(path);
        }
    }
}

#[derive(Debug, Default)]
struct ProjectSourceIndex {
    imports: Vec<Located<Vec<String>>>,
    service_operations: Vec<ServiceOperation>,
    declared_services: Vec<Located<String>>,
    app_services: Vec<Located<String>>,
    callable_symbols: BTreeSet<String>,
    page_symbols: BTreeSet<String>,
    auth_symbols: BTreeSet<String>,
    model_symbols: BTreeSet<String>,
    auth_policies: Vec<Located<AuthPolicy>>,
    routes: Vec<Located<AppRoute>>,
    pages: Vec<Located<AppPage>>,
    auth_target: Option<Located<String>>,
    data_models: Vec<Located<String>>,
    scheduled_tasks: Vec<Located<AppTask>>,
}

#[derive(Debug, Clone)]
struct SourceLocation {
    file: String,
    span: Span,
}

impl SourceLocation {
    fn locate<T>(&self, value: T) -> Located<T> {
        Located {
            value,
            location: self.clone(),
        }
    }

    fn attach(&self, diagnostics: &mut [Diagnostic]) {
        for diagnostic in diagnostics {
            diagnostic.source_file = Some(self.file.clone());
            diagnostic.span = self.span;
        }
    }
}

#[derive(Debug)]
struct Located<T> {
    value: T,
    location: SourceLocation,
}

impl<T> std::ops::Deref for Located<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

fn scan_project_sources(
    source_files: &[PathBuf],
    entry_path: &Path,
    diagnostics: &mut Diagnostics,
) -> ProjectSourceIndex {
    let mut index = ProjectSourceIndex::default();
    for source_file in source_files {
        let source = match fs::read_to_string(source_file) {
            Ok(source) => source,
            Err(error) => {
                push_error(
                    diagnostics,
                    "E4009",
                    format!(
                        "cannot read source file `{}`: {error}",
                        source_file.display()
                    ),
                );
                continue;
            }
        };
        let module_name = source_file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let first_diagnostic = diagnostics.items.len();
        scan_source_file(
            &source,
            source_file,
            module_name,
            source_file == entry_path,
            &mut index,
            diagnostics,
        );
        attach_line_locations(
            source_file,
            &source,
            &mut diagnostics.items[first_diagnostic..],
        );
    }
    index
        .declared_services
        .sort_by(|a, b| a.value.cmp(&b.value));
    index.app_services.sort_by(|a, b| a.value.cmp(&b.value));
    index.routes.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.method.cmp(&right.method))
    });
    index
        .pages
        .sort_by(|left, right| left.path.cmp(&right.path));
    index
        .scheduled_tasks
        .sort_by(|left, right| left.target.cmp(&right.target));
    index.auth_policies.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then(left.models.cmp(&right.models))
    });
    index.data_models.sort_by(|a, b| a.value.cmp(&b.value));
    index
}

fn scan_source_file(
    source: &str,
    source_file: &Path,
    module_name: &str,
    is_entry: bool,
    index: &mut ProjectSourceIndex,
    diagnostics: &mut Diagnostics,
) {
    let mut seen_services = BTreeSet::new();
    let mut service_contract: Option<(String, BTreeSet<String>, usize)> = None;
    let mut pending_service: Option<(String, usize)> = None;
    let mut pending_operation: Option<usize> = None;
    let mut brace_depth = 0usize;
    let spans = source_line_spans(source);
    let lines = source.lines().collect::<Vec<_>>();
    let mut consumed_until = 0;
    for (line_index, line) in lines.iter().enumerate() {
        if line_index < consumed_until {
            continue;
        }
        let line = strip_line_comment(line, "//").trim();
        let (signature, last_line) =
            if brace_depth == 0 || (brace_depth == 1 && service_contract.is_some()) {
                collect_signature_lines(&lines, line_index)
            } else {
                (line.to_owned(), line_index)
            };
        consumed_until = last_line + 1;
        let trimmed = signature.as_str();
        let location = SourceLocation {
            file: source_file.to_string_lossy().into_owned(),
            span: spans[line_index],
        };
        if !trimmed.is_empty() {
            // An unterminated signature can acquire a body on the next substantive
            // line. Never carry it past another declaration or a closing brace.
            if let Some(operation) = pending_operation.take() {
                if brace_depth == 1 && trimmed.starts_with('{') {
                    index.service_operations[operation].has_body = true;
                }
            }
            if let Some((name, declaration_line)) = pending_service.take() {
                if brace_depth == 0 && trimmed == "{" {
                    service_contract = Some((name, BTreeSet::new(), declaration_line));
                } else {
                    push_manifest_error(diagnostics, declaration_line, "E4026",
                        format!("service `{name}` requires an opening brace before the next declaration"));
                }
            }
        }
        let inside_service = service_contract.is_some();
        if brace_depth == 0 && !inside_service {
            match imports::parse(trimmed) {
                Ok(Some(segments)) => index.imports.push(location.clone().locate(segments)),
                Ok(None) => {}
                Err(message) => push_manifest_error(diagnostics, line_index, "E4090", message),
            }
        }
        if brace_depth == 1 {
            if let Some((service, operations, _)) = &mut service_contract {
                match service_contract::parse_operation(trimmed) {
                    Ok(Some(operation)) => {
                        for parameter in &operation.declarations {
                            if parameter.annotation.is_none() {
                                push_manifest_error(
                                    diagnostics,
                                    line_index,
                                    "E4097",
                                    format!("parameter `{}` in service operation `{service}.{}` requires an explicit type annotation", parameter.name, operation.name),
                                );
                            }
                        }
                        if operation.body.is_none() && !operation.suffix.ends_with(';') {
                            pending_operation = Some(index.service_operations.len());
                        }
                        index.service_operations.push(ServiceOperation {
                            service: service.clone(),
                            name: operation.name.to_owned(),
                            parameters: operation
                                .declarations
                                .iter()
                                .map(|parameter| ServiceParameter {
                                    name: parameter.name.to_owned(),
                                    annotation: parameter.annotation.map(str::to_owned),
                                })
                                .collect(),
                            return_annotation: operation.return_annotation.map(str::to_owned),
                            has_body: operation.body.is_some(),
                            source_file: source_file.to_path_buf(),
                            span: Span {
                                end: spans[last_line].end,
                                ..spans[line_index]
                            },
                        });
                        if !operations.insert(operation.name.to_owned()) {
                            push_manifest_error(
                                diagnostics,
                                line_index,
                                "E4025",
                                format!(
                                    "duplicate operation `{}` in service `{service}`",
                                    operation.name
                                ),
                            );
                        }
                    }
                    Ok(None) => {}
                    Err(message) => push_manifest_error(diagnostics, line_index, "E4027", message),
                }
            }
        }
        if inside_service {
            // Service contents belong to their contract, not to application
            // wiring. Keep tracking braces, but do not index nested declarations
            // or interpret body lines as entry-file routes, lists or policies.
            update_brace_depth(trimmed, &mut brace_depth);
            if brace_depth == 0 {
                service_contract = None;
            }
            continue;
        }
        if brace_depth == 0 {
            check_application_parameters(trimmed, line_index, diagnostics);
        }
        let header = match service_contract::parse_header(trimmed) {
            Ok(header) => header,
            Err(message) => {
                push_manifest_error(diagnostics, line_index, "E4026", message);
                None
            }
        };
        if let Some(header) = header {
            let name = header.name;
            if brace_depth == 0 {
                match header.body {
                    BodyStart::Open => {
                        service_contract = Some((name.clone(), BTreeSet::new(), line_index))
                    }
                    BodyStart::Pending => pending_service = Some((name.clone(), line_index)),
                    BodyStart::Empty => {}
                }
            }
            if !seen_services.insert(name.clone())
                || index
                    .declared_services
                    .iter()
                    .any(|item| item.value == name)
            {
                push_manifest_error(
                    diagnostics,
                    line_index,
                    "E4020",
                    format!("duplicate service declaration `{name}`"),
                );
            }
            index.declared_services.push(location.locate(name));
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "fn") {
            index.callable_symbols.insert(name.clone());
            index
                .callable_symbols
                .insert(format!("{module_name}.{name}"));
        }
        let mut is_task_declaration = false;
        if let Some(name) = parse_prefixed_identifier(trimmed, "task") {
            if trimmed
                .strip_prefix("task")
                .and_then(|rest| rest.trim_start().strip_prefix(&name))
                .is_some_and(|rest| rest.trim_start().starts_with('('))
            {
                is_task_declaration = true;
                index.callable_symbols.insert(name.clone());
                index
                    .callable_symbols
                    .insert(format!("{module_name}.{name}"));
            }
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "model") {
            index.model_symbols.insert(name.clone());
            index.model_symbols.insert(format!("{module_name}.{name}"));
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "auth") {
            index.auth_symbols.insert(name.clone());
            index.auth_symbols.insert(format!("{module_name}.{name}"));
        }
        if starts_keyword(trimmed, "allow") {
            match parse_auth_policy(trimmed) {
                Some(policy) => index.auth_policies.push(location.locate(policy)),
                None => push_manifest_error(
                    diagnostics,
                    line_index,
                    "E4080",
                    "malformed auth policy; expected `allow role to action on Model`",
                ),
            }
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "page") {
            index.page_symbols.insert(name.clone());
            index.page_symbols.insert(format!("{module_name}.{name}"));
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "view") {
            index.page_symbols.insert(name.clone());
            index.page_symbols.insert(format!("{module_name}.{name}"));
        }
        update_brace_depth(trimmed, &mut brace_depth);
        if brace_depth == 0 {
            service_contract = None;
        }
        if is_entry {
            for (key, code, values) in [
                ("services", "E4024", &mut index.app_services),
                ("data", "E4062", &mut index.data_models),
            ] {
                if trimmed.strip_prefix(key).is_some_and(|rest| {
                    !rest
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                }) {
                    match parse_named_list(trimmed, key) {
                        Some(items) => values.extend(items.into_iter().map(|value| location.locate(value))),
                        None => push_manifest_error(
                            diagnostics,
                            line_index,
                            code,
                            format!("malformed {key} list; expected `{key}: [name, ...]` with identifier items"),
                        ),
                    }
                }
            }
            if starts_named_key(trimmed, "auth") || starts_keyword(trimmed, "auth") {
                match parse_named_target(trimmed, "auth") {
                    Some(target) => {
                        if index.auth_target.replace(location.locate(target)).is_some() {
                            push_manifest_error(
                                diagnostics,
                                line_index,
                                "E4051",
                                "application entry declares multiple auth bindings",
                            );
                        }
                    }
                    None => push_manifest_error(
                        diagnostics,
                        line_index,
                        "E4050",
                        "malformed auth binding; expected `auth: module.symbol`",
                    ),
                }
            }
            if starts_keyword(trimmed, "route") {
                match parse_app_route(trimmed) {
                    Some(route) => index.routes.push(location.locate(route)),
                    None => push_manifest_error(
                        diagnostics,
                        line_index,
                        "E4034",
                        "malformed route declaration; expected `route METHOD \"/path\" -> target`",
                    ),
                }
            }
            if starts_keyword(trimmed, "page")
                && trimmed
                    .strip_prefix("page")
                    .is_some_and(|rest| rest.trim_start().starts_with('"'))
            {
                match parse_app_page(trimmed) {
                    Some(page) => index.pages.push(location.locate(page)),
                    None => push_manifest_error(
                        diagnostics,
                        line_index,
                        "E4043",
                        "malformed page route declaration; expected `page \"/path\" -> target`",
                    ),
                }
            }
            if starts_keyword(trimmed, "task") && !is_task_declaration && trimmed.contains("->") {
                match parse_app_task(trimmed) {
                    Some(task) => index.scheduled_tasks.push(location.locate(task)),
                    None => push_manifest_error(
                        diagnostics,
                        line_index,
                        "E4070",
                        "malformed scheduled task; expected `task <schedule> -> target`",
                    ),
                }
            }
        }
    }
    if let Some((name, declaration_line)) = pending_service {
        push_manifest_error(
            diagnostics,
            declaration_line,
            "E4026",
            format!("service `{name}` is missing its opening brace"),
        );
    }
    if let Some((name, _, declaration_line)) = service_contract {
        push_manifest_error(
            diagnostics,
            declaration_line,
            "E4026",
            format!("service `{name}` is missing its closing brace"),
        );
    }
}

// Collect only parameter-list continuations. This preserves the scanner's
// declaration recovery boundary and leaves type interpretation to later stages.
fn collect_signature_lines(lines: &[&str], start: usize) -> (String, usize) {
    let first = strip_line_comment(lines[start], "//").trim();
    let parameterized = ["fn", "task", "page", "view"].iter().any(|kind| {
        parse_prefixed_identifier(first, kind).is_some_and(|name| {
            first[kind.len()..].trim_start()[name.len()..]
                .trim_start()
                .starts_with('(')
        })
    });
    if !parameterized {
        return (first.to_owned(), start);
    }
    let mut signature = String::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut last = start;
    for (index, line) in lines.iter().enumerate().skip(start) {
        let line = strip_line_comment(line, "//").trim();
        if index > start
            && (line.starts_with(['{', '}'])
                || [
                    "fn", "task", "page", "view", "service", "model", "type", "enum", "use", "app",
                    "auth",
                ]
                .iter()
                .any(|keyword| starts_keyword(line, keyword)))
        {
            break;
        }
        if index > start {
            signature.push('\n');
        }
        signature.push_str(line);
        last = index;
        for character in line.chars() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    quoted = false;
                }
                continue;
            }
            match character {
                '"' => quoted = true,
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return (signature, last);
                    }
                }
                _ => {}
            }
        }
    }
    (signature, last)
}

fn check_application_parameters(line: &str, line_index: usize, diagnostics: &mut Diagnostics) {
    for kind in ["fn", "task", "page", "view"] {
        let Some(name) = parse_prefixed_identifier(line, kind) else {
            continue;
        };
        let rest = line[kind.len()..].trim_start();
        if !rest[name.len()..].trim_start().starts_with('(') {
            // Scheduled task bindings and page routes are not parameter lists.
            continue;
        }
        let signature = format!("fn {rest}");
        match service_contract::parse_operation(&signature) {
            Ok(Some(operation)) => {
                for parameter in operation.declarations {
                    if parameter.annotation.is_none() {
                        push_manifest_error(diagnostics, line_index, "E4097", format!(
                            "parameter `{}` in {kind} `{name}` requires an explicit type annotation", parameter.name
                        ));
                    }
                }
            }
            Err(message) => push_manifest_error(
                diagnostics,
                line_index,
                "E4098",
                format!("invalid {kind} signature: {message}"),
            ),
            Ok(None) => {}
        }
        break;
    }
}

fn update_brace_depth(line: &str, depth: &mut usize) {
    let mut quoted = false;
    let mut escaped = false;
    for character in line.chars() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else {
            match character {
                '"' => quoted = true,
                '{' => *depth += 1,
                '}' => *depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
}

fn starts_keyword(line: &str, keyword: &str) -> bool {
    line.strip_prefix(keyword).is_some_and(|rest| {
        rest.is_empty()
            || rest
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_whitespace())
    })
}

fn starts_named_key(line: &str, key: &str) -> bool {
    line.strip_prefix(key)
        .is_some_and(|rest| rest.trim_start().starts_with(':'))
}

fn parse_prefixed_identifier(line: &str, keyword: &str) -> Option<String> {
    let rest = line.strip_prefix(keyword)?;
    if !rest
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_whitespace())
    {
        return None;
    }
    let name = rest
        .trim_start()
        .split(|character: char| character.is_ascii_whitespace() || matches!(character, '{' | '('))
        .next()?;
    if is_identifier(name) {
        Some(name.to_owned())
    } else {
        None
    }
}

fn parse_named_list(line: &str, key: &str) -> Option<Vec<String>> {
    let rest = line.strip_prefix(key)?.trim_start();
    let list = rest.strip_prefix(':')?.trim_start().strip_prefix('[')?;
    let (items, rest) = list.split_once(']')?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(';').unwrap_or(rest).trim_start();
    if !rest.is_empty() {
        return None;
    }
    if items.trim().is_empty() {
        return Some(Vec::new());
    }
    // Preserve a single trailing comma, but never discard empty interior items.
    let items = items.trim_end().strip_suffix(',').unwrap_or(items);
    items
        .split(',')
        .map(|item| {
            let item = item.trim();
            is_identifier(item).then(|| item.to_owned())
        })
        .collect()
}

fn parse_named_target(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let target = rest.strip_prefix(':')?.trim();
    if is_dotted_identifier(target) {
        Some(target.to_owned())
    } else {
        None
    }
}

fn parse_app_route(line: &str) -> Option<AppRoute> {
    let rest = line.strip_prefix("route")?.trim_start();
    let (method, rest) = split_identifier(rest)?;
    let (path, rest) = parse_quoted_prefix(rest.trim_start())?;
    let target = parse_arrow_target(rest)?;
    Some(AppRoute {
        method: method.to_owned(),
        path,
        target,
    })
}

fn parse_app_page(line: &str) -> Option<AppPage> {
    let rest = line.strip_prefix("page")?.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let (path, rest) = parse_quoted_prefix(rest)?;
    let target = parse_arrow_target(rest)?;
    Some(AppPage { path, target })
}

fn parse_app_task(line: &str) -> Option<AppTask> {
    let rest = line.strip_prefix("task")?.trim_start();
    let (schedule, target) = rest.split_once("->")?;
    let schedule = schedule.trim();
    if schedule.is_empty() {
        return None;
    }
    let target = target.trim();
    if !is_dotted_identifier(target) {
        return None;
    }
    Some(AppTask {
        schedule: schedule.to_owned(),
        target: target.to_owned(),
    })
}

fn parse_auth_policy(line: &str) -> Option<AuthPolicy> {
    let rest = line.strip_prefix("allow")?.trim_start();
    let (role, rest) = split_identifier(rest)?;
    let rest = rest.trim_start().strip_prefix("to")?.trim_start();
    let (actions, rest) = split_policy_action_and_models(rest)?;
    let models = rest
        .split_once(" where ")
        .map(|(models, _)| models)
        .unwrap_or(rest)
        .trim();
    let actions = parse_policy_list(actions, is_dotted_identifier)?;
    let models = parse_policy_list(models, is_dotted_identifier)?;
    Some(AuthPolicy {
        role: role.to_owned(),
        actions,
        models,
    })
}

fn split_policy_action_and_models(value: &str) -> Option<(&str, &str)> {
    split_policy_list_before_keyword(value, "on").or_else(|| {
        let (action, models) = split_identifier(value)?;
        Some((action, models.trim_start()))
    })
}

fn split_policy_list_before_keyword<'a>(
    value: &'a str,
    keyword: &str,
) -> Option<(&'a str, &'a str)> {
    let mut bracket_depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.checked_sub(1)?,
            _ => {}
        }
        if bracket_depth == 0 {
            let candidate = &value[index..];
            if starts_keyword(candidate.trim_start(), keyword) {
                let before = value[..index].trim();
                let after = candidate.trim_start()[keyword.len()..].trim_start();
                return Some((before, after));
            }
        }
    }
    None
}

fn parse_policy_list(value: &str, is_valid_item: fn(&str) -> bool) -> Option<Vec<String>> {
    let value = value.trim();
    let items = if let Some(rest) = value.strip_prefix('[') {
        rest.strip_suffix(']')?
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>()
    } else {
        vec![value]
    };
    if items.is_empty() || items.iter().any(|item| !is_valid_item(item)) {
        return None;
    }
    Some(items.into_iter().map(str::to_owned).collect())
}

fn split_identifier(value: &str) -> Option<(&str, &str)> {
    let end = value
        .char_indices()
        .find_map(|(index, character)| character.is_ascii_whitespace().then_some(index))
        .unwrap_or(value.len());
    let identifier = &value[..end];
    if identifier.is_empty() || !is_identifier(identifier) {
        return None;
    }
    Some((identifier, &value[end..]))
}

fn parse_quoted_prefix(value: &str) -> Option<(String, &str)> {
    let mut chars = value.chars();
    if chars.next() != Some('"') {
        return None;
    }
    let mut parsed = String::new();
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if escaped {
            match character {
                '"' => parsed.push('"'),
                '\\' => parsed.push('\\'),
                'n' => parsed.push('\n'),
                'r' => parsed.push('\r'),
                't' => parsed.push('\t'),
                _ => return None,
            }
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            return Some((parsed, chars.as_str()));
        }
        parsed.push(character);
    }
    None
}

fn parse_arrow_target(value: &str) -> Option<String> {
    let target = value.trim_start().strip_prefix("->")?.trim();
    if is_dotted_identifier(target) {
        Some(target.to_owned())
    } else {
        None
    }
}

fn validate_imports(
    root: &Path,
    declarations: &[Located<Vec<String>>],
    diagnostics: &mut Diagnostics,
) -> Vec<ProjectImport> {
    let canonical_root = root.canonicalize();
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    for declaration in declarations {
        let module = declaration.value.join(".");
        let mut target = root.to_path_buf();
        for segment in &declaration.value {
            target.push(segment);
        }
        target.set_extension("svr");
        let first = diagnostics.items.len();
        match (&canonical_root, target.canonicalize()) {
            (Ok(root), Ok(target)) if !target.starts_with(root) => {
                push_error(
                    diagnostics,
                    "E4092",
                    format!("import `{module}` resolves outside the project root"),
                );
            }
            (Ok(_), Ok(target)) if target.is_file() => {
                let source = PathBuf::from(&declaration.location.file);
                // Cycles require no recursion: discovery scans each file once.
                if seen.insert((source.clone(), target.clone())) {
                    resolved.push(ProjectImport {
                        source_file: source,
                        module,
                        target_file: target,
                        span: declaration.location.span,
                    });
                }
            }
            _ => push_error(
                diagnostics,
                "E4091",
                format!("import `{module}` does not resolve to an accessible project source file"),
            ),
        }
        declaration.location.attach(&mut diagnostics.items[first..]);
    }
    resolved
}

fn validate_services(
    manifest: &Manifest,
    source_index: &ProjectSourceIndex,
    diagnostics: &mut Diagnostics,
) {
    let manifest_services: Vec<&str> = manifest
        .entries_in_section("services")
        .map(|entry| entry.key.as_str())
        .collect();
    for service in &manifest_services {
        let first = diagnostics.items.len();
        if !is_identifier(service) {
            push_error(
                diagnostics,
                "E4017",
                format!("service binding `{service}` must be a valid identifier"),
            );
        }
        if !source_index
            .declared_services
            .iter()
            .any(|declared| declared.value == *service)
        {
            push_error(
                diagnostics,
                "E4021",
                format!("service binding `{service}` has no matching source declaration"),
            );
        }
        manifest.attach("services", service, &mut diagnostics.items[first..]);
    }
    for item in &source_index.declared_services {
        let service = &item.value;
        let first = diagnostics.items.len();
        if !manifest_services
            .iter()
            .any(|bound_service| *bound_service == service)
        {
            push_error(
                diagnostics,
                "E4022",
                format!("service declaration `{service}` has no manifest binding"),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
    for item in &source_index.app_services {
        let service = &item.value;
        let first = diagnostics.items.len();
        if !manifest_services
            .iter()
            .any(|bound_service| *bound_service == service)
            || !source_index
                .declared_services
                .iter()
                .any(|declared| declared.value == *service)
        {
            push_error(
                diagnostics,
                "E4023",
                format!("app service `{service}` must be declared and bound in the manifest"),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
    validate_routes(source_index, diagnostics);
    validate_pages(source_index, diagnostics);
    validate_auth(source_index, diagnostics);
    validate_data_models(source_index, diagnostics);
    validate_scheduled_tasks(source_index, diagnostics);
    validate_auth_policies(source_index, diagnostics);
}

fn validate_routes(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for item in &source_index.routes {
        let route = &item.value;
        let first = diagnostics.items.len();
        if !is_http_method(&route.method) {
            push_error(
                diagnostics,
                "E4030",
                format!("route method `{}` is not supported", route.method),
            );
        }
        if !route.path.starts_with('/') {
            push_error(
                diagnostics,
                "E4031",
                format!("route path `{}` must start with `/`", route.path),
            );
        }
        if let Err(message) = validate_public_path(&route.path) {
            push_error(diagnostics, "E4034", format!("route path {message}"));
        }
        if !seen.insert((route.method.clone(), route.path.clone())) {
            push_error(
                diagnostics,
                "E4032",
                format!("duplicate route `{} {}`", route.method, route.path),
            );
        }
        if !source_index.callable_symbols.contains(&route.target) {
            push_error(
                diagnostics,
                "E4033",
                format!("route target `{}` was not found", route.target),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
}

fn validate_pages(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for item in &source_index.pages {
        let page = &item.value;
        let first = diagnostics.items.len();
        if !page.path.starts_with('/') {
            push_error(
                diagnostics,
                "E4040",
                format!("page path `{}` must start with `/`", page.path),
            );
        }
        if let Err(message) = validate_public_path(&page.path) {
            push_error(diagnostics, "E4043", format!("page path {message}"));
        }
        if !seen.insert(page.path.clone()) {
            push_error(
                diagnostics,
                "E4041",
                format!("duplicate page path `{}`", page.path),
            );
        }
        if !source_index.page_symbols.contains(&page.target) {
            push_error(
                diagnostics,
                "E4042",
                format!("page target `{}` was not found", page.target),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
}

fn validate_auth(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    if let Some(item) = &source_index.auth_target {
        let target = &item.value;
        let first = diagnostics.items.len();
        if !source_index.auth_symbols.contains(target) {
            push_error(
                diagnostics,
                "E4052",
                format!("auth target `{target}` was not found"),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
}

fn validate_data_models(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for item in &source_index.data_models {
        let model = &item.value;
        let first = diagnostics.items.len();
        if !seen.insert(model.clone()) {
            push_error(
                diagnostics,
                "E4061",
                format!("duplicate app data model `{model}`"),
            );
        }
        if !source_index.model_symbols.contains(model) {
            push_error(
                diagnostics,
                "E4060",
                format!("app data model `{model}` was not found"),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
}

fn validate_scheduled_tasks(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for item in &source_index.scheduled_tasks {
        let task = &item.value;
        let first = diagnostics.items.len();
        if !seen.insert((task.schedule.clone(), task.target.clone())) {
            push_error(
                diagnostics,
                "E4071",
                format!(
                    "duplicate scheduled task `{}` -> `{}`",
                    task.schedule, task.target
                ),
            );
        }
        if !source_index.callable_symbols.contains(&task.target) {
            push_error(
                diagnostics,
                "E4072",
                format!("scheduled task target `{}` was not found", task.target),
            );
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
}

fn validate_auth_policies(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for item in &source_index.auth_policies {
        let policy = &item.value;
        let first = diagnostics.items.len();
        if !seen.insert((
            policy.role.clone(),
            policy.actions.clone(),
            policy.models.clone(),
        )) {
            push_error(
                diagnostics,
                "E4082",
                format!("duplicate auth policy for role `{}`", policy.role),
            );
        }
        for model in &policy.models {
            if !source_index.model_symbols.contains(model) {
                push_error(
                    diagnostics,
                    "E4081",
                    format!("auth policy model `{model}` was not found"),
                );
            }
        }
        item.location.attach(&mut diagnostics.items[first..]);
    }
}

fn validate_public_path(path: &str) -> Result<(), String> {
    if path.chars().any(char::is_whitespace) {
        return Err(format!("`{path}` cannot contain whitespace"));
    }
    if path == "/" {
        return Ok(());
    }
    if path != "/" && path.ends_with('/') {
        return Err(format!("`{path}` cannot end with `/`"));
    }
    for segment in path.split('/').skip(1) {
        if segment.is_empty() {
            return Err(format!("`{path}` cannot contain empty path segments"));
        }
        if let Some(parameter) = segment.strip_prefix(':') {
            if !is_identifier(parameter) {
                return Err(format!(
                    "`{path}` contains invalid route parameter `:{parameter}`"
                ));
            }
        }
    }
    Ok(())
}

fn is_http_method(value: &str) -> bool {
    matches!(
        value,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let starts_valid = chars
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_');
    starts_valid && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_dotted_identifier(value: &str) -> bool {
    let mut parts = value.split('.');
    parts.next().is_some_and(is_identifier) && parts.all(is_identifier)
}

#[derive(Debug, Default)]
struct Manifest {
    sections: Vec<(String, Span)>,
    entries: Vec<ManifestEntry>,
    diagnostics: Diagnostics,
    source_file: Option<String>,
}

impl Manifest {
    fn parse(source: &str) -> Self {
        Self::parse_mode(source, false)
    }

    fn parse_mode(source: &str, dependencies: bool) -> Self {
        let mut manifest = Self::default();
        let mut current_section: Option<String> = None;
        let mut seen_sections = BTreeSet::new();
        let mut seen_keys = BTreeSet::new();
        let spans = source_line_spans(source);

        for (line_index, line) in source.lines().enumerate() {
            let line_without_comment = strip_line_comment(line, "#");
            let trimmed = line_without_comment.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let section = trimmed[1..trimmed.len() - 1].trim();
                if section.is_empty() {
                    push_manifest_error(
                        &mut manifest.diagnostics,
                        line_index,
                        "E4010",
                        "manifest section name cannot be empty",
                    );
                    current_section = None;
                    continue;
                }
                if !SUPPORTED_SECTIONS.contains(&section)
                    && !(dependencies
                        && section
                            .strip_prefix("dependencies.")
                            .is_some_and(|alias| is_identifier(alias) && alias != "std"))
                {
                    push_manifest_error(
                        &mut manifest.diagnostics,
                        line_index,
                        "E4010",
                        format!("unknown manifest section `{section}`"),
                    );
                }
                if !seen_sections.insert(section.to_owned()) {
                    push_manifest_error(
                        &mut manifest.diagnostics,
                        line_index,
                        "E4011",
                        format!("duplicate manifest section `{section}`"),
                    );
                }
                current_section = Some(section.to_owned());
                manifest
                    .sections
                    .push((section.to_owned(), spans[line_index]));
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                push_manifest_error(
                    &mut manifest.diagnostics,
                    line_index,
                    "E4012",
                    "expected manifest assignment `key = \"value\"`",
                );
                continue;
            };
            let Some(section) = current_section.as_deref() else {
                push_manifest_error(
                    &mut manifest.diagnostics,
                    line_index,
                    "E4013",
                    "manifest assignment must appear inside a section",
                );
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                push_manifest_error(
                    &mut manifest.diagnostics,
                    line_index,
                    "E4016",
                    "manifest key cannot be empty",
                );
                continue;
            }
            validate_manifest_key(section, key, line_index, &mut manifest.diagnostics);
            if !seen_keys.insert((section.to_owned(), key.to_owned())) {
                push_manifest_error(
                    &mut manifest.diagnostics,
                    line_index,
                    "E4014",
                    format!("duplicate manifest key `{section}.{key}`"),
                );
            }
            let Some(value) = parse_quoted_value(value.trim()) else {
                push_manifest_error(
                    &mut manifest.diagnostics,
                    line_index,
                    "E4015",
                    format!("manifest key `{section}.{key}` expects a quoted string value"),
                );
                continue;
            };
            manifest.entries.push(ManifestEntry {
                section: section.to_owned(),
                key: key.to_owned(),
                value,
                span: spans[line_index],
            });
        }

        manifest
    }

    fn value(&self, section: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.section == section && entry.key == key)
            .map(|entry| entry.value.as_str())
    }

    fn attach(&self, section: &str, key: &str, diagnostics: &mut [Diagnostic]) {
        if let (Some(file), Some(entry)) = (
            &self.source_file,
            self.entries
                .iter()
                .find(|entry| entry.section == section && entry.key == key),
        ) {
            SourceLocation {
                file: file.clone(),
                span: entry.span,
            }
            .attach(diagnostics);
        }
    }

    fn entries_in_section<'a>(
        &'a self,
        section: &'a str,
    ) -> impl Iterator<Item = &'a ManifestEntry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| entry.section == section)
    }
}

#[derive(Debug)]
struct ManifestEntry {
    section: String,
    key: String,
    value: String,
    span: Span,
}

fn validate_manifest_key(
    section: &str,
    key: &str,
    line_index: usize,
    diagnostics: &mut Diagnostics,
) {
    let supported = match section {
        "project" => PROJECT_KEYS.contains(&key),
        "runtime" => RUNTIME_KEYS.contains(&key),
        "services" => !key.is_empty(),
        section if section.starts_with("dependencies.") => key == "path",
        _ => true,
    };
    if !supported {
        push_manifest_error(
            diagnostics,
            line_index,
            "E4016",
            format!("unknown manifest key `{section}.{key}`"),
        );
    }
}

fn strip_line_comment<'a>(line: &'a str, marker: &str) -> &'a str {
    let mut escaped = false;
    let mut quoted = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quoted {
            escaped = true;
            continue;
        }
        if character == '"' {
            quoted = !quoted;
            continue;
        }
        if !quoted && line[index..].starts_with(marker) {
            return &line[..index];
        }
    }
    line
}

fn parse_quoted_value(value: &str) -> Option<String> {
    let mut chars = value.chars();
    if chars.next() != Some('"') {
        return None;
    }
    let mut parsed = String::new();
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if escaped {
            match character {
                '"' => parsed.push('"'),
                '\\' => parsed.push('\\'),
                'n' => parsed.push('\n'),
                'r' => parsed.push('\r'),
                't' => parsed.push('\t'),
                _ => return None,
            }
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            return if chars.as_str().trim().is_empty() {
                Some(parsed)
            } else {
                None
            };
        }
        parsed.push(character);
    }
    None
}

// Only use for errors emitted while scanning one known file. Their line indices
// are real; later wiring validators use retained declaration locations instead.
fn attach_line_locations(path: &Path, source: &str, diagnostics: &mut [Diagnostic]) {
    if diagnostics.is_empty() {
        return;
    }
    let spans = source_line_spans(source);
    for diagnostic in diagnostics {
        if let Some(span) = spans.get(diagnostic.span.line) {
            diagnostic.span = *span;
            diagnostic.source_file = Some(path.to_string_lossy().into_owned());
        }
    }
}

fn source_line_spans(source: &str) -> Vec<Span> {
    let mut offset = 0;
    source
        .split_inclusive('\n')
        .enumerate()
        .map(|(line, segment)| {
            let content = segment.strip_suffix('\n').map_or(segment, |content| {
                content.strip_suffix('\r').unwrap_or(content)
            });
            let span = Span {
                start: offset,
                end: offset + content.len(),
                line,
                column: 0,
            };
            offset += segment.len();
            span
        })
        .collect()
}

fn push_error(diagnostics: &mut Diagnostics, code: &'static str, message: impl Into<String>) {
    diagnostics.push(Diagnostic {
        source_file: None,
        severity: Severity::Error,
        code,
        message: message.into(),
        span: Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        },
    });
}

fn push_manifest_error(
    diagnostics: &mut Diagnostics,
    line_index: usize,
    code: &'static str,
    message: impl Into<String>,
) {
    diagnostics.push(Diagnostic {
        source_file: None,
        severity: Severity::Error,
        code,
        message: message.into(),
        span: Span {
            start: 0,
            end: 0,
            line: line_index,
            column: 0,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn source_discovery_handles_nested_and_sibling_directories() {
        let project = TestProject::new();
        let deep = format!("{}leaf.svr", "d/".repeat(48));
        project.write_file(&deep, "fn deep() {}");
        project.write_file("z/last.svr", "fn last() {}");
        project.write_file("a/first.svr", "fn first() {}");
        project.write_file("root.svr", "fn root() {}");
        project.write_file("ignored.txt", "not source");
        let mut diagnostics = Diagnostics::new();
        let files = collect_source_files(project.path(), &mut diagnostics);
        let mut expected = vec![
            deep,
            "z/last.svr".into(),
            "a/first.svr".into(),
            "root.svr".into(),
        ]
        .into_iter()
        .map(|path| project.path().join(path))
        .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(files, expected);
        assert!(diagnostics.is_empty());
        assert!(collect_source_files(&project.path().join("missing"), &mut diagnostics).is_empty());
        assert_eq!(diagnostics.items[0].code, "E4006");
    }

    #[test]
    fn service_signature_resolution_rejects_missing_annotations_and_owner_failures() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"types\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file("main.svr", "service mail {\nfn send(value: String);\n}\n");
        let checked = check_project(project.path()).unwrap();
        let good = service_types::resolve_service_signatures(&checked);
        assert!(good.diagnostics.is_empty());
        assert_eq!(
            good.operations[0].return_type,
            crate::compiler::semantic::Type::Unit
        );
        assert!(!good.operations[0].has_body);
        let mut missing = checked.clone();
        missing.service_operations[0].parameters[0].annotation = None;
        let report = service_types::resolve_service_signatures(&missing);
        assert!(report.operations.is_empty());
        assert_eq!(report.diagnostics.items[0].code, "E4097");
        let mut missing_owner = checked;
        missing_owner.service_operations[0].source_file = project.path().join("missing.svr");
        let report = service_types::resolve_service_signatures(&missing_owner);
        assert!(report.operations.is_empty());
        assert_eq!(report.diagnostics.items[0].code, "E4118");
    }

    #[test]
    fn application_inspection_understands_standard_input_line_fields() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"input-inspection\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "fn main() { let line: std::InputLine = std::read_line(); if (line.eof) { print(line.text); } }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file(
            "main.svr",
            "fn main() { let line = std::read_line(); print(line.absent); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|item| item.code == "E4138"));
    }

    #[test]
    fn application_inspection_understands_text_file_results() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"file-inspection\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "main.svr",
            "fn main() { let loaded: std::TextRead = std::read_text(\"x\"); if (loaded.ok) { let saved: std::TextWrite = std::write_text(\"y\", loaded.text); print(saved.error); } }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn application_inspection_checks_mutable_reassignment() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"mutable-app\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "main.svr",
            "fn main() { let mut count: Int = 1; count = 2; print(count); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );

        project.write_file("main.svr", "fn main() { let count: Int = 1; count = 2; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|item| item.code == "E4140"));
        project.write_file("main.svr", "fn main() { let mut values = [1.0]; values = [2]; let mut rows = [[1.0]]; rows = [[2]]; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );

        project.write_file(
            "main.svr",
            "fn main() { let mut values = [1]; values = [\"wrong\"]; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|item| item.code == "E4140"));

        project.write_file(
            "main.svr",
            "fn main() { let count = 1; { let mut count = 2; count = 3; } count = 4; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(
            report
                .diagnostics
                .items
                .iter()
                .filter(|item| item.code == "E4140")
                .count(),
            1
        );

        project.write_file(
            "main.svr",
            "fn main() { let mut count: Int = 1; count = \"wrong\"; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|item| item.code == "E4140"));
    }

    #[test]
    fn application_inspection_checks_indexed_assignment() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"indexed-app\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "main.svr",
            "fn main() { let mut values = [1.0]; values[0] = 2; print(values[0]); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file(
            "main.svr",
            "fn main() { let mut rows = [[1.0]]; rows[0] = [2]; rows[0] = []; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        for source in [
            "fn main() { let values = [1]; values[0] = 2; }",
            "fn main() { let mut values = [1]; values[false] = 2; }",
            "fn main() { let mut values = [1]; values[0] = \"wrong\"; }",
            "fn main() { let mut value = 1; value[0] = 2; }",
            "fn main() { missing[0] = 2; }",
            "fn main() { let mut rows = [[1]]; rows[0][0] = 2; }",
        ] {
            project.write_file("main.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .any(|item| item.code == "E4141"),
                "{source}: {:?}",
                report.diagnostics.items
            );
        }
    }

    #[test]
    fn application_inspection_identifies_invalid_array_expressions() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"array-errors\"\nentry = \"main.svr\"",
        );
        for (source, code) in [
            ("fn main() { let value = [1][false]; }", "E4142"),
            ("fn main() { let value = 1[0]; }", "E4142"),
            ("fn main() { let value = [1, \"wrong\"]; }", "E4143"),
            ("fn main() { let value = [[1], [\"wrong\"]]; }", "E4143"),
            ("fn main() { print([1, \"wrong\"]); }", "E4143"),
            ("fn main() { print([1][false]); }", "E4142"),
        ] {
            project.write_file("main.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .any(|item| item.code == code),
                "{source}: {:?}",
                report.diagnostics.items
            );
        }
        project.write_file(
            "main.svr",
            "fn main() { let values = [1, 2]; print(values[0]); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn application_inspection_checks_task_return_contracts() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"task-results\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "task idle() {}\ntask stop() -> Unit { return; }\ntask count() -> Int { if true { return 1; } else { return 2; } }\nfn main() {}");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        for source in [
            "task count() -> Int {}\nfn main() {}",
            "task count() -> Int { return \"wrong\"; }\nfn main() {}",
            "task count() -> Int { while true { return 1; } }\nfn main() {}",
            "task count() -> Int { return missing; }\nfn main() {}",
            "task unexpected() { return 1; }\nfn main() {}",
            "struct A { value: Int }\nstruct B { value: Int }\ntask choose() -> A { return B { value: 1 }; }\nfn main() {}",
        ] {
            project.write_file("main.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(report.diagnostics.items.iter().any(|item| item.code == "E4131"), "{source}: {:?}", report.diagnostics.items);
        }
    }

    #[test]
    fn task_collisions_with_exports_never_supply_imported_callables() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"collisions\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "use library;\nfn main() { library::work(); }");
        for source in [
            "export fn work() {}\ntask work() {}",
            "task work() {}\nexport fn work() {}",
        ] {
            project.write_file("library.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert_eq!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .map(|error| error.code)
                    .collect::<Vec<_>>(),
                ["E4127", "E4133"]
            );
        }
    }

    #[test]
    fn task_parameters_resolve_local_aliases_and_direct_imported_records() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"task-types\"\nentry = \"main.svr\"",
        );
        project.write_file("app/shapes.svr", "export struct Point { x: Float }");
        project.write_file("main.svr", "use app.shapes;\ntype Count = Int;\ntask scheduled(count: Count, point: app::shapes::Point) -> app::shapes::Point { let x: Float = point.x; return point; }\ntask count() -> Count { return 1; }\nfn main() {}");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file(
            "main.svr",
            "task scheduled(value: Missing) {}\nfn main() {}",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 1);
        assert_eq!(report.diagnostics.items[0].code, "E4134");
        assert!(report.diagnostics.items[0].source_file.is_some());
    }

    #[test]
    fn application_duplicate_parameter_names_fail_for_all_callable_kinds() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"duplicate-parameters\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"");
        project.write_file("main.svr", "service mail {\nfn send(value: Int, value: Int);\n}\nfn ordinary(value: Int, value: Int) {}\ntask scheduled(value: Int, value: Int) {}\nfn main() {}");
        let diagnostics = check_project(project.path()).unwrap_err();
        assert_eq!(
            diagnostics
                .items
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            ["E4027", "E4098", "E4098"]
        );
    }

    #[test]
    fn exported_function_results_carry_fields_without_transitive_type_names() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"factory-records\"\nentry = \"main.svr\"",
        );
        project.write_file("app/shapes.svr", "export struct Point { x: Float }");
        project.write_file("app/factory.svr", "use app.shapes;\nexport fn make() -> app::shapes::Point { return app::shapes::Point { x: 1 }; }");
        project.write_file("main.svr", "use app.factory;\nfn main() { let point = app::factory::make(); let x: Float = point.x; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file(
            "main.svr",
            "use app.factory;\nfn main() { let point = app::shapes::Point { x: 1 }; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4139"));
    }

    #[test]
    fn exported_functions_cannot_smuggle_private_records_through_aliases() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"private-exports\"\nentry = \"main.svr\"",
        );
        project.write_file("app/factory.svr", "struct Secret { x: Int }\ntype Hidden = Secret;\nexport fn make() -> Hidden { return Hidden { x: 1 }; }");
        project.write_file(
            "main.svr",
            "use app.factory;\nfn main() { app::factory::make(); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4116"
                && error
                    .source_file
                    .as_ref()
                    .is_some_and(|file| file.ends_with("factory.svr"))));
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4133"
                && error
                    .source_file
                    .as_ref()
                    .is_some_and(|file| file.ends_with("main.svr"))));
    }

    #[test]
    fn exported_application_functions_preserve_record_interfaces_and_fields() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"record-functions\"\nentry = \"main.svr\"",
        );
        project.write_file("app/shapes.svr", "export struct Point { x: Float }\ntype Position = Point;\nexport fn make(x: Float) -> Position { return Position { x: x }; }\nexport fn read(point: Position) -> Float { return point.x; }");
        project.write_file("main.svr", "use app.shapes;\nstruct Point { x: String }\nfn main() { let point = app::shapes::make(1); let x: Float = point.x; let value = app::shapes::read(point); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn application_type_graph_resolves_chains_without_transitive_names() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"type-chain\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"");
        project.write_file("app/n0.svr", "export struct R0 { x: Float }");
        for index in 1..5 {
            project.write_file(
                &format!("app/n{index}.svr"),
                &format!(
                    "use app.n{};\nexport struct R{index} {{ next: app::n{}::R{} }}",
                    index - 1,
                    index - 1,
                    index - 1
                ),
            );
        }
        project.write_file("main.svr", "use app.n4;\nservice store {\nfn load() -> app::n4::R4;\n}\nfn main() { let value = store.load(); let x: Float = value.next.next.next.next.x; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn cyclic_application_record_dependencies_fail_without_placeholder_types() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"type-cycle\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "use app.a;\nfn main() {}");
        project.write_file("app/a.svr", "use app.b;\nexport struct A { b: app::b::B }");
        project.write_file("app/b.svr", "use app.a;\nexport struct B { a: app::a::A }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E3017"));
    }

    #[test]
    fn imported_record_aliases_and_fields_share_nominal_interfaces() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"type-graph\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"");
        project.write_file("app/shapes.svr", "export struct Point { x: Float }");
        project.write_file("app/wrapper.svr", "use app.shapes;\ntype Position = app::shapes::Point;\nexport struct Box { point: Position }\nservice store {\nfn load() -> Box;\n}");
        project.write_file("main.svr", "use app.wrapper;\nuse app.shapes;\ntype Position = app::shapes::Point;\nfn main() { let point: Position = Position { x: 1 }; let box = app::wrapper::Box { point: point }; let x: Float = box.point.x; let loaded = store.load(); let y: Float = loaded.point.x; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn invalid_imported_record_graphs_never_supply_alias_or_field_types() {
        for declaration in [
            "struct Point { x: Float }",
            "export struct Point { x: Missing }",
            "export struct Point { x: Float }\nfn broken() { @ }",
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"invalid-graph\"\nentry = \"main.svr\"",
            );
            project.write_file("app/shapes.svr", declaration);
            project.write_file("main.svr", "use app.shapes;\ntype Position = app::shapes::Point;\nexport struct Box { point: Position }\nfn main() {}");
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .any(|error| error.code == "E3017"
                        && error
                            .source_file
                            .as_ref()
                            .is_some_and(|file| file.ends_with("main.svr"))),
                "{:?}",
                report.diagnostics.items
            );
        }
    }

    #[test]
    fn imported_service_contracts_carry_fields_without_reexporting_type_names() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"service-records\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"");
        project.write_file("app/shapes.svr", "export struct Point { x: Float }");
        project.write_file(
            "app/store.svr",
            "use app.shapes;\nservice store {\nfn load() -> app::shapes::Point;\n}",
        );
        project.write_file(
            "main.svr",
            "use app.store;\nfn main() { let value = store.load(); let x: Float = value.x; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file(
            "main.svr",
            "use app.store;\nfn main() { let value = app::shapes::Point { x: 1 }; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4139"));
    }

    #[test]
    fn service_record_annotations_reject_private_and_transitive_names() {
        for (record, import) in [
            ("struct Point { x: Float }", "use app.shapes;"),
            ("export struct Point { x: Float }", "use app.bridge;"),
        ] {
            let project = TestProject::new();
            project.write_file("sovra.toml", "[project]\nname = \"service-private\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"");
            project.write_file("app/shapes.svr", record);
            project.write_file("app/bridge.svr", "use app.shapes;\nfn helper() {}");
            project.write_file("main.svr", &format!("{import}\nservice store {{\nfn save(value: app::shapes::Point);\n}}\nfn main() {{}}"));
            let checked = check_project(project.path()).unwrap();
            let signatures = service_types::resolve_service_signatures(&checked);
            assert!(signatures.operations.is_empty());
            assert_eq!(signatures.diagnostics.items.len(), 1);
            assert_eq!(signatures.diagnostics.items[0].code, "E4117");
        }
    }

    #[test]
    fn application_services_resolve_direct_imported_record_contracts() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"service-record-imports\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"");
        project.write_file("app/shapes.svr", "export struct Point { x: Float }");
        project.write_file("main.svr", "use app.shapes;\nservice store {\nfn echo(value: app::shapes::Point) -> app::shapes::Point { return value; }\n}\nfn main() { let value = store.echo(app::shapes::Point { x: 1 }); let x: Float = value.x; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn qualified_application_records_construct_and_typecheck_without_services() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"qualified-records\"\nentry = \"main.svr\"",
        );
        project.write_file("app/shapes.svr", "export struct Point { x: Float }");
        project.write_file("main.svr", "use app.shapes;\nuse app.shapes;\nfn identity(value: app::shapes::Point) -> app::shapes::Point { return value; }\nfn main() { let point: app::shapes::Point = identity(app::shapes::Point { x: 1 }); let x: Float = point.x; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn qualified_application_records_preserve_privacy_and_direct_import_boundaries() {
        for (declaration, import) in [
            ("struct Point { x: Float }", "use app.shapes;"),
            ("export struct Point { x: Float }", "use app.bridge;"),
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"private-records\"\nentry = \"main.svr\"",
            );
            project.write_file("app/shapes.svr", declaration);
            project.write_file("app/bridge.svr", "use app.shapes;\nfn helper() {}");
            project.write_file(
                "main.svr",
                &format!("{import}\nfn main() {{ let value = app::shapes::Point {{ x: 1 }}; }}"),
            );
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .any(|error| error.code == "E4139"),
                "{:?}",
                report.diagnostics.items
            );
        }
    }

    #[test]
    fn same_named_imported_application_records_are_not_interchangeable() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"nominal-imports\"\nentry = \"main.svr\"",
        );
        for module in ["a", "b"] {
            project.write_file(
                &format!("app/{module}.svr"),
                "export struct Point { x: Float }",
            );
        }
        project.write_file("main.svr", "use app.a;\nuse app.b;\nfn take(value: app::a::Point) {}\nfn main() { take(app::b::Point { x: 1 }); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(
            report.diagnostics.items.len(),
            1,
            "{:?}",
            report.diagnostics.items
        );
        assert_eq!(report.diagnostics.items[0].code, "E4129");
    }

    #[test]
    fn application_record_equality_respects_nominal_identity() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"record-equality\"\nentry = \"main.svr\"",
        );
        for module in ["a", "b"] {
            project.write_file(
                &format!("app/{module}.svr"),
                "export struct Point { x: Int }",
            );
        }
        project.write_file("main.svr", "use app.a;\nuse app.b;\nfn main() { let same = app::a::Point { x: 1 } == app::a::Point { x: 1 }; if same {} }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file("main.svr", "use app.a;\nuse app.b;\nfn main() { let wrong = app::a::Point { x: 1 } == app::b::Point { x: 1 }; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4121"));
    }

    #[test]
    fn application_array_literals_and_indexing_propagate_validated_types() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"array-inspection\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "fn main() { let values = [1, 2]; let first = values[0]; let same = values == [1, 2]; if same { print(first); } }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        project.write_file(
            "main.svr",
            "fn main() { let mixed = [1, \"two\"]; let invalid = [1][false]; }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4136"));
    }

    #[test]
    fn imported_service_records_expose_only_exported_fields() {
        for exported in [true, false] {
            let project = TestProject::new();
            project.write_file("sovra.toml", "[project]\nname = \"public-fields\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"");
            let prefix = if exported { "export " } else { "" };
            project.write_file("app/store.svr", &format!("{prefix}struct Point {{ x: Float }}\n{prefix}struct Box {{ point: Point }}\nservice store {{\nfn load() -> Box;\n}}"));
            project.write_file("main.svr", "use app.store;\nstruct Point { x: String }\nfn main() { let value = store.load(); let x: Float = value.point.x; print(x); }");
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(report.files.iter().all(|file| file.functions.is_ok()));
            if exported {
                assert!(
                    report.diagnostics.is_empty(),
                    "{:?}",
                    report.diagnostics.items
                );
            } else {
                assert!(report
                    .diagnostics
                    .items
                    .iter()
                    .any(|error| error.code == "E4138"));
            }
        }
    }

    #[test]
    fn exported_application_record_fields_cannot_expose_private_alias_targets() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"private-fields\"\nentry = \"main.svr\"",
        );
        let source = "struct Secret { value: Int }\ntype Hidden = Secret;\nexport struct Public { hidden: Hidden }\nfn main() {}";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        let error = report
            .diagnostics
            .items
            .iter()
            .find(|error| error.code == "E4116")
            .expect("private field diagnostic");
        assert_eq!(&source[error.span.start..error.span.end], "hidden: Hidden");
    }

    #[test]
    fn application_record_construction_validates_nested_values_and_widening() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"constructors\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "struct Point { x: Float }\nstruct Box { point: Point }\ntype Position = Point;\nfn valid(point: Point) -> Bool { return point.x > 0; }\nfn main() { let value = Box { point: Position { x: 1 } }; let x: Float = value.point.x; let flag = true; if flag {} if valid(Point { x: 2 }) {} if (Point { x: 3 }.x > 0) {} }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn invalid_application_record_constructors_do_not_propagate_types() {
        for expression in [
            "Point {}",
            "Point { x: 1, x: 2 }",
            "Point { y: 1 }",
            "Point { x: true }",
            "Point { x: missing }",
            "Unknown { x: 1 }",
            "Int { x: 1 }",
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"constructors\"\nentry = \"main.svr\"",
            );
            let source =
                format!("struct Point {{ x: Float }}\nfn main() {{ let invalid = {expression}; }}");
            project.write_file("main.svr", &source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            let error = report
                .diagnostics
                .items
                .iter()
                .find(|error| error.code == "E4139")
                .unwrap_or_else(|| panic!("{expression}: {:?}", report.diagnostics.items));
            assert_eq!(&source[error.span.start..error.span.end], expression);
            assert!(
                report.files[0].functions.as_ref().unwrap()[0].local_bindings[0]
                    .resolved_type
                    .is_none()
            );
        }
    }

    #[test]
    fn application_record_fields_propagate_nested_nominal_and_scalar_types() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"fields\"\nentry = \"main.svr\"\n[services]\nstore = \"external\"",
        );
        project.write_file("main.svr", "type Scalar = Float;\nstruct Point { x: Scalar }\nstruct Box { point: Point }\nservice store {\nfn load() -> Box;\nfn save(value: Float);\n}\nfn read(value: Box) -> Float { return value.point.x; }\nfn main() { let value = store.load(); { let point = value.point; store.save(point.x); } store.save(read(value)); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        let functions = report.files[0].functions.as_ref().unwrap();
        assert_eq!(
            functions[0].returns[0].known_type,
            Some(crate::compiler::semantic::Type::Float)
        );
    }

    #[test]
    fn invalid_application_record_fields_and_field_calls_are_rejected() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"fields\"\nentry = \"main.svr\"",
        );
        let source = "struct Point { x: Float }\nfn inspect(point: Point) { point.absent; point.x(); 1.absent; }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        let errors: Vec<_> = report
            .diagnostics
            .items
            .iter()
            .filter(|error| error.code == "E4138")
            .collect();
        assert_eq!(errors.len(), 2, "{:?}", report.diagnostics.items);
        assert_eq!(
            &source[errors[0].span.start..errors[0].span.end],
            "point.absent"
        );
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4133"));
    }

    #[test]
    fn application_record_contracts_preserve_nominal_identity() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"record-contracts\"\nentry = \"main.svr\"\n[services]\nsource = \"external\"\nsink = \"external\"");
        project.write_file("main.svr", "use app.sink;\nstruct Point { x: Float }\ntype Position = Point;\nservice source {\nfn get() -> Point;\n}\nfn identity(value: Position) -> Point { return value; }\nfn main() { let point: Position = source.get(); identity(point); sink.put(point); }");
        project.write_file(
            "app/sink.svr",
            "struct Point { x: Float }\nservice sink {\nfn put(value: Point);\n}",
        );
        let checked = check_project(project.path()).unwrap();
        let report = application::check_service_calls(&checked);
        assert!(report.files.iter().all(|file| file.functions.is_ok()));
        assert_eq!(
            report.diagnostics.items.len(),
            1,
            "{:?}",
            report.diagnostics.items
        );
        assert_eq!(report.diagnostics.items[0].code, "E4119");
        let signatures = service_types::resolve_service_signatures(&checked);
        let get = signatures
            .operations
            .iter()
            .find(|item| item.name == "get")
            .unwrap();
        let put = signatures
            .operations
            .iter()
            .find(|item| item.name == "put")
            .unwrap();
        assert_ne!(get.return_type, put.parameters[0].parameter_type);
    }

    #[test]
    fn application_record_declarations_validate_fields_and_privacy() {
        for (source, expected) in [
            ("struct Point { x: Missing }\nfn main() {}", "E3017"),
            ("struct Point { x: Int, x: Float }\nfn main() {}", "E3008"),
            (
                "struct Point { x: Int }\ntype Point = Int;\nfn main() {}",
                "E3008",
            ),
            (
                "struct Point { x: Int }\nexport fn echo(value: Point) -> Point { return value; }",
                "E4116",
            ),
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"record-errors\"\nentry = \"main.svr\"",
            );
            project.write_file("main.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(report.files[0].functions.is_ok(), "{source}");
            assert!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .any(|error| error.code == expected),
                "{:?}",
                report.diagnostics.items
            );
        }
    }

    #[test]
    fn application_integer_literals_enforce_signed_64_bit_bounds() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"integer-bounds\"\nentry = \"main.svr\"",
        );
        let source = "fn value() -> Int { return 9223372036854775808; }\nfn main() { let large = 18446744073709551616; print(999999999999999999999999); 9223372036854775807; }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        let errors: Vec<_> = report
            .diagnostics
            .items
            .iter()
            .filter(|error| error.code == "E3012")
            .collect();
        assert_eq!(errors.len(), 3);
        for (error, spelling) in errors.iter().zip([
            "9223372036854775808",
            "18446744073709551616",
            "999999999999999999999999",
        ]) {
            assert_eq!(&source[error.span.start..error.span.end], spelling);
            assert!(error.source_file.as_ref().unwrap().ends_with("main.svr"));
        }
    }

    #[test]
    fn application_service_aliases_resolve_from_the_supplied_snapshot() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"alias-snapshot\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"");
        let original = "type Amount = Float;\nservice mail {\nfn send(value: Amount);\n}";
        project.write_file("main.svr", original);
        let checked = check_project(project.path()).unwrap();
        let sources = checked
            .source_files
            .iter()
            .map(|file| (file.clone(), Ok(original.to_owned())))
            .collect();
        project.write_file(
            "main.svr",
            "type Amount = String;\nservice mail {\nfn send(value: Amount);\n}",
        );
        let report = service_types::resolve_with_sources(&checked, &sources);
        assert!(report.diagnostics.is_empty());
        assert_eq!(
            report.operations[0].parameters[0].parameter_type,
            crate::compiler::semantic::Type::Float
        );
        let current = service_types::resolve_service_signatures(&checked);
        assert_eq!(
            current.operations[0].parameters[0].parameter_type,
            crate::compiler::semantic::Type::String
        );
    }

    #[test]
    fn application_aliases_apply_to_service_bodies_and_task_parameters() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"alias-bodies\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"");
        project.write_file("main.svr", "type Amount = Float;\nservice mail {\nfn relay(value: Amount) -> Amount { let copy: Amount = value; return copy; }\n}\ntask refresh(value: Amount) { mail.relay(value); }\nfn main() { mail.relay(1); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        let functions = report.files[0].functions.as_ref().unwrap();
        assert_eq!(
            functions[0].returns[0].known_type,
            Some(crate::compiler::semantic::Type::Float)
        );
        assert!(functions[1].is_task);
        assert_eq!(
            functions[1].calls[0].argument_types[0].resolved_type(),
            Some(&crate::compiler::semantic::Type::Float)
        );
    }

    #[test]
    fn malformed_and_nested_application_aliases_are_not_claimed_as_covered() {
        for source in [
            "type Amount = Float\nfn main() {}",
            "export type Amount = Float;\nfn main() {}",
            "fn main() { type Amount = Float; }",
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"alias-syntax\"\nentry = \"main.svr\"",
            );
            project.write_file("main.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(report.files[0].functions.is_err(), "{source}");
            assert!(report
                .diagnostics
                .items
                .iter()
                .any(|error| error.code == "E4096"));
        }
    }

    #[test]
    fn imported_application_aliases_keep_declaring_file_meaning() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"alias-imports\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "app/helpers.svr",
            "type Scalar = Float;\nexport fn scale(value: Scalar) -> Scalar { return value; }",
        );
        project.write_file("main.svr", "use app.helpers;\ntype Scalar = String;\nfn main() { let text: Scalar = \"value\"; let value = app::helpers::scale(1); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        let main = report
            .files
            .iter()
            .find(|file| file.source_file.ends_with("main.svr"))
            .unwrap();
        let functions = main.functions.as_ref().unwrap();
        assert_eq!(
            functions[0].local_bindings[0].resolved_type,
            Some(crate::compiler::semantic::Type::String)
        );
        assert_eq!(
            functions[0].local_bindings[1].resolved_type,
            Some(crate::compiler::semantic::Type::Float)
        );
        project.write_file("main.svr", "use app.helpers;\nfn main() { app::helpers::scale(\"wrong\"); let hidden: Scalar = 1; }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4129"));
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4135"));
    }

    #[test]
    fn unused_application_alias_errors_retain_owning_file_and_ranges() {
        for (source, expected) in [
            ("// λ\r\ntype Value = Missing;\r\nfn main() {}", "E3017"),
            (
                "type Value = Int;\ntype Value = Float;\nfn main() {}",
                "E3008",
            ),
            ("type A = B; type B = A;\nfn main() {}", "E3017"),
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"bad-aliases\"\nentry = \"main.svr\"",
            );
            project.write_file("main.svr", source);
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(report.files.iter().all(|file| file.functions.is_ok()));
            assert!(!report.diagnostics.is_empty());
            for error in &report.diagnostics.items {
                assert_eq!(error.code, expected);
                assert!(error.source_file.as_ref().unwrap().ends_with("main.svr"));
                assert!(source[error.span.start..error.span.end].starts_with("type "));
            }
        }
    }

    #[test]
    fn application_scalar_aliases_resolve_in_functions_services_and_locals() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"aliases\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file("main.svr", "type Amount = Scalar;\ntype Scalar = Float;\nservice mail {\nfn send(value: Amount) -> Amount;\n}\nfn echo(value: Amount) -> Amount { return value; }\nfn main() { let value: Amount = echo(1); mail.send(value); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        let functions = report.files[0].functions.as_ref().unwrap();
        assert_eq!(
            functions[1].local_bindings[0].resolved_type,
            Some(crate::compiler::semantic::Type::Float)
        );
    }

    #[test]
    fn discarded_application_expressions_require_resolved_types() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"discarded\"\nentry = \"main.svr\"",
        );
        let source = "fn main() { missing; { absent + 1; } if true { unknown.field; } let valid = 1; valid; true && false; print(valid); }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 3);
        for (error, spelling) in
            report
                .diagnostics
                .items
                .iter()
                .zip(["missing", "absent + 1", "unknown.field"])
        {
            assert_eq!(error.code, "E4137");
            assert_eq!(&source[error.span.start..error.span.end], spelling);
        }
    }

    #[test]
    fn unresolved_application_local_types_fail_even_when_unused() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"locals\"\nentry = \"main.svr\"",
        );
        let source =
            "fn main() { let typed: Text = 1; let known: Int = unknown; let inferred = missing; }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 3);
        for (error, (code, spelling)) in report.diagnostics.items.iter().zip([
            ("E4135", "Text"),
            ("E4136", "unknown"),
            ("E4136", "missing"),
        ]) {
            assert_eq!(error.code, code);
            assert_eq!(&source[error.span.start..error.span.end], spelling);
        }
    }

    #[test]
    fn invalid_application_export_interfaces_are_not_consumed() {
        for (source, expected) in [
            (
                "export fn value() -> Int { return 1; }\nfn value() -> Int { return 2; }",
                "E4127",
            ),
            (
                "export fn value() -> Int { return 1; }\nfn broken() { if true }",
                "E4096",
            ),
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"invalidexports\"\nentry = \"main.svr\"",
            );
            project.write_file("helpers.svr", source);
            project.write_file("main.svr", "use helpers;\nfn main() { helpers::value(); }");
            let report = application::check_service_calls(&check_project(project.path()).unwrap());
            assert!(
                report
                    .diagnostics
                    .items
                    .iter()
                    .any(|error| error.code == expected),
                "{:?}",
                report.diagnostics.items
            );
            assert!(report
                .diagnostics
                .items
                .iter()
                .any(|error| error.code == "E4133"));
        }
    }

    #[test]
    fn application_function_imports_preserve_private_and_nontransitive_boundaries() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"imports\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "a.svr",
            "fn hidden() -> Int { return 1; }\nexport fn value() -> Int { return hidden(); }",
        );
        project.write_file("b.svr", "export fn value() -> String { return \"b\"; }");
        project.write_file(
            "bridge.svr",
            "use a;\nexport fn bridge() -> Int { return a::value(); }",
        );
        project.write_file("main.svr", "use bridge;\nuse b;\nfn main() { print(bridge::bridge()); print(b::value()); a::value(); b::hidden(); value(); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(
            report.diagnostics.items.len(),
            3,
            "{:?}",
            report.diagnostics.items
        );
        assert!(report
            .diagnostics
            .items
            .iter()
            .all(|error| error.code == "E4133"
                && error.source_file.as_ref().unwrap().ends_with("main.svr")));
    }

    #[test]
    fn application_function_import_cycles_and_duplicate_imports_are_bounded() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"cycles\"\nentry = \"main.svr\"",
        );
        project.write_file("a.svr", "use b;\nexport fn first(value: Int) -> Int { if value == 0 { return 0; } else { return b::second(value - 1); } }");
        project.write_file(
            "b.svr",
            "use a;\nexport fn second(value: Int) -> Int { return a::first(value); }",
        );
        project.write_file(
            "main.svr",
            "use a;\nuse a;\nfn main() { let a = \"local\"; print(a::first(3)); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.items.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
        let main = report
            .files
            .iter()
            .find(|file| file.source_file.ends_with("main.svr"))
            .unwrap();
        let call = main.functions.as_ref().unwrap()[0]
            .function_calls
            .iter()
            .find(|call| call.name == "a::first")
            .unwrap();
        assert_eq!(
            call.signature.source_file.as_ref().unwrap(),
            &project.path().join("a.svr").canonicalize().unwrap()
        );
    }

    #[test]
    fn imported_function_errors_retain_declaring_file_and_caller_ranges() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"errors\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "helpers.svr",
            "export fn bad(value: Int) -> String { return 1; }",
        );
        project.write_file(
            "main.svr",
            "use helpers;\nfn main() { helpers::bad(true); }",
        );
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(
            report.diagnostics.items.len(),
            2,
            "{:?}",
            report.diagnostics.items
        );
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4131"
                && error.source_file.as_ref().unwrap().ends_with("helpers.svr")));
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4129"
                && error.source_file.as_ref().unwrap().ends_with("main.svr")));
    }

    #[test]
    fn exported_application_functions_resolve_direct_qualified_imports() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"imports\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "main.svr",
            "use app.helpers;\nfn main() { print(app::helpers::format(12)); }",
        );
        project.write_file("app/helpers.svr", "fn prefix() -> String { return \"value: \"; }\nexport fn format(value: Int) -> String { return prefix() + std::to_string(value); }");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert!(
            report.diagnostics.items.is_empty(),
            "{:?}",
            report.diagnostics.items
        );
    }

    #[test]
    fn unresolved_ordinary_calls_fail_with_original_ranges() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"unknowncalls\"\nentry = \"main.svr\"",
        );
        let source = "fn main() { missing(); std::missing(); let helper = 1; helper(); }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 3);
        for (error, spelling) in
            report
                .diagnostics
                .items
                .iter()
                .zip(["missing()", "std::missing()", "helper()"])
        {
            assert_eq!(error.code, "E4133");
            assert_eq!(&source[error.span.start..error.span.end], spelling);
        }
    }

    #[test]
    fn unresolved_service_returns_and_conditions_are_reported() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"unknowns\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\nfn value() -> Int { return unknown; }\n}\nfn main() { if missing {} while unresolved {} }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 3);
        for (error, (code, spelling)) in report.diagnostics.items.iter().zip([
            ("E4126", "unknown"),
            ("E4125", "missing"),
            ("E4125", "unresolved"),
        ]) {
            assert_eq!(error.code, code);
            assert_eq!(&source[error.span.start..error.span.end], spelling);
        }
    }

    #[test]
    fn service_nonunit_bodies_require_returns_in_supported_syntax() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"returns\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\nfn absent() -> Int { let value = 1; }\nfn nested() -> Int { { return 1; } }\nfn external() -> Int;\nfn unit() {}\nfn unresolved() -> Int { return unknown; }\n}\nfn main() {}";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 2);
        assert_eq!(report.diagnostics.items[1].code, "E4126");
        let error = &report.diagnostics.items[0];
        assert_eq!(error.code, "E4123");
        assert_eq!(
            &source[error.span.start..error.span.end],
            "fn absent() -> Int { let value = 1; }"
        );
    }

    #[test]
    fn service_implementation_returns_match_resolved_contracts() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"returns\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\nfn bad() -> String { return 1; }\nfn empty() -> Int { return; }\nfn widened() -> Float { return 1; }\nfn unit() { return true; }\n}\nfn main() {}";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 3);
        for (error, expected) in report.diagnostics.items.iter().zip(["1", "return", "true"]) {
            assert_eq!(error.code, "E4122");
            assert_eq!(&source[error.span.start..error.span.end], expected);
        }
    }

    #[test]
    fn service_result_types_do_not_escape_invalid_or_unknown_calls() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"results\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file("main.svr", "service mail {\nfn echo(value: String) -> String;\nfn count(value: Int);\n}\nfn main() { mail.count(mail.echo(true)); mail.count(mail.echo()); mail.count(mail.echo(unknown)); }\n");
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(
            report
                .diagnostics
                .items
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            ["E4119", "E4094"]
        );
    }

    #[test]
    fn service_results_flow_through_nested_calls_and_locals() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"results\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file(
            "service.svr",
            "service mail {\nfn amount() -> Float;\nfn count(value: Int);\n}\n",
        );
        let source = "use service\nfn main() { mail.count(mail.amount()); let amount = mail.amount(); mail.count(amount); }\nfn shadow(mail: String) { mail.count(mail.amount()); }";
        project.write_file("main.svr", source);
        let report = application::check_service_calls(&check_project(project.path()).unwrap());
        assert_eq!(report.diagnostics.items.len(), 4);
        assert!(report.diagnostics.items[..2]
            .iter()
            .all(|error| error.code == "E4133"));
        for (error, expected) in report
            .diagnostics
            .items
            .iter()
            .skip(2)
            .zip(["mail.amount()", "amount"])
        {
            assert_eq!(error.code, "E4119");
            assert_eq!(&source[error.span.start..error.span.end], expected);
        }
    }

    #[test]
    fn annotated_service_locals_validate_before_propagating() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"locals\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\nfn count(value: Int);\n}\nfn main() { let widened: Float = 1; mail.count(widened); let invalid: String = true; mail.count(invalid); }";
        project.write_file("main.svr", source);
        let checked = check_project(project.path()).unwrap();
        let report = application::check_service_calls(&checked);
        assert_eq!(report.diagnostics.items.len(), 2);
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4119"
                && &source[error.span.start..error.span.end] == "widened"));
        assert!(report
            .diagnostics
            .items
            .iter()
            .any(|error| error.code == "E4120"
                && &source[error.span.start..error.span.end] == "true"));
    }

    #[test]
    fn service_literal_arguments_respect_contracts_and_widening() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"arguments\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"");
        let source = "service mail {\nfn send(value: String);\nfn scale(value: Float);\n}\nfn main() { mail.send((true)); mail.scale(1); mail.scale(1.5); mail.send(\"ok\"); }\nfn local(mail: String) { mail.send(false); }\n";
        project.write_file("main.svr", source);
        let checked = check_project(project.path()).unwrap();
        let report = application::check_service_calls(&checked);
        assert_eq!(report.diagnostics.items.len(), 2);
        assert_eq!(report.diagnostics.items[0].code, "E4133");
        let error = &report.diagnostics.items[1];
        assert_eq!(error.code, "E4119");
        assert_eq!(&source[error.span.start..error.span.end], "(true)");
        assert!(error.source_file.as_ref().unwrap().ends_with("main.svr"));
    }

    #[test]
    fn service_signature_resolution_keeps_only_complete_canonical_contracts() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"types\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "// Unicode: λ\r\nservice mail {\r\nfn send(value: String, count: Int, ratio: Float, enabled: Bool, context: Unit) -> String;\r\nfn ping() {}\r\nfn invalid(value: Text) -> Receipt;\r\n}\r\nfn main() { mail.invalid(1); }\r\n";
        project.write_file("main.svr", source);
        let checked = check_project(project.path()).unwrap();
        let original = checked.clone();
        let resolved = service_types::resolve_service_signatures(&checked);
        assert_eq!(checked, original);
        assert_eq!(resolved.operations.len(), 2);
        assert_eq!(resolved.diagnostics.items.len(), 2);
        let send = &resolved.operations[0];
        assert_eq!(send.return_type, crate::compiler::semantic::Type::String);
        assert_eq!(
            send.parameters
                .iter()
                .map(|parameter| parameter.parameter_type.clone())
                .collect::<Vec<_>>(),
            [
                crate::compiler::semantic::Type::String,
                crate::compiler::semantic::Type::Int,
                crate::compiler::semantic::Type::Float,
                crate::compiler::semantic::Type::Bool,
                crate::compiler::semantic::Type::Unit
            ]
        );
        assert_eq!(
            resolved.operations[1].return_type,
            crate::compiler::semantic::Type::Unit
        );
        assert!(resolved.operations[1].has_body);
        assert_eq!(
            send.service.module,
            project
                .path()
                .join("main.svr")
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        );
        for diagnostic in &resolved.diagnostics.items {
            assert_eq!(diagnostic.code, "E4117");
            assert_eq!(diagnostic.span.line, 4);
            assert_eq!(
                &source[diagnostic.span.start..diagnostic.span.end],
                "fn invalid(value: Text) -> Receipt;"
            );
        }
        let report = application::check_service_calls(&checked);
        assert_eq!(report.diagnostics.items.len(), 2);
        assert!(report
            .diagnostics
            .items
            .iter()
            .all(|error| error.code == "E4117"));
    }

    #[test]
    fn service_call_checks_respect_resolution_and_partial_coverage() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file(
            "services.svr",
            "service mail {\nfn send(value: Int) -> Unit\n}\n",
        );
        let source = "use services\nfn main() { mail.send(); mail.missing(1); mail.send(1); }\nfn local(mail: String) { mail.missing(); }\n";
        project.write_file("main.svr", source);
        project.write_file("unresolved.svr", "fn caller() { mail.missing(); }");
        project.write_file(
            "partial.svr",
            "use services\nfn caller() { mail.missing(); if true }",
        );
        let checked = check_project(project.path()).unwrap();
        let report = application::check_service_calls(&checked);
        assert_eq!(report.diagnostics.items.len(), 5);
        assert_eq!(report.diagnostics.items[0].code, "E4133");
        assert_eq!(report.diagnostics.items[3].code, "E4133");
        assert!(report.diagnostics.items[3]
            .source_file
            .as_ref()
            .unwrap()
            .ends_with("unresolved.svr"));
        assert_eq!(report.diagnostics.items[1].code, "E4094");
        assert_eq!(report.diagnostics.items[2].code, "E4093");
        assert_eq!(report.diagnostics.items[4].code, "E4096");
        assert!(report.diagnostics.items[4].message.contains("partial.svr"));
        assert!(report.diagnostics.items[4].source_file.is_none());
        for (error, expected) in report
            .diagnostics
            .items
            .iter()
            .skip(1)
            .zip(["mail.send", "mail.missing"])
        {
            assert_eq!(&source[error.span.start..error.span.end], expected);
            assert_eq!(
                error.source_file.as_deref(),
                Some(project.path().join("main.svr").to_string_lossy().as_ref())
            );
        }
        assert_eq!(
            report
                .files
                .iter()
                .filter(|file| file.functions.is_err())
                .count(),
            1
        );
    }

    #[test]
    fn service_implementations_validate_imported_calls_with_original_spans() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"\nrelay = \"external\"",
        );
        project.write_file("services.svr", "service mail {\nfn send(value: Int);\n}\n");
        let source = "use services\r\n// λ preserves byte offsets\r\nservice relay {\r\nfn forward() { mail.send(); mail.missing(1); mail.send(1); }\r\nfn shadow(mail: String) { mail.missing(); }\r\nfn again() { mail.send(1); }\r\n}\r\n";
        project.write_file("main.svr", source);
        let checked = check_project(project.path()).unwrap();
        let report = application::check_service_calls(&checked);
        assert_eq!(report.diagnostics.items.len(), 3);
        assert_eq!(report.diagnostics.items[0].code, "E4133");
        assert_eq!(
            &source[report.diagnostics.items[0].span.start..report.diagnostics.items[0].span.end],
            "mail.missing()"
        );
        for (error, (code, text)) in report
            .diagnostics
            .items
            .iter()
            .skip(1)
            .zip([("E4094", "mail.send"), ("E4093", "mail.missing")])
        {
            assert_eq!(error.code, code);
            assert_eq!(&source[error.span.start..error.span.end], text);
            assert_eq!(error.span.line, 3);
            assert_eq!(
                error.source_file.as_deref(),
                Some(project.path().join("main.svr").to_string_lossy().as_ref())
            );
        }
        let functions = report
            .files
            .iter()
            .find(|file| file.source_file == project.path().join("main.svr"))
            .unwrap()
            .functions
            .as_ref()
            .unwrap();
        assert_eq!(
            functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            ["relay.forward", "relay.shadow", "relay.again"]
        );
        assert!(functions.iter().all(|function| !function.is_task));
        assert_eq!(functions[1].calls[0].receiver, scope::Receiver::Local);
        assert!(matches!(
            functions[2].calls[0].receiver,
            scope::Receiver::Service(_)
        ));
    }

    #[test]
    fn project_inspection_uses_direct_imports_and_reports_partial_files() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file(
            "main.svr",
            "use services;\nfn main() { mail.send(); }\nfn shadow(mail: String) { mail.send(); }",
        );
        project.write_file(
            "services.svr",
            "service mail {}\nfn local() { mail.send(); }",
        );
        project.write_file("bridge.svr", "use services\n");
        project.write_file("unimported.svr", "use bridge\nfn other() { mail.send(); }");
        project.write_file("partial.svr", "fn unsupported() { if true }");
        let checked = check_project(project.path()).unwrap();
        let inspections = application::inspect_project(&checked);
        let get = |name: &str| {
            inspections
                .iter()
                .find(|item| item.source_file == project.path().join(name))
                .unwrap()
        };
        let main = get("main.svr").functions.as_ref().unwrap();
        assert!(matches!(
            main[0].calls[0].receiver,
            scope::Receiver::Service(_)
        ));
        assert_eq!(main[1].calls[0].receiver, scope::Receiver::Local);
        assert!(matches!(
            get("services.svr").functions.as_ref().unwrap()[0].calls[0].receiver,
            scope::Receiver::Service(_)
        ));
        assert_eq!(
            get("unimported.svr").functions.as_ref().unwrap()[0].calls[0].receiver,
            scope::Receiver::Unresolved
        );
        assert!(get("partial.svr").functions.is_err());
    }

    #[test]
    fn project_imports_resolve_once_and_allow_cycles() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"",
        );
        project.write_file(
            "main.svr",
            "use app.helper\nuse app.helper; // repeated\nfn main() {}\n",
        );
        project.write_file("app/helper.svr", "use main\nfn helper() {}\n");
        let checked = check_project(project.path()).unwrap();
        assert_eq!(checked.imports.len(), 2);
        let import = checked
            .imports
            .iter()
            .find(|item| item.module == "app.helper")
            .unwrap();
        assert_eq!(
            import.target_file,
            project
                .path()
                .join("app/helper.svr")
                .canonicalize()
                .unwrap()
        );
        assert_eq!(import.span.line, 0);
    }

    #[test]
    fn project_import_errors_identify_the_declaration() {
        for (source, code) in [("use app.missing", "E4091"), ("use ../outside", "E4090")] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"sample\"\nentry = \"main.svr\"",
            );
            project.write_file("main.svr", source);
            let errors = check_project(project.path()).unwrap_err();
            assert_eq!(errors.items.len(), 1);
            assert_eq!(errors.items[0].code, code);
            assert_eq!(errors.items[0].span.end, source.len());
            assert!(errors.items[0].source_file.is_some());
        }
    }

    #[test]
    fn signature_continuations_stop_before_new_declarations() {
        for next in ["fn next() {}", "service mail {}", "}", "{"] {
            let lines = ["fn broken(", " value: Int,", next];
            let (signature, last) = collect_signature_lines(&lines, 0);
            assert_eq!(last, 1);
            assert_eq!(signature, "fn broken(\nvalue: Int,");
        }
        let lines = ["fn valid(", " callback: (Int, String) -> Bool,", ") {}"];
        let (signature, last) = collect_signature_lines(&lines, 0);
        assert_eq!(last, 2);
        assert!(service_contract::parse_operation(&signature).is_ok());
    }

    #[test]
    fn multiline_application_and_service_parameters_are_checked() {
        for kind in ["fn", "task", "page", "view", "service"] {
            let project = TestProject::new();
            let service = kind == "service";
            project.write_file(
                "sovra.toml",
                &format!(
                    "[project]\nname = \"sample\"\nentry = \"main.svr\"\n{}",
                    if service {
                        "[services]\nmail = \"external\""
                    } else {
                        ""
                    }
                ),
            );
            let declaration = if service { "fn" } else { kind };
            for (parameter, valid) in [("value: String", true), ("value", false)] {
                let body = format!("{declaration} example(\r\n // λ comment\r\n {parameter},\r\n context: Int,\r\n) {{}}\r\n");
                let source = if service {
                    format!("service mail {{\r\n{body}}}\r\n")
                } else {
                    body
                };
                project.write_file("main.svr", &source);
                let result = check_project(project.path());
                if valid {
                    let checked = result.unwrap();
                    if service {
                        assert_eq!(checked.service_operations[0].parameters.len(), 2);
                        assert!(source[checked.service_operations[0].span.start
                            ..checked.service_operations[0].span.end]
                            .contains("context: Int"));
                    }
                } else {
                    let errors = result.unwrap_err();
                    assert_eq!(errors.items.len(), 1, "{kind}: {errors:?}");
                    assert_eq!(errors.items[0].code, "E4097");
                    assert_eq!(errors.items[0].span.line, usize::from(service));
                }
            }
        }
    }

    #[test]
    fn application_declaration_parameters_require_annotations() {
        for kind in ["fn", "task", "page", "view"] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"sample\"\nentry = \"main.svr\"",
            );
            project.write_file("main.svr", &format!("{kind} example(value) {{}}\n"));
            let errors = check_project(project.path()).unwrap_err();
            assert_eq!(errors.items.len(), 1, "{kind}: {errors:?}");
            assert_eq!(errors.items[0].code, "E4097");
            assert!(errors.items[0].message.contains("value"));
            project.write_file("main.svr", &format!("{kind} example(value: String) {{}}\n"));
            assert!(check_project(project.path()).is_ok(), "{kind}");
            project.write_file("main.svr", &format!("{kind} example(value:) {{}}\n"));
            let errors = check_project(project.path()).unwrap_err();
            assert_eq!(errors.items.len(), 1, "{kind}: {errors:?}");
            assert_eq!(errors.items[0].code, "E4098");
        }
    }

    #[test]
    fn service_parameters_require_explicit_annotations() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        project.write_file(
            "main.svr",
            "service mail {\nfn send(value, context: Int);\n}\n",
        );
        let errors = check_project(project.path()).unwrap_err();
        assert_eq!(errors.items.len(), 1);
        assert_eq!(errors.items[0].code, "E4097");
        assert!(errors.items[0].message.contains("value"));
        assert_eq!(errors.items[0].span.line, 1);
        assert!(errors.items[0].source_file.is_some());
    }

    #[test]
    fn service_operation_trailing_declarations_are_not_silently_lost() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\n    fn send() {} fn hidden() {}\n}\n";
        project.write_file("main.svr", source);
        let errors = check_project(project.path()).unwrap_err();
        assert_eq!(errors.items.len(), 1);
        assert_eq!(errors.items[0].code, "E4027");
        let span = errors.items[0].span;
        assert_eq!(
            &source[span.start..span.end],
            "    fn send() {} fn hidden() {}"
        );
        assert!(errors.items[0].source_file.is_some());
    }

    #[test]
    fn service_body_metadata_recognizes_braces_on_following_lines() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\nfn declaration();\nfn empty()\n// body follows a comment\n\n{}\nfn relay() -> Unit\n{ mail.empty(); }\nfn last()\n}\n";
        project.write_file("main.svr", source);
        let checked = check_project(project.path()).unwrap();
        assert_eq!(
            checked
                .service_operations
                .iter()
                .map(|operation| (operation.name.as_str(), operation.has_body))
                .collect::<Vec<_>>(),
            [
                ("declaration", false),
                ("empty", true),
                ("relay", true),
                ("last", false)
            ]
        );
        for operation in &checked.service_operations {
            assert!(source[operation.span.start..operation.span.end]
                .trim()
                .starts_with("fn "));
        }
        let report = application::check_service_calls(&checked);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let functions = report.files[0].functions.as_ref().unwrap();
        assert_eq!(
            functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            ["mail.empty", "mail.relay"]
        );
    }

    #[test]
    fn project_result_retains_service_operation_metadata() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\n fn send(value: Result<Text, Error>, context: Int) -> Receipt;\n fn ping() {}\n}\n";
        project.write_file("main.svr", source);
        let checked = check_project(project.path()).unwrap();
        assert_eq!(checked.service_operations.len(), 2);
        let operation = &checked.service_operations[0];
        assert_eq!(operation.service, "mail");
        assert_eq!(operation.name, "send");
        assert_eq!(operation.return_annotation.as_deref(), Some("Receipt"));
        assert_eq!(
            operation.parameters[0].annotation.as_deref(),
            Some("Result<Text, Error>")
        );
        assert_eq!(operation.parameters[1].name, "context");
        assert_eq!(operation.parameters[1].annotation.as_deref(), Some("Int"));
        assert!(!operation.has_body);
        assert_eq!(operation.source_file, project.path().join("main.svr"));
        assert_eq!(
            &source[operation.span.start..operation.span.end],
            " fn send(value: Result<Text, Error>, context: Int) -> Receipt;"
        );
        assert!(checked.service_operations[1].has_body);
        assert_eq!(checked.service_operations[1].return_annotation, None);
    }

    #[test]
    fn malformed_service_operation_retains_source_location() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"",
        );
        let source = "service mail {\n    fn send\n}\n";
        project.write_file("main.svr", source);
        let errors = check_project(project.path()).unwrap_err();
        assert_eq!(errors.items.len(), 1);
        assert_eq!(errors.items[0].code, "E4027");
        let span = errors.items[0].span;
        assert_eq!(&source[span.start..span.end], "    fn send");
        assert!(errors.items[0].source_file.is_some());
    }

    #[test]
    fn invalid_service_names_are_diagnosed_without_indexing() {
        for declaration in [
            "service",
            "service {}",
            "service bad-name {}",
            "service 123 {}",
        ] {
            let mut index = ProjectSourceIndex::default();
            let mut diagnostics = Diagnostics::new();
            scan_source_file(
                declaration,
                Path::new("main.svr"),
                "main",
                false,
                &mut index,
                &mut diagnostics,
            );
            assert_eq!(diagnostics.items.len(), 1);
            assert_eq!(diagnostics.items[0].code, "E4026");
            assert!(index.declared_services.is_empty());
        }
    }

    #[test]
    fn task_indexing_allows_spacing_before_parameters() {
        for spacing in ["", " ", "\t", " \t "] {
            let source = format!("task worker{spacing}() {{}}\n");
            let mut index = ProjectSourceIndex::default();
            let mut diagnostics = Diagnostics::new();
            scan_source_file(
                &source,
                Path::new("jobs.svr"),
                "jobs",
                false,
                &mut index,
                &mut diagnostics,
            );
            assert!(index.callable_symbols.contains("jobs.worker"));
            assert!(diagnostics.items.is_empty());
        }
    }

    #[test]
    fn service_header_rejects_unscanned_inline_operations() {
        for (suffix, valid) in [
            ("{}", true),
            ("{ \t }", true),
            ("unexpected", false),
            ("{ fn send() }", false),
        ] {
            let source = format!("service mail {suffix}\n");
            let mut index = ProjectSourceIndex::default();
            let mut diagnostics = Diagnostics::new();
            scan_source_file(
                &source,
                Path::new("main.svr"),
                "main",
                false,
                &mut index,
                &mut diagnostics,
            );
            if valid {
                assert!(diagnostics.items.is_empty());
            } else {
                assert_eq!(diagnostics.items.len(), 1);
                assert_eq!(diagnostics.items[0].code, "E4026");
            }
        }
    }

    #[test]
    fn service_contents_do_not_leak_into_application_indexes() {
        for header in ["service mail {", "service mail\n{"] {
            let source = format!("{header}\nfn send() {{\ntask hidden() {{}}\nmodel Hidden {{}}\nservice nested {{}}\nauth hidden {{}}\npage hidden {{}}\nview hidden {{}}\nroute GET \"/hidden\" -> hidden\nservices: [nested]\ndata: [Hidden]\nallow invalid\n}}\n}}\nfn handler() {{}}\nmodel Visible {{}}\nroute GET \"/visible\" -> handler\n");
            let mut index = ProjectSourceIndex::default();
            let mut diagnostics = Diagnostics::new();
            scan_source_file(
                &source,
                Path::new("main.svr"),
                "main",
                true,
                &mut index,
                &mut diagnostics,
            );
            assert!(diagnostics.items.is_empty(), "{diagnostics:?}");
            assert_eq!(index.declared_services.len(), 1);
            assert_eq!(index.declared_services[0].value, "mail");
            assert_eq!(
                index.callable_symbols,
                BTreeSet::from(["handler".to_owned(), "main.handler".to_owned()])
            );
            assert_eq!(
                index.model_symbols,
                BTreeSet::from(["Visible".to_owned(), "main.Visible".to_owned()])
            );
            assert!(index.page_symbols.is_empty());
            assert!(index.auth_symbols.is_empty());
            assert!(index.auth_policies.is_empty());
            assert!(index.app_services.is_empty());
            assert!(index.data_models.is_empty());
            assert_eq!(index.routes.len(), 1);
            assert_eq!(index.routes[0].value.target, "handler");
        }
    }

    #[test]
    fn declaration_names_accept_ascii_whitespace_before_delimiters() {
        for whitespace in [" ", "\t", " \t ", "\u{000c}"] {
            for (keyword, suffix) in [("service", "{"), ("fn", "()"), ("model", "{")] {
                let line = format!("{keyword}{whitespace}sample{whitespace}{suffix}");
                assert_eq!(
                    parse_prefixed_identifier(&line, keyword).as_deref(),
                    Some("sample")
                );
            }
        }
        assert_eq!(
            parse_prefixed_identifier("service_extra {}", "service"),
            None
        );
        assert_eq!(parse_prefixed_identifier("fn invalid-name()", "fn"), None);
    }

    #[test]
    fn tabbed_service_operations_still_receive_duplicate_checks() {
        let source = "service\tmail\t{\n\tfn\tsend\t()\n\tfn send ()\n}\nfn\thandler\t() {}\n";
        let mut index = ProjectSourceIndex::default();
        let mut diagnostics = Diagnostics::new();
        scan_source_file(
            source,
            Path::new("main.svr"),
            "main",
            false,
            &mut index,
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 1);
        assert_eq!(diagnostics.items[0].code, "E4025");
        assert_eq!(diagnostics.items[0].span.line, 2);
        assert_eq!(index.declared_services[0].value, "mail");
        assert!(!index.callable_symbols.contains("send"));
        assert!(index.callable_symbols.contains("handler"));
    }

    #[test]
    fn service_opening_brace_can_follow_comments_and_blank_lines() {
        for separator in ["\n", "\r\n\r\n// contract follows\r\n"] {
            let source = format!(
                "service mail{separator}{{ // open\nfn send()\nfn send()\n}}\nfn handler() {{}}\n"
            );
            let mut index = ProjectSourceIndex::default();
            let mut diagnostics = Diagnostics::new();
            scan_source_file(
                &source,
                Path::new("main.svr"),
                "main",
                false,
                &mut index,
                &mut diagnostics,
            );
            assert_eq!(diagnostics.items.len(), 1);
            assert_eq!(diagnostics.items[0].code, "E4025");
            assert!(!index.callable_symbols.contains("send"));
            assert!(index.callable_symbols.contains("handler"));
        }
    }

    #[test]
    fn pending_service_does_not_capture_an_unrelated_block() {
        for declaration in ["service mail", "service mail unexpected"] {
            let source = format!("{declaration}\nfn handler() {{}}\n{{\nfn send()\n}}\n");
            let mut index = ProjectSourceIndex::default();
            let mut diagnostics = Diagnostics::new();
            scan_source_file(
                &source,
                Path::new("main.svr"),
                "main",
                false,
                &mut index,
                &mut diagnostics,
            );
            assert!(index.callable_symbols.contains("handler"));
            assert!(index.callable_symbols.contains("send"));
            assert_eq!(diagnostics.items.len(), 1);
            assert_eq!(diagnostics.items[0].code, "E4026");
        }
    }

    #[test]
    fn service_operations_are_scoped_and_not_route_targets() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"\nsms = \"external\"");
        project.write_file("main.svr", "service mail {\n    fn send(value: Text) -> Text\n    fn send(other: Int) -> Text\n}\nservice sms {\n    fn send(value: Text) -> Text\n}\nroute POST \"/send\" -> send\n");
        let errors = check_project(project.path()).unwrap_err();
        let duplicates: Vec<_> = errors
            .items
            .iter()
            .filter(|error| error.code == "E4025")
            .collect();
        assert_eq!(duplicates.len(), 1);
        assert!(duplicates[0].message.contains("mail"));
        assert_eq!(duplicates[0].span.line, 2);
        assert_eq!(
            duplicates[0].source_file.as_deref(),
            Some(project.path().join("main.svr").to_string_lossy().as_ref())
        );
        assert!(errors.items.iter().any(|error| error.code == "E4033"));
    }

    #[test]
    fn incomplete_service_blocks_report_the_declaration_location() {
        for body in [
            "service mail",
            "service mail\n// waiting\n",
            "service mail {\nfn send()\n",
            "service mail\n{\nfn send()\n",
        ] {
            let project = TestProject::new();
            project.write_file("sovra.toml", "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nmail = \"external\"");
            let source = format!("// header\r\n{body}");
            project.write_file("main.svr", &source);
            let errors = check_project(project.path()).unwrap_err();
            assert_eq!(errors.items.len(), 1, "{errors:?}");
            let error = &errors.items[0];
            assert_eq!(error.code, "E4026");
            assert_eq!(error.span.line, 1);
            assert!(source[error.span.start..error.span.end].starts_with("service mail"));
            assert_eq!(
                error.source_file.as_deref(),
                Some(project.path().join("main.svr").to_string_lossy().as_ref())
            );
        }
    }

    #[test]
    fn service_scanning_respects_strings_comments_and_following_functions() {
        let source = "service mail {\n fn send(value: Text) -> Text // }\n fn nested() {\n print(\"}\\\"{\")\n }\n fn send(value: Text) -> Text\n}\nfn handler() {}\nservice sms {\n fn send(value: Text) -> Text\n}\n";
        let mut index = ProjectSourceIndex::default();
        let mut diagnostics = Diagnostics::new();
        scan_source_file(
            source,
            Path::new("main.svr"),
            "main",
            false,
            &mut index,
            &mut diagnostics,
        );
        assert_eq!(diagnostics.items.len(), 1);
        assert_eq!(diagnostics.items[0].code, "E4025");
        assert_eq!(diagnostics.items[0].span.line, 5);
        assert!(index.callable_symbols.contains("handler"));
        assert!(index.callable_symbols.contains("main.handler"));
        assert!(!index.callable_symbols.contains("send"));
        assert!(!index.callable_symbols.contains("main.send"));
    }

    #[test]
    fn distinguishes_manifest_and_source_comment_markers() {
        for (line, marker, expected) in [
            (
                r#"value = "https://host/#part" # comment"#,
                "#",
                r#"value = "https://host/#part" "#,
            ),
            (
                r#"print("https://host/#part") // comment"#,
                "//",
                r#"print("https://host/#part") "#,
            ),
            (
                r#"print("escaped \" // quoted") // comment"#,
                "//",
                r#"print("escaped \" // quoted") "#,
            ),
            (
                r#"print("slash \\") // comment"#,
                "//",
                r#"print("slash \\") "#,
            ),
            (
                "services: [] # not a source comment",
                "//",
                "services: [] # not a source comment",
            ),
            (
                "name = \"sample\" // not a manifest comment",
                "#",
                "name = \"sample\" // not a manifest comment",
            ),
        ] {
            assert_eq!(strip_line_comment(line, marker), expected);
        }
        let manifest = Manifest::parse("[project]\nname = \"sample\" // invalid");
        assert!(manifest.items_contain("E4015"));
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"",
        );
        project.write_file("main.svr", "services: [] # invalid");
        let errors = check_project(project.path()).expect_err("hash is not a source comment");
        assert!(errors.items.iter().any(|e| e.code == "E4024"));
    }

    #[test]
    fn source_comments_do_not_change_wiring_targets() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\" # manifest comment\nentry = \"main.svr\"\n",
        );
        project.write_file("main.svr", "fn handler() {}\nroute GET \"/hash#fragment\" -> handler // trailing comment\n// route GET \"/ignored\" -> missing\n");
        let checked = check_project(project.path()).expect("source comments are ignored");
        assert_eq!(checked.routes.len(), 1);
        assert_eq!(checked.routes[0].path, "/hash#fragment");
    }

    #[test]
    fn accepts_complete_application_lists() {
        for suffix in [
            "[]",
            "[valid]",
            "[valid,]",
            "[valid, other]",
            "[valid]; // comment",
            "[valid] // comment",
        ] {
            let project = TestProject::new();
            project.write_file("sovra.toml", "[project]\nname = \"sample\"\nentry = \"main.svr\"\n[services]\nvalid = \"external\"\nother = \"external\"");
            project.write_file("main.svr", &format!("service valid {{}}\nservice other {{}}\nmodel valid {{}}\nmodel other {{}}\nservices: {suffix}\ndata: {suffix}\nservices_extra: [ignored]\ndatabase: [ignored]"));
            let checked = check_project(project.path()).expect(suffix);
            let expected = if suffix == "[]" {
                vec![]
            } else if suffix.contains("other") {
                vec!["other", "valid"]
            } else {
                vec!["valid"]
            };
            assert_eq!(checked.app_services, expected);
            assert_eq!(checked.data_models, expected);
        }
    }

    #[test]
    fn rejects_malformed_application_lists() {
        for (key, code) in [("services", "E4024"), ("data", "E4062")] {
            for suffix in [
                ": [valid, bad-name]",
                ": [\"quoted\"]",
                ": [valid,,other]",
                ": [,]",
                ": [valid",
                ": valid]",
                ": [valid] junk",
                " [valid]",
                ": [valid other]",
                ": [[valid]]",
                ": [valid],",
                "=[valid]",
                "[valid]",
            ] {
                let project = TestProject::new();
                project.write_file(
                    "sovra.toml",
                    "[project]\nname = \"sample\"\nentry = \"main.svr\"",
                );
                let declaration = format!("  {key}{suffix}");
                let source = format!("// é\r\n{declaration}\r\n");
                project.write_file("main.svr", &source);
                let errors = check_project(project.path()).expect_err("malformed list must fail");
                let error = errors
                    .items
                    .iter()
                    .find(|error| error.code == code)
                    .unwrap_or_else(|| panic!("{declaration}: {errors:?}"));
                assert_eq!(
                    error.source_file.as_deref(),
                    project.path().join("main.svr").to_str()
                );
                assert_eq!(&source[error.span.start..error.span.end], declaration);
                assert_eq!(error.span.line, 1);
            }
        }
    }

    #[test]
    fn manifest_value_errors_identify_assignments() {
        for (extra, code, spelling) in [
            ("name = \"\"", "E4001", "name = \"\""),
            ("name = \"123\"", "E4002", "name = \"123\""),
            ("entry = \"main.txt\"", "E4004", "entry = \"main.txt\""),
            (
                "entry = \"missing.svr\"",
                "E4005",
                "entry = \"missing.svr\"",
            ),
            (
                "entry = \"../outside.svr\"",
                "E4007",
                "entry = \"../outside.svr\"",
            ),
            (
                "[runtime]\ntarget = \"unknown\"",
                "E4003",
                "target = \"unknown\"",
            ),
            (
                "[services]\nbad-name = \"external\"",
                "E4017",
                "bad-name = \"external\"",
            ),
            (
                "[services]\nmissing = \"external\"",
                "E4021",
                "missing = \"external\"",
            ),
        ] {
            let project = TestProject::new();
            let mut manifest = String::from("# é\r\n[project]\r\n");
            if !extra.starts_with("name") {
                manifest.push_str("name = \"sample\"\r\n");
            }
            if !extra.starts_with("entry") {
                manifest.push_str("entry = \"main.svr\"\r\n");
            }
            manifest.push_str(extra);
            project.write_file("sovra.toml", &manifest);
            project.write_file("main.svr", "fn main() {}");
            let errors = check_project(project.path()).expect_err("invalid manifest value");
            let error = errors.items.iter().find(|e| e.code == code).expect(code);
            assert_eq!(
                error.source_file.as_deref(),
                project.path().join("sovra.toml").to_str()
            );
            assert_eq!(
                &manifest[error.span.start..error.span.end],
                spelling,
                "{code}"
            );
        }
    }

    #[test]
    fn all_wiring_validation_families_retain_locations() {
        for (body, code) in [
            ("service email {}", "E4022"),
            ("services: [email]", "E4023"),
            ("route UNKNOWN \"/x\" -> missing", "E4030"),
            ("route GET \"x\" -> missing", "E4031"),
            ("route GET \"/x/\" -> missing", "E4034"),
            (
                "route GET \"/x\" -> missing\n  route GET \"/x\" -> missing",
                "E4032",
            ),
            ("page \"x\" -> missing", "E4040"),
            ("page \"/x/\" -> missing", "E4043"),
            ("page \"/x\" -> missing", "E4042"),
            ("page \"/x\" -> missing\n  page \"/x\" -> missing", "E4041"),
            ("auth: missing.session", "E4052"),
            ("data: [Missing]", "E4060"),
            ("data: [Missing]\n  data: [Missing]", "E4061"),
            ("task daily -> missing.job", "E4072"),
            (
                "task daily -> missing.job\n  task daily -> missing.job",
                "E4071",
            ),
            ("allow user to read on Missing", "E4081"),
            (
                "allow user to read on Missing\n  allow user to read on Missing",
                "E4082",
            ),
        ] {
            let project = TestProject::new();
            project.write_file(
                "sovra.toml",
                "[project]\nname = \"sample\"\nentry = \"main.svr\"\n",
            );
            let source = format!("// é\r\n{body}");
            project.write_file("main.svr", &source);
            let errors = check_project(project.path()).expect_err("invalid wiring");
            let error = errors.items.iter().find(|e| e.code == code).expect(code);
            assert_eq!(
                error.source_file.as_deref(),
                project.path().join("main.svr").to_str()
            );
            assert_eq!(
                &source[error.span.start..error.span.end],
                body.lines().last().unwrap(),
                "{code}"
            );
            assert_eq!(error.span.line, body.lines().count());
            assert_eq!(error.span.column, 0);
        }
    }

    #[test]
    fn missing_manifest_keys_do_not_invent_locations() {
        let project = TestProject::new();
        project.write_file("sovra.toml", "[project]\nentry = \"main.svr\"");
        project.write_file("main.svr", "fn main() {}");
        let errors = check_project(project.path()).expect_err("missing name");
        let error = errors.items.iter().find(|e| e.code == "E4001").unwrap();
        assert!(error.source_file.is_none());
    }

    #[test]
    fn wiring_errors_retain_declaration_locations() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n",
        );
        let source = "// é\r\nroute GET \"/z\" -> missing.z\r\nroute GET \"/a\" -> missing.a\r\n";
        project.write_file("main.svr", source);
        let errors = check_project(project.path()).expect_err("missing targets");
        for (error, spelling) in errors.items.iter().filter(|e| e.code == "E4033").zip([
            "route GET \"/a\" -> missing.a",
            "route GET \"/z\" -> missing.z",
        ]) {
            assert_eq!(
                error.source_file.as_deref(),
                project.path().join("main.svr").to_str()
            );
            assert_eq!(&source[error.span.start..error.span.end], spelling);
        }
        assert_eq!(errors.items.iter().filter(|e| e.code == "E4033").count(), 2);
    }

    #[test]
    fn scan_errors_retain_source_byte_ranges() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            "[project]\nname = \"sample\"\nentry = \"main.svr\"\n",
        );
        let source = "// é\r\n  route broken\r\n";
        project.write_file("main.svr", source);
        let errors = check_project(project.path()).expect_err("malformed route");
        let error = errors
            .items
            .iter()
            .find(|error| error.code == "E4034")
            .unwrap();
        assert_eq!(&source[error.span.start..error.span.end], "  route broken");
        assert_eq!(error.span.line, 1);
        assert_eq!(error.span.column, 0);
        assert_eq!(
            error.source_file.as_deref(),
            project.path().join("main.svr").to_str()
        );
    }

    #[test]
    fn manifest_parse_errors_retain_file_and_line_ranges() {
        for source in ["[unknown]", "# é\r\n  [unknown]\r\n"] {
            let project = TestProject::new();
            project.write_file("sovra.toml", source);
            let errors = check_project(project.path()).expect_err("unknown section");
            let error = errors
                .items
                .iter()
                .find(|error| error.code == "E4010")
                .unwrap();
            assert_eq!(
                error.source_file.as_deref(),
                project.path().join("sovra.toml").to_str()
            );
            assert_eq!(
                &source[error.span.start..error.span.end],
                if source.starts_with('#') {
                    "  [unknown]"
                } else {
                    "[unknown]"
                }
            );
            assert_eq!(error.span.line, usize::from(source.starts_with('#')));
            assert_eq!(error.span.column, 0);
        }
    }

    #[test]
    fn parses_supported_manifest_values() {
        let manifest = Manifest::parse(
            r#"
[project]
name = "fielddesk"
version = "1.0.0"
entry = "app/main.svr"

[runtime]
target = "web"

[services]
maps = "env:MAPS_API_KEY"
"#,
        );

        assert!(manifest.diagnostics.is_empty());
        assert_eq!(manifest.value("project", "name"), Some("fielddesk"));
        assert_eq!(manifest.value("runtime", "target"), Some("web"));
        assert_eq!(manifest.value("services", "maps"), Some("env:MAPS_API_KEY"));
    }

    #[test]
    fn preserves_comment_markers_inside_quoted_values() {
        let manifest = Manifest::parse(
            r#"
[project]
name = "demo"
entry = "app/main.svr"

[services]
callback = "https://example.test/hook#fragment" # real comment
"#,
        );

        assert!(manifest.diagnostics.is_empty());
        assert_eq!(
            manifest.value("services", "callback"),
            Some("https://example.test/hook#fragment")
        );
    }

    #[test]
    fn decodes_quoted_value_escapes() {
        let manifest = Manifest::parse(
            r#"
[project]
name = "demo"
entry = "app/main.svr"

[services]
label = "quote: \"ok\""
"#,
        );

        assert!(manifest.diagnostics.is_empty());
        assert_eq!(manifest.value("services", "label"), Some("quote: \"ok\""));
    }

    #[test]
    fn reports_unknown_manifest_keys() {
        let manifest = Manifest::parse(
            r#"
[project]
name = "demo"
mystery = "value"
"#,
        );

        assert_eq!(manifest.diagnostics.items[0].code, "E4016");
    }

    #[test]
    fn reports_duplicate_manifest_keys() {
        let manifest = Manifest::parse(
            r#"
[project]
name = "demo"
name = "other"
"#,
        );

        assert!(manifest.items_contain("E4014"));
    }

    #[test]
    fn rejects_project_entry_outside_root() {
        let mut diagnostics = Diagnostics::new();
        let path = resolve_project_path(Path::new("project"), "../outside.svr", &mut diagnostics);

        assert!(path.is_none());
        assert_eq!(diagnostics.items[0].code, "E4007");
    }

    #[test]
    fn validates_project_directory_manifest() {
        let project = TestProject::new();
        project.write_dir("app");
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"

[runtime]
target = "cli"
"#,
        );
        project.write_file("app/main.svr", "fn main() { std::println(\"ready\") }");

        let checked = check_project(project.path()).expect("project should check");

        assert_eq!(checked.name, "demo");
        assert_eq!(checked.runtime_target.as_deref(), Some("cli"));
        assert_eq!(checked.source_files.len(), 1);
    }

    #[test]
    fn validates_manifest_bound_services() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"

[services]
maps = "env:MAPS_API_KEY"
payments = "env:PAYMENTS_API_KEY"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    services: [maps, payments]
}

fn main() {
    Demo.run()
}
"#,
        );
        project.write_file(
            "app/services.svr",
            r#"
service maps {}
service payments {}
"#,
        );

        let checked = check_project(project.path()).expect("project should check");

        assert_eq!(checked.declared_services, ["maps", "payments"]);
        assert_eq!(checked.app_services, ["maps", "payments"]);
        assert!(checked.routes.is_empty());
        assert!(checked.pages.is_empty());
    }

    #[test]
    fn validates_app_routes_and_pages() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    route POST "/api/jobs" -> jobs.create_job
    page "/" -> pages.dashboard
}
"#,
        );
        project.write_file("app/jobs.svr", "fn create_job() {}");
        project.write_file("app/pages.svr", "page dashboard() {}");

        let checked = check_project(project.path()).expect("project should check");

        assert_eq!(checked.routes.len(), 1);
        assert_eq!(checked.routes[0].target, "jobs.create_job");
        assert_eq!(checked.pages.len(), 1);
        assert_eq!(checked.pages[0].target, "pages.dashboard");
    }

    #[test]
    fn validates_auth_data_models_and_scheduled_tasks() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    auth: auth.session
    data: [Customer, Job]
    task every 5 minutes -> jobs.refresh_open_jobs
}
"#,
        );
        project.write_file("app/auth.svr", "auth session {}");
        project.write_file("app/models.svr", "model Customer {}\nmodel Job {}");
        project.write_file("app/jobs.svr", "task refresh_open_jobs() {}");

        let checked = check_project(project.path()).expect("project should check");

        assert_eq!(checked.auth_target.as_deref(), Some("auth.session"));
        assert_eq!(checked.data_models, ["Customer", "Job"]);
        assert_eq!(checked.scheduled_tasks.len(), 1);
        assert_eq!(checked.scheduled_tasks[0].target, "jobs.refresh_open_jobs");
    }

    #[test]
    fn validates_auth_policy_models() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file("app/main.svr", "fn main() {}");
        project.write_file(
            "app/auth.svr",
            r#"
auth session {
    allow manager to [read, write] on [Customer, Job]
    allow technician to update.status on Job where Job.assigned_to.id == user.id
}
"#,
        );
        project.write_file("app/models.svr", "model Customer {}\nmodel Job {}");

        let checked = check_project(project.path()).expect("project should check");

        assert_eq!(checked.auth_policies.len(), 2);
        assert_eq!(checked.auth_policies[0].role, "manager");
        assert_eq!(checked.auth_policies[0].models, ["Customer", "Job"]);
        assert_eq!(checked.auth_policies[1].actions, ["update.status"]);
    }

    #[test]
    fn reports_auth_policy_model_without_declaration() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file("app/main.svr", "fn main() {}");
        project.write_file(
            "app/auth.svr",
            r#"
auth session {
    allow manager to read on Invoice
}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4081"));
    }

    #[test]
    fn reports_malformed_and_duplicate_auth_policies() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file("app/main.svr", "fn main() {}");
        project.write_file(
            "app/auth.svr",
            r#"
auth session {
    allow manager read on Job
    allow technician to read on Job
    allow technician to read on Job
}
"#,
        );
        project.write_file("app/models.svr", "model Job {}");

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4080"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4082"));
    }

    #[test]
    fn reports_missing_auth_data_model_and_scheduled_task_targets() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    auth: auth.session
    data: [Customer]
    task every 5 minutes -> jobs.refresh_open_jobs
}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4052"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4060"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4072"));
    }

    #[test]
    fn reports_malformed_auth_and_scheduled_task_bindings() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    auth auth.session
    task -> jobs.refresh_open_jobs
}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4050"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4070"));
    }

    #[test]
    fn reports_missing_route_target() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    route POST "/api/jobs" -> jobs.create_job
}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4033"));
    }

    #[test]
    fn reports_malformed_route_and_page_declarations() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    route POST "/api/jobs" jobs.create_job
    page "/" pages.dashboard
}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4034"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4043"));
    }

    #[test]
    fn reports_invalid_route_and_page_paths() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    route GET "/api//jobs" -> jobs.list_jobs
    page "/jobs/:bad-id" -> pages.job_detail
}
"#,
        );
        project.write_file("app/jobs.svr", "fn list_jobs() {}");
        project.write_file("app/pages.svr", "page job_detail() {}");

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4034"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4043"));
    }

    #[test]
    fn reports_duplicate_route_and_page_paths() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    route GET "/api/jobs" -> jobs.list_jobs
    route GET "/api/jobs" -> jobs.list_jobs
    page "/" -> pages.dashboard
    page "/" -> pages.dashboard
}
"#,
        );
        project.write_file("app/jobs.svr", "fn list_jobs() {}");
        project.write_file("app/pages.svr", "page dashboard() {}");

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4032"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E4041"));
    }

    #[test]
    fn reports_missing_page_target() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    page "/" -> pages.dashboard
}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4042"));
    }

    #[test]
    fn reports_manifest_service_without_source_declaration() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"

[services]
maps = "env:MAPS_API_KEY"
"#,
        );
        project.write_file("app/main.svr", "fn main() {}");

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4021"));
    }

    #[test]
    fn reports_source_service_without_manifest_binding() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file("app/main.svr", "fn main() {}");
        project.write_file("app/services.svr", "service maps {}");

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4022"));
    }

    #[test]
    fn reports_app_service_without_complete_wiring() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );
        project.write_file(
            "app/main.svr",
            r#"
app Demo {
    services: [maps]
}

fn main() {}
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4023"));
    }

    #[test]
    fn reports_missing_project_entry_file() {
        let project = TestProject::new();
        project.write_file(
            "sovra.toml",
            r#"
[project]
name = "demo"
entry = "app/main.svr"
"#,
        );

        let diagnostics = check_project(project.path()).expect_err("project should fail");

        assert!(diagnostics.items.iter().any(|item| item.code == "E4005"));
    }

    impl Manifest {
        fn items_contain(&self, code: &str) -> bool {
            self.diagnostics.items.iter().any(|item| item.code == code)
        }
    }

    struct TestProject {
        root: PathBuf,
    }

    impl TestProject {
        fn new() -> Self {
            static NEXT_PROJECT: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);
            let sequence = NEXT_PROJECT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "sovra-project-test-{}-{suffix}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&root).expect("test project directory should be created");
            Self { root }
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write_dir(&self, path: &str) {
            fs::create_dir_all(self.root.join(path)).expect("test directory should be created");
        }

        fn write_file(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("test parent directory should be created");
            }
            fs::write(path, contents).expect("test file should be written");
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
