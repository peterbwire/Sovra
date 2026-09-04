//! Project-level validation for `svr check`.

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
    let parsed = Manifest::parse(&manifest);
    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics);
    }

    let mut diagnostics = Diagnostics::new();
    let name = require_manifest_value(&parsed, "project", "name", &mut diagnostics);
    let entry = require_manifest_value(&parsed, "project", "entry", &mut diagnostics);
    let runtime_target = parsed.value("runtime", "target").map(str::to_owned);

    if let Some(name) = name.as_deref() {
        validate_project_name(name, &mut diagnostics);
    }
    if let Some(target) = runtime_target.as_deref() {
        validate_runtime_target(target, &mut diagnostics);
    }

    let entry_path = entry
        .as_deref()
        .and_then(|entry| resolve_project_path(root, entry, &mut diagnostics))
        .unwrap_or_else(|| root.join(""));
    if entry.is_some() {
        validate_entry_path(&entry_path, &mut diagnostics);
    }

    let source_files = collect_source_files(root, &mut diagnostics);
    if source_files.is_empty() {
        push_error(
            &mut diagnostics,
            "E4008",
            "project does not contain any .svr source files",
        );
    }
    let source_index = scan_project_sources(&source_files, &entry_path, &mut diagnostics);
    validate_services(&parsed, &source_index, &mut diagnostics);

    if diagnostics.is_empty() {
        Ok(ProjectCheck {
            manifest_path,
            name: name.expect("name is present when diagnostics are empty"),
            entry_path,
            runtime_target,
            source_files,
            declared_services: source_index.declared_services,
            app_services: source_index.app_services,
            routes: source_index.routes,
            pages: source_index.pages,
            auth_target: source_index.auth_target,
            auth_policies: source_index.auth_policies,
            data_models: source_index.data_models,
            scheduled_tasks: source_index.scheduled_tasks,
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
            push_error(
                diagnostics,
                "E4001",
                format!("project manifest requires `{section}.{key}`"),
            );
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
    collect_source_files_inner(root, &mut files, diagnostics);
    files.sort();
    files
}

fn collect_source_files_inner(
    root: &Path,
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
            collect_source_files_inner(&path, files, diagnostics);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("svr") {
            files.push(path);
        }
    }
}

#[derive(Debug, Default)]
struct ProjectSourceIndex {
    declared_services: Vec<String>,
    app_services: Vec<String>,
    callable_symbols: BTreeSet<String>,
    page_symbols: BTreeSet<String>,
    auth_symbols: BTreeSet<String>,
    model_symbols: BTreeSet<String>,
    auth_policies: Vec<AuthPolicy>,
    routes: Vec<AppRoute>,
    pages: Vec<AppPage>,
    auth_target: Option<String>,
    data_models: Vec<String>,
    scheduled_tasks: Vec<AppTask>,
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
        scan_source_file(
            &source,
            module_name,
            source_file == entry_path,
            &mut index,
            diagnostics,
        );
    }
    index.declared_services.sort();
    index.app_services.sort();
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
    index.data_models.sort();
    index
}

