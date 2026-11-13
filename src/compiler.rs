//! Public boundaries for the Sovra compiler pipeline.
//!
//! Compiler pipeline boundaries and the M1 lexical foundation.

pub mod ast;
pub mod backend;
pub mod check_report;
pub mod checked;
pub mod diagnostics;
pub mod execution;
pub(crate) mod ide_symbols;
pub mod interpreter;
pub mod ir;
pub mod lexer;
mod limits;
pub mod native_image;
pub mod parser;
pub mod project;
pub mod resources;
pub mod semantic;
pub mod stdlib;
pub mod testing;
pub mod text_files;

const COMMANDS: &[&str] = &[
    "build", "check", "doc", "doctor", "fmt", "init", "install", "lock", "new", "repl", "run",
    "test", "update",
];

pub(crate) fn is_known_command(command: &str) -> bool {
    COMMANDS.contains(&command)
}

/// Check and lower standalone executable syntax using retained checked evidence.
/// Ordinary compilation uses bounded AST-only metadata (ADR 0041). Existing
/// frontend/AST and IR limits remain independent; metadata exhaustion is E3025.
/// The returned immutable unit also supplies normalized syntax for test discovery.
pub fn compile_program(
    program: &ast::Program,
) -> Result<(ir::IrProgram, checked::CheckedUnit), diagnostics::Diagnostics> {
    use checked::{
        CheckedUnit, EvidenceError, EvidenceLimits, FlowEvidenceLimits, LoweringEvidenceLimits,
        SemanticEvidenceLimits,
    };
    // Preserve ordinary AST admission diagnostics rather than translating them
    // into a metadata quota error. The evidence index uses the same ceiling.
    resources::check_ast_budget(program, resources::DEFAULT_AST_NODES)?;
    let unit = CheckedUnit::from_program_for_lowering(
        program,
        EvidenceLimits {
            max_nodes: resources::DEFAULT_AST_NODES,
            max_source_bytes: 0,
        },
        SemanticEvidenceLimits {
            max_facts: 250_000,
            max_types: 65_536,
            max_name_bytes: 16 * 1024 * 1024,
        },
        FlowEvidenceLimits {
            max_declarations: 250_000,
            max_uses: 250_000,
            max_coercions: 250_000,
            max_name_bytes: 16 * 1024 * 1024,
        },
        LoweringEvidenceLimits {
            max_function_results: 250_000,
            max_contexts: 1_000_000,
        },
    )
    .map_err(EvidenceError::into_diagnostics)?;
    let ir =
        ir::lower_checked(&unit, resources::LoweringLimits::DEFAULT).map_err(
            |error| match error {
                ir::CheckedLoweringError::Emission(errors) => errors,
                ir::CheckedLoweringError::EvidenceUnavailable => {
                    unreachable!("lowering-ready constructor guarantees complete evidence")
                }
            },
        )?;
    Ok((ir, unit))
}
