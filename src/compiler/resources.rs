//! Default and caller-selected compiler limits, independent of execution budgets.

mod ast_budget;
pub(crate) mod interface_strings;
pub use ast_budget::check_ast_budget;

use super::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

/// Approved default aggregate UTF-8 entry-source bytes per executable package graph.
/// Canonical packages are counted once, including the root (ADR 0035).
pub const DEFAULT_PACKAGE_SOURCE_BYTES: usize = 64 * 1024 * 1024;

/// Approved AST admission ceiling for one checked source unit (ADR 0037).
pub const DEFAULT_AST_NODES: usize = 250_000;

/// Approved cumulative interface UTF-8 materialization ceiling per graph (ADR 0039).
pub const DEFAULT_INTERFACE_STRING_BYTES: usize = 128 * 1024 * 1024;

/// Caller-selected input quotas for primary application-project scanning.
/// These do not bound optional body/service inspection or aggregate diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectScanLimits {
    /// Maximum UTF-8 bytes read from each discovered source path.
    pub max_source_bytes: usize,
    /// Maximum source bytes across successful reads; aliases count per read.
    pub max_total_source_bytes: usize,
    /// Maximum discovered .svr paths retained for scanning.
    pub max_source_files: usize,
    /// Maximum directories scheduled, including the root and empty directories.
    pub max_directories: usize,
    /// Maximum yielded directory entries, including non-source entries/errors.
    pub max_directory_entries: usize,
}

impl ProjectScanLimits {
    /// Approved primary application-project discovery/read defaults (ADR 0040).
    pub const DEFAULT: Self = Self {
        max_source_bytes: 16 * 1024 * 1024,
        max_total_source_bytes: 64 * 1024 * 1024,
        max_source_files: 4096,
        max_directories: 4096,
        max_directory_entries: 100_000,
    };
}

/// Caller-selected retained IR emission limits. These are not execution quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoweringLimits {
    /// Maximum emitted instructions across all functions.
    pub max_instructions: usize,
    /// Maximum UTF-8 payload bytes in function/parameter names and instructions.
    /// This excludes Vec capacity, helper maps and temporary lowering work.
    pub max_string_bytes: usize,
}

impl LoweringLimits {
    /// Approved retained IR ceilings for one source-unit lowering (ADR 0037).
    pub const DEFAULT: Self = Self {
        max_instructions: 1_000_000,
        max_string_bytes: 64 * 1024 * 1024,
    };
}

/// Caller-selected cumulative package compilation quotas (separate from inputs).
/// Canonical source ASTs count once; interface materializations count each time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageCompilationLimits {
    /// AST nodes across canonical package entry files, before semantic cloning.
    pub max_ast_nodes: usize,
    /// Materialized signature/parameter, record/field and generated alias nodes.
    /// Repeated imports and consumer copies count separately, including validation.
    pub max_interface_nodes: usize,
    /// Instructions and final linked UTF-8 string payload across all packages.
    pub lowering: LoweringLimits,
}

impl PackageCompilationLimits {
    /// Approved cumulative executable package graph ceilings (ADR 0038).
    pub const DEFAULT: Self = Self {
        max_ast_nodes: 1_000_000,
        max_interface_nodes: 250_000,
        lowering: LoweringLimits {
            max_instructions: 4_000_000,
            max_string_bytes: 256 * 1024 * 1024,
        },
    };
}

/// Caller-selected budgets for canonical package graph resolution.
/// These do not replace source, semantic or process-level resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageGraphLimits {
    /// Maximum UTF-8 bytes in each package manifest.
    pub max_manifest_bytes: usize,
    /// Maximum manifest bytes across unique canonical packages, including root.
    pub max_total_manifest_bytes: usize,
    /// Maximum dependency alias declarations across unique canonical packages.
    /// Distinct aliases count separately even when they resolve to one target.
    pub max_dependency_edges: usize,
}

