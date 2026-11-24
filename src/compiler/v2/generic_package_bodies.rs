//! Source-aware symbolic package bodies; CLI/native package execution remains separate.
use super::*;
use crate::compiler::diagnostics::{diagnostic_message, Diagnostics};
/// Complete symbolic body proofs for all source-owned package signatures.
#[derive(Debug)]
pub struct CheckedGenericPackageBodies {
    types: CheckedGenericPackageTypes,
    bodies: CheckedGenericCopyBodies,
}
impl CheckedGenericPackageBodies {
    /// Original package types, signatures and source/declaration mappings.
    pub fn types(&self) -> &CheckedGenericPackageTypes {
        &self.types
    }
    /// Sealed source-aware body facts for explicit bounded downstream stages.
    pub fn bodies(&self) -> &CheckedGenericCopyBodies {
        &self.bodies
    }
}
/// Check every original function body with its own imports and active scopes.
/// Calls, constructors, patterns and annotations resolve canonical identities.
/// Aggregate body/propagation profiles do not reset at package/file boundaries.
/// This does not select a root, discover specialization, lower, execute or emit.
pub fn check_generic_package_bodies(
    types: CheckedGenericPackageTypes,
    limits: GenericCopyBodyLimits,
    propagation: GenericPropagationLimits,
) -> Result<CheckedGenericPackageBodies, Diagnostics> {
    check_package_bodies(types, limits, propagation, 4)
}
/// Opt-in collection builtin body checking; no layout/IR/execution authority is granted.
pub fn check_generic_collection_package_bodies(
    types: CheckedGenericPackageTypes,
    limits: GenericCopyBodyLimits,
    propagation: GenericPropagationLimits,
) -> Result<CheckedGenericPackageBodies, Diagnostics> {
    check_package_bodies(types, limits, propagation, 5)
}
/// Check explicitly admitted scalar byte builtin bodies under package ownership.
pub fn check_generic_data_package_bodies(
    types: CheckedGenericPackageTypes,
    limits: GenericCopyBodyLimits,
    propagation: GenericPropagationLimits,
) -> Result<CheckedGenericPackageBodies, Diagnostics> {
    check_package_bodies(types, limits, propagation, 6)
}
fn check_package_bodies(
    types: CheckedGenericPackageTypes,
    limits: GenericCopyBodyLimits,
    propagation: GenericPropagationLimits,
    profile: u8,
) -> Result<CheckedGenericPackageBodies, Diagnostics> {
    let checker = if profile == 6 {
        super::generic_copy_bodies::check_generic_data_package_bodies_inner
    } else if profile == 5 {
        super::generic_copy_bodies::check_generic_collection_package_bodies_inner
    } else {
        super::generic_copy_bodies::check_generic_package_bodies_inner
    };
    let bodies = checker(types.signatures(), limits, propagation).map_err(|error| {
        let location = match &error {
            GenericCopyBodyError::Semantic { function, span, .. }
            | GenericCopyBodyError::Propagation { function, span, .. }
            | GenericCopyBodyError::FieldType { function, span, .. } => (*function, Some(*span)),
            GenericCopyBodyError::Parse {
                function,
                diagnostics,
            } => (*function, diagnostics.items.first().map(|d| d.span)),
            GenericCopyBodyError::Annotation {
                function,
                annotation_range,
                ..
            } => (*function, Some(*annotation_range)),
            GenericCopyBodyError::Call {
                function,
                type_range,
                ..
            }
            | GenericCopyBodyError::RecordType {
                function,
                type_range,
                ..
            }
            | GenericCopyBodyError::EnumType {
                function,
                type_range,
                ..
            } => (*function, Some(*type_range)),
            GenericCopyBodyError::Pattern { function, .. }
            | GenericCopyBodyError::Comparison { function, .. } => (*function, None),
            GenericCopyBodyError::Limit(_) => {
                return super::generic_package_types::failure(
                    types.declarations(),
                    None,
                    diagnostic_message!("{error}"),
                )
            }
        };
        let location = types
            .declaration_for_signature(location.0)
            .map(|id| (id, location.1));
        super::generic_package_types::failure(
            types.declarations(),
            location,
            diagnostic_message!("{error}"),
        )
    })?;
    Ok(CheckedGenericPackageBodies { types, bodies })
}
