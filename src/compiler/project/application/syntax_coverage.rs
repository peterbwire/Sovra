//! Opt-in syntax and scoped semantics from the service checker source snapshots.
use super::{syntax, ServiceCheck};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Maximum project files admitted to optional syntax coverage.
pub const MAX_SYNTAX_FILES: usize = 128;
/// Maximum cumulative source bytes admitted to optional syntax coverage.
pub const MAX_SYNTAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
/// Maximum cumulative non-EOF tokens admitted to retained project syntax.
pub const MAX_SYNTAX_TOKENS: usize = 262_144;

/// One file's syntax coverage; errors never imply successful parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSyntaxCoverage {
    /// Owning project source file.
    pub source_file: PathBuf,
    /// Retained supported syntax or the reason the whole file is unavailable.
    pub syntax: Result<syntax::ApplicationSyntax, String>,
}

/// Existing service checking plus separately bounded, optional syntax coverage.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithSyntax {
    /// Existing checker result; this remains partial application validation.
    pub check: ServiceCheck,
    /// Optional syntax coverage, never a semantic success assertion.
    /// Cumulative quota failure makes the entire projection unavailable.
    pub syntax: Result<Vec<FileSyntaxCoverage>, String>,
}

/// Check services and retain optional syntax using the same source snapshots.
/// No second disk read is used for syntax; existing default APIs stay unchanged.
/// This does not turn default project scanning into full M12 validation.
pub fn check_service_calls_with_syntax(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithSyntax {
    let inspected = super::imports::inspect_snapshot(project);
    let syntax = coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithSyntax { check, syntax }
}

/// One file's combined entry/declaration coverage, separate from checker success.
#[derive(Debug, Clone, PartialEq)]
pub struct FileCombinedSyntaxCoverage {
    /// Owning source file from the checker snapshot.
    pub source_file: PathBuf,
    /// Combined syntax or the reason this file is unavailable.
    pub syntax: Result<syntax::CombinedApplicationSyntax, String>,
}

/// Existing service checking plus separately bounded combined syntax coverage.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithCombinedSyntax {
    /// Unchanged partial service-check result and diagnostics.
    pub check: ServiceCheck,
    /// Combined syntax, never a semantic validity assertion.
    pub syntax: Result<Vec<FileCombinedSyntaxCoverage>, String>,
}

/// Add combined coverage from the same loaded sources without changing diagnostics.
/// Entry coverage does not disable incomplete-check errors or authorize bindings.
pub fn check_service_calls_with_combined_syntax(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithCombinedSyntax {
    let inspected = super::imports::inspect_snapshot(project);
    let syntax = combined_coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithCombinedSyntax { check, syntax }
}

/// One source file's auth-aware syntax, independent of semantic success.
#[derive(Debug, Clone, PartialEq)]
pub struct FileAuthSyntaxCoverage {
    /// Original owning source file.
    pub source_file: PathBuf,
    /// Auth-aware syntax or explicit per-file unavailability.
    pub syntax: Result<syntax::AuthCombinedSyntax, String>,
}

/// Unchanged service diagnostics plus optional auth-aware coverage.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithAuthSyntax {
    /// Existing partial semantic result, retaining incomplete-inspection errors.
    pub check: ServiceCheck,
    /// Separately bounded metadata, never permission or type validity.
    pub syntax: Result<Vec<FileAuthSyntaxCoverage>, String>,
}

