//! Explicit aggregate evidence accounting for the opt-in checked package path.

use crate::compiler::{
    checked::{
        CheckedUnit, EvidenceError, EvidenceLimits, FlowEvidenceLimits, LoweringEvidenceLimits,
        SemanticEvidenceLimits,
    },
    diagnostics::Diagnostics,
    ir::IrProgram,
};
use std::path::PathBuf;

/// Cumulative metadata allowances across canonical units. Attached interface AST
/// copies and per-consumer imports count each time they are retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageEvidenceLimits {
    /// Structural indexed nodes, including attached interface declarations.
    pub max_nodes: usize,
    /// Cumulative facts, interned types and nominal-name bytes (per-unit copies count).
    pub semantic: SemanticEvidenceLimits,
    /// Cumulative declarations, all local/imported uses, coercions and local names.
    pub flow: FlowEvidenceLimits,
    /// Cumulative function results and contextual edges.
    pub lowering: LoweringEvidenceLimits,
    /// Normalized import bindings; distinct aliases count separately.
    pub max_imports: usize,
    /// Retained consumer spellings and canonical linked-symbol bytes per binding.
    pub max_import_bytes: usize,
}

impl PackageEvidenceLimits {
    /// Active cumulative package metadata ceilings (ADR 0041). These do not
    /// replace source/interface/IR admission or bound total process memory.
    pub const DEFAULT: Self = Self {
        max_nodes: 1_250_000,
        semantic: SemanticEvidenceLimits {
            max_facts: 1_250_000,
            max_types: 262_144,
            max_name_bytes: 64 * 1024 * 1024,
        },
        flow: FlowEvidenceLimits {
            max_declarations: 1_250_000,
            max_uses: 1_250_000,
            max_coercions: 1_250_000,
            max_name_bytes: 64 * 1024 * 1024,
        },
        lowering: LoweringEvidenceLimits {
            max_function_results: 1_250_000,
            max_contexts: 5_000_000,
        },
        max_imports: 250_000,
        max_import_bytes: 64 * 1024 * 1024,
    };
}

/// Opt-in compilation output plus unspent explicit metadata allowances.
#[derive(Debug)]
pub struct CheckedPackage {
    /// Fully linked checked IR, with the same canonical symbols as legacy linking.
    pub ir: IrProgram,
    /// Canonical source units compiled once each, regardless of alias count.
    pub units: usize,
    /// Remaining cumulative evidence allowances after all units succeeded.
    pub remaining: PackageEvidenceLimits,
}

/// Ordinary source/link failures remain distinct from metadata exhaustion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedPackageError {
    /// Existing compiler/interface/emission diagnostics, including source identity.
    Diagnostics(Diagnostics),
    /// No partial unit or linked program is published on evidence failure.
    Evidence {
        /// Canonical entry of the excess unit; no fabricated source byte range.
        entry: PathBuf,
        /// Explicit metadata admission/collection error.
        error: EvidenceError,
    },
}
impl From<Diagnostics> for CheckedPackageError {
    fn from(errors: Diagnostics) -> Self {
        Self::Diagnostics(errors)
    }
}
impl CheckedPackageError {
    /// Preserve source/link diagnostics or map metadata exhaustion to E3025.
    /// Metadata errors have no fabricated source range. Inspect the structured
    /// error before conversion when the canonical failing entry path is needed.
    pub fn into_diagnostics(self) -> Diagnostics {
        match self {
            Self::Diagnostics(diagnostics) => diagnostics,
            Self::Evidence { error, .. } => error.into_diagnostics(),
        }
    }
}
impl std::fmt::Display for CheckedPackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostics(errors) => write!(
                f,
                "package compilation failed with {} diagnostics",
                errors.items.len()
            ),
            Self::Evidence { error, .. } => {
                write!(f, "package evidence collection failed: {error}")
            }
        }
    }
}
impl std::error::Error for CheckedPackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Evidence { error, .. } => Some(error),
            _ => None,
        }
    }
}

pub(super) struct EvidenceBudget {
    pub remaining: PackageEvidenceLimits,
    pub units: usize,
}
impl EvidenceBudget {
    pub fn structural(&self) -> EvidenceLimits {
        EvidenceLimits {
            max_nodes: self.remaining.max_nodes,
            max_source_bytes: 0,
        }
    }
    pub fn charge(&mut self, unit: &CheckedUnit) {
        let evidence = unit.semantic_evidence().unwrap();
        let flow = evidence.flow().unwrap();
        let lowering = evidence.lowering().unwrap();
        let package = evidence.package().unwrap();
        self.remaining.max_nodes -= unit.nodes().len();
        self.remaining.semantic.max_facts -= evidence.facts().len();
        self.remaining.semantic.max_types -= evidence.types().len();
        self.remaining.semantic.max_name_bytes -= evidence.name_bytes();
        self.remaining.flow.max_declarations -= flow.declarations().len();
        self.remaining.flow.max_uses -= flow.uses().len() + package.calls().len();
        self.remaining.flow.max_coercions -= flow.coercions().len();
        self.remaining.flow.max_name_bytes -= flow.name_bytes();
        self.remaining.lowering.max_function_results -= lowering.function_results().len();
        self.remaining.lowering.max_contexts -= lowering.contexts().len();
        self.remaining.max_imports -= package.imports().len();
        self.remaining.max_import_bytes -= package.import_bytes();
        self.units += 1;
    }
}
