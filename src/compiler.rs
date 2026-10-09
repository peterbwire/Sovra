//! Public boundaries for the Sovra compiler pipeline.
//!
//! Compiler pipeline boundaries and the M1 lexical foundation.

pub mod ast;
pub mod backend;
pub mod check_report;
pub mod diagnostics;
pub mod interpreter;
pub mod ir;
pub mod lexer;
mod limits;
pub mod parser;
pub mod project;
pub mod semantic;
pub mod stdlib;
pub mod text_files;

const COMMANDS: &[&str] = &[
    "build", "check", "doc", "fmt", "init", "install", "new", "repl", "run", "test", "update",
];

pub(crate) fn is_known_command(command: &str) -> bool {
    COMMANDS.contains(&command)
}
