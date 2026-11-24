//! Sealed compiler-owned data headers; executable layouts remain a separate gate.
use super::*;
use crate::compiler::diagnostics::Span;
/// Standard opaque data constructor identity, distinct from records and primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericDataTypeKind {
    /// Checked scalar byte in 0..255; admitted by the std::Key constraint.
    Byte,
    /// Immutable binary buffer, never a key or an implicit String.
    Bytes,
}
/// Sealed zero-argument standard data header with a fresh parameter scope.
#[derive(Debug, Clone)]
pub struct GenericDataSyntax {
    kind: GenericDataTypeKind,
    parameters: GenericParameters,
}
impl GenericDataSyntax {
    /// Compiler-owned category.
    pub fn kind(&self) -> GenericDataTypeKind {
        self.kind
    }
    /// Canonical qualified name.
    pub fn qualified_name(&self) -> &'static str {
        match self.kind {
            GenericDataTypeKind::Byte => "std::Byte",
            GenericDataTypeKind::Bytes => "std::Bytes",
        }
    }
    /// Empty, declaration-owned parameter list.
    pub fn parameters(&self) -> &GenericParameters {
        &self.parameters
    }
    pub(super) fn span(&self) -> Span {
        Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        }
    }
}
/// Produce the only supported data headers; this grants no body/VM/native execution.
pub fn standard_generic_data_types() -> Result<Vec<GenericDataSyntax>, GenericTypeError> {
    [GenericDataTypeKind::Byte, GenericDataTypeKind::Bytes]
        .into_iter()
        .map(|kind| {
            Ok(GenericDataSyntax {
                kind,
                parameters: GenericParameters::empty()?,
            })
        })
        .collect()
}
/// Checked data declaration retaining exact registry/environment identity.
#[derive(Debug, Clone)]
pub struct CheckedGenericData {
    environment: GenericAliasEnvironment,
    id: GenericTypeDeclarationId,
    syntax: GenericDataSyntax,
}
impl CheckedGenericData {
    pub(super) fn checked(
        environment: GenericAliasEnvironment,
        id: GenericTypeDeclarationId,
        syntax: GenericDataSyntax,
    ) -> Self {
        Self {
            environment,
            id,
            syntax,
        }
    }
    /// Prepared environment owning this declaration identity.
    pub fn environment(&self) -> &GenericAliasEnvironment {
        &self.environment
    }
    /// Registry-owned declaration ID.
    pub fn id(&self) -> GenericTypeDeclarationId {
        self.id
    }
    /// Sealed standard data syntax.
    pub fn syntax(&self) -> &GenericDataSyntax {
        &self.syntax
    }
}
pub(super) fn is_byte_header(registry: &GenericTypeRegistry, id: GenericTypeDeclarationId) -> bool {
    registry
        .header(id)
        .is_some_and(|h| h.kind == GenericTypeDeclarationKind::Data(GenericDataTypeKind::Byte))
}

/// Expected success domain of a sealed data Result; used by independent admission stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DataResultKind {
    Byte,
    Bytes,
    ByteArray,
    String,
    JsonValue,
}
