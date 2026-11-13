//! Optional primary-scanner input admission; existing scanner policy is separate.

use crate::compiler::{
    diagnostics::{diagnostic_message, Diagnostic, Diagnostics, Severity, Span},
    resources::{read_utf8_with_limit, ProjectScanLimits, SourceReadError},
};
use std::path::Path;

pub(super) struct Budget {
    limits: Option<ProjectScanLimits>,
    files: usize,
    directories: usize,
    entries: usize,
    remaining_bytes: usize,
    pub(super) stopped: bool,
}

pub(super) enum Item {
    File,
    Directory,
    Entry,
}

impl Budget {
    pub(super) fn new(limits: Option<ProjectScanLimits>) -> Self {
        Self {
            limits,
            files: 0,
            directories: 0,
            entries: 0,
            remaining_bytes: limits.map_or(usize::MAX, |limits| limits.max_total_source_bytes),
            stopped: false,
        }
    }

    pub(super) fn admit(&mut self, item: Item, path: &Path, diagnostics: &mut Diagnostics) -> bool {
        let Some(limits) = self.limits else {
            return true;
        };
        let (count, limit, kind) = match item {
            Item::File => (
                &mut self.files,
                limits.max_source_files,
                "source file count",
            ),
            Item::Directory => (
                &mut self.directories,
                limits.max_directories,
                "directory count",
            ),
            Item::Entry => (
                &mut self.entries,
                limits.max_directory_entries,
                "directory entry count",
            ),
        };
        if *count >= limit {
            self.stopped = true;
            limit_error(diagnostics, path, kind, limit);
            return false;
        }
        *count += 1;
        true
    }

    pub(super) fn read(&mut self, path: &Path, diagnostics: &mut Diagnostics) -> Option<String> {
        let result = if let Some(limits) = self.limits {
            read_utf8_with_limit(path, limits.max_source_bytes.min(self.remaining_bytes))
        } else {
            std::fs::read_to_string(path).map_err(SourceReadError::Io)
        };
        match result {
            Ok(source) => {
                if self.limits.is_some() {
                    self.remaining_bytes -= source.len();
                }
                Some(source)
            }
            Err(SourceReadError::Limit(_)) => {
                let limits = self.limits.expect("only bounded reads return limit errors");
                let (kind, limit) = if self.remaining_bytes < limits.max_source_bytes {
                    ("total source byte", limits.max_total_source_bytes)
                } else {
                    ("source file byte", limits.max_source_bytes)
                };
                self.stopped = true;
                limit_error(diagnostics, path, kind, limit);
                None
            }
            Err(SourceReadError::Io(error)) => {
                // A failed read may have consumed bytes before returning an
                // error. Limited scans stop here instead of reusing that byte
                // allowance on later files (legacy scans retain their behavior).
                if self.limits.is_some() {
                    self.stopped = true;
                }
                super::push_error(
                    diagnostics,
                    "E4009",
                    diagnostic_message!("cannot read source file `{}`: {error}", path.display()),
                );
                None
            }
        }
    }
}

fn limit_error(diagnostics: &mut Diagnostics, path: &Path, kind: &str, limit: usize) {
    diagnostics.push_bounded(Diagnostic {
        severity: Severity::Error,
        code: "E4150",
        message: diagnostic_message!(
            "project scan {kind} limit of {limit} exceeded at `{}`",
            path.display()
        ),
        source_file: None,
        span: Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        },
    });
}
