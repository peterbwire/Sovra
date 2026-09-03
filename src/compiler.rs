//! Public boundaries for the Sovra compiler pipeline.
//!
//! M0 deliberately contains no language implementation. Each module exposes a
//! small, documented marker so later milestones can add behavior without
//! changing the top-level crate layout.

pub mod ast;
pub mod backend;
pub mod diagnostics;
pub mod interpreter;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;

const COMMANDS: &[&str] = &["build", "check", "fmt", "init", "run"];

pub(crate) fn is_known_command(command: &str) -> bool {
    COMMANDS.contains(&command)
}