/// Retain auth-aware coverage from the same source strings used by inspection.
/// Existing default and opt-in APIs are unchanged; E4096 is never suppressed.
pub fn check_service_calls_with_auth_syntax(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithAuthSyntax {
    let inspected = super::imports::inspect_snapshot(project);
    let syntax = auth_coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithAuthSyntax { check, syntax }
}

fn auth_coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileAuthSyntaxCoverage>, String> {
    Ok(
        project_coverage(sources, syntax::parse_with_auth, |parsed| {
            parsed.token_count
        })?
        .into_iter()
        .map(|(source_file, syntax)| FileAuthSyntaxCoverage {
            source_file,
            syntax,
        })
        .collect(),
    )
}

/// One file's integrated header coverage, independent of semantic success.
#[derive(Debug, Clone, PartialEq)]
pub struct FileStructuredSyntaxCoverage {
    /// Original owning source file.
    pub source_file: PathBuf,
    /// Structured partial syntax or explicit per-file failure.
    pub syntax: Result<syntax::StructuredApplicationSyntax, String>,
}

/// Existing partial semantic result plus separately bounded integrated metadata.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithStructuredSyntax {
    /// Existing checker diagnostics, including incomplete-inspection errors.
    pub check: ServiceCheck,
    /// Integrated metadata, never type/binding/authorization or rendering validity.
    pub syntax: Result<Vec<FileStructuredSyntaxCoverage>, String>,
}

/// Retain integrated partial headers from the same loaded sources as checking.
/// Metadata gaps/quota failures never suppress semantic diagnostics or E4096.
pub fn check_service_calls_with_structured_syntax(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithStructuredSyntax {
    let inspected = super::imports::inspect_snapshot(project);
    let syntax = structured_coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithStructuredSyntax { check, syntax }
}

/// One owning source snapshot's primitive model-field outcome.
#[derive(Debug, Clone)]
pub struct FileModelFieldCheck {
    /// Original owning project file.
    pub source_file: PathBuf,
    /// Scoped field result, including independent diagnostics, or unavailability.
    pub fields: Result<super::ModelPrimitiveFieldCheck, String>,
}

/// Existing partial service checks plus optional, separately bounded field checks.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithModelFields {
    /// Unchanged service result, including E4096.
    pub check: ServiceCheck,
    /// Same-snapshot per-file field outcomes; cumulative exhaustion returns Err.
    /// These local facts neither resolve imports nor authorize model operations.
    pub model_fields: Result<Vec<FileModelFieldCheck>, String>,
}

