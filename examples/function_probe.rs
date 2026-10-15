//! Internal tooling probe: select a checked function without modifying the IR.

use sovra::compiler::{
    diagnostics::Diagnostics,
    interpreter::{run_function_with_host_status, ProcessHost},
    ir,
    parser::Parser,
    project::packages,
    semantic::SemanticAnalyzer,
};
use std::{env, fs, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    match probe() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("function probe: {error}");
            ExitCode::from(1)
        }
    }
}

fn probe() -> Result<u8, String> {
    let mut args = env::args().skip(1);
    let source = PathBuf::from(
        args.next()
            .ok_or("expected SOURCE FUNCTION [program args]")?,
    );
    let function = args
        .next()
        .ok_or("expected SOURCE FUNCTION [program args]")?;
    let program = if source.is_dir() {
        packages::compile(&source).map_err(diagnostics)?
    } else {
        let text = fs::read_to_string(&source).map_err(|error| error.to_string())?;
        let parsed = Parser::new().parse_source(&text).map_err(diagnostics)?;
        let typed = SemanticAnalyzer::new()
            .analyze(&parsed)
            .map_err(diagnostics)?;
        ir::lower(&typed)
    };
    let mut host = ProcessHost::new(args.collect());
    run_function_with_host_status(&program, &function, &mut host).map(|outcome| outcome.exit_code)
}

fn diagnostics(errors: Diagnostics) -> String {
    errors
        .items
        .into_iter()
        .map(|error| format!("error[{}]: {}", error.code, error.message))
        .collect::<Vec<_>>()
        .join("\n")
}
