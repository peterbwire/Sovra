//! Import provenance supplied only by the canonical local-package resolver.

use super::{EvidenceError, NodeId};
use crate::compiler::ast::Function;
use std::{collections::HashMap, sync::Arc};

/// Import ordinal, scoped to its owning package snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportId(pub(super) usize);
impl ImportId {
    /// Snapshot-local import-table index, not a persistent graph identity.
    pub const fn ordinal(self) -> usize {
        self.0
    }
}

/// Resolved normalized function import, without a fabricated local AST node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionImport {
    /// Snapshot-local import identity.
    pub id: ImportId,
    /// Canonical provider ordinal within this resolved graph.
    pub owner: usize,
    /// Consumer-visible alias/module/function spelling.
    pub spelling: Arc<str>,
    /// Canonical linked function symbol selected by the package resolver.
    pub symbol: Arc<str>,
}

/// Source call head bound to an imported function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportedCall {
    /// Call head in this snapshot; never a provider source location.
    pub node: NodeId,
    /// Import-table entry in this same snapshot.
    pub import: ImportId,
}

/// Canonical package/import provenance. Ordinals belong to one resolved graph;
/// mutable paths are not content locks or globally persistent identities.
#[derive(Debug)]
pub struct PackageEvidence {
    pub(super) owner: usize,
    pub(super) imports: Vec<FunctionImport>,
    pub(super) calls: Vec<ImportedCall>,
    pub(super) bytes: usize,
}
impl PackageEvidence {
    /// Consumer package ordinal in the canonical resolved graph.
    pub fn owner(&self) -> usize {
        self.owner
    }
    /// Normalized import bindings; aliases may share one owner/symbol.
    pub fn imports(&self) -> &[FunctionImport] {
        &self.imports
    }
    /// Checked imported call heads, separate from local/builtin flow uses.
    pub fn calls(&self) -> &[ImportedCall] {
        &self.calls
    }
    /// Charged UTF-8 consumer spelling plus linked-symbol bytes per binding.
    pub fn import_bytes(&self) -> usize {
        self.bytes
    }
}

#[derive(Debug)]
pub(crate) struct ImportBinding<'a> {
    pub spelling: &'a str,
    pub owner: usize,
    pub symbol: &'a str,
    pub signature: &'a Function,
}

pub(super) struct ImportRecorder {
    pub evidence: PackageEvidence,
    pub lookup: HashMap<Arc<str>, (ImportId, usize)>,
}
impl ImportRecorder {
    pub fn new<'a>(
        owner: usize,
        imports: impl IntoIterator<Item = ImportBinding<'a>>,
        max_imports: usize,
        max_bytes: usize,
    ) -> Result<Self, EvidenceError> {
        let mut result = Self {
            evidence: PackageEvidence {
                owner,
                imports: Vec::new(),
                calls: Vec::new(),
                bytes: 0,
            },
            lookup: HashMap::new(),
        };
        for binding in imports {
            if result.evidence.imports.len() >= max_imports {
                return Err(EvidenceError::Limit);
            }
            let bytes = result
                .evidence
                .bytes
                .checked_add(binding.spelling.len())
                .and_then(|n| n.checked_add(binding.symbol.len()))
                .ok_or(EvidenceError::Limit)?;
            if bytes > max_bytes {
                return Err(EvidenceError::Limit);
            }
            let id = ImportId(result.evidence.imports.len());
            let spelling: Arc<str> = Arc::from(binding.spelling);
            result.lookup.insert(
                Arc::clone(&spelling),
                (id, binding.signature as *const Function as usize),
            );
            result.evidence.imports.push(FunctionImport {
                id,
                owner: binding.owner,
                spelling,
                symbol: Arc::from(binding.symbol),
            });
            result.evidence.bytes = bytes;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{
        checked::{
            CheckedUnit, EvidenceLimits, FlowEvidenceLimits, LoweringEvidenceLimits,
            SemanticEvidenceLimits,
        },
        ir::{self, Instruction},
        parser::Parser,
        resources::LoweringLimits,
        semantic::SemanticAnalyzer,
    };

    fn build(uses: usize, bytes: usize) -> Result<CheckedUnit, EvidenceError> {
        let provider = SemanticAnalyzer::new()
            .analyze(
                &Parser::new()
                    .parse_source("fn f(x: Float) -> Float { return x; }")
                    .unwrap(),
            )
            .unwrap();
        let signature = &provider.program.functions[0];
        let signatures = vec![
            ("a::api::f".to_owned(), signature),
            ("b::api::f".to_owned(), signature),
        ];
        let bindings = signatures.iter().map(|(name, signature)| ImportBinding {
            spelling: name,
            owner: 7,
            symbol: "@package7::api::f",
            signature,
        });
        let program = Parser::new()
            .parse_source("fn main() { print(a::api::f(1)); print(b::api::f(2)); }")
            .unwrap();
        CheckedUnit::from_package_for_lowering(
            &program,
            &signatures,
            0,
            bindings,
            2,
            bytes,
            EvidenceLimits {
                max_nodes: 128,
                max_source_bytes: 0,
            },
            SemanticEvidenceLimits {
                max_facts: 128,
                max_types: 128,
                max_name_bytes: 4096,
            },
            FlowEvidenceLimits {
                max_declarations: 128,
                max_uses: uses,
                max_coercions: 128,
                max_name_bytes: 4096,
            },
            LoweringEvidenceLimits {
                max_function_results: 128,
                max_contexts: 128,
            },
        )
    }

    #[test]
    fn shared_signature_aliases_keep_owner_identity_without_fake_local_nodes() {
        let unit = build(4, 52).unwrap(); // Providers/signature buffers have already dropped.
        let evidence = unit.semantic_evidence().unwrap();
        let package = evidence.package().unwrap();
        assert_eq!(package.owner(), 0);
        assert_eq!(package.imports().len(), 2);
        assert_eq!(package.import_bytes(), 52);
        assert_eq!(
            package
                .calls()
                .iter()
                .map(|call| call.import.ordinal())
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert!(package
            .imports()
            .iter()
            .all(|binding| binding.owner == 7 && binding.symbol.as_ref() == "@package7::api::f"));
        assert_eq!(evidence.flow().unwrap().declarations().len(), 1); // Only main; no fake imports/params.
        assert_eq!(evidence.flow().unwrap().uses().len(), 2); // Canonical builtin calls; imports are separate.
        assert_eq!(unit.program().functions.len(), 1);
        assert!(unit.source().is_none());
        let emitted = ir::lower_checked(&unit, LoweringLimits::DEFAULT).unwrap();
        assert_eq!(
            emitted.functions[0]
                .instructions
                .iter()
                .filter(
                    |i| matches!(i, Instruction::Call { name, .. } if name == "@package7::api::f")
                )
                .count(),
            2
        );
    }

    #[test]
    fn import_bytes_and_combined_local_import_uses_share_atomic_limits() {
        assert!(matches!(build(3, 52), Err(EvidenceError::Limit)));
        assert!(matches!(build(4, 51), Err(EvidenceError::Limit)));
        assert!(matches!(build(0, 52), Err(EvidenceError::Limit)));
        assert!(matches!(build(4, 0), Err(EvidenceError::Limit)));
        assert!(build(4, 52).is_ok());
    }
}
