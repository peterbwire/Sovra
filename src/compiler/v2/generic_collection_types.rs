//! Compiler-owned intrinsic collection headers; source execution stays independently gated.
use super::*;
use crate::compiler::diagnostics::Span;
/// Sealed standard collection header, not a user record or runtime constructor.
#[derive(Debug, Clone)]
pub struct GenericCollectionSyntax {
    kind: GenericCollectionLayoutKind,
    parameters: GenericParameters,
}
impl GenericCollectionSyntax {
    /// Compiler-owned constructor category.
    pub fn kind(&self) -> GenericCollectionLayoutKind {
        self.kind
    }
    /// Reserved qualified name.
    pub fn qualified_name(&self) -> &'static str {
        match self.kind {
            GenericCollectionLayoutKind::Map => "std::Map",
            GenericCollectionLayoutKind::Set => "std::Set",
        }
    }
    /// Immutable declaration-local parameters, including K:std::Key.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    pub(super) fn span(&self) -> Span {
        Span {
            start: 0,
            end: self.parameters.source().len(),
            line: 0,
            column: 0,
        }
    }
}
/// Produce the only two supported intrinsic collection headers, with fresh parameter owners.
/// Supplying these to the semantic registry does not enable source execution or native features.
pub fn standard_generic_collection_types() -> Result<Vec<GenericCollectionSyntax>, GenericTypeError>
{
    [
        (GenericCollectionLayoutKind::Map, "<K:std::Key,V>"),
        (GenericCollectionLayoutKind::Set, "<K:std::Key>"),
    ]
    .into_iter()
    .map(|(kind, source)| {
        Ok(GenericCollectionSyntax {
            kind,
            parameters: parse_generic_parameters(source, GenericParameterLimits::DEFAULT)?,
        })
    })
    .collect()
}
/// Sealed registered opaque collection definition, without fields or enum variants.
#[derive(Debug, Clone)]
pub struct CheckedGenericCollection {
    environment: GenericAliasEnvironment,
    id: GenericTypeDeclarationId,
    syntax: GenericCollectionSyntax,
}
impl CheckedGenericCollection {
    pub(super) fn checked(
        environment: GenericAliasEnvironment,
        id: GenericTypeDeclarationId,
        syntax: GenericCollectionSyntax,
    ) -> Self {
        Self {
            environment,
            id,
            syntax,
        }
    }
    /// Registry-owned declaration identity.
    pub fn id(&self) -> GenericTypeDeclarationId {
        self.id
    }
    /// Compiler-owned constructor and parameter evidence.
    pub fn syntax(&self) -> &GenericCollectionSyntax {
        &self.syntax
    }
    /// Retained semantic type owner.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
}
