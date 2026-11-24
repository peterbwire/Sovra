//! Local scheduled-declaration bindings; no scheduling or task execution.
use super::syntax::{self, AppMemberKind, CallableKind, CombinedApplicationSyntax};
use crate::compiler::{diagnostics::Diagnostics, diagnostics::Span, lexer::Lexer};
use std::collections::{BTreeMap, BTreeSet};

/// A unique schedule binding resolved to a same-source callable declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalScheduleTarget {
    /// Original scheduled declaration range.
    pub schedule_span: Span,
    /// Exact target identifier range.
    pub target_span: Span,
    /// Index in the retained callable table.
    pub callable_index: usize,
    /// Original callable declaration envelope.
    pub declaration_span: Span,
    /// Ordinary function or application task declaration, not runtime permission.
    pub kind: CallableKind,
}

/// Schedule-name validation, separate from task bodies and runtime scheduling.
#[derive(Debug, Clone)]
pub struct ScheduleTargetCheck {
    /// Original combined syntax and explicit unknown ranges.
    pub syntax: CombinedApplicationSyntax,
    /// Scheduled declarations submitted, including invalid ones.
    pub checked_schedules: usize,
    /// Unique bindings with existing local ordinary-function/task targets.
    pub resolved: Vec<LocalScheduleTarget>,
    /// Existing duplicate/missing target diagnostics, E4071/E4072.
    pub diagnostics: Diagnostics,
}

/// Check local schedule target names using existing scanner rules.
/// Imports/services, qualified targets and ambiguous callable declarations remain
/// unavailable contexts. Schedule text, body/return/argument contracts and runtime
/// lifecycle are not validated or executed. Existing syntax/diagnostic quotas apply.
pub fn check_local_schedule_targets(source: &str) -> Result<ScheduleTargetCheck, String> {
    let syntax = syntax::parse_combined(source)?;
    if !syntax.declarations.imports.is_empty() || !syntax.declarations.base.services.is_empty() {
        return Err(
            "local schedule resolution requires import/service context; context unavailable".into(),
        );
    }
    let callables = &syntax.declarations.base.base.callables;
    let mut names = BTreeMap::new();
    for (index, callable) in callables.iter().enumerate() {
        if !matches!(callable.kind, CallableKind::Function | CallableKind::Task)
            || names
                .insert(callable.function.name.as_str(), index)
                .is_some()
        {
            return Err(
                "local schedule callable context is ambiguous or unavailable; context unavailable"
                    .into(),
            );
        }
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "schedule target lexical errors")?;
    let mut seen = BTreeSet::new();
    let mut diagnostics = Diagnostics::new();
    let mut bytes = 0usize;
    let mut resolved = Vec::new();
    let mut checked_schedules = 0;
    for entry in &syntax.entries.entries {
        for member in &entry.members {
            let AppMemberKind::ScheduledTask(task) = &member.kind else {
                continue;
            };
            if task.target.contains('.') {
                return Err(
                    "qualified schedule target requires module context; context unavailable".into(),
                );
            }
            let start = member
                .span
                .end
                .checked_sub(task.target.len())
                .ok_or("schedule target range mismatch")?;
            let index = tokens.partition_point(|token| token.span.start < start);
            let token = tokens
                .get(index)
                .filter(|token| token.span.start == start && token.span.end == member.span.end)
                .ok_or("schedule target token provenance unavailable")?;
            if source[token.span.start..token.span.end] != task.target {
                return Err("schedule target spelling mismatch".into());
            }
            checked_schedules += 1;
            let mut local = Diagnostics::new();
            let unique = super::super::validate_schedule_shape(task, &mut seen, &mut local);
            let target = names.get(task.target.as_str()).copied();
            super::super::validate_schedule_target(task, target.is_some(), &mut local);
            for mut diagnostic in local.items {
                if diagnostics.items.len() == super::MAX_ENTRY_PATH_DIAGNOSTICS {
                    return Err("schedule target diagnostic count limit exceeded".into());
                }
                bytes = bytes
                    .checked_add(diagnostic.message.len())
                    .filter(|sum| *sum <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                    .ok_or("schedule target diagnostic byte limit exceeded")?;
                diagnostic.span = if diagnostic.code == "E4072" {
                    token.span
                } else {
                    member.span
                };
                diagnostics.push(diagnostic);
            }
            if let Some(callable_index) = target.filter(|_| unique) {
                resolved.push(LocalScheduleTarget {
                    schedule_span: member.span,
                    target_span: token.span,
                    callable_index,
                    declaration_span: callables[callable_index].span,
                    kind: callables[callable_index].kind.clone(),
                });
            }
        }
    }
    Ok(ScheduleTargetCheck {
        syntax,
        checked_schedules,
        resolved,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_function_and_task_bindings_keep_identity_and_existing_errors() {
        let source = "// λ\r\napp Demo{\r\ntask every 5 minutes -> job\r\ntask every day -> helper\r\ntask every day -> helper\r\ntask every hour -> absent\r\n}\r\ntask job(){} fn helper(){}";
        let checked = check_local_schedule_targets(source).unwrap();
        assert_eq!(checked.checked_schedules, 4);
        assert_eq!(checked.resolved.len(), 2);
        assert_eq!(checked.resolved[0].kind, CallableKind::Task);
        assert_eq!(checked.resolved[1].kind, CallableKind::Function);
        assert_eq!(
            checked
                .diagnostics
                .items
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["E4071", "E4072"]
        );
        let span = checked.diagnostics.items[1].span;
        assert_eq!(&source[span.start..span.end], "absent");
        for source in [
            "use app.jobs; task job(){}",
            "service Api{}",
            "fn job(){} task job(){}",
            "app Demo{\ntask daily -> jobs.run\n}",
        ] {
            assert!(check_local_schedule_targets(source)
                .unwrap_err()
                .contains("context unavailable"));
        }
    }
    #[test]
    fn schedule_diagnostic_limits_return_no_partial_bindings() {
        let members = (0..super::super::MAX_ENTRY_PATH_DIAGNOSTICS)
            .map(|n| format!("task {n} -> absent{n}\n"))
            .collect::<String>();
        assert_eq!(
            check_local_schedule_targets(&format!("app Demo{{\n{members}}}"))
                .unwrap()
                .diagnostics
                .items
                .len(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        assert!(check_local_schedule_targets(&format!(
            "app Demo{{\n{members}task extra -> absent\n}}"
        ))
        .unwrap_err()
        .contains("count limit"));
        let members = (0..1024)
            .map(|n| format!("task {n} -> {}\n", "x".repeat(1000)))
            .collect::<String>();
        let source = format!("app Demo{{\n{members}}}");
        assert!(source.len() <= syntax::MAX_SOURCE_BYTES);
        assert!(check_local_schedule_targets(&source)
            .unwrap_err()
            .contains("byte limit"));
    }
}
