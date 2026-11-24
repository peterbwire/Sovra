//! Shared-front-end package suites with all-root preflight and bounded replay.
use super::*;
use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostics},
    project::packages::{CapturedPackageSources, PackageSourceKind},
    testing::{TestOutcome, TestReport},
};
use std::sync::Arc;
/// Whole-suite admission, independently of finite per-root lowering/VM profiles.
#[derive(Debug, Clone, Copy)]
pub struct GenericPackageTestLimits {
    /// Tests in the root entry, hard capped at 32.
    pub max_tests: usize,
    /// Concrete function instances across both passes, hard capped at 8192.
    pub max_instances: usize,
}
impl GenericPackageTestLimits {
    /// Shared body checking once, at most 32 roots and 8192 lowered instances.
    pub const DEFAULT: Self = Self {
        max_tests: 32,
        max_instances: 8192,
    };
}
fn problem(
    bodies: &CheckedGenericPackageBodies,
    root: Option<GenericPackageDeclarationId>,
    code: &'static str,
    message: &str,
) -> Diagnostics {
    let mut errors = super::generic_package_types::failure(
        bodies.types().declarations(),
        root.map(|id| (id, None)),
        message.into(),
    );
    errors.items[0].code = code;
    errors
}
/// Run source-order `test_` functions in the root entry, with no implicit main.
/// Dependencies/modules supply helpers but their own test functions are not run.
/// All original bodies and every selected concrete root are checked before any
/// execution. Replay reuses the same immutable body proof, retaining one lowered
/// program at a time. Every VM has fresh policy/state; runtime failures continue.
pub fn test_generic_package(
    inputs: CapturedPackageSources,
    label: &str,
    compilation: GenericPackageCompilationLimits,
    policy: GenericVmPolicy,
    limits: GenericPackageTestLimits,
) -> TestReport {
    test_package_mode(inputs, label, compilation, policy, limits, false, false)
}
/// Execute an explicitly admitted collection package suite with shared all-root preflight.
pub fn test_generic_collection_package(
    inputs: CapturedPackageSources,
    label: &str,
    compilation: GenericPackageCompilationLimits,
    policy: GenericVmPolicy,
    limits: GenericPackageTestLimits,
) -> TestReport {
    test_package_mode(inputs, label, compilation, policy, limits, true, false)
}
fn test_package_mode(
    inputs: CapturedPackageSources,
    label: &str,
    compilation: GenericPackageCompilationLimits,
    policy: GenericVmPolicy,
    limits: GenericPackageTestLimits,
    collections: bool,
    data: bool,
) -> TestReport {
    let mut report = TestReport {
        target: label.into(),
        package: true,
        diagnostics: Diagnostics::new(),
        tests: Vec::new(),
    };
    let checked = (|| {
        let table = bind_generic_package_declarations(inputs, compilation.declarations)?;
        let types = if data {
            check_generic_data_package_types(table, compilation.types)
        } else if collections {
            check_generic_collection_package_types(table, compilation.types)
        } else {
            check_generic_package_types(table, compilation.types)
        }?;
        if data {
            check_generic_data_package_bodies(types, compilation.bodies, compilation.propagation)
        } else if collections {
            check_generic_collection_package_bodies(
                types,
                compilation.bodies,
                compilation.propagation,
            )
        } else {
            check_generic_package_bodies(types, compilation.bodies, compilation.propagation)
        }
    })();
    let bodies = match checked {
        Ok(bodies) => Arc::new(bodies),
        Err(errors) => {
            report.diagnostics = errors;
            return report;
        }
    };
    let table = bodies.types().declarations();
    let mut selected = Vec::new();
    for declaration in table.declarations() {
        let Some(source) = table.source_registration(declaration.source()) else {
            continue;
        };
        if source.kind != PackageSourceKind::Entry
            || source.package_root != table.inputs().inventory().graph.root
        {
            continue;
        }
        let GenericSourceDeclarationKind::Function(function) = declaration.syntax().kind() else {
            continue;
        };
        if !function.name().starts_with("test_") {
            continue;
        }
        if selected.len() >= limits.max_tests.min(32) {
            report.diagnostics = problem(
                &bodies,
                None,
                "E5003",
                "generic package test count allowance exceeded",
            );
            return report;
        }
        let signature = bodies
            .types()
            .signature_id(declaration.id())
            .and_then(|id| bodies.types().signatures().signature(id));
        let valid = signature.is_some_and(|signature| {
            signature.parameter_types().is_empty()
                && signature.syntax().parameters().parameters().is_empty()
                && match signature.return_type() {
                    GenericSignatureReturn::ImplicitUnit => true,
                    GenericSignatureReturn::Explicit(ty) => matches!(
                        ty.nodes()[ty.root()].kind,
                        ResolvedGenericTypeKind::Primitive(GenericPrimitive::Unit)
                    ),
                }
        });
        if !valid {
            report.diagnostics = problem(
                &bodies,
                Some(declaration.id()),
                "E5001",
                "test functions must be nongeneric, parameterless and return Unit",
            );
            return report;
        }
        selected.push(declaration.id());
    }
    if selected.is_empty() {
        report.diagnostics = problem(
            &bodies,
            None,
            "E5002",
            "no root-entry test_ functions were found",
        );
        return report;
    }
    let mut remaining = limits.max_instances.min(8192);
    let mut plans = Vec::new();
    for root in selected {
        let mut profile = compilation;
        profile.specialization.max_instances =
            profile.specialization.max_instances.min(remaining / 2);
        if profile.specialization.max_instances == 0 {
            report.diagnostics = problem(
                &bodies,
                None,
                "E5003",
                "generic package suite instance allowance exceeded",
            );
            return report;
        }
        let program = match super::generic_package_compile::compile_package_root_profile(
            Arc::clone(&bodies),
            root,
            profile,
            collections,
            data,
        ) {
            Ok(program) => program,
            Err(errors) => {
                report.diagnostics = errors;
                return report;
            }
        };
        let count = program.ir().checked_ir().functions().len();
        let Some(next) = count
            .checked_mul(2)
            .and_then(|used| remaining.checked_sub(used))
        else {
            report.diagnostics = problem(
                &bodies,
                None,
                "E5003",
                "generic package suite instance allowance exceeded",
            );
            return report;
        };
        remaining = next;
        // The replay cannot allocate more instances than were reserved here.
        profile.specialization.max_instances = count;
        plans.push((root, profile));
    }
    for (root, profile) in plans {
        let program = match super::generic_package_compile::compile_package_root_profile(
            Arc::clone(&bodies),
            root,
            profile,
            collections,
            data,
        ) {
            Ok(program) => program,
            Err(errors) => {
                report.diagnostics = errors;
                return report;
            }
        };
        let execute = if data {
            execute_generic_data_package
        } else if collections {
            execute_generic_collection_package
        } else {
            execute_generic_package
        };
        let execution = execute(&program, policy.clone());
        let error = execution
            .as_ref()
            .err()
            .and_then(|errors| errors.items.first())
            .map(|error| match error.source_file.as_deref() {
                Some(path) => diagnostic_message!(
                    "{path}:{}:{}: [{}] {}",
                    error.span.line + 1,
                    error.span.column + 1,
                    error.code,
                    error.message
                ),
                None => diagnostic_message!("[{}] {}", error.code, error.message),
            });
        report.tests.push(TestOutcome {
            name: program.entry().into(),
            passed: execution.is_ok(),
            output: Vec::new(),
            exit_code: execution.as_ref().ok().map(|_| 0),
            error,
        });
    }
    report
}

/// Run all selected data-package tests after complete preflight with fresh per-test policy.
pub fn test_generic_data_package(
    inputs: CapturedPackageSources,
    label: &str,
    compilation: GenericPackageCompilationLimits,
    policy: GenericVmPolicy,
    limits: GenericPackageTestLimits,
) -> TestReport {
    test_package_mode(inputs, label, compilation, policy, limits, true, true)
}