/// Inspect primitive model fields using the existing checker's loaded source strings.
/// No extra source read, diagnostic suppression or default API/CLI change occurs.
/// Imported contexts remain explicitly unavailable under the source checker rules.
pub fn check_service_calls_with_model_fields(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithModelFields {
    let inspected = super::imports::inspect_snapshot(project);
    let model_fields = model_field_coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithModelFields {
        check,
        model_fields,
    }
}

/// One source snapshot's scoped ordinary-function semantics.
#[derive(Debug, Clone)]
pub struct FileLocalFunctionCheck {
    /// Owning source path; diagnostic spans belong to this file.
    pub source_file: PathBuf,
    /// Structured local function check or explicit unavailable context/read failure.
    pub functions: Result<super::StructuredLocalFunctionCheck, String>,
}

/// Original service checking plus optional same-snapshot local function outcomes.
#[derive(Debug, Clone)]
pub struct ServiceCheckWithLocalFunctions {
    /// Unchanged partial service result, retaining E4096.
    pub check: ServiceCheck,
    /// Independent per-file semantics; cumulative exhaustion exposes no partial table.
    pub local_functions: Result<Vec<FileLocalFunctionCheck>, String>,
}

/// Check self-contained ordinary functions from the service checker's loaded sources.
/// Imports and contextual application names remain unavailable under the source API.
/// No cross-file function linking, default API/CLI changes or source rereads occur.
pub fn check_service_calls_with_local_functions(
    project: &super::super::ProjectCheck,
) -> ServiceCheckWithLocalFunctions {
    let inspected = super::imports::inspect_snapshot(project);
    let local_functions = local_function_coverage(&inspected.sources);
    let check = super::check_inspections(project, inspected.files, inspected.signatures);
    ServiceCheckWithLocalFunctions {
        check,
        local_functions,
    }
}

fn local_function_coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileLocalFunctionCheck>, String> {
    let mut functions = 0usize;
    let mut diagnostics = 0usize;
    let mut message_bytes = 0usize;
    let parsed = project_coverage_with_budget(
        sources,
        super::check_structured_local_functions,
        |checked| checked.syntax.token_count,
        |checked| {
            functions = functions
                .checked_add(checked.checked_functions)
                .filter(|count| *count <= syntax::MAX_CALLABLES)
                .ok_or("local functions cumulative function limit exceeded")?;
            diagnostics = diagnostics
                .checked_add(checked.diagnostics.items.len())
                .filter(|count| *count <= super::MAX_ENTRY_PATH_DIAGNOSTICS)
                .ok_or("local functions cumulative diagnostic count limit exceeded")?;
            message_bytes = message_bytes
                .checked_add(
                    checked
                        .diagnostics
                        .items
                        .iter()
                        .map(|d| d.message.len())
                        .sum::<usize>(),
                )
                .filter(|count| *count <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                .ok_or("local functions cumulative diagnostic byte limit exceeded")?;
            Ok(())
        },
    )?;
    Ok(parsed
        .into_iter()
        .map(|(source_file, functions)| FileLocalFunctionCheck {
            source_file,
            functions,
        })
        .collect())
}

fn model_field_coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileModelFieldCheck>, String> {
    let mut fields = 0usize;
    let mut diagnostics = 0usize;
    let mut message_bytes = 0usize;
    let parsed = project_coverage_with_budget(
        sources,
        super::check_model_primitive_fields,
        |checked| checked.token_count,
        |checked| {
            fields = fields
                .checked_add(
                    checked
                        .models
                        .iter()
                        .map(|model| model.fields.len())
                        .sum::<usize>(),
                )
                .filter(|count| *count <= super::MAX_MODEL_FIELDS)
                .ok_or("model fields cumulative field limit exceeded")?;
            diagnostics = diagnostics
                .checked_add(checked.diagnostics.items.len())
                .filter(|count| *count <= super::MAX_ENTRY_PATH_DIAGNOSTICS)
                .ok_or("model fields cumulative diagnostic count limit exceeded")?;
            message_bytes = message_bytes
                .checked_add(
                    checked
                        .diagnostics
                        .items
                        .iter()
                        .map(|d| d.message.len())
                        .sum::<usize>(),
                )
                .filter(|count| *count <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                .ok_or("model fields cumulative diagnostic byte limit exceeded")?;
            Ok(())
        },
    )?;
    Ok(parsed
        .into_iter()
        .map(|(source_file, fields)| FileModelFieldCheck {
            source_file,
            fields,
        })
        .collect())
}

fn structured_coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileStructuredSyntaxCoverage>, String> {
    Ok(
        project_coverage(sources, syntax::parse_structured, |parsed| {
            parsed.token_count
        })?
        .into_iter()
        .map(|(source_file, syntax)| FileStructuredSyntaxCoverage {
            source_file,
            syntax,
        })
        .collect(),
    )
}

fn coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileSyntaxCoverage>, String> {
    Ok(
        project_coverage(sources, syntax::parse_file, |parsed| parsed.token_count)?
            .into_iter()
            .map(|(source_file, syntax)| FileSyntaxCoverage {
                source_file,
                syntax,
            })
            .collect(),
    )
}

fn combined_coverage(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
) -> Result<Vec<FileCombinedSyntaxCoverage>, String> {
    Ok(
        project_coverage(sources, syntax::parse_combined, |parsed| parsed.token_count)?
            .into_iter()
            .map(|(source_file, syntax)| FileCombinedSyntaxCoverage {
                source_file,
                syntax,
            })
            .collect(),
    )
}

pub(super) type ParsedFiles<T> = Vec<(PathBuf, Result<T, String>)>;

fn project_coverage<T>(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
    parse: fn(&str) -> Result<T, String>,
    token_count: fn(&T) -> usize,
) -> Result<ParsedFiles<T>, String> {
    project_coverage_with_budget(sources, parse, token_count, |_| Ok(()))
}

pub(super) fn project_coverage_with_budget<T>(
    sources: &BTreeMap<PathBuf, Result<String, String>>,
    parse: fn(&str) -> Result<T, String>,
    token_count: fn(&T) -> usize,
    mut admit: impl FnMut(&T) -> Result<(), String>,
) -> Result<ParsedFiles<T>, String> {
    if sources.len() > MAX_SYNTAX_FILES {
        return Err("application syntax file limit exceeded".into());
    }
    let bytes = sources
        .values()
        .filter_map(|source| source.as_ref().ok())
        .try_fold(0usize, |used, source| used.checked_add(source.len()))
        .ok_or("application syntax byte accounting overflow")?;
    if bytes > MAX_SYNTAX_INPUT_BYTES {
        return Err("application syntax cumulative source limit exceeded".into());
    }
    let mut tokens = 0usize;
    let mut files = Vec::new();
    for (source_file, source) in sources {
        let parsed = source
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|source| parse(source));
        if let Ok(parsed) = &parsed {
            tokens = tokens
                .checked_add(token_count(parsed))
                .filter(|sum| *sum <= MAX_SYNTAX_TOKENS)
                .ok_or("application syntax cumulative token limit exceeded")?;
            admit(parsed)?;
        }
        files.push((source_file.clone(), parsed));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_projection_is_explicit_for_bad_files_and_cumulative_exhaustion() {
        let sources = BTreeMap::from([
            (PathBuf::from("good.svr"), Ok("fn main(){}".into())),
            (PathBuf::from("bad.svr"), Ok("app Unsupported{}".into())),
            (PathBuf::from("missing.svr"), Err("read failed".into())),
        ]);
        let files = coverage(&sources).unwrap();
        assert!(files
            .iter()
            .find(|file| file.source_file == std::path::Path::new("good.svr"))
            .unwrap()
            .syntax
            .is_ok());
        assert_eq!(files.iter().filter(|file| file.syntax.is_err()).count(), 2);
        let sources = (0..MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(String::new())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(coverage(&sources).unwrap().len(), MAX_SYNTAX_FILES);
        let mut over = sources.clone();
        over.insert(PathBuf::from("extra.svr"), Ok(String::new()));
        assert!(coverage(&over).unwrap_err().contains("file limit"));
        let mut bytes = BTreeMap::new();
        for n in 0..16 {
            bytes.insert(
                PathBuf::from(format!("{n}.svr")),
                Ok(" ".repeat(syntax::MAX_SOURCE_BYTES)),
            );
        }
        assert_eq!(coverage(&bytes).unwrap().len(), 16);
        bytes.insert(PathBuf::from("extra.svr"), Ok(" ".into()));
        assert!(coverage(&bytes).unwrap_err().contains("source limit"));
        let source = format!(
            "fn main(){{{}}}",
            "print(1);".repeat((syntax::MAX_CALLABLE_TOKENS - 6) / 5)
        );
        let mut token_sources = (0..4)
            .map(|n| (PathBuf::from(format!("tokens{n}.svr")), Ok(source.clone())))
            .collect::<BTreeMap<_, _>>();
        let parsed = coverage(&token_sources).unwrap();
        assert_eq!(
            parsed
                .iter()
                .map(|file| file.syntax.as_ref().unwrap().token_count)
                .sum::<usize>(),
            MAX_SYNTAX_TOKENS
        );
        token_sources.insert(PathBuf::from("extra.svr"), Ok("fn extra(){}".into()));
        assert!(coverage(&token_sources)
            .unwrap_err()
            .contains("token limit"));
    }
    #[test]
    fn combined_projection_counts_original_entry_tokens_and_preserves_file_failures() {
        let source = format!(
            "app Demo{{\n{}\n}}",
            "unknown ".repeat(syntax::MAX_CALLABLE_TOKENS - 4)
        );
        let mut sources = (0..4)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(source.clone())))
            .collect::<BTreeMap<_, _>>();
        let exact = combined_coverage(&sources).unwrap();
        assert_eq!(
            exact
                .iter()
                .map(|file| file.syntax.as_ref().unwrap().token_count)
                .sum::<usize>(),
            MAX_SYNTAX_TOKENS
        );
        assert!(exact
            .iter()
            .all(|file| !file.syntax.as_ref().unwrap().unparsed_ranges.is_empty()));
        sources.insert(PathBuf::from("extra.svr"), Ok("app Extra{}".into()));
        assert!(combined_coverage(&sources)
            .unwrap_err()
            .contains("token limit"));
        let mut files = (0..MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok("app Demo{}".into())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(combined_coverage(&files).unwrap().len(), MAX_SYNTAX_FILES);
        files.insert(PathBuf::from("extra.svr"), Err("read failed".into()));
        assert!(combined_coverage(&files)
            .unwrap_err()
            .contains("file limit"));
        let mut bytes = (0..16)
            .map(|n| {
                (
                    PathBuf::from(format!("{n}.svr")),
                    Ok(" ".repeat(syntax::MAX_SOURCE_BYTES)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(combined_coverage(&bytes).unwrap().len(), 16);
        bytes.insert(PathBuf::from("extra.svr"), Ok(" ".into()));
        assert!(combined_coverage(&bytes)
            .unwrap_err()
            .contains("source limit"));
        let mixed = BTreeMap::from([
            (PathBuf::from("good.svr"), Ok("app Demo{}".into())),
            (
                PathBuf::from("unsupported.svr"),
                Ok("model Unsupported{}".into()),
            ),
            (PathBuf::from("missing.svr"), Err("read failed".into())),
        ]);
        assert_eq!(
            combined_coverage(&mixed)
                .unwrap()
                .iter()
                .filter(|file| file.syntax.is_err())
                .count(),
            2
        );
    }
    #[test]
    fn auth_coverage_charges_original_guard_tokens_and_retains_unsupported_file_errors() {
        let source = format!(
            "auth session{{\nallow user to read on Item where {}\n}}",
            "value ".repeat(syntax::MAX_CALLABLE_TOKENS - 11)
        );
        let mut sources = (0..4)
            .map(|n| (PathBuf::from(format!("auth{n}.svr")), Ok(source.clone())))
            .collect::<BTreeMap<_, _>>();
        let exact = auth_coverage(&sources).unwrap();
        assert_eq!(
            exact
                .iter()
                .map(|file| file.syntax.as_ref().unwrap().token_count)
                .sum::<usize>(),
            MAX_SYNTAX_TOKENS
        );
        assert!(exact
            .iter()
            .all(|file| !file.syntax.as_ref().unwrap().unparsed_ranges.is_empty()));
        sources.insert(PathBuf::from("extra.svr"), Ok("auth extra{}".into()));
        assert!(auth_coverage(&sources).unwrap_err().contains("token limit"));
        let mut files = (0..MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(String::new())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(auth_coverage(&files).unwrap().len(), MAX_SYNTAX_FILES);
        files.insert(PathBuf::from("extra.svr"), Err("read failed".into()));
        assert!(auth_coverage(&files).unwrap_err().contains("file limit"));
        let mixed = BTreeMap::from([
            (PathBuf::from("good.svr"), Ok("auth session{}".into())),
            (PathBuf::from("bad.svr"), Ok("model Unsupported{}".into())),
            (PathBuf::from("missing.svr"), Err("read failed".into())),
        ]);
        assert_eq!(
            auth_coverage(&mixed)
                .unwrap()
                .iter()
                .filter(|file| file.syntax.is_err())
                .count(),
            2
        );
    }
    #[test]
    fn structured_projection_counts_opaque_body_tokens_without_hiding_file_failures() {
        let source = format!(
            "model Item{{{}}}",
            "unknown ".repeat(syntax::MAX_CALLABLE_TOKENS - 4)
        );
        let mut sources = (0..4)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(source.clone())))
            .collect::<BTreeMap<_, _>>();
        let exact = structured_coverage(&sources).unwrap();
        assert_eq!(
            exact
                .iter()
                .map(|file| file.syntax.as_ref().unwrap().token_count)
                .sum::<usize>(),
            MAX_SYNTAX_TOKENS
        );
        assert!(exact
            .iter()
            .all(|file| !file.syntax.as_ref().unwrap().unparsed_ranges.is_empty()));
        sources.insert(PathBuf::from("extra.svr"), Ok("view Card(){}".into()));
        assert!(structured_coverage(&sources)
            .unwrap_err()
            .contains("token limit"));
        let mut files = (0..MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(String::new())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(structured_coverage(&files).unwrap().len(), MAX_SYNTAX_FILES);
        files.insert(PathBuf::from("extra.svr"), Err("read failed".into()));
        assert!(structured_coverage(&files)
            .unwrap_err()
            .contains("file limit"));
        let mixed = BTreeMap::from([
            (PathBuf::from("good.svr"), Ok("model Item{}".into())),
            (PathBuf::from("bad.svr"), Ok("unknown Unsupported{}".into())),
            (PathBuf::from("missing.svr"), Err("read failed".into())),
        ]);
        assert_eq!(
            structured_coverage(&mixed)
                .unwrap()
                .iter()
                .filter(|file| file.syntax.is_err())
                .count(),
            2
        );
    }
}

#[cfg(test)]
mod model_field_tests {
    use super::*;
    fn sources(values: &[String]) -> BTreeMap<PathBuf, Result<String, String>> {
        values
            .iter()
            .enumerate()
            .map(|(n, source)| {
                (
                    PathBuf::from(format!("snapshot-{n}.svr")),
                    Ok(source.clone()),
                )
            })
            .collect()
    }
    #[test]
    fn field_projection_retains_owned_errors_and_reads_only_submitted_snapshots() {
        let mut files = sources(&[
            "model Item{id:Int}".into(),
            "model Bad{id:Unknown}".into(),
            "model Unsupported{id:Int=1}".into(),
        ]);
        files.insert(
            PathBuf::from("missing.svr"),
            Err("snapshot read failed".into()),
        );
        let checked = model_field_coverage(&files).unwrap();
        assert_eq!(checked.iter().filter(|f| f.fields.is_err()).count(), 2);
        let valid = checked
            .iter()
            .find(|f| f.source_file == std::path::Path::new("snapshot-0.svr"))
            .unwrap()
            .fields
            .as_ref()
            .unwrap();
        assert_eq!(valid.resolved.len(), 1);
        let invalid = checked
            .iter()
            .find(|f| f.source_file == std::path::Path::new("snapshot-1.svr"))
            .unwrap()
            .fields
            .as_ref()
            .unwrap();
        assert_eq!(invalid.diagnostics.items[0].code, "E3017");
        assert!(invalid.resolved.is_empty());
        assert_eq!(
            checked
                .iter()
                .find(|f| f.source_file == std::path::Path::new("missing.svr"))
                .unwrap()
                .fields
                .as_ref()
                .unwrap_err(),
            "snapshot read failed"
        );
    }
    #[test]
    fn cumulative_field_and_diagnostic_limits_are_exact() {
        let body = (0..super::super::MAX_MODEL_FIELDS / 2)
            .map(|n| format!("f{n}:Int\n"))
            .collect::<String>();
        let source = format!("model Item{{\n{body}}}");
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            model_field_coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.fields.as_ref().unwrap().resolved.len())
                .sum::<usize>(),
            super::super::MAX_MODEL_FIELDS
        );
        files.insert(PathBuf::from("extra.svr"), Ok("model Extra{x:Int}".into()));
        assert!(model_field_coverage(&files)
            .unwrap_err()
            .contains("cumulative field"));
        let body = (0..512)
            .map(|n| format!("f{n}:Unknown\n"))
            .collect::<String>();
        let source = format!("model Item{{\n{body}}}");
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            model_field_coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.fields.as_ref().unwrap().diagnostics.items.len())
                .sum::<usize>(),
            1024
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok("model Extra{x:Unknown}".into()),
        );
        assert!(model_field_coverage(&files)
            .unwrap_err()
            .contains("diagnostic count"));
        let long_name = "M".repeat(5000);
        let source = format!("model {long_name}{{\n{}}}", "id:Int\n".repeat(129));
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            model_field_coverage(&files)
                .unwrap()
                .iter()
                .flat_map(|f| &f.fields.as_ref().unwrap().diagnostics.items)
                .map(|d| d.message.len())
                .sum::<usize>(),
            1024 * 1024
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok(format!("model {long_name}{{id:Int,id:Int}}")),
        );
        assert!(model_field_coverage(&files)
            .unwrap_err()
            .contains("diagnostic byte"));
    }
    #[test]
    fn cumulative_source_file_and_original_opaque_token_limits_are_exact() {
        let mut files = (0..MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(String::new())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            model_field_coverage(&files).unwrap().len(),
            MAX_SYNTAX_FILES
        );
        files.insert(PathBuf::from("extra.svr"), Err("read failed".into()));
        assert!(model_field_coverage(&files)
            .unwrap_err()
            .contains("file limit"));
        let source = " ".repeat(syntax::MAX_SOURCE_BYTES);
        let mut files = sources(&vec![source; 16]);
        assert_eq!(model_field_coverage(&files).unwrap().len(), 16);
        files.insert(PathBuf::from("extra.svr"), Ok(" ".into()));
        assert!(model_field_coverage(&files)
            .unwrap_err()
            .contains("source limit"));
        // Enum bodies stay opaque, but every token contributes to the quota.
        let source = format!(
            "enum Opaque{{{}}}",
            "value ".repeat(syntax::MAX_CALLABLE_TOKENS - 4)
        );
        let mut files = sources(&vec![source; 4]);
        assert_eq!(
            model_field_coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.fields.as_ref().unwrap().token_count)
                .sum::<usize>(),
            MAX_SYNTAX_TOKENS
        );
        files.insert(PathBuf::from("extra.svr"), Ok("model Empty{}".into()));
        assert!(model_field_coverage(&files)
            .unwrap_err()
            .contains("token limit"));
    }
}

#[cfg(test)]
mod local_function_tests {
    use super::*;
    fn sources(values: &[String]) -> BTreeMap<PathBuf, Result<String, String>> {
        values
            .iter()
            .enumerate()
            .map(|(n, source)| {
                (
                    PathBuf::from(format!("snapshot-{n}.svr")),
                    Ok(source.clone()),
                )
            })
            .collect()
    }
    #[test]
    fn local_function_snapshots_preserve_owned_errors_and_unavailable_contexts() {
        let mut files = sources(&[
            "model Item{id:Money}\nfn good(){}".into(),
            "page Home(){}\nfn bad(){let value:Int=true;}".into(),
            "model Item{}\nfn unsupported(value:Item){}".into(),
            "fn caller(){good();}".into(),
        ]);
        files.insert(
            PathBuf::from("missing.svr"),
            Err("snapshot read failed".into()),
        );
        let outcomes = local_function_coverage(&files).unwrap();
        assert_eq!(outcomes.iter().filter(|f| f.functions.is_err()).count(), 2);
        let get = |index: usize| {
            let path = PathBuf::from(format!("snapshot-{index}.svr"));
            outcomes
                .iter()
                .find(|f| f.source_file == path)
                .unwrap()
                .functions
                .as_ref()
                .unwrap()
        };
        assert!(get(0).diagnostics.is_empty());
        assert_eq!(get(0).checked_functions, 1);
        assert!(!get(1).diagnostics.is_empty());
        assert!(get(3)
            .diagnostics
            .items
            .iter()
            .any(|d| d.message.contains("good"))); // Never link files implicitly.
        let exhausted = local_function_coverage(&sources(&[format!(
            "fn exhausted(){{{}}}",
            "missing();".repeat(512)
        )]))
        .unwrap();
        assert!(exhausted[0]
            .functions
            .as_ref()
            .unwrap()
            .diagnostics
            .items
            .iter()
            .any(|d| d.code == "E3019"));
    }
    #[test]
    fn cumulative_function_and_diagnostic_limits_are_exact() {
        let source = (0..1024)
            .map(|n| format!("fn f{n}(){{}}\n"))
            .collect::<String>();
        let mut files = sources(&vec![source; 4]);
        assert_eq!(
            local_function_coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.functions.as_ref().unwrap().checked_functions)
                .sum::<usize>(),
            syntax::MAX_CALLABLES
        );
        files.insert(PathBuf::from("extra.svr"), Ok("fn extra(){}".into()));
        assert!(local_function_coverage(&files)
            .unwrap_err()
            .contains("function limit"));
        let source = format!("fn main(){{{}}}", "missing();".repeat(128));
        let mut files = sources(&vec![source; 8]);
        assert_eq!(
            local_function_coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.functions.as_ref().unwrap().diagnostics.items.len())
                .sum::<usize>(),
            1024
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok("fn extra(){missing();}".into()),
        );
        assert!(local_function_coverage(&files)
            .unwrap_err()
            .contains("diagnostic count"));
        let name = "Long".repeat(1500);
        let source = format!("struct {name}{{{}}}", "id:Int,".repeat(129));
        let mut files = sources(&[source.clone(), source]);
        assert_eq!(
            local_function_coverage(&files)
                .unwrap()
                .iter()
                .flat_map(|f| &f.functions.as_ref().unwrap().diagnostics.items)
                .map(|d| d.message.len())
                .sum::<usize>(),
            1024 * 1024
        );
        files.insert(
            PathBuf::from("extra.svr"),
            Ok("fn extra(){missing();}".into()),
        );
        assert!(local_function_coverage(&files)
            .unwrap_err()
            .contains("diagnostic byte"));
    }
    #[test]
    fn file_source_and_original_opaque_token_limits_are_exact() {
        let mut files = (0..MAX_SYNTAX_FILES)
            .map(|n| (PathBuf::from(format!("{n}.svr")), Ok(String::new())))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            local_function_coverage(&files).unwrap().len(),
            MAX_SYNTAX_FILES
        );
        files.insert(PathBuf::from("extra.svr"), Err("read failed".into()));
        assert!(local_function_coverage(&files)
            .unwrap_err()
            .contains("file limit"));
        let mut files = sources(&vec![" ".repeat(syntax::MAX_SOURCE_BYTES); 16]);
        assert_eq!(local_function_coverage(&files).unwrap().len(), 16);
        files.insert(PathBuf::from("extra.svr"), Ok(" ".into()));
        assert!(local_function_coverage(&files)
            .unwrap_err()
            .contains("source limit"));
        let source = format!(
            "enum Opaque{{{}}}",
            "value ".repeat(syntax::MAX_CALLABLE_TOKENS - 4)
        );
        let mut files = sources(&vec![source; 4]);
        assert_eq!(
            local_function_coverage(&files)
                .unwrap()
                .iter()
                .map(|f| f.functions.as_ref().unwrap().syntax.token_count)
                .sum::<usize>(),
            MAX_SYNTAX_TOKENS
        );
        files.insert(PathBuf::from("extra.svr"), Ok("model Empty{}".into()));
        assert!(local_function_coverage(&files)
            .unwrap_err()
            .contains("token limit"));
    }
}
