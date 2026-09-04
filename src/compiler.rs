//! Public boundaries for the Sovra compiler pipeline.
//!
//! Compiler pipeline boundaries and the M1 lexical foundation.

pub mod ast;
pub mod backend;
pub mod diagnostics;
pub mod interpreter;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod stdlib;

const COMMANDS: &[&str] = &[
    "build", "check", "doc", "fmt", "init", "install", "new", "repl", "run", "test", "update",
];

pub(crate) fn is_known_command(command: &str) -> bool {
    COMMANDS.contains(&command)
}
