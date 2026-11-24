//! Sealed caller/callee type-argument matching, not value calls or execution.
use super::{
    CheckedGenericSignatures, CheckedGenericTypeArguments, GenericParameterBound,
    GenericParameterId, GenericPrimitive, GenericSignatureId, ResolvedGenericTypeKind,
};
use crate::compiler::diagnostics::Span;
use std::sync::Arc;
/// Independent admission for one explicit root-callee type binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCallBindingLimits {
    /// Published callee parameter-to-argument mappings.
    pub max_bindings: usize,
    /// Owner checks, caller parameter comparisons and callee bound checks.
    pub max_work: usize,
}
impl GenericCallBindingLimits {
    /// Finite matching profile; no type/value inference or runtime call.
    pub const DEFAULT: Self = Self {
        max_bindings: 128,
        max_work: 4096,
    };
}
/// Complete matching refusal, without partial binding evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericCallBindingError {
    /// Selected matching budget exhausted.
    Limit(&'static str),
    /// Caller ID is absent or belongs to another signature table.
    Caller,
    /// Callee ID is absent or belongs to another signature table.
    Callee,
    /// Checked arguments use another prepared type/alias environment.
    Environment,
    /// Checked arguments were resolved under another parameter owner.
    CallerScope,
    /// Explicit argument count differs from the callee's parameter count.
    Arity {
        /// Declared callee type parameter count.
        expected: usize,
        /// Supplied checked explicit argument count.
        actual: usize,
    },
    /// Expanded argument does not prove a declared callee Key constraint.
    Bound {
        /// Exact callee parameter identity.
        parameter: GenericParameterId,
        /// Source-order explicit argument index.
        argument: usize,
        /// Original explicit-list argument range.
        span: Span,
    },
}
impl std::fmt::Display for GenericCallBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic callee type binding refused: {self:?}")
    }
}
impl std::error::Error for GenericCallBindingError {}
/// One exact callee-owned type parameter matched to a checked explicit argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericCallTypeBinding {
    /// Exact callee parameter owner/ordinal.
    pub parameter: GenericParameterId,
    /// Source-order argument ordinal in the retained checked list.
    pub argument: usize,
}
/// Sealed caller/callee type matching. This is not a checked value call.
#[derive(Debug, Clone)]
pub struct CheckedGenericCallBinding {
    signatures: CheckedGenericSignatures,
    arguments: CheckedGenericTypeArguments,
    caller: GenericSignatureId,
    callee: GenericSignatureId,
    bindings: Arc<[GenericCallTypeBinding]>,
    work: usize,
}
impl CheckedGenericCallBinding {
    /// Retained original signature owner, including both participants.
    pub fn signatures(&self) -> &CheckedGenericSignatures {
        &self.signatures
    }
    /// Resolved original caller-scope explicit arguments.
    pub fn arguments(&self) -> &CheckedGenericTypeArguments {
        &self.arguments
    }
    /// Original caller signature identity.
    pub fn caller(&self) -> GenericSignatureId {
        self.caller
    }
    /// Original matched callee signature identity.
    pub fn callee(&self) -> GenericSignatureId {
        self.callee
    }
    /// Exact source-order callee parameter mappings.
    pub fn bindings(&self) -> &[GenericCallTypeBinding] {
        &self.bindings
    }
    /// Consumed matching work; argument resolution is separately metered.
    pub fn work(&self) -> usize {
        self.work
    }
}
fn step(work: &mut usize) -> Result<(), GenericCallBindingError> {
    *work = work
        .checked_sub(1)
        .ok_or(GenericCallBindingError::Limit("binding work"))?;
    Ok(())
}
/// Match a checked explicit list to an original root callee in the same table.
/// Verifies caller/table/environment ownership, exact arity and declared Key
/// constraints. Does not substitute signatures, check values/bodies or execute.
pub fn bind_generic_type_arguments(
    signatures: &CheckedGenericSignatures,
    caller: GenericSignatureId,
    callee: GenericSignatureId,
    arguments: &CheckedGenericTypeArguments,
    limits: GenericCallBindingLimits,
) -> Result<CheckedGenericCallBinding, GenericCallBindingError> {
    let mut work = limits.max_work;
    step(&mut work)?;
    let caller_signature = signatures
        .signature(caller)
        .ok_or(GenericCallBindingError::Caller)?;
    step(&mut work)?;
    let callee_signature = signatures
        .signature(callee)
        .ok_or(GenericCallBindingError::Callee)?;
    step(&mut work)?;
    if !signatures
        .types()
        .environment()
        .same_environment(arguments.types().environment())
    {
        return Err(GenericCallBindingError::Environment);
    }
    let caller_parameters = caller_signature.syntax().parameters().parameters();
    if caller_parameters.len() != arguments.parameters().parameters().len() {
        return Err(GenericCallBindingError::CallerScope);
    }
    for (expected, actual) in caller_parameters
        .iter()
        .zip(arguments.parameters().parameters())
    {
        step(&mut work)?;
        if expected.id != actual.id {
            return Err(GenericCallBindingError::CallerScope);
        }
    }
    let parameters = callee_signature.syntax().parameters().parameters();
    if parameters.len() != arguments.arguments().len() {
        return Err(GenericCallBindingError::Arity {
            expected: parameters.len(),
            actual: arguments.arguments().len(),
        });
    }
    if parameters.len() > limits.max_bindings {
        return Err(GenericCallBindingError::Limit("type bindings"));
    }
    let mut bindings = Vec::new();
    for (index, (parameter, argument)) in parameters.iter().zip(arguments.arguments()).enumerate() {
        step(&mut work)?;
        if parameter.bound == Some(GenericParameterBound::Key) {
            step(&mut work)?;
            let allowed = match argument.nodes()[argument.root()].kind {
                ResolvedGenericTypeKind::Primitive(
                    GenericPrimitive::Int | GenericPrimitive::Bool | GenericPrimitive::String,
                ) => true,
                ResolvedGenericTypeKind::Declared { declaration, .. }
                    if super::generic_data_types::is_byte_header(
                        argument.environment().registry(),
                        declaration,
                    ) =>
                {
                    true
                }
                ResolvedGenericTypeKind::Parameter(id) => arguments
                    .parameters()
                    .parameter(id)
                    .is_some_and(|p| p.bound == Some(GenericParameterBound::Key)),
                _ => false,
            };
            if !allowed {
                return Err(GenericCallBindingError::Bound {
                    parameter: parameter.id,
                    argument: index,
                    span: arguments.syntax().nodes()[arguments.syntax().roots()[index]].span,
                });
            }
        }
        bindings.push(GenericCallTypeBinding {
            parameter: parameter.id,
            argument: index,
        });
    }
    Ok(CheckedGenericCallBinding {
        signatures: signatures.clone(),
        arguments: arguments.clone(),
        caller,
        callee,
        bindings: bindings.into(),
        work: limits.max_work - work,
    })
}