impl PackageGraphLimits {
    /// Approved package graph input defaults (ADR 0036).
    pub const DEFAULT: Self = Self {
        max_manifest_bytes: 1024 * 1024,
        max_total_manifest_bytes: 8 * 1024 * 1024,
        max_dependency_edges: 8192,
    };
}

/// Frontend resource limits. These do not bound semantic analysis or execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendLimits {
    /// Maximum UTF-8 source bytes, checked before tokenization allocates tokens.
    pub max_source_bytes: usize,
    /// Maximum non-EOF tokens. The terminal EOF token does not consume this budget.
    pub max_tokens: usize,
    /// Maximum lexical or parser diagnostics, including a terminal limit notice.
    /// Zero is treated as one so exhaustion can never masquerade as success.
    pub max_diagnostics: usize,
}

impl FrontendLimits {
    /// Approved defaults for normal executable source compilation (ADR 0035).
    pub const DEFAULT: Self = Self {
        max_source_bytes: 16 * 1024 * 1024,
        max_tokens: 1_000_000,
        max_diagnostics: 256,
    };
}

#[derive(Debug)]
pub(crate) enum SourceReadError {
    Io(io::Error),
    Limit(Diagnostics),
}

pub(crate) fn read_source(path: &Path) -> Result<String, SourceReadError> {
    read_source_with_limit(path, FrontendLimits::DEFAULT.max_source_bytes)
}

pub(crate) fn read_source_with_limit(
    path: &Path,
    max_bytes: usize,
) -> Result<String, SourceReadError> {
    read_utf8_with_limit(
        path,
        max_bytes.min(FrontendLimits::DEFAULT.max_source_bytes),
    )
}

pub(crate) fn read_utf8_with_limit(
    path: &Path,
    max_bytes: usize,
) -> Result<String, SourceReadError> {
    let file = File::open(path).map_err(SourceReadError::Io)?;
    read_bounded(file, max_bytes)
}

fn read_bounded(reader: impl Read, max_bytes: usize) -> Result<String, SourceReadError> {
    let mut bytes = Vec::new();
    reader
        .take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(SourceReadError::Io)?;
    if bytes.len() > max_bytes {
        return Err(SourceReadError::Limit(Diagnostics {
            items: vec![Diagnostic {
                source_file: None,
                severity: Severity::Error,
                code: "E1003",
                message: format!("source exceeds {max_bytes} UTF-8 bytes"),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    column: 0,
                },
            }],
        }));
    }
    String::from_utf8(bytes)
        .map_err(|error| SourceReadError::Io(io::Error::new(io::ErrorKind::InvalidData, error)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reader_stops_after_one_detection_byte_without_metadata() {
        struct Infinite {
            read: usize,
        }
        impl Read for Infinite {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                output.fill(b' ');
                self.read += output.len();
                Ok(output.len())
            }
        }
        let mut reader = Infinite { read: 0 };
        assert!(matches!(
            read_bounded(&mut reader, 8),
            Err(SourceReadError::Limit(_))
        ));
        assert_eq!(reader.read, 9);
        assert!(matches!(
            read_bounded(&mut reader, 0),
            Err(SourceReadError::Limit(_))
        ));
        assert_eq!(reader.read, 10);
    }

    #[test]
    fn bounded_reader_preserves_utf8_and_rejects_boundary_overflow() {
        assert_eq!(read_bounded("λ".as_bytes(), 2).unwrap(), "λ");
        assert!(matches!(
            read_bounded("λ".as_bytes(), 1),
            Err(SourceReadError::Limit(_))
        ));
        match read_bounded(&[0xff][..], 1) {
            Err(SourceReadError::Io(error)) => assert_eq!(error.kind(), io::ErrorKind::InvalidData),
            _ => panic!("invalid UTF-8 must remain an I/O failure"),
        }
        assert_eq!(read_bounded(&b""[..], 0).unwrap(), "");
    }
}