fn scan_source_file(
    source: &str,
    module_name: &str,
    is_entry: bool,
    index: &mut ProjectSourceIndex,
    diagnostics: &mut Diagnostics,
) {
    let mut seen_services = BTreeSet::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = strip_comment(line).trim();
        if let Some(name) = parse_prefixed_identifier(trimmed, "service") {
            if !seen_services.insert(name.clone()) || index.declared_services.contains(&name) {
                push_manifest_error(
                    diagnostics,
                    line_index,
                    "E4020",
                    format!("duplicate service declaration `{name}`"),
                );
            }
            index.declared_services.push(name);
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "fn") {
            index.callable_symbols.insert(name.clone());
            index
                .callable_symbols
                .insert(format!("{module_name}.{name}"));
        }
        if let Some(name) = parse_prefixed_identifier(trimmed, "task") {
            if trimmed
                .strip_prefix("task")
                .is_some_and(|rest| rest.trim_start().starts_with(&format!("{name}(")))
            {
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
                Some(policy) => index.auth_policies.push(policy),
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
        if is_entry {
            index
                .app_services
                .extend(parse_named_list(trimmed, "services"));
            index.data_models.extend(parse_named_list(trimmed, "data"));
            if starts_named_key(trimmed, "auth") || starts_keyword(trimmed, "auth") {
                match parse_named_target(trimmed, "auth") {
                    Some(target) => {
                        if index.auth_target.replace(target).is_some() {
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
                    Some(route) => index.routes.push(route),
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
                    Some(page) => index.pages.push(page),
                    None => push_manifest_error(
                        diagnostics,
                        line_index,
                        "E4043",
                        "malformed page route declaration; expected `page \"/path\" -> target`",
                    ),
                }
            }
            if starts_keyword(trimmed, "task") && trimmed.contains("->") {
                match parse_app_task(trimmed) {
                    Some(task) => index.scheduled_tasks.push(task),
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
    let name = rest.trim_start().split([' ', '{', '(']).next()?;
    if is_identifier(name) {
        Some(name.to_owned())
    } else {
        None
    }
}

fn parse_named_list(line: &str, key: &str) -> Vec<String> {
    let Some(rest) = line.strip_prefix(key) else {
        return Vec::new();
    };
    let rest = rest.trim_start();
    let Some(list) = rest.strip_prefix(':').map(str::trim_start) else {
        return Vec::new();
    };
    let Some(list) = list.strip_prefix('[') else {
        return Vec::new();
    };
    let Some((items, _)) = list.split_once(']') else {
        return Vec::new();
    };
    items
        .split(',')
        .map(str::trim)
        .filter(|item| is_identifier(item))
        .map(str::to_owned)
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
            .any(|declared| declared == service)
        {
            push_error(
                diagnostics,
                "E4021",
                format!("service binding `{service}` has no matching source declaration"),
            );
        }
    }
    for service in &source_index.declared_services {
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
    }
    for service in &source_index.app_services {
        if !manifest_services
            .iter()
            .any(|bound_service| *bound_service == service)
            || !source_index
                .declared_services
                .iter()
                .any(|declared| declared == service)
        {
            push_error(
                diagnostics,
                "E4023",
                format!("app service `{service}` must be declared and bound in the manifest"),
            );
        }
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
    for route in &source_index.routes {
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
    }
}

fn validate_pages(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for page in &source_index.pages {
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
    }
}

fn validate_auth(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    if let Some(target) = &source_index.auth_target {
        if !source_index.auth_symbols.contains(target) {
            push_error(
                diagnostics,
                "E4052",
                format!("auth target `{target}` was not found"),
            );
        }
    }
}

fn validate_data_models(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for model in &source_index.data_models {
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
    }
}

fn validate_scheduled_tasks(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for task in &source_index.scheduled_tasks {
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
    }
}

fn validate_auth_policies(source_index: &ProjectSourceIndex, diagnostics: &mut Diagnostics) {
    let mut seen = BTreeSet::new();
    for policy in &source_index.auth_policies {
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
    entries: Vec<ManifestEntry>,
    diagnostics: Diagnostics,
}

impl Manifest {
    fn parse(source: &str) -> Self {
        let mut manifest = Self::default();
        let mut current_section: Option<String> = None;
        let mut seen_sections = BTreeSet::new();
        let mut seen_keys = BTreeSet::new();

        for (line_index, line) in source.lines().enumerate() {
            let line_without_comment = strip_comment(line);
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
                if !SUPPORTED_SECTIONS.contains(&section) {
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

fn strip_comment(line: &str) -> &str {
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
        if character == '#' && !quoted {
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

fn push_error(diagnostics: &mut Diagnostics, code: &'static str, message: impl Into<String>) {
    diagnostics.push(Diagnostic {
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
    fn parses_supported_manifest_values() {
        let manifest = Manifest::parse(
            r#"
[project]
name = "fielddesk"
version = "0.1.0"
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
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "sovra-project-test-{}-{suffix}",
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
