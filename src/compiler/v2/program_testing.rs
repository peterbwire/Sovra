//! Source-order, host-free tests over one lowered checked root program.
use super::*;
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span},
    semantic::Type,
    testing::{TestOutcome, TestReport},
};
/// Independent suite admission; runtime fuel is fresh for each test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramTestLimits {
    /// Reject discovery before lowering/execution when this count is exceeded.
    pub max_tests: usize,
    /// Retained captured UTF-8 bytes, including one LF per line, across the suite.
    pub max_output_bytes: usize,
}
impl ProgramTestLimits {
    /// Finite opt-in suite admission, independent of per-test runtime limits.
    pub const DEFAULT: Self = Self {
        max_tests: 32,
        max_output_bytes: 1_048_576,
    };
}
fn diagnostic(code: &'static str, message: String, span: Span) -> Diagnostic {
    Diagnostic {
        code,
        message,
        span,
        severity: Severity::Error,
        source_file: None,
    }
    .bounded()
}
fn unknown() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    }
}
impl CheckedProgramSource {
    /// Discover parameterless Unit test_ functions and lower the complete program once.
    /// No main is required or automatically called. Every test has fresh VM state,
    /// policy, empty arguments/status and no input/file hosts. Continue after runtime
    /// failures. Failed VM output is unavailable; suite-output refusal discards that
    /// outcome's captured output and reports a test failure. No unlimited fallback.
    pub fn test(
        &self,
        lowering: ProgramLoweringLimits,
        policy: RuntimeVmPolicy,
        limits: ProgramTestLimits,
    ) -> TestReport {
        let mut report = TestReport {
            target: self.source_label().into(),
            package: false,
            diagnostics: Diagnostics::new(),
            tests: Vec::new(),
        };
        let mut selected = Vec::new();
        for function in self.functions() {
            let function = function.typed().callable().function();
            let syntax = function.syntax();
            if function.environment().module().is_some() || !syntax.name.starts_with("test_") {
                continue;
            }
            let names = function.environment().body_named_types();
            let result = Type::from_name_with_known(
                syntax.return_type.as_deref().unwrap_or("Unit"),
                Some(names),
            )
            .canonicalize(names);
            if !syntax.parameters.is_empty() || result != Type::Unit {
                report.diagnostics.items.push(diagnostic(
                    "E5001",
                    diagnostic_message!(
                        "test function `{}` must have no parameters and return Unit",
                        syntax.name
                    ),
                    syntax.span,
                ));
            }
            selected.push((
                self.function_id(&syntax.name)
                    .expect("sealed function identity"),
                syntax.name.clone(),
            ));
        }
        if selected.is_empty() {
            report.diagnostics.items.push(diagnostic(
                "E5002",
                "no root-level test_ functions were found".into(),
                unknown(),
            ));
        } else if selected.len() > limits.max_tests {
            report.diagnostics.items.push(diagnostic(
                "E5003",
                "root-program test count allowance exceeded".into(),
                unknown(),
            ));
        }
        // Reuse bounded projection only for failed discovery, preserving error codes.
        if !report.diagnostics.is_empty() {
            report.diagnostics =
                ProgramSourceError::Source(EntrySourceError::Syntax(report.diagnostics))
                    .into_diagnostics();
        }
        if !report.diagnostics.is_empty() {
            for d in &mut report.diagnostics.items {
                if d.span.end > 0 {
                    d.source_file = Some(self.source_label().into());
                }
            }
            return report;
        }
        let execution = self.with_linked_ir(selected[0].0, lowering, |functions, ids, _| {
            let mut remaining = limits.max_output_bytes;
            let mut outcomes = Vec::with_capacity(selected.len());
            for (entry, name) in &selected {
                let result = super::runtime::execute_linked(
                    functions,
                    ids,
                    entry.ordinal(),
                    &[],
                    Some(&[]),
                    RuntimeVmServices::default(),
                    policy.clone(),
                );
                let mut outcome = TestOutcome {
                    name: name.clone(),
                    passed: false,
                    output: Vec::new(),
                    exit_code: None,
                    error: None,
                };
                match result {
                    Ok(result) => {
                        let bytes = result
                            .execution
                            .output
                            .iter()
                            .try_fold(0usize, |total, line| {
                                total.checked_add(line.len())?.checked_add(1)
                            });
                        match bytes.and_then(|bytes| remaining.checked_sub(bytes)) {
                            Some(next) => {
                                remaining = next;
                                outcome.passed = result.exit_code == 0;
                                outcome.exit_code = Some(result.exit_code);
                                outcome.output = result.execution.output;
                            }
                            None => {
                                outcome.error =
                                    Some("test suite captured-output allowance exceeded".into())
                            }
                        }
                    }
                    Err(error) => outcome.error = Some(diagnostic_message!("{error}")),
                }
                outcomes.push(outcome);
            }
            Ok(outcomes)
        });
        match execution {
            Ok(tests) => report.tests = tests,
            Err(error) => {
                let code = match error {
                    ProgramExecutionError::Limit
                    | ProgramExecutionError::Mapping(MappingError::Limit(_))
                    | ProgramExecutionError::Mapping(MappingError::IdentityExhausted)
                    | ProgramExecutionError::Expression(
                        ExpressionIrError::Limit(_) | ExpressionIrError::IdentityExhausted,
                    )
                    | ProgramExecutionError::Function(
                        FunctionIrError::Limit(_)
                        | FunctionIrError::Expression(
                            ExpressionIrError::Limit(_) | ExpressionIrError::IdentityExhausted,
                        ),
                    ) => "E3025",
                    _ => "E3122",
                };
                report.diagnostics.items.push(diagnostic(
                    code,
                    diagnostic_message!("{error}"),
                    unknown(),
                ));
            }
        }
        report
    }
}
